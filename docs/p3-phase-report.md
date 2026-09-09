# P3 阶段报告：规划（任务分解 + 反思 + 子 Agent 委托）

**日期**: 2025-01  
**阶段**: P3 — Planner Crate + Agent Loop Integration  
**状态**: ✅ 全部通过

---

## 1. 阶段概述

P3 实现 Codex Rust 的第三个核心阶段：为 agent loop 引入结构化规划能力。包括：
- **planner crate**: TaskGraph 任务分解 + 三路反思判定（Continue/Replan/GiveUp）
- **agent-core 集成**: `do_plan()` 调用 decompose，`do_reflect()` 调用 reflect
- **卡住检测 (stuck detection)**: 连续错误、无进展步数、预算枯竭检测
- **子 Agent 委托 (A4)**: delegable 任务 spawn 子 agent 异步执行

---

## 2. D2 数据

### 2.1 代码量 (grep 实测)

| Crate | 文件 | LOC |
|-------|------|-----|
| `agent-types` | `src/lib.rs` | 447 |
| `planner` | `src/lib.rs` | 505 |
| `agent-core` | `src/loop.rs` + `src/context.rs` + `src/scheduler.rs` + `src/lib.rs` | 1,852 |
| **全 workspace (src/)** | 所有 crate | **7,016** |

### 2.2 测试统计

| Crate | 测试数 |
|-------|--------|
| `agent-types` (单元) | 9 |
| `planner` (单元) | 4 |
| `agent-core` (单元) | 13 |
| `service` (单元) | 4 |
| `service` (集成) | 8 |
| **总计** | **38** |

### 2.3 编译与测试

```sh
$ cargo test --workspace
# 全部 38 个测试通过，0 失败
```

---

## 3. A1–A6 证据矩阵

### A1: planner crate — TaskGraph + decompose ✅

**证据**: `crates/planner/src/lib.rs` (505 LOC)

- `Planner` trait: `decompose(goal, &PlanContext) -> TaskGraph` + `reflect(&Observation, &PlanState) -> ReflectVerdict`
- `DefaultPlanner`: 使用 `Arc<dyn LlmProvider>` 进行 LLM 辅助分解
- `TaskGraph` 纯函数: `is_acyclic()` (Kahn 算法), `topo_order()`, `next_ready()`, `count_by_status()`
- 回退机制: LLM 解析失败��生成单节点 fallback 计划

**测试** (4/4 通过):
- `test_p3_decompose_builds_dag` — 3 节点 DAG, acyclic, 拓扑排序正确
- `test_p3_taskgraph_no_cycle` — 有效 DAG + 环检测
- `test_p3_reflect_three_way` — 三种判定全走 heuristic fast path
- `test_p3_reflect_llm_path` — LLM 辅助反射路径

---

### A2: reflect 三路判定 ✅

**证据**: `crates/planner/src/lib.rs:155-255`

- `ReflectVerdict::Continue` — 继续当前计划
- `ReflectVerdict::Replan` — 重建 TaskGraph，重置计数器
- `ReflectVerdict::GiveUp` — 优雅终止

**Heuristic fast path** (无 LLM 调用):
- `consecutive_errors >= 3` → GiveUp
- `budget_remaining == 0` → GiveUp
- `budget_fraction <= 0.15 && steps_without_progress >= 2` → GiveUp
- `steps_without_progress >= 5` → Replan
- `consecutive_errors >= 2 && replan_count < 3` → Replan

**LLM slow path**: 仅在 heuristic 未触发时调用，分析执行状态返回判定词。

---

### A3: agent-core 集成 + 卡住检测 ✅

**证据**: `crates/agent-core/src/loop.rs`

- `AgentLoop` 新增字段: `planner: Arc<dyn Planner>`, `task_graph`, `plan_state`, `consecutive_errors`, `steps_without_progress`
- `do_plan()`: 构建 `PlanContext`（含 P2 retrieval_context + lsp_diagnostics），调用 `planner.decompose()`
- `do_observe()`: 始终走 Reflect（P3 设计）
- `do_reflect()`: 追踪 error/progress，构建 Observation，调用 `planner.reflect()`
  - Continue → Plan
  - Replan → 重置计数器，重新进入 Plan
  - GiveUp → Error 终止 (ok=false)
- `build_messages()`: 注入 TaskGraph 状态表 `[DONE]/[IN PROGRESS]/[FAILED]/[ ]`

**测试 `test_p3_stuck_replans_or_gives_up`** (接线验证):
- MockLlm 脚本使工具调用持续 error
- MockPlanner.reflect 返回 GiveUp
- 循环在 5 步内终止，`ok=false`

**测试 `test_p3_a3_e2e_stuck_detection_heuristic`** (端到端启发式验证):
- 真实 `DefaultPlanner` + 持续报错脚本
- `consecutive_errors` 跨 replan 周期累积（不再被清零）
- 第 3 轮 reflect 时 heuristic fast path 触发 GiveUp（`consecutive_errors >= 3`）
- `ok=false, steps=13`（远小于 budget 50）
- `replan_count=1`（有界：replan 后 errors 累积触发 GiveUp）

---

### A4: 子 Agent 委托 ✅

**证据**: `crates/agent-core/src/loop.rs:180-258`

- `spawn_sub_agent(task_id, task_desc, budget) -> JoinHandle<RunReport>`: 创建独立 AgentLoop 子实例
- `collect_sub_agent_results()`: 非阻塞收集已完成子 agent 的结果
- `active_sub_agent_count()`: 活跃子 agent 计数
- `do_plan()`: delegable 任务自动 spawn 子 agent，标记 InProgress
- `do_observe()`: 收集子 agent 结果，更新 TaskNode 状态

