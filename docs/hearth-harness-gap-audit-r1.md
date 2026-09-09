# Hearth Harness Gap Audit R1（AUDIT-01 · 2026-08-27）

> **依据**：`docs/Hearth × Codex Harness 对标与可信委托强化任务书 v1.md` §2（AUDIT-01）+ 守门员批注 1-4。
> **方法**：源码直读（每个锚点 `wc -l`/grep 实测，不信批注行号外的二手描述）；VM 实机取证沿用 2026-08-27 v0.2.4 施工记录。
> **范围**：G1-G5 全量审查 + Round 1 施工项（G1 / G3-03）落地记录。G6-G9 按批注 3 归 Round 2+，此处给 AUDIT 结论与锚点。
> **基线**：v0.2.4 commit `12c94ad`（VM 门禁 351 passed 全绿）。

---

## G1 · Task Lifecycle Truth

### 现状（源码事实）

1. **loop 层终止路径已结构化**：`loop.rs` run 循环五类收尾各自埋 `summary.reason`——`deadline_exceeded`（loop.rs:2795，v0.2.4 H2）/ `budget_exhausted`（:2871）/ `verify_failed`（:2950,2963）/ `completed`（:2976）/ `error`（:2898,3008）。
2. **根因（G1 核心缺陷）**：CLI 投影层把 RunReport 压成二值——`run_local.rs:425` 旧代码 `"status": if r.ok { "completed" } else { "failed" }`，loop 层全部 reason 细节（deadline？预算？验证失败？provider 死？）在投影时**被吞**。用户看到的只有 `✗ Done`，对应任务书 §0.2 B"输出突然结束"。
3. `agent_error`（provider run 级失败）/ `agent_crashed`（panic 隔离）/ `cancelled`（Ctrl-C）三个手写 status 未走统一映射（run_local.rs:390,435,449）。
4. `waiting_for_user` 无显式终态字段，但 UI 层已有投影：inline approval `run_local.rs:619-720`（`⛔ approval — 批准? [y/N]`）、remote SSE `repl.rs:166-187`（30s 超时 auto-deny）。Turn 级 waiting 已可见；task 级挂起场景在 Round 1 的 run 内审批是阻塞的，无"run 结束但仍 waiting"中间态——该状态属 G2/G7 深化，Round 1 不动。
5. H8 执行报告（`report.rs` + `run_local.rs:498-530`）已承载 goal/steps/elapsed/tokens/tool_calls/written_files/approvals/reflections/errors——批注 1 判断成立：**Completion record 载体=扩展 report.rs，不另建系统**。

### 决策与施工（Round 1 已落地）

| 项 | 决策 | 施工 |
|---|---|---|
| G1-01 九态 | **ADAPT**（Codex 无公开等价结构，按任务书状态集自建） | 新模块 `agent-core/src/terminal.rs`：`TERMINAL_STATES` 九态封闭集 + `normalize_terminal_state(ok, reason)` 唯一映射权威 + `is_terminal_state` 校验 |
| G1-02 异常必映射 | **ADOPT** 思想（状态机收口） | run_local.rs 三分支（正常/agent_error/agent_crashed）+ cancelled 全部走 normalize；agent_crashed → aborted（区分异常中止与任务失败） |
| G1-03 Completion Summary | **ADAPT**（扩展 H8） | report.rs 加 `terminal_state`（九态原文+图标投影 ✓/✗/⏸/⏱/⛔）+ `remaining_work`（从 task_graph 未完成节点提取，`collect_remaining_from_graph`；无图兜底通用指引）；CLI 每轮结束渲染终态行（completed 带 goal 达成语；deadline 带 cap/用时/HEARTH_TASK_TIMEOUT_SECS 提示；failed 带 reason） |
| G1-04 负面测试 | **ADOPT** | ① `test_normalize_always_produces_valid_terminal_state`（9 reason × 2 ok 全组合 ∈ 封闭集）② `test_deadline_run_produces_deterministic_terminal_state`（run 集成：max_time_secs=0 → summary.reason=deadline_exceeded → 终态可确定，零 LLM）③ `test_provider_failure_run_terminal_state_failed`（Fatal provider 持续失败 → failed）④ `test_report_terminal_state_labels`（四终态图标投影） |

**关键修正（对任务书假设）**：任务书假设"完成态缺失"需要新建 Task/Turn 终态系统。源码核实：loop 层结构化收尾**已存在且完整**（v0.2.2 起逐版落地），真根因是**投影层二值化**。因此 Round 1 施工量远小于任务书预估——补映射层+投影，不建新状态机。此为"原假设部分不成立，实际根因更浅"的案例。

---

## G2 · Long-running Autonomous Execution

### 现状

