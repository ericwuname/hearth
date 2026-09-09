#!/bin/bash
# RC49: 规范链重跑（v0.2.18 干净二进制）——compact 腿以 archive 落盘为权威信号
cd /tmp/cfr_rc49
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_COMPACT_CHAR_THRESHOLD=7000
export HEARTH_TASK_TIMEOUT_SECS=100
echo "ARCHIVE_BEFORE=$(wc -l < /home/wutao/.config/hearth/archive/compacted.jsonl) lines, $(ls /home/wutao/.config/hearth/archive/ | wc -l) files"
hearth chat '创建一个 Rust 库项目 chainfree/，按以下顺序严格执行：
1) 用 bash 执行 mkdir -p chainfree/src
2) 用 write_file 创建 chainfree/Cargo.toml，内容为 [package] name=chainfree version=0.1.0 edition=2021
3) 用 write_file 创建 chainfree/src/lib.rs，包含 pub fn rev(s: &str) -> String { s.to_string() }（故意留 bug：未反转），以及测试模块 #[cfg(test)] mod tests { #[test] fn t_rev() { assert_eq!(rev("abc"), "cba"); } }
4) 用 bash 执行 cargo test --manifest-path chainfree/Cargo.toml —— 测试失败（受控失败），记录输出
5) 用 write_file 修复 rev：s.chars().rev().collect()
6) 用 bash 执行 cargo test --manifest-path chainfree/Cargo.toml 确认通过
7) 用 bash 执行 seq 1 1100（制造历史体积）
8) 用 write_file 创建 FREEZE_RESULT.txt，内容写 CHAIN_OK
每步完成后立即进入下一步。' \
  --budget 80 \
  --acceptance 'cmd: cargo test --manifest-path chainfree/Cargo.toml' \
  --acceptance 'file: FREEZE_RESULT.txt contains CHAIN_OK'
echo "N08A_EXIT=$?"
echo "ARCHIVE_MID=$(wc -l < /home/wutao/.config/hearth/archive/compacted.jsonl) lines, $(ls /home/wutao/.config/hearth/archive/ | wc -l) files"
SESSION=$(ls /tmp/cfr_rc49/.hearth/reports/ | head -1)
echo "RESUMING_SESSION=$SESSION"
export HEARTH_TASK_TIMEOUT_SECS=420
hearth resume "$SESSION"
echo "N08B_EXIT=$?"
echo "ARCHIVE_AFTER=$(wc -l < /home/wutao/.config/hearth/archive/compacted.jsonl) lines, $(ls /home/wutao/.config/hearth/archive/ | wc -l) files"
echo "RC49_DONE"
