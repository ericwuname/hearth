R2-C 顶层评审结论：有条件通过设计，先做 Evidence Closure，暂不施工 ContextBuilder

已审阅你提交的：

`Hearth R2-C 设计单：Context Builder / Intent Understanding + Cache Evidence + Resource Safety`

以及 R2-D、R1 审计结论。

当前结论：

> **R2-C 设计方向基本成立，但暂不进入 ContextBuilder 大规模施工。**
>
> 不是打回重做，也不是推翻现有设计。
>
> 下一步先完成一个小而严格的 **Evidence Closure** 阶段，把当前几个“强嫌疑”与“已证实事实”彻底分开；证据闭合后，再出正式的 ContextBuilder Implementation 施工单。

---

# 一、先确认当前基线

当前有效基线：

* Hearth v0.2.7
* commit `46ed8f6` 之后的 R2-C 最新工作树，以实际 HEAD 和 VM 为准
* R2-D TaskGoal 已完成
* R2-C 当前已完成：

  * TelemetryProvider v2
  * ResourceLedger Observe 层
  * Context Assembly 源码审计
  * Intent benchmark 设计
  * Resource 事故复盘
* ContextBuilder 本体尚未施工

禁止重复建设已经完成的能力。

---

# 二、本轮不是修代码，先做 Evidence Closure

新增一阶段：

```text
R2-C-Evidence-Closure
```

目标只有一个：

> 把“为什么 Hearth 不如 Codex 好用”从主观体感/强嫌疑，进一步压缩成可复现、可定位、可施工的证据。

完成后再进入：

```text
R2-C ContextBuilder Implementation
```

---

# 三、Evidence Closure-01：Cache v2 真机数据必须完成

R2-C 当前已经确认：

```text
system_hash：
20 requests → 7 个不同值
```

这只能证明：

> system_text 存在不稳定。

**不能直接证明：system_text 不稳定就是 provider cache miss 的头号根因。**

因此现在必须补完 v2 真机数据。

至少：

```text
30–50 requests
```

覆盖：

```text
普通多轮
plan
act
observe
reflect
tool call
resume
compact 前后
```

每次记录：

```text
request_id
session_id
phase
request body hash
system hash
tool schema hash
dynamic suffix hash
prompt tokens
cache hit
cache miss
provider cache metadata
```

然后给出：

```text
Cache hit rate =
实际 cache-hit requests / 可观测 requests
```

但注意：

如果 provider 报告的是 token-level cache，而不是 request-level cache，不得擅自把两个指标混为一谈。

必须明确：

```text
request cache metric
token cache metric
```

分别是什么。

---

# 四、Evidence Closure-02：Cache miss 必须做根因分类

最终不能只给：

```text
hit rate = xx%
```

必须分类：

```text
MISS-A：
system_text 变化

MISS-B：
tool schema 变化

MISS-C：
history / dynamic suffix 变化

MISS-D：
serialization 差异

MISS-E：
provider cache policy / minimum prefix

MISS-F：
其他
```

最终给出：

```text
证据等级：
confirmed
likely
unknown
```

特别禁止：

```text
“TaskGraph 导致 cache miss”
```

除非 provider-level 数据真正支持。

当前只能说：

> TaskGraph 是稳定前缀污染的强嫌疑源。

---

# 五、Evidence Closure-03：Intent Understanding Benchmark 最小版直接执行

R2-C 现在最大的核心问题不是“有没有某个 Codex 命令”，而是：

> 为什么同一个模型在 Codex 中似乎更容易理解我的意图，而在 Hearth 中我需要说得更详细。

因此必须做最小真实 benchmark。

不要求 25 个全部一次完成。

先做：

```text
5 个任务
```

每个任务尽量同时跑：

```text
A：
原始用户输入

B：
当前 Hearth

C：
当前 Hearth + Task Continuity

E：
同模型 Codex
```

如果资源/预算允许，再补：

```text
D：
Context 精简后的 Hearth
```

任务类型至少覆盖：

```text
1. 歧义指令
2. 多步工程目标
3. 隐含约束
4. 中途发生失败
5. 纯问询/控制指令
```

记录：

```text
intent preservation
plan alignment
tool selection
unnecessary assumptions
goal preservation
goal drift
human intervention
completion
```

重要：

不要让 benchmark 变成“哪个模型回答得更像人”。

要测：

> **哪个 Harness 更能把用户真正想做的事情稳定转化为正确行动。**

---

# 六、Evidence Closure-04：TaskGraph 不要直接一刀切搬到 Dynamic Tail

当前 R2-C 设计：

```text
TaskGraph
↓
Task Continuity
↓
dynamic tail
```

原则成立，但请先补一个设计比较：

### 方案 A

整个 TaskGraph 动态化。

### 方案 B

拆成：

```text
Task Graph Topology
= stable / task-stable

Task Graph State
= dynamic
```

需要分析：

```text
token cost
cache stability
模型理解
实现复杂度
```

建议优先评估 B。

因为：

```text
node id
description
deps
```

往往是结构事实；

而：

```text
status
result
completed
remaining
next
```

