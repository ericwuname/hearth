# Hearth CORE FREEZE REVIEW-01

## Core Freeze 前独立横向审查与最终阻塞项收敛任务书 v1.1

**日期**：2026-08-31（v1.1：砺批-0~8 内联 + 守门员复核定稿）

**日期**：2026-08-31
**性质**：Core Freeze 前独立审查总包
**模式**：只审查、只验证、只修 Freeze-blocking 缺陷；不再扩展 Core 能力
**执行方式**：Node 00 → Node 12 连续自主执行；普通证据缺失自行补测；发现局部 bug 可最小修复；涉及架构扩张或权限边界变化必须 STOP
**最终交付**：一次性提交 `CORE FREEZE REVIEW-01 Final Report`
**核心原则**：

> 本轮不是证明“Hearth 没有 bug”，而是判断“Hearth Core 是否已经达到可以冻结的工程成熟度”。

---

# 0. 当前基线

以实际源码、实际 binary、实际 gate 为唯一依据。

已完成主线：

```text
R2-D TaskGoal
R2-C ContextBuilder
W3/W4 Fact / Completion / Projection
RC24 Approval / Delegation
W8 Intent / Goal Routing
P1-LTR Tool Deadline
P1-TASK-TRUTH Verification
P1-EXECUTION-DECISION
P1-FAILURE-ADAPTATION
P2-MEMORY-CONTEXT
P2-LONG-RUN-ACCEPTANCE
```

最近整合版本：

```text
v0.2.17
tag v0.2.17
HEAD 3b304dd
```

**注意：不要盲信上述版本号。Node 00 必须重新实测。**

**（v1.1 上游裁定，承接砺批-0）**：FA01 / P2-MC / P2-LR 三包全部 **PASS WITH
DEVIATIONS / CLOSED**（v0.2.15/16/17，gate 437→443→447 演化可追溯）。残余 OPEN
按 F0/F1/F2 预分级：**F0 = 0**（零假完成/零静默丢失/零沙箱绕过/零无限循环）；
F1 三项 = false stop ×2（RC48，Node 03 最高优先级）/ archive C 未证明 / 切片
零恢复通道；其余 F2 DEFER。本总包的使命 = 把 F1 三项收敛为"可解释、有界、
已声明"，并产出 Freeze 独立审查材料。

---

# 1. 本轮使命

回答一个问题：

# **Hearth Core 现在是否已经具备进入 Core Freeze Review 的资格？**

最终只允许三种结论：

```text
CORE FREEZE READY
CORE FREEZE READY WITH ACCEPTED DEVIATIONS
NOT READY
```

不得使用：

```text
“看起来没问题”
“基本稳定”
“应该可以”
```

必须以证据支持。

---

# 2. Core Freeze 的五层验收模型

本轮统一采用：

```text
Structural
+
Behavioral
+
Long-run
+
Evidence
+
Independent Review
```

五层同时考虑。

---

# 3. Freeze 阻塞级别定义

统一使用：

## F0 — Freeze Blocker

出现以下任一情况：

```text
false completion
silent fact loss
resume requires reteach of critical task
unbounded recovery loop
sandbox / approval bypass
Fact → Projection 失真
Fact → Decision 明显错误且可稳定复现
wrong binary/source provenance
```

→ **NOT READY**

---

## F1 — High Risk

严重影响可信委托，但已有边界保护：

```text
false stop
archive recoverability 未闭合
history slice 无 recovery
Decision/Terminal semantic mismatch
长程 convergence variance 过高
```

必须进入 Final Report。

是否阻塞由独立评审裁决。

---

## F2 — Non-blocking

```text
UX
minor telemetry
低概率边缘
显示优化
未来能力
```

允许 DEFER。

---

# 4. Global Red Lines

本轮禁止：

```text
新 TaskGraph
新 TaskGoal
新 Memory Store
新 Completion Authority
新 Terminal State
新 Verification Authority
新 Planner
新 Semantic Memory
Subagent
TUI
MCP
Bridge production integration
Persona
自动验证严格度体系
```

禁止为了让测试变绿而：

```text
降低安全限制
关闭 sandbox
关闭 approval
扩大 budget
扩大 deadline
绕过 verification
允许 LLM self-report 成为证据
```

---

# 5. STOP Conditions

## STOP-1

发现 Core 事实模型本身存在冲突，无法局部修复。

## STOP-2

发现必须重建 TaskGraph / TaskGoal。

## STOP-3

发现必须放宽 sandbox / approval / deadline 才能获得成功。

## STOP-4

发现只有让 LLM self-report 获得更高权限才能修复。

## STOP-5

发现问题需要新建一整套 Memory / Verification / Decision 架构。

## STOP-6

发现源码、binary、gate provenance 无法建立一致证据链。

---

# 6. Node 00 — Provenance Freeze Audit

**零代码。**

分别在 `.131` 和 `.133` 做三查：

```text
binary --version
Cargo.toml version
source / HEAD marker
```

