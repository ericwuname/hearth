//! D-213/D-214（2026-10-08, traecode）源码级门禁：`PhaseOverride::Simplify` 的**声称必须与事实一致**。
//!
//! **盲区（自报）**：仅断言源码文本**含注释**（本门禁目标正是注释里的"声称"）——不执行、不构造
//! 运行时会话。口径「宁可漏报不误报」。
//!
//! 沿革：D-213 曾如实化（旧文案"skip LLM entirely / Simplify goal"与事实不符——当时调用点
//! **只 `tracing::warn!`**，属空转）。D-214 **已接线**（简化步 = 关闭工具 + 降输出上限）
//! ⇒ 门禁随之改为断言**接线后的事实**，防止文案回退成"未接线/仅告警"或再虚报"跳过 LLM"。
//!
//! 行为级门禁见 `crates/agent-core/tests/simplify_override_wired_gate.rs`。

use std::fs;
use std::path::PathBuf;

#[test]
fn simplify_override_claim_matches_reality() {
    let sub = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let sub_raw = fs::read_to_string(&sub).expect("crates/subconscious/src/lib.rs 必须存在");

    // ① 仍**不得**声称"完全跳过 LLM"（`Simplify` 不跳 LLM——只有 `Abandon` 终止本步）。
    assert!(
        !sub_raw.contains("skip LLM entirely"),
        "D-213：`IntuitionSignal.action` 不得声称 'skip LLM entirely'——`Simplify` 不跳过 LLM"
    );
    // ② 仍**不得**回退成旧的"Simplify goal"（不改目标）。
    assert!(
        !sub_raw.contains("Simplify goal"),
        "D-213：`PhaseOverride::Simplify` 不得声称会简化目标（不改目标/计划）"
    );
    // ③ 不得回退成"未接线/仅告警"（D-214 已接线）。
    assert!(
        !sub_raw.contains("未接线") && !sub_raw.contains("仅告警"),
        "D-214：`Simplify` 已接线，文案不得回退成「未接线/仅告警」"
    );
    // ④ 须写明**接线后的可验证语义**（关闭工具 + 降输出上限）。
    assert!(
        sub_raw.contains("已接线") && sub_raw.contains("关闭工具"),
        "D-214：须写明接线语义——「关闭工具 + 降输出上限」"
    );

    // 第二落点：调用点（agent-core）同口径。
    let call = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../agent-core/src/loop.rs");
    let call_raw = fs::read_to_string(&call).expect("crates/agent-core/src/loop.rs 必须存在");
    assert!(
        !call_raw.contains("execution continues with simplified approach"),
        "D-213：调用点不得声称 'execution continues with simplified approach'（与实现不符的旧文案）"
    );
    assert!(
        call_raw.contains("simplify_step") && call_raw.contains("简化步"),
        "D-214：调用点须**消费** `Simplify`（`simplify_step` 标志 + 简化步语义说明）"
    );
}
