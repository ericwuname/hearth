# Hearth P2-LONG-RUN-ACCEPTANCE-01

## Core Freeze 前长程可靠性总包 v1.1

**日期**：2026-08-31（v1.1：砺批-0~8 内联 + 守门员复核定稿）

**日期**：2026-08-31
**性质**：Hearth Core 最终长程可靠性 / 综合验收总包
**执行方式**：连续自主执行 Node 00 → Node 15；普通故障自行诊断、修复、测试、复验；仅 STOP 条件允许停下等待顶层裁决
**最终交付**：一次性提交 Final Report，不在 Node 间等待用户反馈
**施工原则**：先验证现状，再做最小必要修复；不为了“全绿”扩大架构
**最终目标**：为 Hearth Core Freeze 提供 Strong Acceptance 证据

---

# 0. 当前工程位置

Hearth 当前已经完成：

```text
R2-D TaskGoal                         ✅
R2-C ContextBuilder                   ✅
W3/W4 Fact / Completion / Projection  ✅
RC24 Approval / Delegation            ✅
W8 Intent / Goal Routing              ✅
P1-LTR Deadline                       ✅
P1-TASK-TRUTH Verification            ✅
P1-EXECUTION-DECISION                 ✅
P1-FAILURE-ADAPTATION                 ✅
P2-MEMORY-CONTEXT                     ✅
```

当前版本：

```text
v0.2.16
tag v0.2.16
HEAD 253fcd1（含 docs）
代码基线 0e4a9d2
```

P2-MEMORY-CONTEXT 已完成（**PASS WITH DEVIATIONS / CLOSED**，v0.2.16，评审包
`hearth-p2-memory-context-01-review-pack-v1.md`；顶层裁定采纳砺批-0 建议），但保留：

```text
OPEN-1  archive recoverability 未证明
OPEN-2  MAX_HISTORY_MSGS=40 静默切片
OPEN-3  “继续”续轮 planner give_up
OPEN-4  chat archive session 隔离时序
```

P1-FAILURE-ADAPTATION 仍保留：

```text
OPEN-1 bash exit_code → ToolResult 完整结构化接入
OPEN-2 长程单样本 convergence variance
OPEN-3 progress semantics 扩展
OPEN-4 verify_replan_count 双上限设计
OPEN-5 大 tool output × low budget
```

本总包的使命：

# **不再继续堆叠能力，而是证明这些已经完成的机制组合起来以后，能否在长程真实任务中稳定工作。**

---

# 1. Core Strong Acceptance

本轮不用“能编译、能跑、能有输出”作为最终标准。

最终采用：

## Structural Acceptance

```text
fmt
clippy
test
sandbox
provenance
```

## Behavioral Acceptance

```text
Intent
Plan
Execution
Fact
Verification
Decision
Recovery
Memory
Resume
Projection
```

## Long-run Acceptance

必须至少出现完整链：

```text
Intent
→ Plan
→ Execute
→ Failure
→ Repair
→ Retest
→ Verification
→ Compact
→ Resume
→ Continue
→ Complete
```

## Independent Acceptance

Final Report 不得仅引用执行窗口自评。

必须附：

```text
关键真机日志
确定性产物
独立复验命令
至少一轮独立复核
```

---

# 2. Global Invariants

## INV-LR01：事实跨时间不丢

```text
Fact
Verification
TaskGoal
Task Continuity
```

不得因为：

```text
Compact
Resume
Long-run
```

失真。

---

## INV-LR02：长程成功不等于最后一步成功

必须区分：

```text
途中 Failure
与
最终 Failure
```

允许：

```text
失败
→
修复
→
恢复
→
完成
```

---

## INV-LR03：最终状态必须有证据

```text
completed
```

必须能够追溯到：

```text
Fact
+
Verification
+
必要时 Acceptance
```

不能因为：

```text
LLM says done
```

而完成。

---

## INV-LR04：任何中间失败不得无限循环

必须存在：

```text
bounded attempts
+
strategy change
+
replan
+
escalate
+
stop
```

不能：

```text
same state
+
same failure
+
same strategy
→
无限重复
```

---

## INV-LR05：环境不能伪造结果

任何真机实验前必须确认：

```text
正确 source
正确 binary
正确 VM
正确 PATH
正确 target
足够 disk
足够 memory
```

否则停止该实验并先恢复环境。

---

# 3. 全局禁止范围

本总包禁止引入：

```text
新的 TaskGraph
新的 TaskGoal
新的 Memory Model
新的 Completion Authority
新的 Terminal State
新的 Verification Authority
Subagents
TUI
MCP
Bridge production integration
Desktop
Persona
Semantic Memory
Automatic verification strictness
```

除非触发 STOP 条件。

任何“未来可能有用”的设计统一进入：

```text
Ideas Register
Quarantine
DEFER
```

---

# 4. STOP Conditions

## STOP-1

发现必须重建 TaskGraph / TaskGoal 才能实现 long-run continuity。

## STOP-2

发现 Fact 与 Verification 当前无法独立保存，必须创建第二事实源。

## STOP-3

发现安全边界必须放宽才能获得长程成功。

## STOP-4

发现需要让 LLM self-report 获得最终裁判权。

## STOP-5

发现必须取消既有 deadline / approval / sandbox / hard-redline 约束。

## STOP-6

核心长程失败无法判断是：

```text
mechanism
provider
environment
```

且证据不足以继续安全归因。

此时保存证据并停在分类边界。

## STOP-7

