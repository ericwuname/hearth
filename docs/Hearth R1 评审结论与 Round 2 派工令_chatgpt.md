Hearth R1 评审结论与 Round 2 派工令

项目：Hearth
当前基线：v0.2.4 / commit 12c94ad
依据：

《Hearth × Codex Harness 对标与可信委托强化任务书 v1》
《Hearth Harness Gap Audit R1》
当前 VM 门禁及源码核验结果
一、R1 评审结论

经审阅本轮 AUDIT-01 与 Round 1 执行结果，R1 予以通过，Round 1 任务关闭。

本轮确认的关键事实如下：

1. G1 Task Lifecycle Truth 已完成第一阶段修复

原先认为“缺少 Task/Turn 终态系统”的假设不成立。

源码核实结果为：

Loop 层：
已经存在结构化 termination reason

真正问题：
CLI Projection 将多种终止原因二值化

本轮通过 terminal.rs 统一终态映射，并扩展 H8 report.rs，将 terminal state / remaining work 投影给 CLI。

因此：

G1-01 ✅
G1-02 ✅
G1-03 ✅
G1-04 ✅

完成依据与测试见 Hearth Harness Gap Audit R1。

特别确认：

本轮没有新建第二套 Task 状态机，而是在已有 Loop/Report 体系上完成修复。

该决策正确，继续保持这一原则。

2. G3-03 Archive Session Isolation 已完成

原 H3 已将 compacted turn 落盘，但此前多个 session 共用一个 archive 文件。

本轮改为：

archive/<session_id>.jsonl

并增加 session isolation 与文件名安全处理。

对应隔离测试已经完成。

因此：

G3-03 ✅

但必须注意：

这只解决 Archive 隔离，不代表“Context Compaction 后当前任务连续性”已经解决。

TaskGoal / original_goal / resume semantics 仍属于下一轮任务。

二、R1 正式关闭项

以下项目不得重复施工：

G1-01 终态集合
G1-02 异常终态映射
G1-03 Completion Summary 基础
G1-04 终态负面测试
G3-03 archive session isolation
H1 declared_timeout
H2 task deadline
H3 compact archive
H4 429 轻量熔断
H5 写前目标警示
H6 cache telemetry
H7 local read-only command 路径
H8 structured report
H9 apply_patch lenient match
H10 config get / egress 输出

除非后续新的实证证明已有实现本身存在新缺陷，否则不要重新设计。

三、R1 尚未关闭的核心问题

下一阶段不是重新从零排查，而是围绕以下未闭合链继续推进。

R2-01：TaskGoal / Goal Continuity
当前源码结论

现有：

Goal
TaskGraph
restore_task_graph

已经存在。

但：

TaskGoal

作为独立的、持久的任务语义状态还不存在。

尤其 resume 后当前 goal 可能退化为：

"continue"

再依赖：

history
+
TaskGraph

推断真正任务。

顶层方向

禁止新建第二套 Task 状态系统。

优先评估：

现有 Goal
+
immutable original_goal
+
TaskGraph
+
Checkpoint

进行扩展。

必须回答：

original_goal
normalized_goal
constraints
acceptance_criteria
current_plan
completed
remaining
next_action

分别应由哪里持有、如何持久化、resume 如何恢复。

本任务真正目标

不是新增一个漂亮的 TaskGoal struct。

而是实现：

process dies
↓
resume
↓
Agent 仍然知道原来要做什么
↓
继续当前任务
四、R2-02：Context Builder / Intent Understanding

这是下一阶段第一优先级。

源码审查已经确认：

build_messages() 目前承担大量职责，手工混合：

system_text
constitution
talent
Hearth.md
experience
plan state
compaction
tool schema

这与你的真实体感高度相关：

Hearth：
需要我把要求说得非常详细

Codex：
很容易理解我的意图

现阶段不得直接得出“模型能力不足”的结论。

必须优先验证：

context composition
system prompt
goal injection
tool schema
history arrangement
stable prefix
dynamic suffix
施工前置条件
先采 G9 Cache 数据。

要求：

≥20 requests

最好 20–50。

