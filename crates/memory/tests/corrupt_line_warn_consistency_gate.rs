//! D-203（2026-10-08, traecode）源码级门禁：`memory` 的 **JSONL 读口对坏行的处置必须同口径**。
//!
//! **盲区（自报）**：仅断言源码文本（逐行剥 `//` 注释后）不含 `if let Ok(..) = from_str` 的静默
//! 形态、且含留痕标记——不执行、不验证 warn 的运行时输出（无 subscriber 可观测）。口径「宁可漏报不误报」。
//!
//! 病灶：同 crate 内 `load_session` 对损坏行是 **MEM-4: warn + skip**（不丢整档），而
//! `CivilizationStore::read_entries`（`CivEntry`）与 `WorkLineStore` 的读口却用
//! `if let Ok(..) = serde_json::from_str(..)` **静默丢弃坏行** ⇒ 同一操作三种口径，
//! 坏行（含被截断的更早版本写入）**无声消失**，而 `recent()`/可见性输出缺条**无任何提示**。

use std::fs;
use std::path::PathBuf;

fn code_only(s: &str) -> String {
    s.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn all_jsonl_readers_warn_on_corrupt_line() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let code = code_only(&fs::read_to_string(&p).expect("crates/memory/src/lib.rs 必须存在"));

    assert!(
        !code.contains("if let Ok(entry) = serde_json::from_str::<CivEntry>"),
        "D-203：civ 读口不得用 `if let Ok(..)` **静默丢弃**坏行——应与 `load_session`（MEM-4）同口径 warn 留痕"
    );
    assert!(
        !code.contains("if let Ok(node) = serde_json::from_str::<WorkNode>"),
        "D-203：workline 读口不得用 `if let Ok(..)` **静默丢弃**坏行——应与 `load_session`（MEM-4）同口径 warn 留痕"
    );
    assert!(
        code.contains("损坏行已跳过"),
        "D-203：civ/workline 读口须对坏行 `tracing::warn!` 留痕（标记 `损坏行已跳过`）"
    );
}
