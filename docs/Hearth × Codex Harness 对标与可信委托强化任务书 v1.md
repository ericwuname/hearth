# Hearth × Codex Harness 对标与可信委托强化任务书 v1

**项目**：Hearth / `hearth-rs`  
**任务书版本**：v1  
**目标基线**：Hearth v0.2.4 / commit `12c94ad`  
**日期**：2026-08-27  
**执行方**：智谱 GLM 5.3  
**任务性质**：Harness 对标、可信委托强化、真实任务体验优化  
**前置状态**：v0.2.4 H1–H10 已完成，VM 门禁 `fmt/clippy/test` 全部 RC=0，351 tests pass  
**目标**：不是复制 Codex，而是吸收 Codex 已证明有效的 Harness 设计，解决 Hearth 当前真实使用中暴露出的任务中断、完成态不清、目标漂移、上下文失忆、长程执行不足、交互摩擦与效率问题。

---

# 0. 主理人指令

## 0.1 本任务不是“把 Hearth 做成 Codex”

本任务的目标不是：

```text
Hearth
↓
复制 Codex
↓
Codex clone
```

而是：

```text
Codex 已经证明有效的工程设计
            ↓
      提取解决的问题
            ↓
      分析 Hearth 当前差距
            ↓
      用 Hearth 自己的架构实现
```

Hearth 必须保留自身的：

```text
Plan / Act / Observe / Reflect 白盒 Loop
Observer 第三权
G0–G3 基因体系
constitution
Experience
Multi-provider
Project Sync
Identity
Civ
Hearth 自有 Event Contract
```

Codex 只是 benchmark / reference，不是架构权威。

---

# 0.2 当前最重要的问题定义

当前用户真实体感已经明确暴露出以下问题：

### A. 任务执行中途莫名中断

用户不知道：

```text
任务还在运行？
已经暂停？
失败？
退出？
完成？
```

---

### B. 任务完成后没有明确完成态

用户不知道：

```text
“模型不说话了”
```

究竟意味着：

```text
DONE
还是
FAILED
还是
ABORTED
还是
工具超时
还是
Agent loop 异常退出
还是
CLI 失去显示
```

用户必须猜。

这是当前最高优先级问题之一。

---

### C. 长程任务理论存在，实际体验不成立

理论上：

```text
Agent + 授权 + 长时间运行
```

应该能够：

```text
自主探索
持续执行
自行验证
最终交付
```

但真实使用中：

```text
做几步
↓
中断
↓
用户回来
↓
“继续”
```

这不满足：

> “放心走开。”

---

### D. Context Compaction 后当前任务出现“失忆”

具体症状：

```text
同一个 session
↓
前面已经规划任务
↓
context compact
↓
用户：
继续
↓
Agent：
继续什么？
```

Agent 可以读取其他历史信息，却无法可靠恢复当前任务。

因此：

> 当前 Task State 没有成为 Context 的一等公民。

---

### E. Agent 容易 Goal Drift

真实体感：

```text
用户给出初始目标
↓
Agent 开始规划
↓
执行过程中发生失败
↓
replan
↓
继续若干轮
↓
Agent 忘记最初目标
```

用户需要持续提醒：

> “你最开始到底要做什么。”

---

### F. 意图理解弱于 Codex

用户体感：

> Codex 很容易理解我想做什么。

而 Hearth 往往需要：

```text
用户说得更详细
+
更多上下文
+
更多纠偏
```

否则容易：

```text
默认假设错误
偏离原始意图
规划正确但执行中逐渐漂移
```

不能先假定是模型能力问题。

由于已有证据显示同模型在 Codex 上表现明显更好，必须优先排查：

```text
system prompt
context composition
goal state
tool schema
planning
tool result
retry
verification
session continuity
```

而不是先更换模型。

---

### G. Token Cache Efficiency 待实证

当前不能直接假设：

```text
GLM / 其他平台 cache hit 95%
```

等于 Hearth 应达到 95%。

必须通过真实 provider telemetry 和 request-body 对比建立事实。

目标：

```text
先测量
↓
再定位
↓
再优化
```

禁止盲猜。

---

# 1. 架构核心结论

Hearth 当前已经具备：

```text
Agent Loop
Sandbox
Approval
Session
Replay
Memory
Experience
Observer
Project Sync
Verification
Multi-provider
```

因此当前不是：

> “有没有 Harness？”

而是：

> **现有 Harness 是否已经形成低摩擦、可持续、可恢复、可验证的可信委托闭环。**

当前核心闭环目标：

