use anyhow::{anyhow, Result};
use async_trait::async_trait;
use sandbox::{Sandbox, SandboxConfig};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tool_runtime::{Tool, ToolContext, ToolDescription};

pub struct GrepTool {
    sandbox: Arc<dyn Sandbox>,
    /// PERF-1 (global-audit): cache the `rg --version` sandbox probe result —
    /// it can't change while the process is running, so probing once is enough.
    has_rg_cache: tokio::sync::OnceCell<bool>,
}

impl GrepTool {
    pub fn new() -> Self {
        Self {
            sandbox: Arc::from(sandbox::create_sandbox(SandboxConfig::default())),
            has_rg_cache: tokio::sync::OnceCell::new(),
        }
    }

    pub fn with_sandbox(sandbox: Arc<dyn Sandbox>) -> Self {
        Self {
            sandbox,
            has_rg_cache: tokio::sync::OnceCell::new(),
        }
    }
}

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> ToolDescription {
        ToolDescription {
            name: "grep".into(),
            description: "Search file contents for a regex pattern (ripgrep/grep).\n\
                 何时用: 按内容定位代码——符号名/错误串/配置项；要\"哪一行\"时优先于 read。\n\
                 何时不用: 只知文件名不知内容（用 glob）；要看命中处上下文全文（grep 拿行号后用 read offset=行号 精读）。\n\
                 示例: grep(pattern=\"fn parse_positive\", path=\"src\")；grep(pattern=\"TODO\", glob=\"*.rs\")。\n\
                 边界: 正则语法（rg）；命中过多时输出按上限截断——换更特异的 pattern；\n\
                 只读不改——找到位置后必须落到 read/write_file/apply_patch 才算推进。\n\
                 错误解读: \"no matches\"=零命中（换词/放宽 pattern，不要原样重试）；\n\
                 \"command failed\"=rg 不在环境（改用内置 fallback 自动发生，重试即可）。"
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The regex pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional file or directory path (default: current working directory)"
                    },
                    "glob": {
                        "type": "string",
                        "description": "Optional file glob filter (e.g. '*.rs')"
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

        // M1: sanitize the optional user-supplied path.
        // CONS-1 (global-audit): traversal/absolute paths are now REJECTED with
        // an error instead of silently falling back to cwd — the silent
        // fallback returned misleading results for the caller's query.
        // CONS-2: component-level check instead of `contains("..")` so legal
        // names like `test..txt` aren't false-blocked.
        let search_path = match crate::extract_str_arg(&args, "path") {
            Some(s) => {
                let p = std::path::Path::new(&s);
                // CONS-2: component-level check instead of `contains("..")` so legal
                // names like `test..txt` aren't false-blocked.
                if p.components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
                {
                    tracing::warn!("path traversal attempt denied: {}", s);
                    return Err(anyhow!("path traversal denied: {}", s));
                }
                // R3 (v0.1.3 B5): 绝对路径若落在项目/家目录内则放行（真机
                // grep /home/wutao/codex_6d 被误拒）；越权系统目录仍拒绝。
                if p.is_absolute()
                    && !crate::is_allowed_absolute_roots(
                        &s,
                        &ctx.cwd,
                        ctx.env.get("HEARTH_READ_ROOTS").map(|s| s.as_str()),
                    )
                {
                    tracing::warn!("path outside allowed roots (workspace/home) denied: {}", s);
                    return Err(anyhow!("path traversal denied: {}", s));
                }
                s.to_string()
            }
            None => ctx.cwd.display().to_string(),
        };

        // R1 (v0.1.2): 可选字段 glob 也走 extract_str_arg（与 pattern/path 一致，
        // 防 raw String 降级时漏接——漏了顶多搜索范围不精确，但一致性要紧）。
        let file_glob = crate::extract_str_arg(&args, "glob");

        // Try rg first, fall back to grep. The rg availability probe itself runs
        // THROUGH the sandbox (X5 fix); the actual search also runs sandboxed. (M7)
        // PERF-1: the probe result is cached after the first call.
        let rg_available = *self
            .has_rg_cache
            .get_or_init(|| has_rg(self.sandbox.clone(), ctx.cwd.clone()))
            .await;
        let (cmd, argv) = if rg_available {
            let mut argv: Vec<String> = vec![
                "--line-number".into(),
                "--no-heading".into(),
                "--color=never".into(),
            ];
            if let Some(g) = &file_glob {
                argv.push("--glob".into());
                argv.push(g.to_string());
            }
            argv.push(pattern.to_string());
            argv.push(search_path.clone());
            ("rg", argv)
        } else {
            let mut argv: Vec<String> = vec!["-rn".into(), "--color=never".into()];
            if let Some(g) = &file_glob {
                argv.push("--include".into());
                argv.push(g.to_string());
            }
            argv.push(pattern.to_string());
            argv.push(search_path.clone());
            ("grep", argv)
        };

        let arg_refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
        let output = self
            .sandbox
            .spawn(cmd, &arg_refs, &ctx.cwd, &[], Duration::from_secs(30))
            .await
            .map_err(|e| anyhow!("grep failed: {e}"))?;

        if output.timed_out {
            return Err(anyhow!("grep timed out"));
        }

        let stdout = output.stdout.clone();
        if stdout.is_empty() {
            Ok("(no matches)".into())
        } else {
            Ok(stdout)
        }
    }
}

