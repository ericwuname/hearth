# Hearth Core 蓝图 v2

## 从 Agent Harness 到可信委托核心

**版本**：v2
**日期**：2026-08-30
**性质**：顶层架构路线蓝图 / 设计基线
**用途**：作为后续施工总包的上位设计依据；本文件本身不构成施工授权。

---

# 0. 蓝图使命

Hearth 的目标不是：

> 做一个比 Codex 功能更多的 CLI。

也不是：

> 让 LLM 显得更像一个“有自我”的存在。

当前核心目标已经收敛为：

# **构建一个可以长期委托、能够正确执行、正确验证、正确恢复、正确结束，并且不会轻易把自己的判断当成事实的 Agent Harness。**

核心问题不是：

> “模型有多聪明？”

而是：

> **系统是否能够在不确定、失败、长程执行和上下文变化中，持续保持正确的事实、正确的决策和清晰的终态。**

---

# 1. Hearth Core 九阶段主链

当前蓝图正式收敛为：

```text
                         USER INPUT
                              │
                              ▼
                    ┌──────────────────┐
                    │  ① INTENT        │
                    │  先分类，再行动   │
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │  ② PLAN          │
                    │  如何完成目标     │
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │  ③ EXECUTION     │
                    │  实际执行工具     │
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │  ④ FACT          │
                    │  客观发生了什么   │
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │  ⑤ VERIFICATION  │
                    │  证据是否支持     │
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │  ⑥ REFLECTION    │
                    │  模型如何解释     │
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │  ⑦ DECISION      │
                    │  接下来做什么     │
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │  ⑧ TERMINAL      │
                    │  最终状态是什么   │
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │  ⑨ PROJECTION    │
                    │  用户看到什么     │
                    └──────────────────┘
```

这九层不是九个必须独立的 crate，而是**九种职责**。

允许实现共享模块，但职责不能混淆。

---

# 2. 四条 Core Invariants

当前蓝图的基础公理收敛为四大类。

---

## INV-01：先分类，再行动

任何用户输入必须先判断：

```text
Task
QA / Discussion
Task Control
Conversation
Goal Mutation
Ambiguous
```

之后才能决定：

```text
Planner
TaskGraph
Direct Answer
InteractionRequest
```

禁止：

```text
User Input
↓
先进入 TaskGraph
↓
再猜它是不是任务
```

核心原因：

> **错误的 Intent 会污染后面的全部阶段。**

---

## INV-02：不得自审

生产系统不能仅凭自己的自述宣布最终正确。

必须始终区分：

```text
Statement
Evidence
Verification
Decision
```

严格成立：

```text
LLM self-report
≠
Verification Evidence

Artifact exists
≠
Semantic correctness

Tool success
≠
Goal satisfied

Reflect
≠
Fact
```

这一原则已经由 O-4 / P1-TASK-TRUTH / P1-EXECUTION-DECISION 的真实样本证明其必要性。

---

## INV-03：Failure ≠ Retry

失败不是动作指令。

必须：

```text
Failure
↓
Classification
↓
Strategy
```

而不能：

```text
Failure
↓
counter++
↓
retry
```

恢复策略可能是：

```text
Retry
Repair
Replan
Escalate
Stop
```

具体策略必须与 failure 类型相关。

---

## INV-04：Progress ≠ Mutation

文件修改不是唯一进展。

以下都可能产生真实 progress：

```text
Mutation
Verification
Knowledge gain
Task-state transition
Acceptance evidence
```

但当前尚未建立多套生产级 progress counter。

第一原则是：

> **先定义“什么叫任务状态发生有意义变化”，再决定如何计数。**

---

# 3. Evidence Hierarchy

Hearth 的事实体系正式采用：

```text
L0 — Model Statement
      模型自己说了什么

L1 — Tool Evidence
      exit code / structured result / tool outcome

L2 — Artifact Evidence
      文件存在 / 内容 / 工作区事实

L3 — Verification Evidence
      确定性检查、测试、验收核验

L4 — Acceptance Evidence
      用户声明的结构化验收标准是否满足
```

核心原则：

> 越靠下，越不能被上层主观语言推翻。

典型关系：

```text
LLM: “test passed”
        ↓
cargo test exit = 1
        ↓
L1 > L0
```

因此最终 Completion 必须建立在：

