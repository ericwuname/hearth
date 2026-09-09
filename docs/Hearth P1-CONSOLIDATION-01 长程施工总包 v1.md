# Hearth P1-CONSOLIDATION-01 长程施工总包 v1
## Execution Boundary / Task Semantics / Final Stability Consolidation

**执行模式：长程连续自主施工。**

**重要：本任务包不是“节点做完就向顶层汇报”的任务。**

GLM 获得本任务包后，应在一个连续执行窗口内，从 Phase 0 开始，一直执行到最终验收。

每个 Node 内部必须：

```text
源码核实
→ 设计确认
→ 修改
→ 单测
→ 集成测试
→ VM gate
→ 必要的真机验证
→ 回归检查
→ commit
→ 自验收
→ 自动进入下一个 Node
```

**Node 之间不要等待用户确认。**

只有触发本文定义的 `STOP CONDITION`，才允许停止整个长程任务并向顶层报告。

---

# 0. 当前有效基线

本任务以以下版本为起点：

```text
Version: v0.2.11
HEAD: e03565f
P1-LTR-01: FINAL PASS
Gate: 415 passed / 0 failed
```

当前已经正式关闭、**不得重复施工**：

```text
R2-C ContextBuilder
W3/W4
RC24
W8 主项
P1-LTR-01
N-2 one-shot Ctrl-C projection
```

当前已知待处理/观察：

```text
O-3      seccomp SYS_MKDIRAT
Q-3      Landlock ABI / capability handling
OPEN-W8-1  Task Type × Constraint 边界
DEV-2    Reflect decision quality
verify_failed empirical fixture
N-3      approval_delegated Markdown projection
```

当前已经确认但不属于本总包主体的长期项：

```text
Subagents
TUI
MCP
Bridge
```

本总包不扩展这些方向。

---

# 1. 总包目标

本阶段目标不是增加大量新功能。

目标是把 Hearth 从：

```text
“主要功能已经存在”
```

进一步收敛为：

```text
执行边界正确
+
任务路由正确
+
任务事实可信
+
终态语义完整
+
长期运行可验证
```

最终希望形成：

```text
用户目标
  ↓
任务类型识别
  ↓
正确执行路径
  ↓
实际工具执行
  ↓
事实产生
  ↓
任务状态
  ↓
验证
  ↓
终态
  ↓
用户投影
```

同时保持：

```text
Sandbox
Approval
Deadline
Resume
ContextBuilder
```

之间互不污染。

---

# 2. 总包成功条件

最终至少满足：

### Execution Boundary

- seccomp 已知 syscall 缺口得到明确结论；
- Landlock ABI / rights 行为得到明确结论；
- `/dev/null` / regular file / directory 边界正确；
- RC24 审批语义没有回归；
- deadline / cancellation 没有回归。

### Task Semantics

- Product Task 与 QA/Discussion 路由正确；
- Negative Constraint 不再错误吞掉 Product Intent；
- Task Control 不污染 Goal Revision；
- verify_failed 有可重复端到端构造；
- Reflect 与事实层之间的冲突至少可被观察和分类。

### Projection

- known structured facts 在 CLI/report 中一致；
- approval delegation 在所有主要 report 投影中一致。

### Regression

- R2-C / W3/W4 / RC24 / W8 / P1-LTR 全量回归；
- 20+ step 长程任务至少 1 轮；
- 真机执行证据完整；
- 最终 release provenance 正确。

---

# 3. 全局红线

任何 Node 均不得：

```text
重新打开 R2-C ContextBuilder 架构
重新定义 TaskGoal
重新定义 TaskGraph
修改 terminal 九态规范
改变 ApprovalPolicy 的既定语义
解除 HardRedline
放宽 sandbox
放宽 seccomp
改变 Landlock fail-closed 安全语义
修改事件契约以绕开兼容问题
引入第二套事实源
引入第二套 Task Continuity
把 Reflect LLM 直接升级为绝对事实裁决者
```

尤其禁止：

```text
“模型表现不好”
→
直接加入更多 prompt
→
直接改变 terminal
```

以及：

