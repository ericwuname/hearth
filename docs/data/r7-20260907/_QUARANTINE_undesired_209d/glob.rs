use anyhow::{anyhow, Result};
use async_trait::async_trait;
use sandbox::{Sandbox, SandboxConfig};
use std::sync::Arc;
use tool_runtime::{Tool, ToolContext, ToolDescription};

pub struct GlobTool {
    // RT4: 重写为 Rust 遍历后 sandbox 不再用于 spawn find；保留字段（with_sandbox 测试用）
    #[allow(dead_code)]
    sandbox: Arc<dyn Sandbox>,
}

impl GlobTool {
    pub fn new() -> Self {
        Self {
            // RT3 修复（hearth-cli 直跑实测）：default sandbox writable 为空 → find
            // 遍历后 fchdir 恢复 cwd 被 landlock 拦（"Failed to restore initial working
            // directory: Operation not permitted"）。与 BashTool 一致用 build-tools
            // profile（cwd/CARGO_HOME/tmp writable）——glob 只读遍历不受影响。
            sandbox: Arc::from(sandbox::create_sandbox(SandboxConfig::for_build_tools())),
        }
    }

    pub fn with_sandbox(sandbox: Arc<dyn Sandbox>) -> Self {
        Self { sandbox }
    }
}

impl Default for GlobTool {
    fn default() -> Self {
        Self::new()
    }
}

/// RT4: readonly + 防逃逸递归遍历（symlink 不跟随——防穿透出 cwd 到可写挂载）。
fn walk_readonly(
    dir: &std::path::Path,
    rel: &str,
    pattern: &str,
    out: &mut Vec<String>,
    depth: usize,
) {
    if depth > 64 {
        return; // 防深递归/循环
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        // symlink_metadata：不跟随符号链接（防 symlink 逃逸 + 防循环）
        let md = match std::fs::symlink_metadata(entry.path()) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let name = entry.file_name().to_string_lossy().to_string();
        let rel_path = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        if md.is_dir() {
            walk_readonly(&entry.path(), &rel_path, pattern, out, depth + 1);
        } else if md.is_file() && glob_match(pattern, &rel_path) {
            out.push(rel_path);
        }
    }
}

/// RT4: 最小 glob 匹配——支持 `*`（段内）、`**`（跨段）、`?`（单字符）。
/// 路径相对 cwd（如 `*.rs`、`src/**/*.rs`）。
fn glob_match(pattern: &str, path: &str) -> bool {
    let pats: Vec<&str> = pattern.split('/').collect();
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    match_parts(&pats, &parts, 0, 0)
}

fn match_parts(pats: &[&str], parts: &[&str], pi: usize, si: usize) -> bool {
    if pi == pats.len() {
        return si == parts.len();
    }
    if pats[pi] == "**" {
        for skip in 0..=parts.len().saturating_sub(si) {
            if match_parts(pats, parts, pi + 1, si + skip) {
                return true;
            }
        }
        return false;
    }
    if si >= parts.len() {
        return false;
    }
    if simple_glob(pats[pi], parts[si]) {
        return match_parts(pats, parts, pi + 1, si + 1);
    }
    false
}

/// 单段通配（`*` / `?`，无 `/`）。
fn simple_glob(pat: &str, s: &str) -> bool {
    let p: Vec<char> = pat.chars().collect();
    let t: Vec<char> = s.chars().collect();
    let (mut pi, mut si) = (0usize, 0usize);
    let mut star: Option<usize> = None;
    let mut mark = 0usize;
    while si < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[si]) {
            pi += 1;
            si += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = si;
            pi += 1;
        } else if let Some(sp) = star {
            pi = sp + 1;
            mark += 1;
            si = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> ToolDescription {
        ToolDescription {
            name: "glob".into(),
            description: "Find files matching a glob pattern (e.g. '**/*.rs', 'src/*.ts')".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern to match against file paths"
                    }
                },
                "required": ["pattern"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String> {
        // R1-B (v0.1.1): LLM 层降级 raw string 时调度层再救——完整 parse + 前缀提取
        let pattern = crate::extract_str_arg(&args, "pattern")
            .ok_or_else(|| anyhow!("missing 'pattern' argument"))?;

        // M5: reject patterns that attempt to escape the working directory.
        if pattern.contains("..") {
            return Err(anyhow!(
                "path traversal denied in glob pattern: {}",
                pattern
            ));
        }

        // RT4: 重写为 Rust 进程内遍历——不再 spawn `find` 子进程。
        // 动机：① seccomp KILL 化下 find 调用白名单外 syscall 被杀（strace/bpftrace
        //   无法定位——seccomp 先于 ptrace/tracepoint 缓冲）；Rust 遍历不经子进程 seccomp，
        //   兼容 KILL 化。② R3 readonly 裁剪：遍历只读 + symlink 不跟随（防逃逸出 cwd）。
        // ③ 更快（无进程 spawn 开销）。
        let mut matched = Vec::new();
        walk_readonly(&ctx.cwd, "", &pattern, &mut matched, 0);
        if matched.is_empty() {
            Ok("(no matches)".into())
        } else {
            Ok(matched.join("\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tool_runtime::ToolContext;

    #[tokio::test]
    async fn test_glob_finds_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "// a").unwrap();
        std::fs::write(dir.path().join("b.rs"), "// b").unwrap();
        std::fs::write(dir.path().join("c.txt"), "c").unwrap();

        // KILL 语义（阶段二）下 landlock 边界更严：tempdir 显式 writable——
        // 测试意图是 glob 功能（模式匹配/遍历），隔离边界由 sandbox crate 的
        // test_p5_landlock_* 覆盖（与 test_glob_no_match 一致）。
        let cfg = sandbox::SandboxConfig {
            writable_paths: vec![dir.path().to_path_buf()],
            ..sandbox::SandboxConfig::default()
        };
        let tool = GlobTool::with_sandbox(Arc::from(sandbox::create_sandbox(cfg)));
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };

        let result = tool
            .execute(serde_json::json!({"pattern": "*.rs"}), &ctx)
            .await
            .unwrap();
        assert!(result.contains("a.rs"));
        assert!(result.contains("b.rs"));
        assert!(!result.contains("c.txt"));
    }

    #[tokio::test]
    async fn test_glob_no_match() {
        let dir = tempfile::tempdir().unwrap();
        // RT3: 纯功能测试用 NoopSandbox——glob 的模式匹配/遍历逻辑与 sandbox 隔离
        // 无关；隔离边界由 sandbox crate 的 test_p5_* 覆盖
        let tool = GlobTool::with_sandbox(Arc::new(sandbox::NoopSandbox::new(
            sandbox::SandboxConfig::default(),
        )));
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };

        let result = tool
            .execute(serde_json::json!({"pattern": "*.nonexistent"}), &ctx)
            .await
            .unwrap();
        assert!(result.contains("no matches"));
    }

    #[tokio::test]
    async fn test_glob_missing_pattern() {
        let tool = GlobTool::new();
        let ctx = ToolContext::default();
        let result = tool.execute(serde_json::json!({}), &ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_glob_traversal_pattern_denied() {
        let tool = GlobTool::new();
        let ctx = ToolContext::default();
        assert!(
            tool.execute(serde_json::json!({"pattern": "../../*"}), &ctx)
                .await
                .is_err(),
            "glob pattern with '..' should be denied"
        );
    }
}
