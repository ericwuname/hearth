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
        // D-219：④ 两指标可为**未知**（`None` = 无用量 / 成本未定价）——未知**不参与**
        // 阈值判定（不得把"未知"当 0 判，同 D-176「未知 ≠ 0」）。
        let value = match rule.metric.as_str() {
            "autonomy_rate" => metrics.autonomy_rate,
            "interrupt_count" => metrics.interrupt_count as f64,
            "wait_ratio" => metrics.wait_ratio,
            "clarify_skip_rate" => metrics.clarify_skip_rate,
            "error_rate" => metrics.error_rate,
            "retry_rate" => metrics.retry_rate,
            "cost_per_step" => match metrics.cost_per_step {
                Some(v) => v,
                None => continue,
            },
            "token_efficiency" => match metrics.token_efficiency {
                Some(v) => v,
                None => continue,
            },
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
    // D-221/D-219（2026-10-08, traecode）：④ 两指标**可为未知**（`None` = 无用量 /
    // 成本未定价）——**给人看**的报告**不得**把"未知"渲染成数值（会被读成真实测值，
    // 同 D-176「未知 ≠ 0」）；已知时如实出数。report.json 保留 `null`（机器契约）。
    for (name, v) in [
        ("cost_per_step", metrics.cost_per_step),
        ("token_efficiency", metrics.token_efficiency),
    ] {
        match v {
            Some(x) => out.push_str(&format!("| {name} | {x:.4} |\n")),
            None => out.push_str(&format!(
                "| {name} | n/a（未知：无用量或成本未定价；见 metrics ④） |\n"
            )),
        }
    }
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
        // 高成本事件流 → cost_per_step 超阈值 → Finding 带可复现 evidence。
        //
        // D-219：此处已改用**内核真实形状**（`usage` = `CostEntry` 键 + `cost_usd`）——
        // 不再是"自造 payload 掩盖生产不可达"（D-218 旧态）；该规则现**生产可达**
        // （内核 `Done` 统一携带资源事实）。
        let events = vec![EnvelopedEvent {
            schema_version: 1,
            ts: "2026-08-03T00:00:00Z".into(),
            seq: 1,
            span_id: String::new(),
            parent_id: None,
            event: api::AgentEvent::Done {
                report: serde_json::json!({
                    "ok": true, "steps": 1,
                    "usage": {"prompt_tokens": 400, "completion_tokens": 600, "total_tokens": 1000, "calls": 1},
                    "cost_usd": 9.9
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

        // D-219 对照：**成本未定价**（`cost_usd: null`）⇒ 成本**未知**，规则**不得**触发
        // （不得把"未知"当 0 之外的任何阈值判定；同 D-176）。
        let unknown = vec![EnvelopedEvent {
            schema_version: 1,
            ts: "2026-08-03T00:00:00Z".into(),
            seq: 1,
            span_id: String::new(),
            parent_id: None,
            event: api::AgentEvent::Done {
                report: serde_json::json!({
                    "ok": true, "steps": 1,
                    "usage": {"total_tokens": 1000, "calls": 1},
                    "cost_usd": null
                }),
            },
        }];
        let mu = metrics::compute(&unknown).unwrap();
        assert_eq!(mu.cost_per_step, None, "未定价 ⇒ 成本未知");
        let fu = evaluate(&mu, &unknown).unwrap();
        assert!(
            !fu.iter().any(|f| f.rule == "cost_per_step_high"),
            "成本未知时 `cost_per_step_high` 不得触发（不能拿未知当数值判）"
        );
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

    /// D-221（2026-10-08, traecode）：**给人看**的报告不得把"无数据源的结构性 0"
    /// 渲染成 `0.0000`——那会被读者读成"成本/步 = 0"（D-176/D-218 同族：未知 ≠ 0）。
    /// ④ 两指标当前**无任何可达数据源**（生产恒 0，见 `metrics.rs` 模块头 ④），
    /// 报告应如实标 `n/a`。
    #[test]
    fn test_d221_unavailable_metrics_render_as_na_not_zero() {
        let md = render_md(&Metrics::default(), &[], &[]);
        assert!(
            !md.contains("| cost_per_step | 0.0000 |"),
            "D-221：无数据源的 cost_per_step 不得渲染成 0.0000（会被读成真实测值）: {md}"
        );
        assert!(
            !md.contains("| token_efficiency | 0.0000 |"),
            "D-221：无数据源的 token_efficiency 不得渲染成 0.0000: {md}"
        );
        assert!(
            md.contains("cost_per_step | n/a") && md.contains("token_efficiency | n/a"),
            "D-221：无数据源指标须如实标 n/a: {md}"
        );
    }

    #[test]
    fn test_v24_quarterly_all_rules_trigger() {
        // v24 季度体检：构造阈值越界事件流 → 5 条规则（observer-rules.toml）全触发
        // 事件流：高成本（cost=9.9/1步）→ cost_per_step_high；by=human → autonomy_low；
        // 工具错误 → error_rate_high + retry_storm_high；长等待 → wait_high
        //
        // D-218：`cost_per_step_high` 的触发**仅因**下方 Done 用了**自造（合成）payload**
        // （`usage:{cost,tokens}`）——生产 `Done` payload 不含 `usage` ⇒ 该规则**生产永不触发**。
        // 故本测证明的是"其余 4 条规则在真实事件流下可达 + 引擎对第 5 条亦能工作"，
        // **不**等于"5 条规则在生产全可达"（旧注释"v24 维护合同：规则全触发"对第 5 条**不成立**）。
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
                        // D-219：内核真实形状（usage 用 CostEntry 键；cost_usd 为可空 USD）
                        "usage": {"prompt_tokens": 40, "completion_tokens": 60, "total_tokens": 100, "calls": 1},
                        "cost_usd": 9.9
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
