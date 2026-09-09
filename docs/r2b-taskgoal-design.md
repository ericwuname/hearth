# R2-B 设计单：TaskGoal / Goal Persistence（待顶层评审 · 2026-08-28）

> **状态**：设计产出，**未施工**（批注 3：设计单须回评审通过后才进 R2-D）。
> **作者**：执行窗口（GLM 5.3）· 供顶层（用户 + ChatGPT）晨间评审。
> **基线**：v0.2.5 / commit `4a519a7`。
> **设计原则**（派工令 §三 + 批注 2）：不新建第二套 Task 状态系统；在现有
> `Goal` + `TaskGraph` + `session_store` 上做最小增量扩展。

---

## 一、源码事实基础（全部实测锚点）

| 既有构件 | 位置 | 现状 |
|---|---|---|
| `Goal { text, budget }` | agent-types/lib.rs | run 级短命对象，REPL 每轮被 continue_turn 覆盖 |
| `RunState { goal, budget, history, scratch, steps_used, tokens_used }` | agent-types | ContextManager 持有，单 run 生命周期 |
| `TaskGraph { nodes: Vec<TaskNode> }`，TaskNode{id, description, deps, status, delegable, result} | agent-types/lib.rs:22-32 | planner decompose 产出；**已持久化**（`<sid>.graph.json`） |
| resume 恢复 | codex-cli/lib.rs:506-550 | restore_history + restore_task_graph 真接线 |
| resume 后 goal | lib.rs:530-534 | **退化为 "continue" 字面量**——本设计单要治的核心 |
| compaction 摘要 | agent-core/context.rs | 保留 60 字符 goal + 工具/写盘清单；original_goal 无单独保存 |
| 归档 | context.rs（v0.2.5） | `archive/<sid>.jsonl` 按 session 隔离 |

## 二、八字段归属逐项作答（派工令第四节要求）

| 字段 | 持有者 | 持久化 | resume 恢复 | 设计依据 |
|---|---|---|---|---|
| **original_goal** | `RunState` 新字段 `original_goal: Option<String>`；AgentLoop 首次 run（history 为空分支）写入，continue_turn **不覆盖**（immutable 由"只写一次"保证） | `<sid>.taskgoal.json`（新文件，原子写 tmp+rename，同 graph.json 模式） | resume 时读回 → 同时写入 ctx_mgr 与作为本轮 goal.text 的一部分（见四） | 目标漂移的锚点；60 字符摘要截断不再伤害它 |
| **normalized_goal** | **不设独立字段**。理由：normalize 无确定性收益（谁 normalize？LLM 多一跳；规则式无能力），且 planner 的 gaps/假设机制（B2）已承担"意图结构化"职能。若未来实证需要，由 planner 产出后放 TaskGraph root 节点 description——**DEFER，留 reentry 条件**：cache/意图测试显示 goal 语义丢失是主要失败原因时再启动 | — | — | 守门原则：只降 friction 才做 |
| **constraints** | `RunState.constraints: Vec<String>`；来源：Hearth.md 既有加载（loop.rs:124-129）+ 用户消息中的显式约束（P0-4 机制的自然扩展） | 并入 `<sid>.taskgoal.json` | 恢复后注入 build_messages（G5-04 前置） | Hearth.md 每次构建都重读文件，跨进程约束已由它覆盖；运行中用户口头约束需要这个字段兜住 |
| **acceptance_criteria** | `RunState.acceptance_criteria: Vec<String>`；来源：planner decompose 时让 planner 显式输出（PlanContext 已有 planner 对话通道），用户后续补充追加 | 并入 `<sid>.taskgoal.json` | 恢复；**消费点 = WS13 verify 门禁的扩展**（R2-09 依赖项：completed ⇔ criteria 全过） | 现有 verify 只查"写盘文件存在非空"——criteria 是它的语义升级 |
| **current_plan** | **TaskGraph 已承载**（nodes+deps+status 就是 plan）——不新增字段 | 已有 `<sid>.graph.json` | 已有 restore_task_graph | 批注 2 双轨问题的直接回答：plan 的真相只有一份 |
| **completed** | **TaskGraph 派生**：`nodes.iter().filter(status==Completed)`；**不存**（派生数据存了必漂移） | — | 随 graph 恢复 | 单一事实源原则 |
| **remaining** | **TaskGraph 派生**：非 Completed 节点；v0.2.5 `collect_remaining_from_graph`（report.rs）已按此实现——先例已立 | — | 随 graph 恢复 | 同上 |
| **next_action** | **TaskGraph 派生**：拓扑序第一个 deps 全 Completed 且自身非 Completed 的节点；不存 | — | 派生 | 同上；TaskGraph::topo_order 已有（agent-types） |

