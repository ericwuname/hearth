pub fn fibonacci(n: u32) -> u64 {
    if n == 0 { return 0; }
    if n == 1 { return 1; }
    let mut a = 0u64; let mut b = 1u64;
    for _ in 1..n { let t = a + b; a = b; b = t; }
    a
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_fib7() { assert_eq!(fibonacci(7), 13); }
  #[test] fn t_fib_bug() { assert_eq!(fibonacci(10), 55); }
}