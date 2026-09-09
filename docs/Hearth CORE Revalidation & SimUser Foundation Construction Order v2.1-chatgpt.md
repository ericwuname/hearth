# Hearth CORE Revalidation & SimUser Foundation Construction Order v2.1

> **性质**：长程连续施工总包——一次批准、连续执行、节点内自测、跨节点不等待确认，最终一次性交付。
>
> **执行窗口**：GLM / 砺·执行
>
> **设计窗口**：顶层 / ChatGPT / Claude 独立评审
>
> **当前工作基线**：Hearth v0.2.19，`P3-BACKLOG-01` 完工态；Core Freeze 已因 v0.2.18 真人盲测证据 **SUSPENDED**，不得视为现行有效 Freeze。
>
> **总包目标**：
>
> 1. 建立独立于执行窗口的拟真用户测试基础设施；
> 2. 用真实交互重新验证 Projection、Intent、Decision、Memory、Long-run 等关键性质；
> 3. 对 RC52 连败吸引子进行**因果实验**，禁止先入为主直接修 T4；
> 4. 对已暴露问题按 evidence → attribution → minimal fix → independent rerun 闭环；
> 5. 最终形成一份新的 **Core Freeze Candidate** 证据包，但**执行窗口禁止自行宣布 CORE FREEZE**。
>
> **核心原则**：
>
> `Test infrastructure → Baseline observation → Attribution → Minimal fix → Independent regression → External review → Freeze decision`
>
> 而不是：
>
> `猜根因 → 修代码 → 自测通过 → 宣布 Freeze`

---

# 0. 全局执行模式与绝对红线

## 0.1 连续执行纪律

本总包采用 **Long-run Continuous Execution**：

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

**执行窗口不得在 Node 之间等待用户/顶层确认。**

允许执行窗口在节点内部：

```text
inspect
→ write test
→ run red
→ implement
→ run green
→ VM test
→ self-check
→ commit
→ proceed
```

只有以下 STOP 条件才能中断总包：

- 发现会改变既有安全模型的设计级问题；
- 发现无法维持单一事实源；
- 发现测试台本身无法可靠区分 Core 行为与 Driver 行为；
- 需要新增权限、执行能力、外部副作用能力；
- 需要修改冻结中的核心语义，但本施工单没有授权；
- 发现证据链与实际代码/二进制无法对账。

普通失败、测试失败、模型失败、provider 波动、环境差异，都应在 Node 内完成归因，不得因为一次失败就停包。

---

# 0.2 当前基线与 provenance

基线：

```text
version = 0.2.19
source = ~/codex / ~/codex_t（按 vm-version-sync 分窗登记）
```

必须首先核实：

```text
binary version
cargo version
source marker
git HEAD / tag
VM host
disk free
gate script
```

门禁仍只允许：

```text
~/run_gate_r2c.sh
```

禁止使用任何操作其他窗口 `~/codex` 的旧门禁脚本。

磁盘保险丝：

```text
/home free < 10G → STOP-RESOURCE
```

---

# 0.3 全局不可触碰区

本总包默认禁止修改：

```text
TaskGraph 事实模型
TaskGoal 原始目标持久化语义
Terminal 九态定义
ApprovalPolicy / HardRedline
sandbox fail-closed 总体语义
seccomp 总体策略
ContextBuilder 核心事实模型
单一 Task Continuity 注入路径
Bridge
Subagent
TUI
MCP
Persona 功能化人格系统
多轮显式推理系统
Semantic Memory 新事实源
动态 Verification 权威系统
```

特别强调：

**本总包不允许为了“让 benchmark 好看”去降低真实成功标准。**

---

# 1. Node 00 — Baseline Lock & Freeze Status Reset

## 目标

把当前系统状态正式从：

```text
CORE FREEZE
```

改为内部工程状态：

```text
CORE FREEZE SUSPENDED
CORE REVALIDATION IN PROGRESS
```

不得继续引用旧 Freeze 作为当前有效验收结论。

## 必做

1. 双 VM provenance；
2. gate 基线；
3. v0.2.19 当前 source/binary 对账；
4. 读取以下文档并建立索引：

