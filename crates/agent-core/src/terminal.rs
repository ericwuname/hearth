//! G1 (v0.2.5): Task/Turn 终态规范化——可信委托的"完成态真相"。
//!
//! 痛点（任务书 §0.2 B）："模型不说话了"到底是完成/失败/暂停？此前 loop 层
//! 终止路径各自埋 reason（deadline_exceeded/budget_exhausted/verify_failed/...），
//! 但 CLI 投影层压成二值 completed/failed——用户看不到为什么结束。
//!
//! 规则（G1-01）：九态封闭集；`completed` 只能在最终验证成功后进入（WS13 已有
//! verify 门禁），禁止"LLM 说 Done = completed"。G1-02：所有异常退出必须映射到
//! 本集合之一，禁止 unknown/静默退出。
//!
//! 设计约束：不新增事件类型（事件契约只增不改）；loop 层继续在 summary.reason
//! 携带细粒度原因，本模块负责 reason → 九态的**唯一映射权威**，CLI/报告/Observer
//! 统一从一处取终态，禁止各自硬编码。

/// G1-01: 任务/Turn 终态封闭集（九态）。
pub const TERMINAL_STATES: &[&str] = &[
    "starting",
    "running",
    "waiting_for_user",
    "paused",
    "completed",
    "failed",
    "aborted",
    "cancelled",
    "deadline_exceeded",
];

/// G1: 状态是否为合法终态（封闭集校验——报告/事件写入口用）。
pub fn is_terminal_state(s: &str) -> bool {
    TERMINAL_STATES.contains(&s)
}

/// G1-02: reason → 终态唯一映射。
///
/// - ok=true → completed（WS13 verify 门禁在前置环节已把关，能走到这里
///   且 ok=true 的路径均已经过产物校验/all_done 门禁）
/// - deadline_exceeded → deadline_exceeded（任务书 G1-01 独立终态）
/// - cancelled → cancelled（用户主动取消——非失败）
/// - agent_crashed / panic → aborted（异常中止：进程崩溃隔离）
/// - approval_denied_noninteractive → failed（RC24-B：非交互审批拒绝——
///   可行动失败，reason 经 status_detail 投影 ✗ + 提示）
/// - budget_exhausted → paused（R6-6：交还控制权+当前状态+建议，非 failed）
/// - 其余（verify_failed / error / provider 失败 /
///   未知 reason）→ failed（detail 保留在报告 status_detail）
pub fn normalize_terminal_state(ok: bool, reason: &str) -> &'static str {
    if ok {
        return "completed";
    }
    match reason {
        "deadline_exceeded" => "deadline_exceeded",
        "cancelled" => "cancelled",
        "agent_crashed" => "aborted",
        // RC24-B: 非交互审批拒绝——终态仍属 failed（九态封闭集不变），
        // 显式映射仅为文档化语义。
        "approval_denied_noninteractive" => "failed",
        // R6-6（判定权归还长程任务书 v1.0）：预算耗尽 = 交还控制权 + 当前状态
        // + 建议——不是失败（任务没做完 ≠ 做失败；护栏触发 ≠ 判定失败）。
        // 终态 paused（九态封闭集既有态），报告携带 handover（做到哪了+建议）。
        "budget_exhausted" => "paused",
        _ => "failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// G1-04 负面测试基座：任意 (ok, reason) 组合的终态必须 ∈ 封闭集，
    /// 禁止 unknown/静默退出（G1-02）。
    #[test]
    fn test_normalize_always_produces_valid_terminal_state() {
        let reasons = [
            "budget_exhausted",
            "deadline_exceeded",
            "verify_failed",
            "error",
            "cancelled",
            "agent_crashed",
            "approval_denied_noninteractive",
            "provider failure: HTTP 429",
            "完全未知的怪异 reason",
            "",
        ];
        for ok in [true, false] {
            for r in reasons {
                let s = normalize_terminal_state(ok, r);
                assert!(
                    is_terminal_state(s),
                    "reason={r:?} ok={ok} → {s} 不在封闭集"
                );
            }
        }
    }

    /// G1-01: 具体映射语义。
    #[test]
    fn test_normalize_semantics() {
        // 成功 → completed（唯一入口）
        assert_eq!(normalize_terminal_state(true, ""), "completed");
        assert_eq!(normalize_terminal_state(true, "error"), "completed");
        // deadline 独立终态（任务书 G1-01 明列）
        assert_eq!(
            normalize_terminal_state(false, "deadline_exceeded"),
            "deadline_exceeded"
        );
        // 用户取消 ≠ 失败
        assert_eq!(normalize_terminal_state(false, "cancelled"), "cancelled");
        // 崩溃 → aborted（异常中止，区别于任务失败）
        assert_eq!(normalize_terminal_state(false, "agent_crashed"), "aborted");
        // RC24-B: 非交互审批拒绝 → failed（reason 经 status_detail 投影）
        assert_eq!(
            normalize_terminal_state(false, "approval_denied_noninteractive"),
            "failed"
        );
        // R6-6: 预算耗尽 = 交还控制权（paused，报告携带 handover）——非失败
        assert_eq!(
            normalize_terminal_state(false, "budget_exhausted"),
            "paused"
        );
        // 验证失败/未知 → failed
        assert_eq!(normalize_terminal_state(false, "verify_failed"), "failed");
        assert_eq!(normalize_terminal_state(false, "who knows"), "failed");
    }

    /// 封闭集无重复、含任务书 G1-01 九态。
    #[test]
    fn test_terminal_states_closed_set() {
        assert_eq!(TERMINAL_STATES.len(), 9);
        let mut sorted: Vec<&str> = TERMINAL_STATES.to_vec();
        sorted.sort_unstable();
        let before = sorted.clone();
        sorted.dedup();
        assert_eq!(before, sorted, "封闭集不得有重复项");
    }
}

