# Hearth P1-EXECUTION-DECISION-01 长程施工总包 v1
## Execution Decision / Reflect / Replan / Budget / Completion Authority

**性质**：完整长程施工总包  
**执行方式**：一次授权，连续执行，Node 内部自测、自修、自验收，Node 间不等待顶层反馈  
**最终交付**：一次性提交 Final Report  
**当前基线**：Hearth v0.2.13 / `c14fac0`  
**上游状态**：
- R2-C = CLOSED
- W3/W4 = CLOSED
- RC24 = CLOSED
- W8 = CLOSED
- P1-LTR-01 = CLOSED
- P1-CONSOLIDATION-01 = PASS WITH DEVIATIONS
- P1-TASK-TRUTH-01 = PASS / CLOSED

---

# 0. 本总包的核心问题

当前 Hearth 已经可以比较可靠地回答：

> “某些事情事实上发生了什么？”

也已经开始具备：

```text
Tool Evidence
+
Artifact Evidence
+
Acceptance Verification
→
事实上的完成证据
```

但最近长程任务仍出现：

```text
修复成功
+
真实 test 通过
+
RESULT 正确
+
acceptance verification 通过
↓
planner budget-low / give_up
↓
最终 task failed
```

因此当前最大的未闭合问题已经从：

> **Completion Truth**

进一步进入：

> **Execution Decision Truth**

本总包的目标不是让 LLM “更聪明”。

目标是明确：

```text
什么输入应该进入任务执行？
什么事实可以证明进展？
什么事实可以证明完成？
Reflect 有什么权力？
Planner 有什么权力？
Replan 何时合理？
Budget 何时允许继续？
什么时候应该直接完成？
什么时候必须再次验证？
```

---

# 1. 本总包必须遵守的两个新 Core Invariant

本轮正式将以下两条原则纳入设计输入。

---

## INV-01：先分类，再行动

用户输入不能默认进入完整任务执行循环。

至少存在：

```text
Task / Product
QA / Discussion
Task Control
Conversation
Goal Mutation
```

分类应先于：

```text
planner
TaskGraph
Reflect
Replan
```

也就是说：

```text
User Input
   ↓
Task / Interaction Classification
   ↓
Execution Eligibility
   ↓
Planner / QA / Control / Conversation
```

禁止：

```text
User Input
↓
先进入 TaskGraph
↓
之后再想是不是本来不该执行
```

本原则与 W8 已落地的 QA 跳过 decompose 同向，但本轮要研究它在整个 Execution Decision 中的位置。

---

## INV-02：不得自审

生产执行系统不能仅凭自己的自述宣布自身最终正确。

必须区分：

```text
Statement
Evidence
Verification
Decision
```

最基本的不变量：

```text
LLM self-report
≠
Verification Evidence
```

以及：

```text
artifact_exists
≠
semantic correctness
```

和：

```text
tool_success
≠
goal_satisfied
```

本轮不要求立刻建立完整外部 Judge。

本原则首先要求：

> 最终 Completion Decision 必须依赖独立于 LLM 自述的结构化证据。

如需改变权力结构，必须先经过 Node 01/02 的证据论证。

---

# 2. 总包边界

## 允许处理

```text
Task type / execution routing
TaskGraph progress semantics
Fact / Verification / Reflect interaction
Replan decision
Budget-low behavior
Completion readiness
Reflect conflict observation
Failure adaptation
相关 deterministic tests
相关真机测试
```

## 不允许处理

```text
Memory / Compaction architecture
ContextBuilder architecture
ApprovalPolicy semantics
HardRedline
Sandbox capability expansion
Seccomp widening
Landlock security weakening
TaskGoal model重建
TaskGraph schema重建
Terminal 九态重建
Event schema break
Subagents
TUI
MCP
Bridge
Desktop
```

---

# 3. 长程执行协议

本任务必须遵循：

```text
Node
→
源码核实
→
设计判断
→
实现
→
单测
→
集成测试
→
VM gate
→
真机验证（如适用）
→
自验收
→
commit
→
自动进入下一 Node
```

不得每完成一两个 Node 就向用户报告。

