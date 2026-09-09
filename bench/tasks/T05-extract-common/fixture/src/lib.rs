pub fn validate_email(email: &str) -> Result<(), String> {
    if !email.contains("@") { return Err("missing @".into()); }
    if email.len() > 254 { return Err("too long".into()); }
    Ok(())
}
pub fn validate_phone(phone: &str) -> Result<(), String> {
    if phone.is_empty() { return Err("empty".into()); }
    if phone.len() > 20 { return Err("too long".into()); }
    Ok(())
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_email_ok() { assert!(validate_email("a@b.com").is_ok()); }
  #[test] fn t_email_no_at() { assert!(validate_email("ab.com").is_err()); }
  #[test] fn t_phone_ok() { assert!(validate_phone("13800138000").is_ok()); }
  #[test] fn t_phone_empty() { assert!(validate_phone("").is_err()); }
}