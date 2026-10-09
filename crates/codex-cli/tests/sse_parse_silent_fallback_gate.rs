//! 门禁：**SSE `data:` 解析失败不得静默回落成 `Value::Null`**（D-188，2026-10-08, traecode）。
//!
//! 病灶（"静默回落"族，承 D-164 / D-177 / D-184）：`client.rs::parse_sse_lines` 对事件
//! `data:` 载荷用 `serde_json::from_str(&data).unwrap_or(serde_json::Value::Null)`——**JSON 非法
//! 即静默置空**。后果：SSE 契约漂移（服务端 payload 形状变了 / 被中间人截断 / 残块拼接错）时，
//! 客户端把事件当 `data: null` 交给渲染层 ⇒ `sse.data.get("delta")` 恒 `None` ⇒ **答案内容
//! 静默丢失**，用户只看到"没有输出"，而**无从知晓**是解析失败。
//!
//! 修复口径（与 D-184 同）：`parse_sse_lines` 返回 `Vec<Result<SseEvent>>`，非法 JSON 一律
//! `Err`（带原始 `data` 与原因）；`parse_sse_stream` 原样转发，落到消费端**已有的** `Err` 分支
//! （`lib.rs` `render_events`：`Err(e) => render::error("stream error: …")`）⇒ 失败**可见**。
//!
//! 本套件为**源码级门禁**（修复改了 `parse_sse_lines` 的签名，引用型测试在修复前**无法编译**，
//! 故 RED 只能由源码级承担——同 D-182/D-184/D-185/D-187 先例）。逐行剥 `//` 后断言：
//!  ① 旧静默回落写法 `unwrap_or(serde_json::Value::Null)` **不存在**；
//!  ② `fn parse_sse_lines(` 存在且返回 `Vec<Result<SseEvent>>`（错误可承载）；
//!  ③ **反向对照**：`fn parse_sse_stream(` 与 `SseEvent` 仍存在（防误删）。
//!
//! 文件头自报盲区：仅源码文本断言；**行为**由 `src/client.rs` 单测覆盖（含"非法 JSON → Err"）。

use std::path::PathBuf;

fn client_rs() -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("client.rs");
    std::fs::read_to_string(&p).expect("codex-cli/src/client.rs 必须存在")
}

/// 逐行剥掉 `//` 之后的内容（本文件关键符号均不在块注释中）。
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
fn sse_data_parse_failure_is_surfaced_not_silently_null() {
    let code = code_only(&client_rs());

    // ① 旧静默回落写法必须消失。
    assert!(
        !code.contains("unwrap_or(serde_json::Value::Null)"),
        "D-188：SSE data 解析**不得**静默回落成 Value::Null（修复前写法仍在）"
    );

    // ② parse_sse_lines 须返回可承载错误的 Result。
    assert!(
        code.contains("fn parse_sse_lines("),
        "D-188：`parse_sse_lines` 必须存在"
    );
    assert!(
        code.contains("-> (Vec<Result<SseEvent>>, String)"),
        "D-188：`parse_sse_lines` 须返回 `Vec<Result<SseEvent>>`（把解析失败暴露给流消费端）"
    );

    // ③ 反向对照：流与事件类型不得被误删。
    assert!(
        code.contains("fn parse_sse_stream("),
        "D-188 反向对照：`parse_sse_stream` 不得被误删"
    );
    assert!(
        code.contains("SseEvent"),
        "D-188 反向对照：`SseEvent` 不得被误删"
    );
}
