# R2-B TaskGoal / Goal Persistence 顶层评审批示

**状态：有条件通过设计评审，暂不施工。**

经审阅 `Hearth Harness Gap Audit R1` 与本次 R2-B 设计单，确认设计总体方向成立：

* 不新建第二套 Task 状态系统；
* 复用现有 `Goal + TaskGraph + session_store`；
* TaskGraph 继续作为 Plan State 的唯一事实源；
* `remaining / completed / next_action` 采用派生而非重复存储；
* `normalized_goal` 暂缓；
* resume 保留用户当前 `continue` 指令，而不是粗暴替换为 original_goal。

以上原则批准。

但在进入施工前，必须完成以下设计修订。

---

## 1. original_goal 必须真正 immutable

当前设计存在冲突：

一处定义 original_goal “只写一次、不可变”，另一处又允许用户改目标后覆盖 original_goal。

**顶层裁决：禁止覆盖 original_goal。**

必须区分：

```text
original_goal
current_goal
goal_revision
```

语义：

```text
original_goal
= 任务最初建立时的目标，永久保留

current_goal
= 当前轮用户指令

goal_revision
= 当前目标修订序号
```

用户明确改变目标时：

```text
original_goal 不变
current_goal 改变
goal_revision++
```

并产生结构化事件：

```text
goal_changed
```

不得静默覆盖 original_goal。

原因：

original_goal 是 Goal Preservation / Goal Drift 的历史锚点。

---

## 2. taskgoal.json 与 graph.json 必须具有一致性规则

允许继续采用：

```text
<sid>.taskgoal.json
<sid>.graph.json
```

但两个文件独立原子写，不等于跨文件原子一致。

必须增加最小一致性机制，例如：

```text
state_revision
```

两个文件使用同一 revision。

resume 时：

```text
revision 一致
→ 正常恢复

revision 不一致
→ recovery path
→ 不得静默采用一份当作两份一致
```

允许采用最小实现，不要求数据库或复杂事务。

目标：

> 防止 crash 恰好发生在 taskgoal 与 graph 两次写盘之间时产生隐性状态分叉。

---

## 3. original_goal 初始化条件不得依赖 history.empty()

当前设计使用：

```text
history.is_empty()
```

作为第一次写入 original_goal 的条件。

改为：

> 任务/Session 的首次初始化生命周期条件。

原则：

```text
Task initialized
↓
original_goal absent
↓
write original_goal
```

而不是：

```text
history empty
```

因为 history 是数据状态，不是生命周期标识。

original_goal 必须在首次产生副作用之前完成持久化。

---

## 4. constraints 必须明确 provenance

第一版只允许采用具有确定来源的约束：

```text
Hearth.md
系统/CLI 明确指定 constraint
已完成 InteractionRequest 返回的用户约束
```

暂不要求从自然语言自由推断所有 constraints。

如果未来加入自动约束抽取，必须另有设计与验证。

---

## 5. acceptance_criteria 允许为空，但必须诚实区分 verification scope

批准当前设计：

```text
planner 有 criteria
→ 使用 criteria

planner 无 criteria
→ criteria = empty
→ 退化到现有 artifact-level verification
```

但：

```text
artifact verification passed
```

不得等价表达成：

```text
semantic task completion verified
```

报告必须能够区分至少：

```text
artifact verification
semantic acceptance verification
```

避免“文件存在/非空”被误报为完整任务语义已经满足。

---

## 6. next_action 必须 deterministic

`next_action` 由 TaskGraph 派生，不存储。

如果同时存在多个 ready node，必须规定稳定 tie-break：

```text
TaskGraph.topo_order
+
stable node id
```

保证：

同一 TaskGraph 状态
→ 同一 next_action。

禁止依赖 HashMap / Vec 遍历等非语义稳定顺序。

---

## 7. resume 语义批准，但 Task Continuity 必须进入 Runtime Context

批准：

```text
resume
↓
goal.text = "continue"
```

不要把 `goal.text` 替换为 original_goal。

但恢复时必须注入可信的 Task Continuity：

```text
Original Goal
Completed
Remaining
Constraints
Acceptance Criteria
Next Action
```

该信息属于 Runtime Context，而不是伪装成用户消息。

用户说：

```text
continue
```

表示当前操作指令；

Task Continuity 表示：

```text
系统恢复后的真实任务状态
```

