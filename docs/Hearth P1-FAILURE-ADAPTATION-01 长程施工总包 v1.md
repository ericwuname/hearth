# Hearth P1-FAILURE-ADAPTATION-01 长程施工总包 v1.1
## Failure Classification / Diagnosis / Repair / Retest / Recovery

**日期**：2026-08-30（v1.1：砺批注采纳 + 守门员复核定稿）  
**性质**：完整长程施工总包  
**执行方式**：一次授权，连续执行；Node 间不得等待顶层确认  
**最终交付**：一次性提交 Final Report  
**当前基线**：Hearth v0.2.14 / HEAD `2fe0688`  
**上游状态**：
- R2-C = CLOSED
- W3/W4 = CLOSED
- RC24 = CLOSED
- W8 = CLOSED
- P1-LTR-01 = CLOSED
- P1-CONSOLIDATION-01 = PASS WITH DEVIATIONS
- P1-TASK-TRUTH-01 = PASS / CLOSED
- P1-EXECUTION-DECISION-01 = PASS / CLOSED

---

# 0. 使命

本总包承接 P1-EXECUTION-DECISION-01 留下的最后一块核心缺口：

> **任务失败以后，Hearth 能否识别“失败是什么”，并选择正确的恢复策略，而不是把所有失败都当成同一种 retry。**

当前已知活体问题：

```text
真实任务：
修复成功
↓
真实 test 通过
↓
acceptance verification 通过
↓
Execution Decision 已能阻止错误 GiveUp
```

但另一个场景仍未完全闭合：

```text
受控失败
↓
修复
↓
复测
↓
恢复
```

仍可能出现：

```text
失败
↓
错误重试
↓
重复同一方法
↓
浪费 budget
↓
stall / give_up
```

因此本总包的目标是把：

```text
Failure
    ↓
Classification
    ↓
Diagnosis
    ↓
Strategy
    ↓
Repair / Retry / Replan / Escalate
    ↓
Retest
    ↓
Verification
    ↓
Continue / Complete / Stop
```

真正建立起来。

---

# 1. 三条本轮核心原则

## INV-03：Failure ≠ Retry

失败本身不是动作指令。

禁止：

```text
Failure
↓
counter++
↓
retry
```

正确方向：

```text
Failure
↓
classify
↓
choose strategy
```

---

## INV-04：Progress ≠ Mutation

“文件发生修改”不是唯一的进展定义。

例如：

```text
write_file
```

可能是进展。

但是：

```text
cargo test
```

虽然没有修改文件，却可能证明：

```text
代码修复已经成功
```

同样属于有效进展。

因此本轮不得简单把所有 bash/read/test 都计为 progress，也不得简单把所有 write 都算作 progress。

必须先审计 `steps_without_progress` 的真实语义，再决定是否需要扩展进展定义。

---

## INV-05：Recovery 必须改变状态或知识

一次真正有意义的 recovery attempt 至少应该导致：

```text
状态改变
或
事实增加
或
诊断信息增加
或
执行策略改变
```

否则不能把：

```text
相同输入
+
相同工具
+
相同失败
```

无上限地重复。

---

# 2. 全局边界

## 本轮允许

```text
Failure classification
Failure evidence
Progress semantics
Retry policy
Replan policy
Recovery state
Controlled failure
Retest
Failure telemetry
Long-run recovery
```

## 本轮禁止

```text
Memory / Compaction architecture
ContextBuilder architecture
TaskGoal schema redesign
TaskGraph schema redesign
Terminal 九态 redesign
ApprovalPolicy semantic redesign
Sandbox capability expansion
Seccomp widening
Landlock weakening
Subagents
TUI
MCP
Bridge
Desktop
```

已经 CLOSED 的 O-4 / Execution Decision 不重新设计，只允许回归。

---

# 3. 执行协议

每个 Node 内部必须：

```text
Inspect
→
Design
→
Implement
→
Test
→
VM gate
→
Real-machine validation where required
→
Commit
→
Self-accept
→
Next Node
```

普通失败：

```text
cargo test
clippy
fmt
fixture
provider transient failure
单次真机任务失败
```

不得暂停等待用户。

必须：

```text
diagnose
→
fix
→
retest
→
continue
```

只有 STOP 条件允许暂停。

---

# 4. STOP CONDITIONS

**STOP-1**：必须新增第二套 TaskGraph / TaskGoal / Completion 事实模型。

**STOP-2**：必须改变 Terminal 九态的语义。

**STOP-3**：必须无限扩大总 budget，而无法通过有限 recovery reserve / 策略重排解决。

**STOP-4**：必须改变 Approval / Sandbox / Seccomp / Landlock 安全边界。

**STOP-5**：必须修改事件契约且无法向后兼容。

**STOP-6**：发现核心事实模型与本任务书发生结构性冲突。

**STOP-7**：发现恢复机制可能产生新的未授权 capability。

**STOP-8**：任何单一 failure policy 会导致安全上明显更宽松的执行能力。

**STOP-9**：无法用现有证据层区分失败类型，必须引入 LLM judge 才能继续——此时停在设计边界，不得擅自引入新的语义裁判。

除 STOP 外，一律继续执行。

---

