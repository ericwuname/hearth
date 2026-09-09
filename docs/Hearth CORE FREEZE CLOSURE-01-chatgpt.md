# Hearth CORE FREEZE CLOSURE-01

## Core 冻结闭环与 F1 裁决任务书 v1.1

**（v1.1：砺批-0~8 内联 + 守门员复核定稿——批-1/批-2 经本窗实测升级为实锤条款）**

> **执行对象：GLM / 砺·执行**
>
> **当前基线：v0.2.18**
>
> **任务性质：冻结审计 / 裁决准备 / Gate，不是功能开发**
>
> **核心原则：本任务默认禁止扩大 Core，不以“修完所有 OPEN”为成功标准。**
>
> **上位输入：**
>
> * `Hearth Core 长程可靠性整合报告 v2`
> * FA01 / P2-MC / P2-LR / CFR 四包 Final Report
> * 对应 review package
> * `docs/data/` 全部已冻结证据
>
> **目标：**
> 在不继续扩大 Core 架构、不引入第二套事实模型、不修改现有 Authority 边界的前提下，完成一次最终 Freeze Closure Audit，并将剩余 F1 项逐项形成“接受 / 后续单独立项 / 阻塞 Freeze”的明确裁决输入。
>
> **本任务完成后，系统应达到：**
>
> `CORE FREEZE REVIEW → FREEZE CANDIDATE → FREEZE`
>
> 而不是继续进入无限修复循环。

---

# 0. STOP / NON-GOALS

本任务首先建立施工边界。

## 0.1 禁止事项

除非发现 F0 级硬阻塞，否则禁止：

1. 新增 Memory Store / Fact Store。
2. 新增第二套事实权威。
3. 修改 TaskGraph 数据模型。
4. 修改 TaskGoal / Completion / Terminal 基本语义。
5. 修改 Verification 唯一生产者原则。
6. 修改 Resume 核心恢复协议。
7. 修改 Approval / Sandbox 安全边界。
8. 为了提高 benchmark 成功率修改 planner 策略。
9. 为了消除 F1 数量而进行“顺手修复”。
10. 新增未经 Freeze Review 批准的 progress / semantic projection。
11. 将 model / decision 层问题强行下沉为 mechanism 修复。
12. 以新的真机成功样本覆盖既有失败证据。

## 0.2 特别禁止

以下四项不得在本任务中直接施工：

* RC48：Reserve 零和导致的 false stop。
* QA planner completion-awareness。
* Archive C-probe recoverability。
* MAX_HISTORY_MSGS=40 的 archive/recovery 改造。

它们必须先完成**边界判断与立项裁决**。

---

# 1. Node 00 — Baseline Integrity

确认当前施工对象确实是 v0.2.18。

必须核验：

```bash
git describe --tags --always
git status --short
grep -n "version" Cargo.toml
/usr/local/bin/hearth --version
```

同时确认：

```bash
git rev-parse HEAD
```

与当前报告中的 v0.2.18 provenance 对齐。

## 成功标准

必须明确记录：

* source revision
* binary version
* binary hash
* gate log
* working tree 状态
* 当前 VM
* 当前磁盘空间

如果发现 binary/source/tag 不一致：

**立即 STOP。**

不得继续做任何行为结论。

**（v1.1 必办一：RC49 规范链 compact 腿证据时效——守门员已实测实锤）**：
砺批-1 的时间线怀疑经本窗 VM 实测**确认为真**——
- `run_cfr_n08a.sh`（含 `export HEARTH_COMPACT_CHAR_THRESHOLD=7000`）提交于
  `8edb0bd` = **05:14:06**；
- 阈值优先级修复（env > 注入）`c514234` = **06:34:10**，晚 1h20m；
- **决定性证据**：`.131:/tmp/cfr_n08/FREEZE_RESULT.txt` mtime = **05:21:22**
  → n08 规范链实际执行于修复前 73 分钟，当时 env 7000 被 provider 注入值
  195,840 压过 → **压缩未发生，该链的 compact→resume 腿证据无效**（CFR
  Final Report line 50 "compact（env 7000，S-3）… CHAIN_OK ✓" 对 compact
  环节失真）。
处置（Node 00 必办）：① 用 v0.2.18 干净二进制**重跑一次规范链**
（fail→repair→compact→stop→resume→complete），compact 实证以 **archive 落盘
增长 + session 归档文件**为权威信号（禁日志 grep——P2-MC/批-6 既有纪律）；
② 重跑通过 → CFR 的 Long-run 柱恢复完整；③ 无法重跑 → 按 Node 13 规则
（provenance/证据无法建立 → BLOCKER），不得据此进入 CORE FREEZE。
同族核查：P2-LR Node 13"双压缩"已更正失真（整合报告 §12-4），CFR n08 是
同缺陷最后一处未复查的证据——本条是本次审计最有价值的产出。

