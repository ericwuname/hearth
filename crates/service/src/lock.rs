//! P0-05（2026-10-01, traecode）：锁中毒恢复。
//!
//! `std::sync::{Mutex, RwLock}` 在**某个持锁线程 panic** 后会把锁标记为"中毒"，
//! 此后每次 `lock()` 都返回 `Err(PoisonError)`。默认写法 `.unwrap()` 于是把
//! **一次** panic 放大成**此后每一次**访问都 panic——落在请求路径上，就是
//! "该接口全站永久 500"（D-24/D-25 的次生缺陷正是这条）。
//!
//! 本 crate 中受本模块守卫的数据全部是 `HashMap` / `Vec` 这类**没有跨字段不变式**
//! 的容器：最坏情况只是少一条缓存条目，下一次请求重建即可。因此"中毒"标记在这里
//! 没有保留价值——取回内部数据继续服务才是正确语义（而非静默吞掉错误：真正的
//! 失败仍由调用方按 `Result` 上抛，见 `per_user.rs`）。

use std::sync::LockResult;

/// 忽略中毒标记，取回内部数据。见模块注释。
pub fn recover<T>(r: LockResult<T>) -> T {
    r.unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// 先红后绿：修复前此处若用 `.lock().unwrap()`，第二行断言会 panic。
    #[test]
    fn test_p0_05_recover_survives_poisoned_lock() {
        let m = Mutex::new(vec![1_u32, 2, 3]);
        // 制造中毒：持锁线程 panic。
        let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _g = m.lock().unwrap();
            panic!("simulated holder panic");
        }));
        assert!(poisoned.is_err(), "前置条件：锁必须已被标记中毒");
        assert!(
            m.lock().is_err(),
            "前置条件：中毒锁的裸 lock() 必须返回 Err（这正是旧 .unwrap() 会炸的地方）"
        );
        // 修复后：recover 取回数据，且原数据完好（容器无跨字段不变式）。
        let guard = recover(m.lock());
        assert_eq!(*guard, vec![1, 2, 3], "中毒不应丢失内部数据");
    }
}
