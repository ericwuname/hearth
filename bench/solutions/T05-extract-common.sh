#!/bin/bash
# T05-extract-common 参考解：抽出 validate_length(s, max)，两个校验函数改为复用它。
set -euo pipefail
grep -q 'pub fn validate_email' src/lib.rs || { echo "solution(T05): 未找到 validate_email（fixture 形状已变）" >&2; exit 1; }

cat > src/lib.rs <<'RUST'
pub fn validate_length(s: &str, max: usize) -> Result<(), String> {
    if s.len() > max { return Err("too long".into()); }
    Ok(())
}
pub fn validate_email(email: &str) -> Result<(), String> {
    if !email.contains("@") { return Err("missing @".into()); }
    validate_length(email, 254)?;
    Ok(())
}
pub fn validate_phone(phone: &str) -> Result<(), String> {
    if phone.is_empty() { return Err("empty".into()); }
    validate_length(phone, 20)?;
    Ok(())
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_email_ok() { assert!(validate_email("a@b.com").is_ok()); }
  #[test] fn t_email_no_at() { assert!(validate_email("ab.com").is_err()); }
  #[test] fn t_phone_ok() { assert!(validate_phone("13800138000").is_ok()); }
  #[test] fn t_phone_empty() { assert!(validate_phone("").is_err()); }
}
RUST

grep -q 'fn validate_length' src/lib.rs || { echo "solution(T05): 未写入 validate_length" >&2; exit 1; }
