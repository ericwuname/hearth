//! 跨项目 IMPORT（Phase 2，§11.2）+ P2-02 knowledge_conflict + P2-03 trust_level 四类。
//! 项目级隔离 default-deny（P1）：跨项目读取默认拒绝，IMPORT 显式开启。

use serde::{Deserialize, Serialize};

/// IMPORT 信任等级（P2-03 四类，取代 high/medium/low）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TrustLevel {
    /// 本组织内部来源。
    Internal,
    /// 已验证来源（人工/程序核验过）。
    Verified,
    /// 外部来源（可信但未验证）。
    External,
    /// 未知来源（默认，最低信任）。
    #[default]
    Unknown,
}

/// IMPORT provenance（§11.2）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProvenance {
    pub source_project_id: String,
    pub source_role: String,
    pub source_timestamp: String,
    pub import_reason: String,
    pub trust_level: TrustLevel,
}

/// 导入策略（project.toml default_import_policy）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportPolicy {
    Allow,
    Deny,
}

impl ImportPolicy {
    pub fn parse(s: &str) -> ImportPolicy {
        if s.eq_ignore_ascii_case("allow") {
            ImportPolicy::Allow
        } else {
            ImportPolicy::Deny // default-deny（P1）
        }
    }
}

/// 校验 IMPORT（§11.2 + M-6 + P2-03）：
/// - policy=deny → 拒（default-deny，显式开启才允许）
/// - trust_level 非法（不在四类）→ 拒
/// - 成功 → 生成 [imported] 标签 + provenance（写入目标项目 artifact 的 metadata）
pub fn import_ok(
    policy: ImportPolicy,
    prov: &ImportProvenance,
) -> std::result::Result<ImportArtifact, String> {
    if policy == ImportPolicy::Deny {
        return Err(
            "IMPORT 被拒：项目 default_import_policy=deny（P1 default-deny，需显式开启）"
                .to_string(),
        );
    }
    // P2-03：trust_level 必须是四类之一（serde 已保证——此处防御性校验）
    match prov.trust_level {
        TrustLevel::Internal
        | TrustLevel::Verified
        | TrustLevel::External
        | TrustLevel::Unknown => {}
    }
    Ok(ImportArtifact {
        imported: true,
        tag: "[imported]".to_string(),
        provenance: prov.clone(),
    })
}

/// IMPORT 产物（带 [imported] 标签 + provenance）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportArtifact {
    pub imported: bool,
    pub tag: String,
    pub provenance: ImportProvenance,
}

/// P2-02 knowledge_conflict 判定：新知识是否与既有知识**正确性**冲突
/// （非文件一致性——文件层由最后写入+prev_hash 裁决，这里判语义冲突）。
/// 冲突 → 调用方写 knowledge_conflict 事件提请审计，不被最后写入规则静默吞。
pub fn is_knowledge_conflict(
    existing: &str,
    incoming: &str,
    existing_trust: TrustLevel,
    incoming_trust: TrustLevel,
) -> bool {
    let same = existing.trim() == incoming.trim();
    if same {
        return false;
    }
    // 语义冲突启发式：内容不同 + 双方都有实质信任（内/验证）→ 冲突（不静默覆盖）
    matches!(
        (existing_trust, incoming_trust),
        (
            TrustLevel::Internal | TrustLevel::Verified,
            TrustLevel::Internal | TrustLevel::Verified,
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// M-6 负面：deny 策略下 IMPORT 被拒（default-deny）。
    #[test]
    fn test_import_deny_rejected() {
        let prov = ImportProvenance {
            source_project_id: "other".into(),
            source_role: "施工".into(),
            source_timestamp: "2026-08-26".into(),
            import_reason: "复用实现".into(),
            trust_level: TrustLevel::Verified,
        };
        let err = import_ok(ImportPolicy::Deny, &prov).unwrap_err();
        assert!(err.contains("deny"), "default-deny 必须拒: {err}");
    }

    /// §11.2 正面：allow 策略 + 合法 trust_level → 成功带 [imported] 标签。
    #[test]
    fn test_import_allow_ok() {
        let prov = ImportProvenance {
            source_project_id: "other".into(),
            source_role: "施工".into(),
            source_timestamp: "2026-08-26".into(),
            import_reason: "复用实现".into(),
            trust_level: TrustLevel::Verified,
        };
        let art = import_ok(ImportPolicy::Allow, &prov).unwrap();
        assert!(art.imported);
        assert_eq!(art.tag, "[imported]");
    }

    /// P2-03 负面：非法 trust_level 被拒——serde 解析失败即拒绝（四类之外无法构造）。
    #[test]
    fn test_invalid_trust_level_rejected() {
        let err = toml::from_str::<ImportProvenance>(
            "source_project_id='x'\nsource_role='r'\nsource_timestamp='t'\nimport_reason='r'\ntrust_level='super_duper'\n",
        );
        assert!(err.is_err(), "非法 trust_level 必须解析失败");
    }

    /// P2-02 正面：内部/验证双信任 + 内容不同 → 知识冲突（不被最后写入静默吞）。
    #[test]
    fn test_knowledge_conflict_detected() {
        assert!(is_knowledge_conflict(
            "结论：A 是正确方案",
            "结论：A 是错误的",
            TrustLevel::Internal,
            TrustLevel::Verified
        ));
    }

    /// P2-02 负面：内容相同 → 不误报；未知信任内容不同 → 不报（弱证据不值得升级冲突）。
    #[test]
    fn test_knowledge_conflict_no_false_positive() {
        assert!(!is_knowledge_conflict(
            "同内容",
            "同内容",
            TrustLevel::Internal,
            TrustLevel::Internal
        ));
        assert!(
            !is_knowledge_conflict("A", "B", TrustLevel::Unknown, TrustLevel::External),
            "未知/外部信任不升级为冲突（防噪音）"
        );
    }
}
