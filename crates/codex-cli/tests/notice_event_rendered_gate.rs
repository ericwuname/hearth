//! D-195（2026-10-08, traecode）源码级门禁：**CLI 必须显式渲染服务端 `notice` 事件**。
//!
//! **盲区（自报）**：仅断言**源码文本**（逐行剥 `//` 注释后）含指定 `match` 臂——不执行、
//! 不验证运行时行为。之所以用源码级：`render_events` 是 `lib.rs` 内消费 SSE 流的**大循环**，
//! 无纯函数可驱动（行为侧需整套 CLI/服务端流 harness）；口径「宁可漏报不误报」。
//!
//! 背景：D-195 修复后，服务端在广播被追平时发一条**非终止** `notice` 事件
//! （`{type:"notice",dropped:n}`）报告缺口。若 CLI 的 `render_events` 落进 `_ => {}`
//! 兜底（修复前即如此），该提示**重新静默**——用户看到被截断/跳字的答案却无任何指示，
//! 等于缺陷在消费端复发。本门禁把"CLI 识别 notice 并提示"钉死。

use std::fs;
use std::path::PathBuf;

fn code_only(s: &str) -> String {
    s.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn cli_renders_notice_event() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = fs::read_to_string(root.join("src/lib.rs")).expect("read src/lib.rs");
    let code = code_only(&src);
    assert!(
        code.contains("\"notice\" =>"),
        "D-195：CLI `render_events` 必须显式匹配 `\"notice\"` 事件并提示缺口\
         （否则服务端的 notice 落 `_ => {{}}` 兜底，静默丢失在消费端复发）"
    );
}
