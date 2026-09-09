//! P1-9 (v0.2.4): 结构化执行报告——每轮 run 结束生成人类可读 Markdown 摘要。
//!
//! 背景（手工实测用户原话）：测试效率瓶颈是"发现问题→反馈问题→修复问题"周期太长，
//! 每次都要重新翻几千行终端记录定位问题。此报告让用户/审查 AI **不翻原始终端 log**
//! 就能判断这一轮任务是否正常。
//!
//! 位置：`<HEARTH_REPORTS_DIR | <cwd>/.hearth/reports>/<session_id>/<run_seq>.md`
//! 内容：目标/状态/步数/耗时/token/工具调用序列（含成败）/写盘文件/审批/反思/错误要点。
//! 落盘失败不阻断主流程（warn + 继续）——报告是增益不是依赖。

use anyhow::{Context, Result};
use std::path::PathBuf;

/// 报告根目录。
pub fn reports_dir(cwd: &std::path::Path) -> PathBuf {
    std::env::var("HEARTH_REPORTS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| cwd.join(".hearth").join("reports"))
}

/// 单轮执行报告素材（从 run_local 事件流与 report 汇总）。
pub struct RunReportInput<'a> {
    pub session_id: &'a str,
    pub goal: &'a str,
    /// G1-01 (v0.2.5): 九态终态（terminal::normalize 产物——completed/failed/
    /// aborted/cancelled/deadline_exceeded/...）。
    pub status: &'a str,
    pub ok: bool,
    pub steps: u64,
    pub wall_secs: u64,
    pub usage_line: String,
    /// (工具名, 参数摘要, 是否失败)——保持执行顺序。
    pub tool_calls: Vec<(String, String, bool)>,
    /// 写盘文件清单（Artifact 事件）。
    pub written_files: Vec<String>,
    /// 审批请求与结果。
    pub approvals: Vec<String>,
    /// 反思 verdict 序列（Continue/Replan/GiveUp…）。
    pub reflections: Vec<String>,
    /// ERROR 级日志/事件要点（截断防膨胀）。
    pub errors: Vec<String>,
    /// G1-03: 未完成时的剩余工作/建议下一步（completed 时为空）。
    pub remaining_work: Vec<String>,
    /// R2-D (批示 5 + 补充 4, v0.2.7): 验收验证范围——"none"=criteria 空
    /// （artifact 级验证，绝不等价语义完成）/ "pending"=criteria 非空未确认 /
    /// "passed"|"failed"（criteria 通道落地后）。**criteria 空时恒 "none"，
    /// 绝不写 passed**（防"文件存在"被误报为语义完成）。
    pub acceptance_verification: &'a str,
    /// Node 05 (N-3): 会话级审批委托投影——(delegated, 委托命令清单)。
    /// 仅修 projection（Markdown/审计可见），ApprovalPolicy/delegation 语义零改动。
    pub approval_delegated: (bool, Vec<String>),
}

/// 从 EnvelopedEvent 流提取 (工具名, 参数摘要, 失败) 序列。
pub fn collect_tool_call_details(enveloped: &[api::EnvelopedEvent]) -> Vec<(String, String, bool)> {
    // ToolCall 事件记录 (name, args 摘要)；ToolResult 按 call_id 配对成败
    let mut calls: Vec<(String, String, bool)> = Vec::new();
    for ev in enveloped {
        match &ev.event {
            api::AgentEvent::ToolCall {
                name,
                args,
                call_id,
                ..
            } => {
                let arg_summary = summarize_args(args);
                // 找后续同 call_id 的结果（事件流顺序——先记 pending，结果稍后修正）
                calls.push((name.clone(), format!("#{call_id} {arg_summary}"), false));
            }
            api::AgentEvent::ToolResult {
                call_id, is_error, ..
            } => {
                // 修正对应调用（call_id 前缀匹配——事件可能无结果配对）
                if let Some(c) = calls
                    .iter_mut()
                    .rev()
                    .find(|(_, s, _)| s.contains(&format!("#{call_id} ")))
                {
                    c.2 = *is_error;
                }
            }
            _ => {}
        }
    }
    calls
}

/// 参数摘要：path/cmd 取首个关键值，截 80 字符。
fn summarize_args(args: &serde_json::Value) -> String {
    for key in ["path", "cmd", "pattern", "query", "url"] {
        if let Some(v) = args.get(key).and_then(|v| v.as_str()) {
            let s: String = v.chars().take(80).collect();
            return format!("{key}={s}");
        }
    }
    String::new()
}

