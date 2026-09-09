// R3 (v0.1.2 任务书 P1): 编译期注入版本串 `0.1.2 (83fbf9d)`——`hearth --version`
// 一眼确认二进制代码版本（strings 查 Rust 标识符是 strip 假阴性——release 符号被剥，
// 函数名不在字符串表）。非 git 环境降级 "unknown"，不阻断构建。
// hash 来源优先级：① HEARTH_GIT_SHA env（VM 解压目录无 .git 时由构建者注入）
// ② git rev-parse（正常 git 仓库）。
//
// R7-3（P2 版本串 staleness，收官手册 §二）：96f4161 重建后 `--version` 仍显示
// 旧 commit——本脚本此前只挂 rerun-if-changed=build.rs，git rev-parse 结果变化
// 不触发重编。修复：盯 .git/HEAD **加** .git/refs/heads（HEAD 只是指向 ref 文件
// 的指针，commit 前进只动 ref 文件）+ .git/index（工作区内容变化）。
// HEARTH_GIT_SHA env 优先级保留（VM 解压目录无 .git 的部署路径不回归）；
// 非 git 环境降级 "unknown" 行为不变。
fn main() {
    let sha = std::env::var("HEARTH_GIT_SHA")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(git_short_sha)
        .unwrap_or_else(|| "unknown".to_string());
    let full = format!("{} ({})", env!("CARGO_PKG_VERSION"), sha);
    println!("cargo:rustc-env=HEARTH_VERSION={full}");
    // 变更触发重编（HEARTH_VERSION 可能因 checkout/环境变化）
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=HEARTH_GIT_SHA");
    // R7-3: git 状态变化必须触发重编——HEAD（分支切换/.detach）+ refs/heads
    // 目录（commit 前进/pull）+ index（本地新 commit 均动 index）。
    // 缺 .git 的部署目录：rerun-if-changed 指向不存在路径仅告警，不阻断。
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
    println!("cargo:rerun-if-changed=.git/index");
}

fn git_short_sha() -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}
