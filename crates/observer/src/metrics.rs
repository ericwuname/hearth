//! WP-5 (v23 phase4): 确定性指标引擎——纯函数，禁 LLM。
//!
//! 四象限（top-level-plan §2.3）：
//! ① 自主度   autonomy_rate = by=="Policy" 的响应占比
//! ② 人机交互 interrupt_count / wait_ratio / clarify_skip_rate（通用原语计数，
//!    Z-16：wait 用 InteractionResponse.latency_ms 客户端采集回传——Observer 只聚合）
//! ③ 技术质量 error_rate / retry_rate（span 内 Error 事件推断——SpanClose 无 status）
//! ④ 资源效率 cost_per_step / token_efficiency（Done.report.usage——cost 不在事件流，
//!    Observer 只读事件流，不得直读 CostMeter）

use anyhow::Result;
use api::{AgentEvent, EnvelopedEvent};
use std::collections::HashMap;

/// 四象限指标——全确定性标量（无 HashMap 迭代序依赖，G3 字节级稳定）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Metrics {
    // ① 自主度
    pub interaction_count: u64,
    pub autonomy_rate: f64,
    // ② 人机交互
    pub interrupt_count: u64,
    pub wait_ratio: f64,
    pub clarify_skip_rate: f64,
    // ③ 技术质量
    pub error_rate: f64,
    pub retry_rate: f64,
    // ④ 资源效率
    pub cost_per_step: f64,
    pub token_efficiency: f64,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            interaction_count: 0,
            autonomy_rate: 0.0,
            interrupt_count: 0,
            wait_ratio: 0.0,
            clarify_skip_rate: 0.0,
            error_rate: 0.0,
            retry_rate: 0.0,
            cost_per_step: 0.0,
            token_efficiency: 0.0,
        }
    }
}