---

## 3.1 普通失败不得暂停

例如：

```text
cargo test failed
clippy failed
fixture failed
provider 一次调用失败
真实任务失败
assertion mismatch
```

都必须：

```text
diagnose
→
fix
→
retest
→
continue
```

---

## 3.2 只有 STOP CONDITION 才能暂停

本总包允许停止的条件：

**STOP-1**：需要修改本文红线架构。

**STOP-2**：需要改变 Terminal 九态含义。

**STOP-3**：必须扩大总预算上限，无法通过有限 reserve/策略设计解决。

**STOP-4**：必须修改 Approval/Sandbox/Seccomp/Landlock 安全边界。

**STOP-5**：现有 TaskGraph / TaskGoal 模型不足以表达问题，必须建立第二套事实模型。

**STOP-6**：需要新增 Event schema 才能闭环。

**STOP-7**：发现当前某个核心设计假设被实证推翻，而且无法由本文任务书裁决。

**STOP-8**：不得自审原则与现有完成权威发生无法兼容的结构性冲突。

**STOP-9**：发现新的潜在安全漏洞且无法局部安全修复。

除此之外自动继续。

---

# 4. Node 00 — Baseline Lock / Provenance

先不改生产代码。

核实：

```text
git status
git rev-parse HEAD
git tag
Cargo version
```

核实两 VM：

```text
.133
.131
```

按照当前 `vm-version-sync.md` 的分窗登记规则执行。

使用：

```text
~/run_gate_r2c.sh
```

禁止使用其他窗口的 gate script。

记录：

```text
host
source path
binary path
HEAD
version
gate log
disk free
```

特别检查：

```text
df -h /home
```

如果磁盘自由空间不足以支撑本总包：

先在不改变生产逻辑的前提下处理环境问题。

---

# 5. Node 01 — Execution Decision 当前真实链路审计

**不急于修。先完整审计。**

从源码追踪：

```text
User Input
→ classification
→ goal application
→ planner / QA
→ TaskGraph
→ Act
→ Observe
→ Reflect
→ Replan
→ verification
→ budget
→ completion
→ terminal
```

输出内部事实图：

```text
谁决定进入 TaskGraph？
谁决定 replan？
谁决定继续？
谁决定 give_up？
谁判断 completion？
谁能够否决谁？
budget-low 在哪里产生？
verification 在哪里产生？
acceptance passed 在哪里产生？
```

尤其调查这条实际案例：

```text
repair succeeded
+
test passed
+
acceptance passed
→
budget-low
→
give_up
```

必须找到**准确控制流**，不能只根据日志猜测。

### Node 01 交付

形成：

```text
execution-decision-flow.md
```

包含：

- decision points
- data sources
- decision authority
- current inconsistencies
- evidence strength

本 Node 原则上只审计，不修改逻辑。

---

# 6. Node 02 — Completion Readiness 模型设计

基于 Node 01 的源码事实，定义：

# Completion Readiness

注意：

这不是新增事实源。

只是对既有：

```text
Tool Evidence
Artifact Evidence
Acceptance Verification
TaskGraph
```

进行组合判断。

至少区分：

```text
NOT_READY
READY_FOR_COMPLETION
REQUIRES_VERIFICATION
CONFLICTED
```

不要直接新建第三套 completion state machine。

优先使用既有：

```text
RunState
TaskGraph
acceptance_verification
terminal
```

进行派生。

---

## 6.1 关键原则

如果：

```text
Acceptance = passed
+
required verification = passed
```

那么系统必须能够表达：

> **“任务已经具备结束条件。”**

而不能继续把：

```text
budget-low
```

当作无条件更强的终止信号。

但这里暂时不要直接决定：

> completion 永远覆盖 budget。

需要结合 Node 05 设计。

---

# 7. Node 03 — Reflect Authority Audit

明确 Reflect 当前到底是什么：

```text
Planner?
Observer?
Judge?
Decision-maker?
```

源码事实必须说明。

然后建立 authority matrix：