/// P1-EXECUTION-DECISION-01 Node 02: Completion Readiness——**纯函数**派生视图
/// （约束 2：平台无关、无副作用、禁 LLM 调用、禁文件读取——只组合既有结构化
/// 事实，不是新事实源）。输入全部为快照值：
/// - has_criteria: acceptance criteria 非空
/// - verification: "none"|"pending"|"passed"|"failed"（修 C 反折叠后三态完整）
/// - has_artifacts: written_files 非空（事实层）
/// - consecutive_errors: 事实层错误计数
/// - all_nodes_done: TaskGraph 全 Completed（或 QA 无图=true）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionReadiness {
    /// 无完成事实（继续执行）
    NotReady,
    /// 有未核验的验收要求（REQUIRES_VERIFICATION——先验后判）
    RequiresVerification,
    /// 验收已通过 + 产物在场 → "任务已具备结束条件"（Case D/F 前提）
    ReadyForCompletion,
    /// 事实与判定互相矛盾（conflict 观察）
    Conflicted,
}

pub fn completion_readiness(
    has_criteria: bool,
    verification: &str,
    has_artifacts: bool,
    consecutive_errors: u32,
    all_nodes_done: bool,
) -> CompletionReadiness {
    use CompletionReadiness::*;
    // CONFLICTED：验证明说 failed 却声称有产物通过——事实矛盾优先暴露
    if verification == "failed" && has_artifacts && consecutive_errors == 0 {
        return Conflicted;
    }
    match (has_criteria, verification) {
        // 无 criteria：回退到产物/图双事实（保守——不宣称 ready）
        (false, _) | (_, "none") => {
            if all_nodes_done && has_artifacts && consecutive_errors == 0 {
                ReadyForCompletion
            } else if has_artifacts && consecutive_errors == 0 {
                RequiresVerification
            } else {
                NotReady
            }
        }
        (_, "pending") => {
            if consecutive_errors == 0 {
                RequiresVerification
            } else {
                NotReady
            }
        }
        (_, "passed") => {
            if has_artifacts && consecutive_errors == 0 {
                ReadyForCompletion
            } else {
                Conflicted
            }
        }
        (_, "failed") => NotReady,
        // 未知 verification 值（防御性——不冒充任何完成态）
        _ => NotReady,
    }
}

