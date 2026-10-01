//! 入库卫生门禁：**孤儿源文件**（在 `crates/*/src/` 下，却没有任何 `mod` 声明）。
//!
//! 为什么需要（D-87，2026-10-01, traecode）：本仓反复出现一类"手术残留"——
//! 某次重构/手术把**调用点**删了，但**文件本体**留在 `src/` 里没有任何 `mod` 声明，
//! 于是它**从不参与编译**：编译器看不见它（不会有 unused/dead_code 警告）、
//! 测试盖不到它、clippy 不管它——却会被维护者当成"现役代码"阅读与引用。
//! 实测命中：`crates/agent-core/src/cache_telemetry.rs`（216 行，是
//! `llm-gateway/src/cache_telemetry.rs` 的**陈旧副本**，从未被声明，从未被编译）。
//!
//! 判据（与 Cargo/rustc 的模块解析一致，宁可漏报不误报）：
//! - 跳过 crate 根 `lib.rs` / `main.rs`（它们不是"被声明的模块"）；
//! - 跳过 `src/bin/**`（Cargo **自动发现**为二进制目标）与 `src/examples/**`；
//! - `foo.rs` 由同级 `mod.rs` / `foo/mod.rs` / `lib.rs` / `main.rs` 中
//!   `mod foo;`（含 `pub`、`pub(crate)`、`pub(super)`）声明；
//!   `foo/bar.rs` 由 `foo.rs` 或 `foo/mod.rs` 声明；`foo/mod.rs` 由**其父目录**
//!   对应的文件声明；
//! - 接受 `mod r#foo;`（raw identifier，如 `pub mod r#loop;`）；
//! - 接受 `#[path = "..."] mod x;`（全仓当前 0 处，但显式豁免以免误报）。
//!
//! 文件头自报盲区：本门禁只查"**文件级**孤儿"。*块内*死代码
//! （未被调用的 `pub fn`、未被读取的字段等）不在其覆盖范围——那由 `clippy`、
//! 逐 crate 体检与 xray wiring 门禁各自负责。

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// 仓库根：`crates/codex-cli/tests/xxx.rs` → 上溯三级。
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// 收集某个文件里声明的模块名（含 `r#` 归一化）。
fn declared_modules(file: &Path) -> HashSet<String> {
    let Ok(text) = std::fs::read_to_string(file) else {
        return HashSet::new();
    };
    let mut out = HashSet::new();
    for line in text.lines() {
        let l = line.trim_start();
        // 只认模块声明行（`mod x;` / `pub mod x;` / `pub(crate) mod x;` …）
        let rest = l
            .strip_prefix("pub(crate) ")
            .or_else(|| l.strip_prefix("pub(super) "))
            .or_else(|| l.strip_prefix("pub "))
            .unwrap_or(l);
        let Some(rest) = rest.strip_prefix("mod ") else {
            continue;
        };
        // `mod x;` / `mod x { … }` 皆算；取标识符（去 `r#` 前缀）
        let name: String = rest
            .trim_start()
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '#')
            .collect();
        let name = name.strip_prefix("r#").unwrap_or(&name).to_string();
        if !name.is_empty() {
            out.insert(name);
        }
    }
    out
}

/// 该文件是否被 `#[path = "..."]` 形式引入（此类文件豁免）。
fn has_explicit_path_attr(file: &Path) -> bool {
    std::fs::read_to_string(file)
        .map(|t| t.contains("#[path ="))
        .unwrap_or(false)
}

/// 递归收集 `crates/*/src` 下的所有 `.rs`。
fn collect_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            // Cargo 自动发现的目标目录：不是"被 mod 声明的模块"
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name == "bin" || name == "examples" {
                continue;
            }
            collect_sources(&p, out);
        } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(p);
        }
    }
}

#[test]
fn no_orphan_source_files_in_crates() {
    let root = workspace_root();
    let Ok(crates) = std::fs::read_dir(root.join("crates")) else {
        panic!("crates/ 目录必须存在");
    };
    let mut sources = Vec::new();
    for c in crates.flatten() {
        let src = c.path().join("src");
        if src.is_dir() {
            collect_sources(&src, &mut sources);
        }
    }
    assert!(
        sources.len() >= 50,
        "扫描面过小（{} 个源文件）——门禁自身可能失效",
        sources.len()
    );

    let mut orphans: Vec<String> = Vec::new();
    for p in &sources {
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name == "lib.rs" || name == "main.rs" {
            continue; // crate 根
        }
        if has_explicit_path_attr(p) {
            continue; // #[path] 显式引入，豁免
        }
        // 找"声明方"候选文件 + 期望的模块名
        let (mod_name, declarers): (String, Vec<PathBuf>) = if name == "mod.rs" {
            let dir = p.parent().expect("mod.rs 必有父目录");
            let parent_name = dir
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let grand = dir.parent().expect("父目录必有上级");
            (
                parent_name.clone(),
                vec![
                    grand.join("mod.rs"),
                    grand.join(format!("{parent_name}.rs")),
                    grand.join("lib.rs"),
                    grand.join("main.rs"),
                ],
            )
        } else {
            let stem = name.trim_end_matches(".rs").to_string();
            let dir = p.parent().expect("源文件必有父目录").to_path_buf();
            let dir_name = dir
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            (
                stem.clone(),
                vec![
                    dir.join("mod.rs"),
                    dir.join(format!("{dir_name}.rs")),
                    dir.join("lib.rs"),
                    dir.join("main.rs"),
                ],
            )
        };

        let declared = declarers
            .iter()
            .filter(|d| d.exists())
            .any(|d| declared_modules(d).contains(&mod_name));
        if !declared {
            orphans.push(
                p.strip_prefix(&root)
                    .unwrap_or(p)
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }

    assert!(
        orphans.is_empty(),
        "发现**孤儿源文件**（在 src/ 下却无任何 `mod` 声明 ⇒ 从不参与编译，\
         编译器/测试/clippy 全都看不见它。要么补声明接线，要么删除）：{orphans:#?}"
    );
}