| Decision | Fact | Verification | Reflect | Planner | Budget |
|---|---|---|---|---|---|
| tool succeeded | | | | | |
| artifact exists | | | | | |
| acceptance passed | | | | | |
| should replan | | | | | |
| can complete | | | | | |
| must stop | | | | | |

不要预设谁永远最高。

先根据源码 + 已有测试 + Node 07 实际案例分析。

---

# 8. Node 04 — Reflect / Fact Conflict 证据窗

继续复用已有：

```text
REFLECT_FACT_CONFLICT
```

本 Node 不是为了让 conflict = failure。

目标是：

```text
Fact
+
Reflect
```

冲突时能够：

```text
detect
→
record
→
classify
```

至少分类：

```text
progress misread
completion misread
task-type error
budget artifact
verification omission
unknown
```

要求：

### 自然样本优先

尽可能从真实任务采样。

如果自然样本不足：

使用 replay/mock fixture。

不得为了凑样本人为制造 production 错误。

最终报告区分：

```text
natural samples
replay/mock samples
```

---

# 9. Node 05 — Budget × Replan × Completion 核心矩阵

这是本总包最重要的控制流设计节点。

重点解释：

```text
为什么：
真实验证已经通过
仍然会：
budget-low → give_up
```

---

## 9.1 必须测试至少以下矩阵

### Case A

```text
budget plenty
verification pending
```

### Case B

```text
budget low
verification pending
```

### Case C

```text
budget low
verification failed
```

### Case D

```text
budget low
acceptance passed
```

### Case E

```text
budget low
acceptance passed
Reflect says continue
```

### Case F

```text
budget low
acceptance passed
Reflect says give_up
```

### Case G

```text
budget low
acceptance failed
```

### Case H

```text
deadline low
acceptance passed
```

---

## 9.2 必须回答

```text
何时必须继续？
何时必须验证？
何时允许 replan？
何时允许 give_up？
何时应该直接 completion？
```

特别注意：

> **不能通过增加总 budget cap 来“解决”问题。**

如果最终需要额外执行额度，必须从原始 budget 内建立**有限 Verification Reserve**。

形式上：

```text
Total Budget
├── execution budget
└── bounded verification reserve
```

不是：

```text
Original Budget
+
free verification budget
```

---

# 10. Node 06 — Failure Adaptation

研究：

```text
controlled failure
→
repair
→
retest
```

是否会：

```text
重复原方法
浪费预算
误判 stall
误触 give_up
```

必须建立：

```text
Failure
↓
classification
↓
repair evidence
↓
retry/replan decision
```

至少区分：

```text
transient
tool failure
environment failure
assertion/test failure
plan failure
verification failure
model judgment failure
```

禁止把所有 failure 都进入同一 retry 逻辑。

---

# 11. Node 07 — Routing / W8-1 边界复核

复核当前已知：

```text
negative constraint
+
real product intent
```

不得误路由 QA。

至少测试：

```text
纯 QA
纯 Product
Product + negative constraint
Negative only
TaskControl + negative constraint
Product + multiple constraints
```

---

## 11.1 核心原则

必须分离：

```text
Task Type
```

与：

```text
Constraints
```

也就是说：

```text
“创建 README，但不要修改现有文件”
```

应能够表达：

```text
Product Task
+
Negative Constraint
```

不是：

```text
QA
```

---

# 12. Node 08 — Semantic Completion / O-4 Regression

不重新设计 O-4。

只确保：

```text
cmd:
file:
nonempty
acceptance_verification
verify_failed
```

全部保持。

特别验证：

```text
LLM says PASSED
实际 test FAILED
```

时：

```text
self-report
≠
verification
```

不得发生完成误判。

---

# 13. Node 09 — Product Long-run 真机复测

这是整个总包的核心验收。

必须复用：

> P1-CONSOLIDATION-01 Node 07 mathlib 同任务、同 prompt。

保持：

```text
budget = 50
Agnes
telemetry on
deadline = 600s
HEARTH_ALLOW_NO_CGROUP=1
```

并保持同 acceptance criteria 口径。

---

## 13.1 预先固定评价标准

### PASS

同时满足：

