// v8.0: Per-user store multiplexer.
// Each user gets their own subdirectory under the base memory root.
use crate::lock::recover;
use experience::ExperienceStore;
use memory::{CivilizationStore, WorkLineStore};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Lazily creates per-user civilization and work-line stores.
pub struct PerUserStore {
    base_dir: PathBuf,
    civ: RwLock<HashMap<String, Arc<CivilizationStore>>>,
    work: RwLock<HashMap<String, Arc<WorkLineStore>>>,
    /// D-175（2026-10-08, traecode）：**per-user 经验库**。修复前经验库是**进程级单例**
    /// （`main.rs` 一个 `ExperienceStore`），而 agent-loop 每次 run 收尾都会把
    /// `problem = <会话目标文本>` 追加进去 ⇒ 多租户下**所有租户的目标文本落进同一文件**
    /// （派生数据未按租户分区；目录数据 + 若开复用即跨租户注入）。按业界口径（派生学习制品
    /// 必须按租户命名空间隔离）与 D-108/D-109 对 civ 线的同一处置，改为按 uid 分档。
    exp: RwLock<HashMap<String, Arc<ExperienceStore>>>,
}

impl PerUserStore {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            base_dir: base_dir.to_path_buf(),
            civ: RwLock::new(HashMap::new()),
            work: RwLock::new(HashMap::new()),
            exp: RwLock::new(HashMap::new()),
        }
    }

    /// D-175：取/建该租户的**经验库**（`<base>/<uid>/experience.jsonl`，延迟加载）。
    ///
    /// 同步、不读盘（`set_path_deferred`）——与 civ/workline 的懒建同款；首次
    /// `append`/`recent_failures`/`metrics`/`prune` 时才真正读盘（D-119 懒加载）。
    pub fn experience_for(&self, user_id: &str) -> Arc<ExperienceStore> {
        let mut map = recover(self.exp.write());
        if let Some(store) = map.get(user_id) {
            return store.clone();
        }
        let dir = self.base_dir.join(user_id);
        let store = Arc::new(ExperienceStore::new());
        store.set_path_deferred(dir.join("experience.jsonl"));
        map.insert(user_id.to_string(), store.clone());
        store
    }

    /// D-175：**已创建**的所有 per-user 经验库快照（供后台维护循环逐档剪枝）。
    ///
    /// 盲区：只含"本进程已被请求触达过"的租户——从未被触达者其档尚未创建（懒建固有）；
    /// 首次被触达时由 `prune` 的 `ensure_loaded` 读到旧盘内容，故剪枝不会漏掉已存在的数据
    /// （只是推迟到该租户下次被访问）。与 civ/workline 的懒建语义一致。
    pub fn all_experience(&self) -> Vec<Arc<ExperienceStore>> {
        recover(self.exp.read()).values().cloned().collect()
    }

    /// Get or create a civilization store for the given user.
    ///
    /// P0-05（2026-10-01, traecode）：**请求路径上的 panic 双杀**。
    ///
    /// 病灶（原实现）：
    /// ```ignore
    /// let mut map = self.civ.write().unwrap();
    /// map.entry(user_id.to_string())
    ///     .or_insert_with(|| Arc::new(CivilizationStore::new(&dir, "civ.jsonl")
    ///         .expect("per-user civ store")))   // ← ①在请求路径上 panic
    ///     .clone()
    /// ```
    /// 触发条件**可达**：`create_dir_all` / 打开 jsonl 失败（磁盘满、目录只读、
    /// `base_dir/user_id` 被同名文件占用……）。后果**两段式**：
    /// - ① 当前请求 panic（连接被撕掉）；
    /// - ② panic **发生时仍持有 `self.civ` 写锁** → 锁被标记中毒 →
    ///   其后**每一次** `civ_for()` 的 `.write().unwrap()` 二次 panic →
    ///   `/api/v1/civilization` **全站永久不可用**（一次 I/O 错误 ≈ 永久 DoS）。
    ///
    /// 修复：失败按 `Result` 上抛（由 handler 转成 500，只影响**这一次**请求），
    /// 锁访问走 `recover`（中毒不升级）。**不静默降级**——错误如实上报。
    pub fn civ_for(&self, user_id: &str) -> anyhow::Result<Arc<CivilizationStore>> {
        let mut map = recover(self.civ.write());
        if let Some(store) = map.get(user_id) {
            return Ok(store.clone());
        }
        let dir = self.base_dir.join(user_id);
        let store = Arc::new(CivilizationStore::new(&dir, "civ.jsonl")?);
        map.insert(user_id.to_string(), store.clone());
        Ok(store)
    }

    /// Get or create a work-line store for the given user.
    ///
    /// P0-05：与 [`PerUserStore::civ_for`] 同源同修（同一 `.expect()` 模式，
    /// 同一把写锁中毒链）。
    pub fn workline_for(&self, user_id: &str) -> anyhow::Result<Arc<WorkLineStore>> {
        let mut map = recover(self.work.write());
        if let Some(store) = map.get(user_id) {
            return Ok(store.clone());
        }
        let dir = self.base_dir.join(user_id);
        let store = Arc::new(WorkLineStore::new(&dir, "workline.jsonl")?);
        map.insert(user_id.to_string(), store.clone());
        Ok(store)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 先红后绿：修复前 `civ_for` 在此处 **panic**（`.expect("per-user civ store")`）。
    ///
    /// 构造：把 `base_dir` 指向一个**已存在的普通文件**，则 `base_dir/user_id`
    /// 的 `create_dir_all` 必然失败（`Not a directory`）——跨平台、可控。
    #[test]
    fn test_p0_05_civ_for_failure_is_error_not_panic() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("occupied");
        std::fs::write(&file_path, b"not a directory").unwrap();

        let store = PerUserStore::new(&file_path);
        let r = store.civ_for("alice");
        assert!(
            r.is_err(),
            "存储创建失败必须返回 Err（修复前此处 panic，并把 civ 锁毒成全站 DoS）"
        );

        // 二次调用：证明锁未被中毒、接口仍可用（修复前这里会 panic）。
        assert!(
            store.civ_for("alice").is_err(),
            "首次失败后接口必须仍可服务（锁不得被中毒）"
        );
        assert!(
            store.workline_for("alice").is_err(),
            "workline 同源同修：失败同样是 Err 而非 panic"
        );
    }

    /// 正常路径回归：目录可建时行为不变（缓存命中返回同一实例）。
    #[test]
    fn test_p0_05_per_user_store_happy_path_caches() {
        let tmp = tempfile::tempdir().unwrap();
        let store = PerUserStore::new(tmp.path());
        let a = store.civ_for("bob").expect("可写目录应成功");
        let b = store.civ_for("bob").expect("缓存命中应成功");
        assert!(Arc::ptr_eq(&a, &b), "同一 user 必须复用同一 store 实例");

        let w = store.workline_for("bob").expect("可写目录应成功");
        assert!(Arc::ptr_eq(&w, &store.workline_for("bob").unwrap()));
    }

    /// D-175 回归锁（**先红后绿**）：经验库必须**按 uid 分档**——修复前是进程级单例，
    /// 任一租户 run 收尾追加的 `<会话目标文本>` 会与所有租户共用一个文件（跨租户泄露）。
    #[tokio::test]
    async fn test_d175_experience_is_per_user() {
        let tmp = tempfile::tempdir().unwrap();
        let store = PerUserStore::new(tmp.path());

        // 同一 uid 复用同一实例；不同 uid 是不同实例。
        let a1 = store.experience_for("alice");
        let a2 = store.experience_for("alice");
        assert!(Arc::ptr_eq(&a1, &a2), "同一 uid 必须复用同一 store");
        let b = store.experience_for("bob");
        assert!(
            !Arc::ptr_eq(&a1, &b),
            "不同 uid 必须是不同 store（修复前共用单例）"
        );

        // 写侧隔离：alice 追加后 bob 的档不受影响。
        a1.append(experience::Experience {
            id: "e-a".into(),
            category: "failure".into(),
            problem: "alice-only-goal-d175".into(),
            solution: "s".into(),
            success: false,
            effectiveness: 0.6,
            created_at: chrono::Utc::now().to_rfc3339(),
        })
        .await
        .unwrap();
        assert_eq!(a1.metrics().await.total_experiences, 1);
        assert_eq!(
            b.metrics().await.total_experiences,
            0,
            "**跨租户泄露**：alice 的条目不得出现在 bob 的档里"
        );

        // 维护遍历能看到已创建的档（含 alice、bob）。
        assert_eq!(store.all_experience().len(), 2);

        // 落盘路径按 uid 分区：两档文件各自独立存在。
        assert!(tmp.path().join("alice").join("experience.jsonl").exists());
    }
}
