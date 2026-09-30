pub mod atomic;
pub mod bash;
pub mod edit;
pub mod glob;
pub mod grep;
pub mod patch;
pub mod read;
pub mod search;
pub mod status;
pub mod todo;
pub mod web;

pub use bash::BashTool;
pub use edit::EditTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use patch::PatchTool;
pub use read::ReadTool;
pub use search::WebSearchTool;
pub use status::IntrospectTool;
pub use todo::TodoWriteTool;
pub use web::WebTool;

// D-33 收敛（2026-10-01）：HTTP 响应体有界读取原语已迁至 `bounded-io`（reqwest feature），
// 此处**再导出**以保持既有调用点（`web.rs` / `search.rs` / `codex-cli`）无需改动。
pub use bounded_io::{read_body_capped, MAX_BODY_BYTES};

// D-57（2026-10-01）：原 `pub fn is_allowed_absolute(raw, cwd)` 已删除——全仓**零调用方**
// （仅其自身函数体引用 `is_allowed_absolute_roots`）。且它**直读 `std::env::var`**、绕过
// `ctx.env`，与"env 必须经 ToolContext 注入"的纪律相悖（属遗留）。各工具现统一走
// `is_allowed_absolute_roots(_for_write)` 并显式传入 `ctx.env` 的读根。

/// 判定"该路径在被 `join` 到 cwd 时**可能逃出工作区**"。
///
/// **为什么不能只用 `Path::is_absolute()`**（P0 安全修复 2026-09-30）：
///
/// - **Windows**：`/etc/passwd` 的 `is_absolute()` 为 **false**（Windows 的绝对路径
///   必须带盘符前缀），于是它被误判成"相对路径"**直接放行**；而
///   `PathBuf::join` 在 Windows 上遇到**有根无前缀**的路径（`/x`、`\x`）会
///   **替换掉盘符之后的一切** → `cwd.join("/etc/passwd")` 解析为 `C:\etc\passwd`，
///   **落到工作区之外**。实测（本机 MSVC）：`edit`/`read`/`patch`/`grep` 四个工具
///   均可借此越界，测试实跑已真实写出 `C:\etc\passwd`。
/// - `C:foo` 这类**盘符相对**路径（`Prefix` 组件）同样会被 join 替换盘符。
///
/// 故凡"有根"（`has_root`）或"带盘符前缀"（`Component::Prefix`）者，
/// 一律视为需要走白名单校验的绝对路径。相对的纯文件名（`a.txt`、`test..txt`）
/// 仍返回 false，不受影响。
pub fn is_rooted_path(p: &std::path::Path) -> bool {
    p.has_root() || matches!(p.components().next(), Some(std::path::Component::Prefix(_)))
}

/// **家目录放行是否有真实沙箱兜底？**（2026-10-01 修复，traecode）
///
/// 立项理由（R3 / v0.1.3）：landlock 把 workspace 之外置为**只读**，
/// 因此工具层放行家目录也无妨——"越权写会被拦"由 landlock 负责。
///
/// **但这个理由只在 Linux 成立**：landlock 是 Linux 专有机制；Windows / macOS 走
/// `NoopSandbox`（README 明示"仅开发模式，无真实隔离"）——**兜底是空的**。
/// 此时放行家目录 = **真的能写家目录**（`~/.ssh/authorized_keys`、shell 配置等持久化点）。
///
/// 实测（本机 Windows）：`HOME` 环境变量为空 → 该分支**空转**，风险不显现；
/// **但从 Git Bash 启动时 Git for Windows 会设置 `HOME`** → 分支生效 → 洞出现。
/// 故这是**条件性真洞**：代码声称的防护层在非 Linux 上不存在。
///
/// 处置：**无真实沙箱时，家目录只放行读、不放行写**（见 `is_allowed_absolute_roots_for_write`）。
pub fn home_allowance_backed_by_sandbox() -> bool {
    cfg!(target_os = "linux")
}

/// RC25: 带 root 清单的路径校验。roots 语义：None = 默认（cwd + HOME，不回归）；
/// Some(list) = 显式白名单**替换**默认（逗号分隔绝对路径）。
pub fn is_allowed_absolute_roots(
    raw: &str,
    cwd: &std::path::Path,
    read_roots: Option<&str>,
) -> bool {
    is_allowed_inner(raw, cwd, read_roots, true)
}

/// **写路径专用**：家目录仅在**有真实沙箱兜底**时才放行。
///
/// 修复前 `write_file` / `apply_patch` 在无 landlock 的平台上同样放行家目录，
/// 与代码自述的"越权写会被拦"不符（见 `home_allowance_backed_by_sandbox`）。
pub fn is_allowed_absolute_roots_for_write(
    raw: &str,
    cwd: &std::path::Path,
    read_roots: Option<&str>,
) -> bool {
    is_allowed_inner(raw, cwd, read_roots, home_allowance_backed_by_sandbox())
}