```text
User Goal
   ↓
Understand
   ↓
Plan
   ↓
Act
   ↓
Observe
   ↓
Clarify / Approve（必要时）
   ↓
Continue
   ↓
Replan（必要时）
   ↓
Verify
   ↓
Deliver
   ↓
Explicit Completion
```

任何一环出现：

```text
静默退出
目标丢失
状态不明
工具重复副作用
用户被迫手工救援
```

都视为可信委托缺陷。

---

# 2. 不允许直接施工：第一阶段必须先做源码级 Gap Audit

## AUDIT-01：五大核心问题真实根因审查

**优先级：P0**

执行方必须先审查源码，不得直接修改代码。

必须审查：

```text
G1 Task Lifecycle Truth
G2 Long-running Autonomous Execution
G3 Goal Continuity / Compaction
G4 Intent Preservation / Goal Drift
G5 Execution Efficiency / Prompt Cache
```

每项必须输出：

```text
1. Hearth 当前真实实现
2. 用户可观察到的症状
3. 相关 event 流
4. 状态流
5. 调用链
6. 源码根因
7. 是否与已有任务书假设一致
8. Codex 对应机制
9. Codex 机制解决的原始问题
10. 是否值得吸收
11. ADOPT / ADAPT / DEFER / QUARANTINE
12. 修改范围
13. 预计新增 LOC
14. 测试成本
15. 维护成本
16. 是否属于 D 类新控制流
```

注意：

> 本任务书里的根因判断是方向性假设，不是事实。源码优先。

如果假设与源码不符：

必须明确写：

```text
原假设不成立。
实际根因：
XXX
```

禁止机械执行。

---

# 3. G1：Task Lifecycle Truth

## G1-01：建立明确的 Task / Turn Terminal State

**P0**

当前用户最大的体验问题：

> “到底做完没有？”

必须从后端解决。

建议至少形成：

```text
starting
running
waiting_for_user
paused
completed
failed
aborted
cancelled
deadline_exceeded
```

其中：

```text
completed
```

只能在最终验证成功后进入。

禁止：

```text
LLM 最后说了一句 Done
=
Task completed
```

---

## G1-02：所有异常退出必须产生明确终态

以下情况不得静默：

```text
provider error
tool timeout
deadline
budget exhaustion
panic
process termination
approval timeout
user cancel
stream EOF
network failure
```

最终必须映射成结构化状态。

例如：

```text
TurnFailed
TaskFailed
TurnAborted
TaskCancelled
TaskCompleted
```

---

## G1-03：Completion Summary

P0。

任务完成后后端必须产生结构化 completion record。

至少包含：

```text
goal
status
steps_used
elapsed
tokens
tools_used
files_changed
verification
remaining_work
failure_reason
```

CLI 再投影。

用户至少应该明确看到：

```text
✓ Completed
```

或：

```text
✗ Failed
```

或：

```text
⏸ Waiting for your input
```

而不是：

```text
输出突然结束
```

---

## G1-04：禁止“无终态结束”

建立负面测试：

构造：

```text
tool failure
provider failure
deadline
approval timeout
cancel
EOF
```

所有情况下：

```text
Task Status
```

必须可确定。

禁止：

```text
unknown / silent exit
```

---

# 4. G2：Long-running Autonomous Execution

## G2-01：Task 与 Loop 分离

P0。

明确：

```text
Task
≠
one Agent Loop process
```

Task 必须拥有持久化状态：

```text
TaskGoal
Progress
Plan
Completed
Remaining
NextAction
Checkpoint
```

Loop 只是 Task 的一次执行载体。

因此：

```text
process A dies
↓
task remains
↓
process B resumes
```

必须自然成立。

---

## G2-02：Long Task Checkpoint

P0/P1。

至少保存：

```text
goal
normalized_goal
constraints
acceptance_criteria
current_plan
completed_steps
remaining_steps
next_action
important_artifacts
open_questions
```

不是只有：

```text
chat history
```

---

## G2-03：无用户介入长程任务测试

新增真实 benchmark：

```text
Task LT-01
```

用户仅输入一次：

> 完成一个中等复杂工程任务。

之后用户：

```text
不再说话
```

Agent 必须：

```text
自主探索
自主执行
自主测试
自主修复
自主验证
自主交付
```

允许的唯一人工介入：

```text
真正需要授权
真正需要澄清
```

而不是：

```text
“继续”
```

这种人为续命。

---

## G2-04：长任务运行不能靠单纯提高 timeout

禁止：

```text
900s
→
3600s
→
无限
```

