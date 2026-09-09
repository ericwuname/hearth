//! 事件模型（PWC §4/§7.2/§7.7）——F1 schema_version + F2 UUIDv7 event_id。
//! 事件类型目录 §7.7：生命周期 / 任务 / 同步 / 请求响应 / 知识冲突 / schema 拒绝。

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 当前事件结构版本（F1）。未知版本 → migration layer 拒绝并写 `schema_version_rejected`。
pub const SCHEMA_VERSION: &str = "1.0";

/// 已知可迁移的历史版本（迁移层白名单；空 = 只有 1.0）。
pub const KNOWN_SCHEMA_VERSIONS: &[&str] = &["1.0"];

/// 事件类型（§7.7 目录——migration 与契约校验用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // 生命周期
    WindowBootFailed,
    WindowStale,
    RoleUpdated,
    IdentityEpochMismatch,
    // 任务
    TaskDone,
    TaskUnblocked,
    TaskOrphaned,
    TaskClaimConflict,
    Assign,
    RequestTimeout,
    // 同步/协调
    SubscriptionChanged,
    LockContention,
    EventGapAlert,
    Noop,
    // 请求/响应（H-5）
    Request,
    Response,
    // 知识冲突（P2-02）——正确性冲突提请人工/审计，非文件一致性
    KnowledgeConflict,
    // schema 拒绝（第五轮订正）——未知 schema_version，语义独立于 event_gap_alert/window_boot_failed
    SchemaVersionRejected,
}

/// 事件指针（EVENT_LOG 一行，§7.2）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// F1：事件结构版本（"1.0"）。
    #[serde(default = "default_schema")]
    pub schema_version: String,
    /// 全局单调"允许空洞"（F3：唯一责任方分配）。
    pub seq: u64,
    /// F2：UUIDv7——时间有序 + 分布式唯一。
    pub event_id: String,
    /// 本机 window_id（§7.5 身份防伪：API 自动填充，拒显式传入）。
    pub actor: String,
    /// 角色（与 roles.toml 一致）。
    #[serde(default)]
    pub role: String,
    /// 事件类型。
    #[serde(rename = "type")]
    pub etype: EventType,
    /// 目标引用（可指向 memory/ 或文件，H-4 不可变版本）。
    #[serde(default)]
    pub target_ref: Option<String>,
    /// target_ref 指向文件内容的哈希（消费者校验，H-4）。
    #[serde(default)]
    pub content_hash: Option<String>,
    /// 交接给谁（路由，L-1）。
    #[serde(default)]
    pub hands_off_to: Option<Vec<String>>,
    /// 被回复事件（H-5）。
    #[serde(default)]
    pub in_reply_to: Option<String>,
    /// 请求超时秒数（H-5）。
    #[serde(default)]
    pub await_timeout: Option<u64>,
    /// 摘要。
    #[serde(default)]
    pub summary: Option<String>,
    /// 状态（如 done/pending…）。
    #[serde(default)]
    pub status: Option<String>,
    /// 上一条哈希（哈希链，M-4/L-4）。
    #[serde(default)]
    pub prev_hash: Option<String>,
    /// 生产方提供的粒度（L-7）。
    #[serde(default)]
    pub provided_granularity: Option<String>,
}

fn default_schema() -> String {
    SCHEMA_VERSION.to_string()
}

impl Event {
    /// 生成 UUIDv7 event_id（F2：时间有序 + 分布式唯一）。
    pub fn new_uuid_v7() -> String {
        Uuid::now_v7().to_string()
    }

    /// 校验 schema_version（F1 migration 层入口）。
    /// 未知版本 → Err（调用方负责写 `schema_version_rejected` 事件，不静默跳过）。
    pub fn validate_schema(&self) -> std::result::Result<(), String> {
        if KNOWN_SCHEMA_VERSIONS.contains(&self.schema_version.as_str()) {
            Ok(())
        } else {
            Err(format!(
                "unknown schema_version '{}' (known: {:?})",
                self.schema_version, KNOWN_SCHEMA_VERSIONS
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// F2 正面：UUIDv7 时间有序（字节序与生成序一致）+ 格式可解析。
    #[test]
    fn test_uuid_v7_time_ordered() {
        let a = Event::new_uuid_v7();
        let b = Event::new_uuid_v7();
        let c = Event::new_uuid_v7();
        assert!(
            a < b && b < c,
            "UUIDv7 时间前缀应按生成序单调: {a} < {b} < {c}"
        );
        let parsed = Uuid::parse_str(&b).expect("UUIDv7 可解析");
        assert_eq!(parsed.get_version_num(), 7, "版本号必须为 7");
    }

    /// F2 负面：大量生成零冲突（并发由调用方测试——event_log 层）。
    #[test]
    fn test_uuid_v7_no_collision_bulk() {
        let mut set = std::collections::HashSet::new();
        for _ in 0..100_000 {
            assert!(set.insert(Event::new_uuid_v7()), "10^5 内零冲突");
        }
    }

    /// F1 负面：未知 schema_version 必须被拒（不静默接受）。
    #[test]
    fn test_schema_rejected_unknown_version() {
        let e = Event {
            schema_version: "9.9".into(),
            seq: 1,
            event_id: Event::new_uuid_v7(),
            actor: "impl-01".into(),
            role: "施工".into(),
            etype: EventType::TaskDone,
            target_ref: None,
            content_hash: None,
            hands_off_to: None,
            in_reply_to: None,
            await_timeout: None,
            summary: None,
            status: None,
            prev_hash: None,
            provided_granularity: None,
        };
        let err = e.validate_schema().unwrap_err();
        assert!(
            err.contains("unknown schema_version"),
            "拒绝未知版本: {err}"
        );
    }

    /// F1 正面：1.0 通过。
    #[test]
    fn test_schema_known_passes() {
        let e = Event {
            schema_version: SCHEMA_VERSION.into(),
            seq: 1,
            event_id: Event::new_uuid_v7(),
            actor: "impl-01".into(),
            role: "施工".into(),
            etype: EventType::TaskDone,
            target_ref: None,
            content_hash: None,
            hands_off_to: None,
            in_reply_to: None,
            await_timeout: None,
            summary: None,
            status: None,
            prev_hash: None,
            provided_granularity: None,
        };
        assert!(e.validate_schema().is_ok());
    }
}
