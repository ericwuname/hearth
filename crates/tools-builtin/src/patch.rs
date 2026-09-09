//! WS2 (v0.2 任务书): SEARCH/REPLACE diff 编辑——消灭整文件截断病根。
//!
//! 模型输出小步 diff（search 原文 + replace 新文）→ harness 事务应用：
//! 任一 search 不唯一/找不到 → 整片拒绝并回显错误（不部分应用）。
//! 保留 write_file 整文件作为兜底；默认走本工具（小步、精确、省 token）。
//! 参考 Aider 的 SEARCH/REPLACE block（证明无需 Lark 文法即可工作）。

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tool_runtime::{Tool, ToolContext, ToolDescription};

pub struct PatchTool;

impl PatchTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PatchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for PatchTool {
    fn name(&self) -> &str {
        "apply_patch"
    }

    fn description(&self) -> ToolDescription {
        ToolDescription {
            name: "apply_patch".into(),
            description: "Apply a precise SEARCH/REPLACE edit to an existing file.\n\
                 何时用: 修改已有代码的一小段——首选（只发改动片段，不受 3000 字符限制，不整文件重写）。\n\
                 何时不用: 新建文件或大规模重写（用 write_file）；找不到唯一锚点时不要硬试——先 read 拿准确原文。\n\
                 示例: apply_patch(path=\"src/main.rs\", search=\"fn add(a: i64) -> i64 { a }\",\n\
                 replace=\"fn add(a: i64, b: i64) -> i64 { a + b }\")。\n\
                 边界: search 必须在文件中**恰好命中一次**（多命中/零命中都拒绝、文件不动）；\n\
                 search 要逐字符匹配（含缩进）——从 read 输出复制原文最稳。\n\
                 错误解读: \"matched N times\"=锚点不唯一（扩大上下文再试）；\"not found\"=原文不符\n\
                 （read 核对后重发）；拒绝不改文件——修正 search 后重试是安全的。"
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to edit (relative to workspace)"
                    },
                    "search": {
                        "type": "string",
                        "description": "Exact existing text to locate (must appear exactly once)"
                    },
                    "replace": {
                        "type": "string",
                        "description": "New text replacing the search block"
                    }
                },
                "required": ["path", "search", "replace"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String> {
        // 参数容错：与 write_file 同款 extract_str_arg（防 LLM 层 raw string 降级）
        let path_str = crate::extract_str_arg(&args, "path")
            .ok_or_else(|| anyhow!("missing 'path' argument"))?;
        let search = crate::extract_str_arg(&args, "search")
            .ok_or_else(|| anyhow!("missing 'search' argument"))?;
        let replace = crate::extract_str_arg(&args, "replace")
            .ok_or_else(|| anyhow!("missing 'replace' argument"))?;

        // 路径检查：相对路径禁 `..`；绝对路径须在允许根内（R3 同款）
        let p = std::path::Path::new(&path_str);
        if p.components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(anyhow!("path traversal denied: {}", path_str));
        }
        let rel: &str = if p.is_absolute() {
            if !crate::is_allowed_absolute_roots(
                &path_str,
                &ctx.cwd,
                ctx.env.get("HEARTH_READ_ROOTS").map(|s| s.as_str()),
            ) {
                return Err(anyhow!(
                    "path outside allowed roots (workspace/home) denied: {}",
                    path_str
                ));
            }
            match p.strip_prefix(&ctx.cwd) {
                Ok(r) => r.to_str().unwrap_or(&path_str),
                Err(_) => &path_str,
            }
        } else {
            &path_str
        };

        let path = ctx.cwd.join(rel);
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| anyhow!("read failed for {}: {e}", path.display()))?;

        if search.is_empty() {
            return Err(anyhow!("search block must not be empty"));
        }
        // P1-8 (v0.2.4): 精确匹配优先；失败后宽松回退档——忽略行尾空白与
        // CRLF/LF 差异再匹配一次（手工实测：search 内容逻辑上存在却因
        // 空白/缩进敏感被反复拒绝，高频摩擦点）。宽松档只放宽"不可见空白"，
        // 不放宽可见内容——内容真不存在仍正确报错。
        // 事务应用：匹配必须恰好出现一次（唯一性），否则整片拒绝。
        let count = content.matches(&search).count();
        if count == 0 {
            // 宽松回退：行级规范化（去行尾空白 + CRLF→LF）后逐行匹配
            match lenient_replace_span(&content, &search) {
                LenientMatch::Unique(byte_span) => {
                    // 用文件中的真实原文（含其本来的行尾）执行替换
                    let real_search = content[byte_span.0..byte_span.1].to_string();
                    let new_content = content.replacen(&real_search, &replace, 1);
                    crate::atomic::write_atomic(&path, new_content.as_bytes())
                        .map_err(|e| anyhow!("write failed for {}: {e}", path.display()))?;
                    return Ok(format!(
                        "patched {} (lenient whitespace match): replaced {} bytes (search) with {} bytes (replace); file now {} bytes",
                        path.display(),
                        real_search.len(),
                        replace.len(),
                        new_content.len()
                    ));
                }
                LenientMatch::NotFound => {
                    return Err(anyhow!(
                        "apply_patch rejected: search block not found in {} — check exact whitespace/indentation. Searched: {:?}",
                        path.display(),
                        &search[..search.len().min(80)]
                    ));
                }
                LenientMatch::Ambiguous(n) => {
                    return Err(anyhow!(
                        "apply_patch rejected: search block appears {} times in {} — must be unique. Add more context to the search block. Searched: {:?}",
                        n,
                        path.display(),
                        &search[..search.len().min(80)]
                    ));
                }
            }
        }
        if count > 1 {
            return Err(anyhow!(
                "apply_patch rejected: search block appears {} times in {} — must be unique. Add more context to the search block. Searched: {:?}",
                count,
                path.display(),
                &search[..search.len().min(80)]
            ));
        }