## 三、对批注 2（双轨问题）的正式作答

**结论：TaskGoal 不是新 struct，是"一个新字段组 + 一份新持久化文件 + 派生函数"的组合**：

```text
新增物（全部最小增量）：
1. RunState 三个字段：original_goal / constraints / acceptance_criteria（Vec<String>）
2. session_store 两个函数：save_taskgoal / load_taskgoal（<sid>.taskgoal.json，原子写）
3. 派生函数（agent-types 或 agent-core）：next_action(graph) / completed(graph) / remaining(graph)
   —— report.rs 的 collect_remaining_from_graph 迁移/复用
4. resume 路径 5 行：load_taskgoal → restore
明确不做：
- 不新建 TaskGoal struct（与 Goal/TaskGraph 并存即双轨——批注 2 警告的正是这个）
- 不动 TaskNode 结构（criteria 放 RunState 级而非节点级：验收是任务级语义）
- 不删 Goal/不改 continue_turn 语义（current_goal 仍是"本轮指令"——
  original_goal 与 current_goal 分工：前者是锚，后者是轮内指令）
```

## 四、resume 语义设计（本设计的验收核心）

```text
hearth resume <sid>
  ↓ load_turns + load_graph（现状）
  ↓ load_taskgoal → original_goal / constraints / criteria 恢复
  ↓ goal.text 仍为 "continue"（用户输入不变）
  ↓ build_messages 注入（G5-04 的最小提前实现，本设计内完成）：
     "[Task Continuity] 原始目标: {original_goal}
      已完成: {derived completed titles}
      剩余: {derived remaining titles}
      约束: {constraints}——继续推进剩余工作"
  ↓ Agent 回答"继续什么"时自带答案（G3-05 负面测试的正面依据）
```

compaction 协同：摘要消息保留 original_goal 全文（context.rs summarize 时从
taskgoal 字段取，不受 60 字符截断限制）——compact 后 G3-05 测试可过。

## 五、施工量与风险

| 项 | 估计 |
|---|---|
| agent-types 字段 | +30 LOC（含 serde default 兼容旧会话文件） |
| session_store taskgoal | +50 LOC（复制 graph.json 模式） |
| AgentLoop 首写/恢复/注入 | +60 LOC |
| 派生函数 + report 复用 | +40 LOC |
| 测试（负面先行） | +5 测试：original_goal immutable / resume 恢复 / next_action 派生 / 注入可见 / criteria 空兼容 |
| 合计 | ~180 LOC + 5 测试，单窗口一轮可完成 |

**风险**：
1. acceptance_criteria 依赖 planner 输出格式扩展——planner 瘦身原则下必须走"可选草案"（planner 不给就空，verify 门禁退化为现状文件级校验），不阻塞。
2. original_goal "immutable" 的边界：用户显式说"换个目标"时须允许覆盖——设计为：覆盖时必须记录覆盖事件（civ note），不是静默替换。
3. 旧会话文件无 taskgoal.json——load 缺文件返回默认空（同 load_graph 模式），不炸。

## 六、评审问题清单（请顶层重点裁决）