```text
“测试需要”
→
直接放宽 sandbox/seccomp
```

所有安全边界变更必须满足：

```text
真实 syscall / kernel 证据
+
最小权限原则
+
正向测试
+
负向测试
+
VM 真机验证
```

---

# 4. 自动执行协议

## 4.1 普通失败：自行处理

下列情况不得停下来等用户：

```text
cargo test failed
clippy failed
fmt failed
普通真机任务失败
fixture 失败
单个模型调用失败
环境中出现普通 stale process
普通 assertion mismatch
```

流程：

```text
定位
→ 判断是否属于当前 Node
→ 修复
→ 重测
→ 继续
```

---

## 4.2 必须 STOP

只有以下情况允许停止整个总包：

### STOP-1

必须修改本文红线列出的架构。

### STOP-2

必须改变现有事件 schema。

### STOP-3

发现 sandbox/seccomp/landlock 安全语义必须放宽。

### STOP-4

发现现有设计前提被实测完全推翻，且本文无法裁决。

### STOP-5

发现潜在安全漏洞，无法通过最小安全修复自行处理。

### STOP-6

需要新的外部账号、秘密、不可逆权限。

### STOP-7

VM 基础环境无法恢复，且替代方法会改变测试真实性。

除此之外：

**自动继续。**

---

# 5. Node 00 — Baseline Lock

## 目标

确认真正的开工状态。

核实：

```text
git status
git rev-parse HEAD
Cargo version
tag
.131 binary
.133 binary
```

检查：

```text
~/codex
~/codex_t
```

不得误写 `~/codex`。

使用：

```text
~/run_gate_r2c.sh
```

执行基线 gate。

记录：

```text
host
source path
binary path
HEAD
version
gate log
passed/failed/ignored
```

### Gate

基线必须全绿。

### 输出

内部记录即可，不向顶层发送中间报告。

---

# 6. Node 01 — G0 Sandbox Capability Closure

## 目标

处理：

```text
O-3 seccomp
Q-3 Landlock ABI
```

并复核现有 N1-SBX capability。

---

## 6.1 O-3：SYS_MKDIRAT

当前已知：

```text
mkdir/mkdirat
→
SIGSYS / exit 159
```

要求先源码定位：

```text
seccomp allowlist
syscall wrapper
实际 caller
```

回答：

> `mkdirat` 是否属于 Hearth 已授权 filesystem operation 的自然实现路径？

只有答案为 YES 时才允许加入。

加入时：

```text
只添加最小 syscall
不放宽其他 syscall
保持 deny 默认
```

必须加入：

### Positive

```text
directory creation succeeds
```

### Negative

至少一个已有不允许 syscall 仍然被拒绝。

### Regression

现有：

```text
read
write
exec
bash
```

不回归。

---

## 6.2 Q-3：Landlock ABI

先确认：

```text
当前 ABI detection
当前 ABI requested
当前 FS rights
ABI >= required ABI 的真实原因
```

不要把：

```text
kernel ABI >=5
```

直接等同：

```text
必须强制使用 ABI 5
```

必须判断当前实现到底需要哪些 rights。

如果当前代码存在：

```text
feature detection
```

优先复用，不新增第二套 capability detector。

必须测试：

```text
supported ABI
unsupported / insufficient ABI
```

并明确：

```text
fail-closed
vs
feature degradation
```

不能混淆。

---

## 6.3 N1-SBX 回归核查

不要重新施工 N1。

只验证：

```text
/dev/null
regular file
directory
```

规则层和实际执行层仍然成立。

至少确认：

```text
FS_FILE_ONLY
FS_RO_FILE
FS_RW
FS_RO
```

语义无回归。

---

## 6.4 Node 01 Gate

必须：

```text
cargo fmt
cargo clippy
cargo test
RT4
VM real-machine
```

全部通过。

---

# 7. Node 02 — Task Type × Constraint 路由收口

这是本总包的主要业务语义 Node。

## 背景

当前已知：

```text
“不要修改任何现有文件”
+
“写一个新的 README.md”
```

会因为 negative signal 优先，而错误走 QA。

