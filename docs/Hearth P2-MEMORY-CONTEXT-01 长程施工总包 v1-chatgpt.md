# Hearth P2-MEMORY-CONTEXT-01 长程施工总包 v1.1

## Compaction Calibration / Fact Persistence / Long-context Continuity

**日期**：2026-08-30（v1.1：砺批-0~8 内联 + 守门员复核定稿）

**性质**：完整长程施工总包 / 一次授权连续执行

**执行方式**：Node 00→Node 15 连续自主执行；普通失败自行诊断修复；仅 STOP 条件允许暂停

**最终交付**：一次性提交 Final Report

**当前基线**：Hearth v0.2.15 / HEAD `ceb62b0`

**上游**：

- R2-C ContextBuilder = CLOSED
- W3/W4 = CLOSED
- RC24 = CLOSED
- W8 = CLOSED
- P1-LTR-01 = CLOSED
- P1-CONSOLIDATION-01 = PASS WITH DEVIATIONS
- P1-TASK-TRUTH-01 = PASS
- P1-EXECUTION-DECISION-01 = PASS
- P1-FAILURE-ADAPTATION-01 = PASS WITH DEVIATIONS（**已 CLOSED**，v0.2.15 `ceb62b0`，评审包结论复核采纳；6 项 OPEN 中 RC45/RC46 已入总账，不在 P2 范围）

---

# 0. 使命

本总包处理 Hearth Core 下一块主要未闭合地基：

> **长时间运行以后，Hearth 是否还能记得自己真正发生过什么，并在 Context Compaction / Resume / 长对话后保持任务连续性。**

历史已知症状：

```text
长对话
↓
context 接近阈值
↓
maybe_compact
↓
历史被压缩
↓
用户继续当前任务
↓
Hearth 无法正确回答“继续什么”
```

同时已经发现：

- compaction 阈值为 **32,000 字符**
- 真实 provider Agnes 已确认具有 **512K token context**
- Hearth 当前存在多个静默截断点
- Task Continuity 已从普通 history 中分离并维持
- Fact / Verification 是比普通 Conversation History 更高等级的数据
- R2-C / P1-TASK-TRUTH 已证明“事实≠对话文本”
- 当前仍缺乏充分的“compaction 前后 Fact/Verification 不丢失”长程实证

---

# 1. 本总包核心问题

本轮必须回答四个问题：

### Q1

> Hearth 是否在远未接近 provider 实际能力上限之前就开始过早 Compaction？

### Q2

> Compaction 是否是过去“失忆 / 目标漂移 / 当前任务连续性断裂”的主要原因？

### Q3

> 即使调整 Compaction 阈值，Fact / Verification 是否仍然会因为历史压缩而丢失？

### Q4

> Hearth 最终需要的是：
>
> **参数标定**
>
> 还是：
>
> **真正独立的 Fact / Verification Persistence 架构？**

---

# 2. 核心原则

## INV-M01：Fact 不随普通 Conversation Compression 丢失

```text
Conversation History
    ↓
可压缩

Fact / Verification / Task Continuity
    ↓
不可因普通 compaction 丢失
```

这不是要求：

> 所有原始对话永久保留。

而是要求：

> **决定任务真实状态的重要事实，必须能够在压缩后恢复。**

---

## INV-M02：先标定，再重构

不得直接：

```text
32K
↓
128K
```

必须：

```text
Baseline
↓
Measure
↓
A/B
↓
Behavior evidence
↓
Decision
```

---

## INV-M03：不要把“上下文窗口”与“可用上下文”混为一谈

Provider 声称：

```text
512K
```

不等于：

```text
Hearth 可以安全把 512K 全部塞进去。
```

本轮必须同时测：

```text
provider capacity
+
Hearth prompt composition
+
token cost
+
tool result growth
+
model behavior
+
latency
+
memory usage
```

---

## INV-M04：不改变已经稳定的 Core 事实模型

本轮：

- 不重建 TaskGraph
- 不重建 TaskGoal
- 不重建 Completion
- 不重建 Terminal
- 不重建 Failure Taxonomy
- 不重建 Execution Decision

Memory 只负责：

> **保存 / 压缩 / 恢复既有事实。**

---

# 3. 全局禁止项

本轮未经独立批准禁止：

```text
Subagent architecture
TUI
MCP
Bridge
Desktop
Persona
多代理
新 TaskGraph
新 TaskGoal
新 Completion system
新 Terminal state
新的 Verification authority
新的独立事实源
```

尤其禁止因为“将来可能有用”而提前新增：

```text
memory strategy config
verification strictness config
provider-specific policy layer
```

当前没有证据证明需要它们。

---

# 4. STOP CONDITIONS

## STOP-1

发现 Fact / Verification 当前不存在明确事实源，必须新建第二套事实模型。

## STOP-2

必须修改 TaskGraph / TaskGoal / Completion 的语义才能保存事实。

## STOP-3

Provider 实际 context 能力无法可靠测量，且没有可复现实验方法。

## STOP-4

修改 compaction 需要突破安全边界。

## STOP-5

必须将 LLM judgment 作为 Fact persistence 的唯一手段。

## STOP-6

必须改变 Terminal / Decision authority 才能实现 continuity。

## STOP-7

A/B 实验发现参数改变会明显破坏行为，但没有足够证据判断安全替代值。

## STOP-8

发现某个现有静默截断对事实安全性有直接影响，而修复需要扩大本总包范围。

此时先记录，停在诊断边界。

---

# 5. Node 00 — Baseline / Provenance

先不改代码。

核实：

```text
git rev-parse HEAD
git status
Cargo.toml
git tag
```

双 VM：

```text
.131
.133
```

严格遵守：

```text
vm-version-sync.md
```

记录：

```text
source
HEAD
binary
version
gate log
disk
provider
```

执行：

```bash
df -h /home
```

必须记录采样前：

```text
free disk
free memory
```

输出：

```text
docs/data/memory-context-20260830/node00-baseline.md
```

---

# 6. Node 01 — Actual characters/token calibration

这是本总包最重要的第一步。

## 目标

不要再假设：

```text
1 token ≈ 4 chars
```

也不要假设：

```text
中文 1-2 chars/token
```

必须以 Hearth 实际 workload 测量。

---

## 采样语料

至少包括：

```text
system prompt
Hearth.md
constitution
TaskGraph
Task Continuity
planner output
reflect output
tool result
中文用户对话
英文代码/日志
中英文混合
```

