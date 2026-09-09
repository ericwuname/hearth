//! D5 (hearth-cli): Observer 素材——人类侧观察（零执行权：只记/只回放/只建议）。
//!
//! - `hearth note "..."` 落盘 `~/hearth/observer/human-<session>.jsonl`
//! - `--self` 人类侧自我标注 / `--observer-verdict n` 反审 Observer / `--mood`
//! - 红线：绝不自动改用户配置/内核（反审只落盘，供下次 run bias）

use anyhow::{Context, Result};
use serde_json::json;
use std::path::PathBuf;

/// Observer 素材根目录：`~/hearth/observer/`。
pub fn observer_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    PathBuf::from(home).join("hearth").join("observer")
}

/// `hearth note`：通用素材 + 可选标注，落盘 human 侧 jsonl。
pub fn note(
    session: Option<&str>,
    content: &str,
    self_label: Option<&str>,
    observer_verdict: Option<&str>,
    mood: Option<&str>,
) -> Result<PathBuf> {
    let dir = observer_dir();
    std::fs::create_dir_all(&dir).context("create ~/hearth/observer")?;

    let sid = session.unwrap_or("unspecified");
    let path = dir.join(format!("human-{sid}.jsonl"));
    let entry = json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "kind": "note",
        "session": sid,
        "content": content,
        // 人类侧自我标注（--self）
        "self_label": self_label,
        // 反审 Observer（--observer-verdict y/n + 理由）
        "observer_verdict": observer_verdict,
        // 情绪（--mood）
        "mood": mood,
    });
    let mut line = serde_json::to_string(&entry).context("serialize note")?;
    line.push('\n');
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .context("open human jsonl")?;
    use std::io::Write as _;
    f.write_all(line.as_bytes()).context("append note")?;

    // R7 (D5): --observer-verdict n "理由" → 同步落盘 Observer 反审（rebuttals/）。
    // 零执行权：observer::apply_rebuttal 只写记录供下次 run bias，不改规则。
    if let Some(ov) = observer_verdict {
        if !ov.is_empty() && observer::apply_rebuttal(&dir, sid, ov, content).is_err() {
            // 反审落盘失败不阻断 note 主流程（主素材已落盘），但提示
            eprintln!("⚠ 反审落盘失败（Observer 反审未记录）");
        }
    }
    Ok(path)
}

/// R6 (D5): AI 侧事件流落盘——`~/hearth/observer/<sid>.jsonl`（EnvelopedEvent 行）。
/// 直跑模式在会话结束后调用（Observer 消费 AI 事件流 + 人类行为流双侧）。
pub fn persist_ai_events(sid: &str, enveloped: &[api::EnvelopedEvent]) -> Result<PathBuf> {
    let dir = observer_dir();
    std::fs::create_dir_all(&dir).context("create ~/hearth/observer")?;
    let path = dir.join(format!("{sid}.jsonl"));
    let mut out = String::new();
    for ev in enveloped {
        if let Ok(line) = serde_json::to_string(ev) {
            out.push_str(&line);
            out.push('\n');
        }
    }
    std::fs::write(&path, out).context("write ai events jsonl")?;
    Ok(path)
}

/// 轻探（D5）: 最近一次 human 侧 note 是否有异常信号——chat 开头轻探判定。
/// 只扫最近 5 个 human-*.jsonl 的负面 mood / 反审判定。
pub fn recent_human_abnormal() -> bool {
    let dir = observer_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return false;
    };
    let mut files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("human-") && n.ends_with(".jsonl"))
                .unwrap_or(false)
        })
        .collect();
    // 按 mtime 取最近 5 个
    files.sort_by_key(|p| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });
    let recent: Vec<_> = files.iter().rev().take(5).collect();
    for p in recent {
        let Ok(text) = std::fs::read_to_string(p) else {
            continue;
        };
        for line in text.lines() {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                let mood = v.get("mood").and_then(|m| m.as_str()).unwrap_or("");
                let verdict = v
                    .get("observer_verdict")
                    .and_then(|m| m.as_str())
                    .unwrap_or("");
                if matches!(mood, "angry" | "frustrated") || verdict == "n" {
                    return true;
                }
            }
        }
    }
    false
}

/// 上次 session 是否有异常信号（重试率高 / give_up / 审批高拒批）——轻探判定。
/// 扫描 `~/hearth/observer/human-<sid>.jsonl` + 待扩展 AI 侧。
pub fn last_session_had_abnormal_signal(sid: &str) -> Result<bool> {
    let dir = observer_dir();
    let human = dir.join(format!("human-{sid}.jsonl"));
    if human.exists() {
        let text = std::fs::read_to_string(&human).unwrap_or_default();
        // 拒批/负面情绪 = 异常信号
        for line in text.lines() {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                let mood = v.get("mood").and_then(|m| m.as_str()).unwrap_or("");
                let verdict = v
                    .get("observer_verdict")
                    .and_then(|m| m.as_str())
                    .unwrap_or("");
                if matches!(mood, "angry" | "frustrated") || verdict == "n" {
                    return Ok(true);
                }
            }
        }
    }
    // 扩展点：AI 侧（~/hearth/observer/<sid>.jsonl）give_up/重试率信号
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [自检]: note 各形态落盘 + 拒批触发异常信号。
    #[test]
    fn test_note_persists_and_rebuttal_signal() {
        let dir = std::env::temp_dir().join(format!("hearth-note-test-{}", uuid::Uuid::new_v4()));
        std::env::set_var("HOME", &dir);
        let p = note(
            Some("s1"),
            "这个审批流程卡了我三次",
            Some("理解偏差"),
            Some("n 不该拒批这么多次"),
            Some("frustrated"),
        )
        .unwrap();
        assert!(p.exists(), "note 必须落盘");
        assert!(p
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("human-s1"));
        assert!(last_session_had_abnormal_signal("s1").unwrap());
        // R6: AI 侧落盘
        let ai = persist_ai_events(
            "s1",
            &[api::EnvelopedEvent {
                schema_version: 1,
                ts: "2026-08-22T00:00:00Z".into(),
                seq: 1,
                span_id: String::new(),
                parent_id: None,
                event: api::AgentEvent::Phase {
                    phase: "Plan".into(),
                },
            }],
        )
        .unwrap();
        assert!(ai.exists());
        assert!(ai.file_name().unwrap().to_string_lossy().starts_with("s1"));
        assert!(!last_session_had_abnormal_signal("s2").unwrap());
        // 清理
        let _ = std::fs::remove_dir_all(&dir);
        std::env::remove_var("HOME");
    }
}