记录：

request body
stable prefix hash
dynamic suffix hash
cache_hit
cache_miss

R1 已确认 H6 telemetry 已接线，可以直接采数据。

禁止先重构 build_messages()，再想办法解释 cache 结果。

顺序必须：

Cache Evidence
↓
Context Diagnosis
↓
Context Design
↓
ContextBuilder implementation
↓
再次 Benchmark
五、R2-03：Goal Drift / Intent Preservation

G4 本轮不单独改。

原因：

源码已确认：

单次 run 内 replan
不会修改 goal.text

所以当前更值得追的是：

跨 Turn
跨 Resume
跨 Compact

的目标保持。

因此 G4-01/G4-02 与 R2-01 合并处理。

未来如果需要：

goal_drift_detected

检测可以先做 Observe。

任何真正改变控制流的：

自动暂停
阻止执行
强制重新确认

全部属于 WP-0 D 类，必须由顶层单独出施工单。

六、R2-04：ToolInvocation / Tool Lifecycle

当前确认：

ToolCall
ToolResult

已有。

但生命周期仍不够完整。

目标：

requested
awaiting_approval
approved
running
completed
failed
timeout
cancelled

以及：

started_at
finished_at
result
error
强约束

禁止新建第二套 Event System。

必须复用：

EnvelopedEvent

已有：

seq
span_id
parent_id
schema_version

在其基础上扩展结构化语义。

七、R2-05：Approval / Clarify

这一组必须拆开看。

已有能力
Inline Approval ✅
InteractionRequest ✅
Clarification 雏形 ✅

不得重复开发。

尚缺能力
Approval Policy Matrix
Clarify Batch Collection
Egress Approval

其中：

Approval Policy Matrix
Clarify Batch
Repeat Guard 的控制流动作

全部属于：

WP-0 D 类

因此：

本轮执行窗口不得自行设计这些“什么时候必须停、什么时候可以继续”的新控制流。

必须由顶层另出施工单后再执行。

八、R2-06：Egress Approval

该项已有 T11 任务依据，可以继续。

目标：

Agent:
需要访问 example.com

↓
InteractionRequest

↓
用户批准

↓
落盘 policy

↓
继续当前任务

不得要求用户离开当前任务手工修改配置。

九、R2-07：Budget / Repeat Guard

已有：

deadline
steps
retry

不得重复建设。

待补：

token_budget
same_effect_repeat_guard

其中：

token_budget

先做：

observe
warn
compact
stop
same_effect_repeat_guard

只能先做：

Observe / Diagnostic

如果决定：

replan
clarify
give_up

属于 D 类，必须由顶层出单。

十、R2-08：Long-running Delegation

这是本轮必须保留的终极验证目标，但暂不直接声称“已实现”。

原因已经由源码核验明确：

虽然：

resume
deadline
task graph

存在，

但：

TaskGoal
完整 checkpoint
Long-running benchmark

尚未证明。

因此：

“支持长任务”目前只能算结构能力存在，不能算用户体感上已经成立。

真正的验证必须是：

用户：
只输入一次目标

然后：
不再输入“继续”

Agent：
自主探索
自主执行
自主测试
自主修复
自主验证
明确完成

这个测试暂挂，待 R2 基础完成后进入真实 benchmark。

十一、R2-09：Completion Truth 持续强化

G1 第一阶段已经解决：

“它是不是结束了？”

下一阶段继续保证：

completed

必须对应：

Goal satisfied
+
Acceptance criteria satisfied
+
Verification passed

而不是：

Loop returned normally

Completion Summary 至少继续维护：

Goal
Status
Reason
Changed Files
Verification
Remaining Work
Time
Tokens
十二、R2-10：CLI 先不做“大重构”

当前 CLI/TUI 不进入主改造阶段。

只允许做：

backend facts
↓
projection

所必需的低成本调整。

重点暂定：

/status
/diff
/compact

但：

不允许为了“看起来像 Codex”而提前进行大规模 UI 重构。

原因：

当前用户体感的主要问题，已经初步证明部分根因在后端状态/上下文，而非单纯呈现。