#[cfg(test)]
mod execdec_node02_tests {
    use super::*;

    /// Node 02（修 E 反例用例随附）：全组合关键路径——
    /// 反例 R1：passed 但零产物 → Conflicted（冒充 passed 被抓）。
    /// 反例 R2：criteria 非空 + pending → 不得 Ready（REQUIRES_VERIFICATION）。
    #[test]
    fn test_completion_readiness_matrix() {
        use CompletionReadiness::*;
        // Case D 前提：criteria+passed+产物 → READY
        assert_eq!(
            completion_readiness(true, "passed", true, 0, true),
            ReadyForCompletion
        );
        // Case B/C 前提：pending → REQUIRES_VERIFICATION
        assert_eq!(
            completion_readiness(true, "pending", true, 0, false),
            RequiresVerification
        );
        // 反例 R1：passed 但无产物 → Conflicted
        assert_eq!(
            completion_readiness(true, "passed", false, 0, true),
            Conflicted
        );
        // failed + 产物在场 + 零错误 → Conflicted（事实矛盾——44 步样本形态；
        // 决策层仍会按 failed 走 verify_replan/verify_failed 通道，Conflicted
        // 只是观察面板暴露矛盾）
        assert_eq!(
            completion_readiness(true, "failed", true, 0, true),
            Conflicted
        );
        // failed + 无产物 → NotReady（等 verify_replan 通道）
        assert_eq!(
            completion_readiness(true, "failed", false, 0, false),
            NotReady
        );
        // 无 criteria + 图全完 + 产物 → READY（QA 无图路径）
        assert_eq!(
            completion_readiness(false, "none", true, 0, true),
            ReadyForCompletion
        );
        // 无产物无验证 → NotReady
        assert_eq!(
            completion_readiness(false, "none", false, 0, true),
            NotReady
        );
        // RC44 反折叠语义：failed 不再洗白 pending（组合层可见）
        assert_ne!(
            completion_readiness(true, "failed", true, 2, false),
            RequiresVerification
        );
    }
}

/// P1-FAILURE-ADAPTATION-01 Node 02: Failure Taxonomy——在上轮七分类基础上
/// 扩展为十类（F1-F10），仍为纯确定性函数（结构化输入，禁错误文本关键词猜测
/// ——INV-H/RC20 纪律；STOP-9：不引入 LLM judge，全部判据可审计）。
/// 映射：F1=Transient F2=ToolFailure F3=EnvironmentFailure F4=PermissionFailure
/// F5=AssertionFailure F6=VerificationFailure F7=PlanFailure F8=ResourceFailure
/// F9=ModelJudgmentFailure F10=Unknown。
/// 新增输入（均为既有结构化事实，无新事实源）：
/// - approval_denied: 审批门拒绝/委托审计拒绝（RC24 审批通道既有事实）
/// - budget_exhausted: budget_remaining==0 或 Reserve 耗尽（budget 事实派生）
/// Unknown 判据：exit_code=None 且无任何其他结构化信号——**不得强行塞入
/// ToolFailure**（F10 必须真实存在）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    Transient,
    ToolFailure,
    EnvironmentFailure,
    PermissionFailure,
    AssertionFailure,
    PlanFailure,
    VerificationFailure,
    ResourceFailure,
    ModelJudgmentFailure,
    Unknown,
}

