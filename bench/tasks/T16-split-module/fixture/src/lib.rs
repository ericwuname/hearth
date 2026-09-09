pub mod math {
    pub fn add(a: i32, b: i32) -> i32 { a + b }
    pub fn mul(a: i32, b: i32) -> i32 { a * b }
}
pub mod string_utils {
    pub fn reverse(s: &str) -> String { s.chars().rev().collect() }
}
pub fn run() -> String {
    format!("{}+{}={}, rev({})={}", 2, 3, math::add(2,3), "abc", string_utils::reverse("abc"))
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_run() { assert_eq!(run(), "2+3=5, rev(abc)=cba"); }
}