至少形成：

```text
pure Chinese
pure English
code
mixed
```

四种 corpus。

---


## 输出

记录：

```text
characters
tokens
char/token
token/char
```

并分别记录：

```text
system
history
TaskGraph
tool result
combined prompt
```

最终回答：

> 32,000 chars 在 Hearth 真实 workload 下到底对应多少 tokens？

禁止预写结果。

**（v1.1 必读先验，承接砺批-1）**：Node 01/02 不是从零考古，是**在 v0.2.15 复核已实测事实**。以下锚点本窗已 `grep` 复实（核查锚点原则）：

1. `context.rs:169 COMPACT_CHAR_THRESHOLD = 32_000`——**字符非 token、固定值、provider-unaware**（INV-M02 要求的"先标定"此前从未做过）；
2. `context.rs:174-181 estimate_chars` 用 **`format!("{:?}", m.content)` Debug 格式化**计数——**测量仪器本身系统性高估**（每块 MessageContent 包成 `String("...")` + 转义，约 2× 虚增）；Node 01 校准表必须分列"原始 estimate 口径"与"修正口径"两套数字（批-3）；
3. `llm-cn` provider 表**无 Agnes 条目**（HunyuanProvider `max_context_tokens=32_000`，Agnes 走 config.toml 通用 openai 兼容通道）——Node 02 "provider-aware?" 先验 = **否**；
4. 注释自承 `≈32k chars ≈ 8k tokens`；结合 estimate_chars Debug 虚增，真实触发点约 **16k tokens 级** = Agnes 512K 的 **~3%**——"过早压缩"方向性成立，但精确数字按 INV-M02 实测，禁预写；
5. `context.rs:312 summarize_turn` 纯规则模板不调 LLM、60 字墓碑（中文按字节切历史 panic 已修为 `chars().take(60)`）；Assistant 全文+tool_result 折叠为摘要——Node 03 BEFORE/AFTER 关键先验；
6. **RC40**：`loop.rs:1333 context_fill_pct = total_chars / 32k × 100` 注入 LLM 可见 body（status 工具同口径）——这是**语义错位**（"距压缩阈值距离"冒充"上下文填充率"），Node 02 必答项，且**改阈值治不了它**；
7. `context.rs:189-211 maybe_compact` **先 `archive_compacted_turns` 落盘再折叠**，摘要保留 goal+工具清单+写盘文件并注入归档检索提示——**Fact 不随压缩丢失的雏形已在**，Node 04 情况 A/B 判定先盘点复用（对齐 M8）。

---

# 7. Node 02 — Compaction Trigger Audit

源码审计：

```text
maybe_compact
context threshold
context_fill_pct
history trimming
summary generation
archive
restore
```

必须回答：

```text
什么时候触发？
按什么单位？
字符？
token？
估算 token？
provider usage？
```

以及：

```text
阈值是否 provider-aware？
Agnes / DeepSeek / local 是否统一？
```

重点检查：

```text
32,000
```

是否实际上是：

> 历史遗留值，而不是根据当前 provider capability 得出的值。

**（v1.1 必答，承接砺批-1 #6 / RC40）**：`context_fill_pct`（`loop.rs:1333`，注入

LLM 可见 body 与 status 工具）是"距压缩阈值的距离"却被命名为"上下文填充率"——

**语义错位，改阈值不治它**。Node 02 必须单列一节回答：该字段的真实含义、

命名是否误导模型对疲劳的判断、是否应改名为 `compact_pressure_pct` 或仅作内部

telemetry 不进 LLM 可见 content。这是本轮独立于阈值标定的修正项。

输出：

```text
docs/memory-context-compaction-audit.md
```

---

# 8. Node 03 — Compression Data-loss Audit

构造一个最小可验证 conversation：

```text
Task Goal
+
TaskGraph
+
Tool Result
+
Artifact Fact
+
Verification Result
+
Acceptance Result
+
User Decision
```

然后强制触发 compaction。

比较：

```text
BEFORE
vs
AFTER
```

至少检查：

```text
original_goal
goal_revision
TaskGraph topology
completed nodes
remaining nodes
next_action
successful tools
artifacts
verification result
acceptance result
failure facts
user constraints
```

对每一项标记：

```text
preserved
reconstructed
lost
unknown
```

**（v1.1 补 S-3，守门员）存储位置分类前置**：压缩只作用于 **history**——  
`maybe_compact` 只 drain `state.history`（context.rs:201）。goal / TaskGraph /  
acceptance_criteria / acceptance_result / scratch 等存在于 **state/scratch 层，  
根本不在压缩路径上**。Node 03 的逐项清单必须先标注每项的**存储位置  
（history / state / scratch / archive）**，BEFORE/AFTER 对比以存储位置为准——  
**禁止把本就不在 history 的字段判成"压缩后丢失"**：假阳性会直接把 Node 04  
推向情况 B，误建 Fact Store（违反 M8）。

---

# 9. Node 04 — Fact / Verification Persistence Design

基于 Node 03 的真实结果决定：

## 情况 A

Fact 已经可以从 RunState / TaskGraph / session store 完整恢复。

那么：

> 不新建持久化系统。

只做 compaction integration。

---

## 情况 B

部分 Fact 会跟 history 一起消失。

则设计：

```text
Fact Record
Verification Record
```

的最小持久化载体。

要求：

```text
session_id
task_id
sequence
source
type
payload
```

至少能够回答：

> “这条事实是谁产生的、什么时候产生、属于哪个任务？”

---

**（v1.1 必办，承接砺批-1 #7）**：

`context.rs:189-211 maybe_compact` **已先 `archive_compacted_turns` 落盘再折叠**，

摘要保留 goal+工具清单+写盘文件并注入归档检索提示——这是**可逆压缩雏形**，

已部分满足 M5/M8。Node 04 情况 A/B 判定前**必须先盘点 archive 机制的覆盖度**

（落盘路径 `archive/<sid>.jsonl`、best-effort 失败仅 warn）：能复用就不新建

Fact Store，避免违反 M8"不新增第二套事实模型"。

---

## 红线

不得建立：

```text
Memory Model
```

第二套 TaskGraph。

不得让 Memory 改写 TaskGraph 真相。

---

# 10. Node 05 — Compaction A/B Experimental Design

正式建立实验矩阵。

至少两个条件：

```text
A = 当前阈值
B = candidate threshold
```

candidate threshold **不得预设具体数字**。

由 Node 01/02 结果决定。