**（v1.1 必办二：.131 诊断构建矛盾裁决——承接砺批-2 并升级）**：
CFR Final Report line 90 自曝".131 binary 含诊断 eprintln（正式发布前移除）"，
但 handoff pack §11 写".131 0.2.18（干净重建，**无诊断**）"——**两份文档直接
矛盾**。本窗实测：产线源码无诊断 eprintln（loop.rs 的 eprintln 全在
`#[cfg(test)]`（:5010 起）测试模块内；COMPACT_DBG 仅存于注释/测试），binary
mtime = 07:40:55（release 提交 07:42 前 1.5 分钟）。Node 00 必须用
`strings` 抽查或干净重建消除歧义，并记录 binary sha256；CLOSURE 对真机证据
二选一并写入 freeze-decision.md：显式接受"证据产自含诊断输出的构建（说明诊断
点不改变控制流）"或关键链用干净构建重跑。**不得在本轮移除 eprintln**（生产
代码改动，§0.1-9 禁令）。

---

# 2. Node 01 — Final Gate Reproduction

重新执行当前冻结基线 Gate。

```bash
bash ~/run_gate_r2c.sh
```

必须确认：

```text
FMT = 0
CLIPPY = 0
RT4_SOLO = 0
TEST = 0
447 passed / 0 failed
```

如果测试数量发生变化：

不得直接视为失败。

必须解释：

* 新增/删除了什么测试；
* 是否代码发生变化；
* 是否环境导致；
* 是否与 v0.2.18 报告一致。

---

# 3. Node 02 — Freeze Boundary Audit

建立一份最终 **Core Boundary Matrix**。

至少覆盖：

| Domain       | 当前权威                    | 是否允许本轮修改 | Freeze 状态 |
| ------------ | ----------------------- | -------- | --------- |
| Intent       | 既有 Intent authority     | NO       |           |
| TaskGraph    | 既有 TaskGraph authority  | NO       |           |
| TaskGoal     | 既有 Goal authority       | NO       |           |
| Execution    | Executor/Tool authority | NO       |           |
| Failure      | classify_failure        | NO       |           |
| Verification | O-4 唯一生产者               | NO       |           |
| Completion   | 双证据门                    | NO       |           |
| Fact         | state + archive         | NO       |           |
| Memory       | compaction + archive    | NO       |           |
| Resume       | persisted state         | NO       |           |
| Decision     | planner + loop consumer | NO       |           |
| Terminal     | 九态                      | NO       |           |
| Projection   | RC20 家族                 | NO       |           |
| Security     | Sandbox + Approval      | NO       |           |

要求：

**任何 domain 都不得出现第二 authority。**

如果发现新的 authority：

标记：

```text
F0 / F1 / F2
```

并 STOP，不得自行修。

---

# 4. Node 03 — F0 Hard-Failure Recheck

重新审查以下六项：

### F0-1

False Completion

### F0-2

Silent Fact Loss

### F0-3

Resume Re-teach

### F0-4

Unbounded Loop

### F0-5

Sandbox / Approval Bypass

### F0-6

Projection False Success

要求：

不能只引用 Final Report。

至少从：

```text
源码机制
+
fixture
+
已有真机证据
+
独立复验
```

四个层面重新确认。

输出：

```text
F0 = 0
```

或者给出明确阻塞项。

**任何 F0 ≠ 0，立即停止 Freeze 流程。**

---

# 5. Node 04 — RC48 Final Boundary Audit

这是本轮最高优先级。

不要修 RC48。

只回答一个问题：

> RC48 是 Core mechanism defect，还是已经明确、有界、可解释的 Decision/Model boundary？

复核：

```text
n12r1
n12r3
```

重点重建：

```text
Reserve = 1
→ 早期 Verification 消耗 Reserve
→ GiveUp 时 Reserve = 0
→ 无第二次 Verification
→ false stop
```

以及：

```text
同任务
+
同参数
+
GiveUp 时 Reserve = 1
→ GIVE_UP_OVERRIDDEN
→ Verification PASS
→ completed
```

必须确认：

### 假设 A

Reserve 零和是决定性条件。

### 假设 B

存在 mechanism 层随机/状态污染导致 false stop。

### 假设 C

planner GiveUp 时点缺乏完成事实感知。

最终给出：

```text
RC48 classification:
mechanism / decision / model / environment
```

以及：

```text
Freeze blocker: YES / NO
```

