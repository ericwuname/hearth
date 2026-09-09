# Hearth P1-LTR-01 长程施工总包 v1
## Tool-level Deadline Enforcement + Long-running Execution Reliability

**性质**：完整长程施工任务书  
**执行方式**：GLM 5.3 全权连续执行，**不按单节点向用户请求反馈**  
**目标**：完成本施工包内全部节点后，一次性提交总战报  
**当前基线**：v0.2.10 stable / N1-SBX + O-1 已完成  
**依据**：`docs/hearth-p1-ltr-01-tool-deadline-construction-order-v1.md`

---

# 0. 执行模式：本任务书采用“内部检查点、外部一次汇报”

这是一个长程施工包，不采用：

```text
做一个节点
→ 停止
→ 向用户汇报
→ 等用户再次授权
```

而采用：

```text
Phase 0
↓
自测
↓
通过
↓
Phase 1
↓
自测
↓
通过
↓
Phase 2
↓
……
↓
最终验收
↓
一次性战报
```

**每个 Phase 必须内部完成：**

```text
实现
→ fmt
→ clippy
→ 单测
→ 必要的局部验证
→ 通过后进入下一 Phase
```

如果一个 Phase 失败：

1. 首先自行定位；
2. 在本施工单范围内修复；
3. 重新测试；
4. 通过后继续。

**不要因为普通测试失败而立即停止并向用户汇报。**

只有出现以下情况之一，才允许中止整个长程执行并提交中止报告：

- 需要修改本任务书明确禁止修改的架构；
- 发现必须改变 Budget 数值语义；
- 发现必须改变 ApprovalPolicy；
- 需要修改 N1-SBX 已定格的 sandbox 权限语义；
- 需要修改 seccomp allowlist；
- 需要新增事件契约；
- 发现当前设计中的关键前提在源码中不成立；
- 需要用户提供新的秘密、权限或无法自行获得的外部资源；
- VM/环境发生与本任务无关、且无法安全绕过的基础设施阻塞。

除此之外，不要人为拆成多个用户反馈周期。

---

# 1. 本施工包最终目标

把当前：

```text
Task deadline
    ↓
loop 层检查
    ↓
进入 tool
    ↓
tool 可以继续运行很久
```

升级为：

```text
Task deadline
    ↓
共享 absolute deadline
    ↓
dispatcher 单点计算 effective timeout
    ↓
tool-runtime
    ↓
sandbox / subprocess
    ↓
真正按剩余任务时间终止
```

最终必须保证：

> **Task deadline 是整个任务执行链的硬截止，而不是只有 loop 才知道的软提示。**

当前现场已经证明：

```text
task deadline = 15s
tool = sleep 300s
```

工具仍可继续执行到外部窗口约 360s 才结束。当前 deadline 因此只是 loop-level 而非 tool-level。

---

# 2. 核心设计冻结

严格采用：

```text
effective_timeout
=
min(tool_declared_timeout, remaining_task_time)
```

其中：

```text
remaining_task_time
=
absolute_task_deadline - now
```

### 2.1 Deadline 表示

新增：

```rust
ToolContext.task_deadline:
    Option<std::time::Instant>
```

使用 absolute deadline，不传递“剩余秒数”。

原因：

```text
剩余秒数
→
层层传递
→
排队/调度损耗
→
逐层漂移
```

而：

```text
Instant deadline
```

可以由每层就地计算剩余时间。

该设计已经在 P1-LTR-01 设计单中批准。

---

# 3. 范围红线

本施工包允许修改：

```text
tool-runtime context
dispatcher timeout calculation
tools-builtin/bash timeout consumption
agent-core deadline injection
deadline-related tests
```

禁止：

```text
Budget 数值语义
ApprovalPolicy
Approval 判定
N1-SBX Landlock 权限
seccomp allowlist
RT4
ContextBuilder
TaskGraph 事实模型
Goal Revision
Reflect decision
事件 schema
terminal 九态定义
```

尤其：

**不要在 agent-core loop 里再写一个新的 deadline if。**

deadline 应通过既有：

```text
declared_timeout
dispatcher
ToolContext
sandbox spawn duration
```

通道下发。

---

# 4. Phase 0 — 开工前基线与 blocker preflight

先不要改代码。

执行：

```text
git status
git rev-parse HEAD
cargo version
VM binary --version
```

确认当前工作树。

确认：

```text
~/run_gate_r2c.sh
```

仍然是唯一允许使用的隔离门禁脚本。

运行一次开工前门禁并记录：

```text
gate log path
fmt
clippy
RT4_SOLO
test count
```

---

