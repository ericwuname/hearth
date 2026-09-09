/// v10.2: NervousSystem — bridges brain (AgentLoop) and body (resources/costs/observer).
///
/// The nervous system is an intermediary that:
/// 1. **Collects** sensory data (resource snapshots, cost accumulation)
/// 2. **Perceives** abnormal states (memory high, disk low, cost over budget)
/// 3. **Decides** on actions per the Gene Constitution (Article 3: resources must be guarded)
/// 4. **Executes** actions (simplify plan, reduce steps, abandon)
/// 5. **Verifies** that the action had the intended effect
///
/// The AgentLoop calls `nervous.query()` at the top of each `do_reflect()` cycle.
use serde::Serialize;

/// Actions the nervous system can recommend in response to perceived threats.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum NerveAction {
    /// Everything is fine — proceed as normal.
    None,
    /// Simplify the current plan: reduce parallel steps, prefer simpler tools.
    Simplify,
    /// Hard-cap remaining steps to this number.
    ReduceSteps(u32),
    /// Abandon the current session — resources too constrained.
    Abandon,
    /// Deliver the best result so far and stop.
    DeliverAndQuit,
}

impl NerveAction {
    pub fn is_urgent(&self) -> bool {
        matches!(self, NerveAction::Abandon | NerveAction::DeliverAndQuit)
    }
}

/// Aggregated sensory report consumed by the AgentLoop.
#[derive(Debug, Clone, Serialize)]
pub struct PerceptionReport {
    pub timestamp: String,
    pub resource: resource_monitor::ResourceSnapshot,
    pub cost_accumulated_usd: f64,
    pub budget_remaining: Option<u64>,
    pub alerts: Vec<String>,
    pub action: NerveAction,
    pub is_critical: bool,
}

/// The nervous system — a stateless collector + decision engine.
pub struct NervousSystem {
    /// Rolling cost accumulator (injected from the gateway).
    cost_accumulated_usd: f64,
    /// Budget limit (optional — absent = no enforced budget).
    cost_budget_usd: Option<f64>,
    /// PID for snapshot collection.
    pid: u32,
    /// v10.2.1: Pending civ alerts since last drain.
    civ_pending: std::sync::Mutex<Vec<String>>,
    /// 测试隔离：注入固定快照（生产为 None——query 仍走系统采样）。
    snapshot_override: Option<resource_monitor::ResourceSnapshot>,
}

impl NervousSystem {
    pub fn new() -> Self {
        NervousSystem {
            cost_accumulated_usd: 0.0,
            cost_budget_usd: None,
            pid: std::process::id(),
            civ_pending: std::sync::Mutex::new(Vec::new()),
            snapshot_override: None,
        }
    }

    /// 测试隔离：注入固定快照，避免 memory/disk 系统状态干扰规则判定。
    pub fn with_snapshot(mut self, snap: resource_monitor::ResourceSnapshot) -> Self {
        self.snapshot_override = Some(snap);
        self
    }

    pub fn with_budget(mut self, budget_usd: f64) -> Self {
        self.cost_budget_usd = Some(budget_usd);
        self
    }

    /// v16.0: Normalized cost ratio (0.0~1.0) for the subconscious guard.
    /// Returns 0.0 when no budget is set (guard won't react).
    pub fn cost_ratio(&self) -> f32 {
        match self.cost_budget_usd {
            Some(b) if b > 0.0 => (self.cost_accumulated_usd / b).min(1.0) as f32,
            _ => 0.0,
        }
    }

    /// v10.2.1: Drain accumulated civ alerts (to be written to civilization line).
    pub fn drain_civ_alerts(&self) -> Vec<String> {
        std::mem::take(&mut self.civ_pending.lock().unwrap())
    }

    /// Inject accumulated cost from external accounting (e.g., llm-gateway CostMeter).
    pub fn set_cost(&mut self, usd: f64) {
        self.cost_accumulated_usd = usd;
    }