1. `session_store.rs`：turn 快照（原子写 tmp+rename）+ task_graph 落盘（`<sid>.graph.json`）；resume 路径 `lib.rs:506-550` → `rebuild_agent` + `restore_history` + `restore_task_graph`（v0.1.6/WS11 真接线，v22 源码核验过）。
2. **Task 与 Loop 已部分分离**：process 死后 `hearth resume <sid>` 可重建（历史+任务图），但 **TaskGoal 语义对象不存在**——resume 后 goal 是 `"continue"` 字面量（lib.rs:530-534），agent 靠恢复的 history/graph 推断要继续什么。
3. G2-02 要求的 checkpoint 字段（constraints/acceptance_criteria/completed/remaining/next_action）：task_graph 有 nodes+status（部分覆盖 completed/remaining），其余无落盘。
4. G2-04：v0.2.4 H2 已给 deadline（900s 可调），但"提 timeout"之外的持久化 Task 对象确未做——任务书判断成立。

### 决策

- **G2-01/G2-02：ADAPT，DEFER 到 Round 2**。批注 2 成立：TaskGoal 必须先回答与现有 `Goal`（original+budget）/TaskGraph/restore_task_graph 的关系。AUDIT 结论：**TaskGoal 应扩展现有结构**（Goal 加 immutable original_goal 字段 + TaskGraph 已有节点态），不需要新对象；Full Checkpoint 字段里 acceptance_criteria/open_questions 依赖 planner 产出结构，属 B2 深化范围。Round 2 出独立施工单。
- **G2-03 LT-01 长程 benchmark：DEFER**（需真 LLM 长跑，预算纪律——无授权资金不烧 API；bench 骨架可先入 `bench/tasks/codex-parity/`，执行挂账）。
- **G2-04：ADOPT 已部分落地**（H2 deadline 是 deadline+recovery 模型的时间半边；recovery 半边=上两行）。

---

## G3 · Goal Continuity / Compaction

### 现状（Round 1 施工项）

1. v0.2.4 H3 已做：折叠前原文归档 `~/.config/hearth/archive/compacted.jsonl` + 摘要注入 grep 检索提示。**批注核实：单共享文件问题属实**——`archive_compacted_turns`/`archive_path`（context.rs:229-259）无 session 维度，多 session 堆同文件（检索串扰）。
2. "继续什么"问题的另一半：resume 时 goal="continue"（G2-2 条），当前靠 summary 消息（60 字符 goal+工具+写盘）撑语义——Round 1 不扩（归 G2 TaskGoal）。

### 决策与施工（Round 1 已落地）

- **G3-03：ADOPT，已施工**：`ContextManager` 加 `session_id` 字段（`set_session_id`，`AgentLoop::set_session_id` 同步绑定 loop.rs:842）；归档路径 `archive/<sid>.jsonl`（文件名安全过滤：非 `[a-zA-Z0-9_-]` → `_`，防路径逃逸）；未绑定回落共享文件（向后兼容）；摘要检索提示改指本会话归档（grep 不再命中别的会话）。
- **测试**：`test_archive_per_session_isolation`（负面：A/B 两会话归档内容互不混入；含 `../evil` 文件名安全用例）。
- G3-01/G3-02/G3-04：**DEFER 到 Round 2**（依赖 G2 TaskGoal 形态；摘要保留字段扩展同批）。
- G3-05 长会话负面测试：现有多轮集成测试烧 LLM，挂 LT-01 一起 DEFER；压缩语义层由 `test_compact_archives_original_turns`/`test_maybe_compact_folds_old_turns` 守住。

---

## G4 · Intent Preservation / Goal Drift

### 现状

1. **original_goal 无持久保存**：`ctx_mgr.state().goal` 被 `continue_turn` 每轮覆盖（REPL 每条新输入即新 goal）——任务书 G4-01 的"immutable original_goal"确实缺失。但注意语义分工：REPL 的多轮对话里"每条输入是新指令"本身合理，问题在**单轮 run 内** replan 不丢 goal（`goal.text` 全程 clone 传递，replan 不改 goal——源码核实成立）与**跨轮续做**时无原始目标基准（归 G2）。
2. replan 不修改目标：`do_reflect` replan 路径只重建 plan（planner MockP 场景核验），不改 goal.text。**G4-03 现状达标**。
3. P0-4（v0.2.4 H5）写前目标校验已给"写入偏离"警示（Observe 层）——G4-04 的写路径子集已覆盖。

### 决策

- **G4-01/G4-02：ADAPT，DEFER 到 Round 2**——落点在 G2 TaskGoal 扩展（`original_goal` immutable 字段），不在本轮单独动 `Goal` 结构（避免批注 2 警告的双轨）。
- **G4-04 goal_drift_detected：DEFER（Observe 层可先行挂账）**——检测信号需 original_goal 基准，Round 2 随 TaskGoal 一起出；强制暂停属 D 类须顶层出单（批注 3 已划定）。
- **G4-05：DEFER**（真 LLM 观察型测试，挂 LT-01）。

