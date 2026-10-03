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
//! 判据：`.gitignore` 必须逐行包含下面这些目录规则（比对时归一化掉行首 `/` 与行尾 `/`
//! ——推荐**根锚定**写法 `/x/`，以免误伤同名子目录如 `crates/observer/`）。
//!
//! D-145（2026-10-04, traecode）：清单曾**漏列** `memory/`（`MEMORY_DIR` 默认 `./memory`，
//! 含会话/文明线/工作线/经验库 JSONL，内容含目标与失败明文）与 `observer/`
//! （`CODEX_OBSERVER_DIR` 默认 `./observer`）——实测在仓库根跑一次 CLI 即生成
//! `memory/experience.jsonl` 并被 `git status` 列为 untracked。
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
    // D-145（2026-10-04, traecode）：这两个默认落盘目录此前**漏列**——
    // 在仓库根跑一次 CLI/service 就会生成，内容含目标/失败明文。
    (
        "memory/",
        "MEMORY_DIR 默认 `./memory`：会话/文明线/工作线/经验库 JSONL（service/main.rs:576、\
         codex-cli/run_local.rs:217「`<cwd>/memory/experience.jsonl`」）",
    ),
    (
        "observer/",
        "CODEX_OBSERVER_DIR 默认 `./observer`：每小时 `daily-<日期>.jsonl` + 会话结束 `reports/<sid>/`\
         （service/main.rs:769）",
    ),
];

/// D-113（2026-10-02, traecode）：**解释器字节码缓存**同样是运行期产物。
///
/// 背景：此前 `.gitignore` 只窄忽略 `window-framework/src/__pycache__/`，导致
/// `bench/`、`docs/data/`、`ember/` 三处的 `__pycache__/*.pyc`（共 4 个文件）
/// 被误跟踪——它们是 CPython 自动生成的字节码，与源码无关、可随时重建。
/// 该清单只需覆盖仓库内实际出现过的解释器缓存形态。
const INTERPRETER_CACHE_PATTERNS: &[&str] = &["__pycache__/", "*.pyc"];

fn gitignore_rules() -> Vec<String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let text = std::fs::read_to_string(root.join(".gitignore")).expect(".gitignore 必须存在");
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect()
}

#[test]
fn runtime_product_dirs_are_gitignored() {
    let rules = gitignore_rules();

    let missing: Vec<&str> = RUNTIME_PRODUCT_DIRS
        .iter()
        .filter(|(dir, _)| {
            // D-145：规则允许**根锚定**写法（行首 `/`）——`/observer/` 不会误伤
            // `crates/observer/`（未锚定的 `observer/` 会匹配任意层级同名目录）。
            // 比对时两侧都归一化掉行首 `/` 与行尾 `/`。
            let want = dir.trim_start_matches('/').trim_end_matches('/');
            !rules
                .iter()
                .any(|r| r.trim_start_matches('/').trim_end_matches('/') == want)
        })
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

#[test]
fn interpreter_caches_are_gitignored() {
    let rules = gitignore_rules();
    let missing: Vec<&str> = INTERPRETER_CACHE_PATTERNS
        .iter()
        .filter(|pat| !rules.iter().any(|r| r == *pat))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "这些**解释器字节码缓存**规则没被 .gitignore 忽略：{missing:?}\n\
         它们是解释器自动生成的运行期产物，入库只会带来 git 噪声。\
         若确有新形态（如别的解释器缓存）加入，请同步本门禁清单。"
    );
}
