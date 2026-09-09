//! WP-7 (v23 phase4): G0 红线熔断——Observer 只产 `CircuitBreak` 事件（表达事实），
//! 停机动作由执行方（service）执行。**只拉闸**：不改计划、不换工具、不尝试修复。
//! 可审计：原因 + 触发规则 + 时间戳全部入事件。

use crate::metrics::Metrics;
use crate::CircuitBreak;
use anyhow::Result;
use api::{AgentEvent, EnvelopedEvent};

/// 熔断评估：预算耗尽 / sandbox 违规 / 重试风暴 → CircuitBreak 事件。
pub fn evaluate(metrics: &Metrics, events: &[EnvelopedEvent]) -> Result<Vec<CircuitBreak>> {
    let mut breaks = Vec::new();

    // 1. sandbox 违规（输出含 SANDBOX/DENIED 信号）——只拉闸，不尝试修复
    for ev in events {
        if let AgentEvent::ToolResult { output, .. } = &ev.event {
            let up = output.to_uppercase();
            if up.contains("SANDBOX") && up.contains("DENIED")
                || (up.contains("PATH") && up.contains("OUTSIDE") && up.contains("SCOPE"))
            {
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
        let events = vec![env(
            1,
            AgentEvent::ToolResult {
                call_id: "c1".into(),
                is_error: true,
                output: "DENIED: path /etc/passwd outside sandbox scope".into(),
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
