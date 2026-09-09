# Hearth 战略地图 v3.0

## 从“自研 Harness”转向“认知层 + 标准宿主”的长期路线

### 0. 先给结论

我现在认为，Hearth 不应该再以：

> “把自研 Agent Core 做完整”

作为最终路线。

而应该变成：

```text
                  ┌────────────────────────────┐
                  │          HEARTH             │
                  │      最终交付的整体         │
                  └─────────────┬──────────────┘
                                │
               ┌────────────────┴────────────────┐
               │                                 │
               ▼                                 ▼
       ① Standard Host                    ② Hearth Cognition
       通用标准底座                         Hearth 自己的东西
       = 身体 / 执行环境                    = 大脑之外的认知环境
               │                                 │
      Codex / Goose / OpenCode              Intent
      / 其他成熟 Host                       Evidence
               │                            Discovery
               │                            Memory
               │                            Learning
               │                            Interaction
               │                                 │
               └─────────────┬───────────────────┘
                             │
                             ▼
                       用户实际体验
                             │
                   ┌─────────┴─────────┐
                   ▼                   ▼
               做得对                用得舒服
```

所以以后衡量 Hearth，不是：

> “我们自己的 loop.rs 做得多漂亮。”

而是：

> **“Hearth Cognition 嫁接到一个成熟 Host 后，是否真的让 AI 更可靠、更会发现问题、更会管理上下文、更懂用户，而且用户体感明显更好？”**

---

# 一、Hearth 其实有三条生命周期

这是我认为目前最需要分清的一点。

过去我们一直把三个问题揉在一起，所以项目才会越来越重。

```text
                 Hearth
                    │
        ┌───────────┼───────────┐
        ▼           ▼           ▼
      Host         Cognition    Product
      身体          灵魂/认知     用户产品
```

它们其实可以**分别成功、分别失败、分别演化**。

### ① Host

解决：

> AI 能不能稳定地干活？

包括：

```text
Session
Workspace
Tool
Execution
Sandbox
Permission
Context
Extension
Observability
UX
```

**Host 不是 Hearth 的核心创新资产。**

成熟了就应该复用。

---

### ② Cognition

解决：

> AI 怎样才能“更会干活”？

这才是 Hearth 真正应该投入长期资源的地方。

包括：

```text
Intent
Evidence
Unknown Discovery
Memory
Failure Learning
Issue Learning
Interaction
Calibration
```

这是我们过去几个月真正一点点摸出来的东西。

---

### ③ Product

解决：

> 用户到底愿不愿意天天用？

包括：

```text
UI
UX
速度
稳定性
可解释性
工作流
配置
生态
桌面端
```

这一层不能太早做。

因为：

```text
28% 成功率的发动机
+
漂亮 UI
=
漂亮的失败
```

你之前那个直觉是对的。

---

# 二、现在的 Hearth 在地图上的位置

如果把整个项目看成一张 RPG 地图，现在其实不是“快做完了”。

更准确是：

```text
                         未来
                          ↑

             ┌────────────────────────────┐
             │      Hearth 最终形态       │
             │ Cognition + Mature Host   │
             │        + Product          │
             └────────────┬───────────────┘
                          │
                   ★ 战略转折点
                          │
              ┌───────────┴───────────┐
              │                       │
        Standard Host          Hearth Cognition
         重新定义               开始独立成型
              │                       │
              └───────────┬───────────┘
                          │
                  【我们现在就在这里】
                          │
                 Core Revalidation
                          │
              自研 Engine 暂时冻结/降级
                          │
                    P4/P5 余波
                          │
                  v0.2.18 / v0.2.19
                          │
                  自研 Core 已证明：
                  “机制能做，但不是主航道”
```

所以我不再把当前状态描述成：

> “Hearth Core 还差几个 bug。”

而是：

> **Hearth 已经完成了一次非常昂贵的“自研完整 Harness 可行性实验”，实验结果基本告诉我们：通用 Host 不应该继续由 Hearth 自己承担。**

这其实是非常有价值的结果。

---

# 三、整个战略路线，我建议分成 8 个阶段

---

## S0 · Core Revalidation

### 目标：先确定旧 Hearth Core 到底处于什么状态

### 当前状态

**正在完成最后的独立验证 / 战略重判。**

已经得到的事实非常重要：

```text
Core 机制：
    强

真实行为：
    不稳定

脚本化测试：
    很强

非脚本真实使用：
    暴露严重缺口
```

因此：

### S0 出口不是“Core 完美”

而是：

