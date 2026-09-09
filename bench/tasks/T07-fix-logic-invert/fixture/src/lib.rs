pub fn is_valid_age(age: u8) -> bool {
    age > 0 && age <= 120 // BUG: should be age >= 0
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_normal() { assert!(is_valid_age(25)); }
  #[test] fn t_zero_bug() { assert!(is_valid_age(0)); }
  #[test] fn t_too_big() { assert!(!is_valid_age(200)); }
}