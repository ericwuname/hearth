# Hearth × Codex Harness 对标与后续优化迭代规格 v1

**文档性质**：架构对标与后续施工母任务书
**目标对象**：Hearth `hearth-rs`
**参考对象**：当前 Codex CLI / Codex App Server / Codex TUI
**当前 Hearth 基线**：v0.2.3，审查基线 `f0b4a54`；H1–H10 已形成下一轮地基修复路径
**执行原则**：不追求功能数量对齐，不整包复制 Codex；逐项吸收经过验证、能改善 Hearth 实际使用体验的设计
**核心目标**：让 Hearth 在用户真实执行任务时，逐渐达到 Codex 类 Agent 的低摩擦、高可控、高连续性体验，同时保留 Hearth 自身的白盒 Loop、Observer、G0–G3、PWC、Multi-provider、Identity/Experience 等差异化结构。

---

# 0. 核心决策

Hearth 当前不应进入“继续堆功能”阶段。

当前应该进入：

```text
现有能力
   ↓
与 Codex 的真实体验差异
   ↓
定位差异属于：
    后端 Harness
    Tool Runtime
    CLI / TUI
    Safety / Policy
    Context Engineering
    Session / Continuity
   ↓
判断：
    必须吸收
    值得借鉴
    暂不接入
    留存隔离区
   ↓
实现
   ↓
真实任务对比
   ↓
VM + 手工 + 长时间验证
```

本轮最重要的原则不是“实现更多”。

而是：

> **减少 Hearth 与 Codex 在真实任务使用中的摩擦，同时保持 Hearth 自己的架构方向。**

---

# 1. Hearth 当前基线：已经具备什么

当前源码审查已经确认，Hearth 并不是一个只有 CLI 外壳的 Agent。

已有核心：

```text
Agent Loop
Plan
Act
Observe
Reflect
Done
Error

Tool Runtime
ApprovalState

Sandbox
Landlock
Seccomp
cgroups

Memory
Experience
Subconscious
Nervous System

Observer
Project Sync
Replay

Multi-provider
Local / Remote
Session persistence
```

Hearth 的 Agent Loop 已经是白盒相位状态机：

```text
Init
Plan
Act
Observe
Reflect
Done
Error
```

且具有 `do_plan/do_act/do_observe/do_reflect` 等显式相位。

当前控制流：

```text
Goal
 ↓
Plan
 ↓
Act
 ↓
Observe
 ↓
Reflect
 ↓
Done
```

Act 阶段进入工具 Runtime、Sandbox、Approval；Observe 阶段回喂结果；最终写盘后进行 verify。

因此本轮不能再把目标定义为：

> “让 Hearth 拥有一个 Agent Loop。”

这个目标已经完成。

新的目标应当是：

> **把已有 Agent Loop 做成真正低摩擦、可委托、可恢复、可预测的工作系统。**

---

# 2. Codex 真正值得借鉴的部分

Codex 当前值得借鉴的，不是“命令比较多”。

真正值得借鉴的是以下六类设计：

```text
A. 连续 Thread / Turn 心智模型
B. Tool / Approval 的结构化生命周期
C. Sandbox 与 Approval 的正交设计
D. Context / AGENTS / Skills 的上下文工程
E. TUI 对 Agent 状态的低摩擦投影
F. Session、Fork、Queue、Compact、Review 等工作流
```

当前 Codex TUI 已经把：

```text
/compact
/new
/model
/permissions
/review
/skills
/status
/fork
/side
/init
/mcp
/personality
/rename
/diff
```

等能力做成直接服务当前工作流的交互，而不是让用户通过大量独立命令管理系统。

同时，其 app-server 协议将：

```text
thread
turn
item
approval
tool execution
compaction
```

作为结构化生命周期处理。Approval 请求会关联 `threadId / turnId / itemId`，客户端完成决策后由 Runtime 继续当前工作，而不是让用户脱离当前任务去手动管理另一个流程。

---

# 3. 第一原则：不要复制 Codex 的“长相”，复制它解决问题的方法

Hearth 与 Codex 的正确关系：

```text
Codex
  │
  ├── 证明过的工程实践
  │
  ├── UX 模式
  │
  ├── Tool Runtime 模式
  │
  └── Harness 模式
          ↓
        借鉴
          ↓
Hearth 自己实现
```

错误方式：

```text
Codex source
 ↓
整体搬运
 ↓
Hearth
```

原因：

一旦整体复制：

```text
Hearth
≈
Codex fork + 自定义功能
```

那么 Hearth 的维护成本会随着上游变化不断增长，而且会同时背负两套架构。

