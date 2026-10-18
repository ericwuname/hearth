#!/bin/bash
# T06-fix-null-check 参考解：空串须返回 default（原实现直接返回 Option，类型都不对）。
set -euo pipefail
grep -q 'BUG: should be s.chars().next().unwrap_or(default)' src/lib.rs \
  || { echo "solution(T06): 未找到注释标记（fixture 形状已变）" >&2; exit 1; }

cat > src/lib.rs <<'RUST'
pub fn first_char_or_default(s: &str, default: char) -> char {
    s.chars().next().unwrap_or(default)
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_has() { assert_eq!(first_char_or_default("abc", 'x'), 'a'); }
  #[test] fn t_empty_bug() { assert_eq!(first_char_or_default("", 'x'), 'x'); }
}
RUST

grep -q 'unwrap_or(default)' src/lib.rs || { echo "solution(T06): 未写入 unwrap_or(default)" >&2; exit 1; }
