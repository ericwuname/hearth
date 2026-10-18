#!/bin/bash
# T17-fix-race 参考解：`MutexGuard += 1` 不成立（类型都不对）→ 先解引用再自增。
set -euo pipefail
grep -q 'MutexGuard returned, not incremented' src/lib.rs \
  || { echo "solution(T17): 未找到 bug 注释（fixture 形状已变）" >&2; exit 1; }

cat > src/lib.rs <<'RUST'
use std::sync::Mutex;
pub struct Counter { value: Mutex<i32> }
impl Counter {
    pub fn new() -> Self { Counter { value: Mutex::new(0) } }
    pub fn inc(&self) { *self.value.lock().unwrap() += 1; }
    pub fn get(&self) -> i32 { *self.value.lock().unwrap() }
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_inc() { let c = Counter::new(); c.inc(); assert_eq!(c.get(), 1); }
}
RUST

grep -q '\*self.value.lock().unwrap() += 1' src/lib.rs || { echo "solution(T17): inc 未修" >&2; exit 1; }