**（v1.1 必遵，承接砺批-2 / 批-3）**：

① candidate 阈值必须 **provider-aware 表达**（窗口 × 比例，如 60%），经 config

（如 `HEARTH_CONTEXT_TOKENS` 手工覆盖 + 已知模型表）而非硬编码常量——否则等于

为 Agnes 512K 标定出一个新常数，切 DeepSeek/local 32k 时再坏一次（R-δ 老路）。

配套最小切片：`/v1/models` 列表 + 已知表 + 探针缓存 + 手工覆盖（设想登记册

I-6，设计稿 `docs/design-ideas-best-practice-assessment.md` §2.6，本轮只取表+

覆盖+一个比例配置）。

② 测量纪律：provider usage（`response.prompt_tokens`）优先于任何本地估算——本地

估算只做交叉验证（批-3）。

---

## 控制变量

必须固定：

```text
provider
model
prompt
task
toolset
HEARTH_ALLOW_NO_CGROUP
deadline
budget
telemetry
execution environment
```

---

## 每个条件

最低：

```text
5 次配对实验
```

如果结果方差过大，则自动追加至：

```text
10 次
```

不能只做：

```text
A1
B1
```

就宣布结果。

**（v1.1 必遵，承接砺批-4 / 层归纪律）**：A/B 每条件 5-10 次长程 LLM 任务

（30-60 步/跑），方差主要来自 **model 层**（FA01 Node 09 已实证：同任务 3 跑

层归各不相同）。要求：A/B 结论**只消费 mechanism/decision 层稳定的跑次**；

model 层失败按 FA01 层归纪律单独列示，不得计入 threshold 效应——否则 A/B 会把

模型方差误读成参数效应。终止态对比同理：先分层再比较。

---

# 11. Node 06 — Long Conversation Paired Experiment

使用同一个长会话任务。

要求：

```text
至少触发一次 compaction
```

实验两组：

```text
A: baseline
B: candidate
```

每一组 5 次。

采集：

```text
compression_count
messages_before_compact
messages_after_compact
prompt_tokens
completion_tokens
latency
memory
goal retention
Task Continuity retention
Fact retention
Verification retention
terminal
```

**（v1.1 补 S-10，守门员）配对顺序必须交错**：A/B 各 5-10 次长程跑，若按
"先跑完 A 组再跑 B 组"的分组顺序执行，Agnes 时段性延迟/限流差异会系统性污染
对比（FA01 Node 09 r3 已实证 10 次 backoff 的时段效应）。要求：**A/BAB… 交错
配对执行**（同一天时段内 A、B 各一跑交替），并在报告中记录每跑的时间戳——
时间戳是层归 model 噪声的输入之一（衔接批-4）。

---

# 12. Node 07 — Memory Retention Benchmark

重点不是问模型：

> “你还记得吗？”

因为这是 self-report。

必须构造确定性检查。

例如：

```text
事实 1：
文件 abc.txt 内容 = X

事实 2：
测试结果 = FAILED

事实 3：
第二轮修复写入 Y

事实 4：
第三轮测试 = PASSED
```

压缩后要求系统重新展示这些事实。

检查：

```text
Fact persistence accuracy
Verification persistence accuracy
Goal persistence accuracy
```

其中至少一部分通过 deterministic state comparison 判断。

**（v1.1 补 S-6，守门员）"不丢"必须分两档口径**：压缩后系统找回事实有两条  
本质不同的通道——

- **A 档（上下文内保持）**：模型不借助任何工具直接答对（state/scratch 投影 +  
  摘要质量决定）；
- **B 档（经归档可恢复）**：模型用 bash grep `archive/<sid>.jsonl` 检索原文答对  
  （archive 机制决定）。

B 档依赖"模型知道自己该去 grep"——`maybe_compact` 已注入检索提示  
（context.rs:225-237），但这只是**提示存在 ≠ 检索成功**。Node 07 的三个  
retention accuracy 必须**分档报告**，M4/M5 的 PASS 判据须声明用哪一档：  
Goal/TaskGraph 类事实（state 层）应达 A 档 100%；tool result / 对话细节类  
事实允许 B 档，但必须实证"模型在提示下真实完成过至少一次 grep 检索"  
（日志留痕），不得把 B 档自动算成 A 档。

---

# 13. Node 08 — Goal / Task Continuity Regression

专门复现历史：

> “继续什么？”

场景：

```text
规划任务
↓
执行若干步
↓
强制 compact
↓
用户输入：
“继续”
```

必须验证：

```text
current_goal
original_goal
remaining work
next_action
completed work
```

均保持正确。

重点：

> “继续”不能重新产生 GoalMutation。

复用 W8 / R2-D 已有规则。

**（v1.1 先验，承接砺批-6）**：v0.2.15 已落修 B（疑问句类别 loop.rs:376-410）+

`classify_user_input` 的 TASK_CONTROL 白名单，"继续"→ GoalMutation 的历史形态

应已消失。若 Node 08 复现实验仍见 GoalMutation，先查是否压缩邻接变量（历史证据：

失败落在压缩点 200 行内 53-60%），勿直接怀疑分类器回归。

---

# 14. Node 09 — Resume After Compact

构造：

```text
Run
↓
执行若干步
↓
Compact
↓
停止
↓
Resume
↓
继续执行
```

验收：

```text
resume 后不需要用户重新教育任务
```

必须记录：

```text
re-teach count
goal retention
artifact retention
verification retention
incremental work
terminal
```

强标准：

```text
re-teach count = 0
```

---

# 15. Node 10 — Silent Truncation Inventory

审计当前已知的静默截断：

```text
200
60
8000
6000
4096
```

逐个回答：

```text
在哪？
截断什么？
为什么？
是否仍需要？
是否影响 Fact？
是否影响 Verification？
是否影响 Intent？
是否有标记？
```

---

## 第一阶段只做分类

```text
SAFE
NEEDS_REVIEW
FACT_RISK
VERIFICATION_RISK
```

不要因为发现数字过时就全部改掉。

**（v1.1 必查，承接砺批-5）**：

① 总包列的首 5 处（200/60/8000/6000/4096）与 W13 常量体检清单一致（render 工具

结果 200 / context 摘要 60 / web 正文 8000 / constitution 6000 / 子 agent

max_context_tokens 4096，落点见各 crate）——本窗已 grep 确认 4096 在

`loop.rs:5486/5597/5629/7517` + `code-index:351` **确有消费点**，归 SAFE/

