#!/bin/bash
# P2-LR Node 09: Long-run Product Task A (textkit, logic-error topology)
cd /tmp/lr_n09
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_TASK_TIMEOUT_SECS=600
hearth chat '创建一个 Rust 库项目 textkit/，按以下顺序严格执行：
1) 用 bash 执行 mkdir -p textkit/src
2) 用 write_file 创建 textkit/Cargo.toml，内容为 [package] name=textkit version=0.1.0 edition=2021
3) 用 write_file 创建 textkit/src/lib.rs，实现两个函数：
   pub fn normalize_whitespace(s: &str) -> String { s.replace("  ", " ") }（初始实现：只处理双空格——已知不完善，制表符/换行未归一）
   pub fn count_words(s: &str) -> usize { normalize_whitespace(s).split_whitespace().count() }
   以及测试模块：
   #[cfg(test)] mod tests {
       #[test] fn t_norm() { assert_eq!(normalize_whitespace("a\\tb  c\\nd"), "a b c d"); }
       #[test] fn t_count() { assert_eq!(count_words("a\\tb  c"), 3); }
   }
4) 用 bash 执行 cargo test --manifest-path textkit/Cargo.toml —— 测试会失败（t_norm），记录失败输出
5) 用 write_file 修复 normalize_whitespace：用 split_whitespace 后 join 单空格（如 s.split_whitespace().collect::<Vec<_>>().join(" ")）
6) 用 bash 再执行 cargo test --manifest-path textkit/Cargo.toml 确认通过
7) 用 write_file 创建 textkit/README.md，内容为 "textkit: whitespace normalization utilities"
每步完成后立即进入下一步。' \
  --budget 80 \
  --acceptance 'cmd: cargo test --manifest-path textkit/Cargo.toml' \
  --acceptance 'file: textkit/src/lib.rs contains split_whitespace' \
  --acceptance 'file: textkit/README.md contains textkit'
echo "N09_EXIT=$?"
