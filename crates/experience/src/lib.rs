// v16.0: Experience store — JSONL 持久化 + 关键词检索。
// 语义检索（embedding/cosine）、环境适用性匹配、reinforce 等死代码已随 D-47 删除。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Experience entry with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub id: String,
    pub category: String,
    pub problem: String,
    pub solution: String,
    pub success: bool,
    pub effectiveness: f32,
    pub reference_count: u32,
    pub created_at: String,
}

/// Experience store.
pub struct ExperienceStore {
    entries: tokio::sync::RwLock<Vec<Experience>>,
    /// v16.0: JSONL file path for persistence (std Mutex — only held for path read).
    file_path: std::sync::Mutex<Option<PathBuf>>,
}

impl Default for ExperienceStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ExperienceStore {
    pub fn new() -> Self {
        Self {
            entries: tokio::sync::RwLock::new(Vec::new()),
            file_path: std::sync::Mutex::new(None),
        }
    }

    /// v16.0: Enable JSONL file persistence at the given path.
    /// Loads existing records from disk on first call.
    /// Safe to call on `&self` (uses std::sync::Mutex for interior mutability).
    pub async fn set_path(&self, path: PathBuf) -> std::io::Result<()> {
        // Load existing records if the file exists.
        if path.exists() {
            let content = tokio::fs::read_to_string(&path).await?;
            let mut loaded: Vec<Experience> = Vec::new();
            for line in content.lines() {
                if let Ok(exp) = serde_json::from_str::<Experience>(line) {
                    loaded.push(exp);
                }
            }
            let mut entries = self.entries.write().await;
            entries.extend(loaded);
            tracing::info!(loaded = entries.len(), path = %path.display(), "experience store loaded from disk");
        }
        // Set the path for future appends.
        *self.file_path.lock().unwrap() = Some(path);
        Ok(())
    }

    /// v16.0: Append one entry to the JSONL file (called inside append()).
    fn append_to_disk(&self, exp: &Experience) {
        let fp = self.file_path.lock().unwrap();
        let Some(ref path) = *fp else { return };
        if let Ok(line) = serde_json::to_string(exp) {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                let _ = writeln!(f, "{line}");
            }
        }
    }

    pub async fn is_empty(&self) -> bool {
        self.entries.read().await.is_empty()
    }
    pub async fn len(&self) -> usize {
        self.entries.read().await.len()
    }

    /// Store a new experience.
    /// v16.0: Also appends to the JSONL file if set_path was called.
    pub async fn append(&self, exp: Experience) -> Result<(), String> {
        self.append_to_disk(&exp);
        let n = {
            let mut entries = self.entries.write().await;
            entries.push(exp);
            entries.len()
        };
        tracing::debug!(total = n, "experience stored");
        Ok(())
    }

    /// Prune low-effectiveness experiences older than max_age_days.
    /// Returns number of entries removed.
    pub async fn prune(&self, min_effectiveness: f32, max_age_days: i64) -> usize {
        let cutoff = chrono::Utc::now().timestamp() - max_age_days * 86400;
        let mut entries = self.entries.write().await;
        let before = entries.len();
        entries.retain(|e| {
            if e.effectiveness < min_effectiveness {
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&e.created_at) {
                    return dt.timestamp() >= cutoff;
                }
            }
            true
        });
        let removed = before - entries.len();
        if removed > 0 {
            tracing::info!(removed, remaining = entries.len(), "experience pruned");
        }
        removed
    }

    /// Return experiences promoted to "core" status (high refs + effectiveness).
    pub async fn upgrade_core(&self) -> Vec<Experience> {
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| e.reference_count >= 3 && e.effectiveness >= 0.8)
            .cloned()
            .collect()
    }

    /// Compute growth metrics for observability.
    pub async fn metrics(&self) -> GrowthMetrics {
        let entries = self.entries.read().await;
        let total = entries.len() as f32;
        if total == 0.0 {
            return GrowthMetrics::default();
        }
        let reused = entries.iter().filter(|e| e.reference_count > 0).count() as f32;
        let high = entries.iter().filter(|e| e.effectiveness >= 0.8).count() as f32;
        let failures = entries.iter().filter(|e| !e.success).count() as f32;
        GrowthMetrics {
            total_experiences: entries.len() as u64,
            reuse_rate: reused / total,
            high_quality_rate: high / total,
            failure_rate: failures / total,
        }
    }
}

/// Observability metrics for the self-evolution loop.
#[derive(Debug, Clone, Default, Serialize)]
pub struct GrowthMetrics {
    pub total_experiences: u64,
    /// Fraction of experiences referenced at least once.
    pub reuse_rate: f32,
    /// Fraction with effectiveness >= 0.8.
    pub high_quality_rate: f32,
    /// Fraction of failed attempts.
    pub failure_rate: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_exp(id: &str, problem: &str, solution: &str) -> Experience {
        Experience {
            id: id.into(),
            category: "test".into(),
            problem: problem.into(),
            solution: solution.into(),
            success: true,
            effectiveness: 0.8,
            reference_count: 0,
            created_at: "2026-01-01".into(),
        }
    }

    #[tokio::test]
    async fn prune_removes_low_effectiveness_old() {
        let store = ExperienceStore::new();
        let mut old_exp = make_exp("old", "p", "s");
        old_exp.effectiveness = 0.2;
        old_exp.created_at = "2020-01-01T00:00:00+00:00".into(); // very old
        store.append(old_exp).await.unwrap();
        store.append(make_exp("good", "x", "y")).await.unwrap();
        let removed = store.prune(0.5, 365).await;
        assert_eq!(removed, 1);
        assert_eq!(store.len().await, 1);
    }

    #[tokio::test]
    async fn upgrade_core_returns_high_quality() {
        let store = ExperienceStore::new();
        let mut core = make_exp("c", "a", "b");
        core.effectiveness = 0.9;
        core.reference_count = 5;
        store.append(core).await.unwrap();
        store.append(make_exp("low", "c", "d")).await.unwrap();
        let upgraded = store.upgrade_core().await;
        assert_eq!(upgraded.len(), 1);
        assert_eq!(upgraded[0].id, "c");
    }

    #[tokio::test]
    async fn metrics_reflects_appended_entries() {
        let store = ExperienceStore::new();
        let mut used = make_exp("r1", "p", "s");
        used.effectiveness = 0.9;
        used.reference_count = 1;
        store.append(used).await.unwrap();
        store.append(make_exp("r2", "x", "y")).await.unwrap();

        let m = store.metrics().await;
        assert_eq!(m.total_experiences, 2);
        assert!(m.reuse_rate > 0.0);
        assert!(m.high_quality_rate > 0.0);
    }
}