---

# 4. 第二原则：任何 Codex 功能先做“价值证明”

每个候选能力必须经过：

```text
WHY
 ↓
USER FRICTION
 ↓
Hearth 当前行为
 ↓
Codex 如何解决
 ↓
是否有必要
 ↓
实现成本
 ↓
长期维护成本
 ↓
最终决定
```

决策只能是四种：

### ADOPT

直接吸收设计思想并实现。

### ADAPT

吸收原则，但用 Hearth 自己的架构实现。

### DEFER

当前不做，保留设计接口。

### QUARANTINE

不接入运行主线，但保留实现/实验代码，以后需要时重新启用。

---

# 5. Hearth 应增加“隔离区 / 箱子”设计

这是本轮非常重要的架构建议。

用户提出的：

> “不好用、但以后可能有用的东西，不删除，放到箱子里，平时不看。”

应正式工程化。

建议新增：

```text
crates/
  active/
  experimental/
  quarantine/
```

或者如果不希望修改当前 workspace，也可以：

```text
crates/
  ...
  _quarantine/
```

但**不建议单纯靠目录名前缀长期管理**。

更推荐：

```text
docs/
  architecture/
  active/
  deferred/
  quarantine/

crates/
  ...

experimental/
  ...
```

并规定：

### Active

运行时正式依赖。

### Experimental

允许编译/测试，但默认不进入生产路径。

### Quarantine

代码保留，但：

```text
不默认编译
不进入主 Runtime
不参与正常依赖链
```

必须注明：

```text
why_quarantined
original_use_case
known_failure
reentry_condition
related_commit
```

例如：

```toml
component = "old_patch_engine"

status = "quarantine"

reason = "current implementation causes excessive whitespace sensitivity"

kept_for = "future patch strategy experiments"

reentry_condition = [
    "new matcher has deterministic behavior",
    "negative tests pass",
    "real task benchmark improves"
]
```

这样：

> **“不用” ≠ “删除”。**

同时避免：

> **“以后可能有用” → 永久污染主代码路径。**

---

# 6. Quarantine 设计的核心原则

箱子不是垃圾桶。

必须满足：

```text
可定位
可解释
可恢复
可重新评估
不可默认影响主系统
```

建议每一个隔离项目都有：

```text
README.md
STATUS.md
origin commit
reason
re-entry criteria
```

甚至可以建立：

```text
docs/quarantine-register.md
```

作为总账。

这是防止 Hearth 从：

```text
35k LOC
```

增长到：

```text
200k LOC
```

以后出现“没人敢删、没人敢动、没人知道为什么存在”的关键措施。

---

# 7. 第一梯队：必须对标的 Harness 核心能力

---

## H-A01：Thread / Turn 连续工作模型

### Codex 值得借鉴

Codex 将：

```text
Thread
 └── Turn
      ├── Agent message
      ├── tool calls
      ├── approval
      ├── results
      └── completion
```

作为连续工作单元。

一个审批不是新的工作。

一个工具失败也不是新的工作。

一次恢复也不是新的工作。

它们都属于同一个 Turn / Thread 生命周期。

### Hearth 当前问题

Hearth 已经有 session、resume、history、replay，但是 CLI 命令和运行时心智仍存在一定分离。

需要确认：

```text
repl
 ↓
Agent Turn
 ↓
approval
 ↓
resume
```

是否始终保持同一个逻辑 Turn。

### 目标

用户感觉：

> “我在做一件工作。”

而不是：

> “我在管理一个 session。”

### 实现建议

定义内部概念：

```text
Session
Turn
Step
ToolCall
Approval
Observation
Completion
```

不一定照抄 Codex 数据结构，但语义必须稳定。

---

# 8. H-A02：Approval 必须成为当前工作流的一部分

Codex 的 Approval 是：

```text
Agent
 ↓
工具动作
 ↓
需要批准
 ↓
Inline approval
 ↓
用户决定
 ↓
原 Turn 继续
```

而不是：

```text
Agent
 ↓
pending
 ↓
退出
 ↓
用户运行另一个 CLI
 ↓
approve
```

Codex 当前 app-server 的 approval 请求直接绑定当前 thread/turn/item，并由 Runtime 在用户决策后继续工作。

### Hearth 当前

已有：

```text
approve
deny
ApprovalState
```

这属于基础设施已具备。

### 下一步不是增加 approval 命令

而是：

> **让 approval 在 REPL 中自然出现、自然结束、自动继续。**

### 验收

用户执行：

```text
“修改代码并运行测试”
```