async fn has_rg(sandbox: Arc<dyn Sandbox>, cwd: PathBuf) -> bool {
    // X5 fix: probe rg availability THROUGH the sandbox. The probe previously ran
    // `rg --version` directly in the service process (outside landlock/seccomp);
    // a poisoned `rg` on PATH could then execute unsandboxed. Running it via the
    // sandbox means even a malicious `rg` is confined.
    sandbox
        .spawn("rg", &["--version"], &cwd, &[], Duration::from_secs(2))
        .await
        .map(|o| o.exit_code == 0)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tool_runtime::ToolContext;

    #[tokio::test]
    async fn test_grep_finds_pattern() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("a.rs"),
            "fn main() {\n    println!(\"hello\");\n}",
        )
        .unwrap();
        std::fs::write(dir.path().join("b.txt"), "hello world").unwrap();

        let tool = GrepTool::new();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };

        let result = tool
            .execute(serde_json::json!({"pattern": "hello"}), &ctx)
            .await
            .unwrap();
        // Should find "hello" in both files
        assert!(result.contains("hello"));
    }

    #[tokio::test]
    async fn test_grep_with_glob_filter() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "hello").unwrap();
        std::fs::write(dir.path().join("b.txt"), "hello").unwrap();

        let tool = GrepTool::new();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };

        let result = tool
            .execute(
                serde_json::json!({"pattern": "hello", "glob": "*.rs"}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(result.contains("a.rs"));
        assert!(!result.contains("b.txt"));
    }

    #[tokio::test]
    async fn test_grep_no_match() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("x.rs"), "nothing here").unwrap();

        let tool = GrepTool::new();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };

        let result = tool
            .execute(
                serde_json::json!({"pattern": "nonexistent_pattern_xyz"}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(result.contains("no matches"));
    }

    #[tokio::test]
    async fn test_grep_missing_pattern() {
        let tool = GrepTool::new();
        let ctx = ToolContext::default();
        let result = tool.execute(serde_json::json!({}), &ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_grep_traversal_path_rejected() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "hello").unwrap();

        let tool = GrepTool::new();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };

        // CONS-1: a malicious absolute or traversal path is now an ERROR,
        // not a silent fallback to cwd.
        let abs = tool
            .execute(
                serde_json::json!({"pattern": "hello", "path": "/etc"}),
                &ctx,
            )
            .await;
        assert!(abs.is_err(), "absolute path must be rejected");

        let up = tool
            .execute(
                serde_json::json!({"pattern": "hello", "path": "../x"}),
                &ctx,
            )
            .await;
        assert!(up.is_err(), "parent traversal must be rejected");

        // CONS-2: a legal name containing consecutive dots must NOT be blocked.
        std::fs::write(dir.path().join("test..txt"), "hello dots").unwrap();
        let ok = tool
            .execute(
                serde_json::json!({"pattern": "hello", "path": "test..txt"}),
                &ctx,
            )
            .await;
        assert!(ok.is_ok(), "'test..txt' is a legal filename, not traversal");
    }

    /// 回归 R3 (v0.1.3 B5): 家目录内绝对路径放行（真机 grep /home/wutao/codex_6d
    /// 被误拒）；越权系统目录（/etc）仍拒绝。旧代码（绝对路径一律 denied）此测试红。
    #[tokio::test]
    async fn test_absolute_path_within_root_allowed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "hello home").unwrap();
        let tool = GrepTool::new();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };
        // 绝对路径 = cwd 内 → 放行
        let ok = tool
            .execute(
                serde_json::json!({"pattern": "hello", "path": dir.path().join("a.rs").to_str().unwrap()}),
                &ctx,
            )
            .await;
        assert!(ok.is_ok(), "cwd 内绝对路径必须放行，got: {ok:?}");

        // 越权系统目录 → 拒绝
        let denied = tool
            .execute(
                serde_json::json!({"pattern": "root", "path": "/etc/shadow"}),
                &ctx,
            )
            .await;
        assert!(denied.is_err(), "/etc/shadow 越权必须拒绝（仍防越权）");
    }
}