## 4.1 O-3 blocker probe

当前已知新发现：

> seccomp allowlist 缺 `SYS_MKDIRAT(258)`，导致 `mkdir` 等命令 SIGSYS/exit 159。

这属于独立 G0 问题，本施工包**不修**。

只需确认 P1-LTR 测试是否依赖：

```text
mkdir
mkdirat
目录创建
```

如果不依赖：

> 继续。

如果某个测试被 O-3 阻塞：

> 换用不依赖 mkdir 的等价测试方法。

**不得顺手修 seccomp。**

---

# 5. Phase 1 — ToolContext Deadline 数据通道

实现：

```rust
ToolContext {
    task_deadline: Option<Instant>
}
```

要求：

1. 默认 `None`；
2. serde / persistence 不需要记录 `Instant`；
3. 不影响 service/test 等没有 task deadline 的路径；
4. `None` 时现有行为完全不变；
5. 不建立第二套 deadline 类型。

### 自测

实现纯函数级或最小 integration test，证明：

```text
None
→
原有 timeout 行为

Some(deadline)
→
能够正确得到 remaining duration
```

通过后进入 Phase 2。

---

# 6. Phase 2 — AgentLoop 注入 absolute deadline

在现有 H2 run deadline 生命周期中：

```text
run() 开始
↓
计算 absolute task deadline
↓
AgentLoop 持有
↓
每次 dispatch 注入 ToolContext.task_deadline
```

要求：

### 6.1 单 run

```text
run()
=
一个 deadline
```

### 6.2 REPL continue

每次新的：

```text
continue_turn
```

重新建立本轮 task deadline。

不得把上一轮 deadline 错误带入下一轮。

设计依据已经明确：REPL 每轮 continue_turn 都重新计算 deadline。

### 6.3 不得持久化 deadline

resume 时重新建立当前 run deadline。

不要把：

```text
Instant
```

写进 session/taskgoal 文件。

---

# 7. Phase 3 — Dispatcher 单点 effective timeout

这是本任务核心。

在 dispatcher 真正执行工具之前：

```text
declared_timeout
+
task_deadline
↓
remaining
↓
effective =
min(declared_timeout, remaining)
```

必须只有一个权威计算点。

禁止：

```text
loop 自己算一遍
dispatcher 再算一遍
bash 再算一遍
```

否则以后必然漂移。

---

## 7.1 保留已有 declared timeout 语义

当前声明路径：

```text
未声明
→
dispatcher 默认 timeout

声明
→
使用 declared timeout
```

保持不变。

任务 deadline 只是进一步收紧。

即：

```text
effective
=
min(declared_timeout, remaining_task_time)
```

---

## 7.2 600s tool cap 不得消失

原有 bash hard cap：

```text
600s
```

必须继续存在。

最终关系应至少满足：

```text
effective
<= declared/cap
<= 600s
```

不要因为 task deadline 引入而意外解除工具自己的最大安全上限。

原设计明确要求保持 600s cap。

---

# 8. Phase 4 — Tool execution path 真正消费 effective timeout

将 effective timeout 沿现有调用链传到：

```text
dispatcher
↓
tool runtime
↓
tools-builtin/bash
↓
sandbox spawn
↓
subprocess lifetime
```

不要创建新的 timeout mechanism。

当前既有 bash：

```text
args.timeout_secs
↓
sandbox.spawn(Duration)
```

应直接消费 effective timeout。

现有 sandbox spawn 已经支持 Duration，不需要重建 sandbox execution model。

---

# 9. Phase 5 — Task deadline 与 tool timeout 的语义分离

这是验收重点。

必须明确：

## 情况 A：Task deadline 更早

```text
task remaining = 30s
declared tool timeout = 600s
```

则：

```text
effective = 30s
```

如果工具被终止：

```text
terminal reason
=
deadline_exceeded
```

而不是普通 tool timeout。

---

## 情况 B：Tool timeout 更早

```text
task remaining = 30s
declared timeout = 10s
```

则：

```text
effective = 10s
```

工具超时：

```text
tool timeout
```

任务本身不应被错误标记：

```text
deadline_exceeded
```

P1-LTR-01 已明确要求两种原因分离。

---

## 情况 C：没有 task deadline

```text
task_deadline = None
```

完全保持原有行为。

---

# 10. Phase 6 — Deadline 已耗尽时禁止无意义启动工具

增加边界语义：

如果 dispatcher 准备执行工具时：

```text
remaining <= 0
```

不要再真正启动工具进程。

应通过现有错误/收尾语义立即形成：

```text
deadline_exceeded
```

不要启动一个：

