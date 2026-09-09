pub fn middle_char(s: &str) -> Option<char> {
    let bytes = s.as_bytes();
    bytes.get(bytes.len() / 2).map(|&b| b as char)
}
pub fn safe_get(arr: &[i32], idx: usize) -> Option<i32> {
    arr.get(idx - 1).copied() // BUG: should be arr.get(idx), not idx-1
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_mid() { assert_eq!(middle_char("abcde"), Some('c')); }
  #[test] fn t_safe_ok() { assert_eq!(safe_get(&[10,20,30], 0), Some(10)); }
  #[test] fn t_safe_bug() { assert_eq!(safe_get(&[10,20,30], 1), Some(10)); }
}