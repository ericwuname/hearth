//! facts: workspace 事实采集（X1）。
//!
//! 只读扫描：解析根 Cargo.toml 的 workspace.members，逐 crate 统计
//! .rs 文件数 / 代码行数 / 测试声明数（`#[test]` + `#[tokio::test]`）。
//! 产出 facts.json —— 采集与呈现解耦的唯一中间格式（schema 带版本号）。

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// facts.json 的 schema 版本。破坏性变更时递增。
pub const FACTS_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateFacts {
    /// crate 目录名（如 `agent-core`）。
    pub name: String,
    /// workspace 相对路径（如 `crates/agent-core`）。
    pub path: String,
    pub rs_files: u64,
    pub loc: u64,
    pub tests: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Facts {
    pub schema: u32,
    /// 生成时刻（unix 秒）。用 SystemTime 而非手算时间戳。
    pub generated_unix: u64,
    pub workspace_members: usize,
    pub rs_files: u64,
    pub total_loc: u64,
    /// 测试声明数（`#[test]` / `#[tokio::test]` 出现次数）。
    /// 注意与 cargo test 实跑数可能有出入（ignore/cfg 等），以实跑为准。
    pub test_declarations: u64,
    pub crates: Vec<CrateFacts>,
}

/// 扫描 `root`（workspace 根）产出 Facts。
pub fn scan(root: &Path) -> Result<Facts> {
    let manifest_path = root.join("Cargo.toml");
    let manifest_text = fs::read_to_string(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let manifest: toml::Value = toml::from_str(&manifest_text)
        .with_context(|| format!("parse {}", manifest_path.display()))?;

    let members: Vec<String> = manifest
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .context("Cargo.toml has no [workspace] members")?
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();

    let mut crates = Vec::new();
    let mut rs_files = 0u64;
    let mut total_loc = 0u64;
    let mut test_declarations = 0u64;

    for member in &members {
        let dir = root.join(member);
        let mut cf = CrateFacts {
            name: member.rsplit('/').next().unwrap_or(member).to_string(),
            path: member.clone(),
            rs_files: 0,
            loc: 0,
            tests: 0,
        };
        for file in collect_rs_files(&dir) {
            let Ok(content) = fs::read_to_string(&file) else {
                continue;
            };
            cf.rs_files += 1;
            cf.loc += content.lines().count() as u64;
            cf.tests += count_test_declarations(&content);
        }
        rs_files += cf.rs_files;
        total_loc += cf.loc;
        test_declarations += cf.tests;
        crates.push(cf);
    }

    let generated_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    Ok(Facts {
        schema: FACTS_SCHEMA,
        generated_unix,
        workspace_members: members.len(),
        rs_files,
        total_loc,
        test_declarations,
        crates,
    })
}

/// 递归收集目录下全部 .rs 文件（跳过 target/.git，避免统计编译产物）。
fn collect_rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                if name == "target" || name == ".git" {
                    continue;
                }
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// 统计测试声明：行首（去缩进后）以 `#[test]` 或 `#[tokio::test` 开头。
fn count_test_declarations(content: &str) -> u64 {
    content
        .lines()
        .filter(|line| {
            let t = line.trim_start();
            t.starts_with("#[test]") || t.starts_with("#[tokio::test")
        })
        .count() as u64
}

/// 序列化 Facts 并写入 `out`（自动建父目录）。
pub fn write_facts(facts: &Facts, out: &Path) -> Result<()> {
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(facts)?;
    fs::write(out, json).with_context(|| format!("write {}", out.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_test_declarations() {
        let src = "fn a() {}\n#[test]\nfn t1() {}\n  #[tokio::test]\n  async fn t2() {}\n// #not a test\n";
        assert_eq!(count_test_declarations(src), 2);
    }

    #[test]
    fn test_scan_fixture_workspace() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/a\"]\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("crates/a/src")).unwrap();
        fs::write(
            root.join("crates/a/src/lib.rs"),
            "pub fn f() {}\n#[test]\nfn t() { assert!(true); }\n",
        )
        .unwrap();

        let facts = scan(root).unwrap();
        assert_eq!(facts.workspace_members, 1);
        assert_eq!(facts.rs_files, 1);
        assert_eq!(facts.test_declarations, 1);
        assert_eq!(facts.total_loc, 3);
        assert_eq!(facts.crates[0].name, "a");
    }

    #[test]
    fn test_scan_skips_target_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/a\"]\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("crates/a/src")).unwrap();
        fs::create_dir_all(root.join("crates/a/target/debug")).unwrap();
        fs::write(root.join("crates/a/src/lib.rs"), "pub fn f() {}\n").unwrap();
        fs::write(
            root.join("crates/a/target/debug/junk.rs"),
            "// build artifact, must not be counted\n",
        )
        .unwrap();

        let facts = scan(root).unwrap();
        assert_eq!(facts.rs_files, 1, "target/ artifacts must be excluded");
    }
}