```text
Core 是否值得继续承担 Host 主航道？
```

目前已有战略结论偏向：

> **不承担。**

也就是：

```text
hearth-engine
       ↓
Experimental Backend
```

而不是：

```text
hearth-engine
       ↓
Hearth 的唯一未来
```

---

# S1 · HCC

# Host Capability Contract

### 这是整个新战略真正的第一步

先把“第一个 1”定义清楚。

也就是：

> **什么叫一个合格的通用 Agent Host？**

我建议正式建立：

```text
HCC v0.1
```

包含：

```text
H1 Session
H2 Workspace
H3 Execution
H4 Safety
H5 Context
H6 Extension
H7 Observability
H8 UX
```

每条都定义：

```text
required
optional
observable
testable
```

还要定义：

```text
Cognition depends on what?
```

例如：

```text
Discovery → 需要 Workspace + Execution + Observability

Evidence → 需要 Execution + Artifacts + Observability

Memory → 需要 Session + Context

Interaction → 需要 UX + Extension
```

### S1 的意义

一旦这个完成：

**以后不再用“我觉得 Codex 好”来选底座。**

而是：

```text
HCC
 ↓
Candidate Host
 ↓
Conformance
 ↓
Score
```

---

# S2 · Host Bake-off

## 找“身体”

候选可以包括：

```text
Codex CLI
Goose
OpenCode
其他成熟 Host
```

注意：

### 不是评测模型。

而是评测：

```text
Host 能否满足 HCC
```

核心指标：

```text
Session
Execution
Tooling
Safety
Extension
Context
Observability
UX
```

然后出现一个非常关键的东西：

# Host Scorecard

例如：

```text
              Codex    Goose    OpenCode
Session         5        4         4
Execution       5        4         5
Safety          5        4         4
Extension       5        5         5
Observability   4        3         4
UX              5        4         4
----------------------------------------
Total           29       24        26
```

**只是示意，不是现在的真实数据。**

最后选：

> **最适合承载 Hearth Cognition 的宿主。**

而不是“最强大的 Agent”。

---

# S3 · Hearth Cognition MVP

这是 Hearth 真正重新诞生的地方。

第一阶段我建议**只做六个器官**：

```text
                 Hearth Cognition
                        │
        ┌───────────────┼────────────────┐
        │               │                │
      Intent         Evidence        Discovery
        │               │                │
        ├───────────────┼────────────────┤
        │               │                │
      Memory        Interaction      Learning
```

其中第一优先级：

## Evidence

和：

## Discovery

---

# S4 · Discovery Engine

这是你刚才提出的：

> **未知域发现器**

我会把它正式提成战略核心，而不是辅助功能。

它不是：

> “让模型知道自己不知道。”

而是：

> **持续寻找当前任务模型缺失的东西。**

内部状态可以是：

```text
KNOWN_KNOWN
KNOWN_UNKNOWN
UNKNOWN_KNOWN
UNKNOWN_UNKNOWN
```

然后做：

```text
Task Model
    ↓
Assumption Map
    ↓
Missing Information
    ↓
Probe Selection
    ↓
Environment Observation
    ↓
Contradiction Detection
    ↓
New Knowledge
```

这一层成熟以后，会真正改变我们的开发方式。

因为未来不是：

```text
你发现问题
→ 告诉 AI
→ AI 修
```

而可以慢慢变成：

```text
AI 发现：
“这里存在一个尚未验证的前提。”

AI：
“我需要确认 X。”

AI：
“最便宜的办法是 Y。”

AI：
“真实环境与预期不一致。”

AI：
“这个不一致可能影响 Z。”
```

这才接近你说的：

> **AI 能帮你补足认知盲区。**

---

# S5 · Evidence / Verification OS

Discovery 发现了什么以后，还必须回答：

> **这是真的吗？**

所以接下来会形成：

```text
Claim
 ↓
Evidence Requirement
 ↓
Evidence Acquisition
 ↓
Verification
 ↓
Confidence
 ↓
Decision
```

以后任何关键结论都可以有：

```text
VERIFIED
INFERRED
UNVERIFIED
CONTRADICTED
```

这里会直接吸收我们以前几个月的经验：

```text
LLM self-report
        ≠
Fact
        ≠
Verification
        ≠
Completion
```

---

# S6 · Memory / Learning / Feedback

然后才是你真正想要的：

> **这个系统能不能越来越会做？**

这层包括：

### Memory

```text
session
long-term
facts
preferences
working memory
historical retrieval
```

### Learning

不是训练参数。

而是：