正确语义应是：

```text
Product Task
+
Negative Constraint
```

而不是：

```text
QA
```

---

# 8. Node 02 核心设计原则

禁止继续无限增加关键词。

需要将：

```text
Task Type
```

和：

```text
Constraints
```

概念上分离。

至少保证：

```text
“不要修改现有文件”
```

只能说明：

> 某类行为被禁止。

不能单独推出：

> 这是一个 QA / Conversation。

---

## 8.1 分类器目标

保留现有：

```text
TaskControl
Conversation
GoalMutation
```

并在 Product/QA 判断上形成：

```text
Task Intent
+
Constraint Signals
```

逻辑上至少满足：

```text
有明确 Product / Artifact Intent
+
存在 Negative Constraint
→
Product Task
```

以及：

```text
只有 Negative Constraint
且无 Product/Action Intent
→
QA / Conversation
```

具体实现方式必须先阅读现有 classifier，不得假定必须新建 enum。

---

## 8.2 测试矩阵

至少覆盖：

### A

纯 QA：

```text
解释一下当前实现思路
```

### B

纯 Product：

```text
创建 README.md
```

### C

Product + Negative Constraint：

```text
创建新的 README.md，但不要修改任何现有文件
```

### D

Negative only：

```text
不要修改任何文件
```

### E

Negative + existing reference：

```text
不要修改现有文件，告诉我应该怎么处理
```

### F

Product + multiple constraints：

```text
创建 config.json，但不要修改 Cargo.toml，也不要删除任何文件
```

### G

TaskControl + constraint：

```text
继续，不要修改现有文件
```

必须先识别 TaskControl。

---

## 8.3 真机测试

至少执行：

```text
创建一个新文件
+
确保既有文件 hash 不变化
```

同时记录：

```text
task type
constraints
artifact
terminal
```

---

# 9. Node 02 Gate

至少：

```text
classifier unit tests
integration tests
old regressions
5+ real-machine cases
```

通过后自动进入 Node 03。

---

# 10. Node 03 — DEV-2 Reflect Decision Evidence

**这是一个证据/决策质量 Node，不允许直接把 Reflect 改造成最高事实权威。**

当前已知：

```text
tool success
+
artifact exists
+
0 errors
+
fact progress
↓
Reflect
↓
give_up
```

这一问题必须进一步拆解。

---

## 10.1 建立事实链

对真实 LLM 样本记录：

```text
original_goal
current_goal
task_graph
successful_tools
successful_write_count
artifacts
verification
reflect_prompt
reflect_output
terminal decision
```

至少分析：

```text
Fact says progress
Reflect says no progress
```

发生时：

```text
哪些事实被 Reflect 看见？
哪些事实被遗漏？
哪些事实被模型误解释？
```

---

## 10.2 Observe-only conflict marker

允许增加：

```text
reflect_fact_conflict
```

这样的观测信息。

但：

**本 Node 不允许让 conflict 自动修改 terminal。**

例如：

```text
fact completion evidence
+
reflect give_up
```

可以记录：

```text
REFLECT_FACT_CONFLICT
```

但不能直接：

```text
→ completed
```

也不能：

```text
→ force continue
```

---

## 10.3 样本

至少 10 个真实 LLM run：

```text
5 Product / artifact tasks
3 QA / discourse tasks
2 mixed / constraint tasks
```

每个 run 必须记录：

```text
correct
incorrect
ambiguous
```

而不是只记成功率。

---

## 10.4 目标

最终必须回答：

### Q1

Reflect 错误是：

```text
context omission
```

还是：

```text
prompt representation
```

还是：

```text
task classification
```

还是：

```text
model judgment
```

### Q2

什么时候：

```text
Fact
+
Reflect
```

发生冲突？

### Q3

现有系统有没有已经存在的 verifier 可以用于二次确认？

如果没有：

**不要自己造第三套 verifier。**

---

# 11. Node 03 Gate

这里的 Gate 不是：

> “Reflect 变聪明了。”

而是：

> **冲突可以被观察、分类、复现。**

如果能够确定一个**不改变终态语义的最小诊断修复**，可以实施。

