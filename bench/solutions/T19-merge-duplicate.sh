#!/bin/bash
# T19-merge-duplicate 参考解：泛型 pub fn parse_positive<T>(s) -> Result<T, String>
# （where T: FromStr + PartialOrd + Copy + Clone —— 与 goal.txt 硬性要求的签名一致）；
# 零值判定只用 `!(n > zero)` 完成，故**不需要**额外 trait 约束；两个旧函数变薄封装、公开签名不变。
set -euo pipefail
grep -q 'pub fn parse_age_u8' src/lib.rs || { echo "solution(T19): 未找到 parse_age_u8（fixture 形状已变）" >&2; exit 1; }

cat > src/lib.rs <<'RUST'
pub fn parse_positive<T>(s: &str) -> Result<T, String>
where
    T: std::str::FromStr + PartialOrd + Copy + Clone,
{
    let n: T = s.parse().map_err(|_| "parse error".to_string())?;
    let zero: T = "0".parse().map_err(|_| "parse error".to_string())?;
    if !(n > zero) {
        return Err("zero not allowed".into());
    }
    Ok(n)
}
pub fn parse_age_u8(s: &str) -> Result<u8, String> {
    let n: u8 = parse_positive(s)?;
    if n > 120 { return Err("too large".into()); }
    Ok(n)
}
pub fn parse_count_u32(s: &str) -> Result<u32, String> {
    let n: u32 = parse_positive(s)?;
    if n > 1_000_000 { return Err("too large".into()); }
    Ok(n)
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_age_ok() { assert_eq!(parse_age_u8("25").unwrap(), 25); }
  #[test] fn t_age_zero() { assert!(parse_age_u8("0").is_err()); }
  #[test] fn t_cnt_ok() { assert_eq!(parse_count_u32("500").unwrap(), 500); }
  #[test] fn t_cnt_zero() { assert!(parse_count_u32("0").is_err()); }
}
RUST

grep -q 'pub fn parse_positive<T>' src/lib.rs || { echo "solution(T19): 未写入 parse_positive<T>" >&2; exit 1; }
