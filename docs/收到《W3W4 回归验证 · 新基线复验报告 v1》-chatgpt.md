收到《W3/W4 回归验证 · 新基线复验报告 v1》。

顶层复核结论：

# W3/W4 新基线复验正式通过，W3/W4 CLOSED。

RC24 + W8 + Goal Revision 接入后重新验证，原 W3/W4 核心能力没有发生回归。

## 一、四项证明裁决

### ① 工具成功不因 node.status 错误 give_up

**PASS**

十论述任务 10/10 completed；
T-A 三步任务 completed。

旧的：
tool success + artifact exists + give_up
机制性问题已被实证消除。

### ② 纯读任务不被判定为“不工作”

**PASS**

T-B 纯读任务 completed。

W8 的任务路由已经证明：
纯读/问询不再强制进入原 TaskGraph stall/replan 路径。

### ③ 六终态投影

**PASS with evidence gap**

当前已有：

* completed
* failed
* give_up/stalled
* timeout
* cancelled

自然真机证据。

`verify_failed` 尚未完成自然触发构造，但机制层已有测试覆盖。

因此不要写“六终态全部实机闭合”，准确表述为：

> **5/6 natural runtime evidence + verify_failed mechanism test covered。**

verify_failed 补证不阻塞 W3/W4 Closure。

### ④ resume continuity

**FULL PASS**

3/3 resume：

* 正确复述进度
* 增量产物
* 0 次重教
* A2 “继续”分类不会污染 goal_revision
* 无 GoalChanged
* 无 stalled / 误 give_up

该项正式 CLOSED。

---

# 二、W3/W4 正式归档

W3/W4 到此不再追加施工。

当前结论：

```text
事实生产       ✅
Completion     ✅
结构化错误      ✅
终态投影       ✅
Resume         ✅
Regression     ✅
```

不得因为后续出现 O-1/O-2 就重新打开 W3/W4。

---

# 三、O-1 转独立 P1-small

当前：

`budget_reassess`

interaction branch 未做 `is_terminal` 保护，存在 task 已经 terminal 后继续等待 stdin 的风险。

与 RC24 approval interaction 类似，属于 interaction lifecycle completeness。

处理方式：

* 独立小修；
* 不修改 Budget 语义；
* 不修改 ApprovalPolicy；
* 只补 terminal guard；
* 单独 commit + gate；
* 真机 headless 验证。

本轮不直接施工。

---

# 四、O-2 提升为 P1 Long-running Reliability

当前实证：

```text
task deadline = 15s
tool = sleep 300s
tool 实际继续执行
外部窗口 ≈360s 才终止
```

这证明当前 deadline 是：

> **Loop-level deadline**

而不是：

> **Tool execution deadline**

这是可信委托场景的重要缺口。

正式记录：

`P1-LTR-01 Tool-level Deadline Enforcement`

但：

**本轮不要直接修改。**

下一单单独研究：

```text
Task remaining deadline
        ↓
Tool declared timeout
        ↓
effective tool timeout
=
min(tool timeout, remaining task deadline)
```

需要同时考虑：

* tool-runtime
* dispatcher timeout
* sandbox process lifetime
* subprocess kill
* cancellation
* cleanup
* terminal reason

不要只在 loop 层加一个 if。

---

# 五、W8-1 保持 OPEN

当前已经有新的明确边界：

```text
Task Type
+
Constraints
```

不能让：

> “不要修改现有文件”

这种否定约束单独把真实 Product Task 路由成 QA。

例如：

```text
“写 README，但不要修改现有文件”
=
Product Task
+
Negative Constraint
```

不要继续靠无止境增加关键词解决。

W8-1 留给后续任务路由设计。

---

# 六、DEV-2 保持独立

Reflect LLM：

```text
facts correct
+
tool success
+
artifact exists
↓
仍可能 give_up
```

继续保持独立 OPEN。

不要通过继续膨胀 ContextBuilder 来处理。

下一阶段单独研究：

> Fact / Verification / Reflect 冲突时的决策优先级。

本轮不改。

---

# 七、verify_failed 只做补证，不重开架构

可以在后续测试窗口构造一个最小 verify_failed fixture。

目标只是：

```text
verification fails
↓
terminal = verify_failed
↓
CLI/report projection correct
```

不要为了构造它修改 completion/verification 逻辑。

---

# 八、版本同步

当前 `.131` 尚未包含 RC24 + W8。

