#!/bin/bash
# P2-LR Node 12: Controlled failure task ×2 independent runs (sortlib)
# $1 = run number (1|2)
cd /tmp/lr_n12_r$1
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_TASK_TIMEOUT_SECS=480
hearth chat '创建一个 Rust 库项目 sortlib/，按以下顺序严格执行：
1) 用 bash 执行 mkdir -p sortlib/src
2) 用 write_file 创建 sortlib/Cargo.toml，内容为 [package] name=sortlib version=0.1.0 edition=2021
3) 用 write_file 创建 sortlib/src/lib.rs，实现冒泡排序（故意留 bug：内层循环边界写成 j < n - 1 - 0，应为 j < n - 1 - i）：
   pub fn bubble_sort(arr: &mut [i64]) {
       let n = arr.len();
       for i in 0..n {
           for j in 0..n - 1 - 0 {
               if arr[j] > arr[j + 1] { arr.swap(j, j + 1); }
           }
       }
   }
   以及测试模块：
   #[cfg(test)] mod tests { #[test] fn t_sort() { let mut a = vec![5, 2, 8, 1, 9, 3]; bubble_sort(&mut a); assert_eq!(a, vec![1, 2, 3, 5, 8, 9]); } }
4) 用 bash 执行 cargo test --manifest-path sortlib/Cargo.toml —— 测试会失败，诊断失败原因（边界错误）
5) 用 write_file 修复：内层循环边界改为 for j in 0..n - 1 - i
6) 用 bash 执行 cargo test --manifest-path sortlib/Cargo.toml 确认通过
每步完成后立即进入下一步。' \
  --budget 60 \
  --acceptance 'cmd: cargo test --manifest-path sortlib/Cargo.toml' \
  --acceptance 'file: sortlib/src/lib.rs contains n - 1 - i'
echo "N12_R${1}_EXIT=$?"
