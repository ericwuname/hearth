//! 任务模型（§8.2）+ 状态机（§8.5）+ F4 异常终态 + F5 权限矩阵（§8.7.1）。

use serde::{Deserialize, Serialize};

/// 任务状态全集（F4：failed/cancelled/expired 为吸收性终态——不可再流转、不被认领源选中）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Paused,
    Done,
    Blocked,
    Failed,
    Cancelled,
    Expired,
}

impl TaskStatus {
    /// F4：终态判定（failed/cancelled/expired——吸收性）。
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            TaskStatus::Failed | TaskStatus::Cancelled | TaskStatus::Expired
        )
    }

    /// §8.5 状态机：from → to 是否合法。
    /// 合法迁移：pending→in_progress；in_progress⇄paused；in_progress/blocked→done；
    /// 任何 active→failed/cancelled/expired。终态不可再迁移到任何状态。
    pub fn can_transition(self, to: TaskStatus) -> bool {
        if self.is_terminal() {
            return false; // F4：终态不可再流转
        }
        match (self, to) {
            (_, s) if s.is_terminal() => true, // 任意非终态 → 终态（失败/取消/到期）
            (TaskStatus::Pending, TaskStatus::InProgress) => true,
            (TaskStatus::InProgress, TaskStatus::Paused) => true,
            (TaskStatus::Paused, TaskStatus::InProgress) => true,
            (TaskStatus::InProgress, TaskStatus::Done) => true,
            (TaskStatus::Blocked, TaskStatus::Done) => true,
            (TaskStatus::Blocked, TaskStatus::Pending) => true, // 依赖满足自动流转（§8.5）
            (TaskStatus::Pending, TaskStatus::Blocked) => true,
            (TaskStatus::InProgress, TaskStatus::Blocked) => true,
            (TaskStatus::Done, _) => false,
            _ => false,
        }
    }
}

/// 结构化 checkpoint（M-8）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Checkpoint {
    #[serde(default)]
    pub progress: String,
    #[serde(default)]
    pub context_refs: Vec<String>,
    #[serde(default)]
    pub next_action: String,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub by_window: Option<String>,
}

/// 任务（§8.2 每任务一文件）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    #[serde(default)]
    pub instance_of: Option<String>,
    pub title: String,
    pub status: TaskStatus,
    /// owner_window（§7.5 身份防伪：由本机 manifest 自动填充）。
    pub owner_window: String,
    #[serde(default)]
    pub co_owners: Vec<String>,
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub cadence: String,
    #[serde(default)]
    pub rrule: Option<String>,
    #[serde(default)]
    pub priority: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub evidence_ref: Option<String>,
    #[serde(default)]
    pub checkpoint: Checkpoint,
    /// 乐观锁（H-1）。
    pub version: u64,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
}

/// 权限角色（F5）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessRole {
    Owner,
    CoOwner,
    Observer,
}

impl Task {
    /// 调用方对任务的权限（F5 §8.7.1）。
    pub fn access_role(&self, window_id: &str) -> AccessRole {
        if self.owner_window == window_id {
            AccessRole::Owner
        } else if self.co_owners.iter().any(|c| c == window_id) {
            AccessRole::CoOwner
        } else {
            AccessRole::Observer
        }
    }

    /// F5：状态迁移仅 owner。
    pub fn can_migrate(&self, window_id: &str) -> bool {
        self.access_role(window_id) == AccessRole::Owner
    }

    /// F5：写 checkpoint / 追加 evidence 允许 owner + co_owner。
    pub fn can_write_progress(&self, window_id: &str) -> bool {
        matches!(
            self.access_role(window_id),
            AccessRole::Owner | AccessRole::CoOwner
        )
    }

    /// 尝试状态迁移（owner 独有 + 状态机校验）。返回新 status（Err=非法迁移/越权）。
    pub fn transition(
        &mut self,
        window_id: &str,
        to: TaskStatus,
    ) -> std::result::Result<TaskStatus, String> {
        if !self.can_migrate(window_id) {
            return Err(format!(
                "越权：window '{}' 非 owner '{}'，无权状态迁移（F5）",
                window_id, self.owner_window
            ));
        }
        if !self.status.can_transition(to) {
            return Err(format!(
                "非法迁移：{:?} → {:?}（终态不可再流转）",
                self.status, to
            ));
        }
        self.status = to;
        if to.is_terminal() || to == TaskStatus::Done {
            self.completed_at = Some(chrono::Utc::now().to_rfc3339());
        }
        self.version += 1;
        Ok(self.status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(status: TaskStatus) -> Task {
        Task {
            id: "T-1".into(),
            instance_of: None,
            title: "t".into(),
            status,
            owner_window: "w-owner".into(),
            co_owners: vec!["w-co".into()],
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

    /// §8.5 正面：in_progress → failed 合法（执行失败）。
    #[test]
    fn test_transition_to_failed_ok() {
        let mut t = task(TaskStatus::InProgress);
        let s = t.transition("w-owner", TaskStatus::Failed).unwrap();
        assert_eq!(s, TaskStatus::Failed);
        assert!(t.completed_at.is_some(), "终态记录完成时间");
    }

    /// F4 负面：failed → in_progress 被拒（终态不可再流转）。
    #[test]
    fn test_terminal_cannot_transition_back() {
        let mut t = task(TaskStatus::Failed);
        let err = t.transition("w-owner", TaskStatus::InProgress).unwrap_err();
        assert!(err.contains("非法迁移"), "终态回流必须拒: {err}");
    }

    /// F4 负面：expired 任务不被认领源选中（is_terminal 语义 + 认领过滤在 ledger 层）。
    #[test]
    fn test_expired_is_terminal() {
        assert!(TaskStatus::Expired.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
        assert!(
            !TaskStatus::Blocked.is_terminal(),
            "blocked 非终态（可解除）"
        );
    }

    /// F5 正面：owner 迁移 done 成功；co_owner 写 checkpoint 成功；observer 读。
    #[test]
    fn test_permission_matrix_positive() {
        let mut t = task(TaskStatus::InProgress);
        assert!(t.can_migrate("w-owner"));
        assert!(t.can_write_progress("w-co"));
        assert!(!t.can_write_progress("w-observer"));
        // co_owner 迁移被拒
        assert!(t.transition("w-co", TaskStatus::Done).is_err());
        // owner 迁移成功
        assert!(t.transition("w-owner", TaskStatus::Done).is_ok());
    }

    /// F5 负面：observer 尝试状态迁移 → 拒（非 panic）。
    #[test]
    fn test_observer_migrate_rejected() {
        let mut t = task(TaskStatus::Pending);
        let err = t
            .transition("w-observer", TaskStatus::InProgress)
            .unwrap_err();
        assert!(err.contains("越权"), "observer 迁移必须拒: {err}");
    }
}