# 5. Node 00 — Baseline / Provenance / Health Lock

先不改生产逻辑。

核实：

```text
git status
git rev-parse HEAD
git tag
Cargo.toml version
```

双 VM：

```text
.131
.133
```

按照当前 `vm-version-sync.md` 的分窗登记规则执行，不自行修改历史口径。

门禁只使用：

```text
~/run_gate_r2c.sh
```

记录：

```text
host
source
HEAD
binary
version
gate log
disk free
```

必须先做：

```text
df -h /home
```

长程任务启动前保证有足够磁盘余量。

输出：

```text
docs/data/failure-adaptation-20260830/node00-baseline.md
```

---

# 6. Node 01 — Failure Lifecycle Source Audit

源码追踪一个 Failure 从产生到终局的完整生命周期：

```text
ToolResult / Error
↓
loop observation
↓
consecutive_errors
↓
steps_without_progress
↓
Reflect
↓
Replan / GiveUp
↓
Verification
↓
Terminal
```

必须回答：

```text
谁产生 failure？
谁给 failure 分类？
谁计数？
谁决定 retry？
谁决定 replan？
谁决定 give_up？
谁能清零 counter？
谁能证明 recovery 成功？
```

特别审计：

```text
consecutive_errors
steps_without_progress
verify_replan_count
graph_stall_count
budget_low
deadline
```

之间是否存在互相污染。

交付：

```text
docs/failure-adaptation-flow.md
```

**（v1.1 必补，承接砺批-1）**：交付物必须增加一节——
**"criteria 为空时 give_up 与 completion 的完整行为路径"**。
已实证：修 D 拦截以 `edd_checks.is_empty()` 开门（loop.rs:3945），Done 相位核验
同构守门（loop.rs:4450）——criteria 为空的任务在两处均无保护，bash-only Product
任务仍会 5 步无进展 → Replan×3 → GiveUp。这是上轮 44 步病灶三要素之外的
第四要素，必须先在审计层把它画完整，Node 08 F9 与 Node 12 加注才站得住。

本 Node 原则上只审计，不修改 production control flow。

---

# 7. Node 02 — Failure Taxonomy

基于 Node 01 的真实代码建立最小 Failure Taxonomy。

至少区分：

```text
F1 transient_provider
F2 tool_execution
F3 environment
F4 permission / approval
F5 assertion_or_test
F6 verification
F7 plan / strategy
F8 resource / budget
F9 model_judgment
F10 unknown
```

注意：

不要为了分类而新建复杂继承树。

优先使用：

```text
既有 error/result 类型
+
派生分类函数
```

分类必须是：

```text
deterministic where possible
```

不能靠模糊文本启发式猜测已经存在的结构化事实。

测试要求：

- 每一类至少 1 个正例
- 至少 2 个相邻类别边界反例
- unknown 必须真实存在且不能被强行塞入某类

---

# 8. Node 03 — Failure → Strategy Matrix

建立正式策略矩阵：

| Failure | 默认策略 | 是否 Retry | 是否 Replan | 是否等待/切通道 | 是否允许人工 |
|---|---|---:|---:|---:|---:|
| transient provider | backoff / bounded retry | ✅ | 通常否 | ✅ | 可选 |
| tool execution | diagnose tool | 条件 | 条件 | 否 | 条件 |
| environment | diagnose env | 通常否 | 条件 | 否 | ✅ |
| permission/approval | request/deny/delegate | 否 | 否 | 否 | ✅ |
| test/assertion | inspect + repair | 条件 | 条件 | 否 | 否 |
| verification | inspect acceptance | 否 | ✅ | 否 | 条件 |
| plan failure | replan | 否 | ✅ | 否 | 否 |
| resource/budget | stop / reserve | 否 | 条件 | 否 | 条件 |
| model judgment | record conflict / verify | 否 | 条件 | 否 | 条件 |
| unknown | bounded fallback | 最多有限次数 | 条件 | 否 | ✅ |

这张表不是最终真理。

要求先：

```text源码事实
+
历史证据
+
fixture
```

再定最终版本。

交付：

```text
docs/failure-strategy-matrix.md
```

---

# 9. Node 04 — Progress Semantics Audit

重点解决：

> `steps_without_progress` 是否错误地把验证工作视为“没有进展”。

先不要直接修改 counter。

审计至少以下动作：

```text
write_file
apply_patch
bash build
bash test
bash verification
read
grep
glob
LSP
acceptance verifier
```

对每个动作回答：

```text
是否可能产生新的事实？
是否可能改变任务状态？
是否可能减少不确定性？
是否可能直接证明完成？
是否应该影响“无进展”计数？
```

要求形成最小语义模型，而不是简单增加计数。

**（v1.1 必补，承接砺批-3）交互分析小节**：Node 04 交付物必须含
**"与修 D 拦截的交互分析"**。修 D 拦截（loop.rs:3937-4023）与 progress 口径
扩展（现口径 loop.rs:3752-3778，只认 write_file/apply_patch——砺批-3 引的
3676-3685 为旧行号，施工时以此为准）**都改"give_up 前的行为"**，叠加存在二阶
效应：拦截内核验步若也算 progress → counter 清零 → budget 燃烧节奏改变 →
GiveUp 臂命中条件漂移。任何 progress 语义变更必须先红后绿跑上轮 Case D
fixture（GIVE_UP_OVERRIDDEN 路径不得回归）。