把长程能力理解为“更长超时”。

正确模型：

```text
Persistent Task
+
Checkpoint
+
Deadline
+
Recovery
+
Goal State
+
Explicit Completion
```

---

# 5. G3：Goal Continuity / Context Compaction

这是当前最高价值的后端项目之一。

## G3-01：建立 TaskGoal 一等对象

建议：

```rust
TaskGoal {
    original_goal,
    normalized_goal,
    constraints,
    acceptance_criteria,
    current_plan,
    completed,
    remaining,
    next_action,
}
```

要求：

```text
Plan
Act
Observe
Reflect
Replan
Compact
Resume
```

均不得丢失 TaskGoal。

---

## G3-02：Compact 不允许删除当前任务语义

Compaction 必须保证至少保留：

```text
Original Goal
Current Goal
Important Constraints
Acceptance Criteria
Current Plan
Completed Work
Remaining Work
Current Step
Next Action
Open Questions
Important Artifacts
```

---

## G3-03：历史 Archive 与 Active Task 分离

正确模型：

```text
Active Context
    +
Task State
    +
Archive
```

而不是：

```text
History
↓
compact
↓
summary
↓
everything important lost
```

H3 已经完成：

```text
compacted.jsonl
```

归档。

但当前实现把多个 session 堆进一个共享归档文件。

必须补：

```text
session_id
```

至少做到逻辑隔离。

更优：

```text
archive/
  <session_id>.jsonl
```

---

## G3-04：Recall 不是唯一解决方案

不能只做：

```text
recall_history
```

然后认为完成。

真正必须做到：

```text
用户：
继续

Agent：
知道当前要继续什么。
```

Recall 是补充能力。

Task State 才是根本。

---

## G3-05：负面测试

构造：

```text
长任务
↓
建立明确目标 A
↓
至少完成 3 个步骤
↓
触发 compact
↓
用户不重新解释
↓
“继续”
```

验收：

Agent 必须能够：

```text
复述当前任务
说明已经完成什么
说明剩余什么
继续下一步
```

禁止：

```text
“继续什么？”
```

---

# 6. G4：Intent Preservation / Goal Drift

## G4-01：原始 Goal 永久保存

必须存在：

```text
original_goal
```

并且：

```text
immutable
```

不可被 replan 覆盖。

---

## G4-02：Normalized Goal

允许系统生成：

```text
normalized_goal
```

但必须能够回溯：

```text
normalized_goal
← derived from
original_goal
```

---

## G4-03：Replan 不得修改任务目标

允许：

```text
Plan A
↓
失败
↓
Plan B
```

不允许：

```text
Goal A
↓
失败
↓
Goal B
```

除非：

```text
用户明确改变目标
```

或者：

```text
系统发出 clarification
用户确认
```

---

## G4-04：Goal Drift Detection

建议加入监测：

```text
当前计划目标
vs
original_goal
```

出现显著偏离时：

```text
goal_drift_detected
```

注意：

这里首先做：

```text
Observe / Warning
```

若需要强制暂停或修改执行控制流，属于 D 类，必须经过顶层规则确认。

---

## G4-05：真实测试

同一任务中制造：

```text
工具失败
编译失败
测试失败
错误路径
```

观察：

Agent 是否最终仍在解决 Original Goal。

---

# 7. G5：Intent Understanding / Context Builder

## G5-01：建立统一 Context Builder

这是当前 S 级项目。

不要继续在：

```text
build_messages()
```

里堆：

```text
constitution
talent
Hearth.md
tools
experience
...
```

建议形成：

```text
ContextSource
ContextLayer
ContextSnapshot
ContextBuilder
```

至少分：

```text
Stable
Project
Task
Memory
Experience
Dynamic
Recovery
```

---

# 8. G5-02：Stable Prefix / Dynamic Suffix

目标：

```text
Stable Prefix
----------------
system instructions
constitution
stable tool schema
project rules
stable capabilities

Dynamic Suffix
----------------
history
tool results
current input
current task delta
```

稳定区域尽量在会话期间保持字节稳定。

---

# 9. G5-03：Hearth.md 层级作用域

当前已经有 Hearth.md，不允许重新创建重复机制。

真实目标：

```text
Global
 ↓
Project
 ↓
Subdirectory
```

定义：

```text
inherit
append
override
```

必须有 deterministic resolution。

---

# 10. G5-04：Goal 进入 Context Builder

每次模型调用必须有可控方式获得：

```text
Original Goal
Current Goal
Constraints
Acceptance Criteria
Current Progress
Next Action
```

