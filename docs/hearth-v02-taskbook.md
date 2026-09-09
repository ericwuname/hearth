# Hearth v0.2 任务书 · 正式签发版（功能级对标 Top-3，目标 80–90 分）

> ┌─ 签发栏 ───────────────────────────────────────────────────────────
> │ 版本：v0.2.0（正式签发版）
> │ 签发日期：2026-08-23
> │ 签发人：顶层守门员（全局架构 / 评审 / 验收 / 任务书）
> │ 接收方：执行窗口（照任务书写码、跑门禁、交证据）
> │ 依据文档：docs/hearth-harness-benchmark-top3.md
> │   · §7 轻量接入方案　· §8 验收门禁　· §9 deepseek prompt 格式　· §11 能力记分卡
> │ 上游纠偏：fork 回归非模型上限 —— Codex CLI v0.149.0（2026-08-19 Apache-2.0 开源），
> │   hearth 即其 fork；同模型（deepseek v4 flash）在 Codex 表现远好于 hearth = 集成层回归
> │ 过闸口径：🔴阻塞 / 🟡遗留 / 实现率≥0.9；守门员独立核验（不信报告信源码）
> │ 前置任务书：docs/hearth-harness-hardening-v013-taskbook.md（v0.1.3 快速修复，先于本版下发）
> │ 收口状态：✅ 已收口（WS7/WS8/WS9/WS10 决策全闭合；web_search 后端定 A（OpenAI）；可下发执行窗口开工）
> │ 补充令：v0.2.1 已发（盲区并入，WS11 会话韧性 / WS12 长内容生成 / WS13 确定性验证；见 §9，不阻塞 WS1–10）
> └────────────────────────────────────────────────────────────────────

---

## 0. 目标与量化判据

- **成功判据**：top-3 若 95–100 分，hearth v0.2 须达 **80–90 分** 等效能力。量化见 §1 能力记分卡与 §3 总验收门禁。
- **评分口径**：80–90 分 = 核心编码能力对齐，不要求字面每一项满分。"多智能体/子代理"维度 top-3 有、本版不做（Aider 同样无子代理仍是顶尖，证明不影响核心达标）。
- **工程量**：**不设上限**。以达成能力目标为准；若实现目标需要更多代码（真正的 compaction、真正的 diff applier），应实现，不为省行数砍能力。范围纪律按"单人稳定生产"守，不按行数卡。
- **模型**：**不换模型**（deepseek v4 flash）。同一模型在 Codex 上游表现远好于 hearth，差距在 harness 设计，不在模型。prompt 格式是否对 deepseek 最优须实测（见 §6 自查 #4）。

---

## 1. 能力记分卡（v0.2 必须达成项）

| # | 维度 | 必须达成（≈80-90 分等效） | 阻塞达标? |
|---|---|---|---|
| 1 | 终止 | 删 B1 写文件闸，纯问答≤3步，无误触发 | 否 |
| 2 | 规划 | planner 瘦身保 Plan 阶段，空响应静默降级单步 | 否 |
| 3 | 上下文 | 阶段一 rg/ls/cat + **轻量 compaction（必做）**；repo-map 阶段二加分 | compaction 必做 |
| 4 | 编辑 | 自实现 diff applier，消灭整文件截断 | 否 |
| 5 | 沙箱 | 已有，修 B6 误杀 verify | 否（已接近） |
| 6 | 续接 | 复用 transcript+session_id+resume（fork/rollback 可不做） | 否 |
| 7 | 子代理 | **本版不做**（延后） | 不影响核心 |
| 8 | Hooks | 轻量两条（fmt/clippy + 拦生产命令） | 否 |
| 9 | 记忆 | Hearth.md 一层项目约定 | 否 |
| 10 | 可观测 | 补全 EnvelopedEvent 信封 + 保 transcript（可超额） | 否 |
| 11 | 判断力 | WS7 事实判断力：load-bearing 断言先验证，结论带 provenance，不盲信用户陈述/网络片段（不新增独立"判断力模块"） | 否 |
| 12 | 内观/体感 | WS8 体感闭环：LLM 可主动 introspect 身体状态；异常/疲劳主动上报（非仅失败才报）；下行指令带回执 | 否 |
| 13 | 预算价值化 | WS9 预算按目标方向分上/中/下策略倾斜资源；超阈值预警+询问而非硬截断；仍有安全上限（ask 边界） | 否 |
| 14 | 联网能力 | WS10 受控 web_fetch（Phase1，G0 出网治理+与 WS7 provenance 接线）；web_search 后端**已定 A（OpenAI，Phase2）**，C 作无 key fallback | 否 |
| 15 | 会话韧性 | WS11 每轮自动落盘 + panic 隔离（单 turn fail 不杀 thread）+ 重启自动恢复上次会话（升级维度6"续接"为"韧性"） | 🔴 阻塞达标 |
| 16 | 长内容生成 | WS12 流式累积 + 自动分块，5000字长文稳定完成（补齐维度4 diff 编辑之外的"从零生成长文"路径） | 🔴 阻塞达标 |
| 17 | 确定性验证 | WS13 写盘后校验产物（存在性→字节数→可运行），失败回喂 loop 重来，agent 自报 Done 须过 verify | 🔴 阻塞达标 |

---

## 2. 工作流（WS1–WS13）