遇到需要批准动作：

```text
Hearth：
需要你的确认：

动作：
cargo test ...

原因：
验证刚才的修改。

[允许] [拒绝]
```

批准后：

```text
继续当前任务
```

无需：

```text
退出
重新 resume
```

---

# 9. H-A03：Approval Policy 必须与 Sandbox 正交

Codex 当前明确区分：

```text
Sandbox：
系统最多允许做什么

Approval：
什么时候需要用户同意
```

这两个东西不能混成一个开关。当前 Codex CLI 公开配置中同时存在 sandbox mode 与 approval policy，并支持不同工具类型的 approval mode。

### Hearth 目前方向正确

已有：

```text
G0 sandbox
ApprovalState
```

### 需要进一步完成

明确矩阵：

| 行为              | Sandbox               | Approval |
| --------------- | --------------------- | -------- |
| workspace read  | 允许                    | 不需要      |
| workspace write | 允许                    | 按策略      |
| workspace 外访问   | 拒绝/请求扩权               | 必须       |
| 网络              | 默认拒绝                  | 请求       |
| 危险命令            | Sandbox 限制 + Approval | 必须       |

核心原则：

> **安全不能靠“每件事都弹确认”实现。**

否则最后会变成：

```text
Hearth 很安全
但是没人愿意用
```

---

# 10. H-A04：工具必须有统一的 Runtime 生命周期

Codex 当前的工具执行已经不是简单的：

```text
tool(input) -> output
```

而是：

```text
item/started
 ↓
running
 ↓
approval
 ↓
execution
 ↓
item/completed
```

app-server 当前明确要求客户端把 `item/started` 作为即时状态、`item/completed` 作为权威最终状态。

### Hearth 应定义统一内部模型

```rust
ToolInvocation {
    id,
    tool,
    arguments,
    status,
    approval,
    start_at,
    end_at,
    result,
    error,
}
```

这样：

```text
bash
write_file
apply_patch
web_fetch
MCP
未来其他工具
```

全部都进入同一个生命周期。

---

# 11. H-A05：Apply Patch 不应仅仅是“字符串替换器”

Codex 的 `apply_patch` 已经是独立的文件变更机制，支持：

```text
Add File
Delete File
Update File
Move
Hunks
```

并且 patch 在执行前会经过独立的安全判断和 approval 路径。

### Hearth 当前

SEARCH/REPLACE 能用，但：

```text
空白敏感
CRLF 敏感
```

H9 正在补救。

### 建议

不要目标设成：

> “让 SEARCH/REPLACE 和 Codex patch 一模一样。”

应该设成：

> **让模型拥有稳定的结构化文件修改能力。**

最低标准：

```text
Add
Update
Delete
Rename
Diff
Reject
```

并且：

```text
精确匹配
 ↓
宽松归一化
 ↓
仍不明确
 ↓
拒绝
```

不能为了成功率而错误匹配。

---

# 12. H-A06：Context Engineering 是下一大差距

这是非常重要的一项。

Codex 当前拥有项目级 instructions 体系，并通过 `AGENTS.md` 从项目根向当前工作目录逐级发现、合并 instructions，子目录规则可以覆盖上层规则。

这意味着 Agent 每次工作前不是只收到：

```text
system prompt
+
user message
```

而是：

```text
global instruction
+
project instruction
+
directory instruction
+
skills
+
tool definitions
+
task context
+
history
```

这其实是 Context Harness。

### Hearth 当前

已经拥有：

```text
constitution
template
experience
memory
skills / tool schemas
project sync
```

但这些更像多个能力集合。

需要进一步统一：

```text
Context Builder
```

负责：

```text
build_static_context()
build_project_context()
build_task_context()
build_memory_context()
build_experience_context()
build_dynamic_turn_context()
```

然后得到：

```text
FinalContext
```

---

# 13. H-A07：上下文必须区分“稳定区”和“动态区”

这直接连接你当前 P1-1 / P1-2。

应该抽象：

```text
Stable Prefix
--------------------
constitution
tool schemas
project rules
stable instructions
stable capabilities

Dynamic Suffix
--------------------
history
current task
tool results
current user message
```

Stable Prefix：

> 生命周期尽量稳定。

Dynamic Suffix：

> 每轮变化。

P1-1 当前应该先测量 cache hit/miss，再验证稳定性，而不是盲猜 HashMap。这个顺序已经写进地基任务书。

---

# 14. H-A08：上下文 Compact 必须成为“压缩”，不能成为“删除”

H3 已经沿这个方向修。

正确结构：