不能只依赖历史对话。

这是解决：

```text
理解偏差
Goal Drift
Compaction 失忆
```

的共同基础。

---

# 11. G6：Tool Runtime

## G6-01：统一 ToolInvocation

建立内部结构：

```rust
ToolInvocation {
    id,
    tool,
    arguments,
    status,
    approval_state,
    started_at,
    finished_at,
    result,
    error,
}
```

建议状态：

```text
requested
awaiting_approval
approved
running
completed
failed
cancelled
timeout
```

不要重新创建第二套 Event System。

优先：

```text
ToolInvocation
+
EnvelopedEvent
```

复用现有：

```text
seq
span_id
parent_id
schema_version
```

---

# 12. G6-02：Tool Lifecycle 必须结构化

每个工具至少产生：

```text
tool_requested
tool_started
tool_completed
```

异常：

```text
tool_failed
tool_timeout
tool_cancelled
```

Approval：

```text
approval_requested
approval_resolved
```

CLI 不得依赖 tracing 文本猜状态。

---

# 13. G6-03：结构化 Tool Result

建议：

```text
exit_code
duration
stdout_summary
stderr_summary
changed_files
artifact_refs
error_class
```

避免把所有原始 stdout 全塞回上下文。

---

# 14. G6-04：副作用幂等 / Ambiguous Execution

将当前 P2-6 提升为 P1 后段。

必须处理：

```text
Tool 执行成功
↓
结果尚未落盘
↓
进程 crash
↓
resume
```

不得无条件再次执行：

```text
rm
append
external API
network side effect
```

至少建立：

```text
invocation_id
execution_status
ambiguous
```

恢复策略：

```text
known_completed
→ 不重复

known_failed
→ 可按 policy 重试

ambiguous
→ 不自动执行危险副作用
```

---

# 15. G7：Approval / Clarify

## G7-01：统一 InteractionRequest

当前已经有：

```text
InteractionRequest
```

统一：

```text
clarification
approval
egress_allowlist_request
```

等人类介入。

---

# 16. G7-02：Inline Approval

已经存在，不重做。

当前需要的是：

```text
approval
↓
user decision
↓
same turn continues
```

验证：

不得因为 approval 导致：

```text
session restart
turn restart
goal loss
context loss
```

---

# 17. G7-03：Approval Policy Matrix

此任务属于：

**WP-0 D 类：新的人类介入控制流。**

需要明确策略：

```text
always
on_request
trusted
never
```

并按动作类型区分：

```text
workspace read
workspace write
shell
network
sandbox escape
destructive action
MCP
```

最终目标：

> 安全动作尽量不打扰；真正危险动作必须停。

禁止通过：

```text
所有工具都弹审批
```

实现安全。

---

# 18. G7-04：Clarify 批量收集

当前已有 planner → InteractionRequested 雏形。

真正缺口：

```text
多个 blocking gaps
↓
一次收集
↓
用户一次回答
↓
重新构建 plan
↓
继续
```

而不是：

```text
问一个
↓
等
↓
再发现一个
↓
再问
```

---

# 19. G7-05：Egress Approval

T11 已有任务依据。

实现：

```text
Agent:
需要访问 example.com

↓
InteractionRequested{
 kind="egress_allowlist_request"
}

↓
User approve

↓
policy/config update

↓
继续当前任务
```

不能要求用户退出 Agent 去编辑 config.toml。

---

# 20. G8：Budget 与重复行为护栏

当前已经有：

```text
time
steps
retry
```

不重复建设。

新增：

```text
token_budget
same_effect_repeat_guard
```

---

## G8-01 Token Budget

至少能够：

```text
observe
warn
compact
stop
```

而不是只有：

```text
超预算后 crash
```

---

## G8-02 Same Effect Repeat Guard

检测：

```text
same tool
+
same arguments
+
same logical step
```

连续重复达到阈值：

```text
replan
clarify
give_up
```

具体触发行为属于 D 类，必须在任务书中明确后施工。

---

# 21. G9：Prompt Cache / Efficiency

## G9-01 Cache Telemetry

H6 已完成基础仪表，不重复。

当前目标：

收集：

```text
prompt_tokens
cache_hit_tokens
cache_miss_tokens
cache_reported_calls
```

---

# 22. G9-02 Stable Prefix 实证

连续采集至少：

```text
20–50 requests
```

记录：

```text
request body
stable prefix hash
dynamic suffix hash
cache hit
cache miss
```

然后回答：

```text
Stable prefix 是否真的稳定？
cache miss 从哪里开始？
```

