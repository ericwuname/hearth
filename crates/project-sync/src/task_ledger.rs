//! Task Ledger（§8.2/§8.5/§8.6/§8.7）——每任务一文件 + 乐观锁 + 原子写回 + 状态机 +
//! 依赖自动流转 + 继承认领（co_owners 优先）+ 孤儿告警 + rrule 实例。

use crate::registry::Registry;
use crate::task::{Task, TaskStatus};
use crate::Result;
use std::path::{Path, PathBuf};

/// Task Ledger：tasks/ 每任务一文件。
pub struct TaskLedger {
    root: PathBuf,
}

impl TaskLedger {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    fn path(&self, id: &str) -> PathBuf {
        self.root.join(format!("{id}.toml"))
    }

    /// 写任务（乐观锁 + 原子 rename，H-1）。乐观锁：磁盘当前版本必须 == expect_version
    /// （0 = 新建）；不匹配 → Err（并发修改）。task.version 为写入后的新版本。
    pub fn write(&self, task: &Task, expect_version: u64) -> Result<()> {
        if expect_version != 0 {
            let disk_version = self.read(&task.id)?.map(|t| t.version).unwrap_or(0);
            if disk_version != expect_version {
                return Err(format!(
                    "乐观锁冲突：期望 v{expect_version} 磁盘 v{disk_version}（并发修改）"
                )
                .into());
            }
        }
        let path = self.path(&task.id);
        let tmp = self.root.join(format!("{}.tmp", task.id));
        let s = toml::to_string(task).map_err(|e| crate::PwcError::Other(e.to_string()))?;
        std::fs::write(&tmp, s)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn read(&self, id: &str) -> Result<Option<Task>> {
        let path = self.path(id);
        match std::fs::read_to_string(&path) {
            Ok(s) => toml::from_str(&s)
                .map(Some)
                .map_err(|e| crate::PwcError::Other(e.to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 列出全部任务。
    pub fn list(&self) -> Result<Vec<Task>> {
        let mut tasks = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&self.root) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name.ends_with(".toml") && !name.ends_with(".tmp") {
                    let id = name.trim_end_matches(".toml").to_string();
                    if let Ok(Some(t)) = self.read(&id) {
                        tasks.push(t);
                    }
                }
            }
        }
        Ok(tasks)
    }

    /// §8.5 依赖自动流转：depends_on 全 done → blocked/pending 任务自动 → pending + task_unblocked。
    /// 返回被解除的任务 id 列表（调用方发 task_unblocked 事件）。
    pub fn unblock_dependencies(&self) -> Result<Vec<String>> {
        let tasks = self.list()?;
        let mut unblocked = Vec::new();
        for t in tasks {
            if t.status != TaskStatus::Blocked {
                continue;
            }
            let deps_ok = t
                .depends_on
                .iter()
                .all(|d| matches!(self.read(d), Ok(Some(dt)) if dt.status == TaskStatus::Done));
            if deps_ok {
                let mut t2 = t.clone();
                t2.status = TaskStatus::Pending;
                t2.version += 1;
                self.write(&t2, t.version)?;
                unblocked.push(t.id.clone());
            }
        }
        Ok(unblocked)
    }

    /// §8.7 继承认领：owner 不在线（stale/failed/archived）时，其 in_progress 任务
    /// 优先 co_owners、其次同角色 active 窗口认领。返回 (task_id, 新 owner)。
    /// 忽略终态任务（F4）。冲突（多个同角色在线）→ 返回 Err（task_claim_conflict 语义）。
    pub fn claim_for(&self, registry: &Registry, role: &str) -> Result<Vec<(String, String)>> {
        // 候选窗口：active 且心跳有效（§5.3 在线判定）
        let candidates: Vec<String> = registry
            .list_windows()?
            .into_iter()
            .filter(|id| {
                registry
                    .read_manifest(id)
                    .map(|m| m.role == role && registry.is_online(id))
                    .unwrap_or(false)
            })
            .collect();
        if candidates.is_empty() {
            return Ok(Vec::new()); // 无同角色在线 → 孤儿（调用方发 task_orphaned）
        }
        let mut claims = Vec::new();
        for t in self.list()? {
            if t.status != TaskStatus::InProgress {
                continue;
            }
            if t.status.is_terminal() {
                continue; // F4：终态不参与认领
            }
            let owner_online = registry.is_online(&t.owner_window);
            if owner_online {
                continue;
            }
            // 优先 co_owners（H-7），其次同角色
            let next = t
                .co_owners
                .iter()
                .find(|c| candidates.contains(c))
                .or_else(|| candidates.first());
            if let Some(next_owner) = next {
                let mut t2 = t.clone();
                t2.owner_window = next_owner.clone();
                t2.version += 1;
                self.write(&t2, t.version)?;
                claims.push((t.id.clone(), next_owner.clone()));
            }
        }
        Ok(claims)
    }

    /// rrule 实例（§8.6/L-2）：模板#日期；实例带 instance_of。Phase 1 提供 id 生成。
    pub fn instance_id(template_id: &str, date: &str) -> String {
        format!("{template_id}#{date}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{Manifest, WindowStatus};
    use crate::registry::Registry;
    use crate::task::Checkpoint;

    fn task(id: &str, status: TaskStatus, owner: &str) -> Task {
        Task {
            id: id.into(),
            instance_of: None,
            title: id.into(),
            status,
            owner_window: owner.into(),
            co_owners: vec![],
            project_id: "p".into(),
            cadence: "one-shot".into(),
            rrule: None,
            priority: "p2".into(),
            depends_on: vec![],
            evidence_ref: None,
            checkpoint: Checkpoint::default(),
            version: 1,
            created_at: None,
            completed_at: None,
        }
    }

    fn manifest(id: &str, role: &str, status: WindowStatus) -> Manifest {
        Manifest {
            project_id: "p".into(),
            window_id: id.into(),
            role: role.into(),
            functions: vec![],
            version: 1,
            status,
            last_heartbeat: None,
            autonomy_level: "consult".into(),
            identity_epoch: None,
            subscribes: vec![],
            coordinates_with: vec![],
            created_at: None,
        }
    }

    /// H-1 正面：乐观锁写入 + 读回。
    #[test]
    fn test_ledger_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = TaskLedger::open(dir.path()).unwrap();
        let t = task("T-1", TaskStatus::Pending, "w1");
        ledger.write(&t, 0).unwrap();
        let t2 = ledger.read("T-1").unwrap().unwrap();
        assert_eq!(t2.id, "T-1");
        assert_eq!(t2.status, TaskStatus::Pending);
    }

    /// H-1 负面：乐观锁冲突（期望版本不匹配）→ 拒。
    #[test]
    fn test_optimistic_lock_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = TaskLedger::open(dir.path()).unwrap();
        let t = task("T-1", TaskStatus::Pending, "w1");
        ledger.write(&t, 0).unwrap();
        // 另一写者以错误版本写 → 拒
        let mut t2 = t.clone();
        t2.title = "改".into();
        let err = ledger.write(&t2, 999).unwrap_err();
        assert!(err.to_string().contains("乐观锁"), "版本冲突必须拒: {err}");
    }

    /// §8.5 依赖流转：T-A depends T-B，T-B done → T-A blocked→pending（task_unblocked 语义）。
    #[test]
    fn test_unblock_dependencies() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = TaskLedger::open(dir.path()).unwrap();
        let mut ta = task("T-A", TaskStatus::Blocked, "w1");
        ta.depends_on = vec!["T-B".into()];
        ledger.write(&ta, 0).unwrap();
        ledger
            .write(&task("T-B", TaskStatus::Done, "w1"), 0)
            .unwrap();
        let unblocked = ledger.unblock_dependencies().unwrap();
        assert!(unblocked.contains(&"T-A".to_string()), "依赖全 done 应解除");
        assert_eq!(
            ledger.read("T-A").unwrap().unwrap().status,
            TaskStatus::Pending
        );
    }

    /// F4 负面：depends_on 引用 expired（非 done）任务 → T-A 不因"依赖全完成"误判解除。
    #[test]
    fn test_expired_dependency_does_not_unblock() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = TaskLedger::open(dir.path()).unwrap();
        let mut ta = task("T-A", TaskStatus::Blocked, "w1");
        ta.depends_on = vec!["T-B".into()];
        ledger.write(&ta, 0).unwrap();
        ledger
            .write(&task("T-B", TaskStatus::Expired, "w1"), 0)
            .unwrap();
        let unblocked = ledger.unblock_dependencies().unwrap();
        assert!(
            !unblocked.contains(&"T-A".to_string()),
            "expired 依赖不得触发解除"
        );
        assert_eq!(
            ledger.read("T-A").unwrap().unwrap().status,
            TaskStatus::Blocked,
            "T-A 保持 blocked"
        );
    }