才是高频变化事实。

如果 B 明显更优，再进入 ContextBuilder 施工。

如果没有证据支持 B，不要为了形式拆分。

---

# 七、Evidence Closure-05：Experience 也做稳定性分类

不要简单把：

```text
experience
→ dynamic
```

全部定死。

分析：

```text
Experience selected for current task
```

是否在整个 task 生命周期里保持稳定。

可能存在：

```text
Project-stable
Task-stable
Turn-dynamic
```

不同类别。

重点：

> 已选定、且任务期间不会变化的 Experience，不应每轮无意义地改变 cache prefix/suffix。

最终给出证据和建议。

---

# 八、Evidence Closure-06：57GB 事故继续向下追一层

目前已经确认：

```text
resource-monitor
```

只做系统级：

```text
memory
disk free
```

而：

```text
ResourceLedger
```

开始记录：

```text
per-call
per-tool
total bytes
```

这一方向正确。

但现在还没有真正证明：

> **57GB 到底是 Tool Payload Explosion，还是 Filesystem Artifact Explosion，还是二者同时发生。**

必须继续定位。

重点检查：

```text
bash
format_output
stdout
stderr
ToolResult
history
write_file
generated files
temporary files
resource-monitor
```

特别检查：

```text
bash → stdout → String → ToolResult → history
```

是否可能让巨量输出同时进入：

```text
memory
disk
context
token
```

---

# 九、Evidence Closure-07：ResourceLedger 不能假定等价于磁盘写入

当前 ResourceLedger 主要统计：

```text
args + result bytes
```

请明确区分：

```text
Tool Payload Bytes
```

和：

```text
Filesystem Artifact Bytes
```

例如：

```text
bash
参数很小
stdout 很小
但脚本实际生成 50GB 文件
```

这种情况下：

```text
ResourceLedger
```

可能并不能看到真正的 50GB。

所以请设计：

```text
Tool payload accounting
+
Filesystem write accounting
```

是否需要两个独立观测维度。

本轮先做 Observe / Design。

---

# 十、Evidence Closure-08：bash 输出截断必须进入设计，但先不要擅自做 D 类控制

当前已经发现：

```text
bash format_output
```

无输出上限，stdout 可能全量进入：

```text
String
↓
ToolResult
↓
history
```

这可能同时影响：

```text
resource
context
token
latency
cache
```

因此需要给出设计：

```text
max_stdout_bytes
max_stderr_bytes
head/tail strategy
truncation metadata
artifact reference
```

但本轮：

**只设计/观测，不自行实现“超限即 kill / pause / deny / terminate”。**

任何这种控制流均属于：

```text
WP-0 D 类
```

必须另出顶层施工单。

---

# 十一、Goal Revision 三分类保留设计，但暂不施工

继续保留：

```text
User Input
├── Goal Mutation
├── Task Control
└── Conversation / Question
```

并特别解决已经发现的反例：

```text
“查看状态”
```

不应自动造成：

```text
goal_revision++
```

第一版可以继续采用：

```text
Task Control：
高置信规则集

Conversation：
高置信问询

其余：
默认 Goal Mutation
```

但本轮不要修改 `apply_turn_goal` 控制流。

这是 D 类。

先把分类语义和测试方案写清楚，等顶层另出施工单。

---

# 十二、Bridge 结论保持不变

```text
Bridge
=
INTENDED
=
DEFER
```

re-entry：

```text
B4-2 Human OS / Desktop 真流
```

本轮不施工、不删除。

同时记录：

> 如果未来证明 B4-2 不再需要 bridge，则重新评估为 ABANDONED 并清理依赖。

---

# 十三、当前不要做的事情

本轮禁止：

```text
完整 ContextBuilder implementation
大规模重写 build_messages()
Goal Drift 强制暂停
Approval Policy Matrix
same-effect-repeat 的控制动作
bash 超限强制 kill
filesystem quota 强制 deny
Subagent
Fork
完整 MCP
大规模 TUI
Bridge implementation
```

---

# 十四、Evidence Closure 最终交付文件

建议创建：

```text
docs/hearth-r2c-evidence-closure.md
```

必须包含：

### A. Cache

```text
30–50 request raw/summary table
hit/miss
miss classification
provider cache behavior
stable-prefix findings
```

### B. Intent

```text
5 task benchmark
Hearth vs Codex
```

### C. Context

```text
TaskGraph topology/state analysis
Experience stability analysis
Task Continuity budget analysis
```

### D. Resource

```text
57GB incident
Tool Payload vs Filesystem Artifact
bash output path
ResourceLedger coverage
resource-monitor coverage
```

### E. Architecture

```text
ContextBuilder final recommendation
Goal Revision recommendation
Resource Safety recommendation
```

### F. Decision

每一项：

```text
ADOPT
ADAPT
DEFER
QUARANTINE
```

并注明：

```text
Evidence:
confirmed / likely / unknown
```

---

# 十五、Evidence Closure 的通过标准

只有同时满足以下条件，R2-C 才可以进入正式 ContextBuilder 施工：