// 纯分类器聚合 8 个相互独立的结构化事实输入——重构为 struct 反而制造
// 第二套参数打包约定；allow 为有意裁决（判据见 Node 02 注释）。
#[allow(clippy::too_many_arguments)]
pub fn classify_failure(
    is_timeout: bool,
    is_network: bool,
    same_tool_repeat: u32,
    exit_code: Option<i32>,
    was_verification_step: bool,
    llm_self_reported_ok: bool,
    approval_denied: bool,
    budget_exhausted: bool,
) -> FailureKind {
    use FailureKind::*;
    if is_timeout {
        return EnvironmentFailure; // deadline/timeout = 环境时间预算，非工具缺陷
    }
    if is_network {
        return Transient; // 网络抖动可重试
    }
    if approval_denied {
        return PermissionFailure; // 审批拒绝：重试同调用毫无意义（F4）
    }
    if budget_exhausted {
        return ResourceFailure; // budget/deadline 资源耗尽（F8）
    }
    if was_verification_step && llm_self_reported_ok {
        return ModelJudgmentFailure; // 自述成功 vs 验证失败（O-4 家族）
    }
    if was_verification_step {
        return VerificationFailure;
    }
    if same_tool_repeat >= 2 {
        return PlanFailure; // 同工具反复失败 = 计划层面没换路
    }
    if matches!(exit_code, Some(c) if c > 0) {
        return AssertionFailure; // 命令执行了但断言/测试失败（正 exit code）
    }
    if matches!(exit_code, Some(c) if c < 0) {
        return ToolFailure; // 负值=信号终止（seccomp/SIGKILL 等）
    }
    Unknown // exit_code=None 且无其他信号——无结构化证据，不冒充任何已知类（F10）
}

/// P1-FAILURE-ADAPTATION-01 Node 06: Recovery Strategy——与 FailureKind 的
/// 映射为**纯函数**（策略矩阵 docs/failure-strategy-matrix.md 的代码化）。
/// 六策略：Retry / Repair / Replan / Verify / Escalate / Stop。
/// 边界（单测锁定）：Retry 仅 Transient 一类；VerificationFailure → Replan
/// ≠ Retry（INV-FA01-F）；ResourceFailure → Stop（不得免费续命，INV-FA01-E）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// 同任务同策略重试（仅 transient：429/5xx/网络抖动）
    Retry,
    /// 失败提供了可修复信息（编译错/断言输出）→ 修复后复测
    Repair,
    /// 当前计划不适用 → 重分解（既有 Replan 通道，cap 3）
    Replan,
    /// 先确定性核验再相信任何判断（model_judgment 冲突，O-4 家族）
    Verify,
    /// 升级：审批拒绝/环境问题/无结构化证据——需要人或显式决策
    Escalate,
    /// 资源耗尽 → 有界停止（Reserve/预算硬上限兜底）
    Stop,
}

pub fn failure_strategy(kind: FailureKind) -> RecoveryStrategy {
    use FailureKind as F;
    use RecoveryStrategy as S;
    match kind {
        F::Transient => S::Retry,
        F::ToolFailure => S::Repair,
        F::EnvironmentFailure => S::Escalate,
        F::PermissionFailure => S::Escalate,
        F::AssertionFailure => S::Repair,
        F::PlanFailure => S::Replan,
        F::VerificationFailure => S::Replan,
        F::ResourceFailure => S::Stop,
        F::ModelJudgmentFailure => S::Verify,
        F::Unknown => S::Escalate,
    }
}

/// R5-1（智能性根治长程任务包 v1.0）：策略 → 决策原则建议（纯函数）。
/// 输入 = scratch `last_recovery_strategy` 的 Debug 格式 label（如 "Replan"）；
/// 输出 = 给决策者（LLM）的一句话行动原则。语义与 `RecoveryStrategy` 枚举
/// 文档一一对应（Retry/Repair/Replan/Verify/Escalate/Stop），未知 label 归
/// 兜底（不冒充已知策略）。
pub fn strategy_suggestion(label: &str) -> &'static str {
    match label {
        "Retry" => "transient failure: same approach may work on retry, but re-check inputs first",
        "Repair" => "the error output contains fixable information: read it, fix, then re-verify",
        "Replan" => "the current approach is falsified by facts: change method (different tool / path / decomposition) instead of repeating it",
        "Verify" => "do not trust self-reported success: verify deterministically (run the check) before acting on it",
        "Escalate" => "this needs user input or an explicit decision: ask or stop rather than blind-retry",
        "Stop" => "budget/resources are exhausted: wrap up and report status honestly",
        _ => "failure class unknown: re-examine the error output before acting again",
    }
}