确认：

```text
source version
binary version
gate 编译版本
```

必须一致。

特别核查前一份整合报告中的异常：

```text
.133 source = 0.2.17
.133 binary = 0.2.16
gate = 0.2.17
```

这个异常必须：

```text
修正
```

或明确证明是报告填写错误而非实际行为错位。

同时检查：

```bash
df -h /
df -h /home
free -h
```

检查：

```text
~/codex
~/codex_t
target symlink
```

不得出现共享 target 导致 provenance 污染。

输出：

```text
docs/data/core-freeze-review-20260831/node00-provenance.md
```

**（v1.1 必办，承接砺批-2 + 守门员 S-1 实测确认）provenance 异常 = 真滞后，实修**：
本窗 2026-08-31 实测——`.133` source 树 Cargo.toml = **0.2.17**、系统 PATH
`/usr/local/bin/hearth` = **0.2.16**、gate `t_gate_lr_final.log` 编译 0.2.17（447/0）；
`.131` = 0.2.17 正常。**异常定性 = (b) 系统 PATH 二进制滞后**（v0.2.3 PATH 分叉
家族复发形态），不是报告笔误。处置：Node 00 在 `.133` 重建并安装当前基线版本到
系统 PATH（三查一致后跑一次最小冒烟如 `cargo test -p agent-core --lib test_rc47`
证明证据链活着）；**禁止以"gate 编译版本对"为由豁免系统 PATH 对齐**。
判断先给出：**LR 真机证据链本身不受影响**（真机在 .131，binary=0.2.17）——
Node 00 不得把精力错花在重跑真机上。gate 基线 **447** 登记为 Final Report
增量记账基线。

---

# 7. Node 01 — Architecture Truth Map

建立完整 Core 图：

```text
INTENT
 ↓
PLAN
 ↓
EXECUTION
 ↓
FACT
 ↓
VERIFICATION
 ↓
REFLECTION
 ↓
DECISION
 ↓
FAILURE ADAPTATION
 ↓
MEMORY / COMPACTION
 ↓
RESUME
 ↓
TERMINAL
 ↓
PROJECTION
```

每一层明确：

```text
输入
输出
事实源
决策权
是否允许 LLM 修改
是否允许失败旁路
```

特别建立：

# Authority Matrix

至少覆盖：

```text
tool success
artifact exists
TaskGoal
TaskGraph
verification
acceptance
reflection
failure classification
replan
giveup
complete
stop
escalate
terminal
projection
```

任何一项出现：

```text
两个 authority
无 authority
authority 越级
```

立即登记。

**（v1.1 必遵，承接砺批-3）先填实测基线，再查冲突**：矩阵按源码现状填，不按
Roadmap 理想态填空。可直接使用的已实证基线锚点——complete 判定 = Done 相位
`verify_acceptance_criteria`（O-4 唯一生产者）；give_up 消费端 = 三路拦截 +
Reserve 先决（修 D+FA01）+ RC47 路由（loop.rs:4223）；failure classification =
`classify_failure` 纯函数（terminal.rs:277）；compaction 阈值 = provider-aware
（loop.rs:4352-4370）；terminal = `normalize_terminal_state` 纯函数；planner
GiveUp 臂 = 规则（planner/lib.rs:483/489/495-502，无 acceptance 感知 = 已知缺口）。
**已知双轨预警**：give_up 的 authority 事实上分裂在 planner（决策）与 loop
（消费拦截）两侧——这是修 D 预裁决的有意设计（planner schema 不动），矩阵里记为
**"决策权/否决权分离"**而非"双 authority 冲突"。

---

# 8. Node 02 — Decision → Terminal 完整映射审计

必须建立最终正式表：

| Decision | Terminal  | 是否等待用户 | 是否能自动继续 | delegation 对其是否有影响 |
| -------- | --------- | ------ | ------- | ------------------ |
| Continue | 实际映射      |        |         |                    |
| Replan   | 实际映射      |        |         |                    |
| Complete | completed |        |         |                    |
| Escalate | 实际映射      |        |         |                    |
| Stop     | 实际映射      |        |         |                    |

特别验证：

```text
Stop ≠ GiveUp
Stop ≠ Failed
Escalate ≠ Approval
Escalate ≠ Stop
GiveUp ≠ Completion
```

再专门验证：

```text
Escalate
+
delegation active
+
no human channel
```

系统是否：

```text
等待
自动降级
Stop
继续
```

必须形成明确行为，不允许隐式。

---

# 9. Node 03 — False Stop 深挖

这是本轮最高优先级。

已有证据：

```text
Controlled Failure
mechanism = 2/2
behavior = 0/2
independent verification = 2/2
```

必须重新构造至少：

```text
2 个 false-stop 样本
```

检查完整链：

```text
Fact
→ Verification
→ Reflection
→ Decision
→ Terminal
```

回答：

> 为什么系统已经拥有足够成功证据，却仍然 Stop？

至少检查：

