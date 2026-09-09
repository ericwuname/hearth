# Codex-Rust 架构与实现逻辑总结

> 整理于 2026-07-28。本文所有结论均基于对源码的直接核对（`crates/agent-core/src/loop.rs`、`crates/planner/src/lib.rs`、`crates/service/src/main.rs`、`crates/sandbox/src/lib.rs` 等），非仅依赖审计报告。
>
> 核验基线：本仓库是 Cargo workspace，实际声明 **17 个 crate**（非某些文档口径的 19 个）。

---

## 1. 总体定位

一个 **Rust 编写的自主编码 Agent 系统**：给定目标 → LLM 规划 → 循环调用工具（bash/read/edit/glob/grep）执行 → 观察结果/LSP 诊断/语义检索 → 反思是否继续/重规划/放弃 → 产出结果。
对外通过 `service` crate（`axum` HTTP 服务）暴露 SSE 流式 API；LLM 后端支持 OpenAI / Ollama / vLLM / 腾讯混元，并可配置降级链。

---

## 2. 分层与依赖方向

依赖自上而下、底层在右（箭头 = "依赖于"）：

```
service (bin, main.rs)  唯一可执行入口，聚合所有能力
   │
   ├── agent-core  ──引擎：AgentLoop 主循环、上下文、调度
   │      │ 依赖：llm-gateway / tool-runtime / planner / retriever / code-index / lsp-bridge
   │      └── 这些都以 Arc<dyn Trait> 形式注入（依赖倒置）
   │
   ├── tools-builtin (5 个内置工具) ── tool-runtime (Tool trait + 分发/审批)
   │                                     └── sandbox (命令隔离)
   ├── llm-openai / llm-local / llm-cn ── llm-gateway (LlmProvider 抽象 + 注册表 + 降级链 + 计费)
   ├── memory (会话 JSONL 持久化)
   └── api (北向契约：AgentEvent / 请求/响应类型)
              │
        agent-types  ← 所有 crate 的共享数据基座（消息、工具调用、TaskGraph、运行状态）
```

要点：
- `agent-types` 是零依赖的共享类型层，被全部 crate 依赖。
- `agent-core` 是引擎，但它**不带任何断言/LLM 实现**，所有能力（planner、retriever、lsp、provider）都通过 trait 注入。
- `telemetry` 是叶子 crate（独立 eval 测试台），未被 `service` 依赖。

---

## 3. 核心抽象（trait 与关键类型）

各能力以独立 crate 中的 trait 定义，`agent-core` 仅持有 `Arc<dyn ...>`：

| Trait / 类型 | 所在 crate | 关键签名 |
|---|---|---|
| `LlmProvider` | `llm-gateway` | `chat()` / `stream()` / `embed()`；配套 `ProviderRegistry`、`FallbackChain`（有界顺序重试）、`CostMeter`（真实 token 计费） |
| `Tool` | `tool-runtime` | `name()` / `description()` / `execute(args, ctx)`；`ToolDispatcher` 负责注册、串行/并行分发、超时(30s)、按 `session_id` 隔离的审批状态机 |
| `Planner` | `planner` | `decompose(goal, ctx) -> TaskGraph` / `reflect(obs, state) -> ReflectVerdict` |
| `Retriever` | `retriever` | `search(query, top_k)`；BM25 + 向量 RRF 融合（实现完整，但生产未接线） |
| `LspBridge` | `lsp-bridge` | `diagnostics(file)`；当前仅有 `NoopLspBridge` 占位 |
| `Sandbox` | `sandbox` | `spawn(cmd, args, cwd, env, timeout) -> SandboxOutput` |
| `Agent` | `agent-core` | `step(phase)` / `run(goal)` |

共享类型（`agent-types`）：`Message`/`ToolCall`/`ToolResult`/`Budget`(默认 `max_steps=50`)/`TaskGraph`/`TaskNode`/`TaskStatus`/`PlanContext`/`Observation`/`PlanState`/`ReflectVerdict::{Continue,Replan,GiveUp}`。

---

## 4. 核心控制流：Agent 主循环（源码核实）

入口 `AgentLoop::run()`（`crates/agent-core/src/loop.rs:844`），主体是一个 `loop {}`（line 861）：
每轮检查 `budget_exhausted()` → `step(phase)` → 发射事件 → `steps+=1` → 命中 `Done`/`Error` 即 `break`。
相位由 `step()`（line 823）分派：

```
Init → Plan → Act → Observe → Reflect →（Continue→Plan / Replan→Plan / GiveUp→Error / 无工具调用→Done）
```