#[cfg(test)]
mod execdec_node06_tests {
    use super::*;

    /// 便捷全参封装：默认无审批拒绝/无资源耗尽。
    fn cf(
        is_timeout: bool,
        is_network: bool,
        same_tool_repeat: u32,
        exit_code: Option<i32>,
        was_verification_step: bool,
        llm_self_reported_ok: bool,
    ) -> FailureKind {
        classify_failure(
            is_timeout,
            is_network,
            same_tool_repeat,
            exit_code,
            was_verification_step,
            llm_self_reported_ok,
            false,
            false,
        )
    }

    #[test]
    fn test_failure_classification() {
        use FailureKind::*;
        assert_eq!(cf(true, false, 0, None, false, false), EnvironmentFailure);
        assert_eq!(cf(false, true, 0, None, false, false), Transient);
        assert_eq!(
            cf(false, false, 0, Some(0), true, true),
            ModelJudgmentFailure
        );
        assert_eq!(
            cf(false, false, 0, Some(1), true, false),
            VerificationFailure
        );
        assert_eq!(cf(false, false, 3, Some(1), false, false), PlanFailure);
        assert_eq!(
            cf(false, false, 0, Some(101), false, false),
            AssertionFailure
        );
        assert_eq!(cf(false, false, 0, Some(-1), false, false), ToolFailure);
    }

    /// P1-FAILURE-ADAPTATION-01 Node 02：F1-F10 十类各 ≥1 正例 + 相邻边界
    /// 反例 ≥2 组 + Unknown 真实存在（不强行塞入已知类）。
    #[test]
    fn test_failure_taxonomy_f1_to_f10() {
        use FailureKind::*;
        // F1 transient_provider：网络抖动（正例）
        assert_eq!(cf(false, true, 0, None, false, false), Transient);
        // F2 tool_execution：负 exit code = 信号终止（正例）
        assert_eq!(cf(false, false, 0, Some(-9), false, false), ToolFailure);
        // F3 environment：timeout（正例）
        assert_eq!(cf(true, false, 0, None, false, false), EnvironmentFailure);
        // F4 permission/approval（正例）
        assert_eq!(
            classify_failure(false, false, 0, Some(1), false, false, true, false),
            PermissionFailure
        );
        // F5 assertion_or_test：正 exit code（正例）
        assert_eq!(cf(false, false, 0, Some(1), false, false), AssertionFailure);
        // F6 verification（正例）
        assert_eq!(
            cf(false, false, 0, Some(1), true, false),
            VerificationFailure
        );
        // F7 plan/strategy：同工具反复（正例）
        assert_eq!(cf(false, false, 2, Some(1), false, false), PlanFailure);
        // F8 resource/budget（正例）
        assert_eq!(
            classify_failure(false, false, 0, None, false, false, false, true),
            ResourceFailure
        );
        // F9 model_judgment（正例）
        assert_eq!(
            cf(false, false, 0, Some(0), true, true),
            ModelJudgmentFailure
        );
        // F10 unknown：exit_code=None 且无任何结构化信号——真实存在
        assert_eq!(cf(false, false, 0, None, false, false), Unknown);

        // ── 边界反例组 1（F1 vs F3）：timeout 优先于 network —— 环境时间预算
        // 比可重试抖动更权威（deadline 权威性，INV-FA01-D 同源）。
        assert_eq!(cf(true, true, 0, None, false, false), EnvironmentFailure);
        // ── 边界反例组 2（F4 vs F5）：审批拒绝即使伴随正 exit code（命令"跑完"了）
        // 也不得归为断言失败——重试/修复都无意义，必须走 escalate。
        assert_eq!(
            classify_failure(false, false, 0, Some(1), false, false, true, false),
            PermissionFailure
        );
        // ── 边界反例组 3（F8 vs F7）：budget 耗尽优先于同工具重复——资源耗尽时
        // 不再谈策略，直接 Stop（INV-FA01-E）。
        assert_eq!(
            classify_failure(false, false, 3, Some(1), false, false, false, true),
            ResourceFailure
        );
        // ── 边界反例组 4（F9 vs F6）：验证步 + 自述成功 → 冲突类优先——
        // 事实矛盾比单纯验证失败信息量更大（O-4 家族）。
        assert_eq!(
            cf(false, false, 0, Some(1), true, true),
            ModelJudgmentFailure
        );
        // ── 反例：same_tool_repeat=1（未达 2）+ 正 exit → 仍是断言失败，不拔高为 PlanFailure。
        assert_eq!(cf(false, false, 1, Some(1), false, false), AssertionFailure);
    }

