//! # PWC · 项目间窗口协作模块（Phase 1 工具化）
//!
//! 实现 `docs/project-window-sync-design.md`（FINAL v1）Phase 1 职责 + `docs/pwc-final-v1-patch-spec.md`
//! 补丁 F1–F5 / P2-01~04。**不污染 `crates/bridge`**（智能层 vs 组织层，互补不合并）。
//!
//! 模块地图：
//! - `event`：事件模型（schema_version / UUIDv7 event_id / 类型目录）
//! - `event_log`：EVENT_LOG 引擎（JSONL+跨平台锁+seq allocator+prev_hash 链+migration+gap+水位线）
//! - `manifest` / `roles`：窗口身份与角色契约（fail-closed 校验 / identity_epoch）
//! - `registry`：窗口清册（heartbeat / 水位线 / 乐观锁 / stale 清理）
//! - `task` / `task_ledger`：任务模型（F4 终态）与 Ledger（状态机/依赖流转/认领/F5 权限矩阵）
//! - `fs_api`：受限文件 API（canonicalize+前缀，§7.5 路径安全）
//! - `subscription`：订阅过滤 + 粒度 + 动态订阅（M-10）
//! - `stream`：EVENT_STREAM 逻辑/物理分层（P2-01，按月切片）
//! - `import`（Phase 2）：跨项目 IMPORT + trust_level（P2-03）+ knowledge_conflict（P2-02）
//!
//! 守门员铁律：每条能力配「故意造失败」负面测试（断言不能失败 = 断言不存在）。

pub mod event;
pub mod event_log;
pub mod fs_api;
pub mod import;
pub mod manifest;
pub mod registry;
pub mod stream;
pub mod subscription;
pub mod task;
pub mod task_ledger;

/// PWC 统一错误。
#[derive(Debug, thiserror::Error)]
pub enum PwcError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, PwcError>;

impl From<String> for PwcError {
    fn from(s: String) -> Self {
        PwcError::Other(s)
    }
}

impl From<&str> for PwcError {
    fn from(s: &str) -> Self {
        PwcError::Other(s.to_string())
    }
}