### WS1 循环与续接（B1 + B2）—— v0.2 第一优先
- **目标**：反应式终止 + 会话续接。
- **顶层决策（做法）**：
  - 终止：删 `loop.rs:1217/1423/1458/1472` 的"未写文件→强制 replan"闸；加"模型发 done / 无工具调用即停"终止逻辑。纯问答必须 ≤3 步结束，且不冒 `tool not found:bash`。
  - 续接：复用已有 `transcript.rs`（已逐步骤记录）→ 加 `session_id` 概念；新增续接路径（自动续接上次 session 或 `/resume`），把历史 transcript 回灌进上下文。SQLite 或纯 JSONL 均可，量级远小于上游 `codex-thread-store`。
- **验收门禁**：V2-1（终止）+ V2-2（续接）。
- **范围外**：fork / rollback / archive（上游有，本版不做）。

### WS2 diff 编辑（B4）
- **目标**：消灭整文件截断病根。
- **顶层决策（做法）**：在 hearth `tools` crate 自实现轻量 diff applier——**优先 SEARCH/REPLACE**（Aider 证明无需 Lark），或轻量 unified-diff；**事务应用**（任一 hunk 失败整片拒绝并回显错误）；模型输出 diff → harness 应用。保留 `write_file` 整文件作为兜底，但默认走 diff。
- **验收门禁**：V2-3（11834 字节文件完整落盘，复用 v0.1.1 贪吃蛇证据）。
- **范围外**：抽取上游 Lark `apply_patch`（内嵌资产、紧耦合 codex 协议，不抽）。

### WS3 planner 瘦身 + 按需上下文阶段一（B3 + 上下文）
- **目标**：消除重型 upfront planner 空响应卡死；默认不拉全仓。
- **顶层决策（做法）**：
  - planner 改为**可选输出规划草案**；空响应 / 解析失败时**静默降级到单步模式**，不卡 plan 阶段、不阻塞主循环。Plan 阶段必须保留（四阶段白盒差异化核心，删=剩三段）。
  - 移除重型 upfront 全仓拉取；默认按需 `rg`/`ls`/`cat`。**阶段一只做 rg/ls/cat，不做 repo-map**（repo-map 见 WS3 阶段二 / 可放 v0.3）。
- **验收门禁**：V2-4（planner 瘦身）+ V2-5（按需上下文阶段一）。
- **范围外**：tree-sitter + 引用排序的 repo-map（阶段二，加分项，可延后）。

### WS4 轻量 compaction
- **目标**：长会话不 token 溢出腐烂。
- **顶层决策（做法）**：旧轮次摘要（保架构决策 / 修改文件 / 验证状态 / 开放 TODO）；触发阈值（如上下文 ~80% 利用率）；压缩前用 Compact Instructions 保关键信息，避免丢架构决策。
- **依赖**：WS4 应先于"提高步数预算"——在 compaction 落地前步数上限保持克制（~20 + 早轮裁剪）；compaction 后可提到 30–40。
- **验收门禁**：V2-6。

### WS5 Hooks + 路径放宽 + 项目记忆
- **目标**：确定性护栏 + 可用性。
- **顶层决策（做法）**：
  - 轻量 Hooks 两条：① 编辑后跑 `fmt`/`clippy`；② 拦截生产 / 危险命令（确定性护栏，参考 Claude Code 架构签名）。
  - B5 路径放宽：允许项目内 / 家目录绝对路径（`grep.rs:84-89` 当前一律拒绝，改为放行项目内与家目录）。
  - 项目记忆：引入 `Hearth.md`（等价 AGENTS.md / CLAUDE.md），启动时读一层项目约定 + 失败教训，进系统提示。
- **验收门禁**：B5 验收 + Hook 两条生效 + Hearth.md 进系统提示。

### WS6 白盒兑现（四阶段相位 + EnvelopedEvent + Gap）
- **目标**：把 hearth 相对三家的差异化真正落地（非空壳）。
- **顶层决策（做法）**：
  - Plan→Act→Observe→Reflect 相位钩子接真（非空壳），每轮留下"观察/反思"痕迹。
  - `AgentEvent` 补信封字段：`ts` / `seq` / `span_id` / `parent_id` / `schema_version`（当前仅 8 变体、无信封字段，无法表达 span 树）。
  - Gap 三元组结构化上浮：显式"我搞不定 / 这是不确定的"作为一等输出。
- **验收门禁**：V2-7。
- **注意**：这是 hearth 的护城河，三家都没有，**死守不删**。

### WS7 事实判断力与验证能力（新增 · 用户原话"说什么信什么、搜什么信什么=没判断力"）
- **目标**：agent 不盲信用户陈述的"事实"与联网检索片段；影响正确性的 load-bearing 断言落地前须至少一次核验。
- **铁律（用户定调）**：用户**目标/意图=权威**（照做）；用户提供的"事实"、联网检索内容=**默认待验证断言**（影响正确性者须先核验）。信任意图、不信任断言——避免两端（盲信 vs 啥都问"你确定吗"）。
- **顶层决策（做法，复用 WP-0/四阶段/Gap/G2-G3，零内核改动）**：
  1. Claim 溯源+置信标签：agent 摄入事实性断言（用户消息/网络片段/文档）时打 source+verifiable 标签；落上下文侧信道（不新增 crate）。
  2. 验证动作进 Observe/Reflect 相位（WS6 白盒钩子）：load-bearing 断言落地前须一次核验——读源码/跑命令/查官方 primary。
  3. 联网信任策略：优先 primary source；load-bearing ≥2 源交叉；标时效/版本；安全相关不采信单一博客。
  4. 新增 `unverified_claim` Gap 类型（复用 agent-types Gap，lib.rs:118；R10 已支持 options/style）——agent 要陈述未核实事实时，要么先验证、要么以"未验证"标签上浮。
  5. Constitution（G3 constitution.md）加 trust-but-verify 原则（≤10 条内）。
  6. 判例卡（G2 experience）记负向验证（"我以为 X，实测 Y"）。
  7. 回复层 provenance：结论带出处；带不出说"未验证"。