两者必须保持语义分离。

---

## 8. TaskGoal 不是新的事实源

最终必须保持：

```text
original_goal / constraints / criteria
→ Task semantics

TaskGraph
→ Plan semantics

current_goal
→ current user instruction

RunState
→ current run execution state
```

不得出现：

```text
TaskGoal
+
TaskGraph
+
Goal
```

三套互相竞争的任务真相。

`taskgoal.json` 是持久化载体，不是新的第四套任务模型。

---

## 9. 本轮施工范围

R2-B 允许施工：

```text
original_goal 持久化
constraints 持久化
acceptance_criteria 持久化
revision consistency
resume restore
minimal Task Continuity injection
derived completed/remaining/next_action
```

暂不施工：

```text
normalized_goal
node-level acceptance criteria
完整 ContextBuilder
Goal Drift 强制暂停
复杂自动约束抽取
```

后续完整 ContextBuilder 必须复用本轮 Task Continuity 数据，不得再建第二套注入路径。

---

## 10. 必须增加的负面测试

施工完成后至少增加：

### T1 Original Goal Immutable

```text
Goal A
↓
用户改成 Goal B
```

验证：

```text
original_goal == A
current_goal == B
goal_revision++
```

---

### T2 Crash Between Persistence

模拟：

```text
graph write
↓
taskgoal write
↓
crash
```

验证：

resume 不得静默获得互相矛盾的状态。

---

### T3 Resume

```text
Goal A
↓
完成若干节点
↓
kill
↓
resume
```

验证 Agent 能够明确知道：

```text
原始目标 A
已完成
剩余
下一步
```

---

### T4 Compact + Continue

```text
Goal A
↓
执行若干步骤
↓
compact
↓
continue
```

不得出现：

```text
“继续什么？”
```

---

### T5 Multiple Ready Nodes

构造多个同时 ready 的 TaskGraph。

验证：

```text
next_action deterministic
```

---

### T6 Empty Acceptance Criteria

验证：

```text
criteria empty
```

时：

```text
artifact verification
≠
semantic acceptance verified
```

报告不得误导。

---

### T7 First-Run Initialization

模拟：

```text
session created
↓
original_goal persisted
↓
first side effect
```

验证 original_goal 不依赖 `history.empty()`。

---

## 11. 门禁

施工完成后必须：

```text
cargo fmt --check
clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

全部 RC=0。

并在 VM 真机验证：

```text
fresh run
resume
compact
goal revision
crash recovery
```

门禁以：

```text
~/t_gate.log
```

为准。

---

# 最终裁决

R2-B：

**设计方向通过。**

进入施工前，必须修订：

1. original_goal immutable；
2. taskgoal/graph consistency；
3. 首次初始化条件；
4. constraint provenance；
5. verification scope；
6. next_action deterministic。

完成修订后可施工。

本轮目标不是“再造一个 Task System”，而是：

> **让 Hearth 在任何 run、replan、compact、resume 之后，都仍然知道这个任务最初为什么存在、现在做到哪里、接下来应该做什么。**

这才是 Task Continuity 的真正完成标准。

---

# 守门员批注（2026-08-28 · 源码+VM 实证后补充，批示 1-10 全部同意，另加 6 条施工约束）

> R2 已施工部分独立验收：R2-F egress 审批（`loop.rs:871` loop 层分类 / `scheduler.rs:158` 运行时白名单 / `run_local.rs:309` config 复用回调——批注 5 两坑全合规）✅；R2-A 采集器 + 24 行数据 ✅；commit `5b9ff93`/`334739b` 在库；VM `~/t_gate.log` CLIPPY_RC=0 / TEST_RC=0 ✅。**批示 1–10 照办；以下 6 条与批示同效力。**

## 补充 1（最重要）：批示第 7 条与 R2-A 数据的冲突调和——Task Continuity 注入位置写死

批示第 7 条要求 Task Continuity "进入 Runtime Context，不得伪装成用户消息"——**字面理解若 = 注入 system_text，则与 R2-A 实测头号根因直接冲突**（TaskGraph 状态注入 system 正是 stable prefix 每步失效的元凶）。裁决调和如下，R2-D 按此施工：

```text
Task Continuity 块放 history 尾部的 system-status 标签块（dynamic suffix 区）：
  "[System Status / Task Continuity] 原始目标: … / 已完成: … / 剩余: … / 下一步: …"