各相位实际干活的函数：
- **`do_plan`**（line 364）：构造 `PlanContext` → 调 `planner.decompose()` 生成 `TaskGraph`（仅当非空才替换，保护 delegable 状态，line 387）→ 仅在 `depth==0` 时对 delegable 任务 `spawn_sub_agent`（line 397-418，子代理 `depth=1` 不再派生子代理）→ 把 `TaskGraph` 注入 system prompt（`build_messages` line 312）→ 调 LLM，据 `tool_calls` 决定去 `Act` 还是 `Done`。
- **`do_act`**（line 501）：启发式审批（仅 `edit`/`bash` 且参数含 `rm `/`delete`/`write` 字符串才请求审批，line 506-511）→ 等审批 → `scheduler.execute_tool_calls()` → `Observe`。
- **`do_observe`**（line 566）：从工具参数/输出启发式抽取源码路径 → 查 `lsp_bridge.diagnostics()` 注入 `lsp_diagnostics` scratch → 若有 retriever 则 `retriever.search()` 注入 `retrieval_context` scratch → `collect_sub_agent_results()` 把子代理结果写回 `TaskGraph` 节点状态 → `Reflect`。
- **`do_reflect`**（line 717）：构造 `Observation` → 调 `planner.reflect()` → 按三态裁决分支。

> **关键事实**：`TaskGraph::next_ready()` / `topo_order()` 仅出现在 `agent-types` 与 `planner` 的单测中。主循环**不按 DAG 拓扑真实调度/逐节点推进**——`TaskGraph` 只是注入给 LLM 的"建议性上下文"，推进靠 LLM 自己决定调用哪些工具。

---

## 5. 规划器：decompose / reflect / replan / sub-agent（源码核实）

### 5.1 decompose（`planner/src/lib.rs:43`）
用 prompt 让 LLM 输出任务 JSON 数组，解析为 `Vec<TaskNode>`；解析失败回退为单节点 `[{id:"execute",...}]`（line 133）。会把 `retrieval_context` / `lsp_diagnostics` 注入 prompt（R1 情报注入）。

### 5.2 reflect（`planner/src/lib.rs:155`）—— 先启发式快路径（不调 LLM），再 LLM 慢路径
快路径阈值（`MAX_CONSECUTIVE_ERRORS=3` / `MAX_STEPS_WITHOUT_PROGRESS=5` / `BUDGET_LOW_THRESHOLD=0.15`）：
1. `consecutive_errors >= 3` → **GiveUp**（line 159）
2. `budget_remaining == 0` → **GiveUp**（line 169）
3. `budget_fraction <= 0.15 && steps_without_progress >= 2` → **GiveUp**（line 176）
4. `steps_without_progress >= 5` → **Replan**（line 186，**此分支无 `replan_count` 限制**）
5. `consecutive_errors >= 2 && replan_count < 3` → **Replan**（line 196，**此分支有 `replan_count` 限制**）
否则走 LLM 慢路径（line 207），文本匹配 `give_up`/`replan`/`continue`。

### 5.3 replan 的真实边界（你此前关注的 A3）
`do_reflect` 的 **Replan 分支**（`loop.rs:778-804`）只做：
- `plan_state.replan_count += 1`
- 重置 `steps_without_progress = 0`（**但 `consecutive_errors` 不重置**，line 788-790 注释明确：让 GiveUp 能跨 replan 累积）
- 把 `Failed` 节点重置为 `Pending` → 回到 `Plan` 重新 `decompose` 整图

**边界判定**：
- 错误路径（持续报错）：`consecutive_errors` 跨 replan 累积，到 3 即 `GiveUp`，构造上**有界、安全**。
- "无错也无进展、不停重做"路径：`steps_without_progress>=5` 触发 Replan（line 186），该分支**不检查 `replan_count`**，Replan 又把 `steps_without_progress` 归零 → 这条路径只靠 `Budget.max_steps` 全局兜底（`loop.rs:862`），而非 `replan_count` 封顶。
- 设计注释 `loop.rs:789` 写着"禁止无界 replan"，但实际封顶职责分散在 `consecutive_errors` 阈值与 `Budget.max_steps` 上，`replan_count` 本身未被用作 Replan 路径的硬上限。这与"禁止无界"的严格意图存在落差。
- 结论：**严格说不会死循环**（有 budget 兜底），但 `replan_count` 这个计数器目前对"无错卡死"路径未真正发挥封顶作用。

### 5.4 merge 的真实状态（你此前关注的 A4）
**全仓不存在独立的 `merge` 函数**（仅 `retriever` 的 RRF rank fusion 与 session 用量并入 CostMeter 用了 "merge" 一词）。
子代理结果的"合并"只是 `do_observe`（line 692-707）与 `do_reflect`（line 724-734）里：按 `task_id` 找到对应 `TaskNode`，把 `status` 置 `Completed`/`Failed`，并写入一个**仅含文字摘要的 `TaskResult`**。节点之间、子代理产物之间**没有内容级合并/整合逻辑**。
即 A4 评审项对应的现状：merge = 状态标记 + 摘要写入，**缺少真正的子代理产物整合**。

### 5.5 sub-agent
`spawn_sub_agent`（`loop.rs:210`）：`depth==0` 时对 delegable 节点 `tokio::spawn` 子 `AgentLoop`，子代理 `depth=1` 不再派生子代理（防递归）；`collect_sub_agent_results`（line 263）非阻塞收割已完成句柄。

---

## 6. 安全沙箱 sandbox（源码核实）

