//! 门禁：**README 里引用的仓库内文件必须真实存在**（D-110，2026-10-02, traecode）。
//!
//! 病灶：README §1 的安装命令曾写作 `sh install.sh`，而脚本实际在 `bench/install.sh`
//! —— 仓库根没有 `install.sh`，用户照抄即 `No such file`（文档漂移在**用户第一触点**
//! 上的典型表现）。同批还发现 seccomp 白名单条数、cgroup 数值、`chat` 示例缺 goal、
//! 二进制名等多处与代码不符（已一并订正）。
//!
//! 判据（宁可漏报不误报）：`README.md` 中
//! ① Markdown 相对链接 `](path)` 指向的文件/目录必须存在；
//! ② `sh <path>` / `bash <path>` 里被执行的脚本必须存在。
//!
//! 跳过：绝对路径（`/usr/...`）、`~` 开头、含 `://`（URL）、`target/` 下的构建产物、
//! 纯锚点（`#...`）、以及带占位符 `<>`/`...` 的示例路径。
//!
//! 文件头自报盲区：① 只扫 `README.md`（其他文档多为历史任务书，保留历史表述是有意的）；
//! ② 只查"文件是否存在"，**不校验**文中数值/行为描述是否与代码一致（那需要逐项人工核对，
//! 本卡已手工订正 seccomp 条数与 cgroup 数值）。

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// 是否是"应当存在的仓库内相对路径"。
fn is_checked_rel(p: &str) -> bool {
    if p.is_empty() {
        return false;
    }
    if p.starts_with('/') || p.starts_with('~') || p.starts_with('#') {
        return false;
    }
    if p.contains("://") {
        return false;
    }
    if p.starts_with("target/") {
        return false;
    }
    // 占位符示例（<ver>、... ）不算真实路径
    if p.contains('<') || p.contains('>') || p.contains("...") {
        return false;
    }
    p.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
}

/// 抽出 Markdown 相对链接目标。
fn md_link_targets(md: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = md.as_bytes();
    let mut i = 0;
    while let Some(rel) = md[i..].find("](") {
        let start = i + rel + 2;
        if let Some(end_rel) = md[start..].find(')') {
            let target = &md[start..start + end_rel];
            let target = target.split_whitespace().next().unwrap_or(target);
            out.push(target.to_string());
            i = start + end_rel;
        } else {
            break;
        }
        if i >= bytes.len() {
            break;
        }
    }
    out
}

/// 抽出 `sh <script>` / `bash <script>` 里被执行的脚本路径。
fn shell_script_targets(md: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in md.lines() {
        for prefix in ["sh ", "bash ", "sh -", "bash -"] {
            for tok in line.split(prefix).skip(1) {
                if let Some(first) = tok.split_whitespace().next() {
                    let t = first.trim_matches(|c: char| matches!(c, '"' | '\'' | '`' | ';'));
                    if t.ends_with(".sh") {
                        out.push(t.to_string());
                    }
                }
            }
        }
    }
    out
}

#[test]
fn readme_referenced_repo_files_exist() {
    let root = root();
    let readme = root.join("README.md");
    let md = std::fs::read_to_string(&readme).expect("README.md 必须存在");

    let mut checked = 0usize;
    let mut missing: Vec<String> = Vec::new();

    let consider = |p: &str, missing: &mut Vec<String>, checked: &mut usize| {
        if !is_checked_rel(p) {
            return;
        }
        *checked += 1;
        if !Path::new(&root).join(p).exists() {
            missing.push(p.to_string());
        }
    };

    for t in md_link_targets(&md) {
        consider(&t, &mut missing, &mut checked);
    }
    for t in shell_script_targets(&md) {
        consider(&t, &mut missing, &mut checked);
    }

    assert!(
        checked >= 3,
        "README 里只解析出 {checked} 条可校验路径——门禁自身可能失效（解析规则与 README 写法脱节）"
    );
    assert!(
        missing.is_empty(),
        "README 引用了**不存在**的仓库内文件（用户照抄即失败）：{missing:?}\n\
         请二选一：① 修 README 的路径；② 补上缺失的文件。"
    );
}