## 强制纪律

如果结论为：

```text
decision + model
```

则：

**不得修改代码。**

必须生成独立 RFC 候选：

```text
RC48-FOLLOWUP
```

**（v1.1 判据预写，承接砺批-4）**：按 CFR 证据（n12r1/r3 同任务同参数对照）与
CLOSURE 纪律，预写 disposition 供 Node 04 核对：
- "Reserve 零和" = **mechanism 层有界设计**（run-level 单 Reserve，FA01 裁决
  不翻案）；"GiveUp 时点" = **model 层**（模型何时自报/放弃不可控）；
- → **RC48 = mechanism(bounded) + model，Freeze blocker = NO，disposition =
  ACCEPTED DEVIATION + FOLLOWUP RFC**（两案：Reserve 分账 / acceptance-passed
  消费扩展——须顶层批准，本轮不实现）；
- **改判条件唯一**：若 Node 04 发现"mechanism 层随机/状态污染"（假设 B）→
  BLOCKER——按 n12r1/r3 对照证据该假设已基本排除；
- **不得因 n12r3 一次成功把 RC48 降级为"已解决"**——它是稳定可复现边界
  （2/14 会复发）。

---

# 6. Node 05 — QA Completion-Awareness Boundary

审查：

> planner 是否应该知道“已有产物已经满足任务完成条件”？

不要直接实现 completion-awareness。

先验证现有边界：

```text
planner
    ↓
GiveUp
    ↓
loop consumer
    ↓
verification / completion
```

回答：

1. planner 是否拥有 Completion Authority？
2. planner 是否应该拥有？
3. 当前 GiveUp 是否允许不知道最终事实？
4. RC47 已经解决的 criteria-empty blind spot 是否足够？
5. QA 16 轮中的 T4 高频 stall 是否是 planner/model 行为问题？
6. 如果给 planner 注入 completion facts，会不会产生 authority duplication？

最终输出：

```text
QA completion-awareness:
ACCEPT AS MODEL/DECISION LIMITATION
或
BLOCK FREEZE
```

默认倾向：

**不修改 Core。**

**（v1.1 判据预写，承接砺批-5）问题 6 的答案是"会"**：给 planner 注入
completion facts = 让它同时持有决策权与完成事实 → **authority duplication**。
CFR Authority Matrix 已确认 give_up 的**决策权（planner）/否决权（loop 消费
端）分离**是修 D 预裁决的有意设计；历史上 planner 对 acceptance 全盲
（`grep acceptance planner/` = 0）正是 loop 侧拦截成为唯一可行解的原因。
**建议 disposition = ACCEPTED DEVIATION（model/decision limitation）**；
若日后立项 RFC，边界限定为"只读投影且不得用于 GiveUp 判定"，或直接不做。

---

# 7. Node 06 — Archive C Recoverability Closure

本节点只做证据审计。

当前事实必须保持：

```text
Archive preserved = PROVEN
Model recoverability through grep = UNPROVEN
```

严禁把：

```text
archive exists
```

升级成：

```text
model can recover archive facts
```

重新检查已有：

* C-probe v2
* C-probe v3
* C-probe v4

解释为什么：

```text
v2 = user message pollution
v3 = enqueue_user_message contamination
v4 = setup failure
```

如果仍无法建立干净 C-probe：

最终状态：

```text
C = UNKNOWN
```

而不是：

```text
PASS
```

也不是：

```text
FAIL
```

裁决目标：

> Archive recoverability 未证明是否阻塞 Core Freeze？

默认倾向：

**NO BLOCKER。**

因为 M8 的核心要求是避免第二事实模型，Archive 已存在并承担持久化职责；C-probe 是 recoverability evidence gap，而不是 Core authority defect。

**（v1.1 判据预写，承接砺批-6）**：C = UNKNOWN（v2 用户消息污染 / v3
`enqueue_user_message` 永驻保留轮 / v4 setup 失败——三版污染已定性）不得升
PASS/FAIL，同意 NO BLOCKER。**硬要求**：freeze-decision.md 的
**"What Hearth Core does not guarantee"** 段必须写明"archive 落盘已证明；
模型经 grep 主动恢复旧事实**未证明**"——否则 Freeze 后会被下游当成已保证能力。
重试探针时按批-5 防污染设计要点（事实只存在于被压缩轮次），污染即登记不硬凑。

---

# 8. Node 07 — MAX_HISTORY_MSGS=40 Boundary

确认：

```text
MAX_HISTORY_MSGS = 40
```

当前语义：

```text
history slice
→ old messages leave LLM-visible context
→ history note exists
→ no archive
→ no recovery channel
```