```text
Failure
 ↓
Classification
 ↓
Root Cause
 ↓
Lesson
 ↓
Rule / Skill / Evidence pattern
 ↓
Future behavior
```

### Issue Loop

```text
Issue
 ↓
Evidence
 ↓
Independent Review
 ↓
Confirmed
 ↓
Fix
 ↓
Regression
 ↓
Closed
 ↓
Knowledge returned to system
```

这就是你之前发现的：

> 登记工作了，但回填工作从来没工作。

这里真正补齐它。

---

# S7 · Interaction Intelligence

这是“用户体感”真正进入架构的位置。

AI 不只是：

> “把任务做完。”

而是要理解：

```text
用户说了什么
用户真正想干什么
用户没说什么
用户是不是在纠正我
用户是不是不满意
用户是不是不知道自己不知道
什么时候应该问
什么时候不该问
什么时候可以自己决定
```

包括你之前提出的：

```text
Intent
Clarification
Delegation
Disagreement
Emotion
Context Dependency
Multi-intent
```

最终形成：

```text
User Input
      ↓
Intent
      ↓
Need / Goal / Constraint
      ↓
Assumption
      ↓
Action
```

这里最重要的一句话：

> **不是替用户做主，而是帮助用户把自己的意图表达得更完整。**

这就是你说的：

> “弥补用户思维缺陷。”

---

# S8 · Product Layer

等前面这些真正证明有效以后，才进入：

```text
Desktop
TUI
MCP ecosystem
Skills
UX polish
Visualization
Distribution
```

这时候 UI/UX 才有意义。

因为它服务的是：

```text
已经验证有效的 Cognition
```

而不是包装一个还不确定的系统。

---

# 四、还有一条“暗线”：SimUser / Evaluation

这条不能算普通功能。

它其实是：

# Hearth 的“免疫系统”

地图应该画成：

```text
                     HEARTH
                        │
        ┌───────────────┼────────────────┐
        │               │                │
       Host          Cognition         Product
        │               │                │
        └───────────────┼────────────────┘
                        │
                ┌───────┴───────┐
                │               │
             SimUser         Observer
                │               │
                └───────┬───────┘
                        │
                     Evidence
                        │
                     Feedback
                        │
                   Architecture
```

也就是说：

**测量体系不是一个开发阶段。**

它会逐渐成为 Hearth 的**持续基础设施**。

---

# 五、把你最关心的“设计—执行—使用”正式画进去

以后任何一个 Hearth 新能力，都必须过四层。

```text
                Idea
                  ↓
          ┌──────────────┐
          │ 1. Design    │
          │ 设计是否正确 │
          └──────┬───────┘
                 ↓
          ┌──────────────┐
          │ 2. Execution │
          │ 能不能真正做 │
          └──────┬───────┘
                 ↓
          ┌──────────────┐
          │ 3. Behavior  │
          │ 真实任务有效 │
          └──────┬───────┘
                 ↓
          ┌──────────────┐
          │ 4. Experience│
          │ 用户觉得好用 │
          └──────┬───────┘
                 ↓
              ACCEPT
```

任何一层失败：

```text
不继续堆功能
```

而是回到对应层解决。

---

# 六、而自研 Hearth Engine 的位置，会发生一个非常大的变化

以前：

```text
Hearth
 ↓
Hearth Core
 ↓
Loop
 ↓
Tool
```

未来：

```text
                       Hearth
                          │
                 ┌────────┴────────┐
                 │                 │
            Cognition            Host
                 │                 │
        ┌────────┼───────┐    ┌────┼────┐
        │        │       │    │    │    │
      Intent  Evidence Discovery Codex Goose ...
```

而：

```text
Hearth Engine
```

变成：

```text
                 Experimental Host
                        │
                 ┌──────┴──────┐
                 │             │
             Hearth       Other Host
             Cognition     Cognition
```

这样就产生了真正的保险机制。

---

# 七、这会让“自研底座失败”变成什么？

以前：

```text
Engine 失败
   ↓
Hearth 项目失败
```

以后：

```text
Engine 失败
   ↓
Host 选择失败
   ↓
换 Host
   ↓
Hearth 继续
```

甚至反过来：

```text
自研 Engine 某项能力明显超过成熟 Host
       ↓
HCC 记录证据
       ↓
未来再把这个能力抽出来
       ↓
成熟
       ↓
重新成为 Host Candidate
```

所以它不是：

> “永远放弃自研。”

而是：

> **取消自研 Harness 对整个项目的生杀大权。**

这区别非常大。