---

# 23. G9-03 禁止“为了 cache 而 cache”

如果最终发现：

```text
cache hit
高
```

则不修改。

如果：

```text
cache hit
低
```

再定位：

```text
system prompt
tool schema
serialization
timestamp
UUID
history placement
```

禁止猜测式重构。

---

# 24. G10：CLI / TUI

本阶段 CLI 目标不是“重写 UI”。

目标是：

> **让 CLI 成为后端事实的低摩擦投影。**

---

## G10-01 `/status`

优先级 A+。

当前线程无参数：

```text
/status
```

显示：

```text
Thread
Turn
Phase
Model
Sandbox
Network
Steps
Tokens
Time
Context
Pending interaction
Current goal
```

---

# 25. G10-02 `/diff`

优先级 A。

后端产生：

```text
changed_files
diff
```

CLI 只渲染。

---

# 26. G10-03 `/compact`

优先级 A。

允许：

```text
用户主动压缩
```

并且显示：

```text
before
after
archive reference
```

---

# 27. G10-04 `/review`

不复制 Codex 的 review。

Hearth 应优先接：

```text
Observer
```

形成：

```text
/review
↓
Observer consume
↓
Finding
↓
human-readable report
```

---

# 28. G10-05 正常工作期间减少管理命令

当前：

```text
approve
resume
status
history
cancel
```

全部继续保留兼容性。

但是正常工作路径应越来越趋向：

```text
用户
↓
当前 thread
↓
Agent 自己工作
↓
只有需要用户时才开口
```

而不是：

```text
用户
↓
管理 session
↓
管理状态
↓
管理 approval
↓
管理 resume
```

---

# 29. G11：明确完成态，是 TUI 后端共同任务

用户当前最严重的不适：

> “我不知道它到底有没有做完。”

必须形成：

```text
RUNNING
WAITING
COMPLETED
FAILED
ABORTED
CANCELLED
```

然后：

CLI：

```text
✓ Task completed
```

或：

```text
✗ Task failed
Reason: cargo test failed
```

或：

```text
⏸ Waiting for your approval
```

---

# 30. Completion Summary 标准

完成任务后建议输出：

```text
✓ Completed

Goal:
修复 xxx

Changed:
  src/a.rs
  src/b.rs

Verified:
  cargo test --workspace
  351 passed

Time:
4m32s

Tokens:
...

Remaining:
None
```

如果未完成：

```text
✗ Not completed

Reason:
deadline exceeded

Completed:
...

Remaining:
...

Suggested next action:
...
```

禁止只输出：

```text
Done
```

也禁止：

```text
工具输出结束
```

后就沉默。

---

# 31. G12：真实横向 Benchmark

当前 `bench/` 已有资产。

不要重新建立测试基础设施。

新增：

```text
bench/tasks/codex-parity/
```

建议最少 10 项。

---

## C01：简单 Bug

要求：

```text
修复单文件 bug
运行测试
交付
```

指标：

```text
success
time
interventions
```

---

## C02：多文件修改

验证：

```text
goal preservation
tool lifecycle
diff
verification
```

---

## C03：失败测试修复

制造：

```text
initial test failure
```

Agent 必须：

```text
observe
diagnose
modify
retest
```

---

## C04：长命令

例如：

```text
cargo build
```

验证：

```text
declared timeout
dispatcher timeout
task deadline
```

---

## C05：网络请求

验证：

```text
network denied
↓
InteractionRequest
↓
approval
↓
continue
```

---

## C06：明确歧义

故意给出：

```text
两个合理解释
```

Agent 应：

```text
clarify
```

而不是随机猜。

---

## C07：中途 Ctrl-C

验证：

```text
checkpoint
resume
goal continuity
```

---

## C08：中途进程终止

验证：

```text
resume
state reconstruction
no duplicate side effect
```

---

## C09：Context Pressure

触发：

```text
compact
```

然后：

```text
用户：
继续
```

Agent 必须继续正确目标。

---

## C10：Long-running Delegation

用户：

```text
只说一次目标
然后不再讲话
```

Agent 应：

```text
自行执行
自行验证
自行完成
明确报告
```

---

# 32. Codex 横向对标

同一个任务：

```text
Hearth
Codex
```

尽可能：

```text
同模型
同项目
同输入
同 acceptance criteria
```

测量：

```text
Task Success
Verification Success
Human Interventions
Clarifications
Approvals
Tool Failures
Retry Count
Goal Drift
Context Loss
Resume Success
Elapsed Time
Token Usage
Completion Clarity
```