```text
docs/consolidated-remediation-ledger-v3.md
docs/hearth-p0-attribution-01-final-report-v1.md
docs/hearth-p3-backlog-01-final-report-v1.md
docs/拟真用户测试台与自反馈闭环规划书 v2.0
docs/Hearth CORE FREEZE 顶层评审总览-砺.md
```

5. 建立：

```text
docs/core-revalidation/
```

作为本总包证据总目录。

## 产物

```text
docs/core-revalidation/baseline.md
docs/core-revalidation/decision-status.md
```

## 验收

基线 provenance、版本、gate、磁盘状态全部可复核。

---

# 2. Node 01 — SIMUSER-01 Stage 1：测试台骨架

> **这是本总包的第一主战场。**

目标不是“模拟一个用户”，而是先做一个**可信的测试环境**。

## 2.1 双通道采集

必须同时保存：

### 用户侧 Reality

用户实际看到的：

```text
stdout
stderr
终端 ANSI/渲染结果
裸 ERROR / WARN
状态行
Done 行
截断内容
```

### Hearth 内部 Reality

至少：

```text
telemetry
run report
event log
scratch
archive timestamp
planner input dump（仅在 debug 开关打开时）
TaskGraph fingerprint
goal revision
terminal state
```

两个通道必须以：

```text
session_id + run_id + timestamp
```

对齐。

最终能够生成：

```text
internal_event
↔
user_visible_event
```

diff。

---

# 2.2 PTY Driver

必须真正驱动：

```text
hearth chat
hearth resume
hearth repl
```

优先使用真实 PTY。

禁止把：

```text
stdin=/dev/null
```

伪装成交互测试。

必须解决并记录：

- PTY 输入消费竞争；
- EOF；
- 非 TTY approval；
- Ctrl-C；
- 多轮输入；
- session 生命周期；
- resume。

Driver 必须支持：

```text
send()
expect()
wait_terminal()
capture()
timeout()
interrupt()
```

并且能够保证：

> 用户输入只被测试 Driver 消费一次。

---

# 2.3 Analyzer 1–10

第一阶段实现以下确定性检测器：

```text
projection_leak
consecutive_failure_run
t4_stall_burst
revision_explosion
compaction_adjacency
truncated_output
giveup_completed_coexist
tool_misroute
introspect_failure
user_visible_completion
```

每一个检测器必须有：

```text
input
algorithm
output
severity
run scope
false-positive notes
```

---

# 2.4 指标定义冻结

写入：

```text
tools/simuser/metrics.yaml
```

至少冻结：

```text
success_rate
consecutive_failure
false_completion
false_giveup
false_replan
false_stop
reteach_count
projection_leak_count
projection_completeness_gap
goal_revision_rate
```

所有指标必须按：

```text
run boundary
first terminal state
```

定义。

禁止：

```text
grep "Task completed"
grep "failed"
wc -l
单纯关键词计数
```

直接作为最终成功率。

---

# 2.5 测试台自身验收

执行：

```text
未修复样本 → detector 应命中
已知干净样本 → detector 应归零
```

如果 detector 自身无法复现已知历史缺陷：

```text
STOP-SIMUSER-SELF-TEST
```

---

# 3. Node 02 — Projection Reality Audit

目标：

重新验证“内部状态 → 用户可见状态”的完整性。

## 必查

### A. Projection isolation

搜索所有：

```text
println!
eprintln!
tracing::error!
tracing::warn!
```

确认：

> 后端日志是否绕过 Projection 直接进入用户终端。

特别关注此前已确认的：

```text
do_plan_inner failed
```

路径。

## B. Projection completeness

建立两个数：

```text
internal_summary_size
user_visible_body_size
```

检测：

```text
有内部结果
但用户只看到 Done 行
```

必须单独识别。

## C. Projection correctness

确保：

```text
failed ≠ completed
error ≠ success
give_up ≠ hidden
```

不允许仅验证颜色/符号。

## D. Raw internal representation leak

特别检查：

```text
Text("...")
Debug enum
结构化内部对象
```

是否泄露给终端。

## 产物