```text
1. terminal = completed

2. cargo test
   真实 exit code 0
   且日志出现实际成功结果

3. acceptance verification = passed

4. RESULT 内容与真实事实一致

5. 不存在 unresolved REFLECT_FACT_CONFLICT

6. 步数 <= 42
```

---

### PARTIAL

例如：

```text
真实修复正确
+
真实 test passed
+
acceptance passed
+
最终因为 decision/budget 时序失败
```

不能简单写：

```text
task failed therefore all failed
```

必须标记失败层级。

---

### FAIL

例如：

```text
真实代码未修复
+
test failed
+
acceptance failed
```

---

# 14. Node 10 — Long Discussion Regression

复用：

> Rust 十问 / 10+ turn discussion。

必须确认：

```text
QA
→
不会进入错误 TaskGraph
```

确认：

```text
0 无意义 reflect/replan loop
```

同时确认：

```text
TaskControl
Conversation
```

不会污染 `goal_revision`。

---

# 15. Node 11 — Cross-layer Regression

以下全部回归：

```text
R2-C
W3/W4
RC24
W8
P1-LTR
P1-TASK-TRUTH
N1-SBX
```

重点：

### Completion

```text
tool success
artifact
acceptance
verify_failed
```

### Decision

```text
Reflect
Replan
Budget
Completion
```

### Routing

```text
QA
Product
TaskControl
GoalMutation
```

### Deadline

```text
task deadline
tool timeout
cancel
no deadline
```

### Resume

```text
resume
continue
goal continuity
```

---

# 16. Node 12 — Architecture Fitness / Governance Checks

本 Node 不做大规模代码功能。

先建立少量真正值得自动化的 invariant。

优先检查：

### INV-A

TaskControl 不得增加 goal revision。

### INV-B

Acceptance self-report 不得直接作为 completion evidence。

### INV-C

Task deadline 不得被 tool timeout 绕过。

### INV-D

QA 不得进入完整 TaskGraph execution。

### INV-E

HardRedline 不得被 session delegation bypass。

### INV-F

Verifier 不得创建新的未授权 capability。

### INV-G

Acceptance criteria 不能被 Agent 随意改写后用作自身完成证明。

### INV-H

已知事实字段语义不能通过字符串猜测重新解释。

规则遵循：

> 先建立 baseline，再只阻断新增违规。

不要为了“整洁”一次性把现存历史代码全部打红。

---

# 17. Node 13 — Final Full Gate

使用：

```text
~/run_gate_r2c.sh
```

要求：

```text
fmt = 0
clippy = 0
RT4_SOLO = 0
full test = 0
```

ignored 必须逐项解释。

测试数量变化必须说明：

```text
baseline
added
removed
ignored
```

---

# 18. Node 14 — Long-run Stability Test

如果前面 Node 09 已经成功：

再做一次不同任务的：

```text
20–30+ step Product Task
```

不要求超长。

目标不是刷步数，而是检查：

```text
Fact
Verification
Reflect
Budget
Completion
```

能否形成稳定闭环。

允许：

```text
0~1 controlled failure
```

但必须真实恢复。

---

# 19. Node 15 — Release / Provenance / Ledger

只有前面的功能和 regression 全部完成后：

### Version

按实际变更决定是否 bump。

禁止无意义 bump。

### CHANGELOG

记录：

```text
P1-EXECUTION-DECISION-01
```

### Ledger

分别写：

```text
CLOSED
OPEN
UNKNOWN
DEFERRED
```

### VM

最终：

```text
.133
.131
```

完成 source + binary provenance。

不要恢复旧的错误：

```text
.131
~/codex
```

等于当前“统一施工树”。

必须严格按照当前 `vm-version-sync.md` 的真实分窗登记执行，不能为了表格整齐而篡改事实。

---

# 20. 最终验收：不是“模型变聪明了”

本总包真正的验收标准是：

## A. 正确任务

```text
Product
→
Plan
→
Execute
→
Verify
→
Complete
```

## B. 不该执行的任务

```text
QA / Discussion
→
不进入无意义 TaskGraph
```

## C. 完成事实已经存在

```text
Acceptance = passed
```

之后：

> 系统不得仅因为低预算而无解释地 give_up。

