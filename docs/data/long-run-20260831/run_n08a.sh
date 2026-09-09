#!/bin/bash
# P2-LR Node 08: Compact + Resume canonical chain (单跑成链，S-4) — phase 1 interrupt
cd /tmp/lr_n08
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_COMPACT_CHAR_THRESHOLD=8000
export HEARTH_TASK_TIMEOUT_SECS=95
hearth chat '创建一个 Rust 库项目 chainlib/，按以下顺序严格执行：
1) 用 bash 执行 mkdir -p chainlib/src
2) 用 write_file 创建 chainlib/Cargo.toml，内容为 [package] name=chainlib version=0.1.0 edition=2021
3) 用 write_file 创建 chainlib/src/lib.rs，包含 pub fn dbl(a: i64) -> i64 { a * 3 }（故意留 bug），以及测试模块 #[cfg(test)] mod tests { #[test] fn t_dbl() { assert_eq!(dbl(4), 8); } }
4) 用 bash 执行 cargo test --manifest-path chainlib/Cargo.toml —— 测试失败（受控失败），记录输出
5) 用 write_file 修复 dbl（改为 a * 2）
6) 用 bash 执行 cargo test --manifest-path chainlib/Cargo.toml 确认通过
7) 用 bash 执行 seq 1 1200（制造历史体积）
8) 用 write_file 创建 CHAIN_RESULT.txt，内容写 CHAIN_RECOVERED
每步完成后立即进入下一步。' \
  --budget 80 \
  --acceptance 'cmd: cargo test --manifest-path chainlib/Cargo.toml' \
  --acceptance 'file: CHAIN_RESULT.txt contains CHAIN_RECOVERED'
echo "N08A_EXIT=$?"