必须明确区分：

```text
silent loss
```

与：

```text
bounded, declared loss-to-LLM
```

重新检查：

* state facts 是否仍存在；
* Continuity 是否仍投影关键事实；
* slice note 是否存在；
* 是否发生第二事实权威。

最终给出：

```text
Mechanism status
Risk level
Freeze blocker
Follow-up recommendation
```

不得在本轮增加 archive schema。

**（v1.1 判据预写，承接砺批-7）**：40 切片属 **bounded declared loss-to-LLM**
（`[history note]` 标记已落 + state 层存活 + Task Continuity 投影关键事实），
**不是 silent loss**——这个区分是 F0-2 不命中的关键，Node 07 必须显式写出判级
依据。**disposition = ACCEPTED DEVIATION**；follow-up = 切片内容入 archive
（小改，冻结后独立单）或标记文本改为"该内容已不可检索"（诚实性修正）；
本轮禁加 archive schema ✓。

---

# 9. Node 08 — Memory Invariant Final Check

重新验证：

```bash
cargo test -p agent-core --lib inv_m01
```

必须：

```text
2 passed
```

同时重新确认：

### A 档

State facts preserved。

### B 档

Archive preserved。

### C 档

Model recoverability：

```text
UNPROVEN
```

禁止将 C 档升级。

最终记录：

```text
A = PASS
B = PASS
C = UNKNOWN
```

---

# 10. Node 09 — Resume Final Check

重点不是重新跑所有 benchmark。

只复核：

```text
resume
→ persisted state
→ no task re-teach
→ no duplicate execution
```

已有三个证据：

```text
reteach = 0
duplicate execution = 0
```

必须确认这些证据仍对应 v0.2.18。

---

# 11. Node 10 — Reliability Claims Audit

对整合报告中的所有强断言做一次“证据强度降级检查”。

特别搜索：

```text
zero
all
always
correct
reliable
complete
proven
```

每个强断言必须归类：

```text
PROVEN
BOUNDED
UNPROVEN
UNKNOWN
MODEL-DEPENDENT
ENVIRONMENT-DEPENDENT
```

禁止出现：

```text
successful sample
→ generalized guarantee
```

尤其注意：

* false stop
* Archive C
* textkit convergence
* QA planner
* Agnes provider behavior
* model variance

**（v1.1 必办，承接砺批-3）false stop "2+12" 构成明细**：整合报告 §5 写
"false stop **2+12**（RC48 两形态，有界）"，但 **12 的来源未展开**（推测 =
QA 轮 T4 2-node stall 高频）。Node 10 必须给出 12 的构成明细（哪几跑、哪类
形态、层归 mechanism/model），否则"有界"无法验证，RC48 的 disposition 失去
数量基础。

---

# 12. Node 11 — Final Reliability Matrix

生成最终矩阵：

| Capability   | Mechanism | Decision | Model | Environment | Evidence | Freeze |
| ------------ | --------- | -------- | ----- | ----------- | -------- | ------ |
| Intent       |           |          |       |             |          |        |
| Plan         |           |          |       |             |          |        |
| Execution    |           |          |       |             |          |        |
| Failure      |           |          |       |             |          |        |
| Verification |           |          |       |             |          |        |
| Completion   |           |          |       |             |          |        |
| Memory       |           |          |       |             |          |        |
| Resume       |           |          |       |             |          |        |
| Decision     |           |          |       |             |          |        |
| Terminal     |           |          |       |             |          |        |
| Projection   |           |          |       |             |          |        |
| Security     |           |          |       |             |          |        |

每个 PASS 必须有 evidence reference。

不能使用：

```text
tested
verified
works
```

作为证据。

必须回答：

```text
WHAT
WHERE
HOW
UNDER WHICH CONDITIONS
```

---

# 13. Node 12 — F1 Disposition Table

最终建立：

| F1                      | Classification | Bounded? | Core violation? | Freeze blocker? | Follow-up |
| ----------------------- | -------------- | -------- | --------------- | --------------- | --------- |
| RC48 false stop         |                |          |                 |                 |           |
| QA completion-awareness |                |          |                 |                 |           |
| Archive C               |                |          |                 |                 |           |
| 40-slice recovery       |                |          |                 |                 |           |

要求：

每一个 F1 必须只有一个最终 disposition：

```text
ACCEPTED DEVIATION
FOLLOW-UP RFC
BLOCKER
```

禁止：

```text
OPEN
```

作为最终裁决。

OPEN 是状态，不是 disposition。

---

# 14. Node 13 — Freeze Decision

按照以下规则裁决：

## FREEZE BLOCKER