建议候选：

```text
Mutation Progress
Verification Progress
Knowledge Progress
Task-State Progress
```

但：

> **不得在没有证据的情况下直接创建四套生产计数器。**

优先：

```text
progress = task state transition
```

作为设计候选进行验证。

---

# 10. Node 05 — Recovery State Model

设计“恢复过程”的最小状态。

不新建第四套 Task 模型。

优先使用既有：

```text
RunState
TaskGraph
Failure classification
Verification
Terminal
```

可以增加派生信息，例如：

```text
last_failure_class
recovery_attempts
last_recovery_strategy
```

但这些必须先证明必要。

要求回答：

```text
一次 failure attempt 是什么？
什么时候算 recovery attempt？
什么时候算 recovery 成功？
什么时候算重复策略？
什么时候必须换策略？
```

核心反重复原则：

```text
same failure
+
same state
+
same strategy
```

不得无限重复。

**（v1.1 必答，承接砺批-2）Reserve 粒度显式声明**：源码现状（已复核 confirmed）——
三个恢复类计数器 `acceptance_replan_count`（loop.rs:935）/ `verify_replan_count`
（loop.rs:953）/ `graph_stall_count`（loop.rs:963）中，修 D 拦截
（loop.rs:3972-3976）与 Done 相位 Reserve（loop.rs:4456-4458）**共用**
`acceptance_replan_count`，单 run 单 Reserve，有界性成立但**跨相位零和**：
拦截耗掉后，Done 相位核验失败直接 verify_failed，无第二次修复机会。
**顶层裁决：粒度维持 run-level 共用（上轮总包 v1.1 修 D 已裁决，本轮不翻案）**。
Node 05 交付物只须：①登记上述事实；②分析若 Node 07 调整粒度对
`verify_replan_count` / `graph_stall_count` 的影响——这是 Node 01"互相污染"
审计的最优先案例；③**不得擅自改粒度**（改粒度=控制流变更，触发 STOP-6 走顶层）。

---

# 11. Node 06 — Retry / Replan / Repair Separation

将三种动作严格区分：

## Retry

同一任务、同一策略、失败原因预计为 transient。

例如：

```text
429
5xx
network reset
```

---

## Repair

失败提供了可修复的信息。

例如：

```text
cargo test failed
↓
inspect error
↓
modify code
↓
test again
```

---

## Replan

当前计划本身已经不适用。

例如：

```text
required file unavailable
architecture assumption false
dependency path invalid
```

要求：

```text
Retry ≠ Repair
Repair ≠ Replan
Replan ≠ GiveUp
```

建立单测锁定边界。

---

# 12. Node 07 — Budget-aware Recovery

分析：

```text
Failure
+
remaining budget
```

的交互。

至少测试：

### Case A

```text
budget plenty
transient failure
```

### Case B

```text
budget plenty
test failure
```

### Case C

```text
budget low
transient failure
```

### Case D

```text
budget low
repair still possible
```

### Case E

```text
budget low
verification pending
```

### Case F

```text
budget low
no new evidence after repeated attempt
```

要求：

> 不通过简单提高全局 budget cap 解决。

必须保持：

```text
bounded retries
bounded recovery
deadline remains authoritative
```

如果需要 recovery reserve：

```text
Total Budget
├── execution
└── bounded recovery/verification reserve
```

reserve 必须有上限。

---

# 13. Node 08 — Failure Adaptation Fixture Suite

建立确定性 fixture。

至少：

### Fixture F1

```text
429
→
retry
→
success
```

### Fixture F2

```text
tool timeout
→
retry once
→
success
```

### Fixture F3

```text
cargo test fail
→
repair
→
test pass
```

### Fixture F4

```text
permission denied
→
cannot retry blindly
→
structured escalation
```

### Fixture F5

```text
same failure + same strategy
→
bounded stop / replan
```

### Fixture F6

```text
verification failed
→
recovery
→
reverify
```

### Fixture F7

```text
unknown failure
→
bounded fallback
→
no infinite loop
```

### Fixture F8

```text
deadline nearly exhausted
→
no unbounded recovery
```

### Fixture F9（v1.1 必补，承接砺批-1）

```text
criteria 为空 + bash-only Product 任务 + budget low
→
先红后绿：红 = GiveUp 照旧发生且无任何核验证据
（loop.rs:3945 / :4450 双守门均不触发）；
绿 = 拦截或显式 escalate，terminal 不得假 completed
```

F9 锁的是"criteria 覆盖度=用户责任"边界下的系统行为：系统可以拒绝
核验（criteria 空），但**不得把未核验的放弃伪装成正确终态**——
失败报告必须明示"无 acceptance criteria，放弃未经核验"。

这些 fixture 必须证明：

```text
failure class
+
selected strategy
+
attempt count
+
terminal
```

---

# 14. Node 09 — Controlled Failure End-to-End

构造真实 TDD 风格任务：

```text
红测试
↓
定位
↓
修改
↓
绿测试
↓
验收
↓
完成
```

要求：

