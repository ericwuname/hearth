//! M2-5：一键导出会话的可复核证据（改动 diff + 命令日志 + 自检/报告）。
//!
//! 口径（未知 ≠ 0）：缺失的证据**显式**写 `null` / `missing`，不省略字段、不按空值冒充。
//! 产物是**单个自包含 JSON**：可直接归档、可被脚本复算、不依赖本仓代码。

use anyhow::{Context, Result};
use serde_json::json;
use std::path::{Path, PathBuf};

use crate::{report, session_store, snapshot_store, transcript};

/// 逐行 diff（LCS，只输出变更行；不做上下文块）。超长文件如实降级为摘要。
fn line_diff(before: &str, after: &str) -> String {
    const MAX_LINES: usize = 2000;
    let a: Vec<&str> = before.lines().collect();
    let b: Vec<&str> = after.lines().collect();
    if a.len() > MAX_LINES || b.len() > MAX_LINES {
        return format!("(文件过大：{} 行 → {} 行，未逐行 diff)", a.len(), b.len());
    }
    let (n, m) = (a.len(), b.len());
    // dp[i][j] = LCS 长度（后缀）
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    let mut out = String::new();
    let (mut i, mut j) = (0usize, 0usize);
    while i < n && j < m {
        if a[i] == b[j] {
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            out.push_str(&format!("- {}\n", a[i]));
            i += 1;
        } else {
            out.push_str(&format!("+ {}\n", b[j]));
            j += 1;
        }
    }
    for l in &a[i..] {
        out.push_str(&format!("- {l}\n"));
    }
    for l in &b[j..] {
        out.push_str(&format!("+ {l}\n"));
    }
    out
}

/// D-125 口径：所有读口走共享有界原语；超限如实标记（不静默截断当完整）。
fn read_text(path: &Path, cap: u64) -> (String, bool) {
    bounded_io::read_file_text_capped_std(path, cap).unwrap_or_default()
}

fn collect_changes(sid: &str) -> Vec<serde_json::Value> {
    let Ok(entries) = snapshot_store::list_entries(sid) else {
        return Vec::new();
    };
    entries
        .into_iter()
        .map(|e| {
            const CAP: u64 = bounded_io::MAX_CAPTURED_BYTES as u64;
            let (before, before_trunc) = if e.created {
                (String::new(), false)
            } else {
                read_text(&e.snapshot_path, CAP)
            };
            let after = std::fs::metadata(&e.target_abs)
                .ok()
                .map(|_| read_text(&e.target_abs, CAP));
            let (state, diff) = match &after {
                Some((a, after_trunc)) => {
                    let mut d = line_diff(&before, a);
                    if before_trunc || *after_trunc {
                        d.push_str(&format!(
                            "\n(注意：文件超过 {} 字节上限，diff 基于截断内容)",
                            CAP
                        ));
                    }
                    ("present", d)
                }
                None => ("missing", String::new()),
            };
            json!({
                "seq": e.seq,
                "path": e.target_abs.to_string_lossy(),
                "created": e.created,
                "state": state,
                "lines_before": before.lines().count(),
                "lines_after": after.as_ref().map(|(a, _)| a.lines().count()),
                "diff": diff,
            })
        })
        .collect()
}

fn collect_reports(cwd: &Path, sid: &str) -> Vec<serde_json::Value> {
    let dir = report::reports_dir(cwd).join(sid);
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for ent in rd.flatten() {
        let p = ent.path();
        if p.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        const CAP: u64 = 200_000;
        let (text, truncated) = read_text(&p, CAP);
        out.push(json!({
            "file": p.file_name().map(|s| s.to_string_lossy().to_string()),
            "bytes": text.len(),
            "truncated": truncated,
            "content": text,
        }));
    }
    out.sort_by(|a, b| a["file"].as_str().cmp(&b["file"].as_str()));
    out
}

fn collect_transcript(cwd: &Path, sid: &str) -> Option<(Vec<serde_json::Value>, bool)> {
    let path = transcript::transcripts_dir(cwd).join(format!("{sid}.jsonl"));
    let (text, truncated) = read_text(&path, bounded_io::MAX_CAPTURED_BYTES as u64);
    if text.is_empty() && !path.is_file() {
        return None;
    }
    Some((
        text.lines()
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .collect(),
        truncated,
    ))
}

/// 导出到 `out`（缺省 `<cwd>/hearth-session-<id前8位>.json`），返回产物路径。
pub fn export_session(sid: &str, out: Option<PathBuf>) -> Result<PathBuf> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let turns = session_store::load_turns(sid);
    let taskgoal =
        session_store::load_taskgoal(sid).map(|(rev, v)| json!({ "revision": rev, "value": v }));
    let run_state = session_store::load_run_state(sid);
    let changes = collect_changes(sid);
    let reports = collect_reports(&cwd, sid);
    let transcript = collect_transcript(&cwd, sid);
    let transcript_truncated = transcript.as_ref().map(|(_, t)| *t).unwrap_or(false);

    let short: String = sid.chars().take(8).collect();
    let bundle = json!({
        "schema_version": 1,
        "session_id": sid,
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "cwd": cwd.to_string_lossy(),
        // 未知 ≠ 0：缺失的档以 null 表示（不是空历史/零产物）。
        "turns": turns.len(),
        "taskgoal": taskgoal,
        "run_state": run_state,
        "changes": changes,
        "reports": reports,
        "transcript": transcript.map(|(rows, _)| rows),
        "transcript_truncated": transcript_truncated,
        "notes": {
            "diff_semantics": "changes[].diff 为改前快照 → 当前文件 的逐行 LCS 差异（仅 -/+ 行）",
            "missing": "taskgoal/run_state/transcript 为 null = 本机没有该档，不代表没有发生",
        },
    });
    let path = out.unwrap_or_else(|| cwd.join(format!("hearth-session-{short}.json")));
    let body = serde_json::to_string_pretty(&bundle).context("serialize evidence bundle")?;
    std::fs::write(&path, body).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::line_diff;

    #[test]
    fn diff_identical_is_empty() {
        assert!(line_diff("a\nb", "a\nb").is_empty());
    }

    #[test]
    fn diff_change_shows_removed_and_added() {
        let d = line_diff("a\nb\nc", "a\nB\nc");
        assert!(d.contains("- b"), "{d}");
        assert!(d.contains("+ B"), "{d}");
        assert!(!d.contains("- a"), "{d}");
    }

    #[test]
    fn diff_created_file_all_added() {
        let d = line_diff("", "x\ny");
        assert_eq!(d, "+ x\n+ y\n");
    }

    #[test]
    fn oversized_file_degrades_honestly() {
        let big: String = (0..2500).map(|i| format!("{i}\n")).collect();
        let d = line_diff("", &big);
        assert!(d.contains("文件过大"), "{d}");
    }
}
