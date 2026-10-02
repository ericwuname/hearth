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
//!
//! D-114（2026-10-02, traecode）：本文件原有三段"决策支持层"纯函数
//! （`classify_failure`/`failure_strategy`/`strategy_suggestion` 与
//! `completion_readiness`，P1-FAILURE-ADAPTATION-01 / P1-EXECUTION-DECISION-01 产物）
//! **已整段退役删除**——全仓**零消费者**（仅自测调用），且其唯一下游通道
//! （scratch 键 `last_failure_class`/`last_recovery_strategy` 的"R5-1 中转注入块"）
//! 早于 R6-5 依"判定权归还范式"主动删除（loop.rs 原位注释：失败事实直接附着工具
//! 结果消息，"勿复活旧通道"）⇒ 生产者结构性消失、无从接线，处置同 D-66/D-83/D-85
//! （退役而非复活）。**保留** `normalize_terminal_state`（现役，loop/CLI 多处调用）
//! 与 `TERMINAL_STATES`/`is_terminal_state`（G1 九态封闭集唯一事实源，由下方
//! 负例矩阵单测锁定）。

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

/// G1: 状态是否为合法终态（封闭集校验）。
///
/// D-114 订正：原 doc 称"报告/事件写入口用"——实测**零生产调用方**（全仓仅本文件
/// 的负例矩阵单测引用）。它现役的真实角色是**九态封闭集的判定权威**：单测用它把
/// `normalize_terminal_state` 的任意 (ok, reason) 输出收进封闭集，防止 unknown/静默
/// 退出（G1-02）。此处据实订正，不静默留一个"看起来被写入口使用"的假象。
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
        // S8（手术包二）：统一暂停语义——provider 类失败（key 失效/网络/窗口
        // 耗尽）**可恢复**：修 key/网络后 `hearth resume` 续跑，不再归 failed。
        // 任务判定类失败（verify_failed 等）仍走下方 failed 兜底。
        "provider_error" => "paused",
        "provider_retry_window_exhausted" => "paused",
        // S11（手术包二）：Ctrl-C 打断 = 上下文保留（可继续/可 resume），
        // 同属 paused 语义——不是失败，也不是取消（取消 = cancelled 终态）。
        "interrupted" => "paused",
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
        // S11（手术包二）：Ctrl-C 打断 = 上下文保留（paused，非 failed/cancelled）
        assert_eq!(normalize_terminal_state(false, "interrupted"), "paused");
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