```text
docs/core-revalidation/projection-audit.md
```

**本 Node 原则：先证明问题，再决定修复。**

---

# 4. Node 03 — RC52 Causal Experiment

> **不得直接修改 T4 阈值。**

RC52 当前定义：

```text
Observed attractor
Attribution incomplete
```

必须先做因果实验。

## 实验矩阵

至少三种条件：

### Condition A：Fresh

全新 session：

```text
同一诊断任务
无历史污染
无压缩
```

### Condition B：Contaminated

模拟：

```text
出现一次 stalled
继续进入同一 session
压缩
继续追问失败原因
```

### Condition C：Resume

同一 session：

```text
failed
resume
继续询问失败原因
```

## 每种条件建议

：

```text
n >= 5
```

不要求统计学论文级显著性，但不得 n=1 推导结论。

## 记录

至少：

```text
TaskGraph fingerprint
goal
goal_revision
compact events
planner prompt hash
TaskControl / Conversation / Mutation
T4 count
give_up
terminal
projection
```

## 核心比较

```text
Fresh vs Contaminated
Fresh vs Resume
```

重点看：

> 是否由上下文/历史污染导致相同 TaskGraph 重复出现。

如果：

```text
Fresh 正常
Contaminated 连续失败
```

优先沿：

```text
Context / Anchor / Compact / Continuity
```

调查。

如果三者都一样：

优先沿：

```text
Task type / Decision / T4
```

调查。

---

# 5. Node 04 — RC52 Attribution Decision

只在 Node 03 数据完成后执行。

不得事先写：

```text
T4 是主因
```

必须按：

```text
mechanism
decision
model
provider
environment
driver
```

六层归因。

输出：

```text
CAUSE CONFIRMED
CAUSE LIKELY
CAUSE UNKNOWN
FALSE POSITIVE
DRIVER-INDUCED
```

**特别增加 `DRIVER-INDUCED`。**

这是 SimUser 建设后第一次正式启用这个层级。

---

# 6. Node 05 — Projection Fix Batch

根据 Node 02 实证结果，只修已确认的 Projection 缺陷。

第一候选：

### RC51-A Projection Isolation

处理：

```text
tracing error → stderr → terminal
```

目标：

后端诊断日志仍可存在，但不再直接污染正常用户投影通道。

### RC51-B Projection Completeness

保证：

```text
终态 ≠ 只显示一行状态
```

至少用户能得到足够的结果摘要/行动信息。

注意：

不得把完整内部日志全部搬到用户界面。

应形成：

```text
事实 → 用户可理解投影
```

而不是：

```text
内部日志 → 原样倾倒
```

## 验收

必须使用：

```text
Node 02 baseline
vs
修复后相同场景
```

做 before/after。

---

# 7. Node 06 — RC53 / RC54 最小修复

## RC53

检查：

```text
introspect
```

被错误生成到：

```text
bash "introspect"
```

的路径。

必须区分：

```text
Tool invocation
Shell command
```

任何修复不得靠增加几十个黑名单关键词。

优先找到：

```text
structured tool intent
→ shell serialization
```

的真正边界。

## RC54

检查：

```text
apply_patch
```

空白/缩进脆弱匹配。

第一目标：

建立确定性 fixture。

是否进一步提高容错，依据 fixture 决定。

不得让 patch matcher 变成：

```text
“尽量猜用户到底想改哪”
```

而保持确定性。

---

# 8. Node 07 — Intent Adversarial Audit

基于 SimUser 语料重新构造：

```text
情绪吐槽
闲聊
自我指涉
真正歧义
吐槽 + 真实任务混合
material dump
vague delegation
```

同时加入：

```text
“你能回复我一下情况吗”
“上一轮为什么失败”
“继续”
“帮我看下刚才那个”
```

等真实高频输入。

## 验收原则

不能只看：

```text
Task / QA 分类准确
```

必须同时看：

```text
正确分类
revision 是否变化
是否错误进入 TaskGraph
是否错误进入 Recovery
是否错误触发 Approval
```

---

# 9. Node 08 — Decision / Terminal Mapping Audit

建立正式映射表：

