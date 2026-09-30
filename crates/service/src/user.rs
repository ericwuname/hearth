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
}

/// Context injected into each request after auth.
#[derive(Clone)]
pub struct UserContext {
    pub user_id: String,
    pub memory_dir: std::path::PathBuf,
}
