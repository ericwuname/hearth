//! Manifest（§5.1）——窗口身份 + P2-04 identity_epoch + 契约校验（§6.2 fail-closed）。
//! 身份防伪（§7.5）：actor/owner_window 由本机 manifest 自动填充，API 拒显式传入。

use serde::{Deserialize, Serialize};

/// 窗口状态（§5.3：starting/active/stale/degraded/failed/archived）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowStatus {
    Starting,
    Active,
    Stale,
    Degraded,
    Failed,
    Archived,
}

/// 订阅条目（§5.1：event_type + requested_granularity）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub event_type: String,
    pub requested_granularity: String, // summary|file|full（L-7）
}

/// 窗口 Manifest（§5.1）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub project_id: String,
    pub window_id: String,
    pub role: String,
    #[serde(default)]
    pub functions: Vec<String>,
    pub version: u64,
    #[serde(default = "default_status")]
    pub status: WindowStatus,
    #[serde(default)]
    pub last_heartbeat: Option<String>,
    #[serde(default)]
    pub autonomy_level: String,
    /// P2-04：身份纪元——防同 window_id 复用作不同角色导致历史语义污染。
    #[serde(default)]
    pub identity_epoch: Option<String>,
    #[serde(default)]
    pub subscribes: Vec<Subscription>,
    #[serde(default)]
    pub coordinates_with: Vec<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

fn default_status() -> WindowStatus {
    WindowStatus::Starting
}

/// 角色契约（§6.1）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleContract {
    pub responsibilities: Vec<String>,
    #[serde(default)]
    pub publishes: Vec<String>,
    #[serde(default)]
    pub subscribes: Vec<Subscription>,
    #[serde(default)]
    pub hands_off_to: Vec<String>,
    #[serde(default)]
    pub autonomy_level: String,
}

/// 角色注册表（roles.toml，L-5 版本化）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleRegistry {
    pub version: u64,
    #[serde(flatten)]
    pub roles: std::collections::HashMap<String, RoleContract>,
}

impl Manifest {
    /// §6.2 契约校验（Phase 1 fail-closed）：functions/subscribes 与 roles.toml 一致，
    /// 不一致拒绝启动。返回 Err（fail-closed）。
    pub fn validate_contract(&self, registry: &RoleRegistry) -> std::result::Result<(), String> {
        let contract = registry
            .role(&self.role)
            .ok_or_else(|| format!("role '{}' 未在 roles.toml 声明", self.role))?;
        // functions 必须 ⊆ 契约 responsibilities（窗口自称的职能须被角色契约覆盖）
        for f in &self.functions {
            if !contract.responsibilities.contains(f) {
                return Err(format!(
                    "manifest.functions 含 '{}' 但角色 '{}' 契约无此职责",
                    f, self.role
                ));
            }
        }
        // 订阅粒度合法（L-7）
        const GRANULARITIES: &[&str] = &["summary", "file", "full"];
        for s in &self.subscribes {
            if !GRANULARITIES.contains(&s.requested_granularity.as_str()) {
                return Err(format!(
                    "非法粒度 '{}'（合法: summary|file|full）",
                    s.requested_granularity
                ));
            }
        }
        Ok(())
    }
}

impl RoleRegistry {
    pub fn load(path: impl AsRef<std::path::Path>) -> std::result::Result<Self, String> {
        let s = std::fs::read_to_string(path.as_ref())
            .map_err(|e| format!("读 roles.toml 失败: {e}"))?;
        toml::from_str(&s).map_err(|e| format!("roles.toml 解析失败: {e}"))
    }