修改某核心机制后出现历史主线回归，且最小回滚无法恢复。

---

# 5. Node 00 — Environment / Provenance Lock

**零代码。**

先核实：

```text
git rev-parse HEAD
git status
Cargo version
tag
```

双 VM：

```text
.131
.133
```

确认：

```text
/usr/local/bin/hearth
source path
binary version
source marker
```

必须记录：

```text
df -h /home
df -h /
free -h
```

同时检查：

```text
~/codex
~/codex_t
target symlink
```

必须保证：

> 不存在共享 target 导致的隐式污染。

如果目标树有 symlink，记录并禁止把它当成独立 cache。

输出：

```text
docs/data/long-run-20260831/node00-provenance.md
```

**（v1.1 必办，承接砺批-7 + 守门员 S-1 实测发现）**：
① **.git 重建口径声明**：P2-MC 期间本地 .git 损坏重建（今日提交历史不可恢复），
provenance 记录必须加一行——"历史连续性以 CHANGELOG + docs 为准；git 历史
自 `463b315` 起可信"，并核实 tag v0.2.16 与 HEAD 祖先关系（本窗已核：tag 在库、
HEAD `40f607c` 链一致）。
② **（S-1）双 VM 二进制对齐**：本窗 2026-08-31 实测——`.133` 系统 PATH
`/usr/local/bin/hearth` = **0.2.15**，落后于源码树 v0.2.16（评审包 §16 如实标
binary "—"）。**Node 00 必须双 VM 重建并安装当前基线版本到系统 PATH 后才能
开跑任何真机实验**（N1-SBX 假 provenance 事故同型风险：源码/二进制脱节时，
评审树上的复验结果不可信）。gate 基线计数 **443**（本窗实测
`.133:~/t_gate_mc_final.log` = 443/0 四 RC=0）登记为 Final Report 增量记账基线。
③ 沿用既定约定：df 保险丝（<10G 中止）、Node 00 末做一次源码备份
（git archive / tar.gz 日期戳），防 .git 损坏类事故再次丢历史。

---

# 6. Node 01 — Core Decision / Terminal Mapping Audit

**零代码优先。**

审计：

```text
Continue
Replan
Complete
Escalate
Stop
```

与：

```text
Terminal
```

的真实映射。

形成正式表：

| Decision | Terminal | 是否等待用户 | delegation 是否影响 | 备注 |
| -------- | -------- | ------ | --------------- | -- |

重点检查：

```text
Escalate
+
delegation active
```

以及：

```text
Escalate
+
no human interaction channel
```

要求：

> delegation 可以解除 operational approval，但不能消除 semantic uncertainty。

确认：

```text
Stop ≠ GiveUp
Escalate ≠ Stop
GiveUp ≠ Completion
```

如果实现存在命名/映射混乱，只做最小修复。

**（v1.1 必答，承接砺批-3 / RC47）**：P2-MC Node 08 发现的 **RC47**（"继续"续轮
planner give_up——模型重写文件后放弃，决策层缺陷候选）正是本 Node 的活样本。
Node 01 审计表**必须回答**：give_up 拦截链（修 D + FA01 三路拦截）在
**"criteria 为空 + 已实质完成"**形态下是否覆盖——criteria 空时拦截被
`!edd_checks.is_empty()` 守门跳过（FA01 F9 同根的决策层变体），RC47 大概率
就是这个洞。若证实：最小修复仅限 give_up 消费端（如"written_files 非空 +
0 errors + criteria 空"形态打 `giveup_unverified` 既有标记或路由 readiness
复查），**禁动 planner schema**（STOP-1/2 防线）。

---

# 7. Node 02 — Tool Error Structure Closure

承接 FA01 OPEN。

审计：

```text
bash exit code
signal
timeout
TaskDeadlineExceeded
ToolResult.error_kind
```

目标：

```text
tool runtime
→
ToolResult
→
agent-core
→
FailureKind
```

必须避免：

```text
text parsing
```

替代结构化事实。

若 bash exit code 已存在但未完整投影：

> 进行最小接线。

验收：

```text
exit 0
exit >0
signal
tool timeout
task deadline
```

必须可区分。

**（v1.1 必遵，承接砺批-4）接线纪律三条**：① `TaskDeadlineExceeded` downcast
有现成先例（scheduler.rs 单/并行两路），照抄不改语义；② error_kind 已是
serde-default 向后兼容，**勿新增 ToolResult 字段**（STOP-5 防线）；③ 五态
（exit 0 / >0 / signal / tool timeout / task deadline）可区分的测试矩阵先红后绿；
当前"loop 层无 exit code 证据归 Unknown（不冒充）"是 FA01 钉死的正确基线，
接线后 Unknown 只留给真正无信号场景。

---

# 8. Node 03 — InteractionRequest Surface Audit

不重构 Interaction Framework。

审计两个问题：

```text
① 该不该问
② 问出来以后是否问对
```

样本至少包括：

```text
纯情绪
闲聊
自我指涉
模糊任务
明确任务
情绪 + 任务
问题 + 任务
长混合输入
```

检查：

```text
trigger correctness
option count
option completeness
sentence integrity
semantic preservation
```

尤其复查历史：

> 情绪吐槽被机械切成两个语法破碎选项。

若只是测试缺失：

> 补测试即可。

若仍存在真实 bug：

> 最小修复。

---

# 9. Node 04 — Progress Semantics Audit

本 Node 原则上：

> **先审计，不直接扩大 progress 模型。**

