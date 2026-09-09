pub mod atomic;
pub mod bash;
pub mod edit;
pub mod glob;
pub mod grep;
pub mod patch;
pub mod read;
pub mod status;
pub mod todo;
pub mod web;

pub use bash::BashTool;
pub use edit::EditTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use patch::PatchTool;
pub use read::ReadTool;
pub use status::IntrospectTool;
pub use todo::TodoWriteTool;
pub use web::WebTool;

/// R3 (v0.1.3 任务书 B5): 绝对路径放行判定——落在允许根（cwd 项目目录或用户家目录）
/// 内则放行（真机日志 grep /home/wutao/codex_6d 被误拒）；越权系统目录（/etc /usr）
/// 仍拒绝。相对路径由各工具自行查 `..` 穿越（此处不判）。
pub fn is_allowed_absolute(raw: &str, cwd: &std::path::Path) -> bool {
    // RC25 (P3-BACKLOG): 读范围限域——HEARTH_READ_ROOTS（逗号分隔绝对路径）设置时
    // **替换**默认根（cwd + HOME），实现显式白名单收敛；未设置 = 默认（不回归）。
    let read_roots = std::env::var("HEARTH_READ_ROOTS")
        .ok()
        .filter(|v| !v.trim().is_empty());
    is_allowed_absolute_roots(raw, cwd, read_roots.as_deref())
}

/// RC25: 带 root 清单的路径校验。roots 语义：None = 默认（cwd + HOME，不回归）；
/// Some(list) = 显式白名单**替换**默认（逗号分隔绝对路径）。
pub fn is_allowed_absolute_roots(
    raw: &str,
    cwd: &std::path::Path,
    read_roots: Option<&str>,
) -> bool {
    let p = std::path::Path::new(raw);
    if !p.is_absolute() {
        return true;
    }
    if p.starts_with(cwd) {
        return true;
    }
    match read_roots {
        Some(list) => list.split(',').any(|r| {
            let r = r.trim();
            !r.is_empty() && p.starts_with(r)
        }),
        None => {
            let home = std::env::var("HOME").unwrap_or_default();
            !home.is_empty() && p.starts_with(&home)
        }
    }
}

/// R1-B (v0.1.1 用户真机): 工具参数容错——LLM 层 JSON 解析失败会把参数降级为
/// `Value::String`（raw），此时 `.get("path")` 必失败（String 无字段）→
/// "missing 'path' argument"。调度层再救一次：若 args 是 String 且可 parse 成
/// Object，则转换后照常取参（治本——即使 LLM 层降级也能救回）。
pub fn coerce_args(args: &serde_json::Value) -> serde_json::Value {
    match args {
        serde_json::Value::String(s) => {
            serde_json::from_str::<serde_json::Value>(s).unwrap_or_else(|_| args.clone())
        }
        _ => args.clone(),
    }
}

