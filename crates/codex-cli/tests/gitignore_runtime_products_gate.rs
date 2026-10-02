//! 门禁：**产品自身的默认落盘目录必须被 `.gitignore` 忽略**（D-112，2026-10-02, traecode）。
//!
//! 背景：CLI 默认在 **cwd** 下生成若干运行期产物目录（报告 / 断点镜像 / 快照 /
//! 会话 JSONL / transcript）。它们**不是源码**，内容却含用户目标文本与运行叙述
//! ⇒ 被误提交既是 `git status` 污染，也是"明文内容入库"的入口（D-5 的
//! `.hearth-diag/` 事故即此类：那次是文档归档提交把它连内容一起带进了库）。
//!
//! 实测触发条件极低：在仓库根跑一次 `hearth` 即可让 `git status` 冒出这些目录。
//! 此前 `.gitignore` 只忽略了 `.hearth-diag/`，其余全裸奔。
//!
//! 判据：`.gitignore` 必须逐行包含下面这些目录规则（精确匹配行首，允许尾随 `/`）。
//!
//! 文件头自报盲区：① 只校验"这几个已知默认目录"，不校验 `HEARTH_*_DIR` 自定义路径
//! （那是用户自选位置，管不着）；② 只做文本级规则存在性检查，不真的跑 `git check-ignore`
//! （避免门禁依赖 git 可执行文件）。

use std::path::PathBuf;

/// (目录规则, 来源：谁在 cwd 下生成它)
const RUNTIME_PRODUCT_DIRS: &[(&str, &str)] = &[
    (
        ".hearth/",
        "report.rs 的 run 报告 + session_store.rs 的断点镜像",
    ),
    (".hearth_snapshots/", "snapshot_store.rs 的回滚快照"),
    (".hearth_sessions/", "session_store.rs 的会话 JSONL"),
    ("results/", "transcript.rs 的 run transcript JSONL"),
];

#[test]
fn runtime_product_dirs_are_gitignored() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let text = std::fs::read_to_string(root.join(".gitignore")).expect(".gitignore 必须存在");

    let rules: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    let missing: Vec<&str> = RUNTIME_PRODUCT_DIRS
        .iter()
        .filter(|(dir, _)| !rules.contains(&dir.trim_end_matches('/')) && !rules.contains(dir))
        .map(|(dir, _)| *dir)
        .collect();

    assert!(
        missing.is_empty(),
        "这些**产品默认落盘目录**没被 .gitignore 忽略：{missing:?}\n\
         它们由 hearth 在 cwd 下自动生成、内容是运行期产物（含目标文本/叙述），\
         在仓库根跑一次 CLI 就会污染 git status 并制造明文入库入口。\
         请为每一项补一条忽略规则；若确有新目录加入，请同步本门禁的清单。"
    );
}