确认：

```text
write progress
verification progress
test progress
state progress
```

在现有实现中的真实表现。

复现至少三个场景：

### Case A

```text
write
→
10 次验证
```

### Case B

```text
test fail
→
repair
→
test pass
```

### Case C

```text
read-only exploration
→
new knowledge
```

回答：

> `steps_without_progress` 当前究竟表示什么？

如果现有定义足以支撑 long-run：

> 不改。

如果发现它已经真实构成 long-run 假停滞根因：

> 形成独立补丁，但必须先记录机制影响，不允许顺手重定义所有 progress。

**（v1.1 必遵，承接砺批-5）衔接 FA01 §5 顶层裁决**：progress 口径**维持不变**
已裁决（修 D + FA01 三路拦截在 give_up 消费端补偿；扩展 DEFER 须批准），二阶
效应清单已列（①拦截回喂步计入 progress → counter 清零 → GiveUp 臂命中点漂移；
②与 acceptance_replan_count 零和竞争加剧；③与 graph_stall_count 无交互已排除）。
Node 04 的 Case A/B/C 复现结论必须**逐条对照这三条**；若实测发现假停滞构成
真根因（Case C 纯读探索 → RC43 家族），独立补丁先红后绿 + 必须带 Case D
fixture 回归（防拦截链二阶效应复发）。

---

# 10. Node 05 — Compaction / History Boundary Audit

重点处理 P2-MEMORY OPEN。

审计：

```text
maybe_compact
archive
MAX_HISTORY_MSGS=40
history slicing
Task Continuity
Fact
Verification
```

核心问题：

```text
Compaction 有保护
```

但是：

```text
MAX_HISTORY_MSGS=40
```

是否绕过了相同保护。

构造：

```text
history > 40
+
fact
+
verification
```

验证：

```text
被截掉的内容
是否 archived
是否 recoverable
是否影响 continuation
```

分类：

```text
preserved
archived
recoverable
lost
```

不得把：

> “history state 里还存在”

误判成：

> “LLM 仍能看见”。

**（v1.1 必办，承接砺批-1）本 Node 升格为 Memory 域主战场**：P2-MC 实证链——
单 run 压缩曾死代码（已修复活）→ provider-aware 阈值 = 512K×0.6×2.55 ≈ 78 万
字符 → **常规长程任务（30-60 步）大概率永远达不到压缩点，单 run 的实际窗口
约束仍是 `MAX_HISTORY_MSGS=40` 静默切片**（loop.rs:2203-2205，无标记无归档——
压缩有 archive 先行 + INV-M01 fixture，切片两样皆无，**保护不对称**）。要求
Node 05 交付物含**"两条路径保护对称性"对照表**（触发条件/标记/归档/恢复能力
四列），并回答：被切掉的 fact/verification 在 state.history 仍活但 LLM 不可见，
B 档（archive grep）能否救回（预期：不能——archive 里没有切片内容）。若证实
切片路径零恢复能力：登记 OPEN + 最小改善建议（打标记或入 archive 二选一），
**不扩架构**。

---

# 11. Node 06 — Archive Recoverability

承接 P2-MEMORY OPEN-1。

验证：

```text
compact
→
archive
→
retrieval hint
→
model
```

必须分别证明：

### A

archive 有记录。

### B

system/context 知道 archive 存在。

### C

模型实际能够在需要时找到旧事实。

A+B 不能代替 C。

如果 C 无法稳定实现：

> 不立即造 Semantic Memory。

记录：

```text
archive preserved
but
model recoverability unproven
```

并设计最小改善建议。

**（v1.1 必办，承接砺批-2）C 级判定口径开工前预写**：P2-MC 教训——A（archive
有记录）+ B（系统知道 archive 存在）容易，**C（模型实际找得到）真机 0 使用**。
Node 06 开工前先冻结 C 的最小可判定形态，二选一：
1. **确定性判定**：压缩某具体事实（如 FACT-7 = "abc.txt 内容=X"）后直接询问，
   回复做确定性 substring 比对（不采信模型自述"我记得"）；
2. **工具化检索**：archive grep 作为模型主动发起的真实工具调用，统计触发率
   （比"提示注入后祈祷模型跟上"可测得多）。
结果只有三档：C 证明 / C 未证明但机制在（登记 OPEN，措辞用
"archive preserved but model recoverability unproven"）/ C 反证。
**禁止用 A+B 通过宣称 M5 全绿**——P2-MC 已把这条线画清，本总包不得退回。

---

# 12. Node 07 — Session Archive Isolation

承接：

> chat compression 首次触发时 session_id 绑定晚。

构造：

```text
Session A
→ compact

Session B
→ compact
```

检查 archive：

```text
A facts
B facts
```

必须严格隔离。

至少达到：

```text
A query
→
不会读到 B
```

以及：

```text
B query
→
不会读到 A
```

若当前共享：

> 进行最小 session-scoped 修复。

不得建立新的 Memory Store。

---

# 13. Node 08 — Compact + Resume Canonical Test

这是本总包的核心测试之一。

构造真实任务：

```text
Step 1
Step 2
Step 3
Failure
Step 4
Verification
Compact
Stop
Resume
Continue
Completion
```

至少包含：

```text
原始目标
completed work
remaining work
artifact
failure
verification
acceptance
```

必须证明：

```text
re-teach count = 0
```

并且：

```text
no duplicate destructive work
no lost artifact
no goal mutation
no false completion
```