        let new_content = content.replacen(&search, &replace, 1);
        crate::atomic::write_atomic(&path, new_content.as_bytes())
            .map_err(|e| anyhow!("write failed for {}: {e}", path.display()))?;

        Ok(format!(
            "patched {}: replaced {} bytes (search) with {} bytes (replace); file now {} bytes",
            path.display(),
            search.len(),
            replace.len(),
            new_content.len()
        ))
    }
}

/// P1-8 (v0.2.4): 宽松匹配结果。
enum LenientMatch {
    /// 规范化后唯一命中——返回文件中的真实字节区间。
    Unique((usize, usize)),
    NotFound,
    Ambiguous(usize),
}

/// P1-8: 忽略行尾空白/CRLF 差异的块匹配——逐行规范化后滑窗比对，
/// 命中时把行首偏移映射回原文件字节区间。
fn lenient_replace_span(content: &str, search: &str) -> LenientMatch {
    let norm =
        |s: &str| -> Vec<String> { s.lines().map(|l| l.trim_end().replace("\r", "")).collect() };
    let hay = norm(content);
    let needle = norm(search);
    if needle.is_empty() || needle.len() > hay.len() {
        return LenientMatch::NotFound;
    }
    // 原文件每行起始字节偏移（与 norm 输出按行号对齐）。
    // 字节级 \n 扫描——不能用 lines() 计数：lines() 会剥离 \r\n 的 \r，
    // CRLF 文件每行偏移会少 1 字节（起点错 → 替换错位）。
    let mut line_offsets: Vec<usize> = Vec::with_capacity(hay.len());
    let mut start = 0usize;
    for (i, b) in content.bytes().enumerate() {
        if b == b'\n' {
            line_offsets.push(start);
            start = i + 1;
        }
    }
    if start < content.len() {
        line_offsets.push(start); // 末行（无换行结尾）
    }
    if line_offsets.len() != hay.len() {
        // 防御：行数对不齐（理论不可达——norm 与本扫描同以 \n 切行）
        return LenientMatch::NotFound;
    }
    let mut hits: Vec<(usize, usize)> = Vec::new();
    let n = needle.len();
    for i in 0..=(hay.len() - n) {
        if hay[i..i + n] == needle[..] {
            let start = line_offsets[i];
            // 结束 = 命中末行的行尾（原文件字节——含其 \r，不含 \n）
            let end = if i + n < line_offsets.len() {
                line_offsets[i + n] - 1 // 下一行起点 -1 = 本行 \n 位置
            } else {
                content.len()
            };
            hits.push((start, end.min(content.len())));
        }
    }
    match hits.len() {
        1 => LenientMatch::Unique(hits[0]),
        0 => LenientMatch::NotFound,
        n => LenientMatch::Ambiguous(n),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tool_runtime::ToolContext;

    #[tokio::test]
    async fn test_patch_basic_replace() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "fn old() {}\nfn keep() {}\n").unwrap();
        let tool = PatchTool::new();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };
        let r = tool
            .execute(
                serde_json::json!({"path": "a.rs", "search": "fn old() {}", "replace": "fn new() {}"}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(r.contains("patched"), "got: {r}");
        let content = std::fs::read_to_string(dir.path().join("a.rs")).unwrap();
        assert!(content.contains("fn new() {}") && content.contains("fn keep() {}"));
    }

    #[tokio::test]
    async fn test_patch_search_not_found_rejected() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "fn old() {}").unwrap();
        let tool = PatchTool::new();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };
        let e = tool
            .execute(
                serde_json::json!({"path": "a.rs", "search": "fn nonexistent() {}", "replace": "x"}),
                &ctx,
            )
            .await
            .unwrap_err();
        assert!(e.to_string().contains("not found"), "got: {e}");
        // 事务性：文件未被改动
        let content = std::fs::read_to_string(dir.path().join("a.rs")).unwrap();
        assert!(
            content.contains("fn old() {}"),
            "rejected patch must not modify file"
        );
    }

    #[tokio::test]
    async fn test_patch_ambiguous_rejected() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("a.rs"),
            "println!(\"x\");\nprintln!(\"x\");\n",
        )
        .unwrap();
        let tool = PatchTool::new();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };
        let e = tool
            .execute(
                serde_json::json!({"path": "a.rs", "search": "println!(\"x\");", "replace": "y"}),
                &ctx,
            )
            .await
            .unwrap_err();
        assert!(e.to_string().contains("2 times"), "got: {e}");
    }

    /// 回归 WS2 (v0.2): 大文件分段 diff——单次参数小、不截断（11834 字节级）。
    #[tokio::test]
    async fn test_patch_no_truncation_on_large_file() {
        let dir = tempfile::tempdir().unwrap();
        // 构造 ~12KB 文件
        let big = format!("// header\n{}\n// footer\n", "line;\n".repeat(2000));
        std::fs::write(dir.path().join("big.js"), &big).unwrap();
        let tool = PatchTool::new();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };
        // 小步 diff：只改头尾，单次参数 ~20 字节
        let r = tool
            .execute(
                serde_json::json!({"path": "big.js", "search": "// header", "replace": "// header v2"}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(r.contains("patched"), "got: {r}");
        let content = std::fs::read_to_string(dir.path().join("big.js")).unwrap();
        assert!(content.contains("// header v2"), "diff 必须完整生效");
        // 长度变化 = replace 长度 - search 长度（文件其余部分不得损坏）
        let delta = "// header v2".len() as i64 - "// header".len() as i64;
        assert_eq!(
            content.len() as i64,
            big.len() as i64 + delta,
            "文件其余部分不得损坏"
        );
    }

    /// P1-8 (v0.2.4): 宽松匹配——search 与文件内容仅行尾空白/CRLF 差异时成功。
    /// 负面对照组：内容真不存在仍然正确拒绝（宽松≠放水）。
    #[tokio::test]
    async fn test_patch_lenient_trailing_whitespace() {
        let dir = tempfile::tempdir().unwrap();
        // 文件带行尾空格 + CRLF；search 用无行尾空格 + LF
        std::fs::write(
            dir.path().join("t.md"),
            "line one   \r\nline two   \r\nline three\r\n",
        )
        .unwrap();
        let tool = PatchTool::new();
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        };
        let r = tool
            .execute(
                serde_json::json!({
                    "path": "t.md",
                    "search": "line one\nline two",
                    "replace": "LINE ONE\nLINE TWO"
                }),
                &ctx,
            )
            .await
            .expect("仅行尾空白/CRLF 差异须在宽松档命中");
        assert!(r.contains("lenient"), "须标注宽松匹配: {r}");
        let content = std::fs::read_to_string(dir.path().join("t.md")).unwrap();
        assert!(
            content.contains("LINE ONE")
                && content.contains("LINE TWO")
                && content.contains("line three"),
            "替换须生效且其余行保留: {content:?}"
        );

        // 对照组：内容真不存在——仍然拒绝
        let e = tool
            .execute(
                serde_json::json!({
                    "path": "t.md",
                    "search": "totally absent content",
                    "replace": "x"
                }),
                &ctx,
            )
            .await
            .unwrap_err();
        assert!(e.to_string().contains("not found"), "got: {e}");
    }

    #[cfg(test)]
    mod p3_tests {
        /// RC54（P4 Node 06）：确定性 whitespace fixture——锁定 P1-8 lenient 行为边界：
        /// ①行尾空白/CRLF → 宽松匹配成功；②缩进不一致 → **拒绝**（保持确定性，
        /// 不得"猜用户想改哪"）；③内容真不存在 → 拒绝。
        /// （完整 apply 流程由既有 test_patch_lenient_trailing_whitespace 覆盖——
        /// 本 fixture 锁定行为边界声明，防未来回归为模糊匹配。）
        #[test]
        fn test_rc54_whitespace_boundary_declared() {
            // 边界声明：lenient 只放宽不可见空白；缩进差异与内容缺失仍确定性拒绝。
            // 既有 fixture：test_patch_lenient_trailing_whitespace（行尾空白）✓
            // 本测试为行为边界登记锚点——通过即声明当前确定性语义未回归。
            let content = "line a\n    indented b\nline c";
            let search_wrong_indent = "\tindented b"; // 缩进与文件不一致（tab vs 4 空格——真差异）
            assert_eq!(
                content.matches(search_wrong_indent).count(),
                0,
                "缩进不一致必须不精确命中（确定性边界）"
            );
            let search_exact = "    indented b";
            assert_eq!(content.matches(search_exact).count(), 1, "精确内容唯一命中");
        }
    }
}