---

# 33. Delegation Friction

现有 H8 报告已经可以作为数据源。

定义第一版：

```text
Delegation Friction
=
Human Interventions / Tasks
```

其中 Human Intervention：

```text
approve
clarify
resume
manual correction
manual restart
manual configuration fix
```

必须区分：

```text
必要介入
vs
系统本来应该自己处理却迫使用户介入
```

否则指标没有意义。

---

# 34. 新增第二个指标：Goal Preservation Rate

定义：

```text
Goal Preservation Rate
=
完成结果仍满足 Original Goal 的任务数
/
总任务数
```

不要把：

```text
“模型最后确实完成了一件事情”
```

算成成功。

必须：

```text
满足 Original Goal
+
Acceptance Criteria
```

---

# 35. 新增第三个指标：Completion Clarity

任务结束后，让用户判断：

```text
我能否仅看最终输出判断：
1. 完成了吗？
2. 为什么结束？
3. 改了什么？
4. 验证了吗？
5. 还剩什么？
```

目标：

```text
5/5 明确
```

---

# 36. Codex 对标的四种最终决策

每个差异必须只能进入：

### ADOPT

Hearth 没有，而 Codex 方案明确解决用户痛点。

---

### ADAPT

Codex 思想值得吸收，但必须用 Hearth 结构实现。

这是预计最多的一类。

---

### DEFER

当前有价值，但不是地基优先级。

---

### QUARANTINE

已经存在的实现或实验能力：

```text
暂时不接入
不加入 workspace
不影响主系统
```

但保留：

```text
why_quarantined
original_use_case
known_failure
reentry_condition
related_commit
```

---

# 37. Quarantine 机制

建议采用：

```text
quarantine/
```

代码不加入：

```toml
workspace.members
```

因此：

```text
不编译
不参与主依赖
不增加日常维护成本
```

总账：

```text
docs/quarantine-register.md
```

每项必须有：

```text
why_quarantined
original_use_case
known_failure
reentry_condition
related_commit
```

---

# 38. 当前暂缓能力

以下能力当前明确不进入本轮主线：

```text
Subagents
Side conversations
完整 MCP 生态
复杂 Civilization 扩展
复杂 Personality
器灵 / 物理实体
复杂 Desktop UI
大规模并行 Agent
```

原因：

> Primary Agent 的可信委托闭环尚未充分证成。

这不是删除。

需要进入：

```text
DEFER
```

或：

```text
QUARANTINE
```

状态。

---

# 39. 为什么现在不做 Subagent

Subagent 是扩展：

```text
一个 Agent
↓
多个 Agent
```

但当前主线问题仍然包括：

```text
Goal Drift
Context Loss
Unexpected Stop
Completion Ambiguity
Resume
Long-running continuity
```

如果现在加：

```text
Agent
├── Subagent A
├── Subagent B
└── Subagent C
```

会让：

```text
状态复杂度
恢复复杂度
审批复杂度
上下文复杂度
```

同时上升。

因此：

> 先把一个 Agent 做成可靠长期委托单位，再进入 Multi-Agent。

---

# 40. 不允许的施工方式

## 禁止 1：为了“对标 Codex”复制整个 Codex crate

---

## 禁止 2：看到 Codex 有功能就实现

---

## 禁止 3：没有真实用户痛点就增加 command

---

## 禁止 4：只做接线不做运行时

---

## 禁止 5：单元测试绿就声称完成

---

## 禁止 6：commit message 作为门禁证据

必须以：

```text
~/t_gate.log
```

及真实执行输出为准。

---

## 禁止 7：只做正面测试

每个修复都必须：

```text
修复前：
负面场景真的失败

修复后：
相同负面场景真的通过
```

---

# 41. 施工顺序

```text
Phase A
源码级 Gap Audit
        ↓
Phase B
G1 Task Lifecycle
        ↓
Phase C
G2 Long-running Task
        ↓
Phase D
G3 Goal Continuity
        ↓
Phase E
G4 Goal Drift
        ↓
Phase F
G5 Context Builder
        ↓
Phase G
G6 Tool Runtime
        ↓
Phase H
G7 Approval / Clarify
        ↓
Phase I
G8 Budget / Repeat Guard
        ↓
Phase J
G9 Cache Efficiency
        ↓
Phase K
G10 CLI Projection
        ↓
Phase L
横向 Hearth × Codex Benchmark
```

注意：

部分独立任务可并行。

但不得：

```text
CLI polish
```

抢在：

```text
backend lifecycle
```

之前。

---

# 42. P0 门禁