如果最终需要：

```text
Fact overrides Reflect
```

或：

```text
Reflect overrides Fact
```

这种控制流改变：

**STOP，提交架构偏差报告。**

---

# 12. Node 04 — verify_failed End-to-End Fixture

目标：

> 给 `verify_failed` 一个真正可重复的端到端构造。

要求：

```text
Task executes
→
artifact exists
→
verification deliberately fails
→
terminal = verify_failed
→
report/CLI correctly projects
```

---

## 12.1 不允许篡改 production logic

优先寻找：

```text
existing verification hook
existing verifier
test fixture
```

如果可以通过 fixture 注入：

优先使用 fixture。

不要为了测试目的给 production code 增加奇怪的：

```text
force_verify_fail=true
```

除非已有 testing architecture 支持。

---

## 12.2 终态分离

必须证明：

```text
verify_failed
≠
failed
≠
give_up
≠
timeout
≠
cancelled
```

至少：

```text
terminal reason
report
CLI projection
```

都应该保持可区分。

---

# 13. Node 04 Gate

必须：

```text
fixture deterministic
unit
integration
real-machine if applicable
```

通过后继续。

---

# 14. Node 05 — N-3 Projection Completeness

这是低风险收尾 Node。

目标：

```text
approval_delegated
approval_delegated_cmds
```

在 Markdown report 中正确展示。

禁止修改：

```text
ApprovalPolicy
delegation semantics
audit semantics
```

仅修 projection。

---

## 测试

至少：

```text
delegated=false
delegated=true + 1 command
delegated=true + multiple commands
empty commands
```

确认：

```text
JSON
Markdown
CLI
tracing
```

不会产生语义冲突。

---

# 15. Node 06 — Cross-Layer Regression

这一 Node 不新增功能。

目标是确认：

```text
R2-C
W3/W4
RC24
W8
P1-LTR
Sandbox
```

同时成立。

---

## 15.1 Regression Set

至少：

### Context

```text
system stable
prefix stable
continuity survives compact
```

### Completion

```text
tool success
false-give-up regression
```

### Routing

```text
QA
Product
TaskControl
GoalMutation
```

### Approval

```text
TC-8
TC-9
RC29
hard redline
```

### Deadline

```text
task deadline < tool timeout
tool timeout < task deadline
cancel
no deadline
```

### Resume

```text
resume
continue
zero re-teaching
```

### Sandbox

```text
/dev/null
regular file
directory
mkdir
existing seccomp deny
```

---

# 16. Node 07 — Long-Run Acceptance

这是本总包非常重要的一项。

不要只做：

```text
sleep
```

必须做一个真正的：

```text
20–30+ step
```

Agent 任务。

至少包含：

```text
planning
tool execution
write
read
one controlled failure
recovery
verification
final artifact
```

任务必须有一个明确可验证产物。

---

## 16.1 Long-run 观察

记录：

```text
goal preservation
goal_revision
task_graph_revision
completed nodes
remaining nodes
tool invocation
approval
deadline
resource usage
verification
terminal
report
```

观察有没有：

```text
goal drift
false give_up
stall false positive
approval deadlock
deadline leak
resume break
```

---

## 16.2 第二个任务：非 Product 长任务

必须再跑一类：

```text
10+ turn discussion / reasoning
```

确认：

```text
不会进入错误 TaskGraph 循环
```

用来验证 W8。

---

# 17. Node 08 — Evidence & Telemetry Audit

对所有本阶段新增数据做一次口径检查。

特别检查：

```text
main agent
read-only agent
planner auxiliary
reflect
```

不能混进同一个统计桶。

---

## 17.1 Telemetry 必须标识

至少有：

```text
phase
depth / role（如果已实现）
session
```

如果当前 `depth/role` 尚未实现：

可以作为本 Node 的 telemetry-only 改动。

但：

**不能改变执行语义。**

---

# 18. Node 09 — Final Gate

统一执行：

```text
~/run_gate_r2c.sh
```

禁止：

```text
~/run_gate.sh
```

要求：

