/// v10.2: NervousSystem — bridges brain (AgentLoop) and body (resources/costs/observer).
///
/// 现役职责（D-47 死代码清理后）：本 crate 现**只提供成本比率**（[`cost_ratio`]）。
/// 原「感知-决策链路」（`query()` / `NerveAction` / `PerceptionReport` 及其随附的
/// 资源快照采样、civ 告警累积）为死代码——全仓无 `nervous.query()` 生产调用，
/// 已随 D-47 删除。
///
/// ⚠️ 成本侧同样未接地：`cost_budget_usd` 只有 [`NervousSystem::with_budget`] 会设置，
/// 而生产路径从未调用它（`AgentLoop` 用 `NervousSystem::new()`）→ [`cost_ratio`]
/// 恒返回 0.0，`CostGuard` 在生产中永不触发。是否接通真实成本核算＝产品方向抉择，见 D-46。
pub struct NervousSystem {
    /// Rolling cost accumulator (injected from the gateway).
    cost_accumulated_usd: f64,
    /// Budget limit (optional — absent = no enforced budget).
    cost_budget_usd: Option<f64>,
}

impl NervousSystem {
    pub fn new() -> Self {
        NervousSystem {
            cost_accumulated_usd: 0.0,
            cost_budget_usd: None,
        }
    }

    /// ⚠️ 仅测试调用（P1-12 核验，2026-10-01）：全仓生产路径从未设置预算
    /// ⇒ [`cost_ratio`](Self::cost_ratio) 恒 0 ⇒ `CostGuard` 永不触发。见 D-46。
    pub fn with_budget(mut self, budget_usd: f64) -> Self {
        self.cost_budget_usd = Some(budget_usd);
        self
    }

    /// v16.0: Normalized cost ratio (0.0~1.0) for the subconscious guard.
    /// Returns 0.0 when no budget is set (guard won't react).
    ///
    /// ⚠️ 生产实际值（P1-12 核验，2026-10-01）：`cost_budget_usd` 只有
    /// [`with_budget`](Self::with_budget) 会设置，而**生产路径从未调用它**
    /// （`AgentLoop` 用 `NervousSystem::new()`，agent-core/loop.rs:1627）→ 本函数
    /// **恒返回 0.0**；累加侧 `AgentLoop::update_cost()`（loop.rs:1098）也**零调用者**，
    /// 即便设了预算分子仍恒 0。⇒ subconscious 的 `CostGuard`
    /// （`cost_ratio > 0.80 / > 0.95`）在生产中**永不触发**。
    /// 是否接通真实成本核算（需定价口径）＝产品方向抉择，见 D-46。
    pub fn cost_ratio(&self) -> f32 {
        match self.cost_budget_usd {
            Some(b) if b > 0.0 => (self.cost_accumulated_usd / b).min(1.0) as f32,
            _ => 0.0,
        }
    }

    /// Inject accumulated cost from external accounting (e.g., llm-gateway CostMeter).
    pub fn set_cost(&mut self, usd: f64) {
        self.cost_accumulated_usd = usd;
    }
}

impl Default for NervousSystem {
    fn default() -> Self {
        Self::new()
    }
}