```text
Decision
  Continue
  Replan
  Complete
  Escalate
  Stop
```

↓

```text
Terminal
  completed
  failed
  give_up
  verify_failed
  timeout
  cancelled
  error
  ...
```

要求：

- Stop ≠ GiveUp；
- Escalate ≠ Approval；
- Escalate ≠ Stop；
- GiveUp ≠ Completion；
- Approval Denied ≠一般 Tool Failure。

特别验证：

```text
delegation active
+
Escalate
+
没有人可以响应
```

系统到底如何处理。

不得留下隐式 fallback。

---

# 10. Node 09 — Memory / Context Reality Audit

重新检查：

### A. 40 message slice

验证：

```text
被切掉的信息
用户能否知道被切了
是否可恢复
```

### B. Archive

区分：

```text
archive exists
archive searchable
model can actually use archive
```

三件事。

### C. Compaction

记录：

```text
什么时候触发
为什么触发
触发前后 token
archive
session binding
```

### D. Compression calibration

不得直接把：

```text
32K
512K
```

当成理论结论。

继续使用真实 provider usage 做交叉验证。

---

# 11. Node 10 — SimUser Stage 2

只有 Stage 1 稳定后进入。

## B1

计算维度联合关系：

```text
mode × task_domain
mode × length
tone × style
```

不得各维独立随机。

## B2

建立：

```text
persona_vector
scenario matrix
operational
strategic
blend
```

## B3

加入检测器：

```text
multi_intent_drop
anaphora_resolution_fail
escalation_run
paste_error_probe_quality
longpaste_truncation
```

总数达到：

```text
15 detectors
```

## B4

测试台自检：

```text
历史缺陷 → 大面积命中
干净样本 → 接近零
```

---

# 12. Node 11 — SIMUSER Controlled Campaign

> **第一次正式大规模行为证明。**

版本冻结。

执行窗口停止修改 Core。

建议：

```text
30 runs minimum
```

如果指标稳定，再：

```text
100 runs
```

campaign 内禁止：

```text
边测边改
```

测试窗口负责：

```text
run
capture
detect
archive
```

执行窗口不得自行作为 campaign judge。

---

# 13. Node 12 — Failure / Attractor Campaign

专门追：

```text
RC52
RC48
QA stall
goal revision churn
projection leaks
tool misroute
```

至少包含：

```text
Fresh
Long-session
After-stall
After-compaction
Resume
Continue
Failure-diagnostic
```

每一类都保留：

```text
successful case
failure case
counterexample
```

禁止只保存失败样本。

---

# 14. Node 13 — Minimal Fix Batch

只有 Node 03–12 证据确认之后才能执行。

修复优先级：

```text
P0
Projection isolation/completeness
RC52 confirmed mechanism/decision root cause

P1
RC53
RC54
Intent edge cases

P2
Memory slice
Archive recoverability
minor UX
```

每个修复必须：

```text
failure fixture
↓
red
↓
minimal diff
↓
green
↓
independent rerun
```

不得：

```text
为了提高 benchmark 成功率
修改 benchmark
```

---

# 15. Node 14 — Final Independent Campaign

这是本总包最终行为验收窗口。

## 规则

版本冻结。

执行窗口不再修代码。

由：

```text
独立测试窗口
```

执行：

```text
SimUser campaign
+
历史回放
+
真人盲测回归
```

至少覆盖：

```text
Operational
Strategic
QA
Multi-intent
Anaphora
Failure diagnosis
Continue
Resume
Compaction
Projection
```

---

# 16. Node 15 — Core Freeze Candidate Package

**本 Node 不得宣布 CORE FREEZE。**

只能产出：

```text
CORE FREEZE CANDIDATE
```

必须生成：

```text
docs/core-freeze-candidate/
```

至少包括：

```text
architecture-status.md
behavioral-status.md
projection-status.md
simuser-status.md
open-deviations.md
evidence-index.md
independent-review-request.md
freeze-candidate.md
```

---

# 17. file_issue —— 本总包暂不实施

`file_issue` 不属于第一批。

原因：

> 我们先证明 SimUser 和检测器到底会产生哪些高价值问题，再决定哪些事件值得进入 Core 的 issue 通道。