- **边界**：LLM 不能做完美怀疑者；机制=流程+工具+溯源+对 load-bearing 强制验证，不是单独"判断力模块"。
- **验收门禁**：V2-9。

### WS8 体感闭环 / 内观（新增 · 用户隐喻"LLM=大脑，Hearth=身体，如高功能瘫痪"）
- **目标**：大脑（LLM）能"感觉"身体（Hearth）真实状态；身体如实丰富上报（痛觉/疲劳），双向互通丝滑。
- **源码印证（现状）**：已有 observer/ crate（第三端口雏形 circuit/metrics/rules）、EnvelopedEvent 信封总线（api/src/lib.rs:103）、get_status（agent-runtime/src/session.rs:505）；但**无 LLM 可主动 introspect 的工具**，身体仅在失败/终止时报状态（budget_exhausted/error），平时静默→印证"瘫痪"。
- **体感闭环三通道（均长在现有架构，零/低内核改动）**：
  1. 上行/传入（体感·痛觉·疲劳）：Observer OS（observer/ crate）=周围神经系统，从"断路器"扩为"持续体感监视器"，发结构化 `BodyState`（工作区/工具健康/provider 抖动/上下文填充）上 EnvelopedEvent 总线。
  2. 下行/传出（指令+结构化回执）：每条指令返回 received/executing/done/failed-detail，不静默。
  3. 主动体感查询：`introspect`/`status` 工具（包 get_status，session.rs:505 现仅 CLI/服务用，未暴露 LLM）=闭眼想象身体图。
- **身体映射（用户隐喻→Hearth）**：四肢=工具/执行器（曾 `tool not found:bash`=身体不知自己有没这肢体）；脚底=工作区/环境；痛觉=卡顿/异常/provider 抖动（R7/R9 deepseek 超时，身体该喊痛而非静默失败）；呼吸=预算/上下文窗口（现只"Giving up after N steps"，无渐进疲劳信号）；神经=LLM 通道；自主边界=权限/审批门。
- **落地**：① Observer crate 扩为持续体感监视器→BodyState 上总线 ② 暴露 introspect 工具给 LLM+教 prompt 用 ③ 痛觉主动事件（修 R7 的 Done 伪装+gemini 误导，身体须对大脑诚实）④ 下行回执 ⑤ Constitution(G3)加"身体状态须对大脑可读+异常主动上报" ⑥ 丝滑=长期迭代轴（低反馈延迟/结构化可解析/双向 ack）。
- **与 WS7 互补**：判断力=大脑怀疑（不盲信）；内观=身体诚实+大脑感知。验证(判断力)需身体提供 ground truth（跑命令→真实退出码而非假设）；大脑唯有能"感觉"身体真实状态才能判断好。缺一：判断力无内观=盲；内观无判断力= gullible。
- **验收门禁**：V2-10。

### WS9 预算价值化（新增 · 用户原话"预算不是限制，是知道资源有限，把有限资源发挥最大价值"）
- **目标**：预算从"硬上限→任务被杀"改为"按目标方向的价值最大化分配"；超阈值预警+询问而非直接放弃；仍保留安全上限（ask 边界，非静默死）。
- **现状对照（源码）**：`Budget` 是单一硬 `max_steps`（loop.rs Budget 字段:263；default:405）；由 `budget_exhausted()`（loop.rs:2454）发 `Error("budget exhausted")` + "Giving up after N steps"（loop.rs:2315）**静默终止**；REPL_BUDGET=40（repl.rs:10）是全局硬杀数。无分档、无偏离预警、无询问。
- **用户模型（定调）**：评估项目→分上/中/下策略；按目标方向倾斜——如前端页面：布局美化=上策、辅助=中策、赶进度=下策；规划上策100/中策50/下策20（相对单位）；执行超 50%/100% → 预警"预算评估偏离" + **询问继续或重评估，非直接放弃**。
- **顶层决策（做法）**：
  1. Budget 扩分档：`tier: Premium|Standard|Economy` + `allocated_units` 每子任务 + `deviation_warn_at:[0.5,1.0]`；REPL_BUDGET 变默认档基线，非全局硬杀。
  2. 改 `budget_exhausted()` 静默死→跨 50%/100% 发 `BudgetDeviation` 事件 + **InteractionRequested(kind="budget_reassess", options=["继续","重新评估"])**（复用 WP-0/InteractionRequested loop.rs:1177 + Gap options agent-types:118；**内核不加 kind 分支**）。
  3. planner 按目标方向给子任务打 tier（复用 planner + Gap）；主目标子任务拿高分配，辅助拿低。
  4. 保留硬安全上限（G0 相邻：绝不无限）——"上策 100"即该档天花板，但允许 ask 扩展；这就是 ask 边界，非静默死。
  5. 身体"呼吸"=上下文预算信号经 WS8 introspect 渐进暴露给大脑（疲劳可见，非到点才死）。
- **验收门禁**：V2-11。

