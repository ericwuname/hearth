//! 门禁：**CI 的 Rust 工具链必须与仓库钉住的版本一致，且不得跟最新发行漂移**。
//!
//! 背景（D-91，2026-10-02, traecode）：CI 原用 `dtolnay/rust-toolchain@stable`，仓库内
//! 无固定工具链文件 ⇒ CI 跟随最新 stable。2026-10-01 stable 从 1.98 滚到 1.99（9-28 发布）
//! 后，CI 35 秒即红：`clippy::double_must_use`（1.99 新增）命中
//! `llm-gateway/src/provider.rs` 的 `#[async_trait]` 宏产物，而**本地 1.98.1 看不到**
//! ⇒ "本地门禁四件套全绿、CI 红、且本地无法复现"。这是本项目第二次 CI/本地口径分裂
//! （上一次 D-43 是 clippy flags 口径）。
//!
//! 本门禁把"钉版"变成**不可无声退回**的约束，三条判据：
//! 1. 仓库根必须有 `rust-toolchain.toml`，且 `channel` 是**具体版本号**（非 stable/nightly/beta）；
//! 2. 每个工作流里 `dtolnay/rust-toolchain@<X>` 的 `<X>` **不得**是浮动通道
//!    （`stable` / `nightly` / `beta`）；
//! 3. 工作流声明的 `toolchain:` 值必须与 `rust-toolchain.toml` 的 `channel` **完全一致**。
//!
//! 文件头自报盲区：只检查 `dtolnay/rust-toolchain` 这一种安装方式；若改用其它 action
//! 或 `rustup` 裸命令，本门禁不覆盖（彼时请同步扩展判据）。

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// 取 `channel = "x"` 的值。
fn read_channel(manifest: &str) -> Option<String> {
    manifest.lines().find_map(|l| {
        let l = l.trim();
        let rest = l.strip_prefix("channel")?;
        let rest = rest.trim_start().strip_prefix('=')?.trim();
        let v = rest.trim_matches(|c| c == '"' || c == '\'').trim();
        (!v.is_empty()).then(|| v.to_string())
    })
}

/// `flux` 是否为浮动通道（禁止）。
fn is_floating(channel: &str) -> bool {
    matches!(channel, "stable" | "nightly" | "beta")
}

fn yml_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.ends_with(".yml") || name.ends_with(".yaml") {
                out.push(p);
            }
        }
    }
    out
}

#[test]
fn ci_toolchain_is_pinned_and_matches_toolchain_file() {
    let root = workspace_root();

    // ① 仓库根钉版文件
    let manifest_path = root.join("rust-toolchain.toml");
    let manifest = std::fs::read_to_string(&manifest_path).unwrap_or_else(|e| {
        panic!(
            "缺少仓库根 `rust-toolchain.toml`（D-91 要求钉住工具链，否则 CI 会跟最新发行\
             漂移，造成本地不可复现的红）：{e}"
        )
    });
    let channel = read_channel(&manifest)
        .unwrap_or_else(|| panic!("rust-toolchain.toml 里找不到 `channel = \"…\"`"));
    assert!(
        !is_floating(&channel),
        "rust-toolchain.toml 的 channel 必须是**具体版本号**（如 1.99.0），当前为 `{channel}`\
         ——浮动通道会让门禁随上游发版随机变红"
    );

    // ② + ③ 工作流
    let wf_dir = root.join(".github").join("workflows");
    let files = yml_files(&wf_dir);
    assert!(
        !files.is_empty(),
        "未找到 .github/workflows/*.yml——门禁自身可能失效"
    );

    let mut seen_installer = 0usize;
    for f in &files {
        let text = std::fs::read_to_string(f).expect("读取工作流");
        for (i, line) in text.lines().enumerate() {
            let Some(pos) = line.find("dtolnay/rust-toolchain@") else {
                continue;
            };
            seen_installer += 1;
            let tag = line[pos + "dtolnay/rust-toolchain@".len()..]
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim();
            assert!(
                !is_floating(tag),
                "{}:{} 使用了浮动工具链 `{tag}`——必须改用具体版本（D-91：浮动会导致\
                 CI/本地口径分裂）",
                f.file_name().unwrap().to_string_lossy(),
                i + 1
            );
        }
        // 声明了 toolchain: 就必须与钉版一致
        let declared: Vec<&str> = text
            .lines()
            .filter_map(|l| l.trim().strip_prefix("toolchain:"))
            .map(|v| v.trim().trim_matches('"').trim())
            .collect();
        if !declared.is_empty() {
            assert!(
                declared.iter().all(|d| *d == channel),
                "{} 声明的 toolchain {:?} 与 rust-toolchain.toml 的 `{channel}` 不一致——\
                 两处必须同步（否则 CI 与本地又是两套口径）",
                f.file_name().unwrap().to_string_lossy(),
                declared
            );
        }
    }
    assert!(
        seen_installer > 0,
        "没有找到任何 `dtolnay/rust-toolchain@…` 安装步骤——若已换用其它方式安装 Rust，\
         请同步扩展本门禁判据（不要让它静默失效）"
    );
}