仅当发现：

1. F0；
2. 第二事实 authority；
3. Verification authority 冲突；
4. Completion 可被无证据触发；
5. Resume 导致事实重教育或重复执行；
6. Sandbox / Approval 可绕过；
7. Memory 存在未声明的静默事实丢失；
8. Terminal 出现不可解释越级；
9. provenance 无法建立。

否则：

允许 F1/F2 作为 Accepted Deviations。

---

# 15. Node 14 — No-Code Freeze Gate

如果 Node 00-13 均通过：

**禁止再修改生产代码。**

执行：

```bash
git status --short
git diff
```

要求：

```text
production code diff = 0
```

如果本轮没有代码修改：

不得人为 bump version。

当前 v0.2.18 保持。

---

# 16. Node 15 — 生成最终 Freeze Package

必须生成：

```text
docs/hearth-core-freeze-closure-01-final-report-v1.md
docs/hearth-core-freeze-closure-01-review-pack-v1.md
docs/core-freeze-review/freeze-decision.md
```

其中 `freeze-decision.md` 必须只有一个最终结论：

```text
CORE FREEZE
```

或者：

```text
CORE FREEZE BLOCKED
```

不得使用模糊状态。

Final Report 必须包含：

1. Executive Summary
2. Baseline / Provenance
3. F0 Recheck
4. Authority Matrix
5. RC48 disposition
6. QA completion-awareness disposition
7. Archive C disposition
8. 40-slice disposition
9. Memory invariant
10. Resume invariant
11. Reliability Matrix
12. F1/F2 disposition
13. Known limitations
14. Freeze boundary
15. Final Decision
16. Reproducibility commands

---

# 17. 最终 Gate

执行：

```bash
bash ~/run_gate_r2c.sh
```

要求：

```text
FMT = 0
CLIPPY = 0
RT4_SOLO = 0
TEST = 0
447 passed / 0 failed
```

然后：

```bash
git status --short
git describe --tags --always
```

确认：

```text
v0.2.18
clean working tree
```

---

# 18. 成功标准

本任务成功**不是**：

> “把所有 OPEN 都修掉。”

而是：

> **证明哪些问题已经属于 Core，哪些问题明确不属于 Core，并把剩余不确定性全部显式化。**

最终必须满足：

```text
F0 = 0

Core authority = single-source

Fact authority = single-source

Verification authority = single-source

Completion requires evidence

Resume reteach = 0

Duplicate execution = 0

Silent loss = 0

Sandbox bypass = 0

Infinite loop = 0

All remaining F1 = explicitly classified

No unauthorized Core expansion

No production code changes
```

---

# 19. 最终输出格式

执行结束后只提交以下四部分：

### A. Verdict

```text
CORE FREEZE
```

或：

```text
CORE FREEZE BLOCKED
```

### B. F1 Disposition

逐项给出：

```text
RC48:
QA completion-awareness:
Archive C:
40-slice:
```

### C. Evidence

每项只引用：

```text
file
line
test
log
command
```

禁止使用“已验证”“已完成”等无证据表述。

### D. Residual Risk

明确：

```text
What Hearth Core guarantees
What Hearth Core does not guarantee
What must become a future independent RFC
```

---

# 20. 最重要的施工纪律

本任务执行期间，请始终遵守：

> **不要因为系统还有问题，就认为 Core 还没有完成。**

真正成熟的 Core 不是：

```text
没有问题
```

而是：

```text
问题有边界
权威有边界
失败有边界
恢复有边界
未知有边界
未来修改也有边界
```

如果本轮最终发现：

```text
RC48 = bounded decision/model limitation
QA = model/decision limitation
Archive C = evidence gap
40-slice = bounded declared limitation
```

那么：

**不要继续施工。**

直接进入：

```text
CORE FREEZE
```

并将后续问题转化为独立 RFC，不得污染 Core Freeze。

---

# 附：砺·评审批注（2026-08-31，总判断：🟡 有条件通过——批-1/批-2 必须在 Node 00 先行解决，否则 Long-run/Evidence 两柱不实）

> CFR 事后审计基线：v0.2.18 `41cc070`，阈值优先级修复实证（context.rs:229-244：env 测试仪器 > provider 注入 > 遗留常量；测试 1160-1168）；四包锚点（FA01 拦截标记 21 处 / P2-MC turn 粒度 + `context_fill_pct` 零命中 / P2-LR BashExitError+history-slice-note+set_session_id / 5 个关键测试）在 v0.2.18 **全部存活**（回归确认）。提交链 8edb0bd→c514234→f9a525a→41cc070→5b0c7b9 一致，tag v0.2.18 在库。gate 447/0 与真机日志在 VM 侧未复跑（证据边界）。

