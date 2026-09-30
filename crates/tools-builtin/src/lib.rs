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

/// D-55（2026-10-01）：**有界读取 HTTP 响应体**的字节上限。
///
/// `resp.text()` 会把**整份响应**读进内存（reqwest 无内建上限），随后才被截到
/// `MAX_BODY_CHARS`（8000 字符）——超大/恶意响应可致 OOM。1 MiB ≈ 输出上限的 130 倍，
/// 足以容纳正常文档页的 HTML 标记，同时把"失控响应"从 OOM 退化为"截断 + 留痕"。
pub(crate) const MAX_BODY_BYTES: usize = 1024 * 1024;

/// D-55：有界读取 reqwest 响应体（**网络侧**的"无界读入"落点，与已修的 7 处文件/管道
/// 落点同族）。
///
/// - 先用 `Content-Length`（若有）**提前拒绝**超大响应——不必下载就知道超限；
/// - 否则用 `Response::chunk()` 流式累加，**最多保留 `cap` 字节**；超出即停读并丢弃余量
///   （HTTP 与管道不同：丢弃后续字节不会让写端死锁，只是关闭连接）；
/// - 截断处退到最后一个合法 UTF-8 边界。本仓 reqwest **未启用 `charset` feature**，
///   `.text()` 本就是 UTF-8 lossy → `from_utf8_lossy` 与旧行为等价；
/// - 截断**留痕**（不静默）。
pub(crate) async fn read_body_capped(
    mut resp: reqwest::Response,
    cap: usize,
) -> anyhow::Result<(String, bool)> {
    if let Some(len) = resp.content_length() {
        if len > cap as u64 {
            anyhow::bail!(
                "响应体过大：Content-Length {len} 字节 > 上限 {cap} 字节——已拒绝（避免整份读入内存）"
            );
        }
    }

    let mut buf: Vec<u8> = Vec::with_capacity(cap.min(64 * 1024));
    let mut truncated = false;
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| anyhow::anyhow!("读 body 失败: {e}"))?
    {
        let room = cap.saturating_sub(buf.len());
        if chunk.len() > room {
            buf.extend_from_slice(&chunk[..room]);
            truncated = true;
            break;
        }
        buf.extend_from_slice(&chunk);
    }

    let text = if truncated {
        match String::from_utf8(buf) {
            Ok(s) => s,
            Err(e) => {
                let valid = e.utf8_error().valid_up_to();
                let bytes = e.into_bytes();
                String::from_utf8_lossy(&bytes[..valid]).into_owned()
            }
        }
    } else {
        String::from_utf8_lossy(&buf).into_owned()
    };

    if truncated {
        tracing::warn!(cap, "HTTP 响应体超过上限，已截断（丢弃余量）");
    }
    Ok((text, truncated))
}

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

    // ── D-55：HTTP 响应体有界读取 ──

    /// 起一个一次性本地 HTTP 服务（回固定 body），返回 URL。
    async fn serve_once(content_length: Option<usize>, body_len: usize) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            if let Ok((mut sock, _)) = listener.accept().await {
                let mut req = [0u8; 1024];
                let _ = sock.read(&mut req).await;
                let cl = match content_length {
                    Some(n) => format!("Content-Length: {n}\r\n"),
                    None => String::new(),
                };
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n{cl}Connection: close\r\n\r\n"
                );
                let _ = sock.write_all(head.as_bytes()).await;
                let chunk = vec![b'a'; 64 * 1024];
                let mut left = body_len;
                while left > 0 {
                    let n = left.min(chunk.len());
                    if sock.write_all(&chunk[..n]).await.is_err() {
                        break;
                    }
                    left -= n;
                }
                let _ = sock.flush().await;
            }
        });
        format!("http://{addr}/")
    }

    /// 先红后绿（D-55）：`Content-Length` 超限必须**提前拒绝**（不下载整份）。
    /// 修复前走 `resp.text()` → 整份入内存（本测会拿到 Ok 而非 Err）。
    #[tokio::test]
    async fn test_d55_body_capped_rejects_oversized_content_length() {
        let url = serve_once(Some(5_000_000), 5_000_000).await;
        let resp = reqwest::get(&url).await.unwrap();
        let err = read_body_capped(resp, 1024)
            .await
            .expect_err("Content-Length 超限必须被提前拒绝（不整份读入）");
        assert!(err.to_string().contains("响应体过大"), "got: {err}");
    }

    /// 正常小响应原样返回、不标截断（保证修复不误伤）。
    #[tokio::test]
    async fn test_d55_body_capped_small_body_ok() {
        let url = serve_once(Some(5), 5).await;
        let resp = reqwest::get(&url).await.unwrap();
        let (body, truncated) = read_body_capped(resp, 1024).await.unwrap();
        assert_eq!(body, "aaaaa");
        assert!(!truncated, "未超限不得标截断");
    }
}