```text
1. Cache v2 数据已完成
2. Cache miss 已分类
3. Intent benchmark 已跑最小集
4. TaskGraph stable/dynamic 方案已有证据选择
5. Experience stable/dynamic 已完成判断
6. 57GB 事故至少完成 payload/filesystem 二分
7. bash output explosion 有明确诊断
8. ResourceLedger 的观测边界已经明确
9. Goal Revision 分类设计没有语义冲突
10. 所有 D 类控制流均明确列出
```

---

# 十六、最终目标

这一阶段不是为了“马上把代码改漂亮”。

目标是把我们现在的几个体感：

```text
“为什么 Hearth 不如 Codex 好懂？”

“为什么它干着干着像忘了原任务？”

“为什么 cache 不高？”

“为什么长任务不放心？”

“为什么一个 Agent 能写爆 57GB？”

“为什么它有很多状态，但用户不知道到底发生了什么？”
```

全部转换成：

```text
可观察
可复现
可定位
可施工
可验收
```

然后才进入下一阶段。

---

# 十七、最重要的施工纪律

本轮请继续遵守：

```text
用户体感
↓
源码事实
↓
证据
↓
根因
↓
设计
↓
顶层批准
↓
施工
↓
负面测试
↓
VM 真机
```

任何一个步骤缺失，都不要跳到下一步。

尤其禁止：

```text
“我认为这样更好”
→
直接修改 Runtime
```

以及：

```text
“Codex 就是这样”
→
直接复制实现
```

Hearth 的目标不是成为 Codex fork。

目标是：

> **吸收 Codex 已证明有效的 Harness 原则，在 Hearth 自己的架构中形成更低摩擦、更稳定、更可委托的 Agent Runtime。**

完成 `hearth-r2c-evidence-closure.md` 后停止，等待顶层评审，不要自行进入 ContextBuilder 大规模施工。

---

# 守门员批注（2026-08-28 · 与 Claude Tier3 可靠性任务书联动裁决，与正文同效力）

> 同期 Claude 产出了《Hearth 长程自主任务可靠性抢修任务书 v1.1》（Tier3 真实遥测 0/10：3 挂起 + 7 失败），其 T2/T3 将修改 `llm-openai` 重试结构与 `llm-gateway/types.rs` 错误分类——**与本 Evidence Closure 的采集器 v2 是同一批文件**。以下 5 条为两线协调裁决。

## 补充 1（顺序裁决·最重要）：Cache v2 采集（EC-01）必须在 Tier3 T2/T3 修复合入之后

理由：①T2 改 provider 重试结构，机制未稳先采数 = 采在即将作废的机制上；②**嵌套 3× 重试本身污染 request-level cache 指标**（同一请求 90s 窗口内重复 3 次，hit/miss 全歪）；③采集器 v2 钩子与 T2 动刀的是同一层（llm-openai/llm-gateway）——**合并动刀：T2 改造时顺手落 v2 hook，一次过门禁**，别两次改同一层。修订后的排期：

```text
Tier3 T1（现场留存）→ T2/T3/T4（重试权威/分类/停滞检测 + 顺手落 v2 hook）
        ↓ commit
Evidence Closure-01（30-50 请求采集，机制已稳）→ EC-02 分类 → EC-03 Intent benchmark
```

## 补充 2：Intent benchmark 的排期与配额口径

EC-03 的 5 任务 × A/B/C/E 三至四变体 ≈ **15-20 次完整 hearth 任务跑**，再叠加 Tier3 T6 的 10 轮回归 + T7 的 10 轮工具组合遥测——Agnes 配额够（75k/周），但**VM 与采集窗口全部串行排期**，禁止两条线同时压 VM（57G 事故后纪律）。benchmark 变体 C（精简 context）依赖 ContextBuilder 分析产出而非实现——允许用"手工删减版 prompt"近似，不必等施工。

## 补充 3：§8 57GB 复盘与 Tier3 T1 联动

Tier3 T1（故障现场留存到 `docs/incidents/2026-08-28-tier3/`）先行，EC-06/07 的 57GB 复盘**直接消费那批一手材料**（含 `taskgoal.json`/`graph.json`/telemetry jsonl）——两个证据阶段共用同一批现场，别各采各的。bash stdout→String→ToolResult→history 链路检查从 `bash.rs format_output` 锚点入手。

## 补充 4：§11 "查看状态"反例闭环

R2-D 真机误报（"查看状态"→revision 3，`t_revision.log`）已列入 R2-C 设计输入（前批注 4）——本 Evidence Closure 的 Goal Revision 分类设计必须给出对该误报的**规则级解法**（Task Control 高置信规则集须包含"查看状态/看 diff/发生什么了"类问询），交付时用该案例做回归断言，不许只写原则。

## 补充 5：Token-level vs request-level 指标（§3 已提，落成字段）

Agnes/OpenAI 兼容通道上报的是 **token-level** cache（`prompt_cache_hit_tokens/miss_tokens`，H6 已接线）——EC-01 的 jsonl 采集**以 token 级为主指标、request 级为派生参考**，两栏分开落盘，正文的"不得混为一谈"落成 schema 约束。