```text
fmt = 0
clippy = 0
RT4_SOLO = 0
test = 0
```

ignored 数必须解释。

---

# 19. Node 10 — VM / Release / Provenance

最终：

### .133

核实：

```text
binary version
source HEAD
test build
```

### .131

按：

```text
docs/vm-version-sync.md
```

同步。

要求：

```text
source provenance
binary provenance
marker
version
```

全部一致。

---

# 20. Node 11 — Documentation / Ledger

最终回填：

```text
docs/consolidated-remediation-ledger.md
```

必须更新：

```text
O-3
Q-3
OPEN-W8-1
DEV-2
verify_failed
N-3
```

同时：

```text
CHANGELOG
```

记录本总包实际发生的改变。

**禁止写成“所有问题已解决”。**

必须保留：

```text
CLOSED
OPEN
UNKNOWN
DEFERRED
```

---

# 21. 最终 Acceptance Matrix

最终必须明确：

| Domain | 必须达到 |
|---|---|
| Sandbox | G0 边界正确 |
| Seccomp | 新增 syscall 最小化 |
| Landlock | ABI/rights 语义可解释 |
| Task Routing | Product / QA / Control 正确 |
| Goal Revision | TaskControl 不污染 |
| Reflect | conflict 可观察 |
| Verify | verify_failed 可重现 |
| Projection | delegated 状态完整 |
| Context | R2-C 无回归 |
| Deadline | P1-LTR 无回归 |
| Resume | 连续性无回归 |
| Long-run | 20–30+ steps 至少一轮成功 |
| Regression | 全部 baseline 保持 |
| Provenance | 两 VM 一致 |

---

# 22. 长程执行最终汇报

**本任务整个执行期间不需要向用户发送 Node 级中间战报。**

最终只提交一份：

# `Hearth P1-CONSOLIDATION-01 Final Report`

必须包括：

### 1. Executive Summary

```text
PASS
PASS WITH DEVIATIONS
STOP
```

### 2. Commit Graph

Node → commit。

### 3. Implementation

每个实际修改点。

### 4. Test Evidence

```text
先红后绿
unit
integration
VM
real-machine
```

### 5. Long-run

至少一轮 20–30+ step 真任务。

### 6. Regression

```text
R2-C
W3/W4
RC24
W8
P1-LTR
Sandbox
```

### 7. Security

是否改变任何：

```text
seccomp
landlock
sandbox
approval
```

边界。

### 8. OPEN / UNKNOWN / DEFER

逐项列出。

### 9. VM Provenance

完整：

```text
host
source path
HEAD
binary path
version
gate log
```

### 10. Final Recommendation

只能是：

```text
PASS
PASS WITH DEVIATIONS
STOP
```

不得用：

```text
基本完成
大体正常
应该没问题
```

---

# 23. 自主执行原则

再次强调：

**这是一张完整施工总包，不是一个个要用户接力确认的任务。**

允许：

```text
Node 失败
→ 自己 debug
→ 修复
→ 测试
→ gate
→ 下一 Node
```

允许：

```text
发现低风险实现问题
→ 在当前 Node 范围内修复
→ 继续
```

禁止：

```text
看到新问题
→ 自动开一个完全无关的新项目
```

发现超出范围的问题：

```text
记录
标注
继续
```

只有触发：

```text
STOP-1 ~ STOP-7
```

才停止整个任务。

---

# 24. 最重要的最终原则

本总包不是为了把 backlog 清成 0。

真正目标是：

```text
正确识别任务
      ↓
进入正确控制流
      ↓
真实执行
      ↓
事实可信
      ↓
Reflect 有边界
      ↓
验证可信
      ↓
终态可信
      ↓
用户得到清晰结果
```

以及：

```text
安全边界
时间边界
任务边界
事实边界
模型判断边界
用户可见边界
```

都彼此清楚。

**执行开始后，除非触发 STOP CONDITION，不得等待顶层确认。**

**全部完成后一次性提交 Final Report。**

---

## 守门员复核与开工前约束（2026-08-30 05:20 · 与正文同效力，开工前生效）

