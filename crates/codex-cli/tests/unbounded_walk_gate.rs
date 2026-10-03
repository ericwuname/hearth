//! 门禁：工作区**递归目录遍历**不得跟随符号链接、且必须带深度上限（D-141，2026-10-04, traecode）。
//!
//! 背景：本仓有一条"**无界遍历**"缺陷族——递归/迭代遍历目录时用 `Path::is_dir()`
//! （在 Windows 上还会跟随 junction、在 Linux 上跟随 symlink）判目录，**既不识别软链
//! 成环、也无深度守卫**：一旦工作区里出现 symlink/junction 环（自指或互指），递归版会
//! **栈溢出**（进程 abort）、迭代版会**无限入栈 → OOM/挂死**。仓里其实早有正确范式
//! `crates/tools-builtin/src/glob.rs::walk_readonly`（用 `symlink_metadata` 不跟随 +
//! `depth > 64` 守卫），本轮抓到的两处**落后于既有防线**：
//! ① `crates/agent-core/src/loop.rs::snapshot_workspace`（写盘快照的递归遍历，
//!    `p.is_dir()` 跟随软链 + 无深度守卫 ⇒ symlink 环 → 栈溢出）；
//! ② `crates/project-xray/src/facts.rs::collect_rs_files`（`xray` 统计 .rs 的迭代遍历，
//!    `path.is_dir()` 跟随软链 + 无深度守卫 ⇒ 环 → 栈永不清空、`out`/`stack` 无界增长 → OOM）。
//!
//! 判据（源码级，**只扫上述两个精确函数体**，宁可漏报不误报）：逐行剥掉 `//` 注释后，
//! 对目标函数体断言——
//!   ① **必须**出现"不跟随符号链接"的判定标记：`snapshot_workspace` 用 `symlink_metadata`、
//!      `collect_rs_files` 用 `DirEntry::file_type()`；
//!   ② **必须**出现深度上限守卫（同时含 `depth` 与 `> 64`）；
//!   ③ **不得**出现"跟随软链"的 `.is_dir()`——即接收者不是 metadata 绑定
//!      （`md`/`meta`/`metadata`/`ft`/`file_type`…）的 `.is_dir()` 调用一律判红。
//!
//! 文件头自报盲区：
//! ① 只覆盖上面**两处**遍历；本仓其它目录遍历（如 `glob.rs` 已是安全范式）不在此门禁范围。
//! ② 只做**文本级**扫描（剥 `//` 注释后按字节做花括号配对取函数体），不做语义分析；
//!    若函数体内**字符串字面量**含 `{`/`}` 会误判配对（当前两处均无此写法）。
//! ③ 接受 `symlink_metadata`/`file_type()` 标记**存在**即放行，**无法**区分函数体里另外
//!    出现的跟随式调用（如 `std::fs::metadata` 或 `entry.path().is_dir()` 若接收者恰在
//!    SAFE 名单内）——宁可漏报不误报。

use std::path::PathBuf;

/// 仓库根：`crates/codex-cli/tests/xxx.rs` → 上溯三级。
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// 逐行剥掉 `//` 之后的部分（注释里提到旧写法不算回归）。
fn code_part(line: &str) -> &str {
    match line.find("//") {
        Some(i) => &line[..i],
        None => line,
    }
}

/// 剥掉全文的 `//` 行尾注释（保留行结构）。
fn strip_comments(text: &str) -> String {
    text.lines().map(code_part).collect::<Vec<_>>().join("\n")
}

/// 取出 `sig` 所指函数的函数体（从签名后第一个 `{` 起做花括号配对）。
fn fn_body(text: &str, sig: &str) -> String {
    let start = text
        .find(sig)
        .unwrap_or_else(|| panic!("找不到函数签名 `{sig}`（可能被重命名/移除）"));
    let after = &text[start..];
    let open = after
        .find('{')
        .unwrap_or_else(|| panic!("函数签名 `{sig}` 后应有 `{{`"));
    let mut depth = 0i32;
    let mut end = None;
    for (i, b) in after.as_bytes().iter().enumerate().skip(open) {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let end = end.unwrap_or_else(|| panic!("函数 `{sig}` 的花括号不配对"));
    after[open + 1..end].to_string()
}

/// 找出「跟随符号链接」的 `.is_dir()` 调用的接收者标识符。
///
/// 链式调用（`…file_type().is_dir()`，接收者以 `)` 结尾）视为不跟随，跳过；
/// 接收者在 SAFE 名单内（来自 metadata 绑定）视为安全，跳过；其余判为可疑。
fn unsafe_dir_receivers(code: &str) -> Vec<String> {
    const SAFE: &[&str] = &[
        "md",
        "meta",
        "metadata",
        "m",
        "ft",
        "filetype",
        "file_type",
        "symlink_metadata",
    ];
    let mut out = Vec::new();
    let mut rest = code;
    while let Some(idx) = rest.find(".is_dir(") {
        let before = rest[..idx].trim_end();
        // 以 `)` 结尾 ⇒ 方法链（如 `…file_type()`）——视为不跟随。
        if !before.ends_with(')') {
            let recv: String = before
                .chars()
                .rev()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            if !recv.is_empty() && !SAFE.contains(&recv.as_str()) {
                out.push(recv);
            }
        }
        rest = &rest[idx + ".is_dir(".len()..];
    }
    out
}

struct Target {
    /// 相对仓库根的源文件路径。
    path: &'static str,
    /// 函数签名标记（定位函数体）。
    sig: &'static str,
    /// 函数体内必须出现的"不跟随符号链接"判定标记。
    must_contain: &'static [&'static str],
}

#[test]
fn workspace_walks_do_not_follow_symlinks_and_are_depth_bounded() {
    let root = workspace_root();
    let targets = [
        Target {
            path: "crates/agent-core/src/loop.rs",
            sig: "fn snapshot_workspace(",
            must_contain: &["symlink_metadata"],
        },
        Target {
            path: "crates/project-xray/src/facts.rs",
            sig: "fn collect_rs_files(",
            must_contain: &["file_type()"],
        },
    ];

    let mut problems: Vec<String> = Vec::new();
    for t in &targets {
        let file = root.join(t.path);
        assert!(file.is_file(), "缺少目标源文件：{}", file.display());
        let raw = std::fs::read_to_string(&file).expect("读源文件失败");
        let text = strip_comments(&raw);
        let body = fn_body(&text, t.sig);

        for marker in t.must_contain {
            if !body.contains(marker) {
                problems.push(format!(
                    "{}：遍历体缺少「不跟随符号链接」判定 `{marker}`",
                    t.path
                ));
            }
        }
        if !(body.contains("depth") && body.contains("> 64")) {
            problems.push(format!(
                "{}：遍历体缺少深度上限守卫（期望同时出现 `depth` 与 `> 64`）",
                t.path
            ));
        }
        for recv in unsafe_dir_receivers(&body) {
            problems.push(format!(
                "{}：`{recv}.is_dir()` **跟随符号链接**（软链/junction 成环 → 无界递归/遍历）",
                t.path
            ));
        }
    }

    assert!(
        problems.is_empty(),
        "工作区递归遍历必须**不跟随符号链接**（`symlink_metadata` / `DirEntry::file_type()`）\
         且带**深度上限**（`depth > 64`）——防软链/junction 成环导致栈溢出或 OOM。\
         请镜像既有安全范式 `crates/tools-builtin/src/glob.rs::walk_readonly`。问题：{problems:#?}"
    );
}
