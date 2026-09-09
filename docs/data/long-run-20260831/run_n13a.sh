#!/bin/bash
# P2-LR Node 13: Cross-compaction stress — phase 1 (failure+repair+compact#1+interrupt)
cd /tmp/lr_n13
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_COMPACT_CHAR_THRESHOLD=6000
export HEARTH_TASK_TIMEOUT_SECS=120
hearth chat '创建一个 Rust 库项目 mathnotes/，按以下顺序严格执行：
1) 用 bash 执行 mkdir -p mathnotes/src
2) 用 write_file 创建 mathnotes/Cargo.toml，内容为 [package] name=mathnotes version=0.1.0 edition=2021
3) 用 write_file 创建 mathnotes/src/lib.rs，包含：
   pub fn add(a: i64, b: i64) -> i64 { a + b + 1 }（故意 bug）
   pub fn shout(s: &str) -> String { s.to_string() }（故意 bug：未大写无感叹号）
   测试模块：#[cfg(test)] mod tests { use super::*; #[test] fn t_add() { assert_eq!(add(2, 3), 5); } #[test] fn t_shout() { assert_eq!(shout("hi"), "HI!"); } }
4) 用 bash 执行 cargo test --manifest-path mathnotes/Cargo.toml —— 两个测试失败，记录输出
5) 用 write_file 修复 lib.rs（add 改 a+b；shout 改 format!("{}!", s.to_uppercase())）
6) 用 bash 执行 seq 1 900（制造历史体积）
7) 用 bash 执行 seq 1000 1900（继续制造体积）
每步完成后立即进入下一步。' \
  --budget 80 \
  --acceptance 'cmd: cargo test --manifest-path mathnotes/Cargo.toml'
echo "N13A_EXIT=$?"