- 第一阶段允许真实失败；
- 失败必须被识别为 assertion/test 类；
- Agent 必须产生不同于第一次失败尝试的恢复行为；
- 修复后必须真实重新运行测试；
- 最终 acceptance 必须独立通过；
- 不得把第一次失败直接视为最终 task failure。

重点采集：

```text
failure_class
recovery_strategy
strategy_change
budget_before
budget_after
test_result
acceptance_result
terminal
```

---

# 15. Node 10 — Repeated Failure / Anti-loop

专门验证“不要死磕同一个方法”。

构造：

```text
same failure
+
same state
```

连续出现的场景。

要求系统：

```text
detect repetition
↓
bounded strategy change
↓
replan / escalate / stop
```

不能出现：

```text
same tool
same args
same error
same plan
```

无限循环。

如果现有系统已经有 `same_effect_repeat_guard` 候选或类似机制：

优先复用，不新增第二套。

---

# 16. Node 11 — Resource / Failure Interaction

把前面已经落地的：

```text
ResourceLedger
TaskDeadline
Verification Reserve
```

放在同一矩阵中。

测试：

```text
failure
+
large tool output
+
deadline low
+
budget low
```

不能因为 recovery 而：

```text
突破 deadline
突破 budget
突破 resource guard
```

必须保证：

```text
Safety
>
Deadline
>
Resource
>
Recovery
>
Convenience
```

如果实际源码顺序与此不一致：

先记录事实，再设计修正。

任何需要扩大安全能力的决定必须 STOP。

---

# 17. Node 12 — Long-run Recovery Test #1

使用此前 canonical mathlib 任务作为主样本。

不要只测试：

```text
正常成功
```

而要强制包含：

```text
controlled failure
→
repair
→
test
→
acceptance
```

参数：

```text
budget = 50
deadline = 600s
Agnes
telemetry on
HEARTH_ALLOW_NO_CGROUP=1
```

预写验收：

### PASS

```text
1. controlled failure 被正确分类
2. repair 实际发生
3. test 实际重新执行
4. test 最终真实成功
5. acceptance verification = passed
6. RESULT 与真实事实一致
7. no unresolved conflict
8. no infinite repeated strategy
9. terminal = completed
```

**（v1.1 必补，承接砺批-1/批-5）**：
① 判据 #5 加注——**criteria 为空时本条是空洞真值，不构成完成证据**，
须以 Node 08 F9 结果替代；Node 12 任务的 criteria 全文必须在 draft 阶段
冻结并归档进 Final Report（故障注入点必须被 criteria 显式覆盖，
如"修复后的模块行为=X"而非只验"测试文件存在"——上轮 Node 14 受控失败
未真恢复的假阳性即此根因）。
② **上轮 OPEN 项承接**：Node 12 mathlib 任务复跑时，criteria 必须**显式包含
上轮 shout bug 的真实修复**（"shout 输出符合 Y"级），把上轮
"STATUS.txt 达标但 shout 未修"的 OPEN 变成本轮实证闭环。

**（顶层已裁决 2026-08-30：步数判据口径，替代原"步骤数不作为绝对 PASS 条件"）**：
上轮 Node 09 实测 46 步 completed（最优基线 42，+4 = 修 D 拦截 Reserve 核验 +
复测的**合规成本**，层归 mechanism 非决策/模型）。本轮步数判据据此定为：
- **步数 ≤ 52**（= 46 实测基线 + recovery overhead 上限 8；overhead 机械上界由
  Reserve ≤1 次核验步 + 1 次复测步 + Replan cap 3 兜底，8 取宽松余量）；
- **必须额外统计并单独报告 `recovery overhead`**（相对最优基线 42 的步数差），
  与步数绝对值分列——overhead 超出 8 步即触发层归分析（mechanism / decision /
  model 三层归因），不得笼统判 PASS；
- **terminal=failed 仍不单独否定已证明的 recovery 机制**（§26 分层归因不变），
  但 completed 且步数 > 52 时同样不得直接判 PASS，须先完成层归。

---

# 18. Node 13 — Long-run Recovery Test #2

换一个不同于 mathlib 的真实 Product Task。

要求至少：

```text
20–40 steps
```

中间包含：

```text
一次可恢复工具失败
一次 test/assertion failure
一次 verification
最终 completion
```

目的：

> 验证系统不是只针对 mathlib 样本过拟合。

记录：

```text
goal
steps
failures
failure_classes
strategies
replans
verification
terminal
resource usage
```

---

# 19. Node 14 — Discussion / QA Negative Regression

复用已有十问 Discussion。

必须确认：

```text
QA
↓
direct answer
```

不会进入：

```text
Failure Recovery
TaskGraph
Reflect loop
Retry
```

同时验证：

```text
TaskControl
Conversation
```

不会产生 failure/recovery side effect。

---

# 20. Node 15 — Cross-layer Final Regression

回归全部历史主线：

```text
R2-C
W3/W4
RC24
W8
P1-LTR-01
P1-TASK-TRUTH-01
P1-EXECUTION-DECISION-01
N1-SBX
```

至少验证：

```text
Intent routing
TaskGraph continuity
Completion readiness
Acceptance verification
GiveUp interception
Deadline
Approval
Sandbox
Resume
Terminal
Projection
```