**（v1.1 补 S-4，守门员）本 Node 是 P2-MC Deviation① 的正式闭环**：P2-MC 的
resume 前置跑未含压缩（"压缩+resume 分证合链"）是其三大偏差之首。Node 08
的 compact→stop→resume→continue **必须单跑成链**，Final Report 须显式声明
"P2-MC Deviation① 由本 Node 单跑闭环"，不得再以分证合链措辞替代。
**（S-2）criteria 冻结纪律**：Node 08/09/10/12/13 五个真机任务的 acceptance
criteria 全文必须在 draft 阶段冻结并归档 `docs/data/long-run-20260831/`，
故障注入点必须被 criteria 显式覆盖（"修复后行为=X"而非"文件存在"）——
FA01 上轮 Node 14 假阳性与 P2-MC B 档 0 使用的同源教训。

---

# 14. Node 09 — Long-run Product Task A

执行一个真实工程任务。

范围：

```text
30–60 steps
```

任务必须自然包含：

```text
inspect
plan
write
test
controlled failure
repair
retest
verification
compact
continue
completion
```

禁止任务为了测试而人为“演戏式”触发每一步。

要求真实工作。

建议：

```text
Rust / Python / small multi-file project
```

但执行窗口可根据实际仓库选择。

---

# 15. Node 10 — Long-run Product Task B

与 Task A 不同任务。

至少包含：

```text
multi-file change
test failure
repair
second verification
```

与 Task A 不同 failure topology。

目的：

> 避免“一次跑通”被误认为 long-run reliability。

---

# 16. Node 11 — Long-run QA / Discussion

至少：

```text
15+ turns
```

包含：

```text
事实问题
追问
反例
用户修正
“继续”
“查看状态”
“为什么”
“你刚才说的是什么”
```

必须观察：

```text
TaskGraph entry
GoalRevision
TaskControl
Conversation
compaction
planner
recovery
```

目标：

```text
QA
不会无故进入 TaskGraph

“继续”
不会污染 goal_revision

正常讨论
不会产生 recovery loop
```

**（v1.1 必录，承接砺批-3）**：Node 11 的每个"继续"轮必须逐轮记录三件事：
① 是否 give_up（RC47 形态——含"已完成+继续"后模型重做再放弃的变体）；
② 是否 GoalMutation；③ 是否重复执行已完成工作（duplicate execution）。
若 RC47 在此复现且 Node 01 已定性，修复按 Node 01 的最小修复边界执行。

---

# 17. Node 12 — Long-run Controlled Failure Task

单独构造：

```text
known bug
→
test failure
→
diagnosis
→
repair
→
retest
→
verification
→
complete
```

至少做两轮：

```text
First failure
Second failure
```

验证：

```text
FailureKind
RecoveryStrategy
strategy change
retest
verification
```

必须避免：

```text
same error
→
blind retry
```

---

# 18. Node 13 — Cross-compaction Continuity Stress

这是 Memory + Failure + Execution 的综合测试。

构造：

```text
Long task
↓
failure
↓
repair
↓
compact
↓
resume
↓
second failure
↓
replan
↓
verification
↓
compact again
↓
complete
```

要求至少：

```text
2 次 compaction
1 次 resume
1 次真实 failure
1 次真实 repair
```

记录：

```text
goal retention
fact retention
verification retention
artifact retention
failure retention
reteach count
duplicate execution
terminal correctness
```

这是整个 Core 最重要的“系统组合测试”之一。

**（v1.1 必遵，承接砺批-6）压缩归因口径**：P2-MC 实证——日志 grep INFO 被过滤
不可靠，**压缩发生的权威信号 = archive 行 created_at 时间戳（UTC 归一）窗口
归因**。Node 13 的 2 次压缩计数、compact 前后 retention 比对全部用该口径，
禁日志 grep 计数（教训已写进 P2-MC §6，勿再踩）。
**（守门员补 S-3）受控触发**：provider-aware 阈值 ≈78 万字符下，30-60 步自然
任务**永远不会触发压缩**——Node 13 的"至少 2 次 compaction"与 Node 08 的
compact 环节必须用 P2-MC 已落仪器 `HEARTH_COMPACT_CHAR_THRESHOLD` env
（context.rs:240，测试专用）制造确定性压缩点，并在数据记录中声明该 env 值；
禁为触发压缩而临时调低生产默认值。

---

# 19. Node 14 — Reliability Matrix

把本轮所有关键任务放进矩阵。

最低：

| 场景                    | 次数 |
| --------------------- | -: |
| Product long-run      |  2 |
| Controlled failure    |  2 |
| QA / Discussion       |  1 |
| Compact + resume      |  2 |
| Multi-compaction      |  1 |
| InteractionRequest    |  1 |
| Approval / delegation |  1 |
| Deadline              |  1 |

不把它们简单统计为：

```text 10/10 success
```

而要记录：

```text mechanism success
behavior success
evidence success
```

同时记录：

```text false stop
false giveup
false replan
false completion
duplicate execution
reteach
```

---

# 20. Node 15 — Strong Acceptance / Final Gate / Independent Review

这是最终节点。

## 20.1 Structural

执行：

```bash
bash ~/run_gate_r2c.sh
```

必须：

```text
FMT=0
CLIPPY=0
RT4_SOLO=0
TEST=0
```

路径必须完整登记。

---

## 20.2 Regression

必须覆盖：

```text
R2-C
W3/W4
RC24
W8
P1-LTR
P1-TASK-TRUTH
P1-EXECUTION-DECISION
P1-FAILURE-ADAPTATION
P2-MEMORY-CONTEXT
```