```text
Fact
+
Verification
+
Acceptance
```

而不是：

```text
Reflect says completed
```

---

# 4. Completion Authority

当前已经形成的正式权力关系：

```text
                 FACT
                  │
                  ▼
             VERIFICATION
                  │
                  ▼
        COMPLETION READINESS
                  │
       ┌──────────┴──────────┐
       ▼                     ▼
     READY                NOT READY
       │                     │
       ▼                     ▼
   Completion             Reflection
                             │
                   ┌─────────┴────────┐
                   ▼                  ▼
                 Replan            GiveUp
```

关键原则：

> **GiveUp 是一种执行建议，而不是天然的最终终态。**

如果：

```text
acceptance = passed
+
required verification = passed
```

那么 Reflect / Planner / Budget 不得无依据地把任务重新标成未完成。

这就是 P1-EXECUTION-DECISION-01 已经正式实现的核心。

---

# 5. DECISION 阶段最终模型

Decision 不再简单理解为：

```text
Continue
Replan
Complete
```

概念上应支持：

```text
Continue
Replan
Complete
Escalate
Stop
```

其中：

* `Escalate` 优先复用已有 `InteractionRequest`
* 不新增第二套 Human Interaction Framework
* 是否需要人工参与，取决于 failure / uncertainty / authority boundary
* 已存在的 delegation scope 应被 Decision 消费，而不是重新定义权限体系

---

# 6. INTENT 层

Intent 不再是一个简单的：

```text
goal_requires_product()
```

它是整个 Execution Gate。

最终希望形成：

```text
User Input
↓
Intent Classification
↓
Execution Eligibility
↓
Route
```

必须重点覆盖：

```text
纯任务
纯问询
纯聊天
任务控制
模糊指令
情绪表达
自我指涉问题
情绪 + 任务混合输入
约束 + 任务混合输入
```

特别关注：

> **“不想继续做 / 不好用 / 你是谁 / 现在怎么样”等输入不能因为碰巧包含任务词就进入 TaskGraph。**

未来 Intent 验收必须包含 adversarial / tricky input，而不仅仅是正常指令。

---

# 7. PLAN 层

Planner 的职责：

> **规划，不是事实裁判。**

Planner 可以：

```text
decompose
replan
propose give_up
```

但不能：

```text
覆盖确定性事实
绕过 verification
单独宣布 semantic completion
```

Planner 的 GiveUp 已经证明需要经过 Execution Decision 层二次检查。

因此未来：

```text
Planner output
```

应该理解成：

> **Decision proposal**

而不是：

> **Final authority**

---

# 8. EXECUTION 层

Execution 是工具实际发生事情的位置。

必须保持：

```text
Tool Runtime
Sandbox
Approval
Deadline
Resource
Cancellation
```

这些能力的边界清晰。

当前已经闭合：

* ApprovalPolicy / delegation
* HardRedline
* Task deadline
* Tool-level timeout
* Cancellation
* Sandbox
* Landlock
* Seccomp
* ResourceLedger（Observe）

未来继续维护：

> **执行能力必须受 Harness 控制，不由 LLM 自行获得。**

---

# 9. FACT 层

这是 Hearth 当前最重要的基础资产之一。

FACT 的任务：

> **记录客观发生的事情。**

例如：

```text
tool exit code
write result
file state
test result
acceptance result
artifact snapshot
task state transition
```

必须避免：

```text
Model said X
```

直接成为：

```text
Fact = X
```

---

## FACT 的长期原则

# Statement ≠ Fact

未来任何进入 Completion / Verification / Decision 的事实，都应该有：

```text
source
provenance
timestamp / sequence
scope
```

能够回答：

> “这条事实从哪里来的？”

---

# 10. VERIFICATION 层

当前已经有：

```text
Tool Evidence
Artifact Evidence
Acceptance Verification
verify_failed
```

并且 O-4 已经证明：

```text
LLM self-report
```

可以与：

```text
真实测试结果
```

发生冲突，而系统仍然能够依赖确定性证据。

---

## Verification 长期方向

验证优先级：

```text
Compiler / Test Runner
>
Filesystem / OS
>
Structured Tool Result
>
Static / deterministic rule
>
LLM judgment
```

这里不是要求所有问题都必须 deterministic。

而是：

