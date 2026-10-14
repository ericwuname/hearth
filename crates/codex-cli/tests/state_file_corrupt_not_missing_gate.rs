//! D-220（2026-10-08, traecode）源码级门禁：`session_store` 的状态档读口**不得把"损坏"静默当"不存在"**。
//!
//! **盲区（自报）**：仅断言源码文本（逐行剥 `//` 注释后）不含静默 `.ok()` 解析形态、
//! 且含留痕标记——不执行、不验证 warn 运行时输出。口径「宁可漏报不误报」。
//!
//! 病灶：`load_graph_with_revision` / `load_taskgoal` / `load_run_state` 对状态档用
//! `serde_json::from_str(&s).ok()` **静默吞掉解析失败**，返回 `None` ⇒ 与"文件不存在"
//! **不可区分**：`resume` 时用户以为执行位/任务目标已恢复，实则因档损坏**退化为仅历史恢复**
//! 且**毫无提示**。而**同一文件**的读口 `read_optional_capped` 已对"超限截断" `warn!` 留痕
//! （D-125）——同族两口径（同 D-215 对 `load_turns` 的处置）。

use std::fs;
use std::path::PathBuf;

fn code_only(s: &str) -> String {
    s.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn state_readers_do_not_silently_conflate_corrupt_with_missing() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/session_store.rs");
    let code =
        code_only(&fs::read_to_string(&p).expect("crates/codex-cli/src/session_store.rs 必须存在"));

    assert!(
        !code.contains("serde_json::from_str(&s).ok()"),
        "D-220：状态档读口不得用 `serde_json::from_str(&s).ok()` **静默**把损坏当缺失——\
         须在**文件存在但解析失败**时 `tracing::warn!` 留痕（与同文件「截断留痕」D-125 同口径）"
    );
    assert!(
        code.contains("状态档解析失败") || code.contains("状态档损坏"),
        "D-220：状态档解析失败须 `tracing::warn!` 留痕（标记 `状态档解析失败`/`状态档损坏`）"
    );
}