```text
Active Context
     ↓
Compaction
     ↓
Summary
     +
Archive
     ↓
可回溯历史
```

Codex 当前也提供显式 `/compact`，并且 app-server 将 compaction 作为 Thread 生命周期中的正式事件。

### Hearth 下一步

不仅要让 archive 存在。

还要让：

```text
session-local
archive
search
recall
```

变成明确的数据模型。

特别注意：不要把所有 session 的 compacted history 无限堆进一个没有 scope 的共享文件。

---

# 15. H-A09：Session 生命周期需要进一步增强

当前 Hearth：

```text
resume
sessions
history
cancel
status
replay
```

已经不错。

Codex 当前进一步提供：

```text
resume
queue
archive
delete
unarchive
fork
```

并支持直接在 TUI 中 fork / side conversation。

### Hearth 应吸收的优先级

高：

```text
resume
rename
archive
queue
fork
```

中：

```text
side conversation
```

低：

```text
完全复制 Codex session 管理
```

### 为什么 Fork 很重要

因为：

```text
一个任务
   ↓
方案 A

同时
   ↓
探索方案 B
```

而不是污染主线。

这对你未来的 Agent 连续性设计也很有价值。

---

# 16. H-A10：REPL 应增加“当前任务”的概念

当前：

```text
hearth repl
```

用户进入的是：

> 对话。

Codex 的体验实际上更接近：

> 工作线程。

因此 Hearth 可以保持：

```text
repl
```

命令不变，但内部应该有：

```text
Current Thread
Current Turn
Current Status
```

REPL 内部提供：

```text
/status
/compact
/diff
/tasks
/model
/permissions
/review
/fork
/rename
```

注意：

这些不一定全部做。

重点是：

> **把高频操作变成当前工作流中的“控制动作”，而不是退出 REPL 再开另一个 shell 命令。**

Codex TUI 当前就是这个方向。

---

# 17. H-A11：加入 `/status` 风格的低成本可观测能力

用户应该始终能知道：

```text
当前模型
当前 Thread
当前任务
当前 turn
工具状态
审批状态
sandbox
token
耗时
剩余预算
```

当前 Hearth 已经有：

```text
introspect
status
tokens
context_fill_pct
```

所以这是低成本提升。

目标不是增加更多日志。

目标是：

> **用一个稳定的状态视图替代用户猜测。**

Codex 当前 `/status` 就是用于查看当前 model、approvals、token usage 等运行状态。

---

# 18. H-A12：把“最终 Diff”提升为一等公民

Codex 当前 `/diff` 能直接查看当前改动，包括 untracked files。

Hearth 建议：

```text
/diff
```

至少能显示：

```text
新增
修改
删除
未跟踪
```

而不是逼用户执行：

```bash
git diff
git status
```

这属于**高收益低成本 CLI 能力**。

---

# 19. H-A13：加入 Project Instructions，但不要机械复制 AGENTS.md

Codex 的 AGENTS.md 是非常值得吸收的设计：

```text
全局
 ↓
项目根
 ↓
子目录
```

逐层作用域。

Hearth 可以自己定义：

```text
HEARTH.md
```

或者：

```text
.hearth/instructions.md
```

推荐优先考虑：

```text
HEARTH.md
```

因为：

```text
身份属于 Hearth
```

而不是：

```text
复刻 AGENTS.md
```

但语义可以借鉴：

```text
global
project
directory
```

---

# 20. H-A14：Skills 应该进入 Context，而不是只作为命令列表

Codex 当前已经把 Skills 作为 TUI 一级能力：可以列出 skills，也允许要求 Codex 使用特定 skill。

Hearth 当前有：

```text
template
tools
experience
```

未来可以统一：

```text
Skill
=
可发现
+
可加载
+
有作用域
+
有触发条件
+
可版本化
```

不要现在就实现完整 Skill Marketplace。

只建立：

```text
Skill Descriptor
Skill Loader
Skill Scope
```

即可。

---

# 21. H-A15：网络权限应该变成“请求”，而不是配置文件操作

当前 T11：

> 出网白名单 deny 时还没有 agent 提议 + 用户审批闭环。

这实际上非常接近 Codex 的网络审批模型。

Codex 当前把网络访问作为可以独立产生 approval request 的动作，并且 approval action 中包含 target / host / protocol / port 等结构化信息。

Hearth 应改成：

```text
Agent
 ↓
想访问 example.com
 ↓
network permission request
 ↓
用户确认
 ↓
policy amendment
 ↓
继续
```

而不是：

```text
用户
 ↓
编辑 config.toml
```

