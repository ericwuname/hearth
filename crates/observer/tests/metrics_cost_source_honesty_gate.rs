//! D-218（2026-10-08, traecode）源码级门禁：observer 资源效率指标/规则必须**如实标注无数据源**。
//!
//! **盲区（自报）**：仅断言源码文本**含注释**（本门禁目标正是注释里的"如实化说明"）——
//! 不执行、不跑真实会话。口径「宁可漏报不误报」。
//!
//! 病灶：`metrics` 的 ④ 资源效率 `cost_per_step` / `token_efficiency` 声称取自
//! `Done.report.usage`，但——① `Done` **事件** payload **从不含 `usage`**（`usage` 只在返回的
//! `RunReport` 里）；② 本模块读的键 `cost`/`tokens` 在真实 `CostEntry` 中**不存在**
//! （真实键 `prompt_tokens`/`completion_tokens`/`total_tokens`/`calls`）；③ `cost` 按设计
//! **不在事件流**（Observer 禁读 CostMeter）⇒ 二者**无任何可达数据源**，生产中**恒 0**；
//! 依赖它的规则 `cost_per_step_high` **永不触发**。而单测/规则测试用**自造 payload**
//! （`usage:{cost,tokens}`）"证明"它能触发 ⇒ **假绿**（D-207 同族：只用手造数据自证）。

use std::fs;
use std::path::PathBuf;

fn read(rel: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&p).unwrap_or_else(|_| panic!("crates/observer/{rel} 必须存在"))
}

#[test]
fn resource_metrics_are_honestly_marked_source_less() {
    let metrics = read("src/metrics.rs");
    assert!(
        metrics.contains("无数据源") && metrics.contains("D-218"),
        "D-218：`metrics.rs` 须如实标注 ④ 资源效率指标**无数据源（恒 0）**——不得仍宣称其取自 Done.report.usage 而不提该 payload 从不含 usage"
    );
    assert!(
        metrics.contains("永不触发"),
        "D-218：须写明依赖它的规则 `cost_per_step_high` 在生产中**永不触发**"
    );
}

#[test]
fn dead_rule_is_annotated_and_fabricated_payloads_marked() {
    // 规则表：死规则须带如实注释。
    let rules_toml = read("resources/observer-rules.toml");
    assert!(
        rules_toml.contains("D-218"),
        "D-218：`observer-rules.toml` 的 `cost_per_step_high` 须如实注释（该规则生产永不触发）"
    );
    // 规则/指标测试：自造 payload 必须显式标注为"合成/自造"（非生产形状），否则会被读成"规则可用"的证据。
    let rules_rs = read("src/rules.rs");
    assert!(
        rules_rs.contains("自造") || rules_rs.contains("合成"),
        "D-218：`rules.rs` 里用 `usage:{{cost,tokens}}` 自造 payload 的测试须显式标注为**合成输入**（生产不产出该形状）"
    );
}