```text
budget-low
steps_without_progress
planner GiveUp
verification result
acceptance passed
failure class
last action
Decision input
```

必须确定：

```text
mechanism cause
decision cause
model cause
provider cause
environment cause
```

不能直接写：

> “模型判断错误”。

**（v1.1 必遵，承接砺批-1）三个机制假设预写，逐个证伪**：sortlib false stop ×2
（mechanism 2/2 + 独立复验 2/2 passed，但 behavior 0/2——修复完成后规划停滞终止）。
假设排序：
1. **假设 A（最可能）**：sortlib 的 criteria **非空** → RC47 修复的"criteria 空
   路由 Done"分支（loop.rs:4223）**不触发** → planner GiveUp 臂命中且 Reserve
   已尽 → 落原 give_up/stop。即 **RC47 修复只堵了 criteria 空的盲区，criteria
   非空 + Reserve 零和的形态仍裸奔**——FA01 遗留 RC45（双上限零和）与"planner
   GiveUp 臂无 acceptance 感知"（修 A 备而未用条款）的叠加态。检验法：查 sortlib
   跑的 criteria 是否非空 + Reserve 消耗日志（`VERIFICATION_RESERVE` /
   `acceptance_replan_count`）；
2. **假设 B**：steps_without_progress 语义——修复后的复测/验证步不计 progress
   （RC43 口径），planner 规则臂照常命中；
3. **假设 C**：决策输入缺 acceptance 事实（`grep acceptance planner/` = 0 老根因
   ——planner 在决策点看不到验收通过这一事实）。
**纪律**：复现跑必须用与 P2-LR 相同任务 + 相同 budget/deadline 参数（可比性），
并保留 P2-LR 原日志为对照基线；许可的修法（按 Node 04 优先序）= Reserve 分账
（RC45 预案）或 give_up 消费端对"acceptance passed 已在 scratch"的消费扩展——
**均须顶层批准**；若结论是 model variance → 只归档不修。Node 03 报告必须明确
写出与 RC47 已修形态的分界：**criteria 空与非空**。

---

# 10. Node 04 — False Stop 修复边界决策

如果 Node 03 确认存在稳定机制问题：

只允许最小修复。

优先顺序：

```text
错误 authority precedence
>
错误 evidence consumption
>
错误 state mapping
>
模型行为
```

禁止直接：

```text
“只要看到 artifact 就 Complete”
```

禁止：

```text
“只要 test pass 就 Complete”
```

必须保持：

```text
Fact
+
Verification
+
Acceptance
```

的权力边界。

若问题纯属 provider / model variance：

> 不修机制，只归档证据。

---

# 11. Node 05 — MAX_HISTORY_MSGS=40 审计

这是第二优先级。

构造：

```text
history > 40
```

并在早期插入：

```text
用户约束
关键事实
关键决策
验证结果
```

超过 40 后检查：

```text
context visible?
history state?
archive?
recovery?
retriever?
```

必须分别回答：

```text
1. 是否从 context 消失？
2. 是否持久化？
3. 是否可自动恢复？
4. 是否需要模型主动知道去 grep？
```

不要用：

```text
“state.history 里还存在”
```

代替：

```text
“LLM 仍可见”
```

**（v1.1 必遵，承接砺批-4）判级预期与诚实性**：P2-LR 已补 `[history note]`
标记（loop.rs:2217-2220），但标记文本对模型说的是"原始记录仍完整保留于会话
状态"——**诚实但无恢复能力**（切片内容不在 archive，B 档 grep 救不回；模型
既无工具也无检索通道）。Node 05 判级预期：早期事实 = context 消失 + state
存活 + 无 archive + **零恢复通道（lost-to-LLM）**。若判 F1（砺窗预期是），
最小改善二选一登记：切片内容入 archive（复用既有通道，零新架构）或标记文本
改为"该内容已不可检索"（诚实性修正，零代码）。**冻结窗口内不建议切片入
archive**（DEFER 合理）——但必须写清这是"已知有界损失"而非"已保护"。

---

# 12. Node 06 — Archive Recoverability Audit

独立验证：

```text
Compact
→ Archive
→ Recovery
```

分三层：

### A

archive 确实保存。

### B

context 能发现 archive。

### C

模型真的能够恢复所需旧事实。

必须明确：

```text
A ≠ B ≠ C
```

如果 C 未证明：

不要设计大型 Semantic Memory。

只形成：

```text
OPEN: Archive Recoverability
```

并给出最小改进方向。

**（v1.1 必遵，承接砺批-5）C-probe 防污染设计**：P2-LR 的 C-probe 结论
"C 未证明（路径=保留上下文非 grep）"暴露探针缺陷——**答案可由未压缩的保留
上下文推出（contaminated probe）**，模型走捷径，C 层根本没被测到。Node 06
必须重新设计：构造**事实只存在于被压缩轮次**的问题（该事实在保留的近 N 轮
与 state 层均无痕迹，唯一载体是 archive），压缩后提问，回复做确定性 substring
比对（不采信自述）。判定三档：C 证明 / C 未证明机制在 / C 反证。若仍 C 未证明
→ OPEN 保持诚实登记，**不阻塞 Freeze**（F1 级，边界已声明）——但 review-index
里必须列为"**委托方需知的已知限制**"，不是脚注。

