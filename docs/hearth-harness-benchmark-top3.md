# Hearth Harness 功能对标报告（Top-3 生产级编码 Agent）

> 文档性质：顶层（守门员）功能调研与对标，非源码逐行对照。
> 日期：2026-08-22（2026-08-23 外部顾问评审后修订；2026-08-23 补能力记分卡）
> 目的：回答用户的判断——"如果我们参考行业生产级产品都做不出可用的 hearth，问题就在我们自己身上。"
> 结论先行：**问题确实主要在我们自己身上，而且是"分叉后回归"**，不是模型天花板。下面给出证据与拿来清单。

---

## 修订说明（2026-08-23 外部顾问评审后）

外部顾问对初稿给予高评价，并指出 3 处需修正 + 3 处缺口。本报告已据此修订：

- **修正 1（planner 不删）**：原"删重型 upfront planner"改为"planner 瘦身 + 优雅降级，保留 Plan 阶段（四阶段白盒差异化核心）"。见 §3 P1-4、§6。
- **修正 2（on-demand 两阶段）**：原"按需 + repo-map-lite"拆为阶段一（rg/ls/cat 替换重型 planner，不做 repo-map）+ 阶段二（tree-sitter + 引用排序）。见 §3 P1-5、§6。
- **修正 3（compaction 先于提步数）**：原"步数提到 ~40"改为"compaction 落地前步数保持克制，compaction 后再提"，顺序不能反。见 §3 P1-6、§6。
- **补缺口 #1（P0 接入方案）**：新增 §7，明确"参考上游设计、hearth 侧轻量重实现"，不整搬 `codex-thread-store` / Lark `apply_patch`。
- **补缺口 #2（验收门禁）**：新增 §8 v0.2 验收门禁表草案。
- **补缺口 #3（prompt 格式）**：新增 §9，提醒"不换模型"仍需验证 deepseek prompt 格式适配。
- **新增 §10**：v0.2 执行前技术自查清单（4 项）。
- **用户指令（01:15）**：工程量不设上限，以达成**功能对标 80–90 分**为准；规划钉死后交执行窗口实现。新增 §11 能力记分卡量化该目标；旧设计文档"~3-5k 行"天花板作废，范围纪律改按"单人稳定生产"守，不按行数卡。

> 顾问核心提醒：本报告是灯塔不是施工图。但用户已裁定——目标（80-90 分功能对标）与做法（§7 接入方案）由顶层钉死，执行窗口照此实现并用上游 codex-rs 源码 + 网络自查技术细节，无需另设独立调研阶段。

---

## 0. 为什么选这三个（选择依据）

打分维度不是"谁最火"，而是"对 hearth 这台 Rust 终端 CLI fork 最有用"：

| 候选 | 选型理由 | 与 hearth 的关系 |
|---|---|---|
| **Codex CLI**（OpenAI, Rust, Apache-2.0） | hearth 的直接上游分叉源；终端原生、OS 级沙箱、apply_patch 编辑原语 | **基线 / 镜子**——很多 hearth 的 bug 是它上游本来就有、我们分叉后丢掉的 |
| **Claude Code**（Anthropic） | agentic 循环的事实标杆；Terminal-Bench 2.1 第二（78.9%，Opus 4.8）；Hooks + 子代理 + 压缩最成熟 | **能力天花板参照**——它证明"黑盒反应式循环 + 确定性护栏"能跑得极稳 |
| **Aider**（Paul Gauthier, Apache-2.0, 2023） | OSS 里 diff 编辑格式 + repo-map 的原型鼻祖；最能直接借鉴的轻量实现 | **可抄的作业**——编辑格式与上下文裁剪思路最贴近 hearth 单人场景 |

荣誉提及（未入选但值得看）：**Cline**（Plan/Act 拆分 + MCP 市场）、**opencode**（180k★ MIT，最火 OSS，但偏 VS Code 伴侣）、**Cursor**（编辑器内，非终端）。它们要么与 hearth 终端定位不符，要么能力被上面三家覆盖。

---

## 1. 三个 Harness 的功能解剖

