收到《P1-CONSOLIDATION-01 Final Report v1》。

顶层验收结论：

# P1-CONSOLIDATION-01 = PASS WITH DEVIATIONS

本总包主体施工通过，不回滚，不重开已关闭的 R2-C / W3-W4 / RC24 / W8 / P1-LTR。

## 一、正式 CLOSED

以下确认通过：

* Node 01 Sandbox/Seccomp/Landlock 基础收敛
* Node 02 Task Type × Constraint 路由
* QA skip decompose
* Node 03/DEV-2 observe-only conflict marker
* Node 04 verify_failed fixture
* Node 05 delegation projection
* Cross-layer regression
* P1-LTR deadline
* Resume
* ContextBuilder
* RC24
* W8

特别认可本轮 `SYS_FCHDIR` 编号错位发现：

`FCHDIR 133→81`

导致错误 syscall 被放进 allowlist 的问题属于真实安全级发现；修复后净效果是 capability 收敛，不是能力扩张。

## 二、Node 07 不得简单结案为“Agnes flash 能力边界”

当前已知事实：

```text
Product long-run
→ controlled failure
→ repair actually succeeded
→ retest sequencing became incorrect
→ planner budget consumed
→ give_up
```

其中至少需要区分：

1. Reflect/model decision quality
2. TaskGraph progress / state semantics
3. planner budget / replan policy
4. verification/completion semantics

因此：

**DEV-2 保持 OPEN。**

禁止现在就写：

> “根因就是 Agnes flash 多步编辑能力不足。”

只有完整 trace 能排除 TaskGraph / budget / verification 机制后，才能将某部分责任归因于 provider/model。

## 三、O-4 正式提升为独立研究项

本轮出现：

```text
RESULT.txt
→
自述 TEST_PASSED
```

但实际测试：

```text
FAILED
```

这不是普通“模型说错话”。

它暴露的是：

> **Artifact Exists ≠ Artifact Semantically Correct**

正式登记：

`O-4 Semantic Completion Verification`

研究目标：

```text
Tool execution evidence
+
Artifact evidence
+
Verification evidence
+
Acceptance criteria
→
Completion
```

不得简单使用：

```text
artifact_exists == true
```

作为完成证据。

本轮不直接修改 completion 控制流。

## 四、下一阶段主线

下一总包正式命名：

# P1-TASK-TRUTH-01

目标：

> Task Fact → Verification → Reflect → Completion 的事实与权力边界收口。

完整长程执行，不采用节点间人工接力。

建议内部 Node：

```text
00 baseline/provenance
01 TaskGraph Fact Lifecycle
02 Evidence vs Verification
03 O-4 Semantic Completion
04 Reflect-Fact Conflict
05 Budget/Replan interaction
06 Failure Adaptation
07 20-30+ step Product long-run
08 10+ turn Discussion regression
09 full regression
10 final gate/release
```

Node 内部必须：

```text
inspect
→ implement
→ test
→ gate
→ self-accept
→ next
```

除非触发 STOP CONDITION，不得向顶层等待确认。

## 五、这轮额外需要修正的文档问题

当前 Final Report 中：

```text
.131 source = ~/codex
```

与 W1 已冻结的 provenance 规程冲突。

统一规则仍然是：

```text
.131 → ~/codex_t
.133 → ~/codex_t
```

`~/codex` 是其他窗口资源，禁止作为本项目施工树。

因此请在最终文档/ledger 中修正 provenance 表述。

另外修正 Executive Summary 中“六域/五项”的数量措辞，避免历史文档留下歧义。

## 六、Long-run 偏差不阻塞本轮验收

本轮将：

> P1-CONSOLIDATION = PASS WITH DEVIATIONS

而不是：

> FAILED

原因：

机制链已通过；
回归链已通过；
偏差集中在 long-run semantic completion / model decision interaction。

这正是下一阶段应处理的问题。

## 七、当前状态

```text
R2-C             ✅ CLOSED
W3/W4            ✅ CLOSED
RC24             ✅ CLOSED
W8               ✅ CLOSED
P1-LTR-01        ✅ CLOSED
P1-CONSOLIDATION ✅ PASS WITH DEVIATIONS

NEXT:
P1-TASK-TRUTH-01

OPEN:
DEV-2 Reflect decision quality
O-4 Semantic completion verification
W8-1 residual routing edge
Q-3 kernel/landlock ABI
```

本总包停止，不继续追加修复。

下一步只创建：

`docs/hearth-p1-task-truth-01-construction-order-v1.md`

