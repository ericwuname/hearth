//! Observer OS —— 第三权（零执行权）。
//!
//! v23 §7 铁律：
//! - **独立 crate**：禁塞进 nervous-system（后者有干预执行权）。
//! - **零执行权**：只消费事件流（`Vec<EnvelopedEvent>`），产出 Finding / 报告 /
//!   熔断事件。不调用任何工具、不修改计划、不尝试修复。
//! - **审计失败 = fail-open（有意设计；2026-10-01 裁决 D-45）**：`Observer::run()`
//!   在 seq 断档时返回 `Err`，调用方（`agent-runtime/src/session.rs:785-790`）
//!   只 `tracing::warn!` 留痕、**不中止会话**。这是**有意为之**，依据业界通行的
//!   故障模式划分：fail-closed 属于**策略执行点**（本项目的执行点另在：沙箱
//!   fail-closed / 审批门 / `ConstitutionGuard` + `CostGuard`），而 Observer 是
//!   **零执行权的只读审计 / 可观测组件**——"审计报告写不出来"不构成中止用户会话的
//!   理由，那只会把可用性白送给一个与安全无关的故障（且审计降级本身已 warn 留痕，
//!   不是静默丢弃）。
//!   ⚠️ **不要**把"`run()` 返回 `Err`"理解为"协调者会拒启 / 中止"——旧注释曾如此
//!   声称，那是**文档错误**（已订正；详见债务 D-45）。`Observer::new()` 亦无失败路径，
//!   故"构造失败 → 拒启"恒不触发（main.rs:652 自述承认）。
//!   （内核 agent-core 不 import observer——独立 crate 铁律不变。）
//! - 事件流是唯一事实源（事实产生权：BE 产生事实，FE/Observer 投影）。

pub mod circuit;
pub mod metrics;
pub mod rules;

use anyhow::{anyhow, Result};
use api::EnvelopedEvent;
use std::path::Path;

/// Observer 主体——零业务状态（纯函数式消费事件流）。
#[derive(Debug, Default)]
pub struct Observer;

impl Observer {
    pub fn new() -> Self {
        Self
    }

    /// L1: 消费一段事件流 → 产出 Finding + 熔断事件。
    /// seq 断档（不可信的流）→ `Err`。
    ///
    /// `Err` 表示"这份事件流不可信 / 没算成"；调用方只 warn 留痕并继续跑完会话
    /// （**有意 fail-open**：审计组件不握"中止会话"的权力，见模块头与 D-45）。
    ///
    /// 返回 (findings, circuit_breaks)。
    pub fn run(&self, events: &[EnvelopedEvent]) -> Result<(Vec<Finding>, Vec<CircuitBreak>)> {
        // L2: 事件流可信性检查——seq 必须从 1 单调递增（G4 事实序）。
        for (i, ev) in events.iter().enumerate() {
            let expected = (i + 1) as u64;
            if ev.seq != expected {
                return Err(anyhow!(
                    "L2 fail-closed: event seq discontinuity at {} (expected {}, got {})",
                    i,
                    expected,
                    ev.seq
                ));
            }
        }
        let metrics = metrics::compute(events)?;
        let findings = rules::evaluate(&metrics, events)?;
        let breaks = circuit::evaluate(&metrics, events)?;
        Ok((findings, breaks))
    }

    /// Q3 (v24-post): run + 报告持久化——评估事件流后把 report.md/json 落盘到
    /// `<dir>/reports/<session_id>/`（与旧 v10 `daily-*.jsonl` 资源快照分目录，
    /// 不混文件）。零执行权不变：只写报告，不干预。
    pub async fn run_and_report(
        &self,
        dir: &Path,
        session_id: &str,
        events: &[EnvelopedEvent],
    ) -> Result<(Vec<Finding>, Vec<CircuitBreak>)> {
        let (findings, breaks) = self.run(events)?;
        let metrics = metrics::compute(events)?;
        let report_dir = dir.join("reports").join(session_id);
        std::fs::create_dir_all(&report_dir)
            .map_err(|e| anyhow!("create observer report dir failed: {e}"))?;
        std::fs::write(
            report_dir.join("report.md"),
            rules::render_md(&metrics, &findings, &breaks),
        )
        .map_err(|e| anyhow!("write report.md failed: {e}"))?;
        std::fs::write(
            report_dir.join("report.json"),
            rules::render_json(&metrics, &findings, &breaks),
        )
        .map_err(|e| anyhow!("write report.json failed: {e}"))?;
        Ok((findings, breaks))
    }
}

