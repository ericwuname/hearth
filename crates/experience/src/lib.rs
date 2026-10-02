// v16.0: Experience store — JSONL 持久化 + 剪枝 + 计数（**只写审计档**）。
//
// D-100 → D-107（2026-10-02, traecode）**裁决 + 收口**：本 crate 的"复用回路"已断——
// 唯一会 `reference_count += 1` 的 `search()` 随 D-47 删除，`agent-core` 的注入点
// `injected_experience` 也随 D-9 失去生产者（每 run/step 恒复位 None）。据此按 D-72
// 先例**退役"复用"接口**：删掉 `reference_count`（无任何写入方的字段）、
// `reuse_rate`（对外暴露的结构性 0 指标）与 `upgrade_core()`（恒空的查询），
// 保留**只写审计档**本体（追加 / 剪枝 / 真实计数）。
//
// **为何不按"接线"处理**（联网核实 + 本项目实测，2026-10-02）：
//   · Reflexion 范式（Shinn et al. 2023）与本仓 v17（弱模型 +20pt）都表明"失败教训回灌"
//     有效，但**必须**配三道前置：① 只回灌**失败**教训；② 质量过滤 + **有界窗口**
//     （业界明确：低质反思"浪费上下文并损害后续尝试"）；③ **来源加权**——
//     arXiv 2605.18930（OEP，2026-05）实证：自进化 agent 的"局部正确但不可迁移"经验会被
//     蒸馏成过度泛化规则、显著抬高下游失败率（GPT-4o 上 ASR >50%）；
//   · 本仓 v18 实测：全局注入**害强模型**（−5pt）——与业界"不做无门控全局注入"一致；
//   · 注入文本源自既往 run（可能含 web 抓取内容），属**间接提示注入面**，须按 D-80 纪律
//     带"来源 + 权威序"标注。
// ⇒ 真正的复用是**需先设计 + 基准验证**的课题（驱动文档 D-100 行已登记裁决），
//    不靠恢复一个已删的 `search()` 草率接线。
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Experience entry with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub id: String,
    pub category: String,
    pub problem: String,
    pub solution: String,
    pub success: bool,
    pub effectiveness: f32,
    pub created_at: String,
}

/// D-70（2026-10-01）：经验库 JSONL 单文件**读入上限**（与 memory 的 MEM-3 同值
/// 64 MiB）。
///
/// 该文件是**只增追加**的（每次 run 收尾追加一条，见 `agent-core` 的 condense
/// 块），长期运行可无限增长；`set_path` 此前用 `tokio::fs::read_to_string` 把
/// **整份文件**读进内存（无界读入新落点）——服务重启、文件已涨到 GB 级时即 OOM。
/// 现最多保留前 `MAX_EXPERIENCE_FILE_BYTES` 字节并**留痕**（不静默）。
const MAX_EXPERIENCE_FILE_BYTES: u64 = 64 * 1024 * 1024;

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

    /// D-70：有界读入经验库文件，返回 `(内容, 是否被截断)`。
    ///
    /// 走共享原语 [`bounded_io::read_file_text_capped`]——排空到 EOF 但只保留前
    /// `cap` 字节，截断点退到合法 UTF-8 边界；截断**必须留痕**（不静默）。
    /// `cap` 显式传入以便单测用小上限验证截断路径（无需造 64 MiB 文件）。
    async fn read_file_bounded(path: &Path, cap: u64) -> std::io::Result<(String, bool)> {
        let (text, truncated) = bounded_io::read_file_text_capped(path, cap).await?;
        if truncated {
            tracing::warn!(
                path = %path.display(),
                cap,
                "experience 文件超过上限，已截断读取——尾部（较新）条目不会加载"
            );
        }
        Ok((text, truncated))
    }

    /// v16.0: Enable JSONL file persistence at the given path.
    /// Loads existing records from disk on first call.
    /// Safe to call on `&self` (uses std::sync::Mutex for interior mutability).
    pub async fn set_path(&self, path: PathBuf) -> std::io::Result<()> {
        // Load existing records if the file exists.
        if path.exists() {
            // D-70：有界读入（此前为无界的 `tokio::fs::read_to_string`）。
            let (content, _) = Self::read_file_bounded(&path, MAX_EXPERIENCE_FILE_BYTES).await?;
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

    /// Compute growth metrics for observability.
    ///
    /// D-107：三项指标都取自**真实写入的事实**（条数 / 高质量率 / 失败率）；
    /// 原先的 `reuse_rate` 已删除——它读的 `reference_count` 无任何写入方，
    /// 对外恒报 0（拿"结构性 0"当业务指标比不报更糟）。
    pub async fn metrics(&self) -> GrowthMetrics {
        let entries = self.entries.read().await;
        let total = entries.len() as f32;
        if total == 0.0 {
            return GrowthMetrics::default();
        }
        let high = entries.iter().filter(|e| e.effectiveness >= 0.8).count() as f32;
        let failures = entries.iter().filter(|e| !e.success).count() as f32;
        GrowthMetrics {
            total_experiences: entries.len() as u64,
            high_quality_rate: high / total,
            failure_rate: failures / total,
        }
    }
}

/// Observability metrics for the self-evolution loop.
#[derive(Debug, Clone, Default, Serialize)]
pub struct GrowthMetrics {
    pub total_experiences: u64,
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
            created_at: "2026-01-01".into(),
        }
    }

    /// D-70 回归锁（先红后绿）。
    ///
    /// 红侧：修复前 `set_path` 用 `tokio::fs::read_to_string` 把**整份文件**读进
    /// 内存——保留量等于文件全长（此处 4096+100 字节）。
    /// 绿侧：保留量必须被 `cap` 钳住，且必须**报告截断**（不静默）。
    #[tokio::test]
    async fn read_file_bounded_caps_and_reports_truncation() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("experience.jsonl");
        std::fs::write(&p, "x".repeat(4096 + 100)).unwrap();

        let (text, truncated) = ExperienceStore::read_file_bounded(&p, 4096).await.unwrap();
        assert_eq!(
            text.len(),
            4096,
            "保留量必须被 cap 钳住（修复前等于整份文件长度）"
        );
        assert!(truncated, "超限必须报告截断（不静默）");

        // 边界：恰好等于上限 → 不算截断。
        let exact = dir.path().join("exact.jsonl");
        std::fs::write(&exact, "y".repeat(4096)).unwrap();
        let (text, truncated) = ExperienceStore::read_file_bounded(&exact, 4096)
            .await
            .unwrap();
        assert_eq!(text.len(), 4096);
        assert!(!truncated, "恰好等于上限不算截断");
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

    // D-107：`upgrade_core_returns_high_quality` 已随该接口一并删除
    // （它依赖无写入方的 `reference_count`，恒空；保留测试等于给死接口续命）。

    #[tokio::test]
    async fn metrics_reflects_appended_entries() {
        let store = ExperienceStore::new();
        let mut good = make_exp("r1", "p", "s");
        good.effectiveness = 0.9;
        store.append(good).await.unwrap();
        store.append(make_exp("r2", "x", "y")).await.unwrap();

        let m = store.metrics().await;
        assert_eq!(m.total_experiences, 2);
        assert!(m.high_quality_rate > 0.0);
        // D-107：失败率取自真实 `success` 字段（两条都 success=true ⇒ 0）。
        assert_eq!(m.failure_rate, 0.0);
    }
}