> **凡是可以客观检查的，就不要让模型自己当裁判。**

未来应统计：

```text
Deterministic Verification Ratio
```

而不是一开始硬设目标值。

---

# 11. REFLECTION 层

Reflection 的定位正式收敛为：

> **解释与建议，不是事实来源。**

未来 machine-consumed Reflection 应逐渐结构化为：

```text
{
  observed_facts,
  interpretation,
  uncertainty,
  conflicts,
  rationale,
  proposed_action
}
```

而不是完全依赖：

> “我觉得……”

### 但不禁止第一人称本身。

真正禁止的是：

> 主观表述未经验证就充当事实。

例如：

```text
“我感觉应该已经完成”
```

不能成为完成证据。

但：

```text
“我无法确认测试是否通过”
```

可以作为 uncertainty。

因此：

> **结构化优先，而非文风禁令。**

---

# 12. REFLECT × FACT 冲突模型

如果出现：

```text
Fact
≠
Reflect interpretation
```

不能直接视为 failure。

应该：

```text
Detect
→
Record
→
Classify
→
Resolve
```

冲突分类至少包括：

```text
progress misread
completion misread
task-type error
verification omission
budget artifact
unknown
```

关键最终标准不是：

> “从来没有冲突。”

而是：

> **不存在 unresolved conflict。**

---

# 13. Progress Semantics

当前已知最大问题之一：

```text
write_file
apply_patch
→ progress

bash / test / read / verification
→ no progress
```

这会产生：

```text
代码修复
↓
大量测试/验证
↓
系统认为没有进展
↓
steps_without_progress ↑
↓
GiveUp
```

因此正式建立：

# Progress ≠ Mutation

下一步研究：

```text
Mutation progress
Verification progress
Knowledge progress
Task-state progress
```

但第一阶段不建立四套 counter。

应首先回答：

> `steps_without_progress` 到底要表达什么？

---

# 14. FAILURE 层

Failure 必须成为一等公民。

基本链：

```text
Failure
↓
Classification
↓
Diagnosis
↓
Strategy
↓
Recovery
↓
Retest
↓
Verification
```

最小 Failure Taxonomy：

```text
F1 transient provider
F2 tool execution
F3 environment
F4 permission / approval
F5 assertion / test
F6 verification
F7 plan / strategy
F8 resource / budget
F9 model judgment
F10 unknown
```

分类优先使用：

```text
structured error
typed result
tool status
exit code
```

只有没有结构化信息时才允许有限 fallback。

---

# 15. RETRY / REPAIR / REPLAN / ESCALATE

这四类动作严格分离。

### Retry

预计原操作仍然有效，只是发生暂态失败。

例如：

```text
429
network reset
5xx
```

### Repair

已有事实指出某个实现存在问题。

例如：

```text
test failed
→ inspect
→ modify
→ test again
```

### Replan

当前计划已经失效。

例如：

```text
原依赖不存在
原架构假设错误
当前任务路径不成立
```

### Escalate

系统无法在既定 authority / evidence 下安全做决定。

优先通过：

```text
InteractionRequest
```

实现。

---

# 16. Anti-Repetition

一个核心恢复原则：

```text
same failure
+
same state
+
same strategy
```

不能无限循环。

至少应该存在：

```text
repeat detection
+
bounded attempts
+
strategy change / replan / escalate / stop
```

注意：

> 不要为了反循环而简单设置一个更小的 retry counter。

真正要检测的是：

> **有没有产生新的信息、状态或策略变化。**

---

# 17. Budget

Budget 是：

> **资源约束。**

Budget 不是：

> “任务是否完成”的事实。

因此：

```text
budget = 0
```

不能自动解释为：

```text
task failed
```

它只能表示：

> 没有更多资源可以继续尝试。

---

## Verification / Recovery Reserve

允许存在有限 reserve：

```text
Total Budget
├── Execution
└── bounded Recovery / Verification Reserve
```

禁止：

```text
Original Budget
+
unlimited free verification
```

Reserve 必须：

```text
bounded
deadline-aware
attempt-limited
auditable
```

---

# 18. TERMINAL

Terminal 九态保持：

```text
completed
failed
give_up
verify_failed
timeout
cancelled
...
```

本蓝图不重新设计 Terminal。

重要原则：

