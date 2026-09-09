R2-C 设计任务更新：Context Builder / Intent Understanding + Cache Evidence + Resource Safety

本通知基于最新项目审计、R2-D 执行结果以及前序守门员批注更新。

注意：

当前 Hearth 已不是 v0.2.3 / v0.2.4 基线。
当前有效开发基线以最新提交和实际 VM 门禁为准。

已知最新状态：

* v0.2.7
* commit `46ed8f6`
* VM fmt/clippy/test RC=0
* 370 passed / 0 failed
* R2-D TaskGoal 已施工完成
* G1 terminal / completion、G3-03 archive isolation 已完成
* 不允许重复建设已经完成的能力

上一份“功能全景审计”仍有价值，但其基线为 `f0b4a54 / 339 passed`，明显早于当前代码。它只能作为架构资产清单参考，不能作为当前实现状态权威。

---

# 一、R2-C 不立即施工，先做 Preflight

先提交设计，不直接重构 `build_messages()`。

Preflight 必须完成三件事：

## P0：当前基线重新同步

重新核实：

```text
HEAD
Cargo version
workspace crates
test count
VM gate
agent-core
agent-runtime
context
session_store
llm-gateway
resource-monitor
tool-runtime
sandbox
observer
```

特别确认前序审计中这些状态哪些已经被 v0.2.5/v0.2.7 改变。

最终产出：

```text
docs/hearth-r2c-baseline.md
```

只记录当前真实状态，不复述旧报告。

---

# 二、R2-C 核心目标不变

我们当前真正需要解决的是：

```text
同一个模型
在 Codex 上更容易理解用户意图
在 Hearth 上更容易：
- 偏题
- 误解
- 执行中失去原始任务中心
- compact 后上下文语义变弱
```

因此要研究：

```text
Prompt
Context
Goal State
Tool Schema
History
Task Continuity
Planning
Tool Result
```

不要先把责任归给模型。

---

# 三、G9 Cache Evidence 现在必须先做

在任何 ContextBuilder 重构之前：

采集至少 20–50 次真实请求。

覆盖：

```text
普通多轮
Plan
Act
Observe
Reflect
tool call
resume
compact 前后
```

记录：

```text
request body
stable prefix hash
dynamic suffix hash
prompt tokens
cache hit
cache miss
provider cache metadata
```

必须回答：

1. 当前真实 hit rate
2. miss 的具体位置
3. 哪些内容导致 prefix 不稳定
4. tool schema 是否破坏稳定前缀
5. experience / talent / Hearth.md 是否破坏稳定性
6. Task Continuity 应该放 dynamic 还是 stable
7. provider 实际 cache 规则是什么

注意：

不能用“某平台官网达到 95%”直接作为 Hearth 的目标数字。

先拿 Hearth 自己的数据。

---

# 四、Context Assembly 源码审查

重点检查：

```text
build_messages()
context.rs
experience
constitution
talent
Hearth.md
tool schemas
plan state
Task Continuity
compaction
resume
```

必须绘制当前真实结构：

```text
Stable
Project
Task
Memory
Experience
Dynamic
Recovery
Tool Definitions
History
```

每一项必须标：

```text
stable
session-stable
task-dynamic
turn-dynamic
recovery-only
```

禁止凭概念分类。

---

# 五、TaskGoal 现在已经存在，R2-C 必须复用

不要重新设计 TaskGoal。

当前已存在：

```text
original_goal
current_goal
goal_revision
constraints
acceptance_criteria
TaskGraph
Task Continuity
state_revision
```

ContextBuilder 必须消费这些既有数据。

尤其：

```text
Original Goal
Current Goal
Progress
Remaining
Next Action
Constraints
Acceptance Criteria
```

不能重新从 history 猜。

---

# 六、Goal Revision 必须在 R2-C 继续解决语义边界

R2-D 暴露出一个潜在问题：

如果：

```text
“查看状态”
“继续”
“看 diff”
“发生什么了？”
```

都被视为 goal revision，

那么 revision 会污染。

因此先做设计，不直接改控制流：

