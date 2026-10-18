#!/bin/bash
# T09-add-error-type 参考解：String 错误 → 自定义枚举 MathError { DivisionByZero, NegativeInput } + Display。
set -euo pipefail
grep -q 'pub fn divide' src/lib.rs || { echo "solution(T09): 未找到 divide（fixture 形状已变）" >&2; exit 1; }

cat > src/lib.rs <<'RUST'
#[derive(Debug, PartialEq)]
pub enum MathError {
    DivisionByZero,
    NegativeInput,
}
impl std::fmt::Display for MathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MathError::DivisionByZero => write!(f, "division by zero"),
            MathError::NegativeInput => write!(f, "negative input"),
        }
    }
}
impl std::error::Error for MathError {}
pub fn divide(a: f64, b: f64) -> Result<f64, MathError> {
    if b == 0.0 { return Err(MathError::DivisionByZero); }
    Ok(a / b)
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_ok() { assert!((divide(6.0, 3.0).unwrap() - 2.0).abs() < 0.001); }
  #[test] fn t_zero() { assert!(divide(1.0, 0.0).is_err()); }
}
RUST

grep -q 'enum MathError' src/lib.rs || { echo "solution(T09): 未写入 MathError" >&2; exit 1; }