> **Terminal 表示最终系统状态，不是某个中间模块的个人意见。**

最终状态必须带：

```text
reason
decision source
verification state
remaining work
```

Projection 不得把失败洗成成功。

---

# 19. PROJECTION

Projection 的职责：

> **把后端事实投影给用户。**

必须保证：

```text
Backend
→
Fact
→
Projection
```

而不是：

```text
Backend log
→
随机打印到终端
```

注意：

> tracing 本身不是问题。

问题是：

> **用户可见输出绕过结构化 Projection。**

因此以后验收检查：

```text
user-visible terminal output
```

是否存在内部错误日志直接泄漏。

例如：

错误：

```text
2026... ERROR hearth::loop: permission denied
```

正确：

```text
✗ 工具出错
  reason: permission denied
```

---

# 20. MEMORY / CONTEXT

Fact 与 Conversation History 必须分离。

这是下一阶段最重要的原则之一：

```text
Conversation History
→
可以压缩

FACT / VERIFICATION
→
不能随普通 Conversation Compression 丢失
```

因此最终应该：

```text
Conversation Store
+
Fact Store
+
Verification Store
```

不一定是三个物理文件，但必须有三个清晰生命周期。

---

# 21. ContextBuilder

当前 R2-C 已闭合：

```text
L1 Stable
L2 Task-stable
L3 Task facts
L4 Dynamic
L5 Recovery
```

未来原则：

> **ContextBuilder 负责组装，不负责产生事实。**

尤其：

```text
TaskGraph
RunState
Fact
Verification
Memory
```

是数据源。

ContextBuilder 是投影/组装器。

不能产生第二套事实模型。

---

# 22. Compaction

未来 Compaction 设计必须遵循：

```text
History
→
compressible

Fact
→
non-lossy

Verification
→
non-lossy

Task Continuity
→
non-lossy
```

任何：

> “压缩之后模型不知道刚才干到哪里了”

都应该视为：

> **Context / Memory invariant violation**

而不是单纯模型遗忘。

---

# 23. Constant Calibration

当前已经发现大量遗留常量：

```text
32000 compaction
4096 sub-agent context
200 / 60 / 8000 / 6000 / 4096 silent truncation
```

以及：

```text
Agnes 512K
```

之间存在明显环境代差。

未来不采用：

> “看到数字就调大。”

而采用：

# Calibration Provenance

每一个关键常量都必须能够回答：

```text
是什么？
为什么是它？
依据什么？
哪一天标定？
针对什么环境？
现在是否仍有效？
```

---

# 24. Calibration Workflow

未来：

```text
Constant
↓
Baseline
↓
A/B evidence
↓
Behavior comparison
↓
Decision
↓
Change
```

禁止：

```text
感觉太小
↓
直接调大
```

尤其 Compaction：

> 强相关不等于因果。

因此 W5 / Memory 阶段先验证：

```text
threshold
→
compression frequency
→
memory retention
```

再决定修改。

---

# 25. Telemetry

Telemetry 是：

> **观察系统。**

不是：

> 事实来源。

已经形成的核心观测：

```text
cache telemetry
resource ledger
msg chain
tool schema hash
phase
```

未来新增：

```text
goal_revision_count
execution_turn_count
goal_revision_rate

deterministic_verification_count
model_verification_count
deterministic_verification_ratio

failure_class
recovery_strategy
strategy_changed
recovery_attempt

reflect_fact_conflict
```

第一阶段原则：

> **Observe first，threshold later。**

不提前给所有指标发明阈值。

---

# 26. Architecture Fitness

未来把关键原则变成可失败的检查：

```text
Architecture Invariant
↓
Automated Check
↓
CI
↓
Violation
↓
Fail
```

优先检查：

```text
INV-01 先分类再行动
INV-02 不得自审
INV-03 Failure ≠ Retry
INV-04 Progress ≠ Mutation
INV-05 Fact 不随 Compression 丢失
```

原则：

> **存量违规先建 baseline，只阻止新增违规。**

避免一次性把整个旧仓库打红。

---

# 27. Evaluation / Observer

Observer OS 继续保持：

```text
Zero Execution Authority
```

它未来适合承载：

```text
Trajectory evaluation
Behavior benchmark
Friction metrics
Architecture fitness
Near-miss detection
Incident learning
```

但：

