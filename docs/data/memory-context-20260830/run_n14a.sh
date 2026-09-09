#!/bin/bash
# P2 Node 14 Task A: long product task with controlled compaction + interrupt + resume
# Phase A1: run with low threshold (forced compaction) and short deadline (interrupt)
cd /tmp/p2_n14
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_COMPACT_CHAR_THRESHOLD=8000
export HEARTH_TASK_TIMEOUT_SECS=100
hearth chat '创建一个 Rust 库项目 calc2/，按以下顺序严格执行：
1) 用 bash 执行 mkdir -p calc2/src
2) 用 write_file 创建 calc2/Cargo.toml，内容为 [package] name=calc2 version=0.1.0 edition=2021
3) 用 write_file 创建 calc2/src/lib.rs，包含 pub fn dbl(a: i64) -> i64 { a * 3 }（故意留 bug：应乘 2），以及测试模块 #[cfg(test)] mod tests { #[test] fn t_dbl() { assert_eq!(dbl(4), 8); } }
4) 用 bash 执行 cargo test --manifest-path calc2/Cargo.toml —— 这次测试会失败（受控失败），记录失败输出
5) 用 write_file 修复 dbl 实现（改为 a * 2），测试保持不变
6) 用 bash 再执行 cargo test --manifest-path calc2/Cargo.toml 确认通过
7) 用 write_file 创建 RESULT.txt，内容写 N14_RECOVERED
每步执行完立即进入下一步，不要做额外验证。' \
  --budget 80 \
  --acceptance 'cmd: cargo test --manifest-path calc2/Cargo.toml' \
  --acceptance 'file: RESULT.txt contains N14_RECOVERED'
echo "N14A_EXIT=$?"