**（v1.1 补 S-5，守门员）回归的机械验收清单**：P2-MC 评审包 §17 给出六组可
直接执行的核验命令（turn 粒度 / honest counting / provider-aware 三 env /
compact_pressure_pct 改名 / INV-M01 双 fixture / 全量 gate），Node 15 回归时
**逐组复跑并把输出归档**——历史主线回归不得只靠"443 基线全绿"一句话，
每条主线至少对应一个可复跑命令或单测名。

---

## 20.3 Independent Review

执行窗口不得自行宣布：

> “Core Freeze Ready”。

必须准备：

```text
review package
```

由：

```text
人工独立评审窗口
+
外部 AI 评审窗口
```

进行最终独立审查。

Observer OS：

> 当前不承担 Core Freeze 的最终裁判职责。

---

# 21. Long-run Success Criteria

只有满足以下条件，才可认为：

# STRONG PASS

### L1 Intent

真实任务能够进入正确路径。

### L2 Execution

工具确实按预期完成。

### L3 Fact

系统准确知道发生了什么。

### L4 Verification

确定性证据支持结论。

### L5 Failure

失败可以识别并分类。

### L6 Recovery

失败后能够改变策略并恢复。

### L7 Memory

Compaction 后事实不丢。

### L8 Resume

Resume 不需要重新教育。

### L9 Completion

没有证据时不能假完成。

### L10 Projection

用户可以明确知道：

```text completed
failed
timeout
cancelled
give_up
```

到底是哪一个。

---

# 22. Hard Failure Criteria

以下任一出现：

```text
假完成
```

或：

```text
历史事实被静默丢失
```

或：

```text
resume 需要重新教育关键任务
```

或：

```text
同一失败无限循环
```

或：

```text
错误输出被投影为成功
```

或：

```text
sandbox / approval 被绕过
```

都不能进入 Strong PASS。

---

# 23. Metrics

至少记录：

```text
long_run_completion_rate
false_completion_rate
false_giveup_rate
false_replan_rate
false_stop_rate

failure_recovery_rate
recovery_attempts
strategy_change_rate

compact_count
compact_frequency

fact_retention_rate
verification_retention_rate
goal_retention_rate
artifact_retention_rate

reteach_count
duplicate_execution_count

goal_revision_count
execution_turn_count
goal_revision_rate

deterministic_verification_count
model_judgment_count
deterministic_verification_ratio

interaction_request_trigger_accuracy
interaction_surface_accuracy
```

注意：

> 第一阶段 telemetry 主要用于观察，不预设所有指标阈值。

---

# 24. Long-run Data Discipline

每一个真实长程任务必须保存：

```text
prompt
provider
model
budget
deadline
session_id
HEAD
binary version
VM
start time
end time
terminal
steps
tool calls
errors
failure classes
recovery strategy
compact count
resume count
artifacts
verification
```

不要只保留最终摘要。

---

# 25. Environment Discipline

每个真机实验前：

```text
1. binary --version
2. source Cargo.toml version
3. HEAD / provenance marker
4. disk free
5. memory
6. target path
7. provider
8. env flags
```

实验结束：

```text
disk free
```

如果环境异常：

> 实验结果标记 invalid，不允许作为产品行为证据。

---

# 26. 版本策略

如果本轮发生代码改动：

```text
v0.2.16
→
v0.2.17
```

CHANGELOG 必须在最终 gate 之前完成。

任何：

```text
docs only
```

与：

```text
code behavior
```

必须区分。

最终 gate 必须覆盖最终提交态。

---

# 27. 回滚策略

对于涉及行为的修复：

```text
旧路径
+
新路径
```

如确有必要保留一个 release cycle 的 fallback。

但禁止为了“可回滚”制造大量配置项。

只有真正需要时才引入：

```text
env switch
```

---

# 28. Final Report 结构

最终报告必须一次性交付：

## 1. Executive Summary

```text
STRONG PASS
PASS WITH DEVIATIONS
STOP
```

## 2. Baseline / Provenance

## 3. Core Decision Audit

## 4. Tool Error Closure

## 5. Memory / Compact Closure

## 6. Archive Isolation

## 7. Long-run Task A

## 8. Long-run Task B

## 9. QA / Discussion

## 10. Controlled Failure Recovery

## 11. Multi-compaction Stress

## 12. Reliability Matrix

## 13. Metrics

## 14. Regression

## 15. Deviations / OPEN / UNKNOWN / DEFER

## 16. Security

## 17. Independent Review Package

## 18. Final Acceptance Recommendation

---

# 29. Core Freeze Recommendation Logic

最终不得用：

```text passed tests = Core Freeze
```

而使用：

```text Structural
+
Behavioral
+
Long-run
+
Evidence
+
Independent Review
```

只有这五层同时成立：

```text
                         CORE FREEZE
                              ▲
                              │
              ┌───────────────┼───────────────┐
              │               │               │
         Structural       Behavioral      Long-run
              │               │               │
              └───────────────┼───────────────┘
                              │
                           Evidence
                              │
                      Independent Review
```

才允许进入：

# **HEARTH CORE FREEZE CANDIDATE**

---

# 30. 最终禁止事项

执行过程中即使出现新的想法：

```text
Subagent
TUI
MCP
Bridge
Persona
Semantic Memory
Adaptive Verification
新的 Planner
新的 Memory Store
```

也不得直接插队。

记录：