### 1.1 Codex CLI —— hearth 的上游基线（也是镜子）

- **核心循环**：ReAct 式工具循环，完全跑在沙箱里。三方：用户提示 → LLM Agent（2026 默认 GPT-5 Codex）→ 本地执行器（掌管文件系统 + 沙箱 shell）。模型只拿到 `shell` / `apply_patch` / 可选 MCP 三个工具。
- **终止逻辑**：模型发出 `done` 事件即结束回合。**没有任何"必须写文件才算完成"的闸**。纯问答一轮就停。
- **规划**：**反应式**，不预先生成重型 TaskGraph。上下文按需拉取（`rg`/`ls`/`cat`），输入窗口小 → 对话前缀可走 Prompt Caching。
- **编辑机制**：`apply_patch`——自定义 unified-diff 信封，由 **Lark 文法解析器**校验，**事务性应用**（任一 hunk 失败则整片拒绝并回显错误）。两种模式：freeform（文本 diff）/ JSON（结构化）。好处：① 部分编辑远比整文件重发省 token；② diff 强制模型锚定上下文行，把"改错函数"挡在应用期而非测试期。
- **沙箱/权限**：Linux = Landlock + seccomp（与 hearth 同源思路）；默认拒绝网络 + cwd 外写入。沙箱模式 `read-only`/`workspace-write`/`danger-full-access` × 审批 `suggest`/`auto-edit`/`full-auto`。
- **会话持久化**：**Thread = SQLite 持久会话**，进程重启可 `resume`/`fork`/`archive`/`rollback`；Turn / Item 为内部粒度。上游实现为 `codex-thread-store` + `codex-state`(SQLite) + `codex-rollout`(JSONL) 多 crate 协作，数十文件。
- **记忆**：`AGENTS.md` 启动时分层加载项目约定；两阶段持久记忆管线。
- **其他**：`/plan`（执行前出计划）、`/resume`（续接）、`tool_search`（BM25 在数百工具中检索）、MCP 一等公民、Hooks crate。

### 1.2 Claude Code —— agentic 循环标杆

- **核心循环**：**反应式**，没有工具调用即停 & 等。关键架构事实：循环"闭合在上下文组装上，而非模型调用上"——每次迭代都重建上下文，所以压缩与选择性读文件杠杆极大。
- **终止逻辑**：模型返回"无工具调用"的响应即判完成；护栏（迭代数上限、token 预算、Hooks）提供额外停止点。
- **编辑机制**：**Edit 工具 = 简单字符串替换**（不是 diff！）。整文件重发较少用。
- **上下文管理**：Prompt Caching + **自动压缩**（约 92% 利用率触发，把旧轮次总结掉）；压缩有 pre/post hook 可保留关键信息；`CLAUDE.md` 进每个系统提示；分层记忆 = CLAUDE.md + `~/.claude/memory` + 会话历史 `resume`/`fork`。
- **子代理**：独立上下文窗口的嵌套循环，返回摘要；单 REPL 内串行（~10 并发上限）；fork 继承 prompt-cache 前缀；真正并行用 tmux Teams。研究系统多代理比单代理高 90.2%，但 token 约 15×。
- **Hooks（架构签名）**：在固定循环点（pre-tool / post-edit / session-end）触发确定性脚本——"编辑后跑格式化、拦截生产命令、审计每条工具调用"。**确定性护栏环住概率性核心**是它的标志性设计。
- **Skills**：按需动态加载的指令包（描述常驻、正文用时才加载）。
- **终端 UI**：React via Ink（Yoga flexbox + ANSI），streaming-first，可折叠 thinking 块，iTerm 内联图片。
- **完成判定**：`/goal`——把"完成条件 + 证据"交给一个无工具的 evaluator（Stop hook），证据必须进 transcript，否则不算完成。

### 1.3 Aider —— OSS 轻量原型（最该抄作业）