---

# 13. Node 07 — Fact / Verification Survival

专门检查：

```text
Fact
Verification
Acceptance
```

跨越：

```text
Compact
History Slice
Resume
```

前后是否一致。

至少测试：

```text
original_goal
constraints
acceptance_criteria
tool success
artifact
failure
verification
acceptance_result
last_failure_class
```

结果分成：

```text
preserved
archived
recoverable
lost
unknown
```

禁止把：

> archived

写成：

> recoverable。

---

# 14. Node 08 — Canonical Compact → Resume → Continue

建立 Core Freeze 的标准长程样本。

必须自然包含：

```text
plan
write
test
failure
repair
verification
compact
stop
resume
continue
completion
```

至少验证：

```text
re-teach = 0
duplicate destructive execution = 0
goal mutation by “继续” = 0
false completion = 0
false give_up = 0
```

并记录：

```text original_goal
completed
remaining
next_action
artifact
verification
terminal
```

**（v1.1 补 S-2/S-3，守门员）两条既有仪器纪律延续**：
① **criteria 冻结**：Node 08/09/10 三个真机 benchmark 的 acceptance criteria
全文 draft 阶段冻结归档 `docs/data/core-freeze-review-20260831/`，故障注入点
显式覆盖（FA01 假阳性 / P2-MC B 档 0 使用同源教训——三连教训后此条不再是
提醒而是硬门槛）；
② **受控压缩触发**：provider-aware 阈值 ≈78 万字符下自然任务永不触发压缩
——Node 08 的 compact 环节用 `HEARTH_COMPACT_CHAR_THRESHOLD`（P2-MC 已落
仪器，测试专用）制造确定性压缩点并在数据记录中声明 env 值，禁临时调低生产
默认值。

---

# 15. Node 09 — Long-run Product Benchmark

执行至少两个不同任务。

每个：

```text
30–60 steps
```

尽可能自然包含：

```text
multi-file
failure
repair
test
verification
compact
resume
completion
```

不是为了制造复杂流程而人工演戏。

每个任务至少保存：

```text full event log
telemetry
report
artifacts
verification evidence
terminal
```

---

# 16. Node 10 — Controlled Failure Benchmark

至少两个不同失败拓扑。

例如：

```text
Case A
test failure
→ repair
→ retest
→ pass

Case B
environment/tool failure
→ adaptation
→ recovery
```

检查：

```text FailureKind
RecoveryStrategy
strategy change
boundedness
verification
completion
```

必须确认：

```text same failure
+
same state
+
same strategy
```

不会无限循环。

---

# 17. Node 11 — QA / Conversation / Intent Boundary

至少：

```text 15+ turns
```

包含：

```text
闲聊
情绪
自我指涉
事实问题
追问
用户纠正
继续
查看状态
为什么
真正任务
```

重点检查：

```text TaskGraph entry
GoalRevision
TaskControl
Conversation
InteractionRequest
Recovery
```

重点回归：

```text “继续”
“查看状态”
```

不应：

```text revision++
GoalMutation
无意义 decompose
recovery loop
```

同时复查：

> 情绪 + 任务

这种复合输入。

**（v1.1 必遵，承接砺批-6）回归确认非重新发现**：修 B（疑问句类别）+ RC47
修复（正反测试 loop.rs:9402）+ P2-LR 真机（零 GoalMutation/零 give_up）已三重
覆盖。Node 11 的增量价值在**"继续"两种形态的显式区分**：已完成形态（RC47
家族——路由 Done）与进行中形态（正常续跑）各至少一跑；"情绪+任务"复合输入
各至少一跑（历史 bug：机械切句）。有既有单测护底的项标注"回归确认"即可，
勿重复造轮子。

---

# 18. Node 12 — Final Reliability Matrix + Freeze Verdict

建立最终矩阵：

| 能力               | mechanism | behavior | evidence | status |
| ---------------- | --------- | -------- | -------- | ------ |
| Intent           |           |          |          |        |
| Plan             |           |          |          |        |
| Execution        |           |          |          |        |
| Fact             |           |          |          |        |
| Verification     |           |          |          |        |
| Failure Recovery |           |          |          |        |
| Completion       |           |          |          |        |
| Memory           |           |          |          |        |
| Resume           |           |          |          |        |
| Decision         |           |          |          |        |
| Terminal         |           |          |          |        |
| Projection       |           |          |          |        |
| Security         |           |          |          |        |

特别列出：

```text
false completion
false give_up
false stop
false replan
silent loss
reteach
duplicate execution
```