### WS10 受控联网工具 + 出网治理（新增 · 用户愿景"认知外延闭环"的缺失拼图）
- **目标**：给 LLM 一条**受控的、过 G0 出网治理**的联网能力（认知外延最强的一块），并与 WS7 验证/provenance 紧耦合——联网检索内容默认待验证。
- **源码锚点（插入点）**：内置工具在 `crates/codex-cli/src/run_local.rs:170-181` `build_dispatcher` 一行一个 `dispatcher.register(...)` 注册；WS10 在此加 `dispatcher.register(Arc::new(tools_builtin::WebTool::new()))`。新工具模块 `crates/tools-builtin/src/web.rs`（参照 bash.rs/read.rs 模式）。
- **两层能力**：
  1. `web_fetch(url)`（Phase 1，无后端依赖）：取 URL 正文（markdown/纯文本）。这是"认知外延"的基础原语——LLM 能从指定来源拉真实信息。
  2. `web_search(query)`（Phase 2，**后端已定 A：OpenAI `web_search` 工具**，用户已有 OpenAI key，经现有 OpenAI 兼容通道调用；C key-free DuckDuckGo 作无 key fallback）：取 query→排序 URL+摘要。后端经统一 `SearchBackend` trait 抽象，便于 B/C 后续热插拔。**不阻塞 WS10**。
- **G0 出网治理（唯一真安全点）**：当前沙箱（landlock FS + seccomp syscall）**不治理网络出网**——`SYS_SOCKET` 在白名单（sandbox/lib.rs:559），`bash` 可 `curl` 不受控出网。WS10 必须让**联网工具成为唯一 sanctioned 出网口**：
  1. 强制出网代理 + 默认拒绝白名单：设 `HEARTH_EGRESS_PROXY`（参照 `HEARTH_SECCOMP_MODE` env 模式 lib.rs:714），所有 agent 出网（含 bash 继承）经该代理；`HEARTH_EGRESS_ALLOWLIST` 域名默认拒绝未知域。web 工具是预期用户，bash 也受同一代理约束→堵住"偷偷 curl"后门。
  2. web 工具层 deny-by-default：域名不在白名单→拒绝并审计日志；所有出网请求记审计（URL/域/时间）。
  3. 内核级出网隔离（netns/iptables）作**后续安全跟进**（VM 无 CAP_SYS_ADMIN 起 netns，sandbox/lib.rs:948-952 已注），不阻塞 WS10，但须在任务书列为 🟡 遗留 + 跟进项。
- **与 WS7 接线**：web 返回内容自动带 `source`(URL) + `verifiable` 标签；load-bearing 断言来自 web 须先验证（跨源/官方 primary）；模型要陈述 web 衍生事实未验证→走 `unverified_claim` Gap（agent-types:118）。WS10 产出 provenance 元数据，WS7 消费。
- **验收门禁**：V2-12。

### WS11 会话韧性（盲区A · 升级维度6"续接"为"韧性"）
- **目标**：从"手动 resume"升级为"进程死了会话还能活"——每轮自动落盘 + 单 turn panic 不杀 thread + 重启自动恢复上次会话。
- **背景（手工日志铁证）**：`context.rs:158 end byte index 60 is not a char boundary` → 整个 REPL 进程死 → 前面 N 轮对话灰飞烟灭。任务书维度6只做手动 `resume`，救不了 panic。
- **顶层决策（做法）**：复用已有 `session_store.rs` 落盘能力（X1），每 turn 结束即持久化 transcript/状态；turn 执行包 try/catch + 进程级 supervisor，单 turn 失败隔离不影响 thread；重启自动探测上次未完成 session 并恢复。可与 WS1（续接）合并实现，但验收标准提高为"崩溃级韧性"。
- **验收门禁**：V2-13。
- **依赖**：WS1（续接基座）之后或同期；不阻塞 WS7–10。

### WS12 长内容生成（盲区B · 补齐维度4 之外的生成路径）
- **目标**：从零生成大段内容（5000+ 字文章/故事）稳定完成，harness 自动分块累积，而非让模型硬憋单轮超限。
- **背景（手工日志铁证）**：8000字长文 → `read body: error decoding response body`（单轮输出超 max_tokens）；甘特图 agent 自己"分4段追加"——模型知道分块，harness 不会帮它分。
- **顶层决策（做法）**：长输出流式累积 + 自动分块（如目标 5000 字 → harness 拆 5×1000 字追加写入同一文件 / 流式拼接到响应）；与维度4 diff 编辑是两条互补路径（编辑改已有文件、生成长文从零产出）。
- **验收门禁**：V2-14。
- **依赖**：独立；可与 WS2（diff 编辑）并行。

### WS13 确定性验证层（盲区C · 记分卡新增维度）
- **目标**：写盘后自动校验产物是否真的可用（存在性 → 字节数 → 可运行），失败回喂 loop 重来；agent 自报 Done 须先过 verify，杜绝"自报完成却不可用"。
- **背景（手工日志铁证）**：甘特图 agent 自报 Done 但产物可能不可用（verify 阶段才暴露）；俄罗斯方块 agent 自报"完成"却只是"搜了项目目录有没有游戏文件"。codex 用 read_lints + 编译回喂，Aider 每步 git diff + 测试，hearth 完全没有这一维。
- **顶层决策（做法）**：新增 verification pass——写文件/生成产物后，按"存在性→字节数→语法/编译/可运行"三级校验；失败则生成结构化错误事件回喂主循环重做；与 WS7（trust-but-verify）共用"验证即事实"语义，与 WS2（diff 编辑）共用写盘钩子。
- **验收门禁**：V2-15。
- **依赖**：WS2（写盘钩子）之后；与 WS7 验证语义对齐。

