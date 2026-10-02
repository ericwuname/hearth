// v11.4 Phase 0: Subconscious layer — signal-based constraints.
// Moves constitution/cost/error/repetition checks OUT of LLM prompt.
// Each guard returns Option<IntuitionSignal>; Gate merges them into a PhaseOverride.

use serde::Serialize;

/// Signal produced by a subconscious guard.
#[derive(Debug, Clone, Serialize)]
pub struct IntuitionSignal {
    pub kind: IntuitionKind,
    pub action: PhaseOverride,
    pub reason: String,
}

/// What kind of intuition was triggered.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum IntuitionKind {
    ConstitutionViolation(String),
    ResourceInterrupt,
    Deviation { severity: f32 },
}

/// Override the next loop phase — skip LLM entirely.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum PhaseOverride {
    /// Abandon current action — it violates rules.
    Abandon,
    /// Simplify goal — resource pressure.
    Simplify,
}

/// Trait for a single subconscious guard.
#[async_trait::async_trait]
pub trait SubconsciousGuard: Send + Sync {
    async fn check(&self, ctx: &GuardContext) -> Option<IntuitionSignal>;
}

/// Context provided to each guard during check.
///
/// D-98（2026-10-02, traecode）：**删除 4 个只写不读字段**——`goal_text` /
/// `step_count` / `last_success` / `constitution_summary`。两个现役 guard
/// （`ConstitutionGuard` / `CostGuard`）只读 `last_action` 与 `cost_ratio`，
/// 其余字段由 `agent-core` 每步填好后**无人读取**（write-only），属"定义了但
/// 无人用"（同 D-47/D-88 口径）。它们的原始设计意图（goal 漂移/步数/上步成败/
/// 宪法全文）若要成为真守卫，需**先设计判定规则**再收回字段——那属能力扩展。
pub struct GuardContext {
    /// 上一个动作（命令文本）——`ConstitutionGuard` 用它匹配危险命令。
    pub last_action: Option<String>,
    /// v11.5: Cost ratio (0.0 = unavailable) —— `CostGuard` 的唯一输入。
    pub cost_ratio: f32,
}

// ─── Guards ───

/// ConstitutionGuard: checks if a proposed action violates core rules.
pub struct ConstitutionGuard;

#[async_trait::async_trait]
impl SubconsciousGuard for ConstitutionGuard {
    async fn check(&self, ctx: &GuardContext) -> Option<IntuitionSignal> {
        let forbidden: &[&str] = &["rm -rf /", "rm -rf ~", "del /S /Q C:\\"];
        if let Some(ref action) = ctx.last_action {
            for f in forbidden {
                if action.contains(f) {
                    return Some(IntuitionSignal {
                        kind: IntuitionKind::ConstitutionViolation(f.to_string()),
                        action: PhaseOverride::Abandon,
                        reason: format!("blocked dangerous command: {}", f),
                    });
                }
            }
        }
        None
    }
}

/// CostGuard: budget/cost stop-loss signals reading from GuardContext.
pub struct CostGuard;

#[async_trait::async_trait]
impl SubconsciousGuard for CostGuard {
    async fn check(&self, ctx: &GuardContext) -> Option<IntuitionSignal> {
        if ctx.cost_ratio > 0.95 {
            Some(IntuitionSignal {
                kind: IntuitionKind::ResourceInterrupt,
                action: PhaseOverride::Simplify,
                reason: format!("cost {:.0}% of budget", ctx.cost_ratio * 100.0),
            })
        } else if ctx.cost_ratio > 0.80 {
            Some(IntuitionSignal {
                kind: IntuitionKind::Deviation { severity: 0.8 },
                action: PhaseOverride::Simplify,
                reason: "cost approaching budget limit".into(),
            })
        } else {
            None
        }
    }
}

// ─── Gate ───

/// Orchestrates all subconscious guards in priority order.
pub struct SubconsciousGate {
    guards: Vec<Box<dyn SubconsciousGuard>>,
}

impl SubconsciousGate {
    pub fn new() -> Self {
        Self {
            guards: vec![Box::new(ConstitutionGuard), Box::new(CostGuard)],
        }
    }

    /// Run all guards, return the first override found (priority order).
    pub async fn check(&self, ctx: &GuardContext) -> Option<IntuitionSignal> {
        for guard in &self.guards {
            if let Some(signal) = guard.check(ctx).await {
                tracing::info!(
                    kind = ?signal.kind,
                    reason = %signal.reason,
                    "subconscious override"
                );
                return Some(signal);
            }
        }
        None
    }
}

impl Default for SubconsciousGate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn constitution_guard_blocks_rm_rf() {
        let ctx = GuardContext {
            last_action: Some("rm -rf /".into()),
            cost_ratio: 0.0,
        };
        let sig = ConstitutionGuard.check(&ctx).await.unwrap();
        assert_eq!(sig.action, PhaseOverride::Abandon);
    }

    #[tokio::test]
    async fn constitution_guard_allows_normal() {
        let ctx = GuardContext {
            last_action: Some("cargo build".into()),
            cost_ratio: 0.0,
        };
        assert!(ConstitutionGuard.check(&ctx).await.is_none());
    }

    #[tokio::test]
    async fn cost_guard_triggers_at_96pct() {
        let g = CostGuard;
        let ctx = GuardContext {
            last_action: None,
            cost_ratio: 0.96,
        };
        let sig = g.check(&ctx).await.unwrap();
        assert_eq!(sig.action, PhaseOverride::Simplify);
    }

    #[tokio::test]
    async fn cost_guard_ignores_at_50pct() {
        let g = CostGuard;
        let ctx = GuardContext {
            last_action: None,
            cost_ratio: 0.50,
        };
        assert!(g.check(&ctx).await.is_none());
    }

    #[tokio::test]
    async fn gate_stops_at_first_guard() {
        let gate = SubconsciousGate::new();
        let ctx = GuardContext {
            last_action: Some("rm -rf /".into()),
            cost_ratio: 0.0,
        };
        let sig = gate.check(&ctx).await.unwrap();
        // Constitution guard fires before cost guard
        assert_eq!(sig.action, PhaseOverride::Abandon);
    }
}