/// G1-03 (v0.2.5): 从 task_graph 提取未完成节点（remaining work）。
/// agent 崩溃/无图时返回空——调用方兜底通用指引。
pub fn collect_remaining_from_graph(graph: &serde_json::Value) -> Vec<String> {
    let Some(nodes) = graph.get("nodes").and_then(|n| n.as_array()) else {
        return Vec::new();
    };
    nodes
        .iter()
        .filter(|n| {
            n["status"]
                .as_str()
                .map(|s| s != "Completed")
                .unwrap_or(true)
        })
        .filter_map(|n| n["title"].as_str().map(String::from))
        .collect()
}

/// 从事件流提取审批与反思记录。
pub fn collect_approvals_and_reflections(
    enveloped: &[api::EnvelopedEvent],
) -> (Vec<String>, Vec<String>) {
    let mut approvals = Vec::new();
    let mut reflections = Vec::new();
    for ev in enveloped {
        match &ev.event {
            api::AgentEvent::NeedApproval {
                action, payload, ..
            } => {
                let from = payload.get("from").and_then(|f| f.as_str()).unwrap_or("?");
                approvals.push(format!("{action}（发起: {from}）"));
            }
            api::AgentEvent::Reflection { verdict } => {
                reflections.push(verdict.clone());
            }
            _ => {}
        }
    }
    (approvals, reflections)
}

/// 从事件流提取 ERROR 要点（每条截 200 字符，最多 10 条）。
pub fn collect_errors(enveloped: &[api::EnvelopedEvent]) -> Vec<String> {
    let mut errors = Vec::new();
    for ev in enveloped {
        if let api::AgentEvent::Error { message } = &ev.event {
            let s: String = message.chars().take(200).collect();
            if !errors.iter().any(|e| e == &s) {
                errors.push(s);
            }
            if errors.len() >= 10 {
                break;
            }
        }
    }
    errors
}

