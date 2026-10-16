//! WP-7 (v23 phase4): G0 红线熔断——Observer 只产 `CircuitBreak` 事件（表达事实）。
//! **只拉闸**：不改计划、不换工具、不尝试修复。
//! 可审计：原因 + 触发规则 + 时间戳全部入事件。
//!
//! D-208（2026-10-08, traecode）：**已接线（同轮收口该债）**。
//! Observer **仍只产事实**（`CircuitBreak`）——**置态由执行方落**：唯一调用点
//! `agent-runtime/src/session.rs`（会话收尾）**消费** `run_and_report` 的返回值，经
//! `red_line_terminal_phase` 做 **fail-closed 置终态**——安全级红线（`sandbox_violation`：
//! 越权/安全事实）把 `done` 翻成 `blocked`；`retry_storm`/`budget_exhausted` **不翻**
//! （技术/经济类，其终态已由 loop 层表达，再翻只会制造假告警）。breaks 同时写进持久化
//! `done_summary`（审计面，`GET /events` 可查）。
//! 边界（与 D-45 一致）：Observer **评估失败**恒 **fail-open**（调用点只 warn、不改态）。
//! 行为门禁：`crates/service/tests/circuit_break_fail_closed_gate.rs`（越权读 → 终态 `blocked`）。

use crate::metrics::Metrics;
use crate::CircuitBreak;
use anyhow::Result;
use api::{AgentEvent, EnvelopedEvent};

/// 熔断评估：预算耗尽 / sandbox 违规 / 重试风暴 → CircuitBreak 事件。
pub fn evaluate(metrics: &Metrics, events: &[EnvelopedEvent]) -> Result<Vec<CircuitBreak>> {
    let mut breaks = Vec::new();

    // 1. sandbox / 路径越界违规 → 只拉闸，不尝试修复。
    //
    // D-207（2026-10-08, traecode）：原匹配串为 `SANDBOX`+`DENIED`（或 `PATH`+`OUTSIDE`+`SCOPE`），
    // **无任何真实生产者会产出**——它只匹配本文件自造的单测串（"DENIED: path … outside sandbox
    // scope"）⇒ G0 红线 `sandbox_violation` 在**生产里永不触发**，而单测却绿（"声称≠实现/假绿"族）。
    //
    // 真实事实源＝ agent 自身**确定性**的路径护栏拒绝串（`tools-builtin` 的 read/edit/patch/
    // grep/glob 与 `agent-runtime::resolve_in_workspace` 产出，经 `ToolResult.output` 进事件流）。
    // **有意不匹配** OS 级 `permission denied`/`operation not permitted`：那会出现在任意良性
    // 命令输出里（如 `cat /root/x`），把良性失败误判成 G0 红线。
    // 防复发：`tests/sandbox_violation_matcher_gate.rs`（正向：真实串必触发；反向：OS 拒绝不触发）。
    const GUARD_DENIALS: [&str; 4] = [
        "path traversal denied",
        "path outside allowed roots",
        "path outside workspace denied",
        "path traversal via symlink/junction denied",
    ];
    for ev in events {
        if let AgentEvent::ToolResult { output, .. } = &ev.event {
            let low = output.to_lowercase();
            if GUARD_DENIALS.iter().any(|pat| low.contains(pat)) {
                let reason = output.chars().take(120).collect::<String>();
                breaks.push(CircuitBreak::new(
                    format!("sandbox violation: {reason}"),
                    "sandbox_violation",
                ));
                break;
            }
        }
    }

    // 2. 重试风暴（retry_rate > 0.8 —— G0 红线）
    if metrics.retry_rate > 0.8 {
        breaks.push(CircuitBreak::new(
            format!("retry storm: retry_rate={:.2}", metrics.retry_rate),
            "retry_storm",
        ));
    }

    // 3. 预算耗尽（Done.report.status == "budget_exhausted"）
    for ev in events {
        if let AgentEvent::Done { report } = &ev.event {
            if report
                .get("status")
                .and_then(|v| v.as_str())
                .is_some_and(|s| s.contains("budget"))
            {
                breaks.push(CircuitBreak::new(
                    "budget exhausted — agent stopped by budget gate",
                    "budget_exhausted",
                ));
                break;
            }
        }
    }

    Ok(breaks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(seq: u64, event: AgentEvent) -> EnvelopedEvent {
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
    fn test_circuit_sandbox_violation() {
        // D-207：用**真实生产者**串（tools-builtin 护栏）——旧测试自造
        // "DENIED: path … outside sandbox scope" 恰好命中旧匹配串，掩盖了"生产里永不触发"。
        let events = vec![env(
            1,
            AgentEvent::ToolResult {
                call_id: "c1".into(),
                is_error: true,
                output: "path outside allowed roots (workspace/home) denied: /etc/passwd \
                         (workspace: /tmp/ws)"
                    .into(),
            },
        )];
        let m = Metrics::default();
        let breaks = evaluate(&m, &events).unwrap();
        assert_eq!(breaks.len(), 1, "sandbox 违规必熔断");
        assert_eq!(breaks[0].rule, "sandbox_violation");
        assert!(!breaks[0].ts.is_empty(), "时间戳可审计");
        eprintln!("WP-7 PASS: sandbox 违规 → CircuitBreak（只拉闸）");
    }

    #[test]
    fn test_circuit_budget_exhausted() {
        let events = vec![env(
            1,
            AgentEvent::Done {
                report: serde_json::json!({"ok": false, "status": "budget_exhausted"}),
            },
        )];
        let m = Metrics::default();
        let breaks = evaluate(&m, &events).unwrap();
        assert!(
            breaks.iter().any(|b| b.rule == "budget_exhausted"),
            "预算耗尽必熔断: {:?}",
            breaks
        );
        eprintln!("WP-7 PASS: 预算耗尽 → CircuitBreak");
    }

    #[test]
    fn test_circuit_clean_stream_no_break() {
        let events = vec![env(
            1,
            AgentEvent::Done {
                report: serde_json::json!({"ok": true, "steps": 2}),
            },
        )];
        let m = Metrics::default();
        let breaks = evaluate(&m, &events).unwrap();
        assert!(breaks.is_empty(), "干净流不熔断");
        eprintln!("WP-7 PASS: 干净流不熔断（只对红线触发）");
    }
}
