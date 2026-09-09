// v11.1: Experience store — real embedding + cosine similarity.
// Embedding function is injected at construction (zero deps on LLM crates).
// Falls back to keyword matching when embedding is unavailable.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

/// Async embedding function: text → vector.
pub type EmbedFn = Arc<
    dyn Fn(String) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<f32>, String>> + Send>>
        + Send
        + Sync,
>;

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
    /// v11.1: Environment context for applicability matching.
    #[serde(default)]
    pub context: ContextRef,
    #[serde(skip)]
    pub embedding: Vec<f32>,
}

/// Environment snapshot for context-aware matching.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextRef {
    pub os: String,
    pub toolchain_version: String,
}

/// Applicability verdict after environment matching.
#[derive(Debug, Clone, PartialEq)]
pub enum Applicability {
    /// Environment matches ≥ 0.8 — reuse directly.
    Reuse,
    /// Environment matches 0.4~0.8 — adapt with caution.
    Adapt,
    /// Environment matches < 0.4 — create new.
    Create,
}

/// Experience store with cosine-vector search.
pub struct ExperienceStore {
    entries: tokio::sync::RwLock<Vec<Experience>>,
    embed_fn: Option<EmbedFn>,
    /// v16.0: JSONL file path for persistence (std Mutex — only held for path read).
    file_path: std::sync::Mutex<Option<PathBuf>>,
}

impl Default for ExperienceStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ExperienceStore {
    /// Create a store without embedding (keyword-only fallback).
    pub fn new() -> Self {
        Self {
            entries: tokio::sync::RwLock::new(Vec::new()),
            embed_fn: None,
            file_path: std::sync::Mutex::new(None),
        }
    }
    /// Create a store with an embedding function for semantic search.
    pub fn with_embed_fn(embed_fn: EmbedFn) -> Self {
        Self {
            entries: tokio::sync::RwLock::new(Vec::new()),
            embed_fn: Some(embed_fn),
            file_path: std::sync::Mutex::new(None),
        }
    }

    pub fn set_embed_fn(&mut self, f: EmbedFn) {
        self.embed_fn = Some(f);
    }