`crates/sandbox/src/lib.rs`：
- `create_sandbox` 按 `#[cfg(linux)]` 选 `LinuxSandbox`，否则 `NoopSandbox`（line 95 打印 **"Do NOT use in production"** 警告）。
- **v1.1 修复已落地（与项目记忆一致）**：`unshare` **包裹已被去除**（`grep "unshare"` 仅命中注释 line 353、551）。改为 `std::process::Command` + `child.pre_exec`（line 595），在子进程 `fork` 后、`exec` 前直接落隔离。
- `sandbox_pre_exec`（line 238）：先 `PR_SET_NO_NEW_PRIVS`（非特权 landlock 必需），再 `apply_landlock_in_child`（FS 白/黑名单）+ `apply_seccomp_deny_list`（手写 BPF，封 `ptrace`/`reboot`/`*_module`/`kexec_*`）。
- 不依赖 namespace（注释 line 549-557：namespace 需 `CAP_SYS_ADMIN`，非特权下 `EPERM`，故刻意不用）。
- landlock/seccomp 不可用时降级跳过（warn），不致命。
- **本机 Windows（win32）下走 `NoopSandbox`**：生产环境若在 Windows 运行，bash/工具**无真实隔离**。
- 注意：模块文档注释（line 3）提到 "process namespace + cgroups"，但源码中**没有 namespace/cgroups 实现**，实际只有 landlock + seccomp。

---

## 7. 生产接线状态（源码核实的关键缺口）

`service/src/main.rs` 组装了：provider 注册表（openai 必注册，其余按环境变量可选，可组降级链 line 94）、5 个内置工具（line 119-125）、`JsonlMemoryStore`（line 139-140）。
**但 `main.rs` 从未调用 `set_retriever()` / `set_lsp_bridge()`**（已通读全文确认）。后果：
- `retriever`（A5 语义检索）：生产二进制中 `AgentLoop.retriever = None` → `do_observe` 的检索分支（line 654）永不触发。
- `lsp-bridge`（A4 LSP 诊断）：生产走默认的 `NoopLspBridge` → `do_observe` 的 LSP 分支（line 614）永远返回空。
- 也就是说，A4/A5 在**代码与单测层面已实现并验证，但默认生产路径是关闭的**——这是成熟度判读的最大陷阱。需显式 `set_retriever`/`set_lsp_bridge` 接真实实现（retriever 已有完整实现；lsp-bridge 仅有 Noop 占位）才会生效。

---

## 8. 成熟度判断（基于实际读到的代码量）

**高成熟度（有实质实现 + 覆盖测试）：**
- `agent-core` / `loop.rs`：引擎完整，含子代理、预算、审批、CostMeter、真实 `DefaultPlanner` 端到端卡死检测测试。
- `llm-gateway` + `llm-openai`/`llm-local`/`llm-cn`：三个 provider 均完整 wire 实现，local/cn 带 axum mock server 真实协议测试。最扎实。
- `sandbox`：landlock + seccomp 完整且有针对性测试（含 X1 泄漏修复）。
- `tools-builtin` / `tool-runtime`：5 工具 + 分发/并行/超时/审批 + 路径穿越防护，测试充分。
- `memory`：JSONL 持久化可用。`telemetry`：含 5 个真实 eval 任务的回归测试台。
- `planner`：decompose/reflect（启发式 + LLM 双路径）实现完整。

**中等（实现存在但生产未接线 / 能力受限）：**
- `retriever` + `code-index`：实现与测试完整，但 `service` 未实例化 → 生产不启用语义检索。
- `lsp-bridge`：仅有 `NoopLspBridge` 占位（注释明确"真实 LSP client 延期至 post-1.0"）。

**骨架 / 风险点：**
1. **生产接线缺口**：`service` 未 `set_retriever`/`set_lsp_bridge`，A4/A5 默认关闭（最大陷阱）。
2. **TaskGraph 未被真正调度**：`next_ready`/`topo_order` 仅单测用，引擎不按拓扑推进，规划是"建议性"而非"执行性"。
3. **merge 缺陷（A4）**：无内容级子代理产物合并，仅状态标记 + 摘要写入。
4. **replan 边界（A3）**：`replan_count` 未作为"无错卡死"路径的硬封顶，靠 `Budget.max_steps` 兜底。
5. **审批是字符串启发式**：`do_act`（line 506）用 `args.to_string().contains("rm "||"delete"||"write")` 判断，易绕过/误判。
6. **Windows 无隔离**：本机 win32 下 sandbox 退化 `NoopSandbox`。

---

## 9. 一句话结论

**主 Agent 执行链路（LLM → loop → tool → sandbox → memory → api）实现率约 0.85+、质量高、测试覆盖广；但外围智能增强层（LSP 诊断 / 语义检索 / 规划 DAG 真实调度 / 子代理产物合并 / replan 硬封顶）偏骨架或生产未接线，属于"代码写了、单测用 mock 验证了链路、但没接进 `service` 真实路径、也缺少内容级逻辑"的状态。** 你此前 P3 评审打回的 A3（replan 边界）与 A4（merge）在源码层面当前仍呈现上述特征，建议返工评审时重点核对这两处是否已真正闭环。