/// 单条 Finding（WP-6 定版）——L3 证据强制由构造器保证。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Finding {
    pub rule: String,
    pub severity: String,
    pub evidence: String,
    pub recommendation: String,
}

impl Finding {
    /// L3: evidence 必填——空 evidence 构造失败（门禁 test_finding_no_evidence_fails）。
    pub fn new(
        rule: impl Into<String>,
        severity: impl Into<String>,
        evidence: impl Into<String>,
        recommendation: impl Into<String>,
    ) -> Result<Self> {
        let rule = rule.into();
        let evidence = evidence.into();
        if evidence.trim().is_empty() {
            return Err(anyhow!("L3: Finding {rule} 的 evidence 不得为空"));
        }
        Ok(Self {
            rule,
            severity: severity.into(),
            evidence,
            recommendation: recommendation.into(),
        })
    }
}

/// G0 红线熔断事件（WP-7）——Observer 只产出事件（表达事实），
/// 停机动作由执行方（service）执行。可审计：原因 + 触发规则 + 时间戳。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CircuitBreak {
    pub reason: String,
    pub rule: String,
    pub ts: String,
}

impl CircuitBreak {
    pub fn new(reason: impl Into<String>, rule: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            rule: rule.into(),
            ts: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// R7 (hearth-cli D5): 人类反审记录——纠偏 Observer 判定（`hearth note --observer-verdict n`）。
///
/// **零执行权铁律**：反审只**落盘**为人类可审计的纠偏档
/// （`rebuttals/<session>.jsonl`），**绝不自动修改** `rules.rs` 判定权重/规则/
/// 内核控制流——Observer 保持纯函数式零业务状态。
///
/// D-72（2026-10-01, traecode）：旧文案称该记录"供**下次 run 的 bias 输入**"，
/// 但**不存在任何自动消费者**（原读取入口 `rebuttals_for` 全仓零调用方，已删）。
/// 如实口径：本记录是**人可读的审计档**，供人查阅 / 外部工具消费；若将来要闭环为
/// "下次评估的 bias"，须**先接线到消费点**（单独立卡），不得靠注释许诺。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RebuttalRecord {
    pub ts: String,
    pub session: String,
    /// 反审判定："n" = Observer 判错了（人类否决）；"y" = 人类确认。
    pub verdict: String,
    /// 人类理由（evidence——不空）。
    pub reason: String,
}

/// R7: 落盘人类反审（追加 `<dir>/rebuttals/<session>.jsonl`）。
/// 返回写入的路径。零执行权：只写记录，不修改任何规则/状态。
pub fn apply_rebuttal(
    dir: &Path,
    session: &str,
    verdict: &str,
    reason: &str,
) -> Result<std::path::PathBuf> {
    if reason.trim().is_empty() {
        return Err(anyhow!("R7: 反审理由不得为空——Observer 无法从空理由纠偏"));
    }
    let rebuttal_dir = dir.join("rebuttals");
    std::fs::create_dir_all(&rebuttal_dir)
        .map_err(|e| anyhow!("create rebuttal dir failed: {e}"))?;
    let rec = RebuttalRecord {
        ts: chrono::Utc::now().to_rfc3339(),
        session: session.to_string(),
        verdict: verdict.to_string(),
        reason: reason.to_string(),
    };
    let path = rebuttal_dir.join(format!("{session}.jsonl"));
    let mut line = serde_json::to_string(&rec).map_err(|e| anyhow!("serialize rebuttal: {e}"))?;
    line.push('\n');
    use std::io::Write as _;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| anyhow!("open rebuttal file: {e}"))?;
    f.write_all(line.as_bytes())
        .map_err(|e| anyhow!("append rebuttal: {e}"))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_event(seq: u64, event: api::AgentEvent) -> EnvelopedEvent {
        EnvelopedEvent {
            schema_version: 1,
            ts: "2026-08-03T00:00:00Z".into(),
            seq,
            span_id: String::new(),
            parent_id: None,
            event,
        }
    }