/// 单遍遍历事件流计算全部指标（确定性：纯标量累加 + HashMap 仅按键查询不迭代）。
pub fn compute(events: &[EnvelopedEvent]) -> Result<Metrics> {
    let mut m = Metrics::default();

    // 事件流时长（ts 差，秒）——wait_ratio 分母
    let first_ts = events.first().map(|e| parse_ts(&e.ts)).unwrap_or(0.0);
    let last_ts = events.last().map(|e| parse_ts(&e.ts)).unwrap_or(0.0);
    let total_secs = (last_ts - first_ts).max(0.001);

    // ② 请求侧统计（kind 由 id→kind map 匹配，Observer 不解析 payload）
    let mut req_kind: HashMap<String, String> = HashMap::new();
    let mut clarify_req: u64 = 0;
    let mut clarify_skip: u64 = 0;
    let mut latency_sum_ms: u64 = 0;
    let mut latency_count: u64 = 0;

    // ③ span 侧统计（span 内 Error 事件 → error span）
    let mut span_total: u64 = 0;
    let mut span_error: u64 = 0;
    let mut span_stack: Vec<String> = Vec::new();

    // ④ 工具统计
    let mut tool_calls: u64 = 0;
    let mut tool_errors: u64 = 0;

    // ④ 资源
    let mut steps_used: u64 = 0;
    let mut cost_total: f64 = 0.0;
    let mut tokens_total: u64 = 0;

    for ev in events {
        match &ev.event {
            // ② 交互请求
            AgentEvent::NeedApproval { approval_id, .. } => {
                req_kind.insert(approval_id.clone(), "approval".to_string());
                if true {
                    // 审批 = blocking 交互（WP-0: approval 恒 blocking=true）
                    m.interrupt_count += 1;
                }
            }
            AgentEvent::InteractionResolved {
                interaction_id,
                by,
                resolved,
                latency_ms,
            } => {
                m.interaction_count += 1;
                if by == "Policy" {
                    m.autonomy_rate += 1.0;
                }
                let kind = req_kind.get(interaction_id).cloned().unwrap_or_default();
                if kind == "clarification" {
                    clarify_req += 1;
                    if !resolved {
                        clarify_skip += 1;
                    }
                }
                if let Some(ms) = latency_ms {
                    latency_sum_ms += ms;
                    latency_count += 1;
                }
            }
            // ③ span 生命周期
            AgentEvent::SpanOpen { span_id, .. } => {
                span_total += 1;
                span_stack.push(span_id.clone());
            }
            AgentEvent::SpanClose { .. } => {
                span_stack.pop();
            }
            AgentEvent::Error { .. } => {
                if !span_stack.is_empty() {
                    span_error += 1;
                }
            }
            // ④ 工具 + 资源
            AgentEvent::ToolCall { .. } => {
                tool_calls += 1;
            }
            AgentEvent::ToolResult { is_error, .. } => {
                if *is_error {
                    tool_errors += 1;
                }
            }
            AgentEvent::Done { report } => {
                if let Some(steps) = report.get("steps").and_then(|v| v.as_u64()) {
                    steps_used = steps;
                }
                if let Some(usage) = report.get("usage") {
                    cost_total = usage.get("cost").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    tokens_total = usage.get("tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                }
            }
            _ => {}
        }
    }

    // ① 自主度
    if m.interaction_count > 0 {
        m.autonomy_rate /= m.interaction_count as f64;
    }
    // ② wait_ratio = 等待总时延 / 事件流总时长（Z-16：latency_ms 客户端采集）
    if latency_count > 0 {
        m.wait_ratio = (latency_sum_ms as f64 / 1000.0) / total_secs;
    }
    if clarify_req > 0 {
        m.clarify_skip_rate = clarify_skip as f64 / clarify_req as f64;
    }
    // ③ error_rate = error spans / total spans（span 内 Error 事件推断）
    if span_total > 0 {
        m.error_rate = span_error as f64 / span_total as f64;
    }
    // retry_rate = 工具错误 / 工具调用
    if tool_calls > 0 {
        m.retry_rate = tool_errors as f64 / tool_calls as f64;
    }
    // ④
    if steps_used > 0 {
        m.cost_per_step = cost_total / steps_used as f64;
        m.token_efficiency = tokens_total as f64 / steps_used as f64;
    }

    Ok(m)
}

/// 解析 RFC3339 ts → epoch 秒（失败返回 0——仅用于相对时长）。
fn parse_ts(ts: &str) -> f64 {
    chrono::DateTime::parse_from_rfc3339(ts)
        .map(|d| d.timestamp() as f64 + d.timestamp_subsec_millis() as f64 / 1000.0)
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(seq: u64, event: AgentEvent, ts: &str) -> EnvelopedEvent {
        EnvelopedEvent {
            schema_version: 1,
            ts: ts.into(),
            seq,
            span_id: String::new(),
            parent_id: None,
            event,
        }
    }

    #[test]
    fn test_deterministic_metrics_same_twice() {
        // G3: 同一事件流跑两次 → 字节级相同
        let events = vec![
            env(
                1,
                AgentEvent::SpanOpen {
                    span_id: "s1".into(),
                    parent_id: None,
                    name: "plan".into(),
                    t0: "t".into(),
                },
                "2026-08-03T00:00:00Z",
            ),
            env(
                2,
                AgentEvent::ToolCall {
                    call_id: "c1".into(),
                    name: "bash".into(),
                    args: serde_json::json!({}),
                },
                "2026-08-03T00:00:01Z",
            ),
            env(
                3,
                AgentEvent::ToolResult {
                    call_id: "c1".into(),
                    is_error: true,
                    output: "err".into(),
                },
                "2026-08-03T00:00:02Z",
            ),
            env(
                4,
                AgentEvent::NeedApproval {
                    approval_id: "a1".into(),
                    action: "run bash".into(),
                    payload: serde_json::Value::Null,
                },
                "2026-08-03T00:00:03Z",
            ),
            env(
                5,
                AgentEvent::InteractionResolved {
                    interaction_id: "a1".into(),
                    by: "human".into(),
                    resolved: true,
                    latency_ms: Some(500),
                },
                "2026-08-03T00:00:04Z",
            ),
            env(
                6,
                AgentEvent::SpanClose {
                    span_id: "s1".into(),
                    t1: "t".into(),
                    duration_ms: 100,
                },
                "2026-08-03T00:00:05Z",
            ),
            env(
                7,
                AgentEvent::Done {
                    report: serde_json::json!({"ok": true, "steps": 3, "usage": {"cost": 0.06, "tokens": 900}}),
                },
                "2026-08-03T00:00:06Z",
            ),
        ];
        let a = compute(&events).unwrap();
        let b = compute(&events).unwrap();
        let ja = serde_json::to_string(&a).unwrap();
        let jb = serde_json::to_string(&b).unwrap();
        assert_eq!(ja, jb, "G3: 两次计算字节级相同");
        // 语义抽查
        assert_eq!(a.interaction_count, 1, "1 次交互响应");
        assert_eq!(a.autonomy_rate, 0.0, "by=human 非 Policy");
        assert_eq!(a.interrupt_count, 1, "1 次 blocking 交互");
        assert_eq!(a.retry_rate, 1.0, "1 错 / 1 调用");
        assert_eq!(a.cost_per_step, 0.02, "0.06 / 3 步");
        eprintln!("WP-5 PASS: 确定性（两次字节一致）+ 语义正确");
    }

    #[test]
    fn test_metrics_autonomy_and_wait() {
        let events = vec![
            env(
                1,
                AgentEvent::NeedApproval {
                    approval_id: "a1".into(),
                    action: "x".into(),
                    payload: serde_json::Value::Null,
                },
                "2026-08-03T00:00:00Z",
            ),
            env(
                2,
                AgentEvent::InteractionResolved {
                    interaction_id: "a1".into(),
                    by: "Policy".into(),
                    resolved: true,
                    latency_ms: Some(1000),
                },
                "2026-08-03T00:00:10Z",
            ),
        ];
        let m = compute(&events).unwrap();
        assert_eq!(m.autonomy_rate, 1.0, "by=Policy → 100% 自主");
        assert!(
            (m.wait_ratio - 0.1).abs() < 1e-9,
            "wait=1s/总10s → 0.1, got {}",
            m.wait_ratio
        );
        eprintln!("WP-5 PASS: autonomy_rate + wait_ratio（latency_ms 聚合，Z-16）");
    }

    #[test]
    fn test_metrics_no_llm_dependency() {
        // 门禁：observer crate 内不得出现 llm/chat/generate（禁 LLM 铁律）
        // 只检查非测试代码（测试自身含 banned 词会误报）
        let lib = include_str!("lib.rs");
        let prod = include_str!("metrics.rs")
            .split("mod tests")
            .next()
            .unwrap_or("");
        let src = lib.to_string() + prod;
        for banned in ["llm_gateway", "LlmProvider", "chat("] {
            assert!(!src.contains(banned), "observer 内禁 LLM: {banned}");
        }
        eprintln!("WP-5 PASS: observer 禁 LLM（grep 0 命中）");
    }
}
