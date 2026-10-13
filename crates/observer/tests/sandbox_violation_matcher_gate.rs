//! D-207（2026-10-08, traecode）行为门禁：G0 `sandbox_violation` 熔断的匹配串必须命中**真实生产者**。
//!
//! 病灶：`circuit::evaluate` 的匹配串为 `SANDBOX`+`DENIED`（或 `PATH`+`OUTSIDE`+`SCOPE`），
//! **无任何真实生产者会产出**——它只匹配 `circuit.rs` 自造的单测串（"DENIED: path /etc/passwd
//! outside sandbox scope"）⇒ G0 红线 `sandbox_violation` 在**生产里永不触发**，而单测却绿
//! （"声称≠实现 / 假绿"族）。
//!
//! 真实事实源是 **agent 自身确定性的路径护栏拒绝串**：`tools-builtin` 的
//! read/edit/patch/grep/glob 与 `agent-runtime::resolve_in_workspace` 产出的错误串
//! （这些串只由护栏产出，经 `ToolResult.output` 进入事件流）。
//!
//! 本门禁断言：**每个真实护栏拒绝串**都必须触发 `sandbox_violation`。

use api::{AgentEvent, EnvelopedEvent};
use observer::circuit;
use observer::metrics::Metrics;

fn tool_result(seq: u64, output: &str) -> EnvelopedEvent {
    EnvelopedEvent {
        schema_version: 1,
        ts: "2026-08-03T00:00:00Z".into(),
        seq,
        span_id: String::new(),
        parent_id: None,
        event: AgentEvent::ToolResult {
            call_id: "c1".into(),
            is_error: true,
            output: output.into(),
        },
    }
}

#[test]
fn real_guard_denial_strings_trigger_sandbox_break() {
    // 逐字取自生产代码（见 crates/tools-builtin/src/{read,edit,patch,grep,glob}.rs
    // 与 crates/agent-runtime/src/session.rs 的 `resolve_in_workspace`）。
    let real_producers = [
        "path traversal denied: ../secret.txt",
        "path outside allowed roots (workspace/home) denied: /etc/passwd (workspace: /ws)",
        "path outside workspace denied: ../x",
        "path outside workspace denied (symlink/junction escape): ../x",
        "path traversal via symlink/junction denied: escape (at /ws/escape)",
        "path traversal denied in glob pattern: ../**",
    ];
    for s in real_producers {
        let events = vec![tool_result(1, s)];
        let breaks = circuit::evaluate(&Metrics::default(), &events).unwrap();
        assert!(
            breaks.iter().any(|b| b.rule == "sandbox_violation"),
            "D-207：真实护栏拒绝串必须触发 sandbox_violation 熔断——\
             否则 G0 红线在生产里永不触发。输入 {s:?}，实得规则 {:?}",
            breaks.iter().map(|b| &b.rule).collect::<Vec<_>>()
        );
    }
}

/// 反向对照：普通命令失败（OS 级 `permission denied`/`not permitted`）**不得**误判为 G0 红线——
/// 那会出现在任意良性输出里（如 `cat /root/x`），把可用性白送给误报。
#[test]
fn benign_os_denials_do_not_trigger_sandbox_break() {
    for s in [
        "cat: /root/x: Permission denied",
        "mount: permission denied",
        "Operation not permitted",
    ] {
        let events = vec![tool_result(1, s)];
        let breaks = circuit::evaluate(&Metrics::default(), &events).unwrap();
        assert!(
            !breaks.iter().any(|b| b.rule == "sandbox_violation"),
            "D-207：良性 OS 拒绝 {s:?} 不得触发 G0 红线（防误报）"
        );
    }
}
