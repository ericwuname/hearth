pub fn first_char_or_default(s: &str, default: char) -> char {
    s.chars().next() // BUG: should be s.chars().next().unwrap_or(default)
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_has() { assert_eq!(first_char_or_default("abc", 'x'), 'a'); }
  #[test] fn t_empty_bug() { assert_eq!(first_char_or_default("", 'x'), 'x'); }
}