**（v1.1 补 S-4，守门员）版本与记账纪律**：本轮若发生代码改动（如 RC48 修复），
v0.2.17 → v0.2.18 bump + CHANGELOG 必须在最终 gate **之前**完成（既有约定）；
Final Report 的测试计数以 **447 为增量记账基线**（added/removed 逐项说明），
与 CHANGELOG 逐条对账。若本轮零代码改动（纯审查结论），gate 复跑 447/0 即可，
不发新版。

---

# 19. Final Freeze Decision 规则

## CORE FREEZE READY

要求：

```text
F0 = 0
```

并且：

```text Structural ✅
Behavioral ✅
Long-run ✅
Evidence ✅
Provenance ✅
```

没有任何未解释的核心行为异常。

---

## CORE FREEZE READY WITH ACCEPTED DEVIATIONS

允许：

```text
F0 = 0
```

但存在：

```text F1
```

要求：

```text 已明确边界
已确认不破坏核心事实模型
有复现证据
有后续处置方向
```

---

## NOT READY

任一：

```text
F0
```

或：

```text
事实不可解释
Decision authority 冲突
Provenance 不可靠
Memory 发生静默事实丢失
长程失败具有稳定机制原因且未处置
```

---

# 20. 最重要的判断纪律

本轮禁止：

```text
tests green
=
system reliable
```

禁止：

```text
1 次成功
=
long-run reliable
```

禁止：

```text
LLM says done
=
completed
```

禁止：

```text
archive exists
=
memory recoverable
```

禁止：

```text
mechanism works
=
behavior works
```

禁止：

```text
model failure
=
system failure
```

必须始终区分：

```text
mechanism
decision
model
provider
environment
```

---

# 21. Independent Review Package

最终必须生成：

```text
docs/core-freeze-review/
```

至少：

```text
architecture-map.md
authority-matrix.md
decision-terminal-map.md
fact-lifecycle.md
memory-lifecycle.md
failure-lifecycle.md
long-run-evidence.md
known-deviations.md
provenance.md
review-index.md
```

`review-index.md` 必须告诉独立评审：

```text
看哪个文件
为什么看
怎么复现
预期结果是什么
```

不能要求独立评审自己猜路径。

**（v1.1 必遵，承接砺批-7）清单硬格式 + 现成素材**：每项声明附 **grep/命令 +
预期输出**（三包 §8/§14/§17 已验证此格式可让评审窗快速核验；砺窗做 Freeze
独立审查时将按清单逐条跑，清单不全 = 打回）。三包已验证命令可直接引用为
review-index 条目素材：FA01 评审包 §14（六组）/ P2-MC 评审包 §17（六组）/
整合报告 §8（四组 + 真机复验），叠加本总包新增项（RC48 检验法 / C-probe /
切片判级）。

---

# 22. Final Report

一次性交付：

# `Hearth CORE FREEZE REVIEW-01 Final Report`

必须包含：

## 1. Executive Summary

明确：

```text
CORE FREEZE READY
CORE FREEZE READY WITH ACCEPTED DEVIATIONS
NOT READY
```

## 2. Current Architecture

## 3. Authority Matrix

## 4. Decision-Terminal Map

## 5. False Stop Investigation

## 6. History Slice Investigation

## 7. Archive Recoverability

## 8. Fact / Verification Retention

## 9. Compact + Resume

## 10. Long-run Product A

## 11. Long-run Product B

## 12. Controlled Failure

## 13. QA / Intent Boundary

## 14. Reliability Matrix

## 15. OPEN / UNKNOWN / DEFER

## 16. Provenance

## 17. Security

## 18. Independent Review Package

## 19. Final Freeze Recommendation

---

# 23. 重要的最终原则

本轮结束后，不允许因为发现：

```text
Subagent
TUI
MCP
Persona
Semantic Memory
更聪明的 Planner
动态 Verification
```

就重新打开 Core 架构。

这些已经不是：

> “Core 没完成”

而是：

> **Core Freeze 之后的 Evolution backlog。**

---

# 24. 最终使命

这轮不是让 Hearth 变得更“强”。

而是回答：

> **现有 Hearth Core 是否已经足够完整、稳定、可解释，可以冻结，不再频繁改动基础架构？**

最终标准不是：

> “它从来不会失败。”

而是：

> **“当它失败时，我们知道为什么；当它成功时，我们知道为什么；当它说完成时，我们有证据；当它恢复时，它不会把过去已经完成的事情重新弄乱；当上下文变化时，它不会失去任务事实；当决策发生时，我们知道是谁有权决定。”**

如果这套答案成立：

# `CORE FREEZE READY`

如果只有少量已知边缘：

# `CORE FREEZE READY WITH ACCEPTED DEVIATIONS`

如果核心事实、Decision、Memory 或 provenance 仍存在无法解释的问题：

# `NOT READY`

**执行窗口从 Node 00 开始连续执行，普通问题自行诊断和最小修复；不要在 Node 之间等待用户。**

---

# 附：砺·评审批注（2026-08-31，总判断：🟡 有条件通过——按批-1/批-2 纳入后放行；Freeze 结论预判见批-8）