```text
timeout = 0
```

的假工具调用再等待其失败。

这个行为属于本施工包授权范围。

---

# 11. Phase 7 — Process termination / cleanup 验证

不得新建 cleanup 系统。

复用已有：

```text
SIGKILL
reap
cgroup cleanup
```

以及已有 X1 cleanup/reap 路径。

施工单已经明确当前既有 cleanup 可复用。

必须验证：

```text
tool timeout
↓
process actually terminated
↓
no lingering child process
↓
dispatcher returns
↓
loop can continue/finalize
```

至少做一个轻量子进程生命周期验证。

不要把测试做成新的 process manager。

---

# 12. Phase 8 — Cancellation 优先级

现有 Ctrl-C cancellation 不修改。

必须保证：

```text
Ctrl-C
+
deadline
```

同时接近触发时：

> cancellation 优先于 deadline。

也就是说：

```text
用户明确取消
→
cancelled
```

不能被误判成：

```text
deadline_exceeded
```

设计单已经把 cancellation 与 deadline 定义为两条独立路径。

---

# 13. Phase 9 — Terminal / error 语义接入

不得新建终态。

继续复用：

```text
deadline_exceeded
```

和现有 terminal normalization。

需要保证：

```text
task deadline cutoff
→
deadline_exceeded
→
已有 terminal normalize
→
CLI/report 正确显示
```

而：

```text
tool timeout
→
结构化工具错误
→
W4 投影
```

两者不混淆。

---

# 14. Phase 10 — 单元测试：先红后绿

新增测试至少包括：

### T1 effective timeout

```text
declared=600
remaining=30
→ 30

declared=10
remaining=30
→ 10

deadline=None
→ declared
```

---

### T2 no-deadline regression

```text
task_deadline=None
```

确保所有旧行为不变。

---

### T3 continue-turn reset

证明：

```text
run A deadline
≠
run B deadline
```

不得跨轮复用。

---

### T4 deadline already expired

```text
remaining <= 0
```

证明：

```text
tool 不启动
→ deadline_exceeded
```

---

### T5 tool timeout beats task deadline when earlier

```text
task remaining=30
declared=10
→ tool timeout
```

---

### T6 task deadline beats tool timeout when earlier

```text
task remaining=30
declared=600
→ deadline_exceeded
```

---

### T7 cancellation precedence

```text
cancel
+
near deadline
→ cancelled
```

---

### T8 process cleanup

工具因 effective timeout 终止后：

```text
process reaped
```

不得出现 lingering child。

---

# 15. Phase 11 — 真机测试矩阵

VM：

```text
.133
Ubuntu 24.04
kernel 7.0.0-30
HEARTH_ALLOW_NO_CGROUP=1
Agnes
```

### T9

```bash
HEARTH_TASK_TIMEOUT_SECS=30
sleep 300
```

预期：

```text
≈30s
deadline_exceeded
```

不是：

```text
300s / 360s
```

这是本任务最核心的验收。

---

### T10

```bash
HEARTH_TASK_TIMEOUT_SECS=30
sleep 10
```

预期：

```text
completed
```

---

### T11

```text
task deadline=30
tool timeout=10
sleep 60
```

预期：

```text
tool timeout
```

不是：

```text
deadline_exceeded
```

---

### T12

```text
task deadline=15
sleep 60
```

预期：

```text
deadline_exceeded
```

---

### T13

执行：

```text
无 deadline
```

的旧测试任务。

预期：

```text
行为与 v0.2.10 基线一致
```

---

### T14

Ctrl-C 与 deadline 接近时：

预期：

```text
⏹ cancelled
```

而不是：

```text
deadline_exceeded
```

---

# 16. Phase 12 — 长程任务回归

不要只测试 `sleep`。

至少再做一个真正 Agent 任务：

```text
多步任务
+
至少 1 个长工具调用
+
deadline 足够完成
```

证明：

```text
TaskGraph
+
Tool runtime
+
deadline
+
Completion
```

可以共同工作。

---

# 17. Phase 13 — 负面验收

必须证明以下情况不会发生：

```text
task deadline=30
tool timeout=600
→
工具运行 >30s
```

必须失败。

```text
task deadline=30
tool timeout=10
→
10s 后 tool timeout
```

必须成立。

```text
deadline=None
→
所有旧测试行为变化
```

不能发生。

```text
Ctrl-C
→
deadline_exceeded
```

不能发生。

```text
deadline expired
→
仍启动新 tool
```

不能发生。

---

# 18. Phase 14 — 全量门禁

使用唯一门禁：

```text
~/run_gate_r2c.sh
```