> **当前不立即施工成完整 Evaluation Platform。**

先完成 Core。

---

# 28. Incident Learning

顶层规划窗口提出的：

```text
Incident
→
Classification
→
Near-miss
→
Root Cause
→
Five Whys
→
Governance Change
→
Architecture Fitness
```

是长期正确方向。

但现在只作为：

> **Governance Input**

不作为 Core 当前施工项。

---

# 29. Near-miss

未来不仅记录：

```text
真的失败了
```

还要记录：

```text
差点失败
但被某个保护机制救回来
```

例如：

```text
GiveUp proposal
↓
Completion readiness intercept
↓
completed
```

这是非常有价值的 near-miss。

它证明：

> **系统原本差点做出错误决策。**

这种样本比成功样本更适合改进架构。

---

# 30. D10：不得自审的长期治理含义

最终治理层原则：

> **被测系统不能独自决定自己是否达标。**

因此：

```text
Production System
    → 产生事实

Verification
    → 验证事实

Observer
    → 独立观察

Governance
    → 最终接受/拒绝
```

以后真正的 Acceptance 不应只由：

```text
施工窗口自己宣布
```

完成。

至少最终 Core Freeze 阶段需要：

```text
独立 regression
+
behavioral evidence
+
observer / reviewer
```

---

# 31. Strong Acceptance

以后所有核心施工均区分：

## Weak Acceptance

```text
能编译
测试通过
命令跑完
有输出
```

## Strong Acceptance

```text
旧故障
→
新机制
→
真实行为改善
→
回归无破坏
→
长程任务
→
证据闭合
```

真正的 Core Acceptance：

> **Strong Acceptance 优先。**

---

# 32. 当前开发路线

当前已经完成：

```text
R2-C ContextBuilder              ✅
W3/W4                            ✅
RC24                             ✅
W8                               ✅
P1-LTR-01                        ✅
P1-CONSOLIDATION-01              ✅
P1-TASK-TRUTH-01                 ✅
P1-EXECUTION-DECISION-01        ✅
```

当前下一阶段：

```text
P1-FAILURE-ADAPTATION-01         ← 当前
```

之后：

```text
P2-MEMORY-CONTEXT-01
```

然后：

```text
P2-LONG-RUN-ACCEPTANCE-01
```

最后：

```text
HEARTH CORE FREEZE
```

---

# 33. P1-FAILURE-ADAPTATION-01 的目标

本阶段只解决：

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

特别处理：

```text
Progress ≠ Mutation
Failure ≠ Retry
```

并使用：

> P1-CONSOLIDATION / P1-TASK-TRUTH / P1-EXECUTION-DECISION 的长程失败样本作为 canonical evidence。

---

# 34. P2-MEMORY-CONTEXT-01 的目标

仅在 Failure Adaptation 核心闭合后进入。

范围：

```text
Memory
Compaction
Constant Calibration
Silent Truncation
Fact Persistence
Verification Persistence
Context Continuity
Sub-agent Context Audit
```

不在此之前施工。

---

# 35. P2-LONG-RUN-ACCEPTANCE-01

目标不是：

> “又跑一次长任务。”

而是最终证明：

```text
Intent
→
Plan
→
Execute
→
Fact
→
Verify
→
Reflect
→
Decision
→
Recover
→
Memory
→
Resume
→
Complete
```

能够跨：

```text
20+
50+
长程
失败
恢复
压缩
resume
```

稳定运行。

---

# 36. Core Freeze

Core Freeze 前最终必须回答：

### 输入

```text
“这是任务吗？”
```

### 执行

```text
“真正发生了什么？”
```

### 事实

```text
“什么是客观证据？”
```

### 验证

```text
“为什么相信？”
```

### 决策

```text
“为什么继续 / 重规划 / 停止 / 完成？”
```

### 恢复

```text
“失败之后为什么这么做？”
```

### 连续性

```text
“压缩 / resume 后还知道自己做到哪里了吗？”
```

### 终态

```text
“到底完成了没有？”
```

### 投影

```text
“用户能不能明确知道发生了什么？”
```

如果这些问题全部能够得到：

> **可观察、可验证、可复现的答案**

才进入 Core Freeze。

---

# 37. 暂缓能力

在 Core Freeze 以前：