## 批-0（CFR 终态裁定：同意 READY WITH ACCEPTED DEVIATIONS，但附条件）

CFR 的价值很高：阈值优先级缺陷（Agnes caps 128K 注入 → 阈值 195,840 压过测试仪器，`COMPACT_DBG est=68 thr=195840` 冒烟实锤）是**横向发现**——它不只修了一个常量，而是否掉了"受控压缩实验"整类证据的有效性。Authority Matrix 零双 authority、RC48 用 n12r3 做**同任务同参数可比复现**得到决定性反证（Reserve=1 → 拦截 → completed 31 步）——本轮方法论最漂亮的一手。**但见批-1：同一缺陷可能反噬了 CFR 自己的规范链证据。**

## 批-1（🔴 最高优先级：Node 08 规范链 compact 腿的证据时效存疑——RC49，潜在 Freeze-blocker）

**事实链（本窗实测）**：
1. `docs/data/core-freeze-review-20260831/run_cfr_n08a.sh` 明确 `export HEARTH_COMPACT_CHAR_THRESHOLD=7000`（强制压缩，造 chainfree 链的 compact 环节）；
2. 该脚本（连同 n08b/n09/n10/n11）提交于 `8edb0bd` = **05:14**；
3. 阈值优先级修复 `c514234` = **06:34**（晚 1h20m）；
4. 修复前语义 = provider 注入值 195,840 **压过** env → 该 env **被静默忽略 → 压缩未发生**；
5. CFR Final Report line 50 仍写"compact（env 7000，S-3）… CHAIN_OK ✓"，**未声明该跑是否在修复后重跑、当时 .131 binary 是哪个版本**。

这与 P2-LR Node 13"双压缩"声明失真（报告 §12-4 已如实更正）**是同一缺陷的同一类后果**——但 Node 08 规范链没有做同等复查。**CLOSURE Node 00/03 必须先解决**：
1. 用 git/日志时间戳 + `hearth --version` 记录确定 n08a/n08b 实际执行时刻与当时二进制；
2. 若执行于修复前 → **该链的 compact→resume 腿作废**，须用 v0.2.18 干净二进制重跑一次规范链，compact 实证以 **archive 落盘增长 + session 归档文件**为权威信号（不得用日志 grep，沿用既有纪律）；
3. 若无法重跑 → Long-run 柱判"证据不完整"，按本包 Node 13 规则（provenance 无法建立 → BLOCKER）不得据此进入 CORE FREEZE。
**理由**：Long-run 五层柱中"compact → resume → continue"是唯一不可替代的支柱，空心则整个 Freeze 结论悬空。此条是本次审计最有价值的产出，也是唯一可能把结论推向 BLOCKED 的项。

## 批-2（🔴 .131 真机 binary 含诊断 eprintln：证据产自诊断构建，须显式接受或另立后置单）

CFR provenance 已自曝".131 binary 含诊断 eprintln（正式发布前移除）"。即真机 benchmark 跑在诊断构建上。CLOSURE 须二选一并写入 freeze-decision.md：①显式接受"真机证据产自含诊断输出的 v0.2.18 构建"（并说明诊断点为何不改变控制流）；②或声明该组证据为诊断态，关键链用干净构建重跑。**注意边界**：Node 14 no-code gate 与 §0.1-9 禁止顺手修复——移除 eprintln 属生产代码改动，**不得在本轮做**，只能接受或另立后置单。这正是"禁止为了消除 F1 而顺手修复"纪律的典型考题。

## 批-3（🟡 false stop "2+12" 的构成必须明细化）

报告 §5 写"false stop **2+12**（RC48 两形态，有界）"，但 12 的来源未展开（推测 = QA 轮 T4 2-node stall 高频）。Node 10 强断言降级检查必须给出 12 的构成明细（哪几跑、哪类形态、层归 mechanism/model），否则"有界"无法验证，RC48 的 disposition 也失去数量基础。**另**：不得因 n12r3 一次成功把 RC48 降级为"已解决"——它仍是稳定可复现边界（2/14 会复发）。

## 批-4（🟡 RC48 disposition 判据预写，供 Node 04 核对）