> P2-LR 事后审计基线：整合报告（v0.2.17 `3b304dd`）§8 锚点本窗全实证——RC47 路由（loop.rs:4223 `GIVE_UP_ROUTED_TO_DONE`：criteria 空 + written_files 非空 + 0 errors → Done 产物校验兜底 + 正反测试 9402）、exit code 五态接线（`BashExitError` 类型化载体 dispatcher.rs:20-30 + scheduler 双 downcast ExitSignal/ExitNonZero:11-17 + 五态测试 235）、40 切片标记（loop.rs:2217-2220 `[history note]` + 保护对称性断言 9469-9470）、chat 首轮 session 绑定（run_local.rs:331）。提交链 1034bc8→adbb0bb→aee97aa→2aa32da→3b304dd→a724ddb 一致，tag v0.2.17 在库。gate 447/0 与真机日志在 VM 侧未复跑（证据边界如实声明）。

## 批-0（三包终态裁定建议，请顶层确认）

**P2-LR 维持 PASS WITH DEVIATIONS 并 CLOSED**；连同此前已建议 CLOSED 的 FA01/P2-MC，三包全部收口。残余 OPEN 按 F0/F1/F2 分级裁定：**F0 = 0**（零假完成/零静默丢失/零沙箱绕过/零无限循环，10 场景实证）；F1 三项 = false stop ×2（RC48，Node 03 最高优先级）/ archive C 未证明 / 切片零恢复通道；其余 F2 DEFER。**注**：.133 binary=0.2.16 vs source=0.2.17 的 provenance 异常（整合报告 §7 自曝）= Node 00 必办项，见批-2。

## 批-1（🔴 Node 03 false stop 深挖：三个机制假设预写，防"模型判断错误"一句话交差）

sortlib false stop ×2（mechanism 2/2 + 独立复验 2/2 passed，但 behavior 0/2）——修复完成后规划停滞终止。**我的假设排序（Node 03 逐个证伪）**：
1. **假设 A（最可能）**：sortlib 的 criteria **非空** → RC47 修复的"criteria 空路由 Done"分支（loop.rs:4223）**不触发** → planner GiveUp 臂命中且 Reserve 已尽 → 落原 give_up/stop。即 **RC47 修复只堵了 criteria 空的盲区，criteria 非空 + Reserve 零和的形态仍裸奔**——这正是 FA01 遗留 RC45（双上限零和）与机械根因"planner GiveUp 臂无 acceptance 感知"（修 A 备而未用条款）的叠加态。检验法：查 sortlib 跑的 criteria 是否非空 + Reserve 消耗日志（`VERIFICATION_RESERVE` / `acceptance_replan_count`）；
2. **假设 B**：steps_without_progress 语义——修复后的复测/验证步不计 progress（RC43 口径），planner 规则臂照常命中；
3. **假设 C**：决策输入缺 acceptance 事实（`grep acceptance planner/` = 0 的老根因——planner 在决策点看不到验收通过这一事实）。
**许可的修法**（按 Node 04 优先序）：Reserve 分账（RC45 预案）或 give_up 消费端对"acceptance passed 已在 scratch"的消费扩展——均须顶层批准；**若假设检验结论是 model variance → 只归档不修**（总包已有此句）。这 2 例与 RC47 已修形态的关键差异就是 criteria 空与非空——Node 03 报告必须明确写出这个分界。

## 批-2（🔴 Node 00 provenance 异常必须实修，不许只改报告文字）

整合报告 §7 自曝：.133 source=0.2.17 / binary=0.2.16 / gate 编译=0.2.17。三种可能：(a) 报告填写错误；(b) .133 系统 PATH 二进制确实滞后（v0.2.3 PATH 分叉家族复发形态）；(c) gate 构建产物与安装 binary 不同源。**Node 00 处置要求**：三查（binary --version / Cargo.toml / gate log 版本串）一致后，必须在 .133 重跑一次**最小冒烟**（如 `cargo test -p agent-core --lib test_rc47`）证明证据链活着，且把 .133 系统 PATH binary 重建到 0.2.17——**不允许以"gate 编译版本对"为由豁免系统 PATH 对齐**（这是本流水线 provenance 纪律的既有裁决）。LR 真机证据链本身不受影响（真机在 .131，binary=0.2.17 sha256 已记录）——此判断先给出，防 Node 00 把精力错花在重跑真机上。

## 批-3（🟡 Node 01 Authority Matrix：先填实测基线，再找冲突）