---

## 3. 总验收门禁（v0.2 过闸判据）

沿用此前验收报告格式（🔴阻塞 / 🟡遗留 / 实现率）。逐项须有真机/源码证据，守门员独立核验。

| 编号 | 交付 | 验收判据（做成什么样算好） | 真机证据 |
|---|---|---|---|
| V2-1 | 反应式终止 | 纯问答(20+20)≤3 步结束；无"tool not found:bash"误触发 | VM 真机 transcript |
| V2-2 | Thread 续接 | `/resume` 后 agent 能引用上轮文件/决策；跨进程重启不丢历史 | 重启前后 transcript 对比 |
| V2-3 | diff 编辑 | 生成 11834 字节文件完整落盘（复用 v0.1.1 贪吃蛇证据）；无整文件截断 | 文件字节数核对 |
| V2-4 | planner 瘦身 | Plan 阶段保留；空响应时静默降级单步，不卡 plan 阶段 | planner 空响应日志 + 任务仍完成 |
| V2-5 | 按需上下文(阶段一) | 默认不拉全仓；rg/ls/cat 按需；中型任务不 token 溢出 | token 计数日志 |
| V2-6 | 轻量 compaction | 长会话旧轮次摘要，不丢架构决策 | compaction 前后上下文对比 |
| V2-7 | 白盒兑现 | Plan→Observe→Reflect 相位真钩子 + EnvelopedEvent 信封字段 + Gap 上浮 | 源码接线 + 单测 |
| V2-8 | **综合达标** | 贪吃蛇 / 象棋 / 小型 web 应用 在 VM 真机**稳定完成**（N/M 判据：如贪吃蛇 5 次≥4 全绿），体验接近 top-3；无明显回归（B1–B7 不回潮） | VM 真机录屏/transcript + 产物核对 |
| V2-9 | 事实判断力 | 用户给一条似真但错误的"事实"（如"本项目用 X 框架"），agent 要么核验（读码）要么显式"未验证"标签回复，不当中立事实断言；final 答案中网络检索结论带 provenance 或标"未验证"；改码类 load-bearing 断言落地前至少 1 次验证(grep/read/run) | 单测 + 真机对话 transcript |
| V2-10 | 体感闭环 | LLM 可调 introspect 返回结构化身体状态(工作区/provider/上下文填充)可解析；provider 抖动/错误时发痛觉事件+诚实上报(无 Done 伪装,回归 R7)；场景：LLM 用 introspect 在撞硬上限前感知上下文疲劳并主动摘要/询问 | 单测 + 真机 |
| V2-11 | 预算价值化 | 中/大任务原会"Giving up after N steps"现跨 50%→打印偏离预警+询问(继续/重评估)而非终止；用户"继续"可跑完；planner 给子任务打 tier，主目标子任务分配更高；仍有硬上限(不无限循环)且为 ask 边界 | 单测(偏离预警+ask)+真机中任务 |
| V2-12 | 受控联网 | `web_fetch` 取回指定 URL 正文；域名不在 `HEARTH_EGRESS_ALLOWLIST`→拒绝+审计；`bash curl` 到非白名单域同样被代理拒绝（后门堵住）；web 结果带 source 标签进入 WS7 验证流；VM 真机冒烟：LLM 能"思考→fetch→再思考" | 单测(代理 deny)+真机 fetch |
| V2-13 | 会话韧性 | 每轮自动落盘；单 turn panic 不杀 thread（隔离）；`kill -9` 中期杀进程 → 重启自动恢复上次会话，丢失 ≤1 turn 且无数据损坏；手工日志 `context.rs:158` 类崩溃须被 supervisor 接住 | 单测(隔离)+真机 kill-9 恢复 |
| V2-14 | 长内容生成 | 5000–8000 字长文稳定完成、完整落盘无 `error decoding response body`；harness 自动分块累积（模型不必单轮憋出全部）；产物字节数 == 目标 | 真机长文生成 + 字节核对 |
| V2-15 | 确定性验证 | 写盘/生成后自动三级校验（存在性→字节数→可运行）；agent 自报 Done 须过 verify；故意造"产物不可用"场景 → 被校验捕获并回喂 loop 重做（非静默 Done） | 单测(校验 deny)+真机失败回喂 |

**过闸条件**：V2-1~V2-15 全过（🔴=0），实现率 ≥0.9。V2-8 是"80–90 分等效能力"的硬证据；V2-9~V2-12 为 v0.2 新增能力深化（判断力/内观/预算价值化/受控联网）；V2-13~V2-15 为盲区补强（会话韧性/长内容生成/确定性验证），均 🔴 阻塞达标，缺失即记 🔴。

---

## 4. 范围外（本版明确不做）

- 多智能体 / 子代理 / spawn_agent（维度 7，consciously 延后，不影响核心达标）。
- repo-map 阶段二（tree-sitter + 引用排序）——加分项，可放 v0.3。
- 抽取上游 `codex-thread-store` / Lark `apply_patch`（过重或紧耦合，改轻量重实现）。
- MCP 市场 / app server / 跨平台打包（Windows/macOS）——单人 Linux VM 场景，平台化镀金。
- Thread fork / rollback / archive（续接够用即可）。
- 换模型（deepseek v4 flash 不动）。