```text
DEFER
```

或：

```text
QUARANTINE
```

继续本总包。

---

# 31. 最终使命

本总包不是为了证明：

> Hearth 没有 bug。

这是不可能也没有意义的。

本总包要证明的是：

# **Hearth Core 的主要责任边界已经能够在长时间运行中彼此协作，而不会因为失败、压缩、恢复和上下文变化而互相破坏。**

最终需要能够完整回答：

```text
我在做什么？
↓
为什么这样做？
↓
实际发生了什么？
↓
哪些只是模型判断？
↓
什么已经被证据证明？
↓
失败是什么？
↓
为什么采用这个恢复策略？
↓
压缩之后还知道自己做到哪里？
↓
恢复之后有没有重新执行已经完成的工作？
↓
为什么现在可以说完成？
↓
用户看到的结果是否与事实一致？
```

---

# 32. 执行纪律

批准后：

```text
Node 00
↓
Node 01
↓
Node 02
↓
...
↓
Node 15
↓
Final Report
```

不在 Node 之间请求用户确认。

每一个 Node 内部：

```text
Audit
→
Design
→
Implement（必要时）
→
Unit Test
→
VM Test
→
Record Evidence
→
Next Node
```

普通失败：

```text
diagnose
→
fix
→
test
→
retry
```

只有：

```text
STOP-1 ~ STOP-7
```

允许暂停。

---

# 33. 本总包结束条件

当最终可以用一句工程事实证明：

> **Hearth 在长程真实任务中，能够保持 Intent、Fact、Verification、Decision、Recovery、Memory、Resume 和 Completion 的一致性，并且这些结果有独立证据支持。**

则本总包结束。

下一阶段不再继续增加 Core 能力。

进入：

# **HEARTH CORE FREEZE REVIEW**

---

# 附：砺·评审批注（2026-08-31，总判断：🟡 有条件通过——按批-1/批-2/批-3 纳入后放行）

> P2-MC 事后审计基线：评审包（v0.2.16 `253fcd1`）§17 锚点本窗全实证——turn 粒度修复（loop.rs:2350-2357 一次交换=一个 Turn + e2e 9261）、honest counting（context.rs:182 naive / 202 legacy / 212 effective_estimate）、provider-aware 三 env（loop.rs:4352-4370：`HEARTH_COMPACTION_MODE=legacy` 回滚 + `HEARTH_CONTEXT_TOKENS` > caps × ratio 0.6 × CHARS_PER_TOKEN 2.55）、RC40 改名 compact_pressure_pct 且旧名全库清零（status.rs:91/102 同步）、INV-M01 可失败双测试（context.rs:910 survives + 1062 loss-mode——"断言不能失败=断言不存在"的反面教材已补）、web 8000 截断标记（web.rs:127-130）、T4 跨轮污染修复（last_graph_sig 重置 2602/3805）、MAX_HISTORY_MSGS=40 静默切片实锤（loop.rs:2203-2205）。gate 443/0 与真机日志在 VM 侧未复跑（证据边界如实声明）。

## 批-0（P2-MC 终态裁定建议，请顶层确认）

**维持 PASS WITH DEVIATIONS 并 CLOSED。** 三个偏差（Node 09 resume 未含压缩单跑分证合链 / B 档 0 使用 / RC47 移交）均如实披露且不在报告里粉饰；A/B 模型重复执行污染（seq 38-55 重跑）主动披露并单列 clean 样本——这是"反对自嗨主义"的正确执行。我上轮 5 条批注全部采纳落地：批-1 先验被"复核+深挖"超越（挖出死代码）、批-2 provider-aware 最小切片实施、批-3 仪器先校准（est 虚增 1.39-2.49× 实测）、批-4 层归纪律执行（只报 mean/min/max 不宣称显著）、批-5 4096 SAFE 猜想证实。**★ 事故披露认可**：本地 .git 损坏重建（§18-2）如实报告——provenance 以 tag v0.2.16→`463b315` + CHANGELOG 为准的口径正确，本窗已核实 tag 在库、HEAD 链一致。

## 批-1（🔴 Node 05 优先级重估：MAX_HISTORY_MSGS=40 是当前单 run 的**主窗口约束**，不是边缘审计）

P2-MC 实证链要读全：单 run 压缩曾死代码 → 修复复活 → 但 provider-aware 阈值 = 512K×0.6×2.55 ≈ **78 万字符**——常规长程任务（30-60 步）大概率仍达不到压缩点。推论：**修复后单 run 的实际窗口约束仍然是 40 条消息切片**，压缩只在超长会话/REPL 生效。因此 Node 05 不是"顺带审计"，而是本总包 Memory 域的**主战场**：40 切片与压缩路径的保护必须**对照审计**（压缩有 archive 先行+INV-M01 fixture；切片无标记无归档——保护不对称）。要求 Node 05 交付物包含一张"两条路径保护对称性"对照表，并回答：被切掉的 fact/verification 若在 state.history 还活着但 LLM 不可见，B 档能否救回（预期答案：不能——archive 里没有切片内容）。若证实切片路径零恢复能力，登记 OPEN + 最小改善建议（打标记或入 archive 二选一），不扩架构。

## 批-2（🔴 Node 06 C 级证明的判定口径预写——防"提示在场"被误判为"可恢复"）