- **核心循环**：**chat 驱动 edit-apply-reflect，不是 ReAct**。每条用户消息 = 一次模型回复；**模型不调任何工具**，而是把 SEARCH/REPLACE diff 当纯文本吐出，再由 harness 应用/提交/lint/测试。`max_reflections=3` 封顶。
- **终止逻辑**：**人是外层循环**。没有自主任务预算，没有"task complete"信号；唯一自动迭代是反思循环（且每轮先问用户）。
- **编辑格式**：`whole` / `diff`(SEARCH/REPLACE) / `udiff` / `diff-fenced` / `editor-*`(architect)。**按模型基准自动选最优**；SEARCH 必须匹配当前文件内容，否则**响亮失败**而非静默改错。Aider 证明 SEARCH/REPLACE 无需 Lark 文法也能稳定应用。
- **repo-map**：tree-sitter 抽 AST → **PageRank 式引用图排序** → 默认 1000-token 预算（`--map-tokens`）塞进提示。被 `/add` 的文件才整文件发，其余只走 map。与模型无关（项目属性）。
- **Git 集成**：每次成功编辑自动提交（语义化 message 由 `--weak-model` 从 diff 生成）；`/undo` `/diff` `/commit`。
- **Architect 模式**：两个模型（architect 规划、editor 出编辑），在 polyglot 基准上常比单模型高 10 个点。

---

## 2. 功能对照表（10 维度 × 3 家 × hearth 现状）

> 图例：✅ 有/好　⚠️ 部分/有 bug　❌ 没有/坏　🔁 = hearth 上游本来有、分叉后回归

| # | 维度 | Codex CLI | Claude Code | Aider | **Hearth 现状（2026-08-22 实测）** |
|---|---|---|---|---|---|
| 1 | 核心循环 / 终止 | 反应式，`done` 结束 | 反应式，无工具调用即停 | 人外层循环 | ❌ **B1：loop.rs 强迫"写文件=完成"，纯问答被迫 replan**（loop.rs:1217/1423/1458/1472） |
| 2 | 规划策略 | 反应式 + 可选 `/plan` | 反应式 + 可选 Plan Mode | 无（人规划） | ⚠️ **B3：重型 upfront planner，约 50% 空响应回退单节点**（planner:312） |
| 3 | 上下文管理 | 按需 rg/ls/cat + 前缀缓存 | 压缩 + CLAUDE.md + 分层记忆 | repo-map + 1000-token | ❌ 重型 upfront 拉取（反模式），无压缩、无 repo-map |
| 4 | 编辑机制 | **apply_patch(diff, Lark 校验, 事务)** | Edit（字符串替换） | SEARCH/REPLACE diff | ❌ **write_file 整文件重发 → 截断风险**（🔁 上游 apply_patch 丢了） |
| 5 | 沙箱 / 权限 | Landlock+seccomp, 三层模式 | 较轻（靠允许列表） | 几乎无（靠 git+人） | ⚠️ 沙箱本身好（Landlock+cgroup fail-closed）但 **B6：误杀 verify 命令（node --check/tail 退 -1）** |
| 6 | 会话持久化 / 续接 | **Thread SQLite, resume/fork** | 会话 resume/fork | git 提交 + transcript | ❌ **B2：repl.rs:96 在 loop{} 内 create_session → 零续接**（🔁 上游 Thread 丢了） |
| 7 | 多智能体 / 子代理 | spawn_agent | 子代理 + Teams | 无 | ❌ 无（单人场景可后置） |
| 8 | 确定性护栏 Hooks | 有（hooks crate） | **极强（架构签名）** | 无（lint/test 管道） | ❌ 无 |
| 9 | 项目记忆 | AGENTS.md | CLAUDE.md + auto-memory | 无 | ❌ 无 |
| 10 | 可观测性 / 审计 | event protocol | transcript | transcript | ⚠️ **EnvelopedEvent 设想存在但源码未落地**（AgentEvent 仅 8 变体、无信封字段）→ 白盒差异化未兑现 |

---

## 3. Hearth 应该"拿过来"什么（按优先级，标注回归项）

**P0 — 分叉回归修复（上游本来就有，但采取"轻量重实现、参考上游设计"，不整搬上游重型 crate）：**

