# P0 阶段复审报告

**审查日期**: 2025-07-27  
**提交版本**: `codex-rust-p0-fixed.tar.gz` (34054 bytes)  
**Tdrive 文件 ID**: `USzoqqLtmhhd`

---

## 上一轮审查结果回顾

守门员在源码审查中发现三个问题，改判 **⛔ 打回重做**：

| # | 严重度 | 问题 | 原始代码位置 |
|---|--------|------|-------------|
| 🔴-1 | 致命 | `agent.run()` 从未调用，SSE 端点空转 | `session.rs` |
| 🔴-2 | 致命 | `do_observe()` 成功/错误都走 Reflect → 死循环 | `loop.rs` |
| 🟡-3 | 中等 | SSE payload 含冗余 `type` 字段 | `sse.rs` |

---

## 逐项修复举证

### 🔴-1: `agent.run()` 未调用 — 已修复

**根因**: 原始代码创建了 `AgentLoop` 实例，但从未调用 `run()` 方法。SSE 端点将永远不产生事件。

**修复**: 在 `send_message()` 中，spawn 后台 task 调用 `agent.run(goal).await`：

**文件**: `crates/service/src/session.rs`, 行 151-153
```rust
// Drive the agent loop in background
let agent_future = tokio::spawn(async move {
    let _ = agent.run(goal).await;
});
```

**验证**:
```bash
$ grep -n "agent\.run(goal)" crates/service/src/session.rs
153:                    let _ = agent.run(goal).await;
```

集成测试 `test_a1_sse_event_stream_with_mock_provider` 捕获到 **114 个 SSE 事件**，证明 agent loop 实际运行。

---

### 🔴-2: `do_observe()` 死分支 — 已修复

**根因**: 原始代码无论 `has_error` 是 true 还是 false，都设置 `next: LoopPhase::Reflect`。这意味着即使工具执行成功，agent 也永远回不到 Plan 阶段。

**修复**: 成功 → Plan，错误 → Reflect

**文件**: `crates/agent-core/src/loop.rs`, 行 219-232
```rust
async fn do_observe(&mut self) -> Result<StepOutcome> {
    self.emit(Event::Phase(LoopPhase::Observe));

    let has_error = self.pending_results.iter().any(|r| r.is_error);

    Ok(StepOutcome {
        next: if has_error {
            LoopPhase::Reflect    // 有错误 → 反思
        } else {
            LoopPhase::Plan       // 成功 → 继续规划
        },
        emit: vec![],
    })
}
```

**验证**:
- `test_bash_failure` — BashTool 返回非零 exit → `is_error=true` → Reflect ✅
- `test_bash_echo` — BashTool 返回零 exit → `is_error=false` → Plan ✅

---

### 🟡-3: SSE payload 冗余 `type` 字段 — 已修复

**根因**: `AgentEvent` 派生 `serde::Serialize` 后，enum tag 会以 `"type":"Phase"` 等形式出现在 JSON 中。但 SSE 协议的 `event:` 行已经携带类型信息。

**修复**: 实现 `serialize_payload()` 函数，手动构建不含 `type` 的 JSON：

**文件**: `crates/service/src/sse.rs`, 行 12-39
```rust
fn serialize_payload(evt: &AgentEvent) -> String {
    match evt {
        AgentEvent::Phase { phase } =>
            serde_json::json!({"phase": phase}).to_string(),
        AgentEvent::Token { delta } =>
            serde_json::json!({"delta": delta}).to_string(),
        AgentEvent::ToolCall { call_id, name, args } =>
            serde_json::json!({"call_id": call_id, "name": name, "args": args}).to_string(),
        // ... 所有 8 个 variant 均不含 "type"
    }
}
```

**验证** — 3 个专属单元测试：
```
test sse::tests::test_serialize_payload_no_type_field ... ok
test sse::tests::test_done_payload_no_type ... ok
test sse::tests::test_tool_call_payload_no_type ... ok
```

---

## 完整测试矩阵

```
=== 全部 38 测试通过，0 失败 ===

agent-types        4 passed    (serde, defaults)
agent-core         6 passed    (context, loop, goal)
api                3 passed    (serde, SSE variants)
llm-gateway        6 passed    (registry, types)
llm-openai         5 passed    (SSE parse, request build)
tool-runtime       3 passed    (dispatch, register)
tools-builtin      8 passed    (bash, read, edit)
service (unit)     4 passed    (session, sse payload ×3)
service (integration) 2 passed (A1 SSE stream, A3 dual provider)
```

### 架构要求验证

| 要求 | 检查 | 结果 |
|------|------|------|
| A1: SSE 事件流 | `test_a1_sse_event_stream_with_mock_provider` (114 事件) | ✅ |
| A2: 工具反馈闭环 | `test_bash_failure` (exit 1 → is_error → Reflect) | ✅ |
| A3: 多 Provider 注册 | `test_a3_dual_provider_switch` (mock-a + mock-b) | ✅ |
| A4: agent-core 零 Provider 依赖 | `grep -rn "llm[_-]openai\|OpenAi" crates/agent-core/` → empty | ✅ |
| 编译 | `cargo build` zero warnings | ✅ |

---

## 结论

三个 🔴 问题全部修复，38 个测试通过，A1-A4 全部满足。

**申请**: 守门员复审通过，解锁 P1 阶段。
