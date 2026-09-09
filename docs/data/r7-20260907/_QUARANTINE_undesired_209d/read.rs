use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tool_runtime::{Tool, ToolContext, ToolDescription};

pub struct ReadTool;

impl ReadTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str {
        "read"
    }

    fn description(&self) -> ToolDescription {
        ToolDescription {
            name: "read".into(),
            description: "Read the contents of a file".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String> {
        // R1-B (v0.1.1): LLM 层降级 raw string 时调度层再救——完整 parse + 前缀提取
        let path_str = crate::extract_str_arg(&args, "path")
            .ok_or_else(|| anyhow!("missing 'path' argument"))?;

        // 路径穿越防护 (M1): 拒绝绝对路径与 `..` 穿越，强制留在 cwd 内。
        // CONS-2 (global-audit): 用组件级检查替代 `contains("..")` 子串匹配，
        // 后者会误拦 `test..txt` 这类合法文件名。
        let p = std::path::Path::new(&path_str);
        if p.components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(anyhow!("path traversal denied: {}", path_str));
        }
        // v12.4: mirror write_file — accept an absolute path when it resolves
        // inside the workspace (LLMs echo them back constantly), deny otherwise.
        // R3 (v0.1.3 B5): 家目录内绝对路径也放行（读 ~/xxx 合法）；越权系统目录仍拒。
        let path_str: &str = if p.is_absolute() {
            if !crate::is_allowed_absolute_roots(
                &path_str,
                &ctx.cwd,
                ctx.env.get("HEARTH_READ_ROOTS").map(|s| s.as_str()),
            ) {
                return Err(anyhow!(
                    "path outside allowed roots (workspace/home) denied: {} (workspace: {})",
                    path_str,
                    ctx.cwd.display()
                ));
            }
            match p.strip_prefix(&ctx.cwd) {
                Ok(rel) => rel
                    .to_str()
                    .ok_or_else(|| anyhow!("non-utf8 path: {}", path_str))?,
                Err(_) => &path_str, // home 内：保留绝对路径
            }
        } else {
            &path_str
        };

        let path = ctx.cwd.join(path_str);
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| anyhow!("read failed for {}: {e}", path.display()))?;

        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tool_runtime::ToolContext;

    #[tokio::test]
    async fn test_read_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "hello world").unwrap();

        let tool = ReadTool::new();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };

        let result = tool
            .execute(serde_json::json!({"path": "test.txt"}), &ctx)
            .await
            .unwrap();
        assert_eq!(result, "hello world");
    }

    #[tokio::test]
    async fn test_read_missing_path() {
        let tool = ReadTool::new();
        let ctx = ToolContext::default();
        let result = tool.execute(serde_json::json!({}), &ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_path_traversal_denied() {
        let tool = ReadTool::new();
        let ctx = ToolContext::default();
        // 绝对路径与 `..` 穿越必须被拒绝 (M1)
        assert!(
            tool.execute(serde_json::json!({"path": "/etc/passwd"}), &ctx)
                .await
                .is_err(),
            "absolute path should be denied"
        );
        assert!(
            tool.execute(serde_json::json!({"path": "../secret"}), &ctx)
                .await
                .is_err(),
            "parent traversal should be denied"
        );
    }
}
