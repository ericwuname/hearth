#!/bin/bash
# T16-split-module 参考解：math → src/math.rs、string_utils → src/string_utils.rs；
# lib.rs 用 `pub mod …;` 引入（保持原有公开路径），run() 与测试不变。
set -euo pipefail
grep -q 'pub mod math' src/lib.rs || { echo "solution(T16): 未找到 pub mod math（fixture 形状已变）" >&2; exit 1; }

cat > src/math.rs <<'RUST'
pub fn add(a: i32, b: i32) -> i32 { a + b }
pub fn mul(a: i32, b: i32) -> i32 { a * b }
RUST

cat > src/string_utils.rs <<'RUST'
pub fn reverse(s: &str) -> String { s.chars().rev().collect() }
RUST

cat > src/lib.rs <<'RUST'
pub mod math;
pub mod string_utils;
pub fn run() -> String {
    format!("{}+{}={}, rev({})={}", 2, 3, math::add(2,3), "abc", string_utils::reverse("abc"))
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_run() { assert_eq!(run(), "2+3=5, rev(abc)=cba"); }
}
RUST

test -f src/math.rs || { echo "solution(T16): 未生成 src/math.rs" >&2; exit 1; }
test -f src/string_utils.rs || { echo "solution(T16): 未生成 src/string_utils.rs" >&2; exit 1; }