十三、Codex 对标原则

以后每个 Codex 差异都必须回答：

Hearth 当前是什么？

Codex 当前是什么？

Codex 方案解决了什么问题？

Hearth 为什么没有达到同样体验？

根因是哪一层？

Model？
Prompt？
Context？
Tool Runtime？
Policy？
State？
Recovery？
CLI？

吸收方式：
ADOPT / ADAPT / DEFER / QUARANTINE

禁止：

Codex 有
↓
Hearth 没有
↓
所以加
十四、Round 2 第一阶段的具体执行顺序

执行窗口按以下顺序推进：

R2-A
Cache Evidence Collection
        ↓
R2-B
TaskGoal / Goal Persistence design
        ↓
R2-C
ContextBuilder design
        ↓
R2-D
TaskGoal + Context integration
        ↓
R2-E
ToolInvocation lifecycle
        ↓
R2-F
Egress Approval
        ↓
R2-G
D 类事项由顶层出单后实施
        ↓
R2-H
Codex parity benchmark

其中：

TaskGoal
ContextBuilder

是当前两条最核心的主线。

十五、必须新增一个“真实体感验证”阶段

代码门禁之外，再增加：

Human Delegation Test

每轮至少测试：

Test A：一句话任务
用户只描述一次目标。

观察：

理解
计划
执行

是否需要反复解释。

Test B：中途不说话

观察：

Agent 是否自行继续。
Test C：中断

观察：

resume

是否恢复同一任务。

Test D：Compact

观察：

compact
+
“继续”

是否知道继续什么。

Test E：失败后 Replan

观察：

工具失败
测试失败
编译失败

之后：

是否仍然围绕 Original Goal 工作。

十六、R2 的最终验收标准

不要求：

“Hearth 完全等于 Codex。”

要求：

任务连续性
无静默中断
完成真实性
明确 completed / failed / waiting
Goal Preservation
Original Goal 不因 replan / compact / resume 消失
长程能力
不需要人工不断发送“继续”
Context
Compact 不丢任务核心语义
Intent
同一用户输入下，
Hearth 与 Codex 的意图理解差距明显下降
Efficiency
Cache 数据可测
问题有证据
修改有 Benchmark
十七、Quarantine 正式生效

本项目允许：

有效但暂时不用

而不是：

要么接入
要么删除

因此：

ACTIVE
EXPERIMENTAL
DEFERRED
QUARANTINED
RETIRED

作为组件生命周期状态。

需要隔离的代码：

quarantine/

不得加入 Cargo workspace。

总账：

docs/quarantine-register.md

每个条目必须注明：

why_quarantined
original_use_case
known_failure
reentry_condition
related_commit

这样未来几十万 LOC 时：

“不使用”

不等于：

“忘记了为什么不使用”
十八、最终守门原则

以后 Hearth 的任何新能力都必须首先回答：

它是在帮助 Agent 更好地完成任务，还是仅仅让系统看起来功能更多？

如果只是增加复杂度：

DEFER

如果实现已经存在但现在不适合：

QUARANTINE

如果真正降低用户 Delegation Friction：

ADOPT / ADAPT
十九、本轮正式状态
R1
──────
AUDIT-01       ✅
G1             ✅
G3-03          ✅
VM Gate        ✅

R2
──────
Cache Evidence        → 开始
TaskGoal              → 待施工
ContextBuilder        → 待施工
ToolInvocation        → 待施工
Approval Policy       → 顶层出单
Clarify Batch         → 顶层出单
Repeat Guard Action   → 顶层出单
Egress Approval       → 待施工
CLI/TUI 大改          → 暂缓
Subagent              → DEFER
二十、给执行窗口的一句话

上一轮不是“发现更多功能缺失”，而是成功把问题从“功能层”定位到了“任务连续性、目标状态、Context Assembly 和 Runtime 生命周期”几个真正的系统层。下一轮不要扩张系统，继续收紧这几个核心闭环。

本任务书批准进入 Round 2 设计/施工准备阶段，但 Round 2 中涉及 D 类控制流的部分，必须等待顶层专项任务书，不得自行决策。