NEEDS_REVIEW 而非臆断（seccomp 133 错位教训：常量名自我描述不可信，先 grep

存在性再按实际接线分类）；

② **第 6 处候选**：`estimate_chars` 的 Debug 虚增（context.rs:174）= 变相截断语义

（系统性高估 → 提前触发压缩），归 Node 01 处理，本 Node 表格标注与 Node 01 联动；

③ `context_fill_pct` 语义错位（RC40，loop.rs:1333）如 Node 02 已单列，本 Node 标记

为 FACT_RISK。

---

# 16. Node 11 — Constant Calibration Decision

综合：

```text
Node 01 token calibration
Node 02 compaction audit
Node 06 A/B
Node 07 retention
Node 08 continuity
Node 10 truncation
```

最终只允许三种结果：

### Decision A

```text
阈值明显过早
A/B 明显改善
```

→ 实施 threshold calibration。

**（v1.1 裁决，承接砺批-2）**：Decision A 的阈值实施**必须 provider-aware**（窗口

× 比例 + config 覆盖 + 已知模型表），禁止硬编码新魔法常量。回滚机制沿用 §30

（`HEARTH_COMPACTION_MODE` / `HEARTH_CONTEXT_TOKENS` 等价 feature flag），保留旧

路径至少一个 release cycle。

### Decision B

```text
阈值不是主要问题
```

→ 不改 threshold，进入 persistence architecture。

### Decision C

```text
阈值 + persistence 都有问题
```

→ 先修阈值，再最小化设计 persistence。

不得：

> 两套都一起大改。

---

# 17. Node 12 — Implement Smallest Proven Fix

根据 Node 11 决策施工。

优先级：

```text
参数修复
>
小型 integration 修复
>
Fact persistence
>
大型 memory architecture
```

最小改动原则。

每次修改：

```text
先红
→
修改
→
单测
→
gate
→
真机
```

---

# 18. Node 13 — Fact / Verification Non-lossy Integration

如果 Node 11 证明需要 persistence：

必须实现最小结构：

```text
Conversation History
    ↘
      Compaction
           ↓

Fact Store ─────────────→ Context Recovery
Verification Store ─────→ Context Recovery
```

要求：

- Compaction 不删除 Fact
- Compaction 不删除 Verification
- Resume 可以恢复
- Task Continuity 继续从事实源派生
- 不创建新的 TaskGraph
- 不创建新的 Completion truth source

---

# 19. Node 14 — Long-run Core Benchmark

至少跑两类真实任务：

## Task A

长程 Product Task：

```text
30–60 steps
```

必须包含：

```text
write
test
failure
repair
verification
compact
resume
completion
```

**（v1.1 补 S-9，守门员）受控触发压缩的手段必须先行落实**：若 Node 11 走  
Decision A（阈值上调），**自然长度的任务可能永远不再触发压缩**——届时  
Node 14 Task A 的 "compact" 环节、M3（至少一次真触发）、M4/M5 全部无法验证。  
要求：在 Node 12 施工时同步提供一个**测试专用受控触发手段**（如 env 阈值覆盖  
`HEARTH_COMPACT_CHAR_THRESHOLD` / debug 触发命令 / 单测直调 `maybe_compact`），  
并在 Node 06 A/B 与 Node 14 中用它制造确定性的压缩点。该手段只进测试路径，  
不得成为生产默认行为。

## Task B

长程 Discussion / QA：

```text
10+ turns
```

确保：

```text
QA
Conversation
Task Control
```

不会因为 context compression 重新进入 TaskGraph。

---

# 20. Node 15 — Strong Acceptance / Final Regression

最终不允许只跑：

```text
cargo test
```

必须四层验收：

### Structural

```text
fmt
clippy
tests
```

### Behavioral

```text
compact
resume
continuity
fact retention
```

### Regression

```text
R2-C
W3/W4
RC24
W8
P1-LTR
P1-TASK-TRUTH
P1-EXECUTION-DECISION
P1-FAILURE-ADAPTATION
N1-SBX
```

### Long-run

```text
至少两个真实长程样本
```

---

# 21. Memory Acceptance Hard Requirements

本轮最终必须证明：

### M1

能够准确测量真实 workload 的：

```text
characters ↔ tokens
```

### M2

能够说明当前 Compaction 为什么触发。

### M3

至少一个长会话真正触发 Compaction。

### M4

Compaction 后：

```text
original_goal
Task Continuity
```

不丢。

### M5

Compaction 后：

```text
Fact
Verification
```

不丢。

### M6

Resume 后：

```text
无需重新教育任务
```

### M7

没有通过“无限扩大 context”掩盖问题。

### M8

没有新增第二套事实模型。

**（v1.1 补 S-7，守门员）INV-M01 必须有确定性 fixture**：§2 的四条 INV-M 目前  
只有原则表述。其中 **INV-M01（Fact 不随普通 compaction 丢失）必须落为可失败的  
单测**——直调 `maybe_compact` + 构造含 goal/verification 的 history，断言压缩后  
state 层字段与 archive 原文可恢复（先红后绿）。真机长程样本是补充证据，不是  
替代——"断言不能失败 = 断言不存在"（四层递进审查推论）。INV-M02/03 属实验  
纪律、INV-M04 属范围约束，不强制单测。

---

# 22. Telemetry

本轮扩展观察项：

```text
chars_before_compact
estimated_tokens_before_compact
actual_tokens_if_available

compact_count
compact_duration

facts_before
facts_after
verification_before
verification_after

goal_revision_count
execution_turn_count
goal_revision_rate

reteach_count

fact_retention_rate
verification_retention_rate
continuity_retention_rate
```

其中：

```text
fact_retention_rate
verification_retention_rate
```

必须尽可能基于确定性状态比较，而不是模型自评。

**（v1.1 补 S-8，守门员）新 telemetry 字段的可见性边界**：RC40 教训  
（`context_fill_pct` 语义错位直接进 LLM 可见 body 误导模型）。本轮新增的全部  
telemetry 字段必须逐个声明两类去向之一：**internal-only**（默认）或  
**LLM-visible**（仅当字段能帮模型正确判断自身状态时才进 body/scratch，且须  
通过 Node 02 的命名审计）。禁止把整批新字段顺手塞进 `update_body_state`  
（loop.rs:1327）了事。

---

# 23. Goal Revision Telemetry

加入：

```text
goal_revision_count
execution_turn_count
goal_revision_rate
```

第一阶段：