**先出完整长程施工单，不施工；施工单形成后停在顶层批准。**

---

## 守门员复核与 P1-TASK-TRUTH-01 前置约束（2026-08-30 07:50 · 与正文同效力）

> 复核记录：七 commit 链在库、tag `v0.2.12` → `7ad56cc`；**三份 gate log 全部实测四 RC=0**（node00=415 补票 ✅ / node02=416 / final=417——修 5"版本 bump 前置于 final gate"落地，上轮 fmt=1 欠账闭环）；**FCHDIR 修正实证**（lib.rs:547 `SYS_FCHDIR=81`，注释自证 133=mknod 误放行"可与 writable 目录组合"——真实安全级发现，x86_64 号表核对无误）；REFLECT_FACT_CONFLICT 按修 4 落法（scratch + report summary，非 Event 变体，loop.rs:3628/4106）；证据归档 `docs/data/n1sbx-o1-20260830/` + `p1ltr-20260830/` 在库；双 VM 系统 PATH = 0.2.12 实证。**PASS WITH DEVIATIONS 同意**；O-4 立项同意；"禁止现在归因 Agnes flash"的克制正确——Node 07 的 29/42 步样本里"机制链无故障 + 修复实际成功但复测时序混乱"恰好说明嫌疑分布在 TaskGraph 状态语义 / replan 预算 / 验证语义多处，单怪模型是偷懒。

### 裁决 1（重要）：provenance 表述修正不得照字面执行——改为"分窗登记"

§五要求统一 `.131 → ~/codex_t`——**与 .131 实测现实冲突**（我于 07:34 实测）：

```text
.131:~/codex    = version 0.2.12（现役施工树——0.2.12 二进制就是从这构建的）
.131:~/codex_t  = version 0.2.9（陈旧树！R2-C 时代遗留）
.133:~/codex_t  = 评审树（现行约定正确）；.133 的 ~/codex = 他窗口资源（run_gate.sh 串台事故来源）
```

"~/codex 禁用"是 **.133 侧**的规则（他窗口资源），被错误泛化到 .131——.131 的 ~/codex 是**执行窗口自有**施工树。若照字面改文档 = provenance 记录失真；照字面迁移 = 无谓的高风险搬家。**裁决：per-host 分窗登记**（写死进 `vm-version-sync.md` + P1-TASK-TRUTH-01 Node 00）：

```text
.131（执行 VM）：施工树 = ~/codex（执行窗口自有）——评审/测试窗口禁写
.133（评审 VM）：验证树 = ~/codex_t——~/codex 零触碰
.131 的 ~/codex_t（0.2.9 陈旧树）：标 DEPRECATED 防误用（处置去留由用户拍板，不擅自删）
```

Final Report §9 的 provenance 表按此口径勘误（".131 source = ~/codex（自有施工树，分窗登记）"），不是改成 ~/codex_t。"六域/五项"措辞修正照做。

### P1-TASK-TRUTH-01 设计单前置约束（Node 划分同意，另加 5 条）

1. **O-4 证据分层原则（Node 02/03 核心）**：completion 证据 = **工具层结构化输出**（exit code / test runner 结构化解析——W4/RC20 纪律），artifact 自述（RESULT.txt 写 TEST_PASSED）**不作通过证据**；但同时不得升格为"LLM 语义理解验证"（那是 DEV-2）。验收绑定向 R2-B 既有 `acceptance_criteria` 承载结构对齐，勿新建第二套 criteria 系统。
2. **Node 05 必须吃 Node 07 的两个失败样本**（29 步/42 步："修复实际成功 → 复测时序混乱 → budget-low give_up"）作设计输入——核心问题：**修复后的验证重试有没有预算保障**？replan policy 与 budget/deadline 的交互矩阵要画出来。
3. **Node 07 复跑口径预先写死**：同任务同 prompt 对照 CONSOLIDATION 基线（29/42 步 failed），improvement 判定标准先定后跑，禁事后挑口径。
4. **延续纪律**（历轮已立，全包有效）：版本 bump 前置于 final gate、每 Node `df -h` 保险丝（<10G 中止）、证据当场归档禁 /tmp、gate log 完整路径登记、provenance 系统 PATH 口径、分窗登记（裁决 1）。
5. **Reflect 边界延续**：Node 04（Reflect-Fact Conflict）仍 observe-only；任何 "Fact overrides Reflect" / "Reflect overrides Fact" 控制流变更 = STOP，交顶层。
