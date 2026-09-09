pub fn sum_range(n: u64) -> u64 {
    (1..=n).sum()
}
pub fn sum_formula(n: u64) -> u64 {
    n * (n + 1) / 2
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_both() { assert_eq!(sum_range(100), sum_formula(100)); }
}