---

# 守门员批注（派工前修正 · 2026-08-28 源码+VM 实证）

> R1 已独立验收通过（terminal.rs 九态/归档会话隔离/report 扩展均源码实证；VM `~/t_gate.log` 实测 CLIPPY_RC=0 / TEST_RC=0 / **TOTAL_PASSED=358**，与 commit `4a519a7` 自述吻合）。以下六条为派工令修正与补充，交执行窗口时一并生效。

## 修正 1：基线已过时
派工令头注"当前基线：v0.2.4 / commit 12c94ad"——**R1 已交付 v0.2.5 / commit `4a519a7`（358 tests 全绿）**。所有 R2 锚点以 4a519a7 为基线，开工前先 `git log -1` 自证。

## 修正 2：R2-A 缓存采集——预算授权【已批准，2026-08-28 用户拍板】
用户已开通 Agnes 会员并明确授权本通道为 R2 真实 LLM 任务（缓存采集 / Human Delegation Test / 后续 benchmark）的预算来源：
- **通道**：provider=agnes（OpenAI 兼容），URL `https://api.agnes-ai.cn/v1`，model `agnes-2.5-flash`，key 见用户级记忆（cpk- 会员 key，不写入项目文档）。
- **配额**：每周 75,000 次 / 每 5 小时 7,500 次（按次计，非按 token）——充足，正常采集与测试不会触顶；仍保留常识性自保：单任务狂转时留意 429（配额触顶特征），见 429 按 H4 熔断语义停止而非重试。
- **采集协议不变**：落盘 jsonl（ts/session_id/prefix_hash/suffix_hash/cache_hit/cache_miss/prompt_tokens/model），采集期 R2-B 设计并行推进。
- 注意：key 不落项目仓库文档（本文件与任务书均不写 key），执行窗口从 VM 本地 config.toml / 环境变量注入。

## 修正 3：R2-B 设计产出后必须回评审，不得直通 R2-D
R2-B（TaskGoal/Goal Persistence design）的交付物是**设计单**：逐字段回答 original_goal/normalized_goal/constraints/acceptance_criteria/current_plan/completed/remaining/next_action 各自由谁持有、如何持久化、resume 如何恢复（即派工令第四节"必须回答"八项）+ 批注 2 双轨问题的正式作答（扩展 Goal/TaskGraph 而非新对象）。**设计单须回顶层（用户+ChatGPT）评审通过后才进 R2-D 施工**——这是项目"顶层设计≠施工图"铁律的强制关卡，顺序图里 R2-B→R2-D 的直通线不成立。

## 修正 4：R2-09 写明依赖顺序
"completed 必须对应 Goal satisfied + Acceptance criteria satisfied"——acceptance_criteria 的承载在 R2-B TaskGoal 落地前不存在。R2-09 的强化**依赖 R2-B**，在 TaskGoal 落地前只维护现状字段（reason/remaining_work），禁止提前硬造 acceptance criteria 判定逻辑。

## 修正 5：R2-F Egress 施工的两个已知坑
①WebTool 是纯工具（无事件出口、拿不到 emitter），deny→审批请求**必须在 loop/dispatcher 层做错误分类后 emit `InteractionRequested`**，不得在工具内自建通道；②approve 后落盘**复用 `hearth config set` 既有 config 写入路径**（含并发与转义处理），禁止新写 config writer。T11 任务书（`docs/hearth-manualtest-v023-taskbook.md`）即顶层单，可直接施工。

## 修正 6：R2-E 事件契约三条红线
新增 ToolInvocation lifecycle 事件时：①契约**只增不改**，FE 对未知 kind 宽容（禁白屏）；②Observer 规则引擎对未知事件**默认忽略不炸**（新增事件勿触发既有熔断规则误报）；③G4 断线续传的 seq 单调语义不动。另：R2-A 数据采集与 R2-B 设计**可并行**（互不依赖，R2-C 才消费数据）；§十五 Human Delegation Test 同样吃真 LLM 预算，建议挂用户手工测试节奏（v0.2.3 模式）而非自动门禁，且每次预算同修正 2。