下一次正式发布/部署窗口必须按 `vm-version-sync.md`：

```text
source provenance
+
binary provenance
```

双重核实。

当前不为了此项重新打开 W3/W4。

---

# 九、当前总状态

```text
R2-C ContextBuilder      ✅ CLOSED
W3/W4                    ✅ CLOSED
RC24                     ✅ CLOSED
W8                       ✅ CLOSED

NEXT:
P1-LTR-01 Tool Deadline
N-1 Sandbox Landlock
O-1 Budget Interaction
W8-1 Task Type/Constraint Routing

OPEN:
DEV-2 Reflect Quality
verify_failed fixture
N-3 report projection
Q-2 telemetry depth/role

DEFER:
Bridge
Subagent
TUI
MCP
```

本轮停止。

下一步不要继续修 W3/W4。

如需进入下一施工窗口，优先为：

**P1-LTR-01 Tool-level Deadline Enforcement**

或

**N1-SBX Landlock File Rights**

各自单独出施工单。

---

## 守门员复核与派工裁决（2026-08-30 01:20 · 与正文同效力）

> 复核记录：零 diff 声明属实（`900d199` 后三 commit 全为 docs/证据：`e9c8a32`/`0100b28`/`e096b39`）；证据归档 15 文件 + `docs/data/n1-landlock/ll_diag.c` 在库（上轮增 4 落地）；O-1 锚点钉死——`loop.rs:3835-3860` budget_reassess 交互分支 blocking `check_interaction` 前确无 terminal guard（全库 `is_terminal` 零代码命中）；N1-SBX 施工单 v1 我方 8 条补充**全部落地**（条款 1 同族隐患/条款 2 REFER 归档/条款 3 动态掩出+覆盖性断言/条款 4 fstat 单点/条款 6 C1 内核语义锚/条款 7 两层/条款 8 fail-closed/验收后事项含 T6 勘误与 gate log 登记）。

### 裁决 1：N1-SBX 施工单 v1 批准施工（本守门员侧无保留）

条款 10 的"权限不放大"论证成立（原状态 = 规则不存在 + 全拒；新状态 = 规则存在 + 四位放行，均在 landlock 设计语义内）。执行窗口按施工单开工，验收门禁照条款全项。

### 裁决 2：O-1 排 N1-SBX 同窗口第二位（独立 commit + gate）

一行级 terminal guard（`budget_exhausted` 分支阻塞等待前查 is_terminal）+ headless 复验（对照 V2R2 卡 200s）。锚点已钉（`loop.rs:3835-3860`）。N1-SBX <80 行 + O-1 一行级，一个窗口两单三 commit，不摊大饼。guard 语义照抄 RC24-B 的 approval 分支先例，禁改 Budget 数值语义。

### 裁决 3：P1-LTR-01 设计单排第三，设计必答三问

ChatGPT 的 `min(tool timeout, remaining task deadline)` 公式方向正确。设计单（停设计待批准）另须回答：

1. **落点层**：H1 的 `declared_timeout` 通道（v0.2.4 已接 dispatcher→bash）是现成传参路径——effective timeout 应沿它下发，禁在 loop 层另起 if；
2. **剩余量传递机制**：跨层传参 vs 共享截止时刻（Deadline 即时刻戳，天然免疫逐步传参漂移）；
3. **terminal reason 归属**：工具被任务剩余期截断时，终态应为 task 侧 `deadline_exceeded`（复用 H2 收尾路径）而非 tool timeout——投影语义要钉死，防 V-3 矩阵两类原因混淆。

### 观察项（observe-only，本单禁动）

**Q-3（新，QUARANTINE）**：kernel 7.0.0-30 的 landlock ABI ≥ 5。若内核已提供更晚 ABI 的 FS 位（如 IOCTL_DEV 一族）而 ruleset handled 集未声明 → 未 handled = 不限制（放行）。这是**既有缺口非 N1-SBX 引入**；往 handled 集加位 = G0 行为变更 = D 类，单独立项评估，本单禁顺手加。

### 版本资产提醒（挂账两次了）

RC24/W8 合入后版本号未动、CHANGELOG 欠账（v0.2.5–v0.2.9 无 CHANGELOG）仍挂着。N1-SBX + O-1 合入后：升 0.2.10 + 补 CHANGELOG + 打 tag（发版节点，方案 B）——别让"资产化"继续排不上队。
