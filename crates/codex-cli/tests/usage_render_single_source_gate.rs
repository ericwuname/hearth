//! D-217（2026-10-08, traecode）源码级门禁：token 用量提取/终端渲染必须**唯一实现**。
//!
//! **盲区（自报）**：仅断言源码文本（逐行剥 `//` 注释后）——不执行、不跑真实 run。
//! 口径「宁可漏报不误报」。
//!
//! 病灶：`(prompt_tokens, completion_tokens, calls)` 的提取曾在**三处**各写一份
//! （`lib.rs` 一次性 `chat` 路径、`run_local.rs` 的 `Done` 事件渲染器、`run_local.rs` 的
//! 报告文件 `usage_line`），且渲染器那处**永不触发**（`Done` 事件 payload 不含 `usage`）
//! ⇒ 死代码；而 `resume`/REPL 续跑路径**丢弃返回 report** ⇒ 那一轮 token 用量**看不到**。
//! 收口后：提取只在 `report.rs::usage_counts`；终端行只在 `report::usage_terminal_line`，
//! 由**共享路径** `run_local_continue` 末尾打印一次（fresh/resume/REPL 同口径）。

use std::fs;
use std::path::PathBuf;

fn code_only(s: &str) -> String {
    s.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

fn code_of(rel: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(rel);
    code_only(
        &fs::read_to_string(&p).unwrap_or_else(|_| panic!("crates/codex-cli/src/{rel} 必须存在")),
    )
}

#[test]
fn token_usage_extraction_has_single_source() {
    let lib = code_of("lib.rs");
    let run_local = code_of("run_local.rs");
    let report = code_of("report.rs");

    assert!(
        !lib.contains("prompt_tokens"),
        "D-217：`lib.rs` 不得再内联提取 token 用量（应经 `report::usage_counts`/`usage_terminal_line`）——否则与 resume 路径口径分叉"
    );
    assert!(
        !run_local.contains("prompt_tokens"),
        "D-217：`run_local.rs` 不得再内联提取 token 用量（报告文件行与终端行都走 `report::usage_counts`）"
    );
    assert!(
        report.contains("prompt_tokens") && report.contains("fn usage_counts"),
        "D-217：token 用量提取的唯一实现须在 `report.rs::usage_counts`"
    );
}

#[test]
fn terminal_usage_line_has_single_render_site() {
    let lib = code_of("lib.rs");
    let run_local = code_of("run_local.rs");

    // 终端用量行（唯一格式化点 `report::usage_terminal_line`）不得在别处再拼一次。
    assert!(
        !lib.contains("📊 tokens:"),
        "D-217：`lib.rs` 不得再渲染终端 token 用量行（共享路径已打印一次，否则重复）"
    );
    // 事件渲染器里不得再渲染（Done payload 无 usage ⇒ 必是死代码）。
    assert!(
        !run_local.contains("📊 tokens:"),
        "D-217：`run_local.rs` 的事件渲染器不得再渲染终端 token 用量行（Done payload 无 usage ⇒ 死代码）"
    );
}