---

# 八、这张地图上，哪些东西已经有了？

我大概这样标：

```text
HOST
├─ Session              ████████░░
├─ Execution            █████████░
├─ Safety               █████████░
├─ Context              ███████░░░
├─ Extension            ████░░░░░░
├─ Observability        ███████░░░
└─ UX                   █████░░░░░░

COGNITION
├─ Intent               █████░░░░░
├─ Evidence             ██████░░░░
├─ Discovery            █░░░░░░░░░
├─ Memory               ████░░░░░░
├─ Learning             ███░░░░░░░
└─ Interaction          ███░░░░░░░

MEASUREMENT
├─ SimUser driver       ███████░░░
├─ Analyzer             ███████░░░
├─ Metrics              ██████░░░░
├─ Personas             ██░░░░░░░░
├─ Scenarios            ██░░░░░░░░
└─ Blind campaign       ██░░░░░░░░

PRODUCT
├─ CLI                  ████████░░
├─ TUI                  ░░░░░░░░░░
├─ Desktop              ░░░░░░░░░░
└─ Ecosystem            ██░░░░░░░░
```

这个百分比/格数只是**战略层示意，不是当前源码审计数字**。它表达的是成熟度结构，而不是精确项目进度。

---

# 九、真正的战略里程碑，不应该再用“版本号”定义

以后我更建议用这种东西：

### M0

**知道自己现在是什么**

↓

### M1

**定义 Host Contract**

↓

### M2

**选定成熟 Host**

↓

### M3

**Cognition MVP 真跑起来**

↓

### M4

**Discovery 开始发现原本不知道的问题**

↓

### M5

**Evidence 系统能够约束错误结论**

↓

### M6

**Memory + Learning 形成闭环**

↓

### M7

**用户真实体感明显改善**

↓

### M8

**Hearth 进入稳定产品阶段**

---

# 十、最重要的战略转变

我觉得我们过去几个月最大的错误，是把：

```text
“系统做得越来越复杂”
```

误认为：

```text
“系统越来越强”
```

实际上这两个完全不是一回事。

未来我们要追求的是：

```text
复杂度 ↑
            ×
可靠性 ↑
            ×
用户体感 ↑
```

而不是：

```text
代码量 ↑
文档量 ↑
模块数 ↑
```

所以以后每增加一个系统，都要回答：

> **它让 Hearth 哪一种能力真正变强了？怎么证明？用户为什么会因此感觉更好？**

答不出来，就不做。

---

# 十一、我现在给这张战略地图一个总名字

我会叫：

# **HEARTH · 1+1 Architecture**

### Standard Host + Hearth Cognition

而它的战略路线可以浓缩成：

```text
      阶段 0
   Core Revalidation
         │
         ▼
      阶段 1
      HCC 定义
         │
         ▼
      阶段 2
    Host Bake-off
         │
         ▼
      阶段 3
  Cognition MVP
         │
     ┌───┴────┐
     ▼        ▼
 Discovery   Evidence
     │        │
     └───┬────┘
         ▼
      Memory
         │
         ▼
      Learning
         │
         ▼
     Interaction
         │
         ▼
    Real User UX
         │
         ▼
     Productize
```

旁边始终有一条纵向的：

```text
SimUser
Observer
Evidence
Blind Campaign
    ↓
不断验证整个系统
```

---

## 最后一个非常重要的判断

你之前说：

> “我不知道期待的是灵感，还是更高维的视角。”

我现在越来越倾向于认为，你真正缺的并不是**更多功能创意**。

而是一个能够持续执行：

```text
发现未知
→ 提出候选解释
→ 找到证据
→ 反证
→ 组合
→ 选择
→ 实现
→ 使用
→ 从使用中发现新的未知
```

的**二阶系统**。

也就是说：

> **Hearth 最终可能不是“一个更聪明的 Agent”。**
>
> **而是一个帮助 Agent 和人类持续发现“自己还不知道什么”的系统。**

这也是为什么我会把 **Unknown Domain Discovery** 放在新战略的核心位置，而不是把它当成一个普通 feature。

而现在，**先不要施工。**

你刚刚说得对：P4 的独立冻结报告还没有最终落地。等它出来以后，我们拿这张战略地图去对照最终证据，再决定第一份真正的长程总包应该是 **HCC / Host Bake-off / Cognition MVP** 三者中的哪一个。

届时我们就不再“边走边想大方向”了，而是把这张地图正式变成 **Hearth v1.0 战略蓝图**，然后从蓝图里一次性切出长程施工总包。
