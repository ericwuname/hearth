//! 门禁：CLI 的 SSE 事件匹配必须用**小写**契约名（D-153，2026-10-04, traecode）。
//!
//! 背景（真实缺陷）：`ai-os-event-contract-v1` 规定事件名**小写**（`phase`/`token`/`done`…），
//! 服务端也照此下发。B4-1 早把 `lib.rs::render_events` 改成小写匹配，但 **`repl.rs` 的远程
//! 分支漏改**——仍是 `"Phase"`/`"Token"`/… 大写匹配 ⇒ `hearth --url … repl` 的远程会话
//! **一个事件都匹配不上**，整段（相位/答案/工具/收尾）静默不渲染。
//!
//! 判据（文本级，逐行剥 `//` 注释后匹配）：`crates/codex-cli/src/repl.rs` 中
//! ① **不得**出现大写匹配形态（`"Phase"`/`"Token"`/`"ToolCall"`/`"ToolResult"`/
//!    `"NeedApproval"`/`"Reflection"`/`"Done"`/`"Error"`）；
//! ② 必须出现小写匹配（`"token"` 与 `"done"`）。
//!
//! 文件头自报盲区：① 只扫 `repl.rs`（`lib.rs::render_events` 由 D-153 的直接修改覆盖，
//! 其正确性另有单测 `d153_tests`）；② 只钉"事件名大小写"，不钉字段路径（那由
//! `sse_str` 单测与 `lib.rs` 侧覆盖）。

use std::path::PathBuf;

#[test]
fn repl_sse_matchers_use_lowercase_contract_names() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("repl.rs");
    let raw = std::fs::read_to_string(&p).expect("crates/codex-cli/src/repl.rs 必须存在");
    // 剥掉 `//` 行注释：说明性注释可以点名"旧的大写写法"，而真正的**匹配字面量**一旦回退即报红。
    let src: String = raw
        .lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");

    for bad in [
        "\"Phase\"",
        "\"Token\"",
        "\"ToolCall\"",
        "\"ToolResult\"",
        "\"NeedApproval\"",
        "\"Reflection\"",
        "\"Done\"",
        "\"Error\"",
    ] {
        assert!(
            !src.contains(bad),
            "`repl.rs` 又出现**大写**事件匹配 {bad}——契约是**小写**（ai-os-event-contract-v1），\
             服务端也发小写 ⇒ 大写匹配永不命中，远程 REPL 会整段静默不渲染（D-153 病灶）。"
        );
    }
    for good in ["\"token\"", "\"done\""] {
        assert!(
            src.contains(good),
            "`repl.rs` 必须用小写事件名匹配（缺 {good}）——与契约及 `lib.rs::render_events` 对齐。"
        );
    }
}