fn is_allowed_inner(
    raw: &str,
    cwd: &std::path::Path,
    read_roots: Option<&str>,
    allow_home: bool,
) -> bool {
    let p = std::path::Path::new(raw);
    if !is_rooted_path(p) {
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
            if !allow_home {
                return false;
            }
            let home = std::env::var("HOME").unwrap_or_default();
            !home.is_empty() && p.starts_with(&home)
        }
    }
}

// D-57（2026-10-01）：原 `pub fn coerce_args` 已删除——全仓**零生产调用方**（仅自身测试
// 引用）。其"防 LLM 层 raw string 降级"的意图**已由 `extract_str_arg` 覆盖并在位**：
// 每个工具（bash/read/edit/patch/glob/grep）都显式调用 `extract_str_arg`，
// 即真实防线是活的，被删的是并行且从未接线的旧机制。

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

    /// R5-9 判据（智能性根治长程任务包 v1.0）：每工具描述必须含五要素——
    /// 何时用/何时不用/示例/边界/错误解读。旧语义：read 5 个词、glob/grep
    /// 一句话（根因四：工具契约贫瘠=弱模型表现的直接杠杆）→ 红。
    /// S13（手术包二·顶层附加）：审计面从 6 个工具**扩到全部工具**——此前
    /// introspect / web_fetch 只有一句话（覆盖缺口），且 R5-9 断言集合本身漏了
    /// 三个工具（缺口会复发）。工具存在 ≠ 模型会用：新工具进注册表就必须过这道门。
    #[test]
    fn test_r59_tool_descriptions_have_five_elements() {
        use super::{
            BashTool, EditTool, GlobTool, GrepTool, IntrospectTool, PatchTool, ReadTool,
            TodoWriteTool, WebSearchTool, WebTool,
        };
        const FIVE: [&str; 5] = ["何时用", "何时不用", "示例", "边界", "错误解读"];
        // 与生产装配（codex-cli build_dispatcher）一致的**全量**清单——
        // 新增工具若漏进本清单，下面的数量断言会红（审计面不许缩水）。
        let tools: Vec<Box<dyn tool_runtime::Tool>> = vec![
            Box::new(ReadTool::new()),
            Box::new(BashTool::new()),
            Box::new(EditTool::new()),
            Box::new(PatchTool::new()),
            Box::new(GlobTool::new()),
            Box::new(GrepTool::new()),
            Box::new(TodoWriteTool::new()),
            Box::new(IntrospectTool::new()),
            Box::new(WebTool::new()),
            Box::new(WebSearchTool::new()),
        ];
        assert_eq!(tools.len(), 10, "审计面 = 全部内置工具（10 个）");
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

    // D-57：原 `test_coerce_args_*` 三例随被删的 `coerce_args` 一并移除
    // （它们只覆盖那个零调用方的旧机制；真实防线 `extract_str_arg` 另有测试）。
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

    /// 2026-10-01 修复回归（**先红后绿**）：家目录放行必须与"有无真实沙箱兜底"绑定。
    ///
    /// 修复前 `is_allowed_absolute_roots` **无条件**放行家目录，而立项理由（landlock
    /// 把 workspace 外置为只读）**只在 Linux 成立** → 非 Linux 上"放行家目录"
    /// 等于**真的能写家目录**。本测试直接验证 `allow_home` 两种取值的行为差异
    /// （用 inner 参数注入，不依赖宿主平台，测试确定性）。
    #[test]
    fn test_home_allowance_gated_by_real_sandbox() {
        let cwd = std::path::Path::new("/home/u/ws");
        // ① allow_home=false（= 无兜底的平台）：家目录内、cwd 外的路径**必须被拒**
        assert!(
            !is_allowed_inner("/home/u/other/x.txt", cwd, None, false),
            "无沙箱兜底时，家目录内的路径不得放行（修复前此处会放行）"
        );
        // ② 显式白名单不受 allow_home 影响（白名单是用户显式收敛，始终生效）
        assert!(is_allowed_inner("/data/x.txt", cwd, Some("/data"), false));
        // ③ cwd 内的绝对路径两种取值都放行（不受本修复影响）
        assert!(is_allowed_inner("/home/u/ws/a.txt", cwd, None, false));
        assert!(is_allowed_inner("/home/u/ws/a.txt", cwd, None, true));
        // ④ 相对路径永远放行（不受本修复影响）
        assert!(is_allowed_inner("rel.txt", cwd, None, false));

        // ⑤ 平台门控断言：兜底判定必须与"landlock 是否可用"一致
        assert_eq!(
            home_allowance_backed_by_sandbox(),
            cfg!(target_os = "linux"),
            "家目录放行的兜底前提只能是 Linux（landlock）"
        );
    }
}
