//! D-205（2026-10-08, traecode）源码级门禁：`note.rs` 的 distress 扫描必须对坏行**留痕**。
//!
//! **盲区（自报）**：仅断言源码文本（逐行剥 `//` 注释后）不含 `if let Ok(v) = from_str::<Value>(line)`
//! 的静默形态、且含留痕标记——不执行、不验证 warn 的运行时输出。口径「宁可漏报不误报」。
//!
//! 病灶：`has_distress_signal` 在**同一函数上方**已对"档案超限截断" `tracing::warn!` 留痕，
//! 明言"不能让'没检测到'变成静默假阴性"（D-125），却在紧随其后的逐行解析用
//! `if let Ok(v) = serde_json::from_str::<serde_json::Value>(line)` **静默跳过坏行**——
//! 坏行内容**无从判断**是否携带异常信号（`mood`/`observer_verdict`），静默跳过正是它
//! 自己警告过的那种**假阴性**（漏报用户 distress）。同 crate、同函数、两口径。

use std::fs;
use std::path::PathBuf;

fn code_only(s: &str) -> String {
    s.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn note_distress_scan_warns_on_corrupt_line() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/note.rs");
    let code = code_only(&fs::read_to_string(&p).expect("crates/codex-cli/src/note.rs 必须存在"));

    assert!(
        !code.contains("if let Ok(v) = serde_json::from_str::<serde_json::Value>(line)"),
        "D-205：distress 扫描不得用 `if let Ok(..)` **静默跳过**坏行——与同函数「截断留痕」同一理由，\
         须 `tracing::warn!` 留痕（否则'没检测到'变**假阴性**，漏报 distress）"
    );
    assert!(
        code.contains("损坏行已跳过"),
        "D-205：`has_distress_signal` 须对坏行 `tracing::warn!` 留痕（标记 `损坏行已跳过`）"
    );
}
