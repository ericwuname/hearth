// v8.0: Multi-user support — maps API keys to user identities.
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
        self.keys.read().unwrap().get(api_key).cloned()
    }

    /// Register a new API key → user mapping.
    pub fn register(&self, api_key: String, user_id: String) {
        self.keys.write().unwrap().insert(api_key, user_id);
    }
}

/// Context injected into each request after auth.
#[derive(Clone)]
pub struct UserContext {
    pub user_id: String,
    pub memory_dir: std::path::PathBuf,
}