> **只观察，不设阈值。**

不能看到一次：

```text
revision_rate > X
```

就擅自触发新的控制流。

---

# 24. Verification Telemetry

加入：

```text
deterministic_verification_count
model_judgment_count
deterministic_verification_ratio
```

同样：

> 只观察，不设置硬阈值。

意义是：

> 判断 Hearth 的 Verification 是否越来越依赖客观证据。

---

# 25. Interaction Surface Regression

因为 Escalate 复用已有 `InteractionRequest`，本轮顺手做一个轻量边界审计，但不得重构 Interaction Framework。



至少测试：

```text
纯情绪
闲聊
自我指涉
模糊问题
情绪 + 任务
问题 + 任务
```

检查：

```text
是否应该触发 InteractionRequest
+
触发后 option 是否完整
+
是否发生机械切句
+
是否丢语义
```

这里必须区分：

```text
Trigger correctness
```

与：

```text
Surface correctness
```

两个问题。

---

# 26. Reflection Structured Evidence Audit

本轮不设计 Persona，不禁止自然语言。

只检查：

> machine-consumed reflection 是否能够区分：

```text
observed_facts
interpretation
uncertainty
conflicts
rationale
proposed_action
```

如果当前仍是自由文本：

记录：

```text
OPEN
```

只有当它证明影响本轮 Fact/Verification continuity 时，才允许做最小结构化改造。

---

# 27. Decision / Terminal Mapping Audit

检查：

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

之间的真实映射。

必须形成一张确定性表：

| Decision | Terminal  | 是否等待用户    | 是否允许 delegation | 备注 |
| -------- | --------- | --------- | --------------- | -- |
| Continue | 实际映射      | 否         | —               |    |
| Replan   | 实际映射      | 否         | —               |    |
| Complete | completed | 否         | —               |    |
| Escalate | 实际映射      | 视 channel | 视类型             |    |
| Stop     | 实际映射      | 否         | —               |    |
|          |           |           |                 |    |

重点审计：

```text
Escalate
+
no human interaction channel
+
delegation active
```

在这种情况下系统到底怎么处理。

禁止无限等待。

同时：

> delegation 不能自动覆盖 Semantic Uncertainty。

---

# 28. Core Freeze 独立审查主体

本总包不负责最终 Core Freeze。

未来 Core Freeze 的独立评审身份已经明确：

```text
人工独立评审窗口
+
外部 AI 独立评审窗口
```

Observer OS 当前：

> 不承担 Core Freeze 最终裁判职责。

Observer 的自动化 evaluation 能力属于后续治理阶段。

---

# 29. 实验统计纪律

本轮所有 A/B：

不得使用：

```text
A1 vs B1
```

直接宣称因果。



至少：

```text
5 paired runs
```

结果报告必须给：

```text
mean
min
max
```

如果方差明显较大：

```text
扩大到 10 次
```

对关键结论写：

```text
confirmed
likely
unknown
```

不得混淆。

---

# 30. 回滚策略

如果实施了参数或架构修正：

保留旧路径至少一个 release cycle。

必须能够：

```text
HEARTH_COMPACTION_MODE=...
```

或等价机制安全回滚。

但：

> **只有确实需要 feature flag 才使用。**

不要为了未来而制造配置。

---

# 31. Final Gate

最终使用当前正确隔离门禁：

```text
~/run_gate_r2c.sh
```

严格记录：

```text
host
source
HEAD
binary
version
gate log
```

至少：

```text
FMT = 0
CLIPPY = 0
RT4_SOLO = 0
TEST = 0
```

**（v1.1 补 S-4 / S-5，守门员）**：
- **S-4 版本纪律**：凡 Node 12 落地了生产代码修复，**版本 bump（v0.2.16）与
  CHANGELOG 必须在 Final Gate 之前完成**（先 bump 后 gate 是既定约定——gate
  记录的 binary version 必须是发版号，不是旧号）；Final Report §8 Changes 须与
  CHANGELOG 逐条对得上。
- **S-5 证据归档**：本轮全部真机/A-B 原始日志、gate log、criteria 冻结件统一
  归档 `docs/data/memory-context-20260830/`（Node 00 既有目录，**禁 /tmp**——
  B04/B06 + 57GB 教训）；gate log 按"门禁日志登记约定"写明完整 VM 路径
  （如 `.133:/home/wutao/t_gate_mc_final.log`），Final Report 的每个结论行须能
  反查到归档文件。

---

# 32. Final Report

最终一次性提交：

# `P2-MEMORY-CONTEXT-01 Final Report`

包含：

## 1. Executive Summary

```text
PASS
PASS WITH DEVIATIONS
STOP
```

## 2. Token Calibration

真实 chars/token。

## 3. Compaction Audit

真实触发规则。

## 4. A/B Evidence

5～10 paired runs。

## 5. Fact Retention

before/after。

## 6. Verification Retention

before/after。

## 7. Goal Continuity

compact/resume。

## 8. Changes

逐文件。

## 9. Tests

先红后绿。

## 10. Real-machine

至少两个长程任务。

## 11. Regression

历史所有 Core 主线。

## 12. Telemetry

新指标。

## 13. OPEN / UNKNOWN / DEFER

完整列出。

## 14. Provenance

双 VM。

## 15. Final Decision

明确：

```text
parameter calibration
vs
architecture repair
```

---

# 33. 成功标准

本轮不是：

> “Compaction 能运行。”

而是：

# **压缩之后，Hearth 仍然知道真正发生过什么。**

至少证明：

```text
长会话
↓
触发 compact
↓
事实仍在
↓
Verification 仍在
↓
Goal 仍在
↓
Task Continuity 仍在
↓
resume
↓
无需重新教育
↓
继续执行
↓
最终完成
```

---

# 34. 本轮最重要的取舍纪律

如果 Node 06 A/B 已经证明：

```text
threshold calibration
```

就能解决绝大多数 continuity 问题：

> **停止。**

不要继续造复杂 Memory System。

如果 Node 06 证明：

```text
threshold 改善有限
+
Fact / Verification 仍丢失
```

才继续：

```text
Fact Persistence
Verification Persistence
```

如果最终证明：

```text
参数
+
最小 persistence
```

已经解决：

> **停止。**

不要继续扩展 Memory。

---

# 35. 最终蓝图位置

本轮在整个 Hearth Core 中的位置：

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
TERMINAL
    ↓
PROJECTION
    ↑
    │
