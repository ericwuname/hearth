pub const DEFAULT_TIMEOUT_SECS: u64 = 30;
pub fn fetch_data() -> String { format!("timeout={}s", DEFAULT_TIMEOUT_SECS) }
#[cfg(test)] mod tests { use super::*;
  #[test] fn t() { assert_eq!(fetch_data(), "timeout=30s"); }
}
