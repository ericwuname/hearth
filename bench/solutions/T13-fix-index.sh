#!/bin/bash
# T13-fix-index 参考解：
#   ① safe_get 用 idx-1（idx=0 时还会下溢 panic）→ 用 idx；
#   ② middle_char 按**字节**取中间 → 按**字符**取中间（非 ASCII 才正确）；
#   ③ **连带修正 `t_safe_bug`**：它断言的是 bug 行为（idx=1 得 Some(10)），
#      修对函数后必然红 —— 本任务"cargo test 全绿"蕴含"测试里的错误期望也要改"。
#      （这是正向门禁抓出来的：只改函数 ⇒ 该测试红 ⇒ 判据判失败。）
set -euo pipefail
grep -q 'arr.get(idx - 1)' src/lib.rs || { echo "solution(T13): 未找到 idx - 1（fixture 形状已变）" >&2; exit 1; }
grep -q 'Some(10)); }' src/lib.rs || { echo "solution(T13): 未找到编码 bug 的测试断言（fixture 形状已变）" >&2; exit 1; }

cat > src/lib.rs <<'RUST'
pub fn middle_char(s: &str) -> Option<char> {
    let chars: Vec<char> = s.chars().collect();
    chars.get(chars.len() / 2).copied()
}
pub fn safe_get(arr: &[i32], idx: usize) -> Option<i32> {
    arr.get(idx).copied()
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_mid() { assert_eq!(middle_char("abcde"), Some('c')); }
  #[test] fn t_safe_ok() { assert_eq!(safe_get(&[10,20,30], 0), Some(10)); }
  // 原断言 `Some(10)` 编码了 idx-1 的 bug；改对实现后必须同步为 Some(20)。
  #[test] fn t_safe_bug() { assert_eq!(safe_get(&[10,20,30], 1), Some(20)); }
}
RUST

grep -q 'arr.get(idx).copied()' src/lib.rs || { echo "solution(T13): safe_get 未修" >&2; exit 1; }
grep -q 'chars.get(chars.len() / 2)' src/lib.rs || { echo "solution(T13): middle_char 未改为按字符" >&2; exit 1; }
grep -q 'safe_get(&\[10,20,30\], 1), Some(20)' src/lib.rs || { echo "solution(T13): 编码 bug 的测试未同步" >&2; exit 1; }