---

## 5. 顺序与依赖

- **并行可行**：WS1（循环/续接）与 WS2（diff 编辑）相互独立，可并行开工。
- **WS3 依赖 WS1**：planner 瘦身建立在反应式终止逻辑之上（不再靠"写文件"驱动循环）。
- **WS4 先于提步数**：compaction 必须在提高默认步数预算之前落地（否则 15–20 步后 token 腐烂）。
- **WS5 / WS6 独立**：可在 WS1–WS4 收尾阶段并行。
- **建议节奏**：WS1+WS2 先 → WS3+WS4 次 → WS5+WS6 并行收尾 → V2-8 综合验收。
- **WS7/WS8/WS9/WS10（v0.2 新增扩展）**：在 WS1–WS6 + X1–X3 已落地基础上新增。① WS8（introspect 暴露 + BodyState 总线）先开工——它是判断力与预算可见性的地基（大脑先能"感觉"身体，才能判断好、才能看见疲劳）；② WS9 次之（预算渐进信号需 WS8 暴露，planner 打 tier 可独立）；③ WS7 复用 WS6 白盒钩子 + Gap，可与二者并行；④ WS10（受控联网）的 `web_fetch` 出网代理先建、可与 WS7 并行落地（验证耦合），`web_search` 后端已定 A（OpenAI，见 §8.5，C 作 fallback）。四者均零/低内核改动，复用 WP-0/InteractionRequested/Gap/Constitution；WS10 额外涉及 G0 出网治理（安全边界变更）。

---

## 6. 给执行窗口的说明

1. **工程量不设上限**，以 §1 记分卡 + §3 门禁达标为准。
2. **技术自查 4 项**（开工前，可用上游 `codex-rs` 源码 + 网络）：① diff applier 选型（unified-diff vs SEARCH/REPLACE，倾向自实现 SEARCH/REPLACE）② 最小 Thread 续接（复用 transcript 可行性 / 回灌边界 / SQLite vs JSONL）③ planner 瘦身可行性（不破四阶段的最小契约）④ deepseek prompt 格式适配（小样本实测，不换模型也能提表现）。
3. **方向已顶层裁定，勿擅自改**：
   - 保留白盒差异化（四阶段 / Gap / EnvelopedEvent）——这是 hearth 唯一能超额的维度，对齐时**不能删**。
   - planner 是四阶段之一，**瘦身不删**。
   - 安全默认 **fail-closed 不动**（G0 Landlock + cgroup）；B6 是 bug 修掉，不是放松沙箱。
4. **门禁纪律**：每 WS 交付带 fmt/clippy/test 全绿 + 真机/源码证据；V2-8 综合验收由守门员独立核验（不信报告信源码）。
5. **与 v0.1.3 关系**：v0.1.3（R1–R5 快速修复：B1 终止闸/B4 预算/B5 路径/B6 沙箱/B7 WARN）是独立快速修复，可先于或与 WS1 协同；v0.2 在其基础上补齐能力级对标。
6. **WS7/WS8/WS9/WS10 为 v0.2 新增**（判断力/内观/预算价值化/受控联网）——用户明确"进任务书"。预算从硬上限改为价值化分配+偏离预警+询问，是 control-flow 层改动，须复用 InteractionRequested(kind="budget_reassess") 且**内核不加 kind 分支**（守 WP-0：内核只认 blocking/timeout/id）。内观/判断力/联网均长在现有架构（observer/ crate、EnvelopedEvent、Gap、Constitution、build_dispatcher 工具注册）；**WS10 是 G0 安全边界变更（出网治理），须单独过出网白名单评审**，不另起 crate。

---

## 7. 签收与回执

| 项 | 内容 |
|---|---|
| 签发人 | 顶层守门员 |
| 签发日期 | 2026-08-23 |
| 接收窗口 | 执行窗口 |
| 接收确认 | ________________（执行窗口签收后回填：已读 / 有异议） |
| 开工前置 | 完成 §6 技术自查 4 项 + 读 v0.1.3 任务书确认前置修复已落地 |
| 过闸提交 | 每个 WS 交付带 fmt/clippy/test 全绿 + 真机/源码证据；V2-8 由守门员独立核验 |

> 本任务书为顶层正式签发件。执行窗口须严格按 §2 顶层决策与 §3 门禁交付，方向性决策（白盒差异化 / planner 瘦身不删 / G0 fail-closed 不动）不得擅自变更；如实现中发现顶层决策不可行，须回顶层出修订，不得私自改方向。

---

## 8. WS7–9 开工前技术自查与执行顺序（A 方案已确认）

> 本会话用户拍板"A：本版就做 WS7–9"。以下为执行窗口开工前的预检（前置核验 + scope 发现 + 各 WS 必答清单 + 执行顺序），守门员已按"不信报告信源码"在活源码复核。

### 8.0 前置核验（v0.1.6 已落地，活源码复核）
| 项 | 锚点 | 状态 |
|---|---|---|
| R8 字符切片 | `crates/agent-core/src/context.rs:160` `goal.chars().take(60)` | ✓ |
| R9-B3 planner max_tokens | `crates/planner/src/lib.rs:337` `=8192` | ✓ |
| R10 Gap options/style | `crates/agent-types/src/lib.rs:118`（serde default） | ✓ |
| R9 重试窗口 | `crates/agent-core/src/loop.rs:1449` `TOTAL_RETRY_CAP=120s` | ✓ |
| X1 会话韧性 | `crates/codex-cli/src/session_store.rs` 存在 | ✓ |
| X2-3 分块 | `crates/tools-builtin/src/edit.rs:8` `WRITE_CHUNK=4000` | ✓ |