---

## G5 · Intent Understanding / Context Builder

### 现状

`build_messages()`（loop.rs:956-1042+）手工拼接：goal_requires_product 分流的 system_text（内联工作流/IDENTIFIER CONTRACT/TOOLS 清单）+ compaction + experience 注入 + Hearth.md + talent + plan 状态——**任务书判断成立，现场确认**（比批注写锚点 956-1042 更长，实测体量约 400 行级拼接逻辑）。

### 决策

- **G5-01/G5-02 ContextBuilder/Stable-Dynamic 分层：ADAPT，DEFER 到 Round 2**——批注 3/4 顺序正确且是硬约束：**必须先吃 G9 缓存数据（20-50 请求）再动 build_messages**，否则分层切错位置=白干。本审计已把 G5 定为 Round 2 第一优先（它是 G4-04/G3-01 注入 TaskGoal 的共同基础）。
- **G5-03 Hearth.md 层级作用域：DEFER**（现状单文件 cwd 优先+家目录兜底，loop.rs:124-129；inherit/append/override 解析器 Round 2-3）。
- **G5-04 Goal 进 Context：ADAPT，Round 2**——依赖 TaskGoal（G2）；Round 1 的 H3 归档提示+P0-4 校验已给"goal 可达性"打了两个补丁。

---

## G6-G9 · Round 2+ 审查摘要（锚点留存）

| 组 | AUDIT 结论 | 决策 | 关键锚点 |
|---|---|---|---|
| G6 ToolInvocation | tool 分发已超时/审批结构化（dispatcher.rs dispatch+InteractionState），但 lifecycle 事件只有 ToolCall/ToolResult 两点，缺 requested/started/timeout/cancelled 细分 | ADAPT，Round 2 | `dispatcher.rs:120-149`；复用 EnvelopedEvent（勿建第二事件系统——任务书 §11 已自我约束） |
| G7 Approval | inline approval 已真接线（run_local.rs:619-720 本地 y/N；repl.rs:166-187 SSE 30s auto-deny）；审批策略矩阵与 clarify 批量收集缺口属实 | G7-01/02 ADOPT 已存在；G7-03/04 矩阵+批量 = **D 类，Round 2 顶层出单**；G7-05 egress 审批 = T11 既有任务书，Round 2 | `api/lib.rs:155`；`loop.rs:1286-1301`（clarification 范例） |
| G8 Budget | deadline/steps/retry 齐备（H2/H4）；token_budget 观测缺（CostMeter 已累计但不驱动告警）；same-effect repeat guard 缺 | G8-01 观测层 Round 2 小改；G8-02 触发动作 D 类须顶层出单 | `llm-gateway/src/cost.rs` |
| G9 Cache | H6 仪表生产接线（Usage cache 字段+CostEntry 累计+cache_reported_calls），**零代码可开采** | 批注 4 采纳：Phase B 起每次真机任务顺手采集 request hash+hit/miss；Phase F 设计必须吃数据 | `cost.rs:20-40`；`llm-openai.rs:111-122` |

---

## Round 1 交付核对（对照 §45 统一格式）

| 项 | 交付 |
|---|---|
| G1 | terminal.rs（九态+唯一映射+3 测试）；run_local 三分支归一；report.rs 终态投影+remaining_work+2 测试；CLI 终态行（含 deadline 可行动提示） |
| G3-03 | 归档按会话隔离+文件名安全+1 隔离测试（负面） |
| AUDIT-01 | 本文档 |
| 门禁 | VM fmt/clippy/test 全绿（`~/t_gate.log` RC 行为准） |
| 残余风险 | ① normalize 对未知 reason 的兜底是 failed——若未来新增 loop 终止路径忘带 reason，会归入 failed 而非专属态（缓解：is_terminal_state 写入口校验+新增路径须同步映射表）② HOME/APPDATA 均无时归档落 `.hearth_archive/`（cwd 相对）——单机部署场景不触发 |
| 架构不变量 | 未破坏：三端口/事件契约只增不改（未加事件变体）/Observer 零执行权（report 仍为 CLI 本地投影）/planner 白盒（未动） |

## Round 2 建议派工（出单前摘要）

1. **G5 ContextBuilder**（第一优先——但开工前置条件：G9 采集 ≥20 请求的数据表）
2. **G2/G4 TaskGoal 扩展**（original_goal immutable + resume 语义 + checkpoint 字段；同单处理批注 2 的双轨问题）
3. **G7-03/G7-04/G8-02**（均为 D 类：顶层出审批策略矩阵/clarify 批量/repeat guard 触发动作的施工单）
4. G7-05 egress 审批（T11 既有单，可直接施工）
5. bench/tasks/codex-parity/ 骨架（C01-C10 用例定义先落，执行挂预算）