/// 生成并写入一轮的报告 Markdown。返回写入路径。
pub fn write_run_report(cwd: &std::path::Path, input: &RunReportInput) -> Result<PathBuf> {
    let dir = reports_dir(cwd).join(input.session_id);
    std::fs::create_dir_all(&dir).context("create reports dir")?;
    // run_seq = 目录里已有报告数 + 1（同一会话多轮递增）
    let seq = std::fs::read_dir(&dir)
        .map(|rd| rd.filter_map(|e| e.ok()).count())
        .unwrap_or(0)
        + 1;
    let path = dir.join(format!("run-{seq:03}.md"));

    let mut md = String::new();
    md.push_str("# Hearth 执行报告\n\n");
    // G1-01: 终态九态 + 图标投影（✓/✗/⏸/⏱）
    let (icon, label) = match input.status {
        "completed" => ("✅", "已完成"),
        "cancelled" => ("⏸", "已取消"),
        "deadline_exceeded" => ("⏱", "超时终止"),
        "aborted" => ("⛔", "异常中止"),
        "failed" => ("❌", "失败"),
        other => {
            if input.ok {
                ("✅", other)
            } else {
                ("❌", other)
            }
        }
    };
    md.push_str(&format!(
        "- **会话**: `{}`\n- **时间**: {}\n- **终态**: {icon} {label}（{}；steps={}）\n\n",
        input.session_id,
        input.status,
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        input.steps,
    ));
    md.push_str("## 目标\n\n> ");
    md.push_str(input.goal);
    md.push_str("\n\n");

    md.push_str("## 消耗\n\n");
    md.push_str(&format!(
        "- 耗时: {}s | 步数: {} | {}\n\n",
        input.wall_secs, input.steps, input.usage_line
    ));

    // R2-D (批示 5): verification scope 显式区分——artifact 级 ≠ 语义级
    md.push_str("## 验证范围\n\n");
    md.push_str(&format!(
        "- artifact verification: {}\n- acceptance verification: **{}**（{}）\n\n",
        if input.ok {
            "passed（写盘产物存在且非空）"
        } else {
            "not passed"
        },
        input.acceptance_verification,
        if input.acceptance_verification == "none" {
            "未指定验收标准——artifact 通过不代表任务语义已满足"
        } else {
            "验收标准已在 Task Continuity 注入"
        }
    ));

    if !input.tool_calls.is_empty() {
        md.push_str("## 工具调用\n\n| # | 工具 | 参数 | 结果 |\n|---|---|---|---|\n");
        for (i, (name, args, failed)) in input.tool_calls.iter().enumerate() {
            md.push_str(&format!(
                "| {} | `{}` | `{}` | {} |\n",
                i + 1,
                name,
                args.replace('|', "\\|"),
                if *failed { "❌ 失败" } else { "✓" }
            ));
        }
        md.push('\n');
    }

    // Node 05 (N-3): 审批委托投影——delegated 状态与命令清单完整可见
    // （false/true+1 条/true+多条/空清单 四象限均有一致呈现）。
    {
        let (delegated, cmds) = &input.approval_delegated;
        md.push_str("## 审批委托\n\n");
        if *delegated && !cmds.is_empty() {
            md.push_str(&format!(
                "- 状态: **已委托**（会话级，{} 条命令自动放行）\n",
                cmds.len()
            ));
            for c in cmds {
                md.push_str(&format!("  - `{}`\n", c.replace('|', "\\|")));
            }
        } else if *delegated {
            md.push_str("- 状态: **已委托**（会话级）\n- 本轮无委托放行记录\n");
        } else {
            md.push_str("- 状态: 未委托\n");
        }
        md.push('\n');
    }

    if !input.written_files.is_empty() {
        md.push_str("## 产物文件\n\n");
        for f in &input.written_files {
            md.push_str(&format!("- `{}`\n", f));
        }
        md.push('\n');
    }

    if !input.approvals.is_empty() {
        md.push_str("## 审批\n\n");
        for a in &input.approvals {
            md.push_str(&format!("- {}\n", a));
        }
        md.push('\n');
    }

    if !input.reflections.is_empty() {
        md.push_str("## 反思轨迹\n\n");
        md.push_str(&format!("{}\n\n", input.reflections.join(" → ")));
    }

    if !input.errors.is_empty() {
        md.push_str("## 错误要点\n\n");
        for e in &input.errors {
            md.push_str(&format!("- {}\n", e));
        }
        md.push('\n');
    }

    // G1-03: 剩余工作——未完成时必填（Completion Summary 标准：还剩什么/下一步）
    if !input.remaining_work.is_empty() {
        md.push_str("## 剩余工作\n\n");
        for w in &input.remaining_work {
            md.push_str(&format!("- {}\n", w));
        }
        md.push('\n');
    }

    std::fs::write(&path, md).context("write report")?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_written_and_complete() {
        let dir = std::env::temp_dir().join(format!("hearth_rep_{}", uuid::Uuid::new_v4()));
        let input = RunReportInput {
            session_id: "sid-test",
            goal: "把 bug 记录写到 BUG_LEDGER.md",
            status: "failed",
            ok: false,
            steps: 6,
            wall_secs: 120,
            usage_line: "tokens: ↑3000 ↓500".to_string(),
            tool_calls: vec![
                ("bash".into(), "#c1 cmd=cargo build".into(), true),
                ("write_file".into(), "#c2 path=BUG_LEDGER.md".into(), false),
            ],
            written_files: vec!["BUG_LEDGER.md".into()],
            approvals: vec!["approval（发起: act）".into()],
            reflections: vec!["Replan".into(), "GiveUp".into()],
            errors: vec!["bash: tool 'bash' timed out after 30s".into()],
            remaining_work: vec!["继续修复 cargo build 失败（直接下指令即可）".into()],
            acceptance_verification: "none",
            approval_delegated: (false, vec![]),
        };
        let path = write_run_report(&dir, &input).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        // 关键字段齐全（负面：缺字段 = 用户还得翻原始 log）
        assert!(content.contains("❌ 失败"), "终态图标必须可见（G1-03）");
        assert!(content.contains("failed"), "九态原文须在（G1-01）");
        assert!(content.contains("BUG_LEDGER.md"), "目标与产物须在");
        assert!(content.contains("timed out"), "错误要点须在");
        assert!(content.contains("Replan → GiveUp"), "反思轨迹须在");
        assert!(
            content.contains("剩余工作"),
            "未完成必须含剩余工作段（G1-03）"
        );
        assert!(content.contains("❌ 失败"), "失败工具调用须标红");
        // 同会话第二轮序号递增
        let path2 = write_run_report(&dir, &input).unwrap();
        assert!(path2.to_string_lossy().contains("run-002"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// G1-01: 终态图标投影——completed/deadline/cancelled/aborted 各有专名。
    #[test]
    fn test_report_terminal_state_labels() {
        let dir = std::env::temp_dir().join(format!("hearth_rep_{}", uuid::Uuid::new_v4()));
        for (status, expect) in [
            ("completed", "✅ 已完成"),
            ("deadline_exceeded", "⏱ 超时终止"),
            ("cancelled", "⏸ 已取消"),
            ("aborted", "⛔ 异常中止"),
        ] {
            let input = RunReportInput {
                session_id: "sid-label",
                goal: "g",
                status,
                ok: status == "completed",
                steps: 1,
                wall_secs: 1,
                usage_line: String::new(),
                tool_calls: vec![],
                written_files: vec![],
                approvals: vec![],
                reflections: vec![],
                errors: vec![],
                remaining_work: vec![],
                acceptance_verification: "none",
                approval_delegated: (false, vec![]),
            };
            let path = write_run_report(&dir, &input).unwrap();
            let content = std::fs::read_to_string(&path).unwrap();
            assert!(
                content.contains(expect),
                "status={status} 须投影为 {expect}: {content}"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
