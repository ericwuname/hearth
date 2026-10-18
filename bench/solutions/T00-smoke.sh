#!/bin/bash
# T00-smoke 参考解：新增 multiply(a,b) 与 test_multiply。
# 约定：在工作区根目录执行；形状不符 ⇒ 立刻失败（不得静默产出错误"解"）。
set -euo pipefail
grep -q 'pub fn subtract' src/lib.rs || { echo "solution(T00): 未找到 subtract（fixture 形状已变）" >&2; exit 1; }

cat > src/lib.rs <<'RUST'
/// A simple calculator to test basic agent editing.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn subtract(a: i32, b: i32) -> i32 {
    a - b
}

pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
    }

    #[test]
    fn test_subtract() {
        assert_eq!(subtract(5, 3), 2);
    }

    #[test]
    fn test_multiply() {
        assert_eq!(multiply(2, 3), 6);
    }
}
RUST

grep -q 'fn multiply' src/lib.rs || { echo "solution(T00): 写入后未见 fn multiply" >&2; exit 1; }
grep -q 'fn test_multiply' src/lib.rs || { echo "solution(T00): 写入后未见 test_multiply" >&2; exit 1; }