```text
Subagents
TUI
MCP
Bridge production integration
Desktop
Personality engineering
Long-form autonomous persona
```

全部：

# DEFER / QUARANTINE

不是否定价值。

只是：

> **当前地基优先。**

---

# 38. 当前四大未闭合领域

整个 Hearth Core 已经从“大量问题”收敛成四块：

```text
① Failure Adaptation
   失败以后怎么办

② Memory / Compaction
   长时间以后还记得什么

③ Long-run Reliability
   20~100 步是否稳定

④ Evaluation / Governance
   系统如何知道自己哪里有问题
```

其中：

```text
① 当前施工
② 下一施工
③ Core Freeze 前验收
④ 暂不大施工
```

---

# 39. 当前 Core 状态总图

```text
                         HEARTH CORE

       ┌───────────────────────────────────────────┐
       │               INPUT                       │
       │         Intent / Classification           │
       └──────────────────┬────────────────────────┘
                          ↓
       ┌───────────────────────────────────────────┐
       │                PLAN                       │
       │        Planner / TaskGraph                │
       └──────────────────┬────────────────────────┘
                          ↓
       ┌───────────────────────────────────────────┐
       │             EXECUTION                     │
       │ Tool / Sandbox / Approval / Deadline      │
       └──────────────────┬────────────────────────┘
                          ↓
       ┌───────────────────────────────────────────┐
       │                FACT                       │
       │     Objective Evidence / Artifacts        │
       └──────────────────┬────────────────────────┘
                          ↓
       ┌───────────────────────────────────────────┐
       │            VERIFICATION                   │
       │      Deterministic > Model Judgment       │
       └──────────────────┬────────────────────────┘
                          ↓
       ┌───────────────────────────────────────────┐
       │             REFLECTION                    │
       │       Interpretation / Uncertainty         │
       └──────────────────┬────────────────────────┘
                          ↓
       ┌───────────────────────────────────────────┐
       │              DECISION                     │
       │ Continue / Replan / Complete / Escalate   │
       └──────────────────┬────────────────────────┘
                          ↓
       ┌───────────────────────────────────────────┐
       │              TERMINAL                     │
       │     Completed / Failed / Timeout / ...    │
       └──────────────────┬────────────────────────┘
                          ↓
       ┌───────────────────────────────────────────┐
       │             PROJECTION                    │
       │       Clear / Structured / Auditable       │
       └───────────────────────────────────────────┘


       ════════════════════════════════════════════
                    CROSS-CUTTING
       ════════════════════════════════════════════

       Memory / Context
       Resource / Deadline
       Telemetry / Evidence
       Observer
       Architecture Fitness
       Incident Learning


       ════════════════════════════════════════════
                         END
       ════════════════════════════════════════════

                 LONG-RUN ACCEPTANCE
                          ↓
                     CORE FREEZE
                          ↓
                  新设计重新开放
```

---

# 40. 蓝图的最终原则

Hearth 最终不是：

> 一个更会说话的 Agent。

也不是：

> 一个拥有更多工具的 Codex clone。

而是：

# **一个能够把“行动、事实、验证、决策、恢复、连续性”闭合起来的可信委托系统。**

它应该能够做到：

```text
我知道自己在做什么
    ↓
我知道实际发生了什么
    ↓
我知道什么只是我自己的判断
    ↓
我知道什么已经被证据证明
    ↓
我知道什么时候应该继续
    ↓
我知道什么时候应该改变策略
    ↓
我知道什么时候应该询问人
    ↓
我知道什么时候已经真正完成
    ↓
即使中途失败，我也知道如何恢复
    ↓
即使上下文压缩，我也不会丢掉事实
```

最终形成：

# **Truth → Decision → Recovery → Continuity**

这四件事，是 Hearth Core 真正的地基。

---

# 41. 当前执行顺序

```text
① P1-FAILURE-ADAPTATION-01
       ↓
② P2-MEMORY-CONTEXT-01
       ↓
③ P2-LONG-RUN-ACCEPTANCE-01
       ↓
④ CORE FREEZE
       ↓
⑤ 重新开放新设计
```

在 Core Freeze 之前：

> **不因为新的“有趣想法”改变主路线。**

新的想法只能进入：

```text
Ideas Register
Quarantine
Future Governance Input
```

而不能直接进入施工队列。

---