    /// Main entry point: collect sensory data, perceive threats, decide action.
    /// Called by AgentLoop at the start of each `do_reflect()` cycle.
    pub fn query(&self) -> PerceptionReport {
        let snap = self
            .snapshot_override
            .clone()
            .unwrap_or_else(|| resource_monitor::snapshot(self.pid));
        let mut alerts: Vec<String> = Vec::new();
        let mut action = NerveAction::None;

        // ─── Perception rules (Constitution Article 3) ───

        // Memory > 90% → immediate danger
        if snap.memory_percent > 90.0 {
            alerts.push(format!("memory critical: {:.0}%", snap.memory_percent));
            action = NerveAction::Abandon;
        }
        // Memory > 80% → degrade
        else if snap.is_critical() {
            alerts.push(format!("memory high: {:.0}%", snap.memory_percent));
            action = NerveAction::Simplify;
        }

        // Disk < 500 MB → emergency
        if snap.disk_free_gb < 0.5 {
            alerts.push(format!("disk critical: {:.2} GB free", snap.disk_free_gb));
            action = NerveAction::Abandon;
        } else if snap.disk_free_gb < 1.0 {
            alerts.push(format!("disk low: {:.2} GB free", snap.disk_free_gb));
            if action == NerveAction::None {
                action = NerveAction::Simplify;
            }
        }

        // Cost > 80% of budget → deliver early
        if let Some(budget) = self.cost_budget_usd {
            if self.cost_accumulated_usd > budget * 0.8 {
                alerts.push(format!(
                    "cost high: ${:.4} / ${:.4} ({}%)",
                    self.cost_accumulated_usd,
                    budget,
                    ((self.cost_accumulated_usd / budget) * 100.0) as u32
                ));
                if action == NerveAction::None {
                    action = NerveAction::DeliverAndQuit;
                }
            } else if self.cost_accumulated_usd > budget * 0.5 {
                alerts.push(format!(
                    "cost warning: ${:.4} / ${:.4}",
                    self.cost_accumulated_usd, budget
                ));
                if action == NerveAction::None {
                    action = NerveAction::Simplify;
                }
            }
        }

        let is_critical = action.is_urgent() || action == NerveAction::Abandon;

        // v10.2.1: Accumulate civ alerts for drain
        if !alerts.is_empty() {
            let entry = format!(
                "[NERVOUS] {} | alerts: {} | action: {:?}",
                chrono::Utc::now().to_rfc3339(),
                alerts.join(", "),
                action
            );
            self.civ_pending.lock().unwrap().push(entry);
        }

        PerceptionReport {
            timestamp: chrono::Utc::now().to_rfc3339(),
            resource: snap,
            cost_accumulated_usd: self.cost_accumulated_usd,
            budget_remaining: self
                .cost_budget_usd
                .map(|b| ((b - self.cost_accumulated_usd).max(0.0) * 1_000_000.0) as u64),
            alerts,
            action,
            is_critical,
        }
    }
}

impl Default for NervousSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nerve_action_is_urgent() {
        assert!(!NerveAction::None.is_urgent());
        assert!(!NerveAction::Simplify.is_urgent());
        assert!(NerveAction::Abandon.is_urgent());
        assert!(NerveAction::DeliverAndQuit.is_urgent());
    }

    #[test]
    fn test_query_returns_report() {
        let ns = NervousSystem::new();
        let report = ns.query();
        assert!(!report.timestamp.is_empty());
        assert!(report.resource.process_pid > 0);
    }

    #[test]
    fn test_query_with_budget_warning() {
        // 注入干净快照（内存/磁盘充足）——避免 VM 系统状态触发 memory/disk
        // 规则抢先（cost 规则只在 action==None 时生效，环境依赖会导致本测试假红）。
        let clean = resource_monitor::ResourceSnapshot {
            timestamp: String::new(),
            cpu_percent: 5.0,
            memory_mb: 1024.0,
            memory_percent: 20.0,
            disk_free_gb: 50.0,
            disk_total_gb: 100.0,
            process_pid: std::process::id(),
        };
        let mut ns = NervousSystem::new().with_budget(1.0).with_snapshot(clean);
        ns.set_cost(0.85); // 85% — should trigger cost high alert
        let report = ns.query();
        assert!(!report.alerts.is_empty());
        let has_cost_alert = report.alerts.iter().any(|a| a.contains("cost high"));
        assert!(
            has_cost_alert,
            "expected cost high alert, got: {:?}",
            report.alerts
        );
        assert!(report.is_critical);
    }

    #[test]
    fn test_drain_civ_alerts() {
        let mut ns = NervousSystem::new().with_budget(1.0);
        ns.set_cost(0.85);
        let _ = ns.query();
        let a = ns.drain_civ_alerts();
        assert!(!a.is_empty());
        assert!(a[0].contains("NERVOUS"));
        assert!(ns.drain_civ_alerts().is_empty());
    }
}