```text
User Input
├── Goal Mutation
├── Task Control
└── Conversation / Question
```

明确：

```text
original_goal
=
永久锚点

current_goal
=
当前用户任务指令

goal_revision
=
真正发生 goal mutation 时递增
```

不要让普通控制指令污染 revision。

这项如果需要改变 agent loop 控制流，属于 WP-0 D 类，必须另出顶层施工单。

---

# 七、Resource Safety：把 57GB 事故正式纳入 R2-C

R2-D 真机已经出现：

```text
21 steps
≈57GB
VM disk 100%
```

这不是普通日志事件。

它证明：

```text
Sandbox
解决：
在哪里写

但没有完全解决：
最多写多少
```

当前功能全景审计还显示已经存在：

```text
resource-monitor
```

所以本轮必须先分析：

```text
resource-monitor
+
tool-runtime
+
tools-builtin
+
sandbox
+
agent loop
```

现有资源观测到底覆盖到了什么。

---

# 八、先做 Observe，不擅自增加硬控制

第一阶段只设计/实现观测：

```text
per_file_bytes
per_tool_bytes
per_task_total_bytes
session_total_bytes
artifact_count
temporary_storage
```

并回答：

```text
57GB 是哪个工具产生的？
哪个 step 开始异常？
resource-monitor 是否看到了？
为什么没有形成 actionable signal？
```

如果后续提出：

```text
warn
pause
deny
terminate
```

属于控制流变化，按 WP-0 D 类另出顶层施工单。

---

# 九、已有资产必须优先复用

功能全景审计显示已有：

```text
resource-monitor
experience
retriever
code-index
lsp-bridge
observer
project-sync
llm-replay
```

本轮必须先判断：

```text
现有能力能否复用？
能否扩展？
是否只是没接线？
```

禁止因为任务书没有写就重新建同类能力。

尤其：

```text
Resource telemetry
Experience
Event
Report
```

都已经有现成资产。

---

# 十、Bridge 单独做状态决策，不并入 R2-C 施工

功能审计发现：

```text
bridge
```

存在：

* Cargo dependency
* `BridgeSession`
* public interfaces

但 production code 没有调用。

本轮只做：

```text
INTENDED
还是
ABANDONED
```

的判断。

如果 intended：

```text
DEFER
```

建立 re-entry 条件。

如果 abandoned：

清理无意义依赖。

禁止现在直接施工 bridge。

---

# 十一、Intent Understanding Benchmark

至少设计 5 类：

```text
A 原始用户输入
B 当前 Hearth context
C 精简 Hearth context
D Task Continuity 注入
E 同模型 Codex
```

比较：

```text
intent preservation
plan alignment
tool selection
unnecessary assumptions
goal preservation
goal drift
```

目标不是证明：

“Codex 永远更强。”

目标是定位：

> Hearth 究竟是哪一层让模型理解得更差。

---

# 十二、ContextBuilder 设计原则

候选层：

```text
Stable Prefix
Project Context
Task Context
Memory Context
Experience Context
Dynamic Turn Context
Recovery Context
Tool Definitions
```

最终结构必须由证据决定。

重点原则：

1. Goal State 不依赖 history。
2. Task Continuity 不被 compact 删除。
3. Original Goal 不被 current goal 覆盖。
4. stable prefix 尽可能字节稳定。
5. 动态状态放 dynamic 区。
6. Tool schema 不无脑全部灌入。
7. ContextBuilder 只组装事实，不建立第二套事实源。

---

# 十三、ToolInvocation 仍然不在本轮正式施工

当前已知：

```text
ToolCall
ToolResult
```

存在。

更完整生命周期：

```text
requested
approval
started
running
completed
failed
timeout
cancelled
```

留给下一轮。

但 R2-C 必须考虑：

> ContextBuilder 如何消费结构化 Tool Result。

禁止依赖 tracing 文本。

---

# 十四、Cache / Context / TaskGoal 三者必须联动评估

这是本轮最重要的组合关系：

```text
TaskGoal
↓
需要进入模型上下文

ContextBuilder
↓
决定放哪里

Cache
↓
判断是否破坏稳定 prefix
```

