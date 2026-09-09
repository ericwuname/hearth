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

/// R5-5（智能性根治长程任务包 v1.0）：无显式 limit 时的默认行数上限——
/// 超出即截断 + 给"如何读剩余部分"提示（旧语义：整文件原文直出 → 82k
/// 字符截断死结，引擎亲口承认过；中段丢弃后模型不知有截断）。
const DEFAULT_LINE_CAP: usize = 2000;

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str {
        "read"
    }

    fn description(&self) -> ToolDescription {
        ToolDescription {
            name: "read".into(),
            description: "Read a file's content with cat -n style line numbers (paged).\n\
                 何时用: 需要查看文件内容时——定位后、修改前必读；用 offset/limit 分页读大文件。\n\
                 何时不用: 只是找文件位置（用 grep/glob）；找内容片段（用 grep 直接命中行号）。\n\
                 示例: read(\"src/main.rs\") 读前 2000 行；read(\"src/main.rs\", offset=2001, limit=500) 续读。\n\
                 边界: 输出每行带行号（行号可引用）；默认最多 2000 行，超出会截断并给出续读 offset；\n\
                 越出 EOF 返回空窗口提示；拒绝 workspace/home 之外的绝对路径与 .. 穿越。\n\
                 错误解读: \"path traversal denied\"=路径含 ..；\"path outside allowed roots\"=越界路径；\n\
                 \"read failed\"=文件不存在或无权限——确认路径后重试或用 glob 找正确文件。"
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "R5-5: 1-based line number to start reading from (default 1)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "R5-5: max lines to return (default 2000). Combine with offset for paging"
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

        // ── R5-5: 分页 + 行号（cat -n 格式）——大文件可分页读取，截断必带
        // "如何读剩余部分"提示。offset/limit 均为 1-based 行号（对 LLM 直观）。
        let offset = args
            .get("offset")
            .and_then(|v| v.as_u64())
            .map(|v| v.max(1) as usize)
            .unwrap_or(1);
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v.max(1) as usize)
            .unwrap_or(DEFAULT_LINE_CAP);
        let lines: Vec<&str> = content.lines().collect();
        let total = lines.len();
        let start = (offset - 1).min(total); // 0-based，越界钳到 EOF（空窗口）
        let end = (start + limit).min(total);
        let mut out = String::with_capacity(end.saturating_sub(start) * 32);
        for (i, line) in lines[start..end].iter().enumerate() {
            out.push_str(&format!("{:>6}\t{}\n", start + i + 1, line));
        }
        if out.is_empty() {
            out.push_str(&format!(
                "(no lines in range: file has {total} lines, requested offset={offset})\n"
            ));
        }
        if end < total {
            out.push_str(&format!(
                "... [{} more lines truncated — to read the rest, call read again with offset={} (and limit={} for the same page size)]\n",
                total - end,
                end + 1,
                limit
            ));
        } else if offset > 1 && start + limit >= total {
            // 有前窗且已到 EOF——告知读到了文件尾，防模型再翻页
            out.push_str(&format!("[end of file — {total} lines total]\n"));
        }
        Ok(out)
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
        // R5-5: read 输出升级为 cat -n 格式行号（`{:>6}\t`）
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
        assert_eq!(result, "     1\thello world\n");
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

    // ── R5-5（智能性根治长程任务包 v1.0）──

    /// R5-5 判据：行号输出（cat -n 格式）——每行 `{行号}\t{内容}`。
    /// 旧语义：原文直出无行号 → 红。
    #[tokio::test]
    async fn test_r55_read_line_numbers() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("multi.txt");
        std::fs::write(&file_path, "alpha\nbeta\ngamma").unwrap();
        let tool = ReadTool::new();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };
        let result = tool
            .execute(serde_json::json!({"path": "multi.txt"}), &ctx)
            .await
            .unwrap();
        assert!(
            result.contains("     1\talpha"),
            "行 1 必须带行号: {result:?}"
        );
        assert!(result.contains("     2\tbeta"));
        assert!(result.contains("     3\tgamma"));
        // 读完整个文件（3 行 < 2000 cap）——无截断提示
        assert!(!result.contains("truncated"));
    }

    /// R5-5 判据：offset/limit 分页——大文件可读指定窗口。
    /// 旧语义：read 仅 path 参数（grep offset=0）→ 红（未知参数被忽略，
    /// 输出为全文首行——断言必失败）。
    #[tokio::test]
    async fn test_r55_read_offset_limit_paging() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("big.txt");
        let content: String = (1..=100)
            .map(|i| format!("line-{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&file_path, content).unwrap();
        let tool = ReadTool::new();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };
        let result = tool
            .execute(
                serde_json::json!({"path": "big.txt", "offset": 50, "limit": 10}),
                &ctx,
            )
            .await
            .unwrap();
        // 窗口 = 行 50..=59
        assert!(
            result.contains("    50\tline-50"),
            "窗口首行必须带行号 50: {result:?}"
        );
        assert!(result.contains("    59\tline-59"));
        assert!(!result.contains("line-49"), "窗口前一行不得出现");
        assert!(!result.contains("line-60"), "窗口后一行不得出现");
        // 截断提示必须含"如何读剩余部分"（显式 offset 指引）
        assert!(
            result.contains("offset=60"),
            "截断提示必须给出续读 offset: {result:?}"
        );
        assert!(result.contains("41 more lines truncated"));
    }

    /// R5-5 判据：默认截断——超 2000 行无显式 limit 时截断 + 续读提示。
    /// 旧语义：整文件直出（超长触发 truncate_tool_output 中段丢弃且无提示）
    /// → 红。
    #[tokio::test]
    async fn test_r55_read_default_cap_with_resume_hint() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("huge.txt");
        let content: String = (1..=2500)
            .map(|i| format!("row-{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&file_path, content).unwrap();
        let tool = ReadTool::new();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };
        let result = tool
            .execute(serde_json::json!({"path": "huge.txt"}), &ctx)
            .await
            .unwrap();
        assert!(result.contains("     1\trow-1"), "首行必须在");
        assert!(result.contains("  2000\trow-2000"), "默认 cap=2000 行");
        assert!(!result.contains("row-2001"), "cap 后不得直出行 2001");
        assert!(
            result.contains("offset=2001"),
            "截断提示必须含如何读剩余部分（offset=2001）: {result:?}"
        );
    }

    /// R5-5 边界：offset 越过 EOF → 空窗口语义（不 panic，明确提示）。
    #[tokio::test]
    async fn test_r55_read_offset_beyond_eof() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("small.txt");
        std::fs::write(&file_path, "only\n").unwrap();
        let tool = ReadTool::new();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };
        let result = tool
            .execute(serde_json::json!({"path": "small.txt", "offset": 99}), &ctx)
            .await
            .unwrap();
        assert!(result.contains("no lines in range"), "{result:?}");
    }
}