1. **🔁 反应式终止（B1）**：删掉 loop.rs 的"未写文件→强制 replan"闸（loop.rs:1217/1423/1458/1472），改为"模型发出 done / 无工具调用即停"。纯问答 ≤3 步必须结束。纯 loop 逻辑改动，无上游依赖，风险最低。
2. **🔁 会话续接（B2）**：**不做**重型 `codex-thread-store`（上游含 `codex-state` SQLite + `codex-rollout` JSONL，数十文件，过重）。最小路径 = 复用 hearth 已有的 `transcript.rs`（已逐步骤记录）→ 加 `session_id` + "续接时把历史 transcript 回灌进上下文"。SQLite 或纯 JSONL 均可，量级远小于上游。v0.2 第一优先。
3. **🔁 diff 编辑（B4 编辑侧）**：**不抽取**上游 Lark 文法 `apply_patch`（它是 codex 二进制内嵌资产、紧耦合 codex 协议，抽取成本高）。改在 hearth 自己的 `tools` crate 里实现轻量 unified-diff / SEARCH-REPLACE applier（Aider 证明 SEARCH/REPLACE 无需 Lark 也能稳）。模型输出 diff → harness 事务应用 → 消灭整文件截断。

**P1 — 借鉴三家、按单人场景落地：**

4. **planner 瘦身 + 优雅降级（B3 根因，保留 Plan 阶段）**：**不删除** planner——"Plan" 是 hearth 四阶段白盒状态机的显式阶段（差异化核心），删了就剩三段。改为：planner 从"必须输出完整 TaskGraph"降级为"可选输出规划草案，失败时静默降级到单步模式"，轻量、可跳过、不阻塞主循环。空响应回退不计入步数预算。
5. **按需上下文（两阶段，勿低估难度）**：
   - **阶段一（先上）**：默认不拉全仓，按需 `rg`/`ls`/`cat`；先替换掉重型 upfront planner，不做 repo-map。解决"规划太重+空响应"根因。
   - **阶段二（后上）**：再叠 tree-sitter AST + 引用图排序的 mini repo-map（1000-token 级，参考 Aider）。这是独立工程，不要和阶段一混为一谈。
6. **预算校准（compaction 先于提步数）**：**不能无脑提到 40**——hearth 当前无压缩，40 步大概率在 15–20 步就 token 溢出、上下文腐烂失败。正确顺序：先做轻量 compaction（把旧轮次摘要掉），再把默认步数提到 30–40；在 compaction 落地前，步数上限保持克制（~20 + 早轮裁剪），无进展立即 fail-fast。
7. **路径遍历放宽（B5）**：允许项目内/家目录绝对路径（grep.rs:84-89 当前一律拒绝）。
8. **轻量 Hooks（确定性护栏）**：至少两条——"编辑后跑 fmt/clippy"、"拦截生产/危险命令"。这是 Claude Code 的招牌，且对单人安全极划算。
9. **项目记忆文件（Hearth.md / AGENTS.md 等价）**：启动时读一层项目约定 + 失败教训，进系统提示。

**P2 — 后置（v0.x 更晚）：**

10. 子代理 / spawn_agent（参考 Codex/Claude，用于大仓库并行探索）。
11. 压缩（compaction）在会话变长时兜底——注意它也是 P1-6 提步数的前置条件。

---

## 4. Hearth 应该"比他们做得更好"什么（白盒差异化，保留 + 强化）

这是 hearth 的护城河，**三家都没有**，不能为了对齐而删掉：

- **四阶段显式状态机（Plan→Act→Observe→Reflect）**：Codex/Claude 是黑盒反应式循环，模型"想停就停"，过程不可审计。hearth 把 Observe/Reflect 提升为一等阶段，每一轮都留下"我观察到了什么、我反思了什么"的痕迹——**这才是可审计 AI 的核心**，要保留并让它真正落地（不是空壳相位钩子）。这也正是 §3 P1-4 坚持"planner 瘦身不删"的根本原因：Plan 是四阶段之一，删 planner 等于自废差异化。
- **Gap 三元组（显式"我搞不定/这是不确定的"）**：三家都不会结构性地承认"我不知道 / 这是 gap"。hearth 把 undecidability / uncertainty 当一等公民上浮给用户，是可信委托的关键。强化它。
- **EnvelopedEvent（带 ts/seq/span_id/parent_id/schema_version 的结构化事件）**：当前源码未兑现（AgentEvent 仅 8 变体、无信封字段）——**这是回归点也是差异化点**：补齐信封字段，让 span 树真正可表达，给 Observer 第三权提供零执行权的审计面。Codex 有 event protocol 但无显式 span 树 schema。
- **Fail-closed 安全（G0）**：hearth 的 Landlock + cgroup fail-closed 已**对齐甚至超过** Codex 默认。把它当卖点，不要把沙箱为了"方便"放松（B6 是 bug，不是设计，修掉即可）。