重点检查：

> 新的 failure strategy 是否偷偷改变了以前已经 CLOSED 的语义。

**（v1.1 必补，承接砺批-3）**：Node 15 回归必须包含**上轮
`GIVE_UP_OVERRIDDEN` Case D 路径的真机样本**（44 步病灶闭环不得回归）；
若 Node 04 扩展了 progress 语义，还须对照 Node 04 交互分析小节逐条核销
二阶效应清单。

---

# 21. Architecture Fitness Checks

本轮最终至少形成以下可执行 invariant：

### INV-A

```text
Failure ≠ Retry
```

### INV-B

```text
Progress ≠ Mutation
```

### INV-C

```text
same failure + same strategy
不能无限重复
```

### INV-D

```text
deadline 不得被 recovery 绕过
```

### INV-E

```text
budget reserve 不得变成无限免费预算
```

### INV-F

```text
verification failure ≠ arbitrary retry
```

### INV-G

```text
LLM self-report ≠ failure recovery evidence
```

### INV-H

```text
QA / Conversation 不得进入 failure recovery loop
```

规则遵循：

> 先建立 baseline，只阻断新增违规。

不得为了“整洁”一次性打爆现有 workspace。

**（v1.1 必补）INV 编号防混淆**：上轮 P1-EXECUTION-DECISION-01 已落一套
INV-A～G（INV-A=TaskControl 不污染 goal_revision、INV-G=criteria 不可被
Agent 改写后自证完成等，已单测锁定）；本总包 §21 又定义了一套 INV-A～H
（Failure ≠ Retry 等）。**两套字母编号语义不同，Final Report 与 ledger 回填时
必须加前缀区分**：`INV-ED01-x`（上轮 EXECUTION-DECISION）与
`INV-FA01-x`（本轮 FAILURE-ADAPTATION），禁止裸写 INV-A 造成台账错挂。
两套各自的既有单测全部保持绿，本轮只新增不回改。

---

# 22. Telemetry / Evidence

本轮必须记录至少：

```text
session_id
task_id
failure_class
failure_count
recovery_attempt
recovery_strategy
strategy_changed
progress_before
progress_after
budget_before
budget_after
deadline_remaining
verification_state
terminal
```

如果现有 telemetry schema 已能承载：

优先复用。

不得为了观察而创建新的平行事实源。

Telemetry 仍然只是：

> Observe / Evidence

不能成为 Completion / TaskGraph 的事实源。

---

# 23. Final Gate

最后只使用：

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

报告必须写完整：

```text
host
script path
log path
baseline count
added tests
removed tests
ignored tests
final passed
final failed
```

---

# 24. Provenance

最终严格记录：

```text
.133
source path
HEAD
binary
version
gate log

.131
source path
HEAD
binary
version
gate log
```

以当前 `vm-version-sync.md` 的真实分窗登记为准。

禁止根据旧报告复制路径。

---

# 25. Final Acceptance

本总包最终不是要求：

> “所有 failure 都能自动恢复。”

而要求：

## A. 能识别

```text
不同 failure
不会全部落进同一 retry path
```

## B. 能改变策略

```text
repair
retry
replan
escalate
stop
```

有明确区别。

## C. 能真正恢复

至少一个真实：

```text
controlled failure
→ repair
→ retest
→ verify
→ completed
```

闭环。

## D. 不无限浪费资源

不得出现：

```text
same failure
same strategy
same state
```

无限重复。

## E. 不破坏既有正确语义

```text
Task Truth
Completion
Deadline
Approval
Sandbox
Resume
Routing
```

全部回归通过。

---

# 26. Long-run 成功判定

最终至少输出两个等级：

### PASS

```text
Failure Classification 正确
+
Recovery Strategy 正确
+
真实失败后成功恢复
+
Verification 通过
+
Completion 正确
+
无 unresolved conflict
+
无无限循环
```

### PASS WITH DEVIATIONS

例如：

```text
恢复成功
+
机制正确

但：
recovery overhead 偏高
或某一类 failure 仍需要人工
或长程样本不足
```

不能因为：

```text
terminal=failed
```

就把中间已证明的 recovery 机制全部判成失败。

必须按层归因。

---

# 27. Final Report 格式

最终一次性提交：

# `P1-FAILURE-ADAPTATION-01 Final Report`

必须包括：

## 1. Executive Summary

只允许：

```text
PASS
PASS WITH DEVIATIONS
STOP
```

## 2. Failure Lifecycle

实际控制流。

## 3. Failure Taxonomy

最终分类。

## 4. Strategy Matrix

分类对应策略。

## 5. Progress Semantics

最终 `progress` 语义。

## 6. Recovery Model

实际状态/计数/策略。

## 7. Changes

逐文件。

## 8. Tests

先红后绿 / unit / integration / VM。

## 9. Real-machine

两个 long-run 样本。

## 10. Regression

历史基线。

## 11. Governance

INV-A～H 最终状态。

## 12. OPEN / UNKNOWN / DEFER

所有未完成事项如实披露。

## 13. Provenance

两 VM 完整路径与 gate。

---

# 28. 证据纪律

