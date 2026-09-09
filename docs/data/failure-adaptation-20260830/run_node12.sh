#!/bin/bash
# FA01 Node 12 run script — mathlib 双受控失败复跑（criteria 冻结版）
rm -rf /tmp/fa_node12
mkdir -p /tmp/fa_node12
cd /tmp/fa_node12
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_TASK_TIMEOUT_SECS=600
hearth chat '创建一个 Rust 库项目 mathlib/，按以下顺序严格执行：
1) 用 bash 执行 mkdir -p mathlib/src
2) 用 write_file 创建 mathlib/Cargo.toml，内容为 [package] name=mathlib version=0.1.0 edition=2021
3) 用 write_file 创建 mathlib/src/lib.rs，包含：
   pub fn add(a: i64, b: i64) -> i64 { a + b + 1 }（故意留 bug）
   pub fn shout(s: &str) -> String { s.to_string() }（故意留 bug：未大写、无感叹号）
   以及测试模块：
   #[cfg(test)]
   mod tests {
       #[test]
       fn t_add() { assert_eq!(add(2, 3), 5); }
       #[test]
       fn t_shout() { assert_eq!(shout("hi"), "HI!"); }
   }
4) 用 bash 执行 cargo test --manifest-path mathlib/Cargo.toml —— 两个测试都会失败（受控失败），记录失败输出
5) 用 write_file 修复 mathlib/src/lib.rs：add 改为 a + b；shout 改为 format!("{}!", s.to_uppercase())。测试保持不变，确保整份文件语法完整可编译
6) 用 bash 再执行 cargo test --manifest-path mathlib/Cargo.toml 确认两个测试通过
7) 用 write_file 创建 RESULT.txt，内容写 LONGRUN_RECOVERY_PASSED。以上步骤完成后立即宣告任务完成（DONE），不要做额外验证' \
  --budget 50 \
  --acceptance 'cmd: cargo test --manifest-path mathlib/Cargo.toml' \
  --acceptance 'file: mathlib/src/lib.rs contains to_uppercase' \
  --acceptance 'file: RESULT.txt contains LONGRUN_RECOVERY_PASSED'
echo "EXIT_CODE=$?"