/// R1-B 增强 (v0.1.1 贪吃蛇实测): 提取字符串参数——JSON 被截断（String 降级）时，
/// 完整 parse 会失败，但**关键字段（path/pattern 等）在对象最前，通常完整**。
/// ① Object → 直接取；② String → 先完整 parse，失败则从前缀解析提取 `"key": "value"`。
pub fn extract_str_arg(args: &serde_json::Value, key: &str) -> Option<String> {
    if let serde_json::Value::Object(_) = args {
        return args
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
    }
    if let serde_json::Value::String(s) = args {
        // 完整 parse 优先
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(s) {
            if let Some(v) = v.get(key) {
                if let Some(ss) = v.as_str() {
                    return Some(ss.to_string());
                }
            }
        }
        // 前缀解析：`"key"` 后找冒号 + 引号，取到下一个未转义引号
        let needle = format!("\"{key}\"");
        let pos = s.find(&needle)?;
        let rest = s[pos + needle.len()..].trim_start_matches([' ', ':', '\t']);
        let rest = rest.strip_prefix('"')?;
        let mut out = String::new();
        let mut chars = rest.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                out.push(c);
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            } else if c == '"' {
                return Some(out);
            } else {
                out.push(c);
            }
        }
        None // 字符串未闭合（截断在值中间——救不回，交给错误提示）
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::coerce_args;

    /// R5-9 判据（智能性根治长程任务包 v1.0）：每工具描述必须含五要素——
    /// 何时用/何时不用/示例/边界/错误解读。旧语义：read 5 个词、glob/grep
    /// 一句话（根因四：工具契约贫瘠=弱模型表现的直接杠杆）→ 红。
    #[test]
    fn test_r59_tool_descriptions_have_five_elements() {
        use super::{BashTool, EditTool, GlobTool, GrepTool, PatchTool, ReadTool};
        const FIVE: [&str; 5] = ["何时用", "何时不用", "示例", "边界", "错误解读"];
        let tools: Vec<Box<dyn tool_runtime::Tool>> = vec![
            Box::new(ReadTool::new()),
            Box::new(BashTool::new()),
            Box::new(EditTool::new()),
            Box::new(PatchTool::new()),
            Box::new(GlobTool::new()),
            Box::new(GrepTool::new()),
        ];
        for t in tools {
            let d = t.description();
            for marker in FIVE {
                assert!(
                    d.description.contains(marker),
                    "工具 {} 的描述缺要素「{}」——R5-9 五要素契约：\n{}",
                    d.name,
                    marker,
                    d.description
                );
            }
        }
    }

    #[test]
    fn test_coerce_args_string_to_object() {
        // R1-B: LLM 层降级 raw string → 调度层再救一次
        let raw = serde_json::Value::String(r#"{"path": "a.rs", "content": "hi"}"#.into());
        let v = coerce_args(&raw);
        assert!(v.is_object(), "String 可 parse 时应转 Object");
        assert_eq!(v.get("path").and_then(|p| p.as_str()), Some("a.rs"));
    }

    #[test]
    fn test_coerce_args_object_passthrough() {
        let obj = serde_json::json!({"path": "b.rs"});
        let v = coerce_args(&obj);
        assert!(v.is_object());
    }

    #[test]
    fn test_coerce_args_unparseable_keeps_string() {
        let raw = serde_json::Value::String("not json {{{".into());
        let v = coerce_args(&raw);
        assert!(
            v.is_string(),
            "不可 parse 保持 raw string（下游报错给可行动信息）"
        );
    }
}

#[cfg(test)]
mod p3_tests {
    use super::*;

    /// RC25 ①：默认（未设 HEARTH_READ_ROOTS）= cwd + HOME（不回归）。
    #[test]
    fn test_rc25_default_roots() {
        let cwd = std::path::Path::new("/home/u/ws");
        // workspace 内
        assert!(is_allowed_absolute_roots("/home/u/ws/a.txt", cwd, None));
        // HOME 内（默认保留；用真实 HOME 断言）
        let home = std::env::var("HOME").unwrap_or_default();
        if !home.is_empty() {
            assert!(is_allowed_absolute_roots(
                &format!("{home}/other.txt"),
                cwd,
                None
            ));
        }
        // HOME 外
        assert!(!is_allowed_absolute_roots("/etc/passwd", cwd, None));
    }

    /// RC25 ②：显式 read_roots **替换**默认——白名单内放行、HOME 内也拒。
    #[test]
    fn test_rc25_explicit_roots_replace() {
        let cwd = std::path::Path::new("/home/u/ws");
        let roots = Some("/data/allow,/tmp/allow");
        assert!(is_allowed_absolute_roots("/data/allow/x", cwd, roots));
        assert!(is_allowed_absolute_roots("/tmp/allow/y", cwd, roots));
        // 默认根被替换：HOME 内不再自动放行（显式白名单语义）
        assert!(!is_allowed_absolute_roots(
            &format!("{}/other.txt", std::env::var("HOME").unwrap_or_default()),
            cwd,
            roots
        ));
        assert!(!is_allowed_absolute_roots("/etc/passwd", cwd, roots));
    }

    /// RC25 ③：结构化拒绝语义——相对路径永远放行（cwd 内相对解析）。
    #[test]
    fn test_rc25_relative_always_allowed() {
        let cwd = std::path::Path::new("/home/u/ws");
        assert!(is_allowed_absolute_roots(
            "relative.txt",
            cwd,
            Some("/data")
        ));
    }
}