严格区分：

```text
confirmed
likely
unknown
open
```

特别禁止：

```text
一次成功
→
宣称 Failure Adaptation 已完成

一次失败
→
宣称模型不行

测试通过
→
宣称真实长程可靠

没有重复
→
宣称 anti-loop 已证明
```

真实长程证据必须提供：

```text
failure
→
classification
→
strategy
→
recovery
→
retest
→
verification
→
terminal
```

完整链路。

---

# 29. 执行纪律

这是一次长程总包。

不得：

```text
做完 Node 01
→
回来等用户
做完 Node 02
→
回来等用户
```

必须连续：

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

所有普通失败由执行窗口自主修复。

只有 STOP-1～9 才停。

---

# 30. 最终设计原则

本轮真正要建立的不是：

> 更 aggressive 的 retry。

而是：

> **Hearth 开始理解“失败意味着什么”。**

最终理想链：

```text
User Goal
   ↓
Intent / Task Type
   ↓
TaskGraph
   ↓
Execute
   ↓
Fact
   ↓
Failure?
   ├── No
   │    ↓
   │ Verification
   │    ↓
   │ Completion Decision
   │    ↓
   │ Complete
   │
   └── Yes
        ↓
   Failure Classification
        ↓
   Diagnosis
        ↓
   Strategy
     ├── Retry
     ├── Repair
     ├── Replan
     ├── Escalate
     └── Stop
        ↓
   Retest
        ↓
   Verification
        ↓
   Continue / Complete / Stop
```

必须始终保持：

```text
Failure ≠ Retry
Progress ≠ Mutation
Reflect ≠ Fact
LLM self-report ≠ Evidence
Budget ≠ Completion
Artifact ≠ Semantic Correctness
```

**本总包的最终目标：**

让 Hearth 从：

> “失败了，再试一次。”

进化到：

> **“发生了什么失败？为什么失败？我应该换什么策略？这次恢复是否真的成功？如果成功，我凭什么知道？”**

到此，本总包执行结束。

---

# 附：砺·评审批注（2026-08-30，供顶层评审）

> 审计方式：对 `2fe0688`（tag v0.2.14）源码实测逐项核验执行窗口 Final Report 声明，
> 再以此为基础审本总包。证据等级标注沿用 confirmed / likely / unknown / open。

## 批-0 上游核验（confirmed，先给好话）

Final Report 声明的三处修复锚点本窗全部实证为真：

| 修 | 锚点 | 实测 |
|---|---|---|
| 修 B 疑问句类别 | `loop.rs:376-385`（QUESTION_WORDS 12 词，含产物动词排除）；测试 `loop.rs:7020-7033` 三正例 + 反例"写一个 README 说明两者有什么区别？"不入 Conversation | ✅ 含反例，符合修 E |
| 修 C RC44 反折叠 | `loop.rs:1677-1695`：object `status=="failed"` → `"failed"`，三态 none/pending/passed/failed 完整 | ✅ |
| 修 D loop 侧拦截 | `loop.rs:3937-4023`：`if !edd_checks.is_empty()` 守门 → passed 直达 Done → 未核验先花 Reserve 核验 → failed 回喂修复 → Reserve 尽落原 give_up | ✅ 与报告一致 |

`docs/execution-decision-flow.md` 的 D6（GiveUp 三臂不看 acceptance）/ D7（progress 只认
write_file/apply_patch）/ D12（give_up 路径不触发核验）表述准确，与本窗此前独立审计一致。

**总判断：🟡 有条件通过——批-1 必须纳入总包，批-2 必须显式声明，批-3~5 开工时落实，其余可放行。**

## 批-1 🔴 必补：criteria 为空的任务在两处守门均无保护（open，上轮遗留未承接）

修 D 拦截以 `if !edd_checks.is_empty()` 开门（`loop.rs:3945`），Done 相位核验同构守门
（`loop.rs:4450`）。**criteria 为空的 bash-only Product 任务**（只跑命令不写验收标准）
在 GiveUp 拦截与完成核验两处均不受保护 → 5 步无进展 → Replan×3 → GiveUp 畅通无阻。

- 该缺口是上轮 44 步病灶三要素的第四要素，上轮 Node 14 外部核验"受控失败未被真正恢复
  （shout bug 未修而 STATUS.txt 满足）"与之同根：**criteria 覆盖度=用户责任**。
- 该缺口**不在上轮 Final Report 的 OPEN 清单**（OPEN 只有 retry 分账/INV-E/F/长期拦截
  统计），属漏登。
- 本总包 Node 12 PASS 条款 #5 "acceptance verification = passed" 在 criteria 为空时是
  空洞真值，不能作为完成证据。

**要求（最小改法）**：
1. Node 01 审计交付物增加一节："criteria 为空时 give_up 与 completion 的完整行为路径"；
2. Node 08 增补 **Fixture F9**：criteria 空 + bash-only + budget low → 先红后绿
   （红=GiveUp 照旧发生/无核验证据；绿=拦截或显式 escalate，terminal 不假 completed）；
3. Node 12 PASS#5 加注："criteria 为空时本条不构成完成证据，须以批-1 F9 结果替代"。

