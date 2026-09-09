pub fn sum_positive(nums: &[i32]) -> i32 {
    nums.iter().filter(|&&x| x > 0).sum()
}

pub fn factorial(n: u32) -> u64 {
    (1..=n as u64).product()
}

pub fn is_palindrome(s: &str) -> bool {
    s.chars().eq(s.chars().rev())
}

pub fn max_element(nums: &[i32]) -> Option<i32> {
    nums.iter().max().copied()
}