如果最终仍然需要 give_up：

必须能解释：

```text
为什么
基于哪个事实
由哪个决策层
在什么约束下
```

---

# 21. Final Report 一次性提交

整个任务完成后，只提交：

# `P1-EXECUTION-DECISION-01 Final Report`

必须包括：

## 1. Executive Summary

只能：

```text
PASS
PASS WITH DEVIATIONS
STOP
```

## 2. Decision Flow

给出最终实际控制流。

## 3. Authority Matrix

明确：

```text
Fact
Verification
Reflect
Planner
Budget
Completion
```

各自权力边界。

## 4. Changes

逐文件说明。

## 5. Tests

```text
先红后绿
unit
integration
VM
real-machine
```

## 6. Long-run

给出：

```text
baseline
new run
steps
terminal
verification
acceptance
conflict
```

## 7. Regression

明确旧基线全部通过。

## 8. Governance

说明本轮是否新增 architecture fitness / invariant checks。

## 9. OPEN / UNKNOWN / DEFER

不隐藏任何遗留。

## 10. Provenance

完整：

```text
host
source
HEAD
binary
version
gate log
```

---

# 22. 证据等级纪律

所有结论严格使用：

```text
confirmed
likely
unknown
open
```

禁止：

```text
模型表现变好
→
直接写成架构问题已解决

一次成功
→
写成长期稳定

一次失败
→
写成 provider 不行
```

尤其：

> **模型/Provider 归因必须在排除 TaskGraph、Budget、Verification、Decision Control 后才能成立。**

---

# 23. 最终核心原则

这一轮不是为了打造：

> 一个更会思考的模型。

而是为了打造：

> **一个不会轻易把“正确事实”变成“错误决策”的 Harness。**

最终理想链：

```text
User Input
   ↓
Intent / Task Type
   ↓
Execution Eligibility
   ↓
TaskGraph
   ↓
Tool Execution
   ↓
Fact
   ↓
Verification
   ↓
Reflect
   ↓
Execution Decision
   ↓
Completion / Replan / Stop
   ↓
Terminal
   ↓
Projection
```

其中：

```text
Fact
≠
Reflect

Reflect
≠
Completion

Budget
≠
事实完成

LLM self-report
≠
Verification

Artifact Exists
≠
Semantic Correctness
```

**只有在证据支持下，系统才能宣布完成。**

---

# 24. 最重要的执行纪律

**本总包不是 Node 1 做完就回来报告，Node 2 做完再回来报告。**

GLM 必须连续执行：

```text
Node 00
→
Node 01
→
Node 02
→
……
→
Node 15
→
Final Gate
→
Final Report
```

每个 Node 内部自行：

```text
test
→
debug
→
retest
→
accept
```

只有触发 `STOP-1 ~ STOP-9` 才允许中止并向顶层报告。

普通失败必须自己处理。

---

# 25. 开始条件

现在先执行：

```text
Node 00
```

完成 baseline lock 后，立即进入 Node 01。

**不要等待用户确认。**

**不要在中途请求用户选择方案，除非触发 STOP CONDITION。**

**最终一次性交付完整 Final Report。**

---

## 总包 v1.1 修订（2026-08-30 14:10 · 顶层采纳砺·评审 C-1/C-2/C-3/RC44 + 守门员裁决，开工前生效）

> 砺·评审（`review-p1-execution-decision-01-砺.md`）的源码锚点经守门员逐条实测**全部为真**：budget-low GiveUp 臂（planner/lib.rs:495-502，`budget_fraction<=0.15 && steps_without_progress>=2` → 无条件 GiveUp，**不咨询任何验收事实**——planner 全文 grep "acceptance" = 0 命中）、三扁平常量（:233-235）、progress 只认 write_file/apply_patch（loop.rs:3676-3685，验证步骤按定义永不算 progress）、classify_user_input 无疑问句类别（loop.rs:370-378，CHITCHAT 限 ≤12 字否则一律 GoalMutation——revision 峰值 71 实证）、**RC44**（生产端写 `{"status":"failed"}` object，投影端 `as_str()` 取不到 → 折叠为 pending，loop.rs:1615-1632 vs :4275-4288）。**机械根因成立：44 步病灶 = 确定性陷阱，非模型能力。** 以下修订并入总包，与守门员 6 约束同效力。

