use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tool_runtime::{Tool, ToolContext, ToolDescription};

pub struct EditTool;

impl EditTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> ToolDescription {
        ToolDescription {
            name: "write_file".into(),
            description: "Overwrite (or append to) a file with content.\n\
                 何时用: 创建新文件；整文件重写；修改代码的收尾落盘（改代码 = 必须有写盘动作）。\n\
                 何时不用: 只改文件的一小段（用 apply_patch——精确 SEARCH/REPLACE，不易截断出错）。\n\
                 示例: write_file(path=\"index.html\", content=\"<html>…完整内容…</html>\")。\n\
                 边界: content 单次 ≤~3000 字符，超长分批（首次 overwrite、后续 mode=\"append\" 拼接）；\n\
                 overwrite 会整文件覆盖——改已有文件前先 read 确认内容，append 前确认已有内容结尾。\n\
                 错误解读: \"missing 'path'/'content'\"=参数缺失（完整重发参数）；\n\
                 \"path outside allowed roots\"=越界路径；写入成功无输出 = 已落盘（可 read 复核）。"
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file (keep under ~3000 chars per call; use mode=append for longer files)"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["write", "append"],
                        "description": "write (default): overwrite the file. append: append content to the end of the file (for splitting long files into chunks)"
                    }
                },
                "required": ["path", "content"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String> {
        // R1-B (v0.1.1): LLM 层降级 raw string 时调度层再救——完整 parse + 前缀提取
        let path_str = crate::extract_str_arg(&args, "path")
            .ok_or_else(|| anyhow!("missing 'path' argument"))?;

        // 路径穿越防护 (M1): 拒绝绝对路径与 `..` 穿越，强制留在 cwd 内。
        // CONS-2 (global-audit): 组件级检查，避免误拦 `test..txt`。
        // SBOX-1 (accepted risk): edit 直接走 tokio::fs 而非 sandbox —— 进程
        // 本身已被 landlock 限制（workspace 外只读），且此处强制 cwd 相对路径，
        // 双重防线下保留现状；如需三重防线可改由 sandbox.exec 落盘。
        let p = std::path::Path::new(&path_str);
        if p.components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(anyhow!("path traversal denied: {}", path_str));
        }
        // v12.4: LLMs very often echo back the absolute path they saw in a
        // previous tool result (e.g. "/home/u/ws/src/lib.rs"). Rejecting those
        // outright wasted whole steps. Accept an absolute path *only* when it
        // resolves inside the workspace, and rewrite it to a relative one;
        // anything outside cwd is still denied.
        // R3 (v0.1.3 B5): 家目录内也放行（landlock 兜底只读，越权写会被拦）。
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
                Err(_) => &path_str, // home 内：保留绝对路径（landlock 兜底）
            }
        } else {
            &path_str
        };

        // R1 (v0.1.2 任务书 P0-A): content 与 path 同款容错——LLM 层降级 raw string
        // 时 .get("content") 必失败（String 无字段）→ "missing 'content'"（五子棋 12004
        // 字符截断真凶）。extract_str_arg 前缀提取：content 截断在值中间仍救不回
        // （None），但配合 append 模式 + CONTENT LIMIT 提示，单次 content 短，此场景消失。
        let content = crate::extract_str_arg(&args, "content")
            .ok_or_else(|| anyhow!("missing 'content' argument"))?;

        // R8 (v0.1.2): append 模式——分段写同一文件，单次工具参数小，
        // 从根上避开 max_tokens 截断（与 codex 的 diff 编辑同理：小步、精确）。
        let mode = crate::extract_str_arg(&args, "mode").unwrap_or_else(|| "write".into());
        let append = mode == "append";

        let path = ctx.cwd.join(path_str);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| anyhow!("create parent dir failed: {e}"))?;
        }

        if append {
            // append 语义保留（对既有内容追加，无"毁原文件"风险面）
            use tokio::io::AsyncWriteExt as _;
            let mut f = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await
                .map_err(|e| anyhow!("open append failed for {}: {e}", path.display()))?;
            f.write_all(content.as_bytes())
                .await
                .map_err(|e| anyhow!("append failed for {}: {e}", path.display()))?;
            f.flush()
                .await
                .map_err(|e| anyhow!("flush failed for {}: {e}", path.display()))?;
        } else {
            // S1（P5-FOUNDATION-01 N12）：覆盖写统一走**原子写**——原分块路径
            // "首块 truncate 原文件 + 追加"在崩溃时毁原文件（三层摸底 S1 实证）。
            // content 本就整体在内存，tmp+rename 一次写完即原子且内容等价
            // （原字符分块只服务旧追加实现，原子化后不再需要）。
            // X2-3 的"超长内容对 agent 透明"语义由本路径继承。
            crate::atomic::write_atomic(&path, content.as_bytes())
                .map_err(|e| anyhow!("write failed for {}: {e}", path.display()))?;
        }

        // R5 (v0.1.2 任务书): Cargo.toml 上级 workspace 冲突 hint——AI 在错误目录
        // 建项目被上级 workspace 吞（1+1 任务实测）。轻量：结果追加一行提示，
        // 不阻塞主路径。skip=目标文件自身（刚写入，不得当"上级"误报）。
        let mut extra = String::new();
        if path_str.ends_with("Cargo.toml") {
            if let Some(parent) = path.parent() {
                if let Some(ws) = find_ancestor_cargo_toml(parent, &path) {
                    extra = format!(
                        "\n[workspace-hint] 检测到上级 workspace: {}——若本项目想独立，请在 Cargo.toml 顶部加 `[workspace]` 空表；若想并入 workspace，确认成员声明。",
                        ws.display()
                    );
                }
            }
        }
        // WS5 (v0.1.5) Hooks①: 编辑 Rust 文件后 lint 提示（确定性护栏，对齐 Claude Code
        // PostToolUse hook）——引导模型立即自测，不自动跑（自动编译慢且打断主路径）。
        if path_str.ends_with(".rs") {
            extra.push_str(
                "\n[lint-hint] 已编辑 Rust 文件——建议立即验证: bash(\"cargo fmt 2>&1 | head -5 && cargo clippy 2>&1 | tail -20\")。格式/告警清零后再交。",
            );
        }

        let verb = if append { "appended" } else { "wrote" };
        // S1: 覆盖写已原子化（tmp+rename）——对 agent 透明，不再需要分块标记
        Ok(format!(
            "{verb} {} bytes to {}{extra}",
            content.len(),
            path.display()
        ))
    }
}