语义 = 系统状态（明确标注，非用户消息、非 agent 自述）
缓存 = 在尾部，不破 stable prefix
禁止写进 stable system_text（那里只能放不随步骤变化的内容）
```

这与 R2-C 的第一决策（动态区块从 system 挪到 history 尾部）是同一条原则的两个应用，后续 ContextBuilder 必须把本块纳入 dynamic suffix 统一管理（批示第 9 节"复用本轮注入路径"即指此）。

## 补充 2：goal_changed 事件契约红线（批示第 1 条引入新事件变体）

新增 `goal_changed` 变体遵守既有红线：契约**只增不改**、FE 未知 kind 宽容（禁白屏）、Observer 规则引擎对未知事件默认忽略不炸、seq 单调语义不动。设计单原提"civ note 记录覆盖"与批示要求"结构化事件"**合并执行**：事件进事件流，note 进 civ，两者都要。

## 补充 3：批示第 2 条 revision 一致性的 recovery path 钉死最小规则

"revision 不一致 → recovery path" 需确定行为，R2-D 按此实现（也是 T2 负面测试的断言对象）：

```text
revision 一致          → 正常恢复
revision 不一致        → original_goal 照常采用（immutable，冲突面最小）
                       + graph 照常采用（plan 唯一真相）
                       + constraints/criteria 照旧恢复，但 Task Continuity 块
                         标注 "[constraints/criteria: revision mismatch, may be stale]"
                       + 发 warning tracing
不阻断 resume（恢复优先），不静默（有标注有日志）
```

最小实现即可（taskgoal.json 与 graph.json 各存一份 `state_revision`，写入时同值递增），不引入锁或事务。

## 补充 4：批示第 5 条 verification scope 的落地锚点

`report.rs` verification 段拆两字段：`artifact_verification`（现状 WS13 结果）与 `acceptance_verification: "none"|"passed"|"failed"`（**criteria 为空时恒为 "none"，绝不写 passed**）。completed 判定变化：criteria 空 → 沿用 artifact 判定；criteria 非空 → 须 acceptance passed 才可 completed。**注意**：`terminal.rs` 的 `normalize_terminal_state` 不动、不塞 criteria 逻辑——改的是上游"何时判 completed"的输入，终态映射层保持纯函数。

## 补充 5：R2-E 零代码结论的追认与口径收束

执行窗口"现有 ToolCall/ToolResult + Envelope 可推导完整生命周期，新增事件变体 ROI 不足"——**推导成立，予以追认**。但派工令 R2-04 要求的 8 态/时间戳强约束仍需落：口径 = **Runtime 内部持有 ToolInvocation 派生结构（内存态，从现有事件推导），对外事件流不加变体**。防止执行窗口把"零代码结论"理解成"什么都不做"。

## 补充 6：R2-A 采集器覆盖缺口（本轮发现，v2 必修）+ 测试预算口径

**实测：telemetry 只接在 loop 的 plan-chat 出口（`loop.rs:1907-1915`），而 planner 直接持有 provider 自行 chat（`planner/lib.rs:246`）——decompose 与每步一次的 reflect 调用全部绕过采集**。执行报告"全部 LLM 请求必经"不成立。后果与修正：
- `57.6%` 命中率是**仅 plan 相位的有偏子集**，R2-C 引用时必须标注"下界/有偏"，不得当全量基线；
- `system_hash 20 请求 7 唯一值` 的定性结论在 plan 相位内独立成立（system 不稳定实锤），可用；
- **采集器 v2：钩子下沉到 provider/gateway 层**（llm-openai chat 入口或 llm-gateway 装饰器，一处覆盖 loop+planner 全部出口），R2-C 设计定稿前用 v2 重采一轮校准。
- 测试预算：批示 T1/T3/T4/T5/T6/T7 全部可用 MockLlm/llm-replay 断言，不烧真预算；批示第 11 节 VM 真机 5 项中 crash recovery 用 kill 实测，其余可本地模拟。

## 施工授权

批示 1–10 + 补充 1–6 全部落定，**R2-D（TaskGoal 施工）即日可开工**，交付格式按母任务书 §45；门禁 fmt/clippy/test RC=0 + `~/t_gate.log` 为准 + VM 真机复测。

# END