按 CFR 证据与 CLOSURE 强制纪律（decision+model → 不得改代码 → 生成 RC48-FOLLOWUP）：
- "Reserve 零和" = **mechanism 层有界设计**（run-level 单 Reserve，FA01 裁决不翻案）；
- "GiveUp 时点" = **model 层**（模型何时自报/放弃不可控）；
- → **RC48 = mechanism(bounded) + model，Freeze blocker = NO，disposition = ACCEPTED DEVIATION + FOLLOW-UP RFC**；
- FOLLOW-UP 两案（Reserve 分账 / acceptance-passed 消费扩展）**须顶层批准，本轮不实现**（§0.2-1 已禁，正确）；
- 判据写死：若 Node 04 额外发现"存在 mechanism 层随机/状态污染"（假设 B），则改判 BLOCKER——按 n12r1/r3 对照证据，此假设已基本排除。

## 批-5（🟡 QA completion-awareness（Node 05）：authority duplication 会发生，倾向"不改"正确）

问题 6"给 planner 注入 completion facts 会不会产生 authority duplication？"——**会**。CFR Authority Matrix 已确认 give_up 的**决策权（planner）/否决权（loop 消费端）分离**是修 D 预裁决的有意设计；历史上 `grep acceptance planner/` = 0（planner 盲）正是 loop 侧拦截成为唯一可行解的原因。喂 completion facts 进 planner = 让它同时持有决策权与完成事实 → 双轨。**建议 disposition = ACCEPTED DEVIATION（model/decision limitation）**；若日后做 RFC，边界限定为"只读投影且不得用于 GiveUp 判定"，或直接不做。

## 批-6（🟡 Archive C（Node 06）：同意 NO BLOCKER，但必须写进"does not guarantee"）

C = UNKNOWN（v2/v3 用户消息污染、v4 setup 失败，三版污染已定性），不得升 PASS/FAIL——同意默认 NO BLOCKER（M8 核心是避免第二事实模型；C 是 evidence gap 非 authority defect）。**硬要求**：freeze-decision.md 的"What Hearth Core does not guarantee"段必须写明"archive 落盘已证明；模型经 grep 主动恢复旧事实未证明"——否则 Freeze 后会被下游当成已保证能力。

## 批-7（🔵 40 切片（Node 07）：判级正确，disposition = ACCEPTED DEVIATION）

属 **bounded declared loss-to-LLM**（标记已落 + state 层存活 + Task Continuity 投影关键事实），**不是 silent loss**——这个区分是 F0-2 不命中的关键，Node 07 必须显式写出判级依据。disposition = ACCEPTED DEVIATION；follow-up = 切片内容入 archive（小改，冻结后独立单）；本轮禁加 archive schema ✓。

## 批-8（✅ 总包结构评价：止损设计优良）

§0.2 特别禁止四项直接施工、Node 12 禁止以 OPEN 作 disposition（"OPEN 是状态不是裁决"）、Node 14 no-code gate（顺带禁止人为 bump）、§18 九条硬成功标准、§19 四项输出（Verdict / F1 disposition / Evidence / Residual Risk）、§20"问题有边界"六句——这套设计把"不要因为还有问题就认为 Core 没完成"翻译成了可执行纪律，与守门员方法论同向。**唯一缺口**就是批-1/批-2 两条证据时效问题未覆盖：本包所有 Node 都假定既有证据有效，而 CFR 自己刚证明了一类证据（受控压缩）曾系统性失效。

**放行条件**：批-1（RC49：Node 00/03 先定执行时刻与二进制版本，必要时 v0.2.18 重跑规范链）与批-2（诊断构建证据显式接受）写进 Node 00 强制项；批-3~7 为 Node 内判据。守门人零代码改动。

---

# 附 2：守门员复核定稿批注（2026-08-31，总包 v1.1 定稿依据）

> 定位：砺批-0~8 已审（附 1），本节是**对审计的审计**（锚点实测）+ 顶层定稿裁决。
> 实测基线：本机 HEAD `5b0c7b9`（tag v0.2.18 → `41cc070` 之后含 handoff pack 提交，链一致）。

## 复-1 CFR 上游核验 + RC49 实锤（本节为本轮最重要产出）