这是“可信委托”真正的必要能力之一。

---

# 22. H-A16：Clarify 必须变成真正的一等交互

这是 Hearth 当前 P0/P1 中真正值得优先做完的一项。

成熟 Agent 遇到：

```text
信息不足
```

不是：

```text
随便猜
```

也不是：

```text
永久停住
```

而是：

```text
Agent
 ↓
识别 ambiguity
 ↓
生成结构化 clarification
 ↓
用户回答
 ↓
继续当前 Turn
```

Codex 的 protocol 本身就存在 `request_user_input` / `UserInputAnswer` 生命周期。

Hearth 已经有 `InteractionRequest` 原语，所以这不是重新发明。

应该把：

```text
clarify
approval
```

统一成：

```text
InteractionRequest
```

的不同 action。

---

# 23. H-A17：不要让 Agent 过早进入“无聊的无限循环”

当前 H2 已经修：

```text
max_time_secs
↓
deadline_exceeded()
```

而不是只有：

```text
max_steps
```

这个是正确的。

但是以后还需要：

```text
step budget
time budget
token budget
retry budget
same-action budget
```

五种不同预算。

尤其建议增加：

```text
same_effect_repeat_guard
```

例如：

```text
同一个工具
+
同一个参数
+
连续 N 次
```

应该触发：

```text
replan
clarify
give_up
```

而不是无限重复。

---

# 24. H-A18：错误必须结构化，而不是让 Agent 和人类一起读 tracing

当前任务书已经发现：

```text
ERROR agent_core::scheduler: ...
```

直接混进用户输出。

建议统一：

```text
Internal Error
AgentEvent
User Rendering
```

三层。

Codex app-server 同样以结构化 protocol events 给客户端，而不是要求 UI 从文本日志猜状态。

Hearth 的 `EnvelopedEvent` 已经为此提供了不错基础。

---

# 25. H-A19：结构化执行报告要保留

H8 我建议继续做。

因为：

```text
Agent
 ↓
执行
 ↓
Report
```

实际上是：

**从日志系统进入证据系统。**

最终报告至少：

```text
Goal
Plan
Tools
Approvals
Changes
Tests
Errors
Token
Time
Final result
Verification
```

这会直接降低：

> “下一个审查 AI 需要翻 5000 行日志才能知道发生了什么”

这种成本。

---

# 26. H-A20：统一“可恢复性”

恢复不能只指：

```text
resume session
```

应该是：

```text
恢复：
Thread
Turn
Tool state
Approval state
Task state
Context
Checkpoint
```

尤其要防止：

```text
tool 执行完成
但 execution result 尚未落盘
↓
进程 crash
↓
resume
↓
重复执行副作用
```

你当前任务书中的 P2-6 已经准确指出了这个问题。

它虽然目前是 P2，但从长期可信委托来看，我建议把它提升到：

**P1 边缘 / P0 后继专项。**

---

# 27. CLI 层真正值得做的事情

不要把 CLI 变成命令仓库。

CLI 要围绕三个场景设计：

## 场景 A：一次性委派

```bash
hearth "修复这个 bug，并运行测试"
```

或者保留：

```bash
hearth chat "..."
```

目标：

> 一句话交付。

---

## 场景 B：持续工作

```bash
hearth repl
```

目标：

> 像一个持续工作的工程伙伴。

---

## 场景 C：管理 / 恢复

```bash
hearth sessions
hearth resume
hearth doctor
```

目标：

> 出问题时才进入管理模式。

核心原则：

> **正常工作不应该让用户频繁进入管理模式。**

---

# 28. 建议的 CLI 第二阶段目标

当前：

```text
chat
repl
resume
sessions
history
approve
deny
cancel
status
replay
whoami
template
tools
civ
tasks
```

不要删除。

但应该逐渐形成三个层次。

### 日常

```text
hearth
hearth repl
hearth resume
```

### 高频控制

```text
/status
/compact
/diff
/review
/permissions
/fork
/rename
```

### 系统管理

```text
hearth doctor
hearth config
hearth sessions
hearth replay
hearth coverage
```

这样用户会自然知道：

```text
日常工作
```

和

```text
故障排查
```

是不同事情。

---

# 29. 最值得先借鉴的 Codex 能力排序

本轮不按“酷不酷”，按：

```text价值
×
成熟度
×
维护成本
```

排序。

## S 级：必须做

```text
S1  Inline Approval
S2  Clarify → Answer → Continue
S3  Tool lifecycle normalization
S4  Context Builder
S5  Sandbox / Approval 正交
S6  Deadline / Budget / Retry 分层
S7  Structured diff
S8  Reliable resume
S9  Verification → Done
S10 Network permission request
```

