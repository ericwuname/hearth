//! WP-6 (v23 phase4): 规则引擎与 Finding——阈值配置 + Metrics → Finding。
//! L3 证据强制由 `Finding::new` 保证（空 evidence 构造失败）。
//! 双重报告：report.md（给人）+ report.json（给机器）。

use crate::metrics::Metrics;
use crate::Finding;
use anyhow::Result;
use api::EnvelopedEvent;

/// 规则配置（observer-rules.toml）——阈值表。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RuleDef {
    pub name: String,
    pub metric: String,
    pub op: String, // ">" | "<" | ">=" | "<="
    pub threshold: f64,
    pub severity: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RulesConfig {
    pub rules: Vec<RuleDef>,
}

impl RulesConfig {
    /// 加载内置规则（resources/observer-rules.toml）。
    pub fn load() -> Result<Self> {
        let text = include_str!("../resources/observer-rules.toml");
        Ok(toml::from_str(text)?)
    }
}

/// 规则评估：Metrics + 事件流 → Findings（确定性：规则按配置顺序遍历）。
pub fn evaluate(metrics: &Metrics, _events: &[EnvelopedEvent]) -> Result<Vec<Finding>> {
    let config = RulesConfig::load()?;
    let mut findings = Vec::new();
    for rule in &config.rules {
        let value = match rule.metric.as_str() {
            "autonomy_rate" => metrics.autonomy_rate,
            "interrupt_count" => metrics.interrupt_count as f64,
            "wait_ratio" => metrics.wait_ratio,
            "clarify_skip_rate" => metrics.clarify_skip_rate,
            "error_rate" => metrics.error_rate,
            "retry_rate" => metrics.retry_rate,
            "cost_per_step" => metrics.cost_per_step,
            "token_efficiency" => metrics.token_efficiency,
            other => {
                return Err(anyhow::anyhow!("unknown metric in rule: {other}"));
            }
        };
        let hit = match rule.op.as_str() {
            ">" => value > rule.threshold,
            "<" => value < rule.threshold,
            ">=" => value >= rule.threshold,
            "<=" => value <= rule.threshold,
            other => {
                return Err(anyhow::anyhow!("unknown op in rule: {other}"));
            }
        };
        if hit {
            // L3: evidence 必填——格式化指标实测值（凭 evidence 可复现结论）
            findings.push(Finding::new(
                rule.name.clone(),
                rule.severity.clone(),
                format!(
                    "{} = {:.4} {} {}",
                    rule.metric, value, rule.op, rule.threshold
                ),
                rule.recommendation.clone(),
            )?);
        }
    }
    Ok(findings)
}

/// 双重报告：markdown（给人）。
pub fn render_md(
    metrics: &Metrics,
    findings: &[Finding],
    breaks: &[crate::CircuitBreak],
) -> String {
    let mut out = String::from("# Observer Report\n\n");
    out.push_str("## Metrics\n\n");
    out.push_str("| metric | value |\n|---|---|\n");
    out.push_str(&format!(
        "| interaction_count | {} |\n",
        metrics.interaction_count
    ));
    out.push_str(&format!(
        "| autonomy_rate | {:.4} |\n",
        metrics.autonomy_rate
    ));
    out.push_str(&format!(
        "| interrupt_count | {} |\n",
        metrics.interrupt_count
    ));
    out.push_str(&format!("| wait_ratio | {:.4} |\n", metrics.wait_ratio));
    out.push_str(&format!(
        "| clarify_skip_rate | {:.4} |\n",
        metrics.clarify_skip_rate
    ));
    out.push_str(&format!("| error_rate | {:.4} |\n", metrics.error_rate));
    out.push_str(&format!("| retry_rate | {:.4} |\n", metrics.retry_rate));
    out.push_str(&format!(
        "| cost_per_step | {:.4} |\n",
        metrics.cost_per_step
    ));
    out.push_str(&format!(
        "| token_efficiency | {:.4} |\n",
        metrics.token_efficiency
    ));
    out.push_str("\n## Findings\n\n");
    if findings.is_empty() {
        out.push_str("_none_\n");
    }
    for f in findings {
        out.push_str(&format!(
            "- **{}** [{}] — {}\n  - evidence: `{}`\n  - 建议: {}\n",
            f.rule, f.severity, f.recommendation, f.evidence, f.recommendation
        ));
    }
    out.push_str("\n## Circuit Breaks\n\n");
    if breaks.is_empty() {
        out.push_str("_none_\n");
    }
    for b in breaks {
        out.push_str(&format!("- **{}** [{}] at {}\n", b.rule, b.reason, b.ts));
    }
    out
}

