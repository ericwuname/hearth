//! 最小 transcript 落盘（hearth-harness-review-supplement 补充5）。
//!
//! 每个 run 结束追加一行 JSONL 到 `<HEARTH_TRANSCRIPTS_DIR | <cwd>/results/transcripts>/<sid>.jsonl`：
//! `{ts, session_id, goal, budget, steps, ok, status, wall_secs, tool_calls, written_files}`
//!
//! 价值：验证层 fail 时有据可查——agent 到底写了什么、为什么 fail（无复盘则验证层白做）。
//! 与"平台化可观测性"不同：这是单人复盘自己失败的最小前提，属稳定输出必要项。
//! 落盘失败不阻断主流程（warn + 继续）。

use anyhow::{Context, Result};
use serde_json::json;
use std::path::PathBuf;

/// transcript 目录：`$HEARTH_TRANSCRIPTS_DIR` 或 `<cwd>/results/transcripts/`。
pub fn transcripts_dir(cwd: &std::path::Path) -> PathBuf {
    std::env::var("HEARTH_TRANSCRIPTS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| cwd.join("results").join("transcripts"))
}

/// 单个 run 的摘要记录（run 结束时调用一次）。
/// 参数多但职责单一（汇总落盘）——clippy too_many_arguments 白名单。
#[allow(clippy::too_many_arguments)]
pub fn record_run(
    cwd: &std::path::Path,
    session_id: &str,
    goal: &str,
    budget: u64,
    started_at: std::time::Instant,
    steps: u64,
    ok: bool,
    status: &str,
    tool_calls: &[String],
    written_files: &[String],
) -> Result<PathBuf> {
    let dir = transcripts_dir(cwd);
    std::fs::create_dir_all(&dir).context("create transcripts dir")?;
    let path = dir.join(format!("{session_id}.jsonl"));
    let entry = json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "session_id": session_id,
        "goal": goal,
        "budget": budget,
        "steps": steps,
        "ok": ok,
        "status": status,
        "wall_secs": started_at.elapsed().as_secs_f64().round() as u64,
        "tool_calls": tool_calls,
        "written_files": written_files,
    });
    let mut line = serde_json::to_string(&entry).context("serialize transcript")?;
    line.push('\n');
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .context("open transcript jsonl")?;
    use std::io::Write as _;
    f.write_all(line.as_bytes()).context("append transcript")?;
    Ok(path)
}

/// 从 EnvelopedEvent 流统计工具调用序列（去重保留顺序）。
pub fn collect_tool_calls(enveloped: &[api::EnvelopedEvent]) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for ev in enveloped {
        if let api::AgentEvent::ToolCall { name, .. } = &ev.event {
            let key = name.to_string();
            if seen.insert(key.clone()) {
                out.push(key);
            }
        }
    }
    out
}

/// 从 EnvelopedEvent 流统计写盘文件清单。
pub fn collect_written_files(enveloped: &[api::EnvelopedEvent]) -> Vec<String> {
    let mut out = Vec::new();
    for ev in enveloped {
        if let api::AgentEvent::Artifact { path, .. } = &ev.event {
            out.push(path.clone());
        }
    }
    out
}
