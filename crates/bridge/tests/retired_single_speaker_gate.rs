//! 门禁：**`bridge::BridgeStrategy::SingleSpeaker` 已退役**（D-181，2026-10-08, traecode）。
//!
//! 背景：该变体**全仓零构造方**——HTTP 层（`routes::create_bridge`）只产
//! `RoundRobin`/`Debate`/`MajorityVote`，`create_session` 亦不构造它；唯一 match/读取在
//! `bridge/src/lib.rs` 的 `run()`（分派）与 `run_single()`（取 speaker）⇒ `run_single`
//! **不可达**。且"单模型发言"**不是多模型讨论**（业界多智能体协作模式＝debate / voting /
//! expert-panel / hierarchical / round-robin，无 "single"），单模型诉求走 `/api/v1/sessions`
//! 即可 ⇒ 按 D-78/D-111 纪律删除（"只有定义、无任何构造方" = 漂移陷阱）。
//!
//! 本套件为**源码级 pin**：逐行剥 `//` 注释后断言
//!  ① `SingleSpeaker` **不存在**；② `fn run_single(` **不存在**；
//!  ③ **反向对照**：`RoundRobin` / `Debate` / `MajorityVote` **仍在**（防误删在用策略）。
//!
//! 文件头自报盲区：仅做**源码文本**断言，不校验语义（如策略行为）。

use std::path::PathBuf;

fn bridge_lib() -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("lib.rs");
    std::fs::read_to_string(&p).expect("bridge/src/lib.rs 必须存在")
}

/// 逐行剥掉 `//` 之后的内容（行注释；本文件不含块注释中的关键符号）。
fn code_only(src: &str) -> String {
    src.lines()
        .map(|l| match l.find("//") {
            Some(i) => &l[..i],
            None => l,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn single_speaker_strategy_is_retired() {
    let code = code_only(&bridge_lib());

    assert!(
        !code.contains("SingleSpeaker"),
        "D-181：`SingleSpeaker` 已退役（全仓零构造方 ⇒ run_single 不可达）——不得回潮"
    );
    assert!(
        !code.contains("fn run_single("),
        "D-181：不可达的 `run_single` 已随变体一并退役——不得回潮"
    );

    // 反向对照：在用策略不得被误删。
    for keep in ["RoundRobin", "Debate", "MajorityVote"] {
        assert!(
            code.contains(keep),
            "D-181 反向对照：在用策略 `{keep}` 不得被误删"
        );
    }
    assert!(
        code.contains("pub enum BridgeStrategy"),
        "D-181 反向对照：`BridgeStrategy` 枚举本身须仍在"
    );
}