不能分别优化。

尤其：

```text
Task Continuity
```

必须是动态内容。

否则任务进度每一步变化都会破坏 cache prefix。

---

# 十五、R2-C 文档交付要求

创建：

```text
docs/hearth-r2c-context-builder-design.md
```

必须包含：

1. 当前有效基线
2. Context Assembly 源码事实
3. Cache 20–50 请求数据
4. Cache miss 分类
5. build_messages() 当前结构
6. Context 分层
7. TaskGoal 与 ContextBuilder 接口
8. Goal Revision 三分类
9. Intent benchmark
10. Resource monitor 当前覆盖
11. 57GB 事故复盘
12. Bridge 状态决策
13. ADOPT / ADAPT / DEFER / QUARANTINE
14. WP-0 D 类事项
15. 修改范围
16. 测试计划
17. VM 计划
18. 架构不变量影响

---

# 十六、暂不施工

严格暂缓：

```text
完整 ContextBuilder
Approval Policy Matrix
Clarify Batch control flow
same-effect-repeat 的控制动作
Subagents
Fork
大规模 TUI
完整 MCP
Bridge 本体
```

只允许在设计文档中分析。

---

# 十七、特别禁止

不要：

```text
看到 Codex 有
→ Hearth 没有
→ 直接实现

看到 prompt 很长
→ 直接重写

看到 cache 不高
→ 直接换 serializer

看到 57GB
→ 直接 kill tool

看到用户说“继续”
→ 直接修改 Goal
```

所有行为变化必须有：

```text
证据
→ 根因
→ 设计
→ 顶层批准
→ 施工
→ 负面测试
→ 真机验收
```

---

# 十八、成功标准

R2-C 设计完成后，必须能回答：

### 1

为什么 Hearth 对用户意图的保持能力不如 Codex？

### 2

Hearth 如何让 Agent 在每轮稳定知道：

```text
我最初要做什么
现在做到哪里
剩下什么
下一步是什么
```

### 3

为什么 Context Compression 不应该导致 Task Continuity 消失？

### 4

当前 cache hit/miss 的真实瓶颈是什么？

### 5

57GB 资源事故为什么没有被现有 resource-monitor 提前发现/阻止？

如果还不能回答这五个问题，不进入大规模 ContextBuilder 施工。

---

# 十九、当前优先级

当前主线：

```text
R2-C Preflight / Cache Evidence
        ↓
Context Assembly 设计
        ↓
Resource Safety 设计
        ↓
顶层评审
        ↓
ContextBuilder 施工
```

TaskGoal：

```text
R2-D ✅
```

Bridge：

```text
只决策，不施工
```

Subagent：

```text
DEFER
```

TUI：

```text
暂缓
```

最终目标仍然不变：

> 不是让 Hearth 变成 Codex。

而是让 Hearth 在自己的架构下，逐渐具备 Codex 已证明有效的低摩擦 Harness 特性，并最终做到：

```text
用户给出目标
↓
Agent 正确理解
↓
自主持续工作
↓
必要时才询问
↓
遇到失败能够恢复
↓
目标不丢失
↓
资源不失控
↓
完成后明确交付
```

**先设计，后施工。完成 R2-C 设计单后等待顶层评审。**

---

# 守门员批注（2026-08-28 · R2-D 已独立验收后补充，与正文同效力）

> R2-D 验收实证：commit `46ed8f6`/v0.2.7 在库；GoalChanged 三层接线（api/lib.rs:85、session.rs:1026、run_local.rs:750）；Task Continuity 尾部 Role::System 标签块（loop.rs:939-972/1495，**补充 1 落地原样**）；state_revision 双格式兼容；VM `~/t_gate.log` 实测 CLIPPY_RC=0 / TEST_RC=0 / **370 passed**。以下 7 条为 R2-C 执行约束。

## 补充 1（前置条件）：采集器 v2 必须先做，否则 §3 的采集范围物理上不可达