**测试 `test_p3_a4_sub_agent_delegation`**:
- TaskGraph 包含 `delegable=true` 节点
- 子 agent 被 spawn（depth=1）
- 任务节点状态更新为 `Completed`
- result 非空且 `ok=true`，steps > 0
- 子 agent 的 task_id 通过 `(String, JoinHandle)` 配对正确传递

**深度限制**: `depth >= 1` 的子 agent 不再 spawn 孙 agent，防止无界递归。

---

### A5: planner 抽象注入 ✅

**证据**: `crates/agent-core/src/loop.rs:126-128`

```rust
pub fn new(
    provider: Arc<dyn LlmProvider>,
    planner: Arc<dyn Planner>,  // ← P3: 注入点
    dispatcher: Arc<ToolDispatcher>,
    ...
)
```

**Provider 抽象验证**:
```sh
$ grep -rn "llm[-_]openai\|llm[-_]local\|llm[-_]cn\|OpenAiProvider\|OllamaProvider" crates/planner/
(exit: 1 — 零泄漏)
```

**测试 `test_p3_planner_injected_into_loop`**:
- MockPlanner 注入 AgentLoop
- `planner.decompose()` 被调用
- 循环正常完成

---

### A6: 集成测试 ✅

**证据**: `crates/service/tests/integration_test.rs`

- 全部 8 个集成测试通过
- SSE 事件流包含 `Reflection` 事件（P3 新增）
- 预算枯竭正确产生 Error 事件
- 双 provider 切换正常

---

## 4. 架构决策记录

### 4.1 Observe 后始终走 Reflect

P3 之前，`do_observe()` 仅在 `has_error=true` 时才走 Reflect。P3 改为始终走 Reflect，由 planner 的 `reflect()` 方法统一判断 Continue/Replan/GiveUp。这使 agent loop 的控制流更加统一。

### 4.2 空 TaskGraph 保留现有状态

`do_plan()` 中，如果 `planner.decompose()` 返回空图，不再覆盖现有 `self.task_graph`。这保护了 delegable 任务的状态（InProgress/Completed），避免在 replan 周期中丢失。

### 4.3 GiveUp → Error 阶段

`do_reflect()` 的 GiveUp 分支返回 `LoopPhase::Error(...)` 而非 `LoopPhase::Done`。这确保 `run()` 报告 `ok=false`。

### 4.4 子 Agent 共享 Provider

子 agent 通过 `Arc::clone()` 共享同一个 LLM provider。这避免了为每个子 agent 创建新的 provider 实例。

---

## 5. 关键变更文件

| 文件 | 变更 |
|------|------|
| `crates/agent-types/src/lib.rs` | +TaskStatus, TaskNode, TaskResult, TaskGraph, PlanContext, Observation, PlanState, ReflectVerdict |
| `crates/planner/src/lib.rs` | **新建** — Planner trait + DefaultPlanner (505 LOC) |
| `crates/planner/Cargo.toml` | **新建** |
| `crates/agent-core/src/loop.rs` | +planner 字段, do_plan/do_reflect 重写, spawn_sub_agent, collect_sub_agent_results, all_done 检查 |
| `crates/agent-core/Cargo.toml` | +planner.workspace |
| `crates/service/src/session.rs` | +Budget 存储, create_session/send_message budget 传递修复 |
| `crates/service/Cargo.toml` | +agent-types.workspace, planner.workspace |
| `crates/service/tests/integration_test.rs` | build_script 扩展（P3 LLM 调用增加） |
| `Cargo.toml` | +planner workspace member |

---

## 6. v2 修复记录（针对 P3 v1 闸门 🔴×2）

### 🔴 A3 — replan_count 无界 + 证据缺口
- **修复**: `replan_count` 移至 `run()` 初始化清零，`do_plan` 不再重置
- **修复**: Replan 分支不再清零 `consecutive_errors`（GiveUp 需跨周期累积）
- **新测试**: `test_p3_a3_e2e_stuck_detection_heuristic` — 真实 DefaultPlanner + 持续报错脚本，端到端证明启发式触发 GiveUp

### 🔴 A4 — 成功子 agent 结果合并不回来
- **修复**: `sub_agent_handles` 改为 `Vec<(String, JoinHandle)>` 配对存储，task_id 不依赖 summary 约定
- **修复**: 子 agent 的 summary 始终注入 task_id（成功/失败路径均覆盖）
- **修复**: 添加 `depth` 字段，`depth >= 1` 不再 spawn 孙 agent
- **测试升级**: 断言从 `InProgress|Completed` 升级为 `Completed + result.ok=true + steps>0`

---

## 7. 闸门判定

| 证据 | 要求 | 结果 |
|------|------|------|
| 🔴 A1 | planner crate + decompose | ✅ 4/4 测试 |
| 🔴 A2 | reflect 三路判定 | ✅ heuristic + LLM 路径 |
| 🔴 A3 | 卡住检测 (replan/give_up) + 端到端启发式证明 | ✅ v2 修复：replan 有界 + e2e 测试 |
| 🟡 A4 | 子 agent 委托 + depth 限制 | ✅ v2 修复：配对存储 + depth 硬限 |
| 🟡 A5 | planner 注入 + provider 抽象 | ✅ grep 验证 + 测试 |
| 🔵 A6 | 集成测试 + D2 + 报告 | ✅ 全部 38 测试 |
| 🔵 D2 | LOC/test 数据 | ✅ 6,895 LOC, 38 tests |

**闸门结论**: ✅ **v2 修复完毕，建议过闸**
