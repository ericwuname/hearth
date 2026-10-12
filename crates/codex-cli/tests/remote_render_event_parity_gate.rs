//! D-197（2026-10-08, traecode）源码级门禁：CLI 的**远程**渲染点事件覆盖必须与契约**齐全**。
//!
//! **盲区（自报）**：仅断言源码文本（逐行剥 `//` 注释后）含 `"<name>" =>` 匹配臂——不执行、
//! 不验证各臂的**字段路径/渲染内容**（那由 `sse_str` 单测与既有行为门禁覆盖）。之所以用
//! 源码级：`render_events`（`lib.rs`）与远程 REPL（`repl.rs`）都是消费 SSE 流的**大循环**，
//! 无纯函数可驱动（行为侧需整套 CLI/服务端流 harness）；口径「宁可漏报不误报」。
//!
//! 病灶（D-153/D-152 同族"远程渲染点漏改一处"）：契约 `api::AgentEvent` 共 **15** 个变体，
//! 服务端另发一条合成 **`notice`**（D-195）。本地渲染器 `run_local.rs` 对 `AgentEvent` 是
//! **穷尽 match**（编译器保证齐全），而两个**远程**渲染器按字符串匹配、缺臂即**静默落
//! `_ => {}`**：`lib.rs::render_events` 缺 `goal_changed`/`interaction_resolved`；
//! `repl.rs` 缺 `span_open`/`span_close`/`artifact`/`think_summary`/`plan_draft`/
//! `goal_changed`/`interaction_resolved`/`notice` ⇒ 远程用户**看不到**这些事实（REPL 连
//! 产物/规划草案/目标修订都丢）。修复＝把两个远程渲染器补齐到与本地一致的事件覆盖，并本门禁钉死。

use std::fs;
use std::path::PathBuf;

/// 契约 `api::AgentEvent` 的 15 个子事件 + 合成 `notice`（D-195），共 16。
const REQUIRED: &[&str] = &[
    "phase",
    "token",
    "tool_call",
    "tool_result",
    "need_approval",
    "reflection",
    "done",
    "error",
    "span_open",
    "span_close",
    "artifact",
    "think_summary",
    "plan_draft",
    "goal_changed",
    "interaction_resolved",
    "notice",
];

fn code_only(s: &str) -> String {
    s.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn both_remote_renderers_cover_all_events() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for file in ["src/lib.rs", "src/repl.rs"] {
        let raw =
            fs::read_to_string(root.join(file)).unwrap_or_else(|e| panic!("读 {file} 失败: {e}"));
        let code = code_only(&raw);
        for ev in REQUIRED {
            let pat = format!("\"{ev}\" =>");
            assert!(
                code.contains(&pat),
                "{file} 缺事件匹配臂 `{pat}`——该事件在**远程**渲染会被静默落 `_ => {{}}` 兜底\
                 （D-197：远程渲染点事件覆盖不齐，承 D-153）。"
            );
        }
    }
}
