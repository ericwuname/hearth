//! D-215（2026-10-08, traecode）源码级门禁：`session_store::load_turns` 必须对损坏 Turn 行**留痕**。
//!
//! **盲区（自报）**：仅断言源码文本（逐行剥 `//` 注释后）不含静默丢弃形态、且含留痕标记
//! ——不执行、不验证 warn 的运行时输出（无 subscriber 可观测）。口径「宁可漏报不误报」。
//!
//! 病灶：`load_turns`（resume 重建上下文的读口）在**同一文件上方**的 `read_optional_capped`
//! 已对"档案超限截断" `tracing::warn!` 留痕（明言"后段内容本轮不可见"，D-125），却在紧随其后的
//! 逐行解析用 `.filter_map(|l| serde_json::from_str::<Turn>(l).ok())` **静默丢弃坏行**——
//! 坏行（含旧版本 Turn 结构、被外部写坏/半写）**无声消失**，而 `resume` 用的历史与
//! `list_sessions` 报的轮数**缺条且无任何提示**（同 crate、同文件、两口径）。

use std::fs;
use std::path::PathBuf;

fn code_only(s: &str) -> String {
    s.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn session_turns_load_warns_on_corrupt_line() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/session_store.rs");
    let code =
        code_only(&fs::read_to_string(&p).expect("crates/codex-cli/src/session_store.rs 必须存在"));

    assert!(
        !code.contains(".filter_map(|l| serde_json::from_str::<Turn>(l).ok())"),
        "D-215：`load_turns` 不得用 `filter_map(..).ok()` **静默丢弃**坏 Turn 行——与同文件\
         「截断留痕」（D-125）同一理由，须 `tracing::warn!` 留痕（否则 resume 历史缺条而用户不知）"
    );
    assert!(
        code.contains("损坏 Turn 行已跳过"),
        "D-215：`load_turns` 须对坏行 `tracing::warn!` 留痕（标记 `损坏 Turn 行已跳过`）"
    );
}