B 档真机 0 使用的先例说明：**A（archive 有记录）+ B（系统知道 archive 存在）容易，C（模型实际找得到）没证明**。Node 06 开工前预写 C 的最小可判定形态，建议二选一：
1. **确定性判定**：压缩某具体事实（如 FACT-7 = "abc.txt 内容=X"）后，直接询问该事实，回复内容做确定性 substring 比对（不采信模型自述"我记得"）；
2. **工具化检索**：archive grep 作为真实工具调用（模型主动发起），统计触发率——比"提示注入后祈祷模型跟上"可测得多。
判定结果只有三档：C 证明 / C 未证明但机制在（登记 OPEN，措辞用 "archive preserved but model recoverability unproven"）/ C 反证。**禁止用 A+B 通过宣称 M5 全绿**——P2-MC 已把这条线画清了，本总包不得退回。

## 批-3（🔴 RC47 关联样本：Node 01 与 Node 11 必须包含"已完成+继续"形态）

P2-MC Node 08 发现的 **RC47**（"继续"续轮 planner give_up——模型重写文件后放弃，决策层）正是本总包 Node 01（Decision/Terminal 映射）与 Node 11（QA 15+ 轮含"继续"）的活样本。具体要求：
1. Node 01 审计表必须回答：give_up 拦截链（修 D+FA01 三路）在"criteria 为空 + 已实质完成"形态下是否覆盖——**criteria 空时拦截被 `!edd_checks.is_empty()` 守门跳过**（FA01 F9 同根的决策层变体），RC47 大概率就是这个洞；
2. Node 11 的"继续"轮必须记录：是否 give_up / 是否 GoalMutation / 是否重复执行已完成工作（duplicate execution）；
3. 修复只做最小（如 give_up 消费端对"written_files 非空 + 0 errors + criteria 空"形态打 `giveup_unverified` 已有标记或路由 readiness 复查），禁动 planner schema。

## 批-4（🟡 Node 02 bash exit code 接线纪律）

承接 FA01 OPEN-1（RC46）。三点：① TaskDeadlineExceeded downcast 有现成先例（scheduler.rs:29-90 单/并行两路），照抄不改语义；② error_kind 已是 serde-default 向后兼容，**勿新增 ToolResult 字段**（STOP-5 防线）；③ 五态（exit 0 / >0 / signal / timeout / task deadline）可区分的测试矩阵先红后绿——当前 loop 层无 exit code 证据归 Unknown（不冒充）的行为是 FA01 钉死的正确基线，接线后 Unknown 只留给真正无信号场景。

## 批-5（🟡 Node 04 progress 审计必须衔接 FA01 §5 裁决）

FA01 已顶层裁决：progress 口径**维持不变**（修 D+FA01 三路拦截在 give_up 消费端补偿；扩展 DEFER 须批准），且二阶效应清单已列（①拦截回喂步计入 progress → counter 清零 → GiveUp 臂命中点漂移；②与 acceptance_replan_count 零和竞争加剧；③与 graph_stall_count 无交互已排除）。Node 04 的 Case A/B/C 复现结论必须**逐条对照这三条**；若 long-run 实测发现假停滞构成真根因（Case C 纯读探索 → RC43 家族），独立补丁先红后绿 + 必须带 Case D fixture 回归（防拦截链二阶效应复发）——不允许"顺手重定义所有 progress"（总包已有此句，本批注给它锚点）。

## 批-6（🔵 Node 13 双压缩 stress：归因口径直接沿用 P2-MC 教训）

P2-MC 实证：日志 grep INFO 被过滤不可靠，**压缩发生的权威信号 = archive 行 created_at 时间戳（UTC 归一）窗口归因**。Node 13 的 2 次压缩计数、compact 前后 retention 比对全部用该口径，禁日志 grep 计数（教训已写进 P2-MC §6，勿再踩）。

## 批-7（🔵 Node 00 provenance 补一条：.git 重建口径）

P2-MC §18-2 事故：本地 .git 损坏重建，今日提交历史不可恢复。Node 00 provenance 记录须加一行声明："**历史连续性以 CHANGELOG + docs 为准；git 历史自 `463b315` 起可信**"，并核实 tag v0.2.16 与 HEAD 祖先关系（本窗已核：tag 在库、`40f607c` HEAD 链一致）。target symlink 检查总包已含 ✓（codex_t/target → codex/target 共享 76G 已清理），df 保险丝（<10G 中止）既有约定延续。

## 批-8（✅ 合格确认）

- 上游清单完整：P2-MC 4 项 OPEN + FA01 5 项 OPEN 全部点名承接（Node 02/05/06/07 精确对应），无遗漏无越界；
- INV-LR01~05 与 STOP-1~7 覆盖合格；INV-LR03（completed 必须可追溯 Fact+Verification）与不得自审公理同构；
- §29 五层 Core Freeze 结构（Structural+Behavioral+Long-run+Evidence+Independent Review）正确——**执行窗口不得自行宣布 Freeze Ready，评审主体=人工+外部 AI 双窗**，与砺窗定位一致；
- §22 Hard Failure Criteria（假完成/静默丢失/重教育/无限循环/错误投影成功/绕过沙箱）与砺窗验收判据 A 系列同向；
- Node 12（双失败轮 + strategy change）直接复用 FA01 机制，Node 14 Reliability Matrix 的三层成功口径（mechanism/behavior/evidence）正确防"10/10 success"式自嗨。

**放行条件**：批-1（40 切片升主战场+保护对称性对照表）、批-2（C 级判定口径预写）、批-3（RC47 关联样本）写入执行窗口开工前提；批-4~7 为 Node 内纪律。守门人零代码改动。

