//! 门禁：**`docs/configuration.md` 里列出的环境变量，必须真的被代码读取**。
//!
//! 为什么需要（D-89，2026-10-01, traecode）：该文件是**面向用户/运维的活配置参考**，
//! 却与代码实际读取的旋钮**大面积脱节**——实测 8 个旋钮全仓零读取：
//!   · `RETRIEVER_ENABLED` / `LSP_ENABLED`：对应能力已在 P1-04（顶层裁决「删除」）整删；
//!   · `EMBED_API_KEY` / `EMBED_BASE_URL` / `EMBED_MODEL`：embedding 注入已随 v21.0 移除；
//!   · `CODEX_SANDBOX_ENABLED` / `CODEX_SANDBOX_WORKSPACE`：`create_sandbox` 只按平台选择，
//!     **不存在**任何 env 开关；
//!   · `SESSION_TTL_SECS`：真名是 **`OOM_TTL_SECS`**（默认同为 3600）——文档写错名字，
//!     照文档设置的用户**什么也不会发生**。
//! 用户照文档设了旋钮却毫无效果，是本仓反复出现的"声称≠实现"病灶在**用户文档面**的
//! 投射；本门禁把它钉死在**配置参考**这一份文件上（历史任务书不在射程内）。
//!
//! 判据（宁可漏报不误报）：
//! - 只解析 Markdown **表格行**（`| \`NAME\` | …`）里的**反引号包住的全大写标识符**；
//! - 该名字必须在 `crates/**` 里以 `env::var("NAME")` / `env::var_os("NAME")` 形式出现；
//! - `EXTERNAL_ALLOWLIST` 显式登记"由外部/三方消费、本仓确实不读"的变量（附理由）。
//!
//! 文件头自报盲区：① 只覆盖 `docs/configuration.md`（其他文档多为历史任务书，
//! 保留历史表述是有意的）；② 不校验默认值/语义是否与代码一致（那需要逐项人工核对）。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// 由本仓之外消费的变量（白名单，需给理由）。
const EXTERNAL_ALLOWLIST: &[(&str, &str)] = &[(
    "RUST_LOG",
    "`tracing` 生态通用变量，由 tracing-subscriber 直接读取，本仓代码不出现 env::var(\"RUST_LOG\")",
)];

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_rs(&p, out);
        } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(p);
        }
    }
}

/// 从配置参考的表格行里抽出反引号包住的全大写环境变量名。
fn documented_env_names(md: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in md.lines() {
        let l = line.trim();
        if !l.starts_with('|') {
            continue;
        }
        for seg in l.split('`').skip(1).step_by(2) {
            let name = seg.trim();
            let ok = !name.is_empty()
                && name.starts_with(|c: char| c.is_ascii_uppercase())
                && name
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
            if ok {
                out.insert(name.to_string());
            }
        }
    }
    out
}

#[test]
fn config_doc_env_vars_are_actually_read_by_code() {
    let root = workspace_root();
    let doc = root.join("docs").join("configuration.md");
    let md = std::fs::read_to_string(&doc).expect("docs/configuration.md 必须存在");
    let names = documented_env_names(&md);
    assert!(
        names.len() >= 5,
        "配置参考里解析到的环境变量过少（{} 个）——门禁自身可能失效：{names:?}",
        names.len()
    );

    // 全仓源码文本（用于 `env::var("NAME")` 判定）
    let mut sources = Vec::new();
    collect_rs(&root.join("crates"), &mut sources);
    let blob: String = sources
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .collect::<Vec<_>>()
        .join("\n");

    let mut ghosts: Vec<String> = Vec::new();
    for n in &names {
        if EXTERNAL_ALLOWLIST.iter().any(|(w, _)| w == n) {
            continue;
        }
        let read = blob.contains(&format!("env::var(\"{n}\")"))
            || blob.contains(&format!("env::var_os(\"{n}\")"));
        if !read {
            ghosts.push(n.clone());
        }
    }

    assert!(
        ghosts.is_empty(),
        "docs/configuration.md 列了这些环境变量，但**全仓代码从不读取**——\
         用户照文档设置将毫无效果。请二选一：① 代码里接线；② 从文档删除/更正为真名；\
         ③ 若确由外部消费，登记进 EXTERNAL_ALLOWLIST 并写明理由。幽灵旋钮：{ghosts:#?}"
    );
}