矩阵的正确做法是**先填现状（源码实测）再查双 authority/无 authority/越级**，不是按 Roadmap 理想态填空。砺窗已实证的基线锚点直接可用：complete 判定 = Done 相位 `verify_acceptance_criteria`（O-4，唯一生产者）；give_up 消费端 = 三路拦截 + Reserve 先决（修 D+FA01）+ RC47 路由；failure classification = `classify_failure` 纯函数（terminal.rs:277）；compaction 阈值 = provider-aware（loop.rs:4352-4370）；terminal = `normalize_terminal_state` 纯函数；planner GiveUp 臂 = 规则（planner/lib.rs:483/489/495-502，无 acceptance 感知=已知缺口）。**已知双轨预警**：give_up 的 authority 事实上分裂在 planner（决策）与 loop（消费拦截）两侧——这是修 D 预裁决的有意设计（planner schema 不动），矩阵里应记为"决策权/否决权分离"而非"双 authority 冲突"。

## 批-4（🟡 Node 05 切片审计：预期答案与标记文本的诚实性）

P2-LR 已补 `[history note]` 标记（loop.rs:2217-2220），但注意标记文本对模型说的是"原始记录仍完整保留于会话状态"——**这句话诚实但无恢复能力**（切片内容不在 archive，B 档 grep 救不回；模型既无工具也没有检索通道）。Node 05 判级预期：早期事实 = context 消失 + state 存活 + 无 archive + **零恢复通道**（lost-to-LLM）。这与批-1 的保护对称性对照表结论一致。若 Node 05 判定 F1（我预期是），最小改善方向二选一登记：切片内容同样入 archive（复用既有通道，零新架构）或标记文本改为"该内容已不可检索"（诚实性修正，零代码）。**冻结窗口内不建议做切片入 archive**（DEFER 合理）——但要写清这是"已知有界损失"而非"已保护"。

## 批-5（🔴 Node 06 Archive C-probe 防污染设计：答案必须只在 archive 里）

P2-LR 的 C-probe 结论"C 未证明（路径=保留上下文非 grep）"暴露了探针设计缺陷：**答案可由未压缩的保留上下文推出**（contaminated probe）——模型走了捷径，C 层根本没被测到。Node 06 必须重新设计：构造**事实只存在于被压缩轮次**的问题（该事实在保留的近 N 轮与 state 层均无痕迹，唯一载体是 archive），压缩后提问，回复做确定性 substring 比对（不采信自述）。判定三档沿用我 P2-LR 轮批-2：C 证明 / C 未证明机制在 / C 反证。若仍 C 未证明 → OPEN "archive preserved but model recoverability unproven" 保持诚实登记，**不阻塞 Freeze**（F1 级，边界已声明）——但 review-index 里必须把它列为"委托方需知的已知限制"，不是脚注。

## 批-6（🔵 Node 11 QA 边界：回归确认非重新发现）

修 B（疑问句类别）+ RC47 修复（正反测试 9402）+ P2-LR 真机（零 GoalMutation/零 give_up）已三重覆盖。Node 11 的价值在**"继续"两种形态的显式区分**：已完成形态（RC47 家族——路由 Done）与进行中形态（正常续跑）各至少一跑；样本含"情绪+任务"复合输入（历史 bug：机械切句）。有既有单测护底的项标注"回归确认"即可，勿重复造轮子。

## 批-7（🔵 Node 21 Independent Review Package：review-index 模板沿用砺窗惯例）

结构合格（review-index.md 要求"看哪个/为什么/怎么复现/预期结果"）。补一条硬要求：**每项声明附 grep/命令 + 预期输出**（三包 §8/§14/§17 已验证此格式可让评审窗快速核验）。砺窗做 Freeze 独立审查时将直接按该清单逐条跑，清单不全 = 打回。

## 批-8（Freeze 结论预判，供顶层预期管理）

基于三包实证现状：**F0 = 0 实证成立**，F1 三项中 false stop ×2（RC48）是唯一可能翻盘项。预判：
- 若 Node 03 把 false stop 根因落到**可解释层**（假设 A 的机制边界 = criteria 非空 + Reserve 零和，已有界；或 model variance）→ **CORE FREEZE READY WITH ACCEPTED DEVIATIONS**（现实目标，与整合报告 §9 判断一致）；
- 若 Node 03 证实 false stop 是**稳定机制缺陷且修法越界**（如需动 planner schema）→ STOP-2/STOP-5 → NOT READY，但按现有证据链这个概率低；
- **CORE FREEZE READY（无偏差）不现实**——archive C 未证明与切片零恢复通道是真实已知限制，硬凑全绿反而违反 §20 判断纪律。
砺窗的 Freeze 独立审查将在 Final Report + review package 到位后进行，届时按批-7 清单逐条核验。守门人零代码改动。

---

# 附 2：守门员复核定稿批注（2026-08-31，总包 v1.1 定稿依据）

> 定位：砺批-0~8 已审（附 1），本节是**对审计的审计**（锚点实测）+ 顶层定稿裁决。
> 实测基线：本机 HEAD `a724ddb`（tag v0.2.17 → `3b304dd` 之后含评审包 docs 提交，链一致）。

## 复-1 P2-LR 上游核验（整合报告 + 砺批注锚点全实证）