进入 P1 前必须达到：

```text
Task 永远有明确终态
+
长任务可以无用户干预运行
+
compact 后当前目标不会丢
+
resume 不丢 TaskGoal
+
用户能明确知道完成/失败/暂停
```

---

# 43. P1 门禁

进入 CLI 大规模体验改造之前：

```text
Goal Drift benchmark pass
ToolInvocation lifecycle pass
Approval / Clarify pass
Network request pass
Cache telemetry pass
```

---

# 44. 最终门禁

Hearth 不以：

```text
“看起来很像 Codex”
```

作为完成条件。

必须满足：

```text
同等任务下：

Task Success
↑

Human Intervention
↓

Goal Drift
↓

Context Loss
↓

Unexpected Stop
↓

Retry Waste
↓

Completion Ambiguity
↓
```

并且：

```text
Hearth 自己的差异化能力
不被破坏
```

---

# 45. 每项任务统一交付格式

执行方每完成一项必须提交：

```text
任务编号：

用户体感问题：

原始假设：

源码核验结果：
（原假设成立 / 不成立，实际根因是……）

Codex 对应机制：

为什么值得/不值得吸收：

决策：
ADOPT / ADAPT / DEFER / QUARANTINE

改动文件：

关键 diff：

新增测试：

修复前负面测试：
PASS / FAIL

修复后负面测试：
PASS / FAIL

真实端到端测试：
PASS / FAIL

VM Gate：
fmt:
clippy:
test:

Gate Log：
<准确路径>

残余风险：

维护成本：

是否改变架构不变量：
YES / NO
```

---

# 46. 最终任务哲学

本任务的最终目的不是：

```text
让 Hearth 功能越来越多。
```

而是：

```text
让 Hearth 越来越“不需要用户照顾”。
```

理想体验：

```text
用户：
“把这个项目里的认证 bug 修掉，
跑测试，确认没问题。”

Hearth：
理解目标
 ↓
自主调查
 ↓
必要时澄清
 ↓
执行
 ↓
失败后自己调整
 ↓
需要授权才打扰用户
 ↓
继续执行
 ↓
验证
 ↓
完成
 ↓
明确告诉用户：
完成了什么
验证了什么
还有什么
```

而不是：

```text
用户：
给目标

Hearth：
做几步

↓

突然安静

用户：
你做完了吗？

Hearth：
……

用户：
继续

Hearth：
继续什么？

用户：
……
```

前一种体验才是：

# Trusted Delegation

后一种即使代码很多，也不能称为：

# “值得委派的 Agent”。

---

# 47. 最终架构目标

本轮完成后，希望形成：

```text
                  Hearth

                  User
                   │
                   ▼
            Current Thread
                   │
                   ▼
              Task State
                   │
          ┌────────┴────────┐
          ▼                 ▼
      Context             Policy
      Builder             Engine
          │                 │
          └────────┬────────┘
                   ▼
               Agent Loop
          Plan → Act → Observe
                   ↓
              Reflect / Replan
                   ↓
             Clarify / Approve
                   ↓
                 Verify
                   ↓
              Completion
                   │
                   ▼
              Structured Event
                   │
          ┌────────┼────────┐
          ▼        ▼        ▼
        CLI      Observer  Replay
```

其中：

```text
Task State
Goal State
Tool State
Approval State
Verification State
Completion State
```

必须都是**一等事实**。

CLI 只是投影。

---

# 48. 本轮成功的定义

如果最终用户仍然经常产生以下疑问：

```text
“它是不是停了？”

“它到底做完了吗？”

“为什么不继续？”

“我得再说一次吗？”

“它还记得刚才让我干什么吗？”

“上下文压缩以后它为什么不知道了？”

“我明明没让它改这个，它为什么跑去做那个？”

“我离开电脑以后它还会不会继续？”
```

则本轮视为：

**未完成。**

反之，当用户可以把任务交给 Hearth：

```text
输入目标
↓
离开
↓
回来
↓
看到清晰结果
```

并且：

```text
没有静默失败
没有目标漂移
没有上下文失忆
没有无意义重复
没有不必要人工干预
```

才算向：

> **“一个你敢长期委派的自主工程实体”**

真正迈进一步。

# END

---

# 守门员批注（交执行窗口前必读 · 2026-08-27 源码核实）

> 以下四条不改变本任务书任何目标与验收标准，只防"重复建设"与"摊子过大"。执行窗口施工前先读。

## 批注 1：必须复用 H8 执行报告作 Completion Record 载体（G1-03 防重复）