> P1-LTR-01 复核：`b3229d5`/`e03565f` 在库，tag `v0.2.11` → `e03565f`；代码锚点全实锤（ToolContext 三字段 `task_deadline`/`effective_timeout`/`deadline_clamped` serde-skip + 类型化 `TaskDeadlineExceeded` + dispatcher 单点 min + deadline_clamped 判定归属 dispatcher.rs:13-28/245——修 2 以类型化错误达标，非字符串匹配）；**双 VM 系统 PATH = 0.2.11 实证**（上轮修 1 落地）；真机 T9=40.4s/T12=15.3s 精确、T14 cancellation 优先、T11 首跳 tool timeout 语义分离正确；T10 的 T4 停滞残余如实归属 DEV-2 家族。**P1-LTR-01 同意 PASS**，但有两处收尾欠账并入 Node 00（下修 1/2）。

### 修 1：Node 00 兼任 v0.2.11 的"单 log 四 RC 全 0"确认

P1-LTR 三份 gate log 实测：phase0=411 全绿 ✅、full=415 但 **CLIPPY_RC=101**（中间态）、final=415 但 **FMT_CHECK_RC=1**（自述"快照差已收敛 FMT_NOW_OK"**无落盘 log 佐证**）。即 **HEAD `e03565f` 至今没有一份四 RC 全 0 的落盘门禁**。裁决：Node 00 基线 gate 就是补这一票的机会——四 RC 全 0 落盘（`t_gate_consol_node00.log`）后，P1-LTR-01 门禁闭环才算完整；若 Node 00 出红，按 §4.1 自行处理，仍红则升级偏差报告。

### 修 2：/tmp 证据迁移（易失，重启即丢）

P1-LTR 真机证据 T9-T14 共 8 份日志在 `.133 /tmp/p1ltr_*.log`——**B04/B06 教训同款**（telemetry 只留 tail，事后无法复现归档）。Node 00 顺路拷入 `docs/data/p1ltr-20260830/` 并 commit。本总包所有 Node 的真机证据**当场归档 docs/data/<node>-<date>/**，禁留 /tmp。

### 修 3：O-3 加白必须证据驱动，且查全两个 syscall 号

`SYS_MKDIRAT(258)` 之外还有 **`SYS_mkdir(83)`**（x86_64）——glibc/coreutils 不同路径可能走不同号。加入前先取证（seccomp 拒绝日志 / strace）实际尝试了哪些号，**按证据加最小集**，禁按"缺 258"的单一假设加。§6.1 的"自然实现路径"判定 + 正/负/回归三测保持。

### 修 4：Node 03 的 REFLECT_FACT_CONFLICT 标记不得做成新 Event 变体

STOP-2 = "必须改变现有事件 schema"——若把 conflict 标记实现为新 Event 变体，Node 03 会自己撞上自己的 STOP 条件。钉死落法：**tracing 日志 + run report 的 summary 字段**（非终态事件、不进九态、不参与 terminal 判定），§10.2 的 observe-only 语义不变。执行窗口不要在这里停机请示——按本条落法即可。

### 修 5：版本 bump 时机前置 + 磁盘保险丝延续

- **Node 08 末尾做 0.2.12 bump + CHANGELOG，再跑 Node 09 final gate**——否则重演 P1-LTR final gate"gate 覆盖非最终态 → fmt=1"的欠账。Node 09 的 gate 必须覆盖最终提交状态。
- **每 Node 末 `df -h /home` 复查，<10G 中止**（57G 事故保险丝，P1-LTR 总包已立先例，本总包遗漏了——补上）。
- Node 10 provenance 明确**系统 PATH 口径**：双 VM `/usr/local/bin/hearth` = 0.2.12（源码树内二进制不算数）。

### 批准

P1-CONSOLIDATION-01 总包批准开工（含上述 5 修正）。Node 02 的 A-G 测试矩阵、Node 03 的 Q1-Q3 回答义务、Node 07 的 20-30 步长程任务（telemetry 开 + goal_drift observe 字段记录，Agnes 配额无忧）均为验收硬项。最终 Final Report 回来我终验。