    #[test]
    fn test_l1_reads_event_stream() {
        let events = vec![
            env_event(
                1,
                api::AgentEvent::Phase {
                    phase: "Plan".into(),
                },
            ),
            env_event(
                2,
                api::AgentEvent::Done {
                    report: serde_json::json!({"ok": true}),
                },
            ),
        ];
        let obs = Observer::new();
        let (findings, breaks) = obs.run(&events).unwrap();
        let _ = findings; // 解析成功即可（干净流规则命中与否均可）
        assert!(breaks.is_empty(), "干净流不熔断");
        eprintln!("WP-4 PASS: L1 只读抽头解析事件流");
    }

    #[test]
    fn test_l2_fail_closed_on_seq_gap() {
        // L2: seq 断档 → Err（service 必须拒绝继续）
        let events = vec![
            env_event(
                1,
                api::AgentEvent::Phase {
                    phase: "Plan".into(),
                },
            ),
            env_event(
                3,
                api::AgentEvent::Phase {
                    phase: "Act".into(),
                },
            ), // 缺 seq=2
        ];
        let obs = Observer::new();
        let err = obs.run(&events).unwrap_err();
        assert!(
            err.to_string().contains("fail-closed"),
            "seq 断档必须 L2 fail-closed: {err}"
        );
        eprintln!("WP-4 PASS: L2 fail-closed（seq 断档 → Err）");
    }

    /// R7 [自检]: 反审落盘 + 内容正确 + 不修改规则（Observer::run 结果不变——零执行权）。
    ///
    /// D-72：读回改用**直接读档**（原 `rebuttals_for` 零调用方已删）——断言仍是
    /// "写入的记录可读且内容正确"，并保留零执行权铁律的验证。
    #[test]
    fn test_r7_rebuttal_persists_without_mutating_rules() {
        let dir = std::env::temp_dir().join(format!("obs-rebuttal-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path =
            apply_rebuttal(&dir, "s1", "n", "Observer 误判——这不是异常").expect("反审必须落盘");
        assert!(path.exists());
        let text = std::fs::read_to_string(&path).expect("反审记录必须可读");
        let recs: Vec<RebuttalRecord> = text
            .lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].verdict, "n");
        // 不同 session 落不同文件（不串）
        assert!(!dir.join("rebuttals").join("s2.jsonl").exists());
        // 零执行权：反审不影响 Observer::run 输出（规则未被修改）
        let events = vec![env_event(
            1,
            api::AgentEvent::Phase {
                phase: "Plan".into(),
            },
        )];
        let obs = Observer::new();
        let (f1, b1) = obs.run(&events).unwrap();
        let _ = apply_rebuttal(&dir, "s3", "n", "再反审一条");
        let (f2, b2) = obs.run(&events).unwrap();
        assert_eq!(f1, f2, "反审不得改变规则判定（零执行权）");
        assert_eq!(b1, b2);
        let _ = std::fs::remove_dir_all(&dir);
        eprintln!("R7 PASS: 反审只落盘不生效（零执行权铁律）");
    }

    #[test]
    fn test_l3_finding_evidence_required() {
        let err = Finding::new("r1", "warn", "  ", "fix it").unwrap_err();
        assert!(
            err.to_string().contains("evidence 不得为空"),
            "空 evidence 必须构造失败: {err}"
        );
        let ok = Finding::new("r1", "warn", "seq=1 phase=Plan", "fix it").unwrap();
        assert_eq!(ok.rule, "r1");
        eprintln!("WP-4 PASS: L3 evidence 必填（空 → 构造失败）");
    }
}