/// 向上找最近的 Cargo.toml（workspace 冲突检测用——R5）。
/// skip = 目标文件自身（刚写入，不得当"上级"误报）。
fn find_ancestor_cargo_toml(
    from: &std::path::Path,
    skip: &std::path::Path,
) -> Option<std::path::PathBuf> {
    let mut cur = from;
    loop {
        let candidate = cur.join("Cargo.toml");
        if candidate.exists() && candidate != skip {
            return Some(candidate);
        }
        cur = cur.parent()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tool_runtime::ToolContext;

    #[tokio::test]
    async fn test_edit_write_file() {
        let dir = tempfile::tempdir().unwrap();

        let tool = EditTool::new();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };

        let result = tool
            .execute(
                serde_json::json!({"path": "out.txt", "content": "hello"}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(result.contains("wrote 5 bytes"));

        let saved = std::fs::read_to_string(dir.path().join("out.txt")).unwrap();
        assert_eq!(saved, "hello");
    }

    #[tokio::test]
    async fn test_edit_missing_args() {
        let tool = EditTool::new();
        let ctx = ToolContext::default();
        assert!(tool.execute(serde_json::json!({}), &ctx).await.is_err());
        assert!(tool
            .execute(serde_json::json!({"path": "x"}), &ctx)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_edit_path_traversal_denied() {
        let tool = EditTool::new();
        let ctx = ToolContext::default();
        // 绝对路径与 `..` 穿越必须被拒绝 (M1)
        assert!(
            tool.execute(
                serde_json::json!({"path": "/etc/passwd", "content": "x"}),
                &ctx
            )
            .await
            .is_err(),
            "absolute path should be denied"
        );
        assert!(
            tool.execute(
                serde_json::json!({"path": "../escape", "content": "x"}),
                &ctx
            )
            .await
            .is_err(),
            "parent traversal should be denied"
        );
    }

    /// v12.4: an absolute path that resolves *inside* the workspace is accepted
    /// and normalised, because LLMs routinely echo back absolute paths.
    #[tokio::test]
    async fn test_edit_absolute_path_inside_workspace_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let tool = EditTool::new();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };

        let abs = dir.path().join("src").join("lib.rs");
        let result = tool
            .execute(
                serde_json::json!({"path": abs.to_str().unwrap(), "content": "fn main() {}"}),
                &ctx,
            )
            .await
            .expect("absolute path inside workspace must be accepted");
        assert!(result.contains("wrote 12 bytes"), "got: {result}");
        assert_eq!(
            std::fs::read_to_string(&abs).unwrap(),
            "fn main() {}",
            "content must land at the absolute path"
        );
    }

    /// An absolute path outside the workspace is still denied.
    #[tokio::test]
    async fn test_edit_absolute_path_outside_workspace_denied() {
        let dir = tempfile::tempdir().unwrap();
        let tool = EditTool::new();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };
        let err = tool
            .execute(
                serde_json::json!({"path": "/etc/passwd", "content": "x"}),
                &ctx,
            )
            .await
            .unwrap_err();
        // R3 (v0.1.3): 消息改为 "outside allowed roots (workspace/home) denied"
        assert!(
            err.to_string().contains("denied") && err.to_string().contains("/etc/passwd"),
            "got: {err}"
        );
    }

    /// 回归 R1 (v0.1.2 任务书 P0-A): LLM 层降级 raw String 时，content 也必须走
    /// extract_str_arg——旧代码 .get("content") 对 String 必失败 → "missing 'content'"
    /// （五子棋 12004 字符截断真凶）。本测试在旧代码上必红（String 无 .get），
    /// 修复后绿（前缀提取救回 path + content）。
    #[tokio::test]
    async fn test_write_file_raw_string_content_rescued() {
        let dir = tempfile::tempdir().unwrap();
        let tool = EditTool::new();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };
        // 模拟 LLM 层 JSON 截断降级：整个参数变成 raw String，且截断在 content 中部
        let raw = serde_json::Value::String(
            r#"{"path": "game.html", "content": "<html><body><script>let x = 1;"#.into(),
        );
        let result = tool.execute(raw, &ctx).await;
        assert!(
            result.is_err(),
            "content 截断在值中间应救不回（返回可行动错误，而非写半个文件）"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("missing 'content'"),
            "应给出 missing 'content' 可行动错误，got: {err}"
        );

        // 完整 raw String（JSON 完整，但 LLM 层没能 parse 成 Object）——两个字段都应救回
        let raw = serde_json::Value::String(
            r#"{"path": "game.html", "content": "<html>ok</html>"}"#.into(),
        );
        let result = tool
            .execute(raw, &ctx)
            .await
            .expect("raw string 完整时应救回");
        assert!(result.contains("wrote"), "got: {result}");
        let saved = std::fs::read_to_string(dir.path().join("game.html")).unwrap();
        assert_eq!(saved, "<html>ok</html>");
    }

    /// 回归 R8 (v0.1.2): append 模式——分段写同一文件（超长文件防截断的结构性解法）。
    #[tokio::test]
    async fn test_write_file_append_chunks() {
        let dir = tempfile::tempdir().unwrap();
        let tool = EditTool::new();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };
        let head = "line1\nline2\n".repeat(100);
        let tail = "tail-end".to_string();
        tool.execute(
            serde_json::json!({"path": "big.txt", "content": head}),
            &ctx,
        )
        .await
        .expect("first chunk (overwrite)");
        tool.execute(
            serde_json::json!({"path": "big.txt", "content": tail, "mode": "append"}),
            &ctx,
        )
        .await
        .expect("second chunk (append)");
        let saved = std::fs::read_to_string(dir.path().join("big.txt")).unwrap();
        assert!(
            saved.starts_with(&head) && saved.ends_with(&tail),
            "append 必须接在文件尾部"
        );
        assert_eq!(saved.len(), head.len() + tail.len());
    }

    /// 回归 R5 (v0.1.2): 生成 Cargo.toml 且存在上级 workspace 时，结果带 hint。
    /// 场景：cwd=ws（预建 workspace Cargo.toml），AI 写 ws/sub/Cargo.toml（子项目）
    /// → 被上级 workspace 吞 → 必须给 hint。skip 逻辑：自己刚写的 Cargo.toml
    /// 不得误报为"上级"。
    #[tokio::test]
    async fn test_write_file_workspace_hint() {
        let dir = tempfile::tempdir().unwrap();
        // 上级 workspace：dir/ws/Cargo.toml
        let ws = dir.path().join("ws");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::write(ws.join("Cargo.toml"), "[workspace]\n").unwrap();
        let tool = EditTool::new();
        let ctx = ToolContext {
            cwd: ws.clone(),
            ..Default::default()
        };
        // 子项目：ws/sub/Cargo.toml
        let result = tool
            .execute(
                serde_json::json!({"path": "sub/Cargo.toml", "content": "[package]\nname=\"x\""}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(
            result.contains("workspace-hint"),
            "子项目 Cargo.toml 且上级有 workspace 时应给 hint，got: {result}"
        );
        // 反例：写的就是 workspace 自身的 Cargo.toml（cwd 根）——不得误报"上级"
        let result = tool
            .execute(
                serde_json::json!({"path": "Cargo.toml", "content": "[package]\nname=\"x\""}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(
            !result.contains("workspace-hint"),
            "写 workspace 自身的 Cargo.toml 不应误报 hint，got: {result}"
        );
    }

    /// 回归 X2-3 → S1（v0.2.23）：超长 write **原子写入**——内容完整落盘、
    /// 中文不被劈开（原分块路径的等价语义由 tmp+rename 继承，且不再有
    /// "truncate 原文件再追加"的崩溃毁文件窗口）。旧断言"注明分块"随分块
    /// 路径一并移除。
    #[tokio::test]
    async fn test_write_long_content_atomic() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };
        let tool = EditTool::new();
        // 12000 字符中文内容（原 > WRITE_CHUNK 4000 量级，含多字节字符）
        let long: String = (0..4000).map(|_| "中文内容段".repeat(3)).collect();
        assert!(long.chars().count() > 4000, "夹具须保持超长量级");
        let r = tool
            .execute(
                serde_json::json!({"path": "big.txt", "content": long}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(r.contains("wrote"), "结果投影保留，got: {r}");
        assert!(!r.contains("自动分块"), "分块路径已移除，got: {r}");
        let saved = std::fs::read_to_string(dir.path().join("big.txt")).unwrap();
        assert_eq!(saved, long, "原子写入必须与输入完全一致（含中文）");
        // S1 附加保证：无 tmp 残留
        let residues: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.contains("hearth-tmp"))
            .collect();
        assert!(residues.is_empty(), "原子写不得残留 tmp，got: {residues:?}");
    }

    /// 回归 X2-3 (v0.1.6): 覆盖语义保持——对已存在文件写超长内容，
    /// 最终文件 = 新内容（不是新旧拼接）。
    #[tokio::test]
    async fn test_write_long_content_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };
        let tool = EditTool::new();
        std::fs::write(dir.path().join("f.txt"), "OLD_CONTENT").unwrap();
        let long: String = (0..5000).map(|i| format!("新{i}")).collect();
        let _ = tool
            .execute(serde_json::json!({"path": "f.txt", "content": long}), &ctx)
            .await
            .unwrap();
        let saved = std::fs::read_to_string(dir.path().join("f.txt")).unwrap();
        assert_eq!(saved, long, "覆盖语义：最终文件必须是完整新内容");
        assert!(!saved.contains("OLD_CONTENT"), "旧内容必须被完全覆盖");
    }
}