> 对标姿态总结：**黑盒循环该抄的就抄（终止/续接/diff/上下文），白盒差异化该守的死守（四阶段/Gap/EnvelopedEvent）。** 不能在"对齐 Codex 正常功能"时把自家真正先进的东西也删了。

---

## 5. 三家各自做得不好的地方（我们的优化空间）

| Harness | 弱点 | Hearth 的优化机会 |
|---|---|---|
| Codex CLI | 激进沙箱会误杀合法 verify 命令（hearth B6 同源）；默认绑专有模型；diff 偶尔应用失败 | 沙箱做成"白名单放行 verify 类命令"；BYOK 友好（hearth 已支持多 provider） |
| Claude Code | Edit 仅字符串替换（不如 apply_patch 结构化）；MCP 工具定义吃 12.5% 上下文才发首条消息；压缩会丢架构决策；多代理 15× token 贵 | hearth 用 diff 编辑 + 精简工具面；压缩前用 Compact Instructions 保架构决策 |
| Aider | 无自主循环（人外层）；无真沙箱（靠 git+人）；diff 对空白脆弱；超大 monorepo 吃力 | hearth 有自主循环 + 真沙箱 + 失败回滚，天然补 Aider 短板 |

---

## 6. 落到 v0.2 的路线（本基准如何指导 B2 / B3）

- **v0.1.3（快速修复，R1–R5）已立项**：B1 终止闸、B4 预算、B5 路径、B6 沙箱误杀、B7 WARN 污染 + 门禁。这些不依赖本基准，可立即下发给执行窗口。
- **v0.2 第一优先（B2 续接）**：最小路径 = 复用已有 `transcript.rs` + `session_id` + 续接回灌，**不整搬** `codex-thread-store`。本基准证明 Codex/Claude/Aider 三家无一靠"超重型会话存储"，都是轻量持久 + 续接。
- **v0.2 第二优先（B3 循环对齐）**：① **planner 瘦身非删除**——保留 Plan 阶段，改为可选草案 + 静默降级单步；② 按需上下文**两阶段**（先 rg/ls/cat 替换重型 planner，后上 repo-map）；③ 编辑切轻量 diff applier（自实现，非抽上游）。**不换模型**——本基准用事实推翻了"模型天花板"假设：同样的 deepseek v4 flash 在 Codex 上游表现远好于 hearth，差的是 harness 设计。
- **v0.2 第三优先（白盒兑现）**：把 Plan→Observe→Reflect 相位钩子接真、补齐 EnvelopedEvent 信封、把 Gap 上浮做成一等输出。这是 hearth 相对三家的真正卖点。
- **顺序约束**：compaction（轻量压缩）必须先于"提高步数预算"——否则高步数只会在后半段因上下文腐烂失败（呼应顾问提醒）。

---

## 7. P0 三项轻量接入技术方案（回应顾问：方案具体化，且不整搬上游）

顾问正确指出原报告 P0"接入方案不够具体"。补充如下，结论是目标与做法由顶层钉死，执行窗口照此实现（无需另设独立调研阶段）：