    /// §8.7：owner 崩溃（stale）→ 同角色 active 窗口认领 in_progress 任务。
    #[test]
    fn test_claim_after_owner_stale() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = TaskLedger::open(dir.path().join("tasks")).unwrap();
        let reg = Registry::open(dir.path().join("registry")).unwrap();
        // owner 崩溃（failed 窗口，心跳有效也离线）
        reg.write_manifest(&manifest("w-owner", "施工", WindowStatus::Failed))
            .unwrap();
        reg.touch_heartbeat("w-owner").unwrap();
        // 候选同角色窗口
        reg.write_manifest(&manifest("w-impl2", "施工", WindowStatus::Active))
            .unwrap();
        reg.touch_heartbeat("w-impl2").unwrap();
        ledger
            .write(&task("T-9", TaskStatus::InProgress, "w-owner"), 0)
            .unwrap();
        let claims = ledger.claim_for(&reg, "施工").unwrap();
        assert!(!claims.is_empty(), "owner 不在线应认领");
        assert_eq!(claims[0].1, "w-impl2");
        assert_eq!(ledger.read("T-9").unwrap().unwrap().owner_window, "w-impl2");
    }

    /// F4 负面：终态任务（failed）不被认领源选中。
    #[test]
    fn test_terminal_task_not_claimed() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = TaskLedger::open(dir.path().join("tasks")).unwrap();
        let reg = Registry::open(dir.path().join("registry")).unwrap();
        reg.write_manifest(&manifest("w-owner", "施工", WindowStatus::Failed))
            .unwrap();
        reg.touch_heartbeat("w-owner").unwrap();
        reg.write_manifest(&manifest("w-impl2", "施工", WindowStatus::Active))
            .unwrap();
        reg.touch_heartbeat("w-impl2").unwrap();
        ledger
            .write(&task("T-9", TaskStatus::Failed, "w-owner"), 0)
            .unwrap();
        let claims = ledger.claim_for(&reg, "施工").unwrap();
        assert!(claims.is_empty(), "failed 任务不参与认领（F4）");
    }

    /// L-2：rrule 实例 ID 模板#日期。
    #[test]
    fn test_instance_id() {
        assert_eq!(
            TaskLedger::instance_id("T-040", "2026-08-26"),
            "T-040#2026-08-26"
        );
    }
}
