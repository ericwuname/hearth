//! Observer OS —— 第三权（零执行权）。
//!
//! v23 §7 铁律：
//! - **独立 crate**：禁塞进 nervous-system（后者 `NerveAction` 有干预执行权）。
//! - **零执行权**：只消费事件流（`Vec<EnvelopedEvent>`），产出 Finding / 报告 /
//!   熔断事件。不调用任何工具、不修改计划、不尝试修复。
//! - **L2 fail-closed**：`Observer::run()` 返回 `Err` → 由 **service 层**拒绝继续
//!   （内核 agent-core 不 import observer——独立 crate 铁律；fail-closed 在协调者
//!   service 落地，而非内核）。
//! - 事件流是唯一事实源（事实产生权：BE 产生事实，FE/Observer 投影）。

pub mod circuit;
pub mod metrics;
pub mod rules;

use anyhow::{anyhow, Result};
use api::EnvelopedEvent;
use std::path::Path;

/// Observer 主体——零业务状态（纯函数式消费事件流）。
#[derive(Debug, Default)]
pub struct Observer {
    /// 熔断是否已触发（熔断后不再产出新 Finding——只停）。
    tripped: bool,
}

impl Observer {
    pub fn new() -> Self {
        Self::default()
    }

    /// L1: 消费一段事件流 → 产出 Finding + 熔断事件。
    /// L2 fail-closed：事件流解析失败（不可信的流）→ Err —— 调用方（service）
    /// 必须拒绝继续执行。
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

    /// 熔断状态查询（service 决定是否停机）。
    pub fn is_tripped(&self) -> bool {
        self.tripped
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
/// **零执行权铁律**：反审只**落盘**为可审计记录（供下次 `run` 的 bias 输入），
/// **绝不自动修改** `rules.rs` 判定权重/规则/内核控制流。纠偏是"下次评估时
/// 调用方参考"而非"Observer 自改"——Observer 保持纯函数式零业务状态。
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

/// R7: 读取某 session 的全部反审记录（供下次 run 的 bias——调用方决定如何用）。
pub fn rebuttals_for(dir: &Path, session: &str) -> Vec<RebuttalRecord> {
    let path = dir.join("rebuttals").join(format!("{session}.jsonl"));
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|l| serde_json::from_str::<RebuttalRecord>(l).ok())
        .collect()
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

    /// R7 [自检]: 反审落盘 + 可读回 + 不修改规则（Observer::run 结果不变——零执行权）。
    #[test]
    fn test_r7_rebuttal_persists_without_mutating_rules() {
        let dir = std::env::temp_dir().join(format!("obs-rebuttal-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path =
            apply_rebuttal(&dir, "s1", "n", "Observer 误判——这不是异常").expect("反审必须落盘");
        assert!(path.exists());
        let recs = rebuttals_for(&dir, "s1");
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].verdict, "n");
        assert!(rebuttals_for(&dir, "s2").is_empty(), "不同 session 不串");
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