---

# 30. A 级：强烈建议做

```text
A1 /status
A2 /compact
A3 /diff
A4 /review
A5 Thread rename
A6 Thread archive
A7 Fork
A8 Queue
A9 Project-level instructions
A10 Skills discovery
A11 Structured execution report
A12 doctor/config diagnostics
```

---

# 31. B 级：可以借鉴，但不要着急

```text
B1 Side conversation
B2 Personality
B3 MCP 完整生态
B4 Desktop integration
B5 advanced subagents
B6 background terminals
B7 elaborate app-server compatibility
```

这些不是 Hearth 当前地基问题。

---

# 32. C 级：暂不做

```text
C1 完整复制 Codex CLI UX
C2 整套 Codex app-server protocol
C3 复制 Codex crate 结构
C4 为了兼容而复制大量上游 abstraction
C5 追逐 Codex 每一个新命令
```

原因：

> **这些东西会把 Hearth 拉回“Codex fork”路线。**

---

# 33. Hearth 必须保留的差异

无论对标结果如何，下列结构不应因为 Codex 对标而删除：

```text
1. Plan / Act / Observe / Reflect 白盒状态机
2. Observer 第三权
3. G0–G3 基因体系
4. constitution
5. Experience
6. Multi-provider
7. Project Sync
8. Civ
9. Identity
10. Hearth 自己的事件契约
```

Hearth 当前已经形成这些结构性差异。

对标 Codex 的目的：

> 改善 Harness。

不是：

> 消灭 Hearth。

---

# 34. 一个非常关键的评估标准：不要问“Codex 有没有”

每项候选能力都应该问五个问题：

```text
Q1：
Hearth 当前有没有对应能力？

Q2：
如果有，为什么体验仍然不如 Codex？

Q3：
Codex 的优势来自：
    数据结构？
    Agent Loop？
    Runtime？
    CLI UX？
    Prompt？
    Tool schema？
    默认值？
    生态？

Q4：
吸收后是否破坏 Hearth 原有架构？

Q5：
如果现在不做，未来重新启用成本是多少？
```

只有回答完五个问题，才能改。

---

# 35. 建议建立正式 Benchmark Matrix

每一项记录：

| 项目                        | Hearth | Codex | 差异   | 根因               | 动作          |
| ------------------------- | ------ | ----- | ---- | ---------------- | ----------- |
| Approval                  | 有      | 强     | 中    | 交互不闭环            | ADAPT       |
| Resume                    | 有      | 强     | 中    | 可靠性              | ADOPT       |
| Context                   | 有      | 强     | 大    | context assembly | ADAPT       |
| Patch                     | 有      | 强     | 中    | matcher          | ADAPT       |
| Sandbox                   | 强      | 强     | 小    | 策略层              | KEEP/ADAPT  |
| Network approval          | 弱      | 强     | 大    | 缺 request flow   | ADOPT       |
| Fork                      | 无      | 有     | 中    | session model    | DEFER/ADAPT |
| AGENTS-style instructions | 弱      | 强     | 大    | scope model      | ADAPT       |
| Skills                    | 有雏形    | 成熟    | 中    | discovery        | ADAPT       |
| Identity                  | 强      | 弱     | 方向差异 | Hearth特色         | KEEP        |
| Observer                  | 强      | 无同等目标 | 方向差异 | Hearth特色         | KEEP        |
| Civ                       | 强      | 无同等目标 | 方向差异 | Hearth特色         | KEEP        |

这张表未来应该成为 Hearth 的长期架构资产。

---

# 36. “维护成本”必须正式进入每次技术决策

每一个新功能必须额外回答：

```text
新增多少代码？
增加多少依赖？
增加多少测试？
增加多少状态？
增加多少恢复路径？
增加多少文档？
谁维护？
未来怎么删除？
```

例如：

```text
一个新 crate
```

不是：

```text
2,000 行代码
```

而是：

```text
2,000 LOC
+
Cargo dependency
+
API surface
+
测试
+
错误处理
+
CI
+
文档
+
未来迁移
```

因此建议引入：

```text
Maintenance Cost: S / M / L / XL
```

没有这个字段，未来很容易不断加东西。

---

# 37. Quarantine 的重新进入条件

隔离区里的能力未来重新启用必须满足：

```text
1. 明确 use case
2. 新实现或旧实现已经通过当前架构标准
3. 有 regression test
4. 有 negative test
5. maintenance cost 可接受
6. 有明确 owner
```