MEMORY / CONTEXT
    │
    └── 保证上述事实跨时间持续存在


FAILURE ADAPTATION
    └── 已完成
```

因此 Memory 的职责不是：

> 创造新的“记忆人格”。

而是：

> **保证已有事实和任务连续性不会因为上下文生命周期结束而消失。**

---

# 36. 总体停止条件

当以下六项全部达到：

```text
1. Context token calibration 已实测
2. Compaction 行为已可解释
3. Fact retention 已实证
4. Verification retention 已实证
5. Compact + Resume continuity 已实证
6. Long-run strong acceptance 已通过
```

本总包结束。

不要因为发现：

```text
未来可以做 semantic memory
未来可以做 personality memory
未来可以做 episodic memory
```

而扩大范围。

这些全部进入：

```text
Ideas Register / Quarantine
```

等待 Core Freeze 之后再处理。

---

# 37. 最终原则

Hearth Memory 的目标不是：

> **记住一切。**

而是：

> **不忘记重要的事实。**

不要求：

```text
所有聊天永久保存
```

只要求：

```text
影响任务状态的事实
+
影响验证结论的事实
+
用户明确的任务锚点
```

在：

```text
Compaction
Resume
Long-running
```

之后仍然可以被可靠恢复。

---

# 38. 执行模式

本文件批准后：

```text
Node 00
→ Node 01
→ ...
→ Node 15
→ Final Gate
→ Final Report
```

连续执行。

执行窗口不得：

```text
完成 Node 01 → 等用户
完成 Node 03 → 等用户
```

普通失败自行处理：

```text
diagnose
→ fix
→ test
→ retry
```

只有 STOP-1～8 才暂停。

---

# 39. 最终使命

完成这一轮后，Hearth Core 应该进一步接近：

```text
我知道我要做什么
        ↓
我知道我做了什么
        ↓
我知道什么是真实发生的
        ↓
我知道什么已经被验证
        ↓
我知道失败为什么发生
        ↓
我知道如何恢复
        ↓
我即使压缩上下文
也不会丢掉关键事实
        ↓
我即使重新启动
也不需要用户重新教我整个任务
        ↓
