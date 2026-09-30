/// v10.2: NervousSystem — bridges brain (AgentLoop) and body (resources/costs/observer).
///
/// 现役职责（D-47 死代码清理后）：本 crate 现**只提供成本比率**（[`cost_ratio`]）。
/// 原「感知-决策链路」（`query()` / `NerveAction` / `PerceptionReport` 及其随附的
/// 资源快照采样、civ 告警累积）为死代码——全仓无 `nervous.query()` 生产调用，
/// 已随 D-47 删除。
///
/// D-46（2026-10-01，已接通）：成本侧已接地——`AgentLoop` 每步在构造
/// `GuardContext` 前，用 `cost_meter` 经 `PriceTable` 换算 USD 后调用
/// [`set_cost`](NervousSystem::set_cost)；预算由环境变量
/// `HEARTH_COST_BUDGET_USD`（>0）经 [`with_budget`](NervousSystem::with_budget)
/// 注入。若 cost_meter 中存在无价格条目的模型，则成本显式"不可用"（不更新、
/// [`cost_ratio`] 保持原值），调用方据此 warn。
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

    /// D-46（2026-10-01，已接通）：生产路径在 `AgentLoop` 构造时，若环境变量
    /// `HEARTH_COST_BUDGET_USD` 解析为 >0 的数值，则经本方法注入预算；
    /// 未设置 / 非法 / ≤0 ⇒ 无预算 ⇒ [`cost_ratio`](Self::cost_ratio) 返回 0.0
    /// ⇒ 该守卫不拦（既有语义，保持不变）。
    pub fn with_budget(mut self, budget_usd: f64) -> Self {
        self.cost_budget_usd = Some(budget_usd);
        self
    }

    /// v16.0: Normalized cost ratio (0.0~1.0) for the subconscious guard.
    /// Returns 0.0 when no budget is set (guard won't react).
    ///
    /// D-46（2026-10-01，已接通）：分子由 `AgentLoop` 每步经
    /// [`set_cost`](Self::set_cost) 从 `cost_meter`→`PriceTable` 同步真实 USD；
    /// 分母由 `HEARTH_COST_BUDGET_USD` 经 [`with_budget`](Self::with_budget) 注入。
    /// 无预算 → 返回 0.0（守卫不拦）；有预算但某模型无价格条目时，调用方显式
    /// 判定"成本不可用"并跳过同步（不会把未知当 0）。
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