    /// Node 06：Retry ≠ Repair ≠ Replan ≠ GiveUp 边界锁定（策略矩阵代码化）。
    /// 反例导向：任何把非 transient 类映射到 Retry 的回归都会在此失败。
    #[test]
    fn test_failure_strategy_matrix_boundaries() {
        use FailureKind as F;
        use RecoveryStrategy as S;
        // Retry 仅 transient 一类（INV-FA01-A：Failure ≠ Retry）
        assert_eq!(failure_strategy(F::Transient), S::Retry);
        assert_ne!(failure_strategy(F::AssertionFailure), S::Retry);
        assert_ne!(failure_strategy(F::VerificationFailure), S::Retry);
        assert_ne!(failure_strategy(F::ToolFailure), S::Retry);
        assert_ne!(failure_strategy(F::Unknown), S::Retry);
        // 修复通道：断言/测试失败与工具执行失败 → Repair（有可修复信息）
        assert_eq!(failure_strategy(F::AssertionFailure), S::Repair);
        assert_eq!(failure_strategy(F::ToolFailure), S::Repair);
        // 计划通道：验证失败与计划失败 → Replan（INV-FA01-F：verification failure ≠ arbitrary retry）
        assert_eq!(failure_strategy(F::VerificationFailure), S::Replan);
        assert_eq!(failure_strategy(F::PlanFailure), S::Replan);
        // 冲突通道：自述 vs 事实 → 先核验（LLM self-report ≠ evidence，INV-FA01-G）
        assert_eq!(failure_strategy(F::ModelJudgmentFailure), S::Verify);
        // 升级通道：审批/环境/未知
        assert_eq!(failure_strategy(F::PermissionFailure), S::Escalate);
        assert_eq!(failure_strategy(F::EnvironmentFailure), S::Escalate);
        assert_eq!(failure_strategy(F::Unknown), S::Escalate);
        // 停止通道：资源耗尽（INV-FA01-E：reserve 不得变成无限免费预算）
        assert_eq!(failure_strategy(F::ResourceFailure), S::Stop);
        // Replan ≠ GiveUp：Replan 策略的终点不是直接放弃——GiveUp 只能由
        // 资源耗尽（Stop）或既有 planner caps 触发，映射层不含 GiveUp 语义。
        assert_ne!(failure_strategy(F::VerificationFailure), S::Stop);
        assert_ne!(failure_strategy(F::PlanFailure), S::Stop);
    }

    /// R5-1: 策略建议纯函数——六策略全覆盖 + 未知 label 兜底（不冒充已知策略）。
    #[test]
    fn test_strategy_suggestion_labels() {
        // 六个合法 label 各有非兜底建议
        for (label, must_contain) in [
            ("Retry", "retry"),
            ("Repair", "fix"),
            ("Replan", "change method"),
            ("Verify", "verify"),
            ("Escalate", "user input"),
            ("Stop", "wrap up"),
        ] {
            let s = strategy_suggestion(label);
            assert!(
                s.contains(must_contain),
                "strategy {label} suggestion should mention '{must_contain}', got: {s}"
            );
        }
        // 未知 label → 兜底（显式 unknown 提示，不得错挂已知策略语义）
        let fallback = strategy_suggestion("Nonsense");
        assert!(
            fallback.contains("unknown"),
            "fallback should say unknown, got: {fallback}"
        );
        assert_ne!(fallback, strategy_suggestion("Replan"));
    }
}
