//! D-213（2026-10-08, traecode）源码级门禁：`PhaseOverride::Simplify` 的**声称必须与事实一致**。
//!
//! **盲区（自报）**：仅断言源码文本**含注释**（本门禁目标正是注释里的"声称"）——不执行、不构造
//! 运行时会话。口径「宁可漏报不误报」。
//!
//! 病灶：`subconscious` 声明 `Simplify`＝"Simplify goal — resource pressure"、`IntuitionSignal.action`
//! 声明"Override the next loop phase — **skip LLM entirely**"；`agent-core` 的调用点注释亦称
//! "execution continues with simplified approach"。**事实**：`CostGuard` 触发 `Simplify` 后，调用点
//! **只 `tracing::warn!`**，不跳过 LLM、不简化目标 ⇒ 该 override 是**空转**（"声称≠实现/半接线"族）。
//!
//! 处置（遵"已设计未接线不得擅自删除/接线"）：**如实化**两处文案 + 另立接线债 D-214。

use std::fs;
use std::path::PathBuf;

#[test]
fn simplify_override_claim_is_honest() {
    let sub = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let sub_raw = fs::read_to_string(&sub).expect("crates/subconscious/src/lib.rs 必须存在");

    assert!(
        !sub_raw.contains("skip LLM entirely"),
        "D-213：`IntuitionSignal.action` 不得声称 'skip LLM entirely'——`Simplify` 并不跳过 LLM（只 warn）"
    );
    assert!(
        !sub_raw.contains("Simplify goal"),
        "D-213：`PhaseOverride::Simplify` 不得声称会简化目标——当前调用点只 `tracing::warn!`"
    );
    assert!(
        sub_raw.contains("未接线") || sub_raw.contains("仅告警"),
        "D-213：须如实标注 `Simplify` 当前**仅告警 / 未接线**"
    );

    // 第二落点：调用点（agent-core）不得声称"继续以简化方式执行"。
    let call = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../agent-core/src/loop.rs");
    let call_raw = fs::read_to_string(&call).expect("crates/agent-core/src/loop.rs 必须存在");
    assert!(
        !call_raw.contains("execution continues with simplified approach"),
        "D-213：调用点不得声称 'execution continues with simplified approach'——事实是**只 warn、照常继续**"
    );
}
