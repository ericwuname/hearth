# Agent 循环上限 · 成熟 Harness 做法调研 v1.0

- **日期**：2026-09-12　**动因**：用户指出 EMBER 的"20 轮工具循环上限"导致"每个 step 代码写完、没余量跑自测就停"，要求**先调研成熟 harness 的做法，不许猜**
- **方法**：网络检索（Anthropic 官方文档 / SWE-agent 文档 / OpenHands 架构分析 / Loop Engineering 综述 / 生产实践博客），**结论均带出处**

---

## 一、轮次上限：各家用什么值

| 来源 | 上限 | 说明 |
|---|---|---|
| **Claude Code（CLI）** | **50 turns** 默认 | 超限报 `Maximum turns exceeded (50)`；`--max-turns 100` 或 `CLAUDE_MAX_TURNS=100` 可调（[来源](https://claudecodeguides.com/claude-code-max-turns-exceeded-fix-2026)） |
| **Claude Code（Agent SDK）** | **默认无限制** | 可选 `max_turns` / `max_budget_usd`；达限返回 `ResultMessage` 带 `error_max_turns` / `error_max_budget_usd` 子类型（[来源](https://docs.anthropic.com/en/docs/agent-sdk/agent-loop)） |
| **SWE-agent** | **50 个动作** | 官方自陈短板：*"50 上限对错别字修复绰绰有余，但**任何需要读第三个文件的 bug 都很紧**，跨文件重构经常在编辑途中被截断"*（[来源](https://marovi.ai/SWE-agent/zh)） |
| **LangGraph** | **25 步** 默认 | Loop Engineering 综述引用（[来源](https://ithub.global.ssl.fastly.net/op7ic/Loop_Engineering_Skill)） |
| **学术/实践综述** | **"scoped task ≈ 15–25 步"**；**SWE-agent 成功中位数 ≈ 12 步** | 同上 |
| **生产实践建议** | **"Typical production values are 15 to 25 steps"** | [来源](https://github.com/stevekinney/stevekinney.net/blob/main/writing/agent-loops.md) |

**读数**：业界带工具的 agent 上限集中在 **15–50** 步；**EMBER 的 20 在区间内偏低**，Claude Code CLI 取的是 50。

## 二、⭐ 到限了怎么办（这是本次调研的核心）

### 2.1 Claude Code 的七条终止路径（不止一条）

`completed` · **`max_turns`** · `aborted`（用户中断）· **`max_budget`** · `blocking_limit`（Token 上限）· `prompt_too_long`（上下文过长）· `stop_hook`（外部阻止）
—— 且达限时**返回结构化结果**（`error_max_turns`），**不是静默停止**；用户可 `claude "continue"` 续跑。

### 2.2 生产实践的关键手法：**"early stopping generate"**（早停综合）

> 到限时**不要直接停**，而是追加一条消息：
> *"You've reached the maximum number of steps. Provide your best answer now based on the work you've done so far."*
> **再调一次 LLM（不带工具）**，让模型把已做的工作**综合成交付物**，而不是让用户两手空空。

### 2.3 SWE-agent 的收口方式

- 循环在 **`submit`（模型主动交卷）/ 成本或轮次预算耗尽 / harness 出错** 时结束；
- **`submit` 是"计划内出口"**，预算上限是**兜底（backstop）**；
- 每次 LLM 调用**记录 token/费用/轮次**，产出 CSV（**可复现的成本账**）。

## 三、⭐ 更重要的一条：**加轮次不是正解**

Loop Engineering 综述的原文（两句都很硬）：

> **"If a scoped task is not converging near its median success step count, more steps rarely rescue it. Raise caps only after adding checkpointing and external verification, never to paper over their absence."**
>
> **"Hitting the cap: the task probably needs decomposition into smaller verifiable sub-goals, not more iterations. Restructure; do not add passes."**

**Claude Code 官方指南**也明文给出同类建议：

> *"Decompose tasks requiring more than **30 file operations** into sequential subtasks. Each subtask should target a single module or concern. **Use explicit step numbering (step 1, step 2)** to maintain progress across sessions."*

### 长任务的另外四件（Anthropic 博客针对"长运行 agent 失效模式"）

| 失效模式 | 对策 |
|---|---|
| 过早宣布项目胜利 | **建立功能清单文件**（结构化 JSON），会话开始先读它、**只挑一项做** |
| 留下带 bug / 未记录进展的环境 | **初始化 git + 撰写进展笔记文件**；会话开始读进展与提交日志、跑基础测试 |
| 过早把功能标记完成 | **自我验证后才标"通过"**（且要有外部验证器） |
| 每轮都要重新研究怎么跑应用 | 写 `init.sh`，会话开始读它 |

### 循环健康度检查（生产实践）

| 检查 | 阈值/做法 |
|---|---|
| **Doom loop（死循环）** | hash 每轮 `(tool_name, result_preview)`，**连续 3 次相同指纹 = 卡死**（"有系统重复同一答案 58 次才被发现"） |
| **连续失败断路** | **连续 2–3 次失败就跳闸**（重复失败动作是 doom loop 签名） |
| **墙钟超时** | 约 **300 秒** |
| **成本上限** | 约 **$2.00/run**（SWE-agent 默认 ≈$3/instance） |
| 成本曲线 | 观察 token 是线性增长（正常）还是**平方增长**（每轮重发全上下文、无压缩 → 会炸预算） |

## 四、对照 EMBER 的差距与改进清单

| 维度 | 业界做法 | EMBER 现状（2026-09-12） | 建议 |
|---|---|---|---|
| 轮次上限 | 15–50（**可配**） | **20，硬编码** | ① 改为**可配**（env/参数）；② 默认提到 **25–30**（对齐 LangGraph/生产建议；不必一步跳到 50） |
| 到限行为 | **早停综合**（不带工具再调一次）+ **结构化终态** + **可续跑** | 打印一行"达到上限，请缩小范围"即停 ✗ | ① **加"早停综合"**（成本 ≈ 1 次调用，收益最大）② 终态结构化（`rounds_exhausted`）③ 提示 `-c "继续"` 可续跑（**EMBER 已有 `-c` + M5.3 断点 ✓ 天然支持**） |
| 任务分解 | **官方明文**（>30 文件操作要拆；带步骤编号） | 靠人工拆卡（本日实践：≤15 步/卡，有效） | 把"**每卡 ≤15 步**"写成硬约束（**与业界 15–25 读数吻合**） |
| checkpoints | 进展笔记文件 + git | **`.ember/last_action.txt`（M5.3 ✓）** | 扩展为"进展笔记"（累计做了什么/产物/结论），会话开始自动读 |
| 循环检测 | 连续 3 次相同 `(tool,result)` 指纹 | 无 | 加（实现简单，防"58 次重复"） |
| 连续失败断路 | 2–3 次跳闸 | 只有 API 层重试（3 次） | 加工具层断路器 |
| 成本/轮次账 | 每轮记 token/费用 → CSV | 无（只有 Agnes 侧统计） | 加轻量账（轮次/token 累计，落 stderr 或 run report） |

## 五、结论（给用户的三句话）

1. **20 确实偏小**：业界带工具 agent 集中在 **15–50**，Claude Code CLI 默认 **50**；可先调到 **25–30** 并**开放配置**。
2. **但"加轮次"不能替代"分解 + 外部验证"**——这是调研里最硬的一条（综述原文：加轮次很少能救不收敛的任务；该分解，而不是加轮次）。我们今天的"**≤15 步/卡**"方向与业界读数一致，**该保留并写成硬约束**。
3. **最该立刻补的一件是"早停综合"**：到限时**不带工具再调一次模型**，要求它"基于已完成的工作给出最好的交付"——成本仅一次调用，直接把"半成品停住"变成"尽力交付"；EMBER 已有 `-c` 与断点文件，**续跑链路天然具备**。
