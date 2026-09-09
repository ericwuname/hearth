use std::sync::Mutex;
pub struct Counter { value: Mutex<i32> }
impl Counter {
    pub fn new() -> Self { Counter { value: Mutex::new(0) } }
    pub fn inc(&self) { self.value.lock().unwrap() += 1; } // BUG: MutexGuard returned, not incremented
    pub fn get(&self) -> i32 { *self.value.lock().unwrap() }
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_inc() { let c = Counter::new(); c.inc(); assert_eq!(c.get(), 1); }
}