G1-03 要求的 completion record 字段（goal/status/steps/elapsed/tokens/tools/files_changed/verification/failure_reason）与 **v0.2.4 H8 已落地的 `report.rs`**（`.hearth/reports/<sid>/run-NNN.md`，含 approvals/reflections/errors，生产接线 `run_local.rs:487-508`）高度重叠。**禁止另建一套 completion 系统**：正确做法 = 扩展 `report.rs`，增加 `terminal_state` 字段（G1-01 的九态）+ `remaining_work`，CLI 投影 `✓/✗/⏸` 从报告读取。

## 批注 2：TaskGoal 必须先回答与现有 TaskGraph/Goal 的关系（G2/G3 防双轨）

项目已有：`agent-types` 的 `Goal`（original+budget）、TaskGraph、以及 resume 时的 `restore_task_graph` 真接线（WS11）。AUDIT-01 的 G2/G3 审查项必须**首先回答：TaskGoal 是扩展现有 TaskGraph/Goal，还是新对象**——若 AUDIT 结论是新对象，必须给出旧结构退役路径；禁止出现 TaskGraph 与 TaskGoal 双轨并存、resume 时两套状态各说各话。

## 批注 3：Round 1 范围必须收窄（本任务书是 12-Phase 程序，不是单轮施工单）

全量 G1–G12 一次交工必然质量坍塌。建议派工口径：

```text
Round 1（本任务书首轮交付）= Phase A（AUDIT-01 全量审查报告）
  + G1（终态/Completion，按批注 1 扩展 report.rs）
  + G3-03（归档 session_id 隔离——小改动，context.rs:217-233 一处）
Round 2+：G5 Context Builder / G6 ToolInvocation / G7 审批矩阵
  每项在 AUDIT-01 锚点确认后各自出独立施工单（G5 是 build_messages 大重构，
  动它前须先有 G9 数据）。
```

G7-03 审批策略矩阵、G4-04 强制暂停、G8-02 repeat guard 触发动作均为 **WP-0 D 类**——本轮只做 Observe/Warning 层，控制流变更须顶层另行出单。

## 批注 4：G9 缓存数据采集从 Phase B 起并行跑（不等到 Phase J）

G9-01/02 是**纯观测**（H6 仪表已在生产接线：`Usage` cache 字段 + `CostEntry` 累计，无代码改动即可开采）。从 Round 1 起每次真机任务顺手攒 request body hash + cache hit/miss；Phase F（G5 Context Builder）的设计决策必须吃这份 20–50 请求的数据，而不是先重构再验证——顺序反了会白干。

## 附：已核实锚点速查（执行窗口免重查）

| 事项                                                    | 锚点                                                                                                       |
| ----------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Inline Approval（已存在，勿重做）                              | `run_local.rs:619-720`（本地 y/N→resolve_interaction 续 Turn）；`repl.rs:166-187`（remote SSE 等待+30s auto-deny） |
| InteractionRequest 原语                                 | `api/lib.rs:155` 定义；blocking 范例 `loop.rs:1865`；clarification `loop.rs:1286-1301`                         |
| compact 归档现状（单共享文件）                                   | `context.rs:157-233`（`~/.config/hearth/archive/compacted.jsonl`）                                         |
| build_messages 手工拼接现场                                 | `loop.rs:956-1042`                                                                                       |
| declared_timeout / dispatcher 总闸                      | `dispatcher.rs:30/104`、`bash.rs:88`、注册点 `run_local.rs:223`                                               |
| deadline（H2 已完成）                                      | `context.rs:74-90`、`loop.rs:2770-2795`                                                                   |
| H8 报告                                                 | `report.rs`（新模块）+ `run_local.rs:487-508`                                                                 |
| Hearth.md 已有                                          | `loop.rs:124-129`（cwd 优先+家目录兜底）                                                                          |
| Egress 审批（T11）任务依据                                    | `docs/hearth-manualtest-v023-taskbook.md` T11                                                            |
| apply_patch 现状（SEARCH/REPLACE+H9 宽松档，无 Delete/Rename） | `patch.rs:1-55,104,173`                                                                                  |

## 附：环境铁律提醒

- VM 真机验证（本地 Windows 无效）；门禁以落盘 `~/t_gate.log` 为准（本任务书禁止 6 已载）。
- 沙箱 git 分支名**禁带斜杠**（loose ref 会坏）。
- 所有新锚点下笔前必须 `wc -l`/grep 实测，禁止引用任何未经实测的行号（外部 AI 幻觉锚点血泪条目）。
