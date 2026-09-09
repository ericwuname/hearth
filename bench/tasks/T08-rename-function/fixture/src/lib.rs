pub fn calc_sum(nums: &[i32]) -> i32 { nums.iter().sum() }
pub fn calc_avg(nums: &[i32]) -> f64 {
    if nums.is_empty() { return 0.0; }
    calc_sum(nums) as f64 / nums.len() as f64
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_sum() { assert_eq!(calc_sum(&[1,2,3]), 6); }
  #[test] fn t_avg() { assert!((calc_avg(&[1,2,3]) - 2.0).abs() < 0.01); }
}