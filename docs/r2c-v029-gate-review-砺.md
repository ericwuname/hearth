# R2-C v0.2.9 施工门禁审查（守门员 砺·评审）

**时间**：2026-08-29 凌晨（评审窗口自主执行）
**对象**：`crates/agent-core/src/loop.rs` v0.2.9 施工（R2-C ContextBuilder，`6c599e3` 起）
**方法**：把工作区 loop.rs 同步到评审 VM `.133`（保留其 35G Linux target），`cargo test -p agent-core`，逐条比对 v1.1 批准书验收口径。

## 门禁裁决：🔴 未通过（官方树现状），但根因是测试脚手架缺陷，非施工逻辑缺陷

### 一、官方树当前状态（同步 v0.2.8→v0.2.9 后首次 `cargo test -p agent-core`）

```
error[E0277]: the trait bound `TaskResult: From<&str>` is not satisfied
    --> crates/agent-core/src/loop.rs:5561:75
5561 |  agent.task_graph.nodes[0].result = Some("cargo test 失败: 3 errors".into());
```
→ lib test 编译失败，**6 个 R2-C 测试一个都跑不了**。

### 二、修复编译错误后，3/6 R2-C 测试失败（全量运行 67 passed / 3 failed，3 个全是 R2-C 测试）

| 测试 | 失败点 | 根因定性 |
|---|---|---|
| `test_cb_failed_node_preserved_in_continuity` | 5560 `index out of bounds`（nodes 空） | **测试脚手架**：没调 `do_plan_inner()`，`task_graph` 初始为空（`make_test_agent` 把 `make_simple_task_graph()` 丢了，没传 `AgentLoop::new`） |
| `test_cb_compact_keeps_continuity_and_topology` | 5594 `compact 不触 Continuity` | **测试脚手架**：没设 `original_goal` → `task_continuity_message()` 于 960 行 `original_goal.as_ref()?` 返回 None |
| `test_cb_status_change_keeps_system` | 5492 `unwrap()` on None | **测试脚手架**：同上，没设 `original_goal`；且 5495 行断言字符串 `"accomplish the goal"` 根本不在 Continuity 格式里（格式含「原始目标/已完成/剩余/下一步」）→ **错断言** |

> `test_cb_topology_sig_semantics` / `test_cb_experience_l4_only` / `test_cb_system_stable_across_calls` 三个**本身写法正确，直接通过**；
> `topology_block` 经压缩存活（5593 断言通过）→ 证明压缩**不动拓扑块**（RC5 治理核心机制成立）。

### 三、在 `.133` 副本上补齐 4 处测试缺陷后：6/6 R2-C 测试全绿（`cargo test -p agent-core r#loop::tests::test_cb` → 6 passed; 0 failed）

**证明：施工逻辑正确，无需改 build_messages / topology_sig / Continuity 任何一行。**

### 四、执行窗口须落到工作区的 4 处修正（机械、非设计变更）

1. **编译修复** `loop.rs:5561`
   ```rust
   // 旧（编译失败）：
   agent.task_graph.nodes[0].result = Some("cargo test 失败: 3 errors".into());
   // 新（对照生产构造 loop.rs:2798）：
   agent.task_graph.nodes[0].result = Some(agent_types::TaskResult {
       ok: false,
       output: "cargo test 失败: 3 errors".to_string(),
       steps: 0,
   });
   ```

2. **`test_cb_failed_node` 补 `do_plan_inner`**（在 5559 行 `set_session_id("cb-5")` 之后插入）
   ```rust
   let _ = agent.do_plan_inner().await;   // 否则 nodes 为空，nodes[0] 越界
   ```
   （该测试已在 5562 行手动设 `original_goal`，无需再加。）

3. **`test_cb_status_change` 补 `original_goal`**（在 5475 行 `set_session_id("cb-2")` 之后插入）
   ```rust
   agent.ctx_mgr.state_mut().original_goal = Some("cb status".into());
   ```
   并修正错断言（5495 行）：`ct.contains("accomplish the goal")` → `ct.contains("原始目标") && ct.contains("已完成")`

4. **`test_cb_compact` 补 `original_goal`**（在 5580 行 `set_session_id("cb-6")` 之后插入）
   ```rust
   agent.ctx_mgr.state_mut().original_goal = Some("cb compact".into());
   ```

### 五、执行窗口收尾动作
- 落上述 4 处修正到工作区 → 重新同步 `.133` → `cargo test -p agent-core` 须 **全绿（含 6 R2-C）** 才算施工闭环。
- 注意 `loop.rs` 是 **CRLF** 行尾（continuity format 折行相关），改完用 `cargo fmt` 校验。
- 本审查副本（`.133` 上已打临时补丁）下次同步会被工作区覆盖，无副作用。

## 守门员结论
- 施工**逻辑正确**，v1.1 批准书的 L1–L5 分层、topology_sig 双签名分立、Experience 降级通道移 L4、Task Continuity 块、[FAILED] 保全、compact 存活——**全部经测试实证成立**。
- 阻塞项是**测试代码自身的 4 处缺陷**（1 编译 typo + 2 缺前置 + 1 错断言），非 build_messages 实现问题。
- 按「审计工具自身必须被审计」纪律，守门员只拦不产；修正交执行窗口落工作区后重跑门禁确认。