否则永远留在箱子里。

---

# 38. 下一阶段施工顺序

建议：

```text
Phase A
地基 P0 收口

↓

Phase B
Codex Harness Benchmark

↓

Phase C
S 级 Harness 改造

↓

Phase D
A 级 UX 改造

↓

Phase E
实机横向任务测试

↓

Phase F
决定哪些进入 quarantine

↓

Phase G
再决定是否进入下一代 Hearth
```

当前不要反过来。

---

# 39. 真实横向测试必须改变

不能只：

```text
cargo test
```

而应该准备一组完全相同任务：

```text
Task 01
修一个 Rust bug

Task 02
重构一个模块

Task 03
运行测试并修失败

Task 04
修改多个文件

Task 05
访问网络后继续

Task 06
故意制造歧义

Task 07
中途 Ctrl-C

Task 08
中途 kill

Task 09
超过 context

Task 10
长时间 cargo build
```

分别在：

```text
Hearth
Codex
```

运行。

记录：

```text
完成率
用户介入次数
错误恢复次数
重复工具调用
总耗时
token
tool failure
approval count
resume success
最终 verify
```

这样才能真正知道：

> **Hearth 为什么“不如 Codex 好用”。**

而不是靠印象。

---

# 40. 用户体验的最终指标

Hearth 最终不应该追求：

```text
“命令比 Codex 多”
```

也不应该追求：

```text
“测试比 Codex 多”
```

核心指标应该是：

# Delegation Friction

定义：

> 用户为了让一个 Agent 正确完成一个真实任务，需要主动介入多少次。

例如：

```text
用户发出任务
      ↓
Agent
      ↓
自动执行
      ↓
只有真正需要人类判断时：
    问一次
      ↓
继续
      ↓
验证
      ↓
交付
```

理想情况：

```text
用户主动干预：1 次以内
```

差的情况：

```text
approve
resume
status
重新解释
修改配置
手工恢复
```

五六次。

这就是：

**为什么“同一个模型在 Codex 上很好用，在 Hearth 上不好用”。**

---

# 41. 当前任务优先级建议

在 Hearth 当前基线上，我建议 GLM 5.3 暂时按照下面顺序执行：

## P0

```text
P0-1 resume 实机真实性确认/修复
P0-2 task wall-clock deadline
P0-4 clarification / approval 完整闭环
P0-5 context archive
P0-6 verification / done truthfulness
```

其中 P0-6 虽然现有 verify 已经有基础，但需要专门定义：

> Agent 声明 Done ≠ Task Done。

---

## P1

```text
P1-1 Tool lifecycle
P1-2 Inline approval
P1-3 Network permission request
P1-4 Context Builder
P1-5 stable prefix
P1-6 apply/diff
P1-7 resume / recovery hardening
P1-8 structured execution report
P1-9 local CLI command consistency
P1-10 retry / rate-limit policy
```

---

## P2

```text
P2-1 /status
P2-2 /compact
P2-3 /diff
P2-4 /review
P2-5 /fork
P2-6 /rename
P2-7 /archive
P2-8 Project instructions
P2-9 Skills
```

---

# 42. GLM 5.3 的执行铁律

### 铁律一

不要因为 Codex 有某功能就添加。

---

### 铁律二

不要因为 Hearth 没有某功能就认为它是缺陷。

---

### 铁律三

任何“Codex 更好”的结论必须找到实际行为依据。

---

### 铁律四

源码分析不能替代真实任务测试。

---

### 铁律五

单元测试不能替代端到端。

---

### 铁律六

commit message 不能作为门禁证据。

你们已经有一次：

```text
文档：
VM 全绿

实际：
TEST_RC=101
CLIPPY_RC=101
```

的反例。

以后统一：

```text
t_gate.log
+
真实命令输出
```

为权威。

---

### 铁律七

所有失败必须判断：

```text
真正缺陷
配置问题
模型问题
Harness 问题
CLI 问题
环境问题
文档问题
```

不能全部叫 bug。

---

# 43. 最终目标

Hearth 不需要变成：

```text
Codex 2
```

而应该达到：

```text
Codex 已经证明好用的 Harness 原则
               +
Hearth 自己的白盒 Agent Loop
               +
Observer
               +
Experience
               +
Identity
               +
Project Coordination
               +
Multi-provider
               +
长期连续性
```

最终形成：

