//! D-204（2026-10-08, traecode）源码级门禁：`experience` 读口对坏行必须**留痕**（与 D-203 同口径）。
//!
//! **盲区（自报）**：仅断言源码文本（逐行剥 `//` 注释后）不含 `if let Ok(..) = from_str` 的静默形态、
//! 且含留痕标记——不执行、不验证 warn 的运行时输出（无 subscriber 可观测）。口径「宁可漏报不误报」。
//!
//! 病灶：`experience` 的**懒加载**（`ensure_loaded`）与**急加载**（`set_path`）两处读 JSONL 时，
//! 对无法反序列化的行用 `if let Ok(exp) = serde_json::from_str::<Experience>(line)` **静默丢弃**
//! （而本 crate 对"文件超限截断"是**有 warn 留痕**的——同 crate 两口径）⇒ `metrics()`/
//! `recent_failures()`（后者会被注入回 prompt，间接影响 agent 行为）反映的是**不完整**语料，
//! 坏行无声消失、失败率被低估。

use std::fs;
use std::path::PathBuf;

fn code_only(s: &str) -> String {
    s.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn experience_readers_warn_on_corrupt_line() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let code = code_only(&fs::read_to_string(&p).expect("crates/experience/src/lib.rs 必须存在"));

    assert!(
        !code.contains("if let Ok(exp) = serde_json::from_str::<Experience>"),
        "D-204：`experience` 读口不得用 `if let Ok(..)` **静默丢弃**坏行——应 `tracing::warn!` 留痕（与 D-203/截断留痕同口径）"
    );
    assert!(
        code.contains("损坏行已跳过"),
        "D-204：`experience` 读口须对坏行 `tracing::warn!` 留痕（标记 `损坏行已跳过`）"
    );
}