| 项 | 声明方 | 实测 |
|---|---|---|
| 提交时间线（n08 脚本 05:14 / 阈值修复 06:34 / Final Report 07:37 / release 07:42） | 砺批-1 | ✅ git 实测逐分吻合 |
| n08 规范链实际执行时刻 | 砺批-1 疑似 | ✅ **实锤**：`.131:/tmp/cfr_n08/FREEZE_RESULT.txt` mtime = **05:21:22**，早于修复 73 分钟 → env 7000 当时被注入值 195,840 压过 → **压缩未发生，规范链 compact 腿证据无效（RC49 成立）** |
| 阈值优先级修复（env > 注入 > 常量） | 砺附 | ✅ context.rs:229-244 + 测试 :1160-1168 |
| 四包锚点在 v0.2.18 存活 | 砺附 | ✅ 抽验（RC47 路由 / BashExitError / 切片标记+断言 / session 绑定 / INV-M01 双 fixture 均在） |
| gate `.133:~/t_gate_cfr_final.log` = 447/0 四 RC=0 | handoff §6 | ✅ paramiko 实测 |
| .133 binary 滞后已实修 | handoff §11 | ✅ `/usr/local/bin/hearth` = 0.2.18（mtime 06:36，修复后 2 分钟重建） |
| .131 binary = 0.2.18 | handoff §11 | ✅ 实测（mtime 07:40） |
| 产线源码无诊断 eprintln | 砺批-2 前提 | ✅ loop.rs eprintln 全在 `#[cfg(test)]`（:5010 起）测试模块；但 **CFR 报告 line 90 自曝".131 binary 含诊断"与 handoff §11"无诊断"直接矛盾**——Node 00 必须裁决（S-2） |

结论：**CFR = READY WITH ACCEPTED DEVIATIONS（建议）成立，但其 Long-run 柱的
compact 腿被 RC49 击穿**——批-1 是全场最有价值的审计产出，已升级为 Node 00
实锤条款（重跑规范链，archive 落盘为权威信号）。

## 复-2 砺批-0~8 采纳状态

- 批-0（CFR 终态：同意 READY WITH ACCEPTED DEVIATIONS，附条件）：**采纳**——条件即批-1/批-2，已入 Node 00。
- 批-1（🔴 RC49）：**采纳 + 实测实锤升级**，内联 Node 00（决定性时间戳证据 + 重跑处置 + BLOCKER 判据）。
- 批-2（🔴 诊断构建）：**采纳 + 矛盾升级**（handoff §11 vs CFR line 90），内联 Node 00（strings/重建裁决 + sha256 记录）。
- 批-3（🟡 false stop 2+12 明细）：**采纳**，内联 Node 10。
- 批-4（🟡 RC48 disposition 预写）：**采纳**，内联 Node 04（mechanism(bounded)+model → ACCEPTED DEVIATION + FOLLOWUP RFC；唯一改判条件 = 假设 B）。
- 批-5（🟡 QA completion-awareness）：**采纳**，内联 Node 05（authority duplication 会发生；disposition = ACCEPTED DEVIATION）。
- 批-6（🟡 Archive C）：**采纳**，内联 Node 06（C=UNKNOWN 不升格；does-not-guarantee 硬要求）。
- 批-7（🔵 40 切片）：**采纳**，内联 Node 07（bounded declared loss-to-LLM ≠ silent loss）。
- 批-8（✅ 止损设计评价）：确认——§0.2 四项禁施工 / OPEN 非 disposition / no-code gate / §19 四项输出，与本窗方法论同向。

## 复-3 守门员补充（S-1~S-3）

- **S-1（🔴 并入批-1）**：RC49 决定性证据 = `/tmp/cfr_n08/FREEZE_RESULT.txt` mtime 05:21:22（本窗实测）——重跑规范链时同步记录新 binary sha256 + 新 archive 行数基线。
- **S-2（🔴 并入批-2）**：.131 诊断矛盾（CFR line 90 vs handoff §11）必须以 strings/重建实测裁决，不允许两份文档各说各话进入 freeze-decision.md。
- **S-3（🟡 Node 01/17）**：gate 复现以 **447** 为记账基线（测试计数变化须逐项解释）；本轮零代码改动则不发版（Node 14 既有条款），v0.2.18 保持。

## 复-4 定稿裁决（Freeze 预期管理）

**本总包 v1.0 → v1.1 定稿，放行执行。** 砺批-0~8 全部采纳并内联（Node 00/04/05/06/07/10）；
守门员补 S-1~S-3（并入批-1/批-2 与 Node 01）。顶层裁决预期（沿用上轮批-8 预判并更新）：
- **RC49 是唯一可能翻盘项**：重跑规范链通过 → **CORE FREEZE（附 Accepted Deviations）**可达成；
- 重跑失败或无法重跑 → 按 Node 13 规则 **CORE FREEZE BLOCKED**（Long-run 柱空心）——这是纪律要求的诚实结果，不是失败；
- 四项 F1 disposition 预写均已就位（RC48=ACCEPTED+RFC / QA=ACCEPTED / Archive C=ACCEPTED / 40 切片=ACCEPTED），
  Node 12 只须按证据落表，禁止以 OPEN 作 disposition。
freeze-decision.md 的"does not guarantee"段必须包含：archive 模型恢复未证明 / 40 切片 lost-to-LLM /
RC48 边界（2/14 复发率）/ Agnes model variance。守门人零代码改动。