| 项 | 上游真实形态 | hearth 轻量路径（不整搬） | 风险/工作量 |
|---|---|---|---|
| 反应式终止(B1) | Codex/Claude：模型发 done / 无工具调用即停 | 删 loop.rs:1217/1423/1458/1472 的"未写→强制 replan"闸；加 done/no-tool-call 终止。纯逻辑 | 低 |
| Thread 续接(B2) | 上游 `codex-thread-store`+`codex-state`(SQLite)+`codex-rollout`(JSONL)，数十文件 | 复用 hearth `transcript.rs`（已逐步骤记录）+ `session_id` + 续接回灌历史。SQLite/JSONL 均可 | 中（需定义回灌边界） |
| diff 编辑(B4) | 上游 `apply_patch` = 内嵌 Lark 文法解析器，紧耦合 codex 协议 | hearth `tools` crate 自实现 unified-diff / SEARCH-REPLACE applier（Aider 证明无需 Lark） | 中（diff 匹配/事务应用） |

> 关键修正：原报告"re-import 上游三件套"的措辞过重。上游 ThreadStore / apply_patch 都过重或紧耦合，**正确姿态是"参考上游设计、在 hearth 侧轻量重实现"**。这既避免 10–20k 行重型依赖，也守住单人场景的简洁；但**工程量本身不设上限**——若实现 80-90 分目标需要更多代码（如真正的 compaction、真正的 diff applier），执行窗口应实现，而非为省行数砍能力。

---

## 8. v0.2 验收门禁表（草案，回应顾问缺口 #2）

任务书下发前需补此表（沿用此前验收报告格式：🔴阻塞/🟡遗留/实现率）。

| 编号 | 交付 | 验收判据（做成什么样算好） | 真机证据 |
|---|---|---|---|
| V2-1 | 反应式终止 | 纯问答(20+20)≤3 步结束；无"tool not found:bash"误触发 | VM 真机 transcript |
| V2-2 | Thread 续接 | `/resume` 后 agent 能引用上轮文件/决策；跨进程重启不丢历史 | 重启前后 transcript 对比 |
| V2-3 | diff 编辑 | 生成 11834 字节文件完整落盘（复用 v0.1.1 贪吃蛇证据）；无整文件截断 | 文件字节数核对 |
| V2-4 | planner 瘦身 | Plan 阶段保留；空响应时静默降级单步，不卡 plan 阶段 | planner 空响应日志 + 任务仍完成 |
| V2-5 | 按需上下文(阶段一) | 默认不拉全仓；rg/ls/cat 按需；中型任务不 token 溢出 | token 计数日志 |
| V2-6 | 轻量 compaction | 长会话旧轮次摘要，不丢架构决策 | compaction 前后上下文对比 |
| V2-7 | 白盒兑现 | Plan→Observe→Reflect 相位真钩子 + EnvelopedEvent 信封字段 + Gap 上浮 | 源码接线 + 单测 |

---

## 9. 不改模型约束：prompt 格式适配（回应顾问缺口 #3）

顾问指出：同一模型在不同 prompt 格式下表现差异可能很大；Codex 的 prompt 格式针对 GPT 系列优化，hearth 用 deepseek。**本基准"不换模型"的结论成立，但需补一项调研**：hearth 当前的 system prompt / 工具描述格式是否对 deepseek v4 flash 最优？是否需要针对 deepseek 调整（角色设定、diff 格式提示、few-shot）。这是"不换模型也能提升表现"的低悬果实，执行窗口实现时实测验证，不能想当然。

---

## 10. v0.2 执行前技术自查清单（4 项，执行窗口实现时自查）

报告是灯塔不是施工图，但目标与做法已由顶层裁定。执行窗口开工前自查（可用上游 codex-rs 源码 + 网络）：

1. **diff applier 选型**：unified-diff vs SEARCH/REPLACE？结论倾向自实现 SEARCH/REPLACE / 轻量 unified-diff，不抽上游。
2. **最小 Thread 续接方案**：复用 `transcript.rs` 的可行性、回灌边界、SQLite vs JSONL 取舍。
3. **planner 瘦身可行性**：当前 planner 代码能否逐步瘦身而不破坏四阶段架构？Plan 阶段保留的最小契约是什么？
4. **deepseek prompt 格式适配**：当前 prompt 对 deepseek 是否最优？小样本实测对比。

---

## 11. v0.2 功能对标目标与能力记分卡（80-90 分）