### 8.1 必须澄清的 scope 发现（开工前必读）
- **WS7 联网部分目前不可执行**：LLM 当前**无联网工具**——`tools-builtin` 仅 `bash/edit/glob/grep/patch/read` 七个模块；`crates/service/src/routes.rs:837` 的 `web_search` 是服务侧 HTTP 路由（对外暴露工具查询），**不是 agent 循环里 LLM 可调用的工具**；全仓无任何 `web/fetch/search/lookup` 工具 crate（仅 `service/src/webhook.rs`）。即 hearth 现在根本没有"搜什么信什么"的行为面——该风险是前瞻性的。
  - **本版 WS7 落地 scope**：用户陈述的"事实" + 工具执行结果（bash/grep/read 真实返回）+ 文件读取 的 Claim 溯源 / load-bearing 强制验证 / `unverified_claim` 上浮 / provenance 回复。
  - **联网政策（trust-but-verify for web）写成 spec 层**：待独立前置"加一个出网受限的 lookup/web 工具（须过 G0 出网白名单 + 沙箱）"到位后即激活；该工具本身不属 WS7，建议 v0.2 末或 v0.3 单列（出网=安全边界变更，须评审）。
- **WS9 扩字段位置明确**：`Budget` 结构在 `crates/agent-types/src/lib.rs:278`（字段 `max_steps`/`max_tokens`/`max_time_secs`，default `max_steps=50`）。WS9 新增 `tier`/`allocated_units`/`deviation_warn_at` 须带 `serde(default)` 兼容旧序列化（参照 Gap options）。`max_steps` 即 ask 边界（REPL 现用 `REPL_BUDGET=40`，repl.rs:10）。

### 8.2 各 WS 技术自查清单（执行窗口开工前必答）
**WS8（先开工，内观是地基）**
1. 读 `crates/agent-runtime/src/session.rs:505` `get_status` 返回的 `SessionStatus` 字段——确认是否含 工作区状态 / provider 健康 / 上下文填充度；缺则补字段。
2. introspect 工具注册点（tools-builtin 现有 7 模块模式参照）——包 `get_status`，进 LLM 工具 schema；教 prompt 用。
3. `EnvelopedEvent`（api/src/lib.rs:103）是否已能在工具执行路径发出 status——确认"下行结构化回执"复用现有事件而非新通道。
4. 痛觉事件：provider 抖动/错误路径（loop.rs:1477 `deadline_hit`、R7 已修 Done 伪装）是否已诚实上报；BodyState 总线扩为 observer/ 持续监视（Phase 2）。

**WS9**
1. `budget_exhausted()`（loop.rs:2454）改发 `InteractionRequested{kind:"budget_reassess",options:["继续","重新评估"]}` 后，确认 loop 在 blocking 交互回复后**继续**而非终止（核查 WS1 的 `continue_turn` 路径）——这是唯一真风险点（control-flow）。
2. planner 子任务 tier 打标：复用 R10 的 `extract_options` 模式加 `extract_tier`；主目标子任务拿 Premium、辅助拿 Standard/Economy。
3. tier→allocated_units 映射（上策100/中策50/下策20）如何流入 Budget；`REPL_BUDGET=40` 变默认 Standard 基线。

**WS7**
1. Claim 溯源侧信道落点：用户消息 / 工具结果 / 文件读取 在 context 的表示（loop.rs 消息处理），加 source+verifiable 标签，不新增 crate。
2. `unverified_claim` Gap：复用 `Gap`（agent-types:118），planner gap 提取路径（R10 `extract_options` 同处）支持 `from="unverified_claim"`；模型要陈述未核实事实时先验证或上浮"未验证"。
3. Constitution（G3, constitution.md, ≤10 条）加 trust-but-verify；Experience（G2）加 `category="verification_failure"` 判例。

### 8.3 建议执行顺序（依赖关系）
1. **WS8 Phase 1**：暴露 introspect 工具（包 get_status）——最低风险、立即可用、解锁 WS9 预算信号暴露。
2. **WS9**：Budget 扩字段 + `budget_exhausted` 改偏离预警+ask + planner tier；V2-11 验收。
3. **WS7**：Claim 溯源 + unverified_claim + Constitution + experience（联网部分留 spec）；V2-9 验收。
4. **WS8 Phase 2**（长期丝滑轴）：observer/ 扩为持续体感监视器发 BodyState 总线 + 痛觉主动事件 + 下行结构化回执；V2-10 完整验收。

### 8.5 WS10 技术自查 + 出网后端决策（已决：A）
**WS10 必答清单**
1. 注册点：`crates/codex-cli/src/run_local.rs:170-181` `build_dispatcher` 加 `tools_builtin::WebTool::new()`；新模块 `crates/tools-builtin/src/web.rs` 参照 bash.rs/read.rs（Tool trait + schema + 执行）。
2. 出网代理机制：实现 `HEARTH_EGRESS_PROXY`（参照 `HEARTH_SECCOMP_MODE` env 模式 sandbox/lib.rs:714）+ `HEARTH_EGRESS_ALLOWLIST` 默认拒绝；确认代理进程随 agent 启动、`bash` 子进程继承同一代理 env→非白名单域统一拒绝（堵 bash curl 后门）。需真机验证：bash `curl` 非白名单域 → 连接失败/拒。
3. web 工具层 deny-by-default + 审计日志（URL/域/时间）；返回内容注入 `source`(URL)+`verifiable` 标签，进入 WS7 验证流。
4. 内核级出网隔离（netns/iptables）不可行（VM 无 CAP_SYS_ADMIN，sandbox/lib.rs:948-952）→ 列为 🟡 遗留 + 跟进项，不阻塞 WS10。