因此本总包：

```text
file_issue = DEFERRED
```

不得因为“以后需要”而顺手进 Core。

---

# 18. 强制证据分层

所有新发现必须标记：

```text
OBSERVED
REPRODUCED
CONFIRMED
LIKELY
UNKNOWN
FALSE POSITIVE
DRIVER-INDUCED
```

禁止：

```text
Observed → Confirmed
n=1 → reproducible
script pass → behavior pass
model self-report → evidence
```

---

# 19. 强制“测试台也可能错”

SimUser 结果必须区分：

```text
Core defect
Driver defect
Detector defect
Environment
Provider
Model
Unknown
```

测试台不得成为“自动真理机”。

---

# 20. Final Acceptance：两层标准

## Weak Acceptance

```text
所有节点执行成功
所有 gate 绿色
所有工具正常
```

只能证明：

```text
结构完整
```

## Strong Acceptance

必须同时证明：

```text
1. Projection internal/user-visible 一致
2. RC52 在至少一个真实场景被因果解释
3. 没有新的无限失败吸引子
4. Intent/QA/Task 路由稳定
5. Resume reteach = 0
6. Compaction 不静默吞事实
7. Verification 不被 self-report 替代
8. 无 sandbox/approval 绕过
9. SimUser 30+ run campaign 有稳定指标
10. 真人盲测不出现已知 F0 级缺陷
```

**只有 Strong Acceptance 才能进入 Freeze Review。**

---

# 21. Freeze Review 程序边界

执行窗口不得写：

```text
CORE FREEZE = APPROVED
```

只能写：

```text
CORE FREEZE CANDIDATE = READY
```

之后必须由：

```text
人工评审窗口
+
外部 AI 评审窗口
+
至少一次真正独立的命令/证据复核
```

共同形成最终裁决。

明确：

> **执行窗口负责施工与证据归档，不拥有最终 Freeze 权。**

---

# 22. Final Report 格式

最终一次性提交：

```text
1. Executive Summary
2. Version / Provenance
3. Nodes executed
4. Test infrastructure status
5. SimUser campaign
6. Projection comparison
7. RC52 attribution
8. Fixes
9. Regression
10. Strong Acceptance
11. OPEN / ACCEPTED DEVIATION / UNKNOWN / DEFERRED
12. Evidence index
13. Independent review handoff
14. Recommendation = CORE FREEZE CANDIDATE / NOT READY
```

---

# 23. 本总包成功的真正定义

不是：

```text
gate 通过
```

不是：

```text
30 次跑完
```

甚至也不是：

```text
成功率提高
```

而是：

> **我们终于具备了一套能独立发现、复现、归因、验证 Hearth 行为问题的系统，因此下一次即使再发现问题，也不会重新陷入“到底是模型、代码、环境还是测试方法”的循环。**

---

# 24. 最终执行顺序

严格按：

```text
00 Baseline
 ↓
01 SimUser Stage 1
 ↓
02 Projection Audit
 ↓
03 RC52 Causal Experiment
 ↓
04 Attribution
 ↓
05 Projection Fix
 ↓
06 RC53/54
 ↓
07 Intent Audit
 ↓
08 Decision-Terminal Audit
 ↓
09 Memory Audit
 ↓
10 SimUser Stage 2
 ↓
11 Controlled Campaign
 ↓
12 Failure/Attractor Campaign
 ↓
13 Minimal Fix Batch
 ↓
14 Independent Final Campaign
 ↓
15 Core Freeze Candidate
```

---

# 25. 最重要的执行纪律

**本总包不允许出现以下节奏：**

```text
做 2 个 Node
→ 等用户
→ 再做 2 个 Node
→ 等用户
→ 再做 3 个 Node
```

正确节奏必须是：

```text
一次下发
→ 连续施工
→ 节点内自测
→ 节点间自动推进
→ STOP 条件才停
→ 最终一次报告
```

**Node 只是内部施工边界，不是反馈边界。**

每一个 Node 完成后：

```text
先测试
先验收
先归档
先 commit
```

然后直接进入下一个 Node。