    /// v16.0: Enable JSONL file persistence at the given path.
    /// Loads existing records from disk on first call.
    /// v18.0: Backfills embeddings for legacy entries without one.
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
            // v18: backfill embeddings for entries loaded without one (legacy
            // JSONL written before embedding was enabled). Sequential await —
            // startup-only, a handful of entries.
            if let Some(ref f) = self.embed_fn {
                let mut enriched: Vec<Experience> = Vec::with_capacity(loaded.len());
                for mut exp in loaded {
                    if exp.embedding.is_empty() {
                        let text = format!("{} {} {}", exp.problem, exp.solution, exp.category);
                        if let Ok(v) = f(text).await {
                            exp.embedding = v;
                        }
                    }
                    enriched.push(exp);
                }
                loaded = enriched;
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

    /// Store a new experience, computing its embedding if available.
    /// v16.0: Also appends to the JSONL file if set_path was called.
    pub async fn append(&self, mut exp: Experience) -> Result<(), String> {
        if let Some(ref f) = self.embed_fn {
            let text = format!("{} {} {}", exp.problem, exp.solution, exp.category);
            exp.embedding = f(text).await?;
        }
        self.append_to_disk(&exp);
        let n = {
            let mut entries = self.entries.write().await;
            entries.push(exp);
            entries.len()
        };
        tracing::debug!(total = n, "experience stored");
        Ok(())
    }

    pub async fn append_raw(&self, exp: Experience) {
        self.entries.write().await.push(exp);
    }

    /// Search top-K experiences, preferring cosine over keyword fallback.
    pub async fn search(&self, query: &str, top_k: usize) -> Vec<Experience> {
        let entries = self.entries.read().await;
        if entries.is_empty() {
            return vec![];
        }
        let results = if let Some(ref f) = self.embed_fn {
            match f(query.to_string()).await {
                Ok(q_vec) if !q_vec.is_empty() => {
                    let mut scored: Vec<(f32, &Experience)> = entries
                        .iter()
                        .map(|e| (cosine_similarity(&q_vec, &e.embedding), e))
                        .collect();
                    scored
                        .sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
                    let cosine_hits: Vec<(f32, &Experience)> =
                        scored.into_iter().filter(|(s, _)| *s > 0.3).collect();
                    // v18: if ALL entries are legacy (no embedding → cosine = 0),
                    // cosine filter returns empty — fall back to keyword so old
                    // experiences are still found during the migration window.
                    if cosine_hits.is_empty() {
                        keyword_scored(&query.to_lowercase(), &entries)
                    } else {
                        cosine_hits
                    }
                }
                _ => keyword_scored(&query.to_lowercase(), &entries),
            }
        } else {
            keyword_scored(&query.to_lowercase(), &entries)
        };
        let mut out: Vec<Experience> = results
            .into_iter()
            .take(top_k)
            .map(|(_, e)| e.clone())
            .collect();
        for r in &mut out {
            r.reference_count += 1;
        }
        out
    }

    /// Evaluate applicability of an experience against current context.
    pub fn evaluate(&self, exp: &Experience, current: &ContextRef) -> Applicability {
        let score = env_match_score(&exp.context, current);
        if score >= 0.8 {
            Applicability::Reuse
        } else if score >= 0.4 {
            Applicability::Adapt
        } else {
            Applicability::Create
        }
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

    /// Bump effectiveness of an experience (called when it was applied).
    pub async fn reinforce(&self, id: &str, delta: f32) {
        let mut entries = self.entries.write().await;
        if let Some(e) = entries.iter_mut().find(|e| e.id == id) {
            e.effectiveness = (e.effectiveness + delta).min(1.0);
            e.reference_count += 1;
        }
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

// ─── scoring helpers ───

fn keyword_scored<'a>(query_lower: &str, entries: &'a [Experience]) -> Vec<(f32, &'a Experience)> {
    let query_words: Vec<&str> = query_lower.split_whitespace().collect();
    if query_words.is_empty() {
        return vec![];
    }
    let mut scored: Vec<(f32, &Experience)> = entries
        .iter()
        .map(|e| {
            let text = format!(
                "{} {} {}",
                e.problem.to_lowercase(),
                e.solution.to_lowercase(),
                e.category.to_lowercase()
            );
            let hits = query_words.iter().filter(|w| text.contains(*w)).count() as f32;
            (hits / query_words.len() as f32, e)
        })
        .filter(|(s, _)| *s > 0.0)
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na * nb)
}

fn env_match_score(exp_ctx: &ContextRef, current: &ContextRef) -> f32 {
    let os_match = if exp_ctx.os == current.os { 1.0 } else { 0.3 };
    let tc_match = if exp_ctx.toolchain_version == current.toolchain_version {
        1.0
    } else {
        0.5
    };
    os_match * 0.6 + tc_match * 0.4
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_exp(id: &str, problem: &str, solution: &str, embed: Vec<f32>) -> Experience {
        Experience {
            id: id.into(),
            category: "test".into(),
            problem: problem.into(),
            solution: solution.into(),
            success: true,
            effectiveness: 0.8,
            reference_count: 0,
            created_at: "2026-01-01".into(),
            context: ContextRef::default(),
            embedding: embed,
        }
    }

    #[test]
    fn cosine_same_vector_returns_one() {
        let v = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 0.001);
    }
    #[test]
    fn cosine_orthogonal_returns_zero() {
        assert!((cosine_similarity(&[1.0, 0.0], &[0.0, 1.0])).abs() < 0.001);
    }
    #[test]
    fn cosine_mismatched_length_returns_zero() {
        assert!((cosine_similarity(&[1.0], &[1.0, 0.0])).abs() < 0.001);
    }

    #[test]
    fn env_match_same_context() {
        let ctx = ContextRef {
            os: "linux".into(),
            toolchain_version: "1.85".into(),
        };
        assert!(env_match_score(&ctx, &ctx) > 0.95);
    }
    #[test]
    fn env_match_different_os() {
        let exp = ContextRef {
            os: "linux".into(),
            toolchain_version: "1.85".into(),
        };
        let cur = ContextRef {
            os: "windows".into(),
            toolchain_version: "1.85".into(),
        };
        let s = env_match_score(&exp, &cur);
        assert!(s < 0.8, "expected lower score, got {}", s);
    }

    #[tokio::test]
    async fn search_with_keyword_fallback() {
        let store = ExperienceStore::new();
        store
            .append(make_exp("e1", "null pointer", "add guard", vec![1.0, 0.0]))
            .await
            .unwrap();
        store
            .append(make_exp("e2", "layout broken", "flexbox", vec![0.0, 1.0]))
            .await
            .unwrap();
        assert_eq!(store.len().await, 2);
        let r = store.search("null crash", 3).await;
        assert!(!r.is_empty());
        assert!(r[0].problem.contains("null"));
    }

    #[tokio::test]
    async fn append_with_embed_fn() {
        let store = ExperienceStore::with_embed_fn(Arc::new(|text| {
            let v: Vec<f32> = text
                .chars()
                .take(4)
                .map(|c| c as u32 as f32 / 100.0)
                .collect();
            Box::pin(async move { Ok(v) })
        }));
        store
            .append(make_exp("e3", "test", "test", vec![]))
            .await
            .unwrap();
        let entries = store.entries.read().await;
        assert!(!entries[0].embedding.is_empty());
    }

    #[test]
    fn evaluate_applicability() {
        let store = ExperienceStore::new();
        let exp = Experience {
            context: ContextRef {
                os: "linux".into(),
                toolchain_version: "1.85".into(),
            },
            ..make_exp("x", "a", "b", vec![])
        };
        let same = ContextRef {
            os: "linux".into(),
            toolchain_version: "1.85".into(),
        };
        assert_eq!(store.evaluate(&exp, &same), Applicability::Reuse);
        let diff = ContextRef {
            os: "windows".into(),
            toolchain_version: "1.80".into(),
        };
        assert_eq!(store.evaluate(&exp, &diff), Applicability::Create);
    }

    #[tokio::test]
    async fn prune_removes_low_effectiveness_old() {
        let store = ExperienceStore::new();
        let mut old_exp = make_exp("old", "p", "s", vec![]);
        old_exp.effectiveness = 0.2;
        old_exp.created_at = "2020-01-01T00:00:00+00:00".into(); // very old
        store.append_raw(old_exp).await;
        store.append_raw(make_exp("good", "x", "y", vec![])).await;
        let removed = store.prune(0.5, 365).await;
        assert_eq!(removed, 1);
        assert_eq!(store.len().await, 1);
    }

    #[tokio::test]
    async fn upgrade_core_returns_high_quality() {
        let store = ExperienceStore::new();
        let mut core = make_exp("c", "a", "b", vec![]);
        core.effectiveness = 0.9;
        core.reference_count = 5;
        store.append_raw(core).await;
        store.append_raw(make_exp("low", "c", "d", vec![])).await;
        let upgraded = store.upgrade_core().await;
        assert_eq!(upgraded.len(), 1);
        assert_eq!(upgraded[0].id, "c");
    }

    #[tokio::test]
    async fn reinforce_and_metrics() {
        let store = ExperienceStore::new();
        let mut e = make_exp("r1", "p", "s", vec![]);
        e.effectiveness = 0.7;
        store.append_raw(e).await;
        store.append_raw(make_exp("r2", "x", "y", vec![])).await;

        store.reinforce("r1", 0.05).await;
        store.reinforce("r1", 0.05).await;

        let m = store.metrics().await;
        assert_eq!(m.total_experiences, 2);
        assert!(m.reuse_rate > 0.0);

        // r1 should now be >= 0.8
        let entries = store.entries.read().await;
        assert!(entries.iter().find(|e| e.id == "r1").unwrap().effectiveness >= 0.8);
    }
}
