// v8.0: Per-user store multiplexer.
// Each user gets their own subdirectory under the base memory root.
use memory::{CivilizationStore, WorkLineStore};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Lazily creates per-user civilization and work-line stores.
pub struct PerUserStore {
    base_dir: PathBuf,
    civ: RwLock<HashMap<String, Arc<CivilizationStore>>>,
    work: RwLock<HashMap<String, Arc<WorkLineStore>>>,
}

impl PerUserStore {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            base_dir: base_dir.to_path_buf(),
            civ: RwLock::new(HashMap::new()),
            work: RwLock::new(HashMap::new()),
        }
    }

    /// Get or create a civilization store for the given user.
    pub fn civ_for(&self, user_id: &str) -> Arc<CivilizationStore> {
        let mut map = self.civ.write().unwrap();
        map.entry(user_id.to_string())
            .or_insert_with(|| {
                let dir = self.base_dir.join(user_id);
                Arc::new(CivilizationStore::new(&dir, "civ.jsonl").expect("per-user civ store"))
            })
            .clone()
    }

    /// Get or create a work-line store for the given user.
    pub fn workline_for(&self, user_id: &str) -> Arc<WorkLineStore> {
        let mut map = self.work.write().unwrap();
        map.entry(user_id.to_string())
            .or_insert_with(|| {
                let dir = self.base_dir.join(user_id);
                Arc::new(
                    WorkLineStore::new(&dir, "workline.jsonl").expect("per-user workline store"),
                )
            })
            .clone()
    }
}