用户给定量化目标：top-3 若得 95-100 分，hearth 抄完应达 **80-90 分** 等效能力。本节能化该目标为可验收的能力清单。

**评分口径**：80-90 分指**核心编码能力**对齐（能稳定完成贪吃蛇 / 象棋 / 小型 web 应用等中等任务，且体验接近 top-3），不要求字面每一项都满分。其中"多智能体 / 子代理"维度 top-3 有、hearth 暂不做——但 Aider 同样无子代理却仍是顶尖编辑型 agent，证明它**不影响核心编码能力达标**，故 consciously 延后（见 v0.2 任务书范围外），不拉低 80-90 目标。

| # | 维度 | top-3 参考能力 | hearth v0.2 必须达成（≈80-90 分等效） | 是否阻塞达标 |
|---|---|---|---|---|
| 1 | 终止 | 反应式 done/no-tool-call | 删 B1 写文件闸，纯问答≤3步，无误触发 | 否 |
| 2 | 规划 | 反应式+可选 plan | planner 瘦身保 Plan 阶段，空响应静默降级单步 | 否 |
| 3 | 上下文 | 按需+压缩(+repo-map) | 阶段一 rg/ls/cat + **轻量 compaction（必做）**；repo-map 阶段二（加分项） | compaction 必做 |
| 4 | 编辑 | diff（apply_patch/SEARCH-REPLACE） | 自实现 diff applier，消灭整文件截断 | 否 |
| 5 | 沙箱 | Landlock+seccomp 模式 | 已有，修 B6 误杀 verify | 否（已接近） |
| 6 | 续接 | Thread resume/fork | 复用 transcript+session_id+resume（fork/rollback 可不做） | 否 |
| 7 | 子代理 | spawn/子代理 | **v0.2 不做**（延后） | 不影响核心达标 |
| 8 | Hooks | 确定性护栏 | 轻量两条（fmt/clippy + 拦生产命令） | 否 |
| 9 | 记忆 | AGENTS/CLAUDE.md | Hearth.md 一层项目约定 | 否 |
| 10 | 可观测 | event/transcript | 补全 EnvelopedEvent 信封 + 保 transcript（可超额） | 否 |

**结论**：除第 7 项（consciously 延后）外，其余 9 项均为可实现的"抄作业"，且第 10 项 hearth 可凭白盒差异化**超额**。因此 80-90 分目标务实可达——前提是顶层把"做什么 / 做成什么样"钉死（本报告 §7 接入方案 + §8 验收门禁），执行窗口照此实现。

---

## 12. 一句话结论

> **不是模型不行，是分叉后把上游 Codex 最好的三件套（反应式终止、Thread 续接、apply_patch diff 编辑）弄丢了，又自创了一套重型 upfront planner 把它拖垮。** 三家生产级 harness 共同证明了"轻量反应式循环 + 按需上下文 + 确定性护栏"才是正路；hearth 只需把丢的接回来（轻量重实现、参考上游设计）、把重型 planner **瘦身而非删除**、再守住自己独有的白盒四阶段 / Gap / EnvelopedEvent，就能做出可用的、且比他们更可审计的 harness。问题确实在我们自己身上——但好消息是，答案就在自家上游 fork 里，不用从零发明。目标很朴素：**抄完达到 top-3 的 80-90 分等效能力**，有开源模型、架构、代码和网络可查，只要规划钉死、执行照做，这一步不该难。

---

### 附录：下一步建议（待用户拍板）
1. 下发 `hearth-harness-hardening-v013-taskbook.md`（R1–R5 快速修复）给执行窗口——不依赖本基准，可立即做。
2. **v0.2 任务书已出** `hearth-v02-taskbook.md`（功能对标 80-90 分目标 + 能力记分卡 + 工作流 + 总验收门禁），直接交执行窗口；技术细节执行自查上游 codex-rs + 网络，目标与做法已顶层裁定，勿擅自改方向或删白盒差异化。
3. 明确"白盒差异化死守清单"（四阶段/Gap/EnvelopedEvent）写进顶层规划，防止对齐时误删——尤其 planner 是四阶段之一，**瘦身不删**。