§3 要求采集覆盖 `Plan/Act/Observe/Reflect/tool call/resume/compact 前后`——但当前采集器只接在 loop 的 plan-chat 出口（`loop.rs:1907-1915`），**planner 直连 provider chat（`planner/lib.rs:246`）绕过采集**（R2-1 验收已实锤，R2-D 报告挂账 2 亦承认）。不先做采集器 v2（钩子下沉 llm-gateway/llm-openai 层，一处覆盖全部出口），"覆盖 Reflect"就是空话。**顺序钉死：v2 → 20-50 请求采集 → miss 分类 → ContextBuilder 设计。** 采集用 Agnes 通道（用户已授权预算）；`HEARTH_CACHE_TELEMETRY` 开关保持默认关零开销；撞 429 按 H4 熔断语义停。

## 补充 2：Task Continuity 位置已有裁决，勿重开决策

§3 问题 6 "Task Continuity 应该放 dynamic 还是 stable"——**批示补充 1 已裁决：history 尾部 Role::System 标签块（dynamic suffix 区）**，R2-D 已按此落地（loop.rs:939-972/1495）。R2-C 只做两件事：①用新 cache 数据**验证**"尾部块不破前缀"（拿数据背书而非重新讨论）；②ContextBuilder 把该块纳入 dynamic 区统一管理。禁止翻案。

## 补充 3：57GB 复盘的技术路径——per-write 记账的单漏斗位置钉死

§8 的观测指标（per_file/per_tool/per_task bytes）落点建议：**tool-runtime/dispatcher 是所有工具执行的单漏斗**，per-call 字节记账放这里才可能"每步 2.7G 级别"实时可见。§8 必答的"resource-monitor 为什么没信号"先查一个具体 wiring 断点：resource-monitor（105 行，相位级内存/磁盘/成本阈值→宪法触发）**大概率根本没接工具写盘路径**——它是相位体检器不是写盘哨兵，这就是答案的候选根因，设计单须实测确认而非推测。

## 补充 4：Goal Revision 三分类的实现成本红线 + 真实误报案例必作设计输入

§6 三分类（Goal Mutation / Task Control / Conversation）方向对，但**禁止用"每轮一次额外 LLM 调用"做分类**（成本+延迟双重惩罚，与 T5 降本背道而驰）。第一版用零成本规则门：控制/询问类模式（继续 / 查看状态 / 看 diff / 发生什么了 / 继续+疑问）→ Task Control，不触 revision；规则判不了的歧义输入才升级（升级机制设计时可考虑复用 clarify 通道而非新建）。**设计单必须分析 t_revision.log 里的真实误报**："查看状态"被判成 revision 3——为什么 mutation 检测没拦住它？这是判定逻辑的实测反例，比任何假想用例都有价值。

## 补充 5：tool schema 精简不是纯 cache 优化——有行为回归风险

§12 原则 6 "Tool schema 不无脑全部灌入"要注意：**schema 从请求移除 = 模型当轮无法调用该工具**（function-calling 可用性随 schema 走）。按相位裁剪 schema 时必须保证当前相位所需工具仍全部在册，且加行为回归测试（精简后任务成功率不降）——防止"cache 上去了、任务成功率下来了"这种净负优化。

## 补充 6：Bridge 状态决策的倾向性建议（供设计单裁决参考）

bridge（284 行，"前端桥接"）大概率是 **INTENDED**——它对应三端口架构里 Human OS 接真流的方向（路线图 B4-2）。建议裁决：DEFER，re-entry 条件 = B4-2 桌面接真流开工；届时若仍决定不接，改 ABANDONED 并清理依赖。设计单按此模板作答即可。

## 补充 7：R2-D 挂账与 R2-C 的衔接清单

R2-D 报告挂账 5 项，与 R2-C 的关系钉死：①criteria 写入通道（planner 扩展）= 独立单，不塞进 R2-C；②采集器 v2 = R2-C 前置（补充 1）；③ToolInvocation 派生结构 = 留下轮（§13 一致）；④crash mutation 直验 = 随 R2-C VM 验收顺手做；⑤ContextBuilder 设计 = R2-C 本体。防止执行窗口把 5 项挂账全塞进 R2-C 摊子。
