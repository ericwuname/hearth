# Hearth 可用性问题发现方案（矛盾论 + 头脑风暴驱动）

> 性质：纯分析 / **问题发现**。**不出 patch、不改 crates/**（用户指令：只分析、找问题、设计发现方案）。
> 配套：核心缺陷清单 `core-issues-summary-v0222.md`、对标诊断 `hearth-usability-diagnosis-vs-competitors.md`。
> 源码取证累计（✅）：L0.5/L2.5/L0.6/L1.5/L3.5/L2.6/L3.6/L5/L6（L6=模型仅在 Plan 调用、Act/Observe/Reflect 全程零模型调用，loop.rs:3237 单点接线实证）；**F2 resume ✅（CLI `hearth resume <id>` 已暴露并接 restore_history/restore_taskgoal）**；L3 计划可见性 修正；L2.7 新发现；F1 系统提示明文把预算自管甩给模型实锤；**L7 潜意识过度开火 已排除（ConstitutionGuard 仅对 3 条硬编码危险命令 Abandon，CostGuard/Repetition 仅 Simplify 不终止）**。

## 0. 方法论：为什么用「矛盾论 + 头脑风暴」
- **头脑风暴负责发散**：把"不好用"拆成尽可能多的候选裂缝，先求全、不求对，避免早收敛漏掉真问题。
- **矛盾论负责收敛**：在海量候选里识别**主要矛盾**（决定事物性质）与**主要方面**（矛盾该从哪侧解决），避免陷入"修不完的 bug 清单"式伪勤奋。
- 二者接力：头脑风暴产出候选 → 矛盾论判定哪些候选是主要矛盾的表现、该优先取证。

## 1. 矛盾论骨架（先定骨架，再发散）
**主要矛盾**：Hearth 的「控制导向架构」（为安全/可观测而约束模型）↔ 用户对「流畅自主助手」的期待。
- **普遍性**：所有 agent harness 都在这对矛盾里取舍（控制 vs 流畅）。
- **特殊性（Hearth 为何是离群值）**：它把矛盾**硬推向控制端**——Plan/Act/Observe/Reflect 全是同步闸门，每步都设栅栏；竞品把矛盾推向「护栏端」（流畅为主、越界才拦）。所以 Hearth 落在频谱极端 → 这是它"不如竞品好用"的**结构性位置**，不是某几个 bug 的偶然。
- **同一性（可解决性）**：安全与流畅**不互斥**（竞品已证），故矛盾可调和；Hearth 当前是"取舍配置错了"，非"不可能"。
- **主要方面在 Hearth 侧**：用户期待由市场锚定（固定），可改的是 Hearth 的契约配置。
- **次要矛盾（均为主矛盾的表现/后果）**：盲规划 vs 结果驱动(L0)、写文件完成 vs 答案交付(L1)、give_up vs 恢复(L2)、黑箱 vs 透明(L3)、工具人格 vs 助手人格(L4)。

## 2. 头脑风暴候选问题清单（发散，按矛盾方面标记 + 状态）
状态图例：✅ 源码实证 ｜ 🔶 分析推断（待真机复现）｜ 🆕 本轮新增。

### A. 推理 / 反馈回路（主要矛盾核心表现，L0 及其子面）
- **L0 盲规划**：每周期 1 次 LLM 调用（loop.rs:3159），模型在看不到工具结果下规划整批动作（pending_tool_calls @ 3271）。竞品单轮连续反应。
- 🆕✅ **L0.5 工具结果降质回灌**：`record_tool_exchange` 把工具输出经 `truncate_tool_output` 再入历史（loop.rs:2512）；该函数（loop.rs:5278）阈值 **MAX 6000 / HEAD 4000 / TAIL 1500**——超长输出**中间整段被砍**。模型下一轮 Plan 基于**残缺反馈**决策 → 盲规划叠加盲反馈。
- ✅ **L0.6 上下文预算更早撞墙 + 长程记忆降质（本轮坐实）**：`build_messages`（loop.rs:2195）每次 Plan 都重发**全量历史 + 一条巨静态 system prompt**（product 模式 ~800 词，2213-2258 行）；`maybe_compact`（2198 注释"不调 LLM，不阻塞"）是**规则式折叠**——旧轮变规则摘要而非 LLM 摘要。后果：① 每周期 payload 重 → 更早撑大上下文/撞预算；② 长跑后模型工作记忆被规则折叠**降质**，叠加 L0.5 截断 → 长任务在"越来越瞎"的上下文里决策。
- ✅ **L5 静态 prompt 膨胀 + 强敕令锁人格（本轮全文取证，2213-2272）**：控制靠"prompt 敕令"而非架构优雅双模。product 提示**逐字硬编码**以下内容，且每周期重生重发：
  - "You are a coding agent that **ONLY uses tool_calls (NEVER text)**"（2214）→ L4 工具人格、L1 锁死对话任务；
  - 6 步刚性工作流（2217-2230）：grep ONCE → read FULL → write_file → `cargo test` VERIFY → NEVER grep twice → write_file CONTENT LIMIT 3000 chars（超限 SPLIT）；
  - IDENTIFIER CONTRACT（2232-2236）：必须定义与用户所给**完全一致**的符号名，否则判 FAILED；
  - "CRITICAL… you **MUST end by calling write_file** with the complete new file content"（2254）→ L1 写文件即完成；
  - CARGO NOTES（2256-2258）：硬编码编译建议。
  机制源坐实：这条巨提示既是 L0.6 的 token 浪费源，又是 L1/L4 的**直接指令源**——模型被字面敕令钉死成"只写文件的工具"，对话/分析任务生硬。静态 prompt 无法随任务自适应。
- 🆕✅ **L6 盲批无依赖 + 模型全程冻结（本轮新发现，机制全证）**：`do_act` 对 `pending_tool_calls.len()>1` 走 `execute_plan`（loop.rs:3612，顺序 `for step`，`condition:Always`，**不读上一步结果**），否则 `execute_tool_calls`（3666 → `dispatcher.rs:340` `join_all` **并行**）。但 loop 构建 TaskStep 时 `capture:None`（3612-3626）——**orchestrator 本有的 `{{key}}` 跨步变量替换（resolve_vars / orchestrator.rs:147-167）被完全没接上**：上下游依赖型操作（read→edit、grep→write）在批量里**拿不到前一步输出**。
  - **更致命的结构确认（loop.rs:3237 单点接线 + do_reflect 全文 4112-4242）**：`provider.chat` 在 loop 内**唯一出口就是 `do_plan`**（注释原文"本函数是 loop 内唯一 provider.chat 出口，全部 LLM 请求必经"）。`do_act`/`do_observe`/`do_reflect` **三阶段均不调用模型**——`do_reflect` 仅是规则式遥测记账（错误计数/失败分类/进度跟踪），**模型不在那里"反思"**。
  - 结论：模型**每周期只被调用一次**，且从 `do_plan` 提交整批工具调用、到 `do_act` 盲跑完、到下一轮 `do_plan` 才重新评估——**中间整批工具结果模型一个都看不到**。这与 Codex/Claude Code/WB 的 **react-interleaved**（每个工具结果后立即回模型）是**结构性相反**的设计。即便是"每周期只发 1 个工具"的最佳情形，Hearth 仍每步重发 ~800 词提示+全量历史（L0.6），反应粒度仍粗于竞品且开销更高。
- 🔴→❌ **L7 潜意识层过度开火（本轮排查：已排除）**：`PhaseOverride::Abandon`（loop.rs:3048-3053）可不经模型直接 `return LoopPhase::Error` 终止 run。但经读 `subconscious/src/lib.rs` 全守卫：**`ConstitutionGuard` 仅对 3 条硬编码危险命令（`rm -rf /`、`rm -rf ~`、`del /S /Q C:\\`）Abandon**(66-77)；`CostGuard`/`RepetitionDetector` 只发 `Simplify`（>0.80/0.95 成本比、>70% 失败率），而 `Simplify` 在 loop.rs:3054-3057 **仅记 warning、执行继续、不终止**。故潜意识层是**正确实现的护栏**（fail-closed 只拦真危险），**不构成可用性断崖**。反向启示：Hearth 已有"护栏而非栅栏"的正确范式，矛盾论主修复方向（六相闸门降级为护栏）与此一致、可照搬其实现。

### B. 完成语义（L1）
- **L1 写文件即完成**：系统提示 loop.rs:2206-2214 "NEVER text"；`goal_requires_product` 真时纯文本不算 Done（3291），纯读类注入"必须 write_file 且构建绿"（3366-3371）。
- ✅ **L1.5 目标单点分类器（本轮坐实）**：完成契约全押在 `goal_requires_product(goal)`（loop.rs:167）——**纯字符串关键词分类**：先剥离 6 条负向短语（"不要修改"等），再匹配 34 条产物动词（"创建/修复/修改/重构…"）。脆弱点：同义/漏词即误路由——"优化这个函数"不含 34 词→误判 QA（本该写代码）；"写下你的结论"含"写"→误判 product 被逼写文件。负担与误判风险全压在用户 prompt 措辞上。

### C. 失败恢复（L2）
- **L2 give_up 即终止**：loop.rs:2820；出口散落（P1-8）。
- 🆕✅ **L2.5 非交互默认即 abort（细化：默认开启）**：`run_local.rs:332-339` 审批策略注入——`--approve-within session`→DelegateSession（自动放行）；**否则若 stdin 非 tty（管道/CI/agent-to-agent 调用）→ 默认 `DenyAllNonInteractive`** → 任一需审批的 bash/edit/apply_patch 立即终止 run（loop.rs:3485-3534）；终端交互（无 flag）→ 停留 `Interactive` 默认（loop.rs:1505）→ 每个破坏性工具弹审批、**逐工具打断流畅性**。即 L2.5 断崖对自动化是**默认开启**；仅 `--approve-within` 或显式信任才流畅。对比竞品 chat 默认信任/自动放行，Hearth 默认"每步栅栏"。
- ✅ **L2.6 replan 双重计步（本轮坐实）**：主驱动每周期 `inc_step`（loop.rs:5007）已 +1，而 forcing-replan 路径在 `do_plan_inner` 内**再** `inc_step`（3373 无写强制重规划 / 3403 无 tool_call 重提示）。即 replan 重任务比正向推进**多耗约 1 步/周期** → 预算耗尽更快（与 L0 盲规划互为放大器）。
- 🔶 **L2.7 恢复依赖验收 criteria（本轮新发现）**：预算耗尽/假完成处有核验恢复——FA01 旁路（loop.rs:4812，budget exhausted 先核验 acceptance 再 abort）、Verification Reserve（≤1 次，5062-5090）、T4 停滞。但**全部 keyed to `acceptance_criteria` 存在**。无显式 criteria 的开放式任务（"把它改好/优化下"类多数）落不到这些护栏 → 退回纯 give_up。恢复的覆盖面由契约是否带 criteria 决定。

### D. 感知 / 透明（L3）
- **L3 步数黑箱 + 计划可见性修正**（P0-δ）："✓ Done N steps" 无叙事。🔧 修正：Hearth **有**结构化计划透传——`PlanDraft` 事件（loop.rs:2686）emit 任务图 steps/gaps_to_ask/auto_assumed；但仅 decompose/replan 时发、且是机器结构化（非自然语言"因为 Y 所以做 X"）。故应改为"计划有但非叙事、且只在重规划点"。文件级 diff 仍缺（见 L3.5）。
- ✅ **L3.5 进度事件无"改了什么"diff（本轮坐实）**：`Event` 枚举（session.rs:1002-1074 映射）含 Phase/Token/ToolCall/ToolResult/ThinkSummary/PlanDraft/Artifact/Reflection/Done…，但**没有"文件改了哪几行"的结构化 diff 事件**。用户/Observer 只能看到"调了 write_file"或裸工具输出，看不到可读变更。
- ✅ **L3.6 终止无"可接手"叙事（本轮坐实）**：`Done` 报告（loop.rs:4802/4953）仅含 `ok/status/goal/elapsed/steps`——有状态串（如 deadline_exceeded）但**无"我试了 X、被 Y 卡住、你该怎么做"的接手叙事**。用户拿到"status + 步数"仍不知如何继续。
- **L3.7 中文 panic**（P0-ζ）：patch.rs:127/135/145 字节切片。
- **L3.8 误判告警**（P1-7）：call./done./glob. 被当文件名。

### E. 人格 / 契约（L4）
- **L4 工具人格**："NEVER text" 锁死，对话式任务生硬（机制源见 L5 / loop.rs:2214）。

### F. 战略层矛盾（🔶，最高杠杆的"元问题"）
- 🆕🔶 **F1 弱模型放大效应（与用户战略自相矛盾，本轮系统提示实锤）**：用户战略是"弱 LLM 下也交付高质量"；但 plan-execute-gated **最依赖强模型的盲规划能力**——弱模型盲规划更易错、更易 give_up。且**系统提示明文把预算/上下文自管甩给模型**：product 提示（2248-2250）写"Call `introspect()` when the task is long or you feel context pressure — then proactively summarize old steps or ask the user instead of hitting the budget"。这意味着架构**假定模型有强自省力**去主动管理预算——与"弱 LLM 也能用"战略直接打架。`introspect` 工具本身（status.rs:33）就是把上下文/预算自管**外包给模型的主动性**，弱模型不会主动调 → 撞墙即 give_up。需 Phase D 用免费/弱模型实跑验证放大系数。
- ✅ **F2 resume（本轮纠正此前"未证"降级——实为生产级已暴露）**：CLI 确有子命令 `hearth resume <id> [goal]`（codex-cli/src/lib.rs:644 `Commands::Resume`），处理器调用 `agent.restore_history(turns)`(680) + `restore_taskgoal`(702)，并对 crash 落在两次写盘之间的 `state_revision` mismatch 提供恢复路径（687-700，不阻断、标 stale warning）。故"run 中途完全无法续跑"**不成立**——生产续跑已实现。剩余风险（Phase D）：resume 后任务能否真收敛、长会话 JSONL 体积。此前"降级为未证实"的判断被源码推翻。

## 3. 问题发现方案（Discovery Plan）—— 阶段化、可复现
> 目标：把"我觉得不好用"变成"可复现、可计数、可对照竞品"的证据链。

- **Phase A 矛盾论定骨架**（§1 已完成）。
- **Phase B 头脑风暴发散**（§2 候选台账已完成）。
- **Phase C 源码取证（不信报告信源码）**：对每个 🔶 候选 grep/read 精确代码路径，判定 ✅/❌/🔶。已证：L0.5/L2.5（首轮）、L0.6/L1.5/L3.5/L2.6/L3.6/L5/L6（续证）；F2 与 L3 计划可见性修正；L2.7 新发现。
- **Phase D 真机复现（实验设计，交执行窗口跑）**——每类失败的最小触发 prompt + 抓取信号：
  - **L1/L1.5**："用一段话解释 X 模块的风险" → 预期触发 write_file 强制或 give_up；抓 loop.rs:3366 注入与 final verdict。再测"优化这个函数"→ 验证是否误判 QA 漏写。
  - **L0.5**：对 8000 字符编译错误跑 `bash("cargo build")` → 抓回灌历史的 middle 是否丢失关键行。
  - **L0.6/L5**：长跑任务 → 抓每周期 system prompt 体积与 maybe_compact 触发后的历史保真度。
  - **L2.5**：缺省非交互跑含 bash 的任务 → 预期 run_abort（loop.rs:3527）。
  - **L3.5**：正常编辑任务 → 抓事件流是否含 diff 类事件（预期无）。
  - **L6**：read→edit 依赖型两步任务 → 预期批量里 edit 拿不到 read 输出（capture:None 实锤）；对比竞品 react 单步可见。
  - **F1**：同一多步编辑任务分别用免费弱模型 vs 强模型 → 对比 give_up 率与步数。
  - **F2**：长跑任务中途 kill → 验证是否有 CLI resume 且能无损续跑。
- **Phase E 竞品对照基线**：对每类失败，记录 Claude Code / Codex 的"应然"行为（react 单轮、答案即交付、同轮重试、可读 diff、resume）。
- **Phase F 综合定级**：按（体感影响）×（是否主要矛盾直接表现）排序，输出"问题在哪"的最终答案。

## 4. 矛盾论视角下的「抓主要矛盾」
- 主要矛盾 = 控制导向 vs 流畅期待；**主要方面（该改侧）= Hearth 的"每步栅栏"配置**。
- 最高价值的发现/修复方向只有一个：**把栅栏降级为护栏**（对标诊断文档 #1）。其余 L1–L4 均为主矛盾的派生表现——修了它们但不动 L0，体感仍差；动了 L0，L1–L4 大部分自然消解（react 循环里"答案即交付 / 同轮重试 / 可读"本就是默认）。
- **反直觉的关键洞察**：L0.5（降质回灌）比 L0 更隐蔽、更该先取证——它让"即使是 react 也白搭"，因为反馈本身就是瞎的。先证 L0.5，再谈 L0 改造。
- **L1.5 的警示**：完成契约的"开关"是一个 34 词关键词分类器（loop.rs:167）。这意味着"问题出在哪"的一个具体抓手——**契约路由本身就很脆**，很多"不好用"是分类器误判把对话/分析任务逼成写文件任务。这是主要矛盾在"入口"处的显影。
- **L6 的定性升级（本轮）**：L0 不再是"每周期 1 次调用"的孤立现象——它根植于 `provider.chat` 单点接线（3237）+ 三阶段零模型调用。这意味着 Hearth 与 react-interleaved 的差距是**架构级**而非参数级：要让模型逐结果反应，必须打破"单点 chat 出口 + 六相串行闸门"，而非调几个常量。

## 5. 问题台账（ledger）
| 候选 | 矛盾方面 | 状态 | 取证动作 |
|---|---|---|---|
| L0 盲规划 | 主/控制端 | ✅ | loop.rs:3159 / 3271 |
| L0.5 工具结果降质 | 主/反馈 | 🆕✅ | loop.rs:5278 / 2512 → Phase D |
| L0.6 上下文早撞墙+记忆降质 | 主 | ✅(本轮) | build_messages 2195 / maybe_compact 2198 |
| L5 静态prompt膨胀+强敕令 | 主/控制机制 | ✅(本轮全文) | loop.rs:2213-2272 NEVER text/MUST write_file/6步工作流 |
| L6 盲批无依赖+模型冻结 | 主/反馈回路 | 🆕✅ | do_act 3612/3666 capture:None；provider.chat 单点 3237；do_reflect 4112 零模型 |
| L1 写文件完成 | 次/L1 | ✅ | loop.rs:3291 / 3366 |
| L1.5 目标单点分类 | 次/L1 | ✅(本轮) | goal_requires_product 167（34动词+6负向） |
| L2 give_up | 次/L2 | ✅ | loop.rs:2820 |
| L2.5 非交互审批abort | 次/L2 | 🆕✅ | loop.rs:3485-3534 |
| L2.6 replan双计步 | 次/L2 | ✅(本轮) | 5007驱动+3373/3403内增 |
| L2.7 恢复依赖criteria | 次/L2 | 🔶(本轮) | FA01@4812 / Verification Reserve 5062 keyed to acceptance_criteria |
| L3 步数黑箱 | 次/L3 | ✅ | P0-δ |
| L3.5 无diff事件 | 次/L3 | ✅(本轮) | Event 枚举审查（session.rs:1002-1074） |
| L3.6 终止无接手叙事 | 次/L3 | ✅(本轮) | Done报告 4802/4953 |
| L3.7 中文panic | 次/L3 | ✅ | patch.rs:127/135/145 |
| L3.8 误判告警 | 次/L3 | ✅ | P1-7 |
| L4 工具人格 | 次/L4 | ✅ | loop.rs:2214 |
| F1 弱模型放大 | 战略 | 🔶(系统提示实锤) | 2248-2250 introspect 自管预算；Phase D 弱模型实跑定级 |
| F2 resume | 战略 | ✅(本轮纠正) | CLI `hearth resume <id>` lib.rs:644 → restore_history 680 + restore_taskgoal 702 + state_revision 恢复 687-700 |
| L7 潜意识过度开火 | 主/控制 | ❌已排除 | ConstitutionGuard 仅 3 危险命令 Abandon(lib.rs:66-77)；CostGuard 仅 Simplify 不终止 |

## 6. 本轮 Phase C 续证结果（明细）
- **L1.5 坐实**：`goal_requires_product`（loop.rs:167）实现 = 小写化 → 剥离 6 负向短语 → 匹配 34 产物动词。纯关键词，无语义理解。误路由风险真实存在。
- **L0.6 坐实**：`build_messages`（loop.rs:2195）每次 Plan：`maybe_compact()`（规则式，不调 LLM）+ 重建含 ~800 词 product 系统提示（2213-2258）+ 全量历史。每周期重发 → 上下文/预算更早吃紧；规则折叠使长程记忆降质。
- **L3.5 坐实**：`Event` 变体（session.rs 映射 1002-1074）无文件 diff 事件；只有 ToolCall/ToolResult 裸输出。用户看不到"改了什么"。
- **F2 修正**：core 有 `restore_history`/`on_turn_checkpoint`/resume 测试；"完全无 resume" 过度断言，降级为生产续跑未证实。

## 7. 本轮续证（三）：步计数 / 终止叙事 / 恢复覆盖 / 计划可见性
- **L2.6 坐实（双重计步）**：主驱动每周期 `inc_step`（loop.rs:5007）；forcing-replan 在 `do_plan_inner` 内再 `inc_step`（3373/3403）。replan 周期步数 ×2 → 盲规划(L0) 越多 replan，预算烧得越快。
- **L3.6 坐实**：`Done` 报告（4802/4953）仅 ok/status/goal/elapsed/steps，无接手叙事。
- **L2.7 新发现**：恢复（FA01@4812、Verification Reserve、T4）全部 keyed to `acceptance_criteria` 存在；开放式无 criteria 任务落不到 → 纯 give_up。恢复覆盖面由契约带不带 criteria 决定。
- **L3 计划可见性修正**：`PlanDraft`（2686）确 emit 结构化任务图，故"完全看不到计划"过度——修正为"计划有但机器结构化、且只在 decompose/replan 发"。

## 8. 本轮续证（四）：react-interleaved 缺位坐实 + 系统提示全文取证（L6/L5/F1 收口）
- **L6 收口（架构级根因）**：
  1. `provider.chat` 在 loop 内**唯一出口 = `do_plan`**（loop.rs:3237 注释"本函数是 loop 内唯一 provider.chat 出口，全部 LLM 请求必经"）；grep 全文仅 3159（do_plan）与 1324（restore_history，非主循环）两处 chat。
  2. `do_act`（3413）/ `do_observe`（3867）/ `do_reflect`（4112）**均不调用模型**：`do_reflect` 经全文核验为规则式遥测记账（错误计数、failure 分类、progress 跟踪、plan_state 更新），**模型不在 Reflect 相"思考"**——所谓"六相"里模型实际只参与 1 相（Plan）。
  3. `do_act` 批量执行：`pending_tool_calls.len()>1` 走 `execute_plan`（3612，顺序、condition:Always、不读上步结果），否则 `execute_tool_calls`（3666→dispatcher.rs:340 `join_all` 并行）；TaskStep `capture:None`（3624）使 orchestrator `{{key}}` 跨步变量替换（orchestrator.rs:147-167）**完全没接上** → 依赖型两步（read→edit）在批量里拿不到前步输出。
  4. **结论**：模型每周期被打一次，提交整批后盲跑，下一周期才重估——与竞品 react-interleaved（每结果即回模型）结构性相反。L0 是架构事实，非调参可解。
- **L5 收口（全文取证 2213-2272）**：product system prompt 逐字硬编码"ONLY uses tool_calls (NEVER text)"（2214）、6 步刚性工作流（2217-2230）、IDENTIFIER CONTRACT（2232-2236）、"MUST end by calling write_file"（2254）、CARGO NOTES（2256-2258）。是 L1/L4 直接指令源 + L0.6 token 浪费源。每周期重生重发（build_messages 2195）。
- **F1 收口（系统提示实锤）**：product 提示（2248-2250）明令模型"长任务/上下文压力大时调 `introspect()` 主动总结旧步骤或询问用户，而非撞预算"——架构把预算/上下文自管**外包给模型主动性**；`introspect`（status.rs:33）即此通道。弱模型不主动调 → 撞墙 give_up，与"弱 LLM 也高质量"战略直接冲突。弱模型放大系数待 Phase D 实跑定级。

## 9. 本轮续证（五）：F2 纠正 + L2.5 默认开启 + L7 排除（用户"继续分析"）
- **F2 纠正（此前降级误判）**：CLI 确有 `hearth resume <id> [goal]` 子命令（codex-cli/src/lib.rs:644 `Commands::Resume`）；处理器 `restore_history(turns)`(680)+`restore_taskgoal`(702)，并对 `state_revision` mismatch（crash 落两次写盘间）提供恢复路径(687-700，不阻断)。→ F2 升 ✅（生产续跑已实现），此前"原语存在、生产未证"被源码推翻。剩 Phase D 验证：resume 后能否真收敛、长会话 JSONL 体积。
- **L2.5 细化（默认开启）**：`run_local.rs:332-339`——非 tty 调用（管道/CI/agent-to-agent）**默认 `DenyAllNonInteractive`** → 首条需审批 bash/edit 即 abort（loop.rs:3485-3534）；终端交互（无 flag）停留 `Interactive`(loop.rs:1505) → 每破坏性工具弹审批、逐工具打断；仅 `--approve-within session` 自动放行。即 Hearth 默认"每步栅栏"，与竞品 chat 默认信任相反。
- **L7 排除（潜意识层非断崖）**：`PhaseOverride::Abandon`(loop.rs:3048) 可不经模型直接终止 run；但 `subconscious/src/lib.rs` 全守卫核验——`ConstitutionGuard` 仅对 3 条硬编码危险命令 Abandon(66-77)，`CostGuard`/`RepetitionDetector` 仅 `Simplify`（记 warning、执行继续、不终止，loop.rs:3054-3057）。→ 潜意识层是**正确实现的护栏**，不构成可用性断崖。反向启示：Hearth 已有"护栏范式"，主修复方向（六相闸门降级为护栏）可照搬其实现。

> 下一步（仍只分析）：① 剩余真正 🔶：L2.7 恢复覆盖（keyed to acceptance_criteria，需验证开放式任务落点）；F1 弱模型放大（Phase D 实跑定级）；② 把 Phase D 实验脚本化交执行窗口；③ 或就"模型冻结/单点 chat 出口 + 默认每步审批"做一页架构级改造**问题描述**（仍不出 patch）。
