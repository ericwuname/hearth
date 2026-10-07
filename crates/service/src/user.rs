// v8.0: Multi-user support — maps API keys to user identities.
// P0-05（2026-10-01, traecode）：锁访问改走 `recover`——`find` 在**每个请求**的
// 鉴权路径上，旧 `.unwrap()` 一旦因中毒而炸即等于全站鉴权不可用。
use crate::lock::recover;
use std::collections::HashMap;
use std::sync::RwLock;

/// Maps API key → user identity.
pub struct UserStore {
    keys: RwLock<HashMap<String, String>>, // api_key → user_id
}

impl UserStore {
    /// Create with optional list of "api_key=user_id" pairs.
    pub fn new(pairs: &[(String, String)]) -> Self {
        let mut keys = HashMap::new();
        for (k, v) in pairs {
            keys.insert(k.clone(), v.clone());
        }
        Self {
            keys: RwLock::new(keys),
        }
    }

    /// Look up user_id from API key. Returns None if key not recognized.
    pub fn find(&self, api_key: &str) -> Option<String> {
        recover(self.keys.read()).get(api_key).cloned()
    }

    /// Register a new API key → user mapping.
    pub fn register(&self, api_key: String, user_id: String) {
        recover(self.keys.write()).insert(api_key, user_id);
    }

    /// D-169（2026-10-05, traecode）：是否**已注册任何 key**。
    ///
    /// 供鉴权中间件判定"本次部署是否要求凭据"——既有单 `API_KEY` 亦经
    /// `register(key, "default")` 入册（见 `main.rs`），故两种配置方式同一判定。
    /// 这也修掉了 `require_api_key` 原先只看 `state.api_key` 的盲区：只配 `HEARTH_USERS`
    /// （无 `API_KEY`）时，旧代码会把"已配 key"误判成"未配 key"。
    pub fn has_keys(&self) -> bool {
        !recover(self.keys.read()).is_empty()
    }

    /// D-169：`provided` 是否命中**任一**已注册 key。
    ///
    /// 逐 key **常数时间**比对（复用 `ct_eq`，与单 key 路径同一原语），且**遍历全部 key
    /// 不早退**——否则"是否命中 / 命中第几个"会从耗时泄漏（2026-10-05 联网核实的通行口径）。
    pub fn authenticates(&self, provided: &str) -> bool {
        let map = recover(self.keys.read());
        let mut hit = 0u8;
        for k in map.keys() {
            hit |= u8::from(ct_eq(provided, k));
        }
        hit != 0
    }
}

/// 常数时间字符串等值比较：长度不等 → 不匹配；长度相等 → 逐字节 XOR 折叠（**不早退**）。
///
/// 与 `routes::require_api_key` 既有的单 key 比对**同一实现**（此处抽函数以复用）。
/// 注：长度不等仍走快路径（不比较内容）——长度不属本威胁模型的秘密面。
fn ct_eq(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

// D-111②（2026-10-02, traecode）：原 `UserContext { user_id, memory_dir }` 结构体**已删**。
// 它自称 "Context injected into each request after auth"，但全仓**无构造方、无读取方**——
// 真实的按请求身份解析是 `routes::get_user_id(headers, user_store) -> String` +
// `PerUserStore::civ_for(uid)`（per-user store 复用同一 uid 字符串），并不需要这个类型。
// ⇒ 属"死类型 + 不实文档"（同 D-76/D-88/D-98 口径）；日后若真要做请求级上下文，
// 应随使用点一起设计，而不是先摆一个空壳。