```text
                 Hearth

      ┌──────────────────────────┐
      │ Identity / Experience    │
      │ Observer / Civilization  │
      │ Project / Task           │
      ├──────────────────────────┤
      │      Agent Harness       │
      │ Plan Act Observe Reflect │
      ├──────────────────────────┤
      │ Tool / Approval / Policy │
      ├──────────────────────────┤
      │ Sandbox / Runtime        │
      ├──────────────────────────┤
      │ LLM Providers             │
      └──────────────────────────┘
```

其中：

```text
Codex
```

是：

> **参照系。**

不是：

> **最终结构。**

---

# 44. 当前不应该做的事情

本阶段暂缓：

```text
完整器灵形态
复杂人格系统
大规模 Civilization 扩展
复杂 Multi-Agent
大规模 MCP 生态
新的桌面 UI 花活
复杂子代理体系
```

这些全部建立在：

```text
可信委托
```

之上。

---

# 45. 最重要的判断

当前 Hearth 的真实问题不是：

> “它有没有 Codex 那么多功能？”

而是：

> **为什么同一个模型，在 Codex Harness 中更容易把事情做好，而在 Hearth 中用户会觉得不舒服？**

必须把这句话拆成可测试变量：

```text
Prompt
Context
Tool schema
Tool lifecycle
Approval
Sandbox
Timeout
Retry
State
Recovery
TUI
Defaults
Instruction discovery
Diff
Verification
```

然后一个一个排除。

这才是真正的对标。

---

# 46. 交付要求

每一个 Codex 对标项交付必须包括：

```text
1. Hearth 当前行为
2. Codex 对应行为
3. 差异描述
4. 根因
5. 判断：
   ADOPT / ADAPT / DEFER / QUARANTINE
6. 改动文件
7. 新增测试
8. 正面验收
9. 负面验收
10. 真实任务验收
11. 维护成本
12. 是否改变已有架构不变量
```

没有第 10 项的：

> “真实任务验收”

不能宣称：

> “体验已经对标完成”。

---

# 47. 结论

Hearth 下一阶段不是“继续造”。

而是：

> **精炼。**

把已经存在的 35k LOC 从：

```text
功能集合
```

逐步变成：

```text
稳定的 Agent Runtime
```

把 Codex 已经验证过的工程经验吸收进来。

把不适合 Hearth 的部分拒绝掉。

把现在没有必要启用、但是以后可能有价值的设计放入隔离区。

最后形成：

```text
主系统
=
少而强

隔离区
=
多而备用

文档
=
知道为什么存在
```

这比单纯追求代码规模健康得多。

---

# 48. 下一轮必须向 Hearth 项目补齐的资料清单

为了把下一轮对标从“架构级比较”进一步推进到“逐项定位具体差距”，需要补充下面这些资料。

### A. Hearth 方面——最重要

1. **当前 `hearth repl` 一次完整真实任务的原始 transcript**

   最好包含：

   * 用户输入；
   * Agent 文本；
   * Plan；
   * Tool call；
   * Approval；
   * Tool result；
   * Reflect；
   * Done。

2. **一次“你认为明显不如 Codex 好用”的完整案例**

   不要总结，最好直接给原始终端记录。

3. **当前 `hearth repl` 的内部/help**

   特别是：

   ```text
   /help
   ```

   或 REPL 内部所有可用命令。

4. **当前配置样例**

   脱敏即可：

   ```text
   ~/.config/hearth/config.toml
   ```

   API key 不要提供。

5. **当前项目根目录结构**

   例如：

   ```bash
   tree -L 2
   ```

6. **当前 Agent Loop 与 Tool Runtime 相关源码位置**

   主要确认：

   ```text
   loop
   dispatcher
   tool-runtime
   repl
   session_store
   context
   approval
   sandbox
   ```

### B. Codex 方面——重点不是整仓库

如果你愿意继续做严格对标，最好固定一个**具体 Codex 版本**作为 benchmark baseline，然后收集：

```text
1. codex --help
2. 进入 codex 后 /help
3. /status
4. /permissions
5. /compact
6. /diff
7. /review
8. /fork
```

再选 3～5 个和 Hearth 完全相同的真实任务跑一遍。

最有价值的不是 Codex 源码本身，而是：

> **同一个任务，Hearth 做哪里不舒服，Codex 哪里顺。**

---

### C. 最后再补一个非常有价值的东西

请你给我一个你亲自使用 Codex 时的评价，哪怕只有十几条：

```text
“这里 Codex 让我觉得舒服”
“这里 Hearth 让我觉得别扭”
```

不用技术化。

因为这能把**“机器可测的差距”**和**“人的体感差距”**对上。最终 Hearth 是否好用，前者和后者缺一不可。