我最终可以可靠地说明：
“现在到底完成了吗？”
```

**本总包唯一真正的目标：**

# Truth survives time.

也就是：

> **让 Hearth 的"事实"能够跨越上下文生命周期继续存在。**

---

# 附：砺·评审批注（2026-08-30，总判断：🟡 有条件通过——按批-1/批-2 纳入后放行）

> 审计基线：FA01 评审包（v0.2.15 `ceb62b0`）源码锚点本窗全实证——FailureKind 10 类纯函数（terminal.rs:252-352+测试 403-490）、T4 停滞拦截（loop.rs:2542/2561）、预算 headless 拦截（4477）、giveup_unverified ×3（2576/4173/4487）+ Error 相位 error_detail 投影（4840-4841）、ToolResult.error_kind 结构化通道（agent-types:261/275 + scheduler 单/并行两路 downcast）、deadline 旁路豁免（4407-4439 无 Reserve，Safety>Deadline 落码）、F9 正反测试（7229-7337）。gate 437/0 与真机日志在 VM 侧（.133/.131），本窗未复跑——证据边界如实声明。

## 批-0（FA01 终态裁定建议，请顶层确认）

**维持 PASS WITH DEVIATIONS 并 CLOSED。** §26 分层归因条款执行正确：Node 09 calcpkg 3 跑未收敛如实披露且系统未假报 completed（r3 独立复验 lib 编译失败→criteria 必败），机制层两个真机闭环（Node 12 r3 mathlib 21 步 / Node 13 wordcount 46 步）+ 上轮 shout 假阳性被拦截正确拒绝（Node 12 r1）——硬升 PASS 会抹掉这份诚实。OPEN 6 项中两项已入总账（RC45 verify_replan_count 双上限零和 / RC46 bash exit code 未接 ToolResult）。

## 批-1（🔴 必读先验：Node 01/02 不是从零发现，是从已实测事实复核）

以下源码事实为砺窗在 v0.2.8-0.2.13 期间实测（含联网业界调研交叉验证），Node 01/02 的任务是**在 v0.2.15 复核确认**，不是重新考古：

1. `context.rs:169 COMPACT_CHAR_THRESHOLD = 32_000`——**字符不是 token**、固定值、provider-unaware（INV-M02 要求的"先标定"此前从未做过）；
2. `context.rs:174-181 estimate_chars` 用 **Debug 格式化**计数——**测量仪器本身虚增**（与 RC6 同源）。批-3 详见；
3. `llm-cn:412 max_context_tokens=32_000` 写死、provider 表**无 Agnes**——Node 02 "provider-aware?" 的答案先验 = **否**；
4. 用户已确认 Agnes 512K：32k 字符 ≈ 21k-53k tokens = **真实容量的 4-10%**——"过早压缩"方向性成立，但精确数字按 INV-M02 实测，禁预写；
5. `summarize_turn` **纯规则模板不调 LLM**、Assistant 全文+tool_result 100% 丢弃、压缩摘要 **60 字墓碑**（context.rs:310）——Node 03 BEFORE/AFTER 的关键先验（RC5/RC37 的机制本体）；
6. **RC40**：`context_fill_pct = total_chars / 32k × 100`——衡量"距压缩阈值的距离"却注入为"上下文填充率"（用户实测 83%→65% 回落当场质疑）。**Node 02 必答项**：这不是 token 校准问题，是**语义错位**，改阈值不治它；
7. `archive_compacted_turns` **已落盘**——可逆压缩雏形已在。Node 04 情况 A/B 判定时先盘点它，**能复用就不新建**（对齐 M8）。

## 批-2（🔴 设计裁决预声明：Decision A 的阈值不得写成新魔法常量）

若 Node 11 走 Decision A（threshold calibration），**候选阈值必须 provider-aware 表达**（窗口 × 比例，如 60%），且经 config（如 `HEARTH_CONTEXT_TOKENS` 手工覆盖 + 已知模型表）而非硬编码。否则等于为 Agnes 512K 标定出一个新常数，切 DeepSeek/local 32k 时**再坏一次**——R-δ（常量标定失效）的老路。配套设计 = 设想登记册 I-6（model_caps 模块：`/v1/models` 列表 + 已知表 + 探针缓存 + 手工覆盖），设计稿已在 `docs/design-ideas-best-practice-assessment.md` §2.6。**本轮只取最小切片**（表+覆盖+一个比例配置），不实施完整自动发现。

## 批-3（🟡 Node 01 仪器纪律：先校准测量仪器，再做校准）

`estimate_chars` Debug 格式化虚增意味着：当前 32k 阈值的真实含义是"约 20k-53k tokens 的某个未知点"——**在虚增仪器上做 chars↔tokens 校准，校准结果继承了仪器的偏差**。要求 Node 01 第一小节先量化 estimate_chars 的偏差（对四种 corpus 比对 Debug 计数 vs 朴素 chars 计数），校准表分列"原始 estimate 口径"与"修正口径"两套数字。另：**provider usage（response 的 prompt_tokens）优先于任何本地估算**——真实 API 反推是唯一权威口径，本地估算只做交叉验证。

## 批-4（🟡 Node 05/06 A/B 纪律：层归过滤）

A/B 每条件 5-10 次长程 LLM 任务（30-60 步/跑），Agnes 配额充足但方差主要来自 **model 层**（FA01 Node 09 已实证：同任务 3 跑层归各不相同）。要求：A/B 结论**只消费 mechanism/decision 层稳定的跑次**；model 层失败按 FA01 层归纪律单独列示，不得计入 threshold 效应，否则 A/B 会把模型方差误读成参数效应。终止态对比同理——先分层再比较。

## 批-5（🟡 Node 10 截断清单核对与第 6 处候选）

总包列的 200/60/8000/6000/4096 与砺窗 W13 常量体检清单一致（render 工具结果 200 / context 摘要 60 / web 正文 8000 / constitution 6000 / 子 agent max_context_tokens 4096，落点 loop.rs:4796/4907/4939、code-index:351）。补两点：

1. **第 6 处候选**：`estimate_chars` Debug 虚增 = 变相截断语义（系统性高估 → 提前触发压缩），归 Node 01 处理，Node 10 表格标注联动；
2. 4096 一处**待查是否真被消费**（可能只是能力声明未接入截断路径）——审计存在性（grep）后按实际接线分类，勿按常量名臆断（seccomp 133 错位教训：常量名自我描述不可信）。

## 批-6（🔵 Node 08 "继续"复现先验）

修 B 后 `classify_user_input` 有疑问句类别（loop.rs:376-385），"继续"在 TASK_CONTROL 白名单——v0.2.15 上"继续"→ GoalMutation 的历史形态应已消失。若 Node 08 复现实验仍见 GoalMutation，先查是否压缩邻接变量（历史证据：失败落在压缩点 200 行内 53-60%），勿直接怀疑分类器回归。

## 批-7（🔵 FA01 移交项不越界）

RC45（verify_replan_count 双上限零和）与 RC46（bash exit code 未接 ToolResult）**不属 P2 范围**——已入总账待独立单。P2 执行中若 compaction 审计顺带撞见，只登记不修复（范围纪律，STOP-8 同款）。

## 批-8（✅ 合格确认）

- 四问框架（Q1 过早压缩 / Q2 归因 / Q3 事实丢失 / Q4 参数 vs 架构）与砺窗 I-3 评估（"何时压 vs 压成什么"两问，B 层更关键）同构，且 Q3/Q4 的 A/B 先行设计比砺窗建议更严谨（5-10 配对 + 方差扩样）；
- INV-M03（可用上下文≠窗口）正确防住了"直接把 32k 改成 512K"的偷懒解；
- §34 取舍纪律（threshold 够用就停）与"先少破坏→再破坏得少→最后能找回"原则一致；
- STOP-8（静默截断影响事实安全时停诊断边界）覆盖到位。

**放行条件**：批-1（先验采纳，Node 01/02/03 按复核而非发现执行）与批-2（Decision A provider-aware 预声明）写入执行窗口开工前提；批-3/4 为 Node 内纪律；其余观察。守门人零代码改动。

---

# 附 2：守门员复核定稿批注（2026-08-30，总包 v1.1 定稿依据）

> 定位：砺批-0~8 已审（附 1），本节是**对审计的审计**（锚点实测）+ 顶层定稿裁决。
>
> 实测基线：本机 HEAD `7f6319c`（v0.2.15 tag `ceb62b0` 之后含 docs 提交；P2 执行基线 = v0.2.15 `ceb62b0`）；P1 评审包证据全部以 v0.2.15 源码复核。


## 复-1 砺批-1 七项先验锚点全实证（confirmed）

| # | 声明                                                            | 实测                                                                           |
| - | ------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| 1 | `COMPACT_CHAR_THRESHOLD=32_000` 字符非 token、固定、provider-unaware | ✅ context.rs:169（注释自承 `≈8k tokens`）                                          |
| 2 | `estimate_chars` Debug 格式化虚增（`{:?}` 包 `String("...")`+转义）     | ✅ context.rs:174-181，系统性高估约 2×                                               |
| 3 | `llm-cn` provider 表无 Agnes 条目                                 | ✅ grep 全库 Agnes 仅 config.toml 通道；HunyuanProvider `max_context_tokens=32_000` |
| 4 | 32k 字符 ≈ 真实容量 4-10%（过早压缩方向性）                                  | ✅ 注释 `≈8k tokens` + Debug 虚增 → 真实触发点约 16k tokens 级 = Agnes 512K 的 ~3%        |
| 5 | `summarize_turn` 纯规则 60 字墓碑、不调 LLM、折叠全文                       | ✅ context.rs:312-323（中文字节切 panic 已修为 `chars().take(60)`）                     |
| 6 | **RC40** `context_fill_pct=total/32k` 语义错位（冒充填充率）             | ✅ loop.rs:1333-1346 注入 LLM 可见 body + status 工具；改阈值不治                         |
| 7 | `archive_compacted_turns` 已落盘（可逆压缩雏形）                         | ✅ context.rs:189-211 先归档再折叠、摘要含归档检索提示                                        |

结论：**批-1 的"先验采纳"成立且措辞准确**，Node 01/02/03/04 按此执行而非重新考古——已内联为硬条款。

## 复-2 P1 上游收口核对（来自评审包 `hearth-p1-failure-adaptation-01-review-pack-v1.md`）

- P1-FAILURE-ADAPTATION-01 终态 **PASS WITH DEVIATIONS / CLOSED**，v0.2.15 `ceb62b0`，gate 437/0 四 RC=0（本窗已在上一轮核验双 VM 0.2.15，一致）。
- 评审包 §14 给出 6 条可独立执行的核验命令（十类 taxonomy / 三拦截路径 / F9 标记 / 结构化通道 / deadline 旁路 / 真机日志）——**可作为 P2 开工前的历史主线复验清单**，建议 Node 15 回归时复用其 §14-5/6。
- 评审包 OPEN 6 项：RC45（verify_replan_count 双上限零和）、RC46（bash exit code 未接 ToolResult）**不属 P2 范围**（砺批-7），已入总账待独立单；P2 撞见只登记不修复（范围纪律，STOP-8 同款）。
- P1 的层归纪律（mechanism/decision/model 三层）已被批-4 继承进 P2 A/B——一致。

## 复-3 设计裁决与放行

1. **批-2 接纳（🔴 必遵）**：Decision A 阈值必须 provider-aware（窗口×比例 + config 覆盖 + 已知表），禁止新魔法常量；已写入 Node 05/11。配套最小切片 I-6 仅取表+覆盖+比例配置。
2. **批-3 接纳（🟡 仪器纪律）**：Node 01 先量化 estimate_chars 偏差再校准，校准表分"原始/修正"两套；provider usage 优先于本地估算。已写入 Node 01。
3. **批-4 接纳（🟡 层归过滤）**：A/B 只消费 mechanism/decision 层稳定跑次，model 层单列。已写入 Node 06。
4. **批-5 接纳（🟡 截断清单）**：第 6 候选 = estimate_chars 虚增；4096 已确认有消费点归 SAFE；RC40 标 FACT_RISK。已写入 Node 10。
5. **批-6 接纳（🔵 先验）**："继续"→GoalMutation 历史形态应已消失，复现先查压缩邻接变量。已写入 Node 08。
6. **批-7 接纳（🔵 不越界）**：RC45/RC46 只登记不修。已写入 §0 + 附 1。
7. **批-8 合格项**确认（四问框架 / INV-M03 / §34 取舍纪律 / STOP-8 覆盖）。

## 复-4 守门员补充（非重复，新增十条）

- **S-1（telemetry 命名闭环）**：Node 10 探明的 RC40 `context_fill_pct` 若决定改名
  `compact_pressure_pct`，须同步改 `tools-builtin/src/status.rs:91/101` 的硬编码示例
  与断言（status.rs:101 `assert_eq!(v2["context_fill_pct"], 62)`）——否则单测与
  真机 telemetry 字段名不一致会回归。Node 02 改名决策须连带列 status.rs 改动。
- **S-2（archive 复用的边界声明）**：Node 04 复用 `archive_compacted_turns` 满足
  M5/M8 的**前提是**归档路径在 cgroup/sandbox 下可写（回顾 RC23：env 没传
  `HEARTH_CGROUP_BASE` 时 base 落 root `/sys/fs/cgroup` 写失败仅 warn）。Node 04
  必须实测归档在本机/VM 真实落盘（best-effort 失败的 warn 不等于 Fact 已持久化），
  不得把"代码里有 archive 调用"当成"Fact 已持久化"证据——这是 F 列"自述≠事实"的
  又一实例。
- **S-3（存储位置分类前置，Node 03）**：压缩只 drain `state.history`（context.rs:201），
  goal/TaskGraph/acceptance_*/scratch 均在 state 层**不在压缩路径**。Node 03 逐项
  清单必须先标存储位置（history/state/scratch/archive）再对比——禁止把不在
  history 的字段判成"压缩后丢失"（假阳性会把 Node 04 误推向情况 B，违反 M8）。
- **S-4（版本纪律，§31）**：Node 12 落地生产修复则 v0.2.16 bump + CHANGELOG 必须在
  Final Gate **之前**完成（先 bump 后 gate 既定约定）；Final Report §8 与 CHANGELOG
  逐条对账。
- **S-5（证据归档，§31）**：全部真机/AB 日志、gate log、criteria 冻结件统一归档
  `docs/data/memory-context-20260830/`（禁 /tmp）；gate log 按"门禁日志登记约定"
  写明完整 VM 路径，Final Report 结论行可反查归档。
- **S-6（"不丢"分两档口径，Node 07）**：A 档=上下文内保持（无工具直答），
  B 档=经 archive grep 可恢复。三 retention accuracy **分档报告**；state 层事实
  （Goal/TaskGraph）应达 A 档 100%，tool result 类允许 B 档但须实证"模型真实
  完成过至少一次 grep 检索"（日志留痕），不得把 B 档算成 A 档。
- **S-7（INV-M01 确定性 fixture，§21）**：INV-M01 必须落为可失败单测（直调
  `maybe_compact` + 断言 state 字段与 archive 原文可恢复，先红后绿）；真机样本是
  补充非替代——"断言不能失败=断言不存在"。
- **S-8（telemetry 可见性边界，§22）**：新 telemetry 字段逐个声明 internal-only
  （默认）或 LLM-visible（须过 Node 02 命名审计），禁止整批塞进
  `update_body_state`（loop.rs:1327）——RC40 语义错位直进 LLM 可见 body 的教训。
- **S-9（受控触发压缩手段，Node 14）**：若 Decision A 上调阈值，自然任务可能
  永不再触发压缩 → M3/M4/M5 与 Node 14 的 compact 环节全部无法验证。Node 12
  须同步提供测试专用触发手段（env 阈值覆盖 / debug 触发 / 单测直调），只进
  测试路径，不作生产默认。
- **S-10（A/B 交错配对，Node 06）**：A/BAB… 交错执行防 Agnes 时段效应污染
  对比（FA01 r3 十次 backoff 实证），每跑记录时间戳（衔接批-4 层归）。

## 复-5 定稿裁决

**本总包 v1 → v1.1 定稿，放行执行。** 砺批-0~8 全部采纳且已内联为 Node 正文硬性
条款（Node 01 先验采纳 / Node 02 RC40 单列 / Node 04 archive 复用 / Node 05 provider-
aware + I-6 / Node 06 层归过滤 / Node 08 继续先验 / Node 10 第6候选+4096 消费性 /
Node 11 Decision A 裁决）；守门员补 **S-1~S-10 十条**（详单见复-4，落点：Node 03 /
Node 06 / Node 07 / Node 12+14 / §21 / §22 / §31）。不新增 STOP、不翻 P1 已 CLOSED
事实；INV-M 系列编号独立（不与 P1 的 INV-FA01-* 冲突，Final Report 继续用前缀
区分 `INV-ED01-`/`INV-FA01-`/`INV-M-`）。守门人零代码改动。