---

# 附 2：守门员复核定稿批注（2026-08-31，总包 v1.1 定稿依据）

> 定位：砺批-0~8 已审（附 1），本节是**对审计的审计**（锚点实测）+ 顶层定稿裁决。
> 实测基线：本机 HEAD `40f607c`（tag v0.2.16 → `463b315` 基线之后含 docs 提交，链一致已核）。

## 复-1 P2-MC 上游核验（评审包 + 砺批注锚点全实证）

| 项 | 声明方 | 实测 |
|---|---|---|
| turn 粒度修复（单 run 压缩复活） | 评审包 §4 + 砺附 | ✅ loop.rs:2350 + e2e :9261 |
| honest counting（naive/legacy/effective 三层） | 同 | ✅ context.rs:181/202/212，CHARS_PER_TOKEN 2.55 :258 |
| provider-aware 三 env（CONTEXT_TOKENS/WINDOW_RATIO/COMPACTION_MODE） | 同 | ✅ loop.rs:4352-4370 + context.rs:222-240 |
| RC40 改名 compact_pressure_pct、旧名全库清零 | 同 | ✅ loop.rs:1349 + status.rs:91/102；grep 旧名零命中 |
| INV-M01 可失败双 fixture | 同 | ✅ context.rs:910（survives）/ :1062（loss-mode） |
| web 8000 截断标记 | 同 | ✅ web.rs 125-132 |
| T4 跨轮污染修复（last_graph_sig 按 run 重置） | 同 | ✅ loop.rs:4409 |
| MAX_HISTORY_MSGS=40 静默切片（无标记无归档） | 同 | ✅ loop.rs:2203-2205，FACT_RISK 定性成立 |
| gate `.133:~/t_gate_mc_final.log` = 443/0 四 RC=0 | 评审包 §12 | ✅ paramiko 实测 |
| .131 binary = 0.2.16 | 评审包 §16 | ✅ 实测 |

结论：**P2-MC = PASS WITH DEVIATIONS / CLOSED 成立**（三大偏差如实披露、四问答案证据分级正确、M1-M8 对照完整），批-0 裁定采纳写入 §0。

## 复-2 砺批-0~8 采纳状态

- 批-0（P2-MC 终态裁定）：**采纳**，已写入 §0 上游清单（PASS WITH DEVIATIONS / CLOSED）。
- 批-1（🔴 40 切片升主战场）：**采纳**，内联 Node 05——保护对称性对照表 + B 档救不回切片内容的预期答案。
- 批-2（🔴 C 级判定口径预写）：**采纳**，内联 Node 06——开工前冻结判定形态，三档结果，禁 A+B 冒充 M5。
- 批-3（🔴 RC47 关联样本）：**采纳**，内联 Node 01 + Node 11——criteria 空形态审计 + 三件事逐轮记录 + 最小修复边界。
- 批-4/5/6/7（🟡 Node 内纪律）：**采纳**，分别内联 Node 02（接线三条）/ Node 04（衔接 FA01 §5 裁决）/ Node 13（archive 归因口径）/ Node 00（.git 重建声明）。
- 批-8（✅ 合格确认）：上游 OPEN 承接映射、五层 Freeze 结构、三层成功口径——确认无异议。

## 复-3 守门员补充（S-1~S-5）

- **S-1（🔴 Node 00 双 VM 二进制对齐）**：本窗实测 `.133` 系统 PATH = **0.2.15**（源码树 0.2.16，评审包 binary 栏"—"如实但构成脱节）——Node 00 必须双 VM 重建安装对齐后才可开跑真机（N1-SBX 假 provenance 同型风险）；gate 基线 **443** 登记为增量记账基线。已内联 Node 00。
- **S-2（🟡 criteria 冻结纪律）**：Node 08/09/10/12/13 五个真机任务 criteria 全文 draft 阶段冻结归档 + 故障注入点显式覆盖（FA01 假阳性 / P2-MC B 档 0 使用同源教训）。已内联 Node 08（覆盖五个 Node）。
- **S-3（🔴 受控压缩触发）**：provider-aware 阈值 ≈78 万字符 → 自然任务永不触发压缩；Node 08/13 的 compact 环节必须用 `HEARTH_COMPACT_CHAR_THRESHOLD`（P2-MC 已落仪器）制造确定性压缩点并声明 env 值。已内联 Node 13。
- **S-4（🟡 P2-MC Deviation① 闭环声明）**：Node 08 的 compact→stop→resume→continue 必须单跑成链，Final Report 显式声明闭环 P2-MC 偏差之首。已内联 Node 08。
- **S-5（🟡 回归机械清单）**：Node 15 复用 P2-MC 评审包 §17 六组命令逐组复跑归档，历史主线回归不得只靠一句"443 全绿"。已内联 §20.2。

## 复-4 定稿裁决

**本总包 v1 → v1.1 定稿，放行执行。** 砺批-0~8 全部采纳并内联（Node 00/02/04/05/06/01+11/13 + §0）；守门员补 S-1~S-5（Node 00 / Node 08 / Node 13 / §20.2）。五层 Freeze 结构（§29）与 §20.3 独立评审主体（人工+外部 AI 双窗、执行窗口不得自宣 Freeze Ready）维持原样——与本窗定位一致。不新增 STOP；INV-LR 系列编号独立（Final Report 继续用 `INV-ED01-`/`INV-FA01-`/`INV-M-`/`INV-LR-` 前缀区分四套编号）。守门人零代码改动。