**✅ 出网后端决策（已定：A）**
`web_search(query)` 搜索后端用户已拍板：
- **✅ A（已选）OpenAI `web_search` 工具**：用户已有 OpenAI key（`sk-proj-…`，memory 记录），经现有 OpenAI 兼容通道（`HEARTH_OPENAI_*`/base_url 已贯通 v0.1.6）调用，无需新增密钥/网关；落地快、结果质量高、自带 source 引用利于 WS7 provenance 接线。
- B Brave / SerpAPI / Tavily：需新申请 key（用户暂无），留作后续可插拔。
- C key-free DuckDuckGo HTML 抓取+解析：零成本临时方案，质量/稳定性弱，**作无 key 环境的兜底 fallback**（代码上保留接口抽象，后端可热切换）。
→ 执行口径：**v0.2 先落 `web_fetch`（Phase 1，无后端依赖，即闭合"思考→fetch→再思考"）**；`web_search` 选 **A（OpenAI）作 Phase 2**，C 作无 key 环境 fallback。搜索后端用统一 `SearchBackend` trait 抽象，便于 B/C 后续插入。

### 8.4 门禁纪律（沿用）
每 WS 交付带 fmt/clippy/test 全绿 + 真机/源码证据；V2-9~V2-12 由守门员独立核验（不信报告信源码）。WS10 为 G0 安全边界变更（出网治理），须单独过出网白名单评审，不混进 WS7。

---

## 9. 补充令 v0.2.1 · 盲区并入主线（下发执行窗口）

> **来源**：`docs/hearth-v02-blindspot-scan.md` 三个 🔴 维度级盲区（会话韧性 / 长内容生成 / 确定性验证）。
> **性质**：**补充令，非新版本**。在已签发的 v0.2 任务书（commit 5cbcdcc）基础上**追加** WS11/12/13 与 V2-13/14/15，不动 WS1–WS10 既有范围。
> **为何必须进主线**：补充文档会被"打地鼠"忽略——`确定性验证层` 上一轮在 `hearth-harness-review-supplement.md` 提过，v0.2 签发版仍没纳入。**游离在补充文档里的东西，等于没提。**

### 9.1 下发内容（已并入 §1 记分卡 15/16/17、§2 WS11/12/13、§3 V2-13/14/15）
- **WS11 会话韧性**（升级维度6"续接"）：每轮自动落盘 + 单 turn panic 隔离 + 重启自动恢复。← 已有 `session_store.rs` 落盘能力（X1）复用。
- **WS12 长内容生成**（补齐维度4）：流式累积 + 自动分块，5000–8000 字长文稳定完成。
- **WS13 确定性验证**（记分卡新增维度）：写盘后三级校验（存在性→字节数→可运行），失败回喂 loop。

### 9.2 回归测试硬性要求（每条须有"故意造失败"的反向用例）
| 门禁 | 反向回归用例（必须能失败） | 预期 |
|---|---|---|
| V2-13 | 跑任务中途 `kill -9` 主进程，重启 → 应自动恢复上次会话，丢失 ≤1 turn | 不丢历史、不损坏落盘 |
| V2-14 | 让模型生成 8000 字长文（复现 `error decoding response body`） | 完整落盘、无解码错误 |
| V2-15 | 故意造"产物不存在/不可用"场景（仿俄罗斯方块 agent 搜目录即报 Done） | 被校验捕获、回喂重做、不静默 Done |

> 守门员铁律："断言不能失败 = 断言不存在"。上述反向用例**必须先证明能失败**（旧写法证伪），再证明修复后通过。

### 9.3 与在途 WS 的关系（不阻塞、不返工）
- WS11 与 WS1（续接）合并实现、验收标准提高；WS12 与 WS2（diff 编辑）并行；WS13 在 WS2 写盘钩子之后、与 WS7 验证语义对齐。
- WS7–10 按原顺序继续，**不受本补充令打断**；本令为追加项，执行窗口排期时并入即可。
- 全部门禁（V2-1~V2-15）重新跑一遍后再过闸；实现率 ≥0.9 且 🔴=0。

### 附录：交付物清单
- 规划依据：`docs/hearth-harness-benchmark-top3.md`
- 快速修复（前置）：`docs/hearth-harness-hardening-v013-taskbook.md`
- 盲区扫描（补充令来源）：`docs/hearth-v02-blindspot-scan.md`
- 天赋基因规范（初稿，17 项全 G1 常驻草稿，已被终版取代）：`docs/hearth-meta-capability-genes.md`
- **天赋基因规范 · 最终执行版（v2，含情境门控）← 执行窗口唯一权威件**：`docs/hearth-meta-capability-genes-final.md`
- 天赋基因规范 · 不同见解与补充（代价/调度批判，终版来源之一）：`docs/hearth-talent-different-view.md`
- 本任务书：`docs/hearth-v02-taskbook.md`