禁止：

```text
~/run_gate.sh
```

门禁必须：

```text
fmt = 0
clippy = 0
RT4_SOLO = 0
test = 0
```

并记录：

```text
完整 host
完整 log path
测试计数
```

不得只写：

```text
all green
```

---

# 19. Phase 15 — Regression / provenance

完成后：

1. 当前 release version bump；
2. CHANGELOG 增加 P1-LTR-01；
3. 新 release binary；
4. `.133` 更新；
5. 最后再按 `vm-version-sync.md` 同步 `.131`；
6. 双 VM 做：
   - binary version
   - source provenance
   - marker / HEAD
   三查。

注意：

**不要在施工中途同步 `.131`。**

等整个 P1-LTR-01 完成并形成稳定 release，再一次同步。

---

# 20. 不要处理 O-3

当前已知：

```text
seccomp allowlist
缺 SYS_MKDIRAT(258)
```

本施工包：

```text
只记录
不修复
不修改 seccomp
```

如果 P1-LTR 测试被 `mkdir` / `mkdirat` 阻塞，改用不依赖目录创建的测试方式。

O-3 独立开：

```text
G0-SBX/SECCOMP
```

施工单。

---

# 21. 不要借本任务顺手处理以下 OPEN

保持：

```text
DEV-2 Reflect quality
OPEN-W8-1 Task Type + Constraints
verify_failed fixture
N-3 report projection
O-3 seccomp
Q-3 Landlock ABI
```

不要因为在测试中碰到它们就顺手修。

只记录：

```text
observed
impact
evidence
```

然后继续 P1-LTR。

---

# 22. 偏差处理机制

如果施工中发现设计与源码实际不完全一致：

不要立刻重写架构。

先：

```text
记录偏差
↓
判断是否可在现有设计范围内修正
```

如果可以：

> 修正并继续。

如果必须改变：

```text
Budget
Approval
Sandbox
seccomp
Event contract
Task model
```

则：

> 停止整个施工包，输出 deviation report，等待顶层裁决。

---

# 23. 最终验收标准

全部满足后，才能宣布：

```text
P1-LTR-01 = PASS
```

必须同时满足：

```text
① effective timeout 单测通过
② no-deadline regression 通过
③ continue-turn deadline reset 通过
④ task deadline 比 tool timeout 更早时：
   deadline_exceeded
⑤ tool timeout 比 task deadline 更早时：
   tool timeout
⑥ expired deadline 不再启动工具
⑦ cancellation precedence 正确
⑧ process cleanup 正确
⑨ 真机 sleep 300 在 30s deadline 左右终止
⑩ 真机正常短工具不被误杀
⑪ 长程真实任务可以正常完成
⑫ 全量 VM gate 全绿
⑬ release binary 与 source provenance 一致
⑭ .131 / .133 最终稳定版本同步
```

---

# 24. 最终交付战报格式

全部施工完成后，一次性提交：

## A. Commit

```text
commit chain
最终 HEAD
version
tag
```

## B. Implementation

```text
修改文件
核心逻辑
effective timeout 计算点
deadline 传播路径
cleanup 路径
```

## C. Tests

```text
先红后绿证据
unit
integration
negative
VM
```

## D. Real-machine

至少报告：

```text
sleep 300 / deadline 30
sleep 10 / deadline 30
tool timeout < deadline
deadline < tool timeout
cancel
long-running task
```

分别给：

```text
实际耗时
terminal state
reason
```

## E. Gate

完整：

```text
VM host
gate script
log path
fmt/clippy/RT4/test
passed/failed/ignored
```

## F. Regression

证明：

```text
N1-SBX
O-1
R2-C
W3/W4
RC24
W8
```

均无回归。

## G. Deviations

只列：

```text
OPEN
UNKNOWN
DEFER
新发现
```

## H. Final

明确：

```text
PASS
PARTIAL
BLOCKED
```

不得用含糊的：

```text
基本完成
大体正常
应该没问题
```

---

# 25. 最终执行纪律

本任务最重要的一条不是代码，而是：

> **不要把已经完成的长程施工重新拆成人类接力。**

你有权在 GLM 内部使用：

```text
Phase Gate
Self-test
Local rollback
Self-debug
VM gate
```

但这些都属于：

> **GLM 内部执行循环。**

不是：

> **用户交互循环。**

除非触发第 22 节的真正架构偏差，否则完成一个 Phase 后自动进入下一个 Phase。

最终只提交一次完整战报。

**开始执行。**

---

## 守门员复核与开工前修正（2026-08-30 02:55 · 与正文同效力，开工前生效）