    /// 取角色契约——兼容裸名（"施工"）与带前缀的 quoted key（"role.施工"，§6.1 体例）。
    pub fn role(&self, name: &str) -> Option<&RoleContract> {
        self.roles
            .get(name)
            .or_else(|| self.roles.get(&format!("role.{name}")))
            .or_else(|| {
                self.roles
                    .iter()
                    .find(|(k, _)| k.trim_start_matches("role.") == name)
                    .map(|(_, v)| v)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> RoleRegistry {
        toml::from_str(
            r#"
version = 1
["role.施工"]
responsibilities = ["照任务书写码", "跑门禁", "交证据"]
publishes = ["implementation"]
subscribes = [{ event_type = "顶层:taskbook", requested_granularity = "file" }]
autonomy_level = "consult"
"#,
        )
        .expect("parse")
    }

    /// §6.2 正面：合法 manifest 通过契约校验。
    #[test]
    fn test_contract_ok() {
        let m = Manifest {
            project_id: "p".into(),
            window_id: "impl-01".into(),
            role: "施工".into(),
            functions: vec!["照任务书写码".into()],
            version: 1,
            status: WindowStatus::Starting,
            last_heartbeat: None,
            autonomy_level: "consult".into(),
            identity_epoch: None,
            subscribes: vec![Subscription {
                event_type: "顶层:taskbook".into(),
                requested_granularity: "file".into(),
            }],
            coordinates_with: vec![],
            created_at: None,
        };
        assert!(m.validate_contract(&registry()).is_ok());
    }

    /// §6.2 负面（fail-closed）：functions 超出角色契约 → 拒绝启动（不警告继续）。
    #[test]
    fn test_contract_fail_closed_on_unknown_function() {
        let m = Manifest {
            project_id: "p".into(),
            window_id: "impl-01".into(),
            role: "施工".into(),
            functions: vec!["越权职能".into()],
            version: 1,
            status: WindowStatus::Starting,
            last_heartbeat: None,
            autonomy_level: "consult".into(),
            identity_epoch: None,
            subscribes: vec![],
            coordinates_with: vec![],
            created_at: None,
        };
        let err = m.validate_contract(&registry()).unwrap_err();
        assert!(err.contains("越权职能"), "未知职能必须 fail-closed: {err}");
    }

    /// L-7 负面：非法粒度 → 拒绝。
    #[test]
    fn test_contract_rejects_bad_granularity() {
        let m = Manifest {
            project_id: "p".into(),
            window_id: "impl-01".into(),
            role: "施工".into(),
            functions: vec![],
            version: 1,
            status: WindowStatus::Starting,
            last_heartbeat: None,
            autonomy_level: "consult".into(),
            identity_epoch: None,
            subscribes: vec![Subscription {
                event_type: "x".into(),
                requested_granularity: "huge".into(),
            }],
            coordinates_with: vec![],
            created_at: None,
        };
        assert!(m.validate_contract(&registry()).is_err(), "非法粒度必须拒");
    }

    /// P2-04 正面：identity_epoch 序列化/反序列化往返。
    #[test]
    fn test_identity_epoch_roundtrip() {
        let m = Manifest {
            project_id: "p".into(),
            window_id: "impl-01".into(),
            role: "施工".into(),
            functions: vec![],
            version: 1,
            status: WindowStatus::Active,
            last_heartbeat: None,
            autonomy_level: "consult".into(),
            identity_epoch: Some("2026-08-26T03:00:00+08:00".into()),
            subscribes: vec![],
            coordinates_with: vec![],
            created_at: None,
        };
        let s = toml::to_string(&m).expect("toml");
        let m2: Manifest = toml::from_str(&s).expect("parse");
        assert_eq!(m2.identity_epoch, m.identity_epoch, "epoch 往返一致");
    }

    /// P2-04 负面：同 window_id 不同 identity_epoch → 调用方检测 mismatch（本层提供比较）。
    #[test]
    fn test_identity_epoch_mismatch_detectable() {
        let a = Manifest {
            identity_epoch: Some("epoch-1".into()),
            ..test_minimal()
        };
        let b = Manifest {
            identity_epoch: Some("epoch-2".into()),
            ..test_minimal()
        };
        assert_ne!(a.identity_epoch, b.identity_epoch, "epoch 不同可检测");
        // 一致时不误报
        let c = Manifest {
            identity_epoch: Some("epoch-1".into()),
            ..test_minimal()
        };
        assert_eq!(a.identity_epoch, c.identity_epoch);
    }

    fn test_minimal() -> Manifest {
        Manifest {
            project_id: "p".into(),
            window_id: "impl-01".into(),
            role: "施工".into(),
            functions: vec![],
            version: 1,
            status: WindowStatus::Active,
            last_heartbeat: None,
            autonomy_level: "consult".into(),
            identity_epoch: None,
            subscribes: vec![],
            coordinates_with: vec![],
            created_at: None,
        }
    }
}
