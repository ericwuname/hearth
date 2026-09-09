pub fn divide(a: f64, b: f64) -> Result<f64, String> {
    if b == 0.0 { return Err("division by zero".into()); }
    Ok(a / b)
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_ok() { assert!((divide(6.0, 3.0).unwrap() - 2.0).abs() < 0.001); }
  #[test] fn t_zero() { assert!(divide(1.0, 0.0).is_err()); }
}