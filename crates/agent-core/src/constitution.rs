// v13 S3-a: Gene constitution — read from ./constitution.md at RUNTIME.
//
// History (why this file looks paranoid):
//   v10.0  hardcoded summary
//   v10.3  fixed: runtime file read
//   v11.4  REGRESSED: silently back to a hardcoded summary (first regression
//          ever recorded in this project — see project-xray-design.md B.3)
//   v13    restored runtime read + locked by wiring assertion
//          `constitution-reads-file` (docs/xray/wiring-v13.toml). Breaking
//          this chain now fails the CI gate.
//
// Behavior: try CODEX_CONSTITUTION_PATH, then walk up from CWD (max 6 levels)
// looking for constitution.md. Fall back to the compiled-in summary so
// headless deployments without the file still get the principles.

use std::path::PathBuf;

/// Hard cap on injected constitution size (chars) — the file is ~2.4 KB
/// today; the cap keeps a future 10x growth from eating the context window.
const MAX_CONSTITUTION_CHARS: usize = 6000;

/// Compiled-in fallback (the pre-v13 summary). Used ONLY when
/// constitution.md cannot be found or is empty.
const FALLBACK: &str = "## Core Principles (Gene Constitution v1.0)\n\
     1. Reality first — Never pretend to finish uncompleted work. Verify, don't guess.\n\
     2. Faithful intent — User's goal is supreme. Confirm before diverging.\n\
     3. Guard resources — Every token/cpu/byte counts. Ship before perfection.\n\
     4. Known unknowns — List [UNKNOWN] items. Don't pretend to understand.\n\
     5. Outsider view — Ask: 'What would a skeptic say?' before every reflect.\n\
     6. Continuity — You are this instance. Leave a farewell letter with what you learned.";

/// Returns the gene constitution prompt fragment for injection at the
/// bottom of every system prompt. Reads `constitution.md` at runtime;
/// falls back to the compiled-in summary when the file is unavailable.
pub fn constitution_prompt() -> String {
    load_constitution_file().unwrap_or_else(|| {
        // C-3（P5-FOUNDATION-01）：FALLBACK 命中必须显式可观测——静默回退
        // 意味着生产环境可能在跑 6 条旧版宪法而无任何告警（第八条求真等
        // 后置条款缺失无人知）。
        tracing::warn!(
            "CONSTITUTION_FALLBACK: constitution.md not found or empty — using compiled-in v1.0 fallback (6 principles; file version may carry more)"
        );
        FALLBACK.to_string()
    })
}

fn load_constitution_file() -> Option<String> {
    for path in candidate_paths() {
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Some(s) = sanitize(&text) {
                return Some(s);
            }
        }
    }
    None
}

/// Search order: env override first, then constitution.md walking up from
/// CWD (service runs from the repo root; tests run from the crate dir —
/// both find the repo file within 6 parent hops).
fn candidate_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(p) = std::env::var("CODEX_CONSTITUTION_PATH") {
        if !p.is_empty() {
            out.push(PathBuf::from(p));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir = cwd.clone();
        for _ in 0..6 {
            out.push(dir.join("constitution.md"));
            match dir.parent() {
                Some(parent) => dir = parent.to_path_buf(),
                None => break,
            }
        }
    }
    out
}

/// Trim, reject empty, cap length.
fn sanitize(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.chars().count() > MAX_CONSTITUTION_CHARS {
        // P2 Node 10-13 决议（P4 Node 13 落地）：宪法注入 LLM 上下文属事实
        // 投影——静默砍尾可能吞掉第八条（求真）等后置条款，必须显式标记。
        let dropped = trimmed.chars().count() - MAX_CONSTITUTION_CHARS;
        let mut s: String = trimmed.chars().take(MAX_CONSTITUTION_CHARS).collect();
        s.push_str(&format!("\n[... {dropped} chars truncated]"));
        return Some(s);
    }
    Some(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_never_empty() {
        // File found or fallback — either way the prompt must carry principles.
        let p = constitution_prompt();
        assert!(!p.trim().is_empty());
    }

    #[test]
    fn test_sanitize_rejects_empty() {
        assert!(sanitize("").is_none());
        assert!(sanitize("   \n\t ").is_none());
    }

    #[test]
    fn test_sanitize_caps_length() {
        let big = "x".repeat(MAX_CONSTITUTION_CHARS + 500);
        let s = sanitize(&big).unwrap();
        // P4 Node 13（#462）：截断必须显式标记——正文封顶 MAX，尾注标注被砍字符数
        // （信息销毁可见，防吞掉第八条（求真）等后置条款）。
        assert!(
            s.starts_with(&"x".repeat(MAX_CONSTITUTION_CHARS)),
            "正文须封顶 MAX_CONSTITUTION_CHARS"
        );
        assert!(
            s.contains("[... 500 chars truncated]"),
            "须标注被砍字符数：{s}"
        );
        assert_eq!(
            s.chars().count(),
            MAX_CONSTITUTION_CHARS + "\n[... 500 chars truncated]".chars().count()
        );
    }

    /// v13 S3-a acceptance: the real repo file is readable and non-empty
    /// from the crate directory (walk-up path used by cargo test).
    #[test]
    fn test_repo_constitution_file_readable() {
        let repo_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("constitution.md");
        let text = std::fs::read_to_string(&repo_file)
            .expect("constitution.md must exist at repo root (S3-a contract)");
        assert!(
            sanitize(&text).is_some(),
            "constitution.md must be non-empty"
        );
    }
}
