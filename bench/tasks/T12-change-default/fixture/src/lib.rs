pub const MAX_ITEMS: usize = 100;
pub const PAGE_SIZE: usize = 10;
pub fn paginate(total: usize) -> usize {
    if total == 0 { return 0; }
    (total + PAGE_SIZE - 1) / PAGE_SIZE
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_pages() { assert_eq!(paginate(25), 3); }
  #[test] fn t_zero() { assert_eq!(paginate(0), 0); }
}