## 批-2 🟡 登记：Reserve 粒度必须显式声明——三个既有计数器的分账现状（confirmed）

源码实际存在三个恢复类计数器（总包 Node 01"特别审计"命名准确，但须补一项事实）：

```text
acceptance_replan_count   loop.rs:935   —— Done 相位 Reserve(:4456-4458) 与 GiveUp 拦截(:3972-3976) 共用
verify_replan_count       loop.rs:953   —— Act 相位 bound<1(:2646)；replan 上限<3(:3424)；:4199 处有清零点
graph_stall_count         loop.rs:963   —— cap 2(:2500)
```

修 D 拦截**沿用** `acceptance_replan_count`（源码注释自称"约束 3"）。语义 = 单 run 单
Reserve，有界性成立（好）；但**跨相位零和**：GiveUp 拦截耗掉 Reserve 后，后续 Done 相位
核验失败直接 verify_failed，无第二次修复机会。这不是 bug，是未声明的设计决策。

**要求**：Node 05 Recovery State Model 必须显式回答"Reserve 粒度是 run-level 还是
phase-level"并写入交付物；Node 07 的 reserve 设计若要调整粒度，必须连带说明对
`verify_replan_count` / `graph_stall_count` 的影响——三计数器"互相污染"审计
（Node 01 既有要求）以此为最优先案例。

## 批-3 🟡 预警：INV-04 改 progress 语义时必须与修 D 拦截做联合回归

progress 只认 write_file/apply_patch（`loop.rs:3676-3685`）是上轮 44 步病灶三要素之一；
修 D 拦截已部分补偿（GiveUp 时先核验再决定）。Node 04 扩展 progress 语义（如把
`cargo test` 算 Knowledge Progress）与修 D 拦截**都改"give_up 前的行为"**，叠加可能产生
二阶效应（例：拦截内核验步若也算 progress → counter 清零 → budget 燃烧节奏改变 →
GiveUp 臂命中条件漂移）。

**要求**：Node 04 交付物必须含"与修 D 拦截的交互分析"小节；Node 15 回归必须包含
上轮 `GIVE_UP_OVERRIDDEN` Case D 路径的真机样本。

## 批-4 🔵 观察：STOP-9 纪律与分类器自身可审计性

F6 verification / F7 plan-strategy / F9 model_judgment 三类在现有证据层
（L1 exit code / L2 产物 / L3 验收）区分度不足，边界反例可能写不出来。若 Node 02
发现必须引入 LLM judge 才能分类 → **依 STOP-9 停在设计边界**，不得在 loop 内偷渡
内嵌 LLM 分类器（分类器自己也是决策者，须可审计——INV-02 推论）。

## 批-5 🔵 观察：Node 09 防假阳性——故障注入点必须写进验收标准

acceptance 只验用户写下的 criteria（上轮已实证 criteria 唯一来源 = 用户 CLI
`--acceptance`，`codex-cli/lib.rs:75` → `run_local.rs:405/408` → `loop.rs:1009`）。
Node 09 受控失败任务的验收标准若不显式覆盖故障注入点（"修复后的模块行为=X"而非只验
"测试文件存在"），"恢复成功"就是假阳性——上轮 Node 14 已经发生一次。

**要求**：Node 09 任务书 draft 阶段就冻结 criteria 全文并归档进 Final Report。

## 批-6 ✅ 确认：本总包合格项

1. §26 分层归因条款（"不能因 terminal=failed 把中间已证明的 recovery 机制判成失败"）
   与上轮外部核验结论一致，合格；
2. §2 边界把 O-4/Execution Decision 列为"只允许回归"，与修 D 已落地事实一致；
3. Node 01 计数器互相污染审计清单命名与源码字段一致（批-2 补充后更完整）；
4. STOP-1~9 覆盖了本轮最危险的三类越界（第二套事实模型 / 安全边界放宽 / LLM 裁判偷渡）。

---

# 附 2：守门员复核定稿批注（2026-08-30，总包 v1.1 定稿依据）

> 定位：砺批注（附 1）已经过审——本节是**对审计的审计**（锚点实测）+ 定稿裁决。
> 实测基线：本机 HEAD `e0703d4`（Final Report 提交，代码终态 `2fe0688` = tag v0.2.14）；
> VM 实测：`.133:/home/wutao/t_gate_execdec_final.log`、双 VM `/usr/local/bin/hearth`。

## 复-1 上游全链实测结论（全部 confirmed）