> 复核记录：四 commit（`f8f549a`/`1d5bd86`/`c8b6532`/`fb93f48`）在库；tag `v0.2.10`（annotated → `fb93f48`，HEAD 祖先链上）；Cargo 0.2.10 + CHANGELOG-v0.2.10.md 在库；N1-SBX 锚点实锤（`FILE_MASK`/`DIR_ONLY_MASK`/`FS_FILE_ONLY`/`FS_RO_FILE` + 编译期覆盖性断言 lib.rs:273-298）；O-1 guard 实锤（run_local.rs:864 + clarification :303/:920）；**gate `.133:/home/wutao/t_gate_n1sbx_o1.log` = 411/0 实证**。O-1 落点偏差（loop→CLI 消费端）**追认**：与 RC24-B 同构、语义一致。P1-LTR-01 设计稿三问合格，批准。总包结构（Phase Gate 自主循环 + 停机条件 + 偏差上报）批准。以下 5 条为开工前修正。

### 修 1（必须）：binary provenance 在系统 PATH 层不成立——Phase 0 加安装步骤

实测（02:47）：**双 VM 系统 PATH 的 hearth 都是 0.2.9**——

```text
.131: ~/codex/target/release/hearth = hearth 0.2.10 ✅（但 /usr/local/bin/hearth = 0.2.9，8月29 17:40 旧）
.133: ~/codex_t/target/release/hearth = hearth 0.2.9 串（gate 构建早于版本号 bump，代码含 N1-SBX/O-1）；/usr/local/bin = 0.2.9
```

执行报告 provenance 表"双 VM 0.2.10 实证"在系统 PATH 层**不成立**——这是 v0.2.3 PATH 分叉家族复发（该家族 08-27 刚闭环过一次）。修正：**Phase 0 preflight 增加"从 tag v0.2.10 构建 → 安装到 /usr/local/bin（双 VM）→ 系统 PATH `hearth --version` = 0.2.10 实证"**，通过后才进 Phase 1；最终战报 provenance 表按**系统 PATH 口径**重写（源码树内二进制不算数）。

### 修 2（必须）：terminal reason 分类禁用 stderr 字符串标记——设计稿问 3 实现方式修正

设计稿问 3 的实现方式（"工具截断错误输出携带 stderr 前缀 `[task-deadline]` → loop 层 error 分类消费"）是 **RC20 家族反模式**——按工具输出字符串分类，正是"中文错误画绿勾"的根因形态。正确做法：

```text
dispatcher 单点计算 effective 时已知 clamp 来源（remaining < declared）
→ 截断信息走结构化字段（ToolTimeout | TaskDeadlineClamped）
→ bash/sandbox 的 timeout 错误结构携带该字段（W4 结构化错误通道现成）
→ loop 消费结构化字段判定 deadline_exceeded vs tool timeout
```

禁任何"错误文本含某子串"式判定。总包 §13 的语义分离目标不变，实现路径按本条修正。

### 修 3：N1-SBX/O-1 真机证据补归档

gate log 已核（411），但 TC8_EXEC_OK 运行日志、零 EINVAL 的 grep 输出、O-1 V2R2 前后对照（卡 200s → 秒级收口）**未入 docs/data/**（对比 regv/w8 各有归档目录）。Phase 0 顺路补 `docs/data/n1sbx-o1-20260830/` 归档（证据资产化纪律，历轮惯例）。

### 修 4：Phase 0 加 VM 磁盘余量检查（57G 事故保险丝）

长程自主执行模式 = 无人值守。Phase 0 preflight 增加 `df -h /home` 基线记录，**每 Phase 末复查一次**；余量 <10G → 中止报告（§0 停机条件新增一条）。57G 写盘事故的教训是"磁盘死之前没人知道"。

### 修 5：T13 简化 + O-3 优先级确认

- **T13**（无 deadline 旧行为回归）：不需要单独真机构造——全量 gate 的 400+ 测试绝大多数无 deadline 路径，gate 全绿即 T13 证据，战报写明口径即可。
- **O-3 优先级确认**：mkdir 在沙箱内全灭 = agent"建目录"类任务全废，真实伤害面大于 DEV-2/W8-1。P1-LTR-01 验收后 **O-3 独立单（G0-SBX/SECCOMP）直接顶上**，排在 DEV-2 证据窗之前。

### 版本资产（Phase 15 确认）

0.2.10 → 0.2.11 + CHANGELOG + tag + 双 VM 同步（含本次修正的 /usr/local/bin 安装步骤）——CHANGELOG 惯例自 v0.2.10 恢复，保持每版不断档。