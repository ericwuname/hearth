//! D-219（2026-10-08, traecode）源码级门禁：observer ④ 资源效率指标的**数据源必须真实接线**。
//!
//! **盲区（自报）**：仅断言源码/配置文本——不执行、不跑真实会话。口径「宁可漏报不误报」。
//!
//! 沿革：本门禁原为 D-218（断言"如实标注**无数据源、恒 0、规则永不触发**"）——当时属实：
//! `Done` 事件 payload 从不含 `usage`，且读的键 `cost`/`tokens` 在真实 `CostEntry` 里不存在。
//! D-219 **已接线**（内核 `Done` 统一携带 `usage` + 可空 `cost_usd`）⇒ 门禁随之改为断言
//! **接线后的事实**，防回退成"无数据源"或再退回自造 payload 自证。
//!
//! 行为级验证：`crates/service/tests/observer_resource_metrics_wired_gate.rs`（真实会话跑出的
//! report.json 必须含非零 `token_efficiency`）+ `observer::metrics` 的 D-219 单测。

use std::fs;
use std::path::PathBuf;

fn read(rel: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&p).unwrap_or_else(|_| panic!("crates/observer/{rel} 必须存在"))
}

#[test]
fn resource_metrics_are_wired_to_a_real_source() {
    let metrics = read("src/metrics.rs");
    // ① 须写明**已接线**（不得回退成"无数据源"的旧态——D-218 那段沿革可保留，但现状必须写清）。
    assert!(
        metrics.contains("D-219") && metrics.contains("已接线"),
        "D-219：`metrics.rs` 须写明 ④ 资源效率指标**已接线**（数据源 = Done payload 的 usage/cost_usd）"
    );
    // ② 须写明**真实数据源形状**（内核 Done payload 的 usage / cost_usd）。
    assert!(
        metrics.contains("cost_usd") && metrics.contains("total_tokens"),
        "D-219：`metrics.rs` 须写明取自 Done payload 的 `usage.total_tokens` 与可空 `cost_usd`"
    );
    // ③ 须写明**未知语义**（None ≠ 0，不参与阈值判定）。
    assert!(
        metrics.contains("未知"),
        "D-219：须写明未知（None）语义——不得用 0 冒充"
    );
}

#[test]
fn rule_config_and_tests_use_the_real_kernel_shape() {
    let rules_toml = read("resources/observer-rules.toml");
    assert!(
        rules_toml.contains("D-219"),
        "D-219：`observer-rules.toml` 的 `cost_per_step_high` 须注明「已接线、生产可达」及未知不判"
    );
    // 规则/指标测试须用**内核真实形状**（`total_tokens`/`cost_usd`），
    // 不得再用 `usage:{cost,tokens}` 自造 payload 自证（D-218 假绿成因）。
    let rules_rs = read("src/rules.rs");
    assert!(
        rules_rs.contains("cost_usd") && rules_rs.contains("total_tokens"),
        "D-219：`rules.rs` 的规则测试须用内核真实形状（`total_tokens`/`cost_usd`），不得用自造 payload"
    );
}