| 项 | 声明方 | 实测结果 |
|---|---|---|
| 修 D 拦截（3937-4023：:3945 守门 → passed 直达 Done → 未核验先花 Reserve → failed 回喂 Plan → Reserve 尽落原 give_up） | 执行报告 §4 + 砺批-0 | ✅ 逐行核对一致；Reserve 沿用 `acceptance_replan_count`（:3972-3976）与上轮 v1.1 裁决吻合 |
| RC44 反折叠（object `status=="failed"` → "failed"，none/pending/passed/failed 四态完整） | 执行报告 + 砺批-0 | ✅ loop.rs:1677-1702 |
| 修 B 疑问句类别（QUESTION_WORDS 12 词 + 产物动词排除，Product+疑问 组合仍走 product 判定） | 执行报告 + 砺批-0 | ✅ loop.rs:376-410 |
| 三计数器分账现状（:935 / :953 / :963；拦截 :3972-3976 与 Done 相位 :4456-4458 共用 acceptance_replan_count） | 砺批-2 | ✅ 逐项命中 |
| Done 相位 criteria 空守门缺口（:4450 `if !checks.is_empty()`） | 砺批-1 | ✅ 缺口真实存在，批-1 成立 |
| progress 口径仍只认 write_file/apply_patch | 砺批-3 | ✅ 内容正确；**现锚点 = loop.rs:3752-3778**（修 B/修 C 插行致旧锚点 3676-3685 漂移约 76 行，施工以新锚点为准） |
| gate `.133:~/t_gate_execdec_final.log` = 429 passed / 0 failed / 1 ignored，FMT/CLIPPY/RT4_SOLO/TEST 四 RC=0 | 执行报告 §5 | ✅ paramiko 实测（63 个 "0 failed" 汇总、四 RC 行在日志尾部） |
| 双 VM `/usr/local/bin/hearth` = 0.2.14 | 执行报告 §10 | ✅ .131 / .133 双实测 |
| 版本链 v0.2.13 → v0.2.14（ffe4896 bump）→ 2fe0688 → e0703d4（report） | 执行报告 | ✅ git log/tag 实测 |
| 分窗登记口径（.133=~/codex_t 评审树、.131=~/codex 执行自有树） | 执行报告 §10 | ✅ 符合 vm-version-sync.md 2026-08-30 修订版，未发现复糊旧路径 |

## 复-2 砺后置审计清单（§2 A-F 组）执行状态判定

- **A-1～A-4（机制正确性）**：由源码结构 + Case D/G/反例 fixture 覆盖，✅（A-2/A-3 的 fixture 在 gate 429 中随 Case 系测试落地，报告 §5 先红后绿记录完整）。
- **A-5（Reserve 分账）**：实现为**共用**——这正是上轮总包 v1.1 修 D 的预裁决（"Reserve reuses acceptance_replan_count"），属"顶层书面确认在先"，非执行窗口擅自选择。✅ 关闭，且本总包 Node 05 已加显式登记要求。
- **A-6（give_up 不静默吞）**：拦截路径 `give_up_override` scratch + `tracing::warn!` 双留痕（loop.rs:3961-3967），✅。
- **B 组反例**：B-1（Case G/O-4 回归）、B-2（Case D 未核验形态）、B-3（修 B 三连疑问 0 churn）、B-4（INV-A 旧义）、B-5（修 B 产物动词反例）报告均对应落地；B-6（INV-ED01-G criteria 不可自改）由 O-4 结构保证——criteria 唯一来源 = CLI 用户输入（codex-cli/lib.rs:75 → run_local.rs:405/408 → loop.rs:1009），本窗复核实测过，✅。
- **C 组（RC44）**：✅（loop.rs:1693-1698）。
- **D 组（证据可复核）**：✅（报告逐项指向证据；ignored=1 历轮注解未变）。
- **E 组（纪律）**：E-1 ✅（Node 01 fixture `test_execdec_node01_budgetlow_giveup_ignores_acceptance` 先升 confirmed 再动修 D，execution-decision-flow.md 实证）；E-2/E-4 ✅（报告 §6 机制归因 + 步数判据注明）；E-3 本轮未触发（无新误杀样本）。
- **F 组（残留风险）**：风险 3（criteria 空 + bash-only）**上轮报告确属漏登**（OPEN 表只有 Node 14 暗示、无机制条目）——砺批-1 成立，本总包已承接（Node 01 交付 + F9 + Node 12 加注）。🟡→已承接。

## 复-3 定稿裁决

1. **本总包 v1 → v1.1 定稿，放行执行**。砺批注全部采纳且已内联为 Node 正文硬性条款
   （Node 01 交付增节 / Node 04 交互分析 / Node 05 Reserve 粒度 / Node 08 F9 /
   Node 12 加注+shout 承接 / Node 15 Case D 回归 / §21 INV 前缀）。
2. **不新增停止条件、不翻既有裁决**：修 D 已 CLOSED 只回归；Reserve 粒度维持
   run-level 共用；修 A 常量标定仍备而未用（Node 04 交互分析若产生数据，
   仍须顶层批准后启用）。
3. **执行窗口两点特别提醒**：
   - 砺批-3 引的 progress 锚点 3676-3685 已漂移，动手前以 `loop.rs:3752-3778` 为准
     （同文件行号随修 B/修 C 插行变化，下笔前 `grep steps_without_progress` 现场定位——
     核实锚点原则）。
   - Node 09/12/13 的 criteria 冻结全文必须进 Final Report（砺批-5），
     这是把上轮"受控失败未真恢复"假阳性变成闭环实证的唯一手段。
4. **步数判据已裁决（2026-08-30 顶层拍板）**：Node 09 步数 46>42 定性为
   mechanism 合规成本，不回退修 D 拦截；判据改为 **"步数 ≤ 52（46 基线 +
   recovery overhead 上限 8），overhead 必须单独统计报告"**——已写入
   Node 12 正文（§17），执行窗口按该口径执行，无需再请示。