/// 双重报告：json（给机器）。
pub fn render_json(
    metrics: &Metrics,
    findings: &[Finding],
    breaks: &[crate::CircuitBreak],
) -> String {
    serde_json::json!({
        "metrics": metrics,
        "findings": findings,
        "circuit_breaks": breaks,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics;

    #[test]
    fn test_rules_config_parses() {
        let config = RulesConfig::load().unwrap();
        assert!(!config.rules.is_empty(), "规则表不得为空");
        eprintln!(
            "WP-6 PASS: observer-rules.toml 可解析（{} 条规则）",
            config.rules.len()
        );
    }

    #[test]
    fn test_finding_evidence_reproducible() {
        // 高成本事件流 → cost_per_step 超阈值 → Finding 带可复现 evidence
        let events = vec![EnvelopedEvent {
            schema_version: 1,
            ts: "2026-08-03T00:00:00Z".into(),
            seq: 1,
            span_id: String::new(),
            parent_id: None,
            event: api::AgentEvent::Done {
                report: serde_json::json!({
                    "ok": true, "steps": 1,
                    "usage": {"cost": 9.9, "tokens": 1000}
                }),
            },
        }];
        let m = metrics::compute(&events).unwrap();
        let findings = evaluate(&m, &events).unwrap();
        let cost = findings.iter().find(|f| f.rule == "cost_per_step_high");
        assert!(cost.is_some(), "cost_per_step=9.9 必触发阈值");
        let f = cost.unwrap();
        assert!(
            f.evidence.contains("cost_per_step = 9.9000"),
            "evidence 凭值可复现: {}",
            f.evidence
        );
        eprintln!("WP-6 PASS: Finding 带可复现 evidence");
    }

    #[test]
    fn test_double_report_formats() {
        let m = Metrics::default();
        let md = render_md(&m, &[], &[]);
        let json = render_json(&m, &[], &[]);
        assert!(md.contains("# Observer Report"));
        assert!(json.contains("\"metrics\""));
        eprintln!("WP-6 PASS: 双重报告（md + json）");
    }

    #[test]
    fn test_v24_quarterly_all_rules_trigger() {
        // v24 季度体检：构造阈值越界事件流 → 5 条规则（observer-rules.toml）全触发
        // 事件流：高成本（cost=9.9/1步）→ cost_per_step_high；by=human → autonomy_low；
        // 工具错误 → error_rate_high + retry_storm_high；长等待 → wait_high
        let events = vec![
            EnvelopedEvent {
                schema_version: 1,
                ts: "2026-08-04T00:00:00Z".into(),
                seq: 1,
                span_id: String::new(),
                parent_id: None,
                event: api::AgentEvent::SpanOpen {
                    span_id: "s1".into(),
                    parent_id: None,
                    name: "act".into(),
                    t0: "t0".into(),
                },
            },
            EnvelopedEvent {
                schema_version: 1,
                ts: "2026-08-04T00:00:01Z".into(),
                seq: 2,
                span_id: String::new(),
                parent_id: None,
                event: api::AgentEvent::ToolCall {
                    call_id: "c1".into(),
                    name: "bash".into(),
                    args: serde_json::json!({}),
                },
            },
            EnvelopedEvent {
                schema_version: 1,
                ts: "2026-08-04T00:00:02Z".into(),
                seq: 3,
                span_id: String::new(),
                parent_id: None,
                event: api::AgentEvent::ToolResult {
                    call_id: "c1".into(),
                    is_error: true,
                    output: "boom".into(),
                },
            },
            EnvelopedEvent {
                schema_version: 1,
                ts: "2026-08-04T00:00:03Z".into(),
                seq: 4,
                span_id: String::new(),
                parent_id: None,
                event: api::AgentEvent::Error {
                    message: "tool boom".into(),
                },
            },
            EnvelopedEvent {
                schema_version: 1,
                ts: "2026-08-04T00:00:04Z".into(),
                seq: 5,
                span_id: String::new(),
                parent_id: None,
                event: api::AgentEvent::NeedApproval {
                    approval_id: "a1".into(),
                    action: "run".into(),
                    payload: serde_json::Value::Null,
                },
            },
            EnvelopedEvent {
                schema_version: 1,
                ts: "2026-08-04T00:00:15Z".into(),
                seq: 7,
                span_id: String::new(),
                parent_id: None,
                event: api::AgentEvent::InteractionResolved {
                    interaction_id: "a1".into(),
                    by: "human".into(),
                    resolved: true,
                    latency_ms: Some(10_000),
                },
            },
            EnvelopedEvent {
                schema_version: 1,
                ts: "2026-08-04T00:00:14Z".into(),
                seq: 6,
                span_id: String::new(),
                parent_id: None,
                event: api::AgentEvent::Done {
                    report: serde_json::json!({
                        "ok": true, "steps": 1,
                        "usage": {"cost": 9.9, "tokens": 100}
                    }),
                },
            },
        ];
        let m = metrics::compute(&events).unwrap();
        let findings = evaluate(&m, &events).unwrap();
        let triggered: Vec<&str> = findings.iter().map(|f| f.rule.as_str()).collect();
        // 5 条规则必须全触发（v24 维护合同：Observer OS 规则全触发）
        for rule in [
            "cost_per_step_high",
            "autonomy_low",
            "error_rate_high",
            "retry_storm_high",
            "wait_high",
        ] {
            assert!(
                triggered.contains(&rule),
                "规则 {rule} 必须触发，实际: {:?}",
                triggered
            );
        }
        // L3: 全部 Finding 带非空 evidence（凭值可复现）
        for f in &findings {
            assert!(
                !f.evidence.trim().is_empty(),
                "Finding {} 必须带 evidence",
                f.rule
            );
        }
        eprintln!(
            "v24 季度体检 PASS: Observer 5 规则全触发（{} 条 Finding 全带 evidence）",
            findings.len()
        );
    }
}