### 修 A（C-1）：§2 排除清单勘误——常量标定纳入范围

"Memory / Compaction architecture" 条款**只排除架构重建**，明确纳入：

```text
执行决策相关的阈值与常量标定：BUDGET_LOW_THRESHOLD、MAX_STEPS_WITHOUT_PROGRESS、
steps_without_progress 记分口径、budget 默认值（codex-cli/src/lib.rs:65 默认 40）。
属"参数标定"，不属"架构重建"，不受排除清单约束。
```

但**优先级裁决**：改 progress 记分口径是备选不是首选——read/glob 不计 progress 是为抓"重复定位类"死循环（W3 批示 5 的原意），口径放宽会弱化停滞检测。**首选方案见修 D。**

### 修 B（C-2）：Node 07 增加修复动作——α 目标侧疑问句类别

Node 07 从"复核"升级为"复核 + 修复"：`classify_user_input`（loop.rs:324）增加疑问句类别——问句标点/疑问词命中 → `Conversation`（不动 goal、不 revision++），纯规则零 LLM。验收：连续 3 次纯疑问输入零 GoalChanged 且必须给出回答；与 `goal_requires_product` 维持正交（loop.rs:156 既有约束）。此为 RC36/RC33（追问被吞 + revision 爆炸）的归属落点。

### 修 C（RC44）：Node 02 前置修复——投影层反折叠

`acceptance_verification_status()` 对 `{"status":"failed"}` object 必须返回 **"failed"**（携带 failures 明细），不得折叠为 pending——否则 Node 02 四态与 Node 08 的"自述 PASSED 实际 FAILED 不得误判"测的都是断路。单测锁定三态 + failed-object 路径。小改，Node 02 开工前完成。

### 修 D（Node 05 设计分叉预裁决——避免 Node 05 烧节点）

砺指出的分叉（acceptance 事实喂进 planner obs vs loop 侧拦截）**顶层预裁决：选 loop 侧拦截**，理由与落法：

```text
① planner obs 不动（避开 STOP-5/6：不加字段、不加事件）；
② give_up 决策点前插 readiness 先决：
   budget-low GiveUp 触发前，若 acceptance criteria 非空且尚未核验
   → 先花 Reserve（1 次，沿用 acceptance_replan_count）跑核验：
     passed → completed（Case D/F/E 行为，守门员约束 1）；
     failed → 走既有 verify_replan/verify_failed 通道（Case C/G）；
   已核验且 passed → 直接 completed，planner GiveUp 判决降级为记录项；
③ planner 的 GiveUp 臂与三常量保持原样（本轮不标定）——
   机制修好后若 Node 09 复跑仍有误杀，再启用修 A 的标定权限（数据驱动，不预设）。
```

一句话：**"无验证不放弃"（criteria 存在时）+ "已验证通过不否决"**——两条都是确定性规则，不碰 planner schema，不碰 progress 口径。

### 修 E（C-3）：可独立复核性条款（连续执行不变，留痕必须）

§3/§21/§24 增补：①每 Node gate 日志含真实 exit code 与测试计数（baseline/added/removed/ignored）；②Node 02/05/08 三个决策类 Node 须附**反例用例**（构造一个应当不通过的输入，证明判据真能失败）；③Final Report 的结论逐项指向可复核证据条目，禁结论性描述；④砺·评审在 Final Report 提交后做后置独立审计（不阻塞施工，审计结论进总账并决定是否回补）。

### 修 F：Node 09 基线归因更正

"Agnes flash 多步编辑能力边界"归因**作废**（§2 机械根因已排除模型因素——放弃发生在 steps_without_progress>=2 × 预算 ≤15%，与编辑能力无关）。Node 09 复跑的对照基线表述改为："29/42/44 步三带失败，机械根因 = budget-low GiveUp 臂 × progress 口径 × 验证不记分（本总包修 D 修复对象）"。PASS 判据 #6"步数 ≤42"注明来源（CONSOLIDATION 最优基线）。砺 §10 的诚实声明同样成立：其"机械对齐"未实跑复现——**Node 01 须先用确定性 fixture 坐实 GiveUp 臂行为（把 likely 升 confirmed），再动修 D**。