1. **criteria 放任务级（本设计）还是节点级？** 节点级更精细但 planner 产出租价大增；本设计选任务级。
2. **resume 时 goal.text 是否直接替换为 original_goal？** 本设计选"保持 continue + 注入 original_goal"（保留用户续做指令的显式性）。
3. **normalized_goal 的 DEFER 判定**是否同意？（reentry 条件已写明）
4. R2-D 施工排序：TaskGoal 先于 ContextBuilder 大重构（本设计是 G5-04 的最小提前版）——是否同意先小后大？

---

# 修订 v2（顶层批示后 · 2026-08-28 · 已随 R2-D 施工落地）

> 顶层批示 1-10 + 守门员补充 1-6 全部采纳，以下为逐条修订记录与实际实现锚点。

## 修订 1（批示 1）：original_goal 三层语义 + immutable 铁律
- `RunState.original_goal: Option<String>`——**写入一次，永不覆盖**（`apply_turn_goal`：original absent 才写）
- `goal`（current_goal）每轮更新；`goal_revision` 在 current_goal 变化时 ++ 并 emit `GoalChanged{revision, new_goal}`（契约只增，api/agent-runtime/CLI 三层接线；civ note 双写挂 CivWriter 既有路径）
- 实现：agent-types RunState 4 新字段（serde default 兼容旧文件）；loop.rs `apply_turn_goal`

## 修订 2（批示 2 + 补充 3）：state_revision 一致性
- `save_taskgoal` / `save_graph_with_revision` 同值写入；两文件各带 `state_revision`
- resume 校验：不一致 → **recovery path**（original 照常 + graph 照常 + constraints 标注 "[revision mismatch, may be stale]" + warning tracing；不阻断不静默）
- 旧格式裸 graph 兼容（rev=0）；最小实现无锁无事务
- 实现：session_store.rs 四函数 + lib.rs resume 链

## 修订 3（批示 3）：初始化条件
- `apply_turn_goal`/`init_taskgoal`：**original absent → 写**（生命周期条件）；CLI 在 spawn 前（首次副作用前）持久化 taskgoal（run_local.rs init 块）
- 测试 T7 锁定：history 非空 + original absent（旧会话）同样初始化

## 修订 4（批示 4）：constraints provenance
- 第一版只收：Hearth.md（system 注入，不改）/ CLI 显式 / InteractionRequest 答复；**不做自由文本推断**；本轮写入通道 = 恢复值或空

## 修订 5（批示 5 + 补充 4）：verification scope
- report.rs 拆两字段：`artifact verification`（WS13 现状）与 `acceptance_verification: "none"|"pending"|"passed"|"failed"`
- **criteria 空恒 "none"，绝不写 passed**；非空未确认 = "pending"（骨架——确认通道待 planner 扩展单，本轮不可达路径由 T6 锁定）
- `normalize_terminal_state` 保持纯函数不动（补充 4 红线）

## 修订 6（批示 6）：next_action deterministic
- `TaskGraph::next_action_deterministic`：排序键 (拓扑深度, node id)；`completed_titles`/`remaining_titles` 派生函数同批落地
- 测试 T5 锁定：diamond 图多 ready 同状态同结果 + 深度优先 + id tie-break

## 修订 7（批示 7 + 补充 1）：Task Continuity 注入位置
- **history 尾部 Role::System 标签块**（dynamic suffix 区），**禁止进 stable system_text**——与 R2-A 实测根因（TaskGraph 状态注入 system = 前缀失效元凶）调和：同一条原则的两个应用
- 块内容：原始目标/已完成/剩余/下一步/约束/验收标准；后续 ContextBuilder 复用本块（批示 9"不得再建第二套注入路径"）

## 负面测试落地（批示 10）
T1 immutable / T2 revision mismatch 可检测+旧格式兼容 / T3 resume 四要素+System 角色 / T4 compact 后 continuity 存活 / T5 deterministic / T6 verification scope / T7 初始化条件——**7/7 全部落地**（agent-types r2d_tests / loop.rs tests / session_store r2d_tests）
