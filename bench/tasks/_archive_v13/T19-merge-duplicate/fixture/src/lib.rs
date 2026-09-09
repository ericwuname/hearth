pub fn parse_age_u8(s: &str) -> Result<u8, String> {
    let n: u32 = s.parse().map_err(|e| format!("parse error: {e}"))?;
    if n > 120 { return Err("too large".into()); }
    if n == 0 { return Err("zero not allowed".into()); }
    Ok(n as u8)
}
pub fn parse_count_u32(s: &str) -> Result<u32, String> {
    let n: u64 = s.parse().map_err(|e| format!("parse error: {e}"))?;
    if n > 1_000_000 { return Err("too large".into()); }
    if n == 0 { return Err("zero not allowed".into()); }
    Ok(n as u32)
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_age_ok() { assert_eq!(parse_age_u8("25").unwrap(), 25); }
  #[test] fn t_age_zero() { assert!(parse_age_u8("0").is_err()); }
  #[test] fn t_cnt_ok() { assert_eq!(parse_count_u32("500").unwrap(), 500); }
  #[test] fn t_cnt_zero() { assert!(parse_count_u32("0").is_err()); }
}