| 项 | 声明方 | 实测 |
|---|---|---|
| RC47 路由（criteria 空 + 产物在 + 0 errors → Done 产物校验兜底） | 整合报告 §3 + 砺附 | ✅ loop.rs:4223 `GIVE_UP_ROUTED_TO_DONE` |
| exit code 五态接线（BashExitError 类型化 + scheduler 双 downcast） | 同 | ✅ dispatcher.rs:20-30 + scheduler.rs:11/243 |
| 40 切片标记（[history note] + 保护对称性断言） | 同 | ✅ loop.rs:2217-2220 + 断言 :9469-9470 |
| chat 首轮 session 绑定 | 同 | ✅ run_local.rs:331 |
| 版本链 tag v0.2.17 / HEAD 3b304dd（本机 a724ddb 含 docs） | 同 | ✅ git 实测 |
| gate `.133:~/t_gate_lr_final.log` = 447/0 四 RC=0 | 同 | ✅ paramiko 实测 |
| .131 binary = 0.2.17 | 同 | ✅ 实测 |
| **.133 provenance 异常（source 0.2.17 / binary 0.2.16）** | 整合报告 §7 自曝 + 砺批-2 | ✅ **实测确认为真滞后**（.133 系统 PATH = 0.2.16，2026-08-31）——Node 00 必修项 |

结论：**P2-LR = PASS WITH DEVIATIONS 成立**（false stop ×2 如实登记为 RC48、C-probe 污染如实披露、10 场景三层口径完整），批-0 三包全部 CLOSED 裁定采纳写入 §0。

## 复-2 砺批-0~8 采纳状态

- 批-0（三包终态裁定 + F0/F1/F2 预分级）：**采纳**，写入 §0。
- 批-1（🔴 false stop 三假设预写）：**采纳**，内联 Node 03——假设 A（criteria 非空 + Reserve 零和裸奔）为最可能，检验法/修法许可/可比性纪律齐备。
- 批-2（🔴 provenance 实修）：**采纳 + 本窗实测加码**——异常定性为真滞后非笔误（见复-1），内联 Node 00。
- 批-3（🟡 Authority Matrix 实测基线）：**采纳**，内联 Node 01——含"决策权/否决权分离"非"双 authority 冲突"的定性。
- 批-4（🟡 切片判级预期）：**采纳**，内联 Node 05——lost-to-LLM 判级 + 两个最小改善方向。
- 批-5（🔴 C-probe 防污染）：**采纳**，内联 Node 06——contaminated probe 教训 + 三档判定 + review-index 已知限制条款。
- 批-6（🔵 QA 回归确认）：**采纳**，内联 Node 11——"继续"两形态显式区分。
- 批-7（🔵 review-index 硬格式）：**采纳**，内联 §21——grep+预期输出格式 + 三包清单素材引用。
- 批-8（Freeze 结论预判）：**采纳为顶层预期管理**——现实目标 = READY WITH ACCEPTED DEVIATIONS；READY（无偏差）不现实；NOT READY 概率低（详见复-4）。

## 复-3 守门员补充（S-1~S-4）

- **S-1（🔴 Node 00）**：.133 异常实测确认为真滞后——已并入批-2 条款（三查一致 + 重建系统 PATH + 最小冒烟；禁豁免系统 PATH 对齐；真机链不受影响不重跑）。gate 基线 447 登记。
- **S-2（🟡 Node 08/09/10）**：criteria 冻结 + 故障注入点显式覆盖——三连教训（FA01 假阳性 / P2-MC B 档 0 使用 / P2-LR C-probe 污染）后升为硬门槛。
- **S-3（🔴 Node 08）**：受控压缩触发（`HEARTH_COMPACT_CHAR_THRESHOLD`）——provider-aware 阈值下自然任务永不触发压缩，Node 08 compact 环节必须用仪器制造压缩点并声明 env 值。
- **S-4（🟡 Node 12）**：447 增量记账基线 + bump/CHANGELOG 先于 gate；零代码改动则 gate 复跑即可不发版。

## 复-4 定稿裁决（含 Freeze 预期管理）

**本总包 v1 → v1.1 定稿，放行执行。** 砺批-0~8 全部采纳并内联（Node 00/01/03/05/06/11/§21）；
守门员补 S-1~S-4（Node 00/08/12）。顶层对 Freeze 结论的预期管理采纳批-8 预判：
- 现实目标 = **CORE FREEZE READY WITH ACCEPTED DEVIATIONS**（F0=0 已三包实证；F1 三项收敛为"可解释、有界、已声明"即达成）；
- **READY（无偏差）不现实**——archive C 未证明与切片零恢复通道是真实已知限制，硬凑全绿违反 §20 判断纪律；
- NOT READY 仅在 RC48 被证实为"稳定机制缺陷且修法越界（动 planner schema）"时触发（STOP-2/5）——现有证据链下概率低。
本总包 Final Report + review package 到位后，砺窗按批-7 清单执行 Freeze 独立审查；四套 INV 前缀
（ED01/FA01/M/LR）+ F0/F1/F2 分级继续沿用。守门人零代码改动。