---

## 守门员复核与开工前约束（2026-08-30 13:55 · 与正文同效力，开工前生效）

> 上游复核：TASK-TRUTH-01 PASS 同意——gate `.133:/home/wutao/t_gate_tasktruth_final.log` = **420/0/1 四 RC 全 0 实测**；O-4 三道防线锚点实锤（`verify_acceptance_criteria` loop.rs:1093/:4261、Reserve "1/1" 对齐真实额度、耗尽→verify_failed 带 acceptance_failures 明细、CLI `--acceptance` 接线 + pending 桥 run_local.rs:403-408——Producer Audit 抓到"criteria 静默丢失"隐藏缺陷是真产出）；双 VM 系统 PATH = 0.2.13 实证；Node 07 部分达成归因诚实（机制嫌疑未排除不怪模型）。以下 6 条为本总包开工前约束。

### 约束 1（最重要）：Case D/F 的期望行为预写——这就是 44 步病灶的修法

Node 05 矩阵的三个关键格，期望行为现在就钉死（防实施时和稀泥）：

```text
Case D（budget low + acceptance passed）
  → completed。readiness 检查必须先于 budget-low give_up——
    "顺序保证"就是控制流修正的精确落点（44 步样本：give_up 抢在实质完成后触发）。

Case F（budget low + acceptance passed + Reflect says give_up）
  → completed。Reflect 不得否决已成立的完成事实（INV-02 推论）；
    give_up 意愿记入 conflict 观察面板。

Case E（budget low + acceptance passed + Reflect says continue）
  → completed 同样合法（continue 意愿记录，但不阻塞完成）。
```

**授权澄清**：这三格落地 = "Fact overrides Reflect/Budget" 控制流变更本体——**本总包顶层批准即授权**，此前各轮批注的"控制流变更 = STOP"不再适用于本包 Node 05 的这一范围（防执行窗口拿旧 STOP 条款不敢动）。超出这三格的新控制流变更仍须 STOP。

### 约束 2：Completion Readiness 做成纯函数

像 `terminal.rs` 的 `normalize_terminal_state` 一样：平台无关、无副作用、全组合可单测——Node 05 的 Cases A-H 就是它的测试矩阵。从 RunState / acceptance_verification / TaskGraph 派生（总包已写），补充：**禁止在 readiness 内部调 LLM 或读文件**（它只组合既有结构化事实——否则就不是"派生视图"而是新事实源了）。

### 约束 3：Reserve 延续既有计数器

复用/扩展现有 `acceptance_replan_count`（TASK-TRUTH 已对齐真实额度 1/1），**不建第二个 reserve 计数器**；telemetry 记账延续（每笔 reserve 消耗带标记）。

### 约束 4：Node 09 战报按失败层级列

PARTIAL 的报告格式：每项失败标注层级（mechanism / decision / model），对照 CONSOLIDATION 29/42 步与 TASK-TRUTH 44 步三带基线——三层基线摆齐后，"模型 vs 机制"的归因才有统计意义（§22 证据等级纪律的落地形式）。

### 约束 5：证据归档与版本 bump 时机

- 真机日志**当场归档 `docs/data/execdec-<date>/`**，禁留 /tmp（历轮惯例，本总包正文漏写——补上）。
- Node 15"按实际变更决定是否 bump"——**若 bump，前置于 Node 13 final gate**（gate 必须覆盖最终提交态，历轮 fmt=1 教训）；若不 bump，战报写明理由。

### 约束 6：Node 12 的 INV 自动化选型

INV-A（TaskControl 不增 revision）/ INV-G（criteria 改写审计）/ INV-C（deadline 不可绕过）三个最适合先做成单测断言（纯函数、已有结构承载）；INV-E/F 涉及沙箱行为验证放真机脚本。"先 baseline 后阻断新增"原则正确，保持。