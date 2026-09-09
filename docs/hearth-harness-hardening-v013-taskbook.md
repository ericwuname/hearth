# Hearth Harness 加固任务书 v0.1.3 · 正式签发版（纠偏版）

> ┌─ 签发栏 ───────────────────────────────────────────────────────────
> │ 版本：v0.1.3（正式签发版 · 快速修复）
> │ 签发日期：2026-08-23
> │ 签发人：顶层守门员（全局架构 / 评审 / 验收 / 任务书）
> │ 接收方：执行窗口
> │ 下发时序：先于 v0.2 能力对标任务书下发（R1–R5 落地后，v0.2 在其上补齐能力级对标）
> │ 依据：release/手工测试v0.1.2.txt 真机测试 + 守门员源码核验
> │ 目标：先收"高 ROI 且范围可控"的内核逻辑缺陷（B1/B4/B5/B6/B7），
> │   把 hearth 从"答个 20+20 都要 9 步"拉回"像样能用"
> │ 本任务书只收快速修复（R1-R5）；B2 REPL 连续性、B3 loop 对齐 Codex 开源实现，
> │   列为 v0.2 第一/第二优先（见 §4，已按用户 2026-08-23 反馈纠偏）
> └────────────────────────────────────────────────────────────────────
>
> ⚠️ **纠偏说明（2026-08-23 用户反馈后修订）**：原稿曾把 B3 误判为"模型天花板/换模型"。**这是错的。** Codex CLI 已于 2026-08-19 全面开源（`github.com/openai/codex`，Apache-2.0，v0.149.0），hearth 本就是从其 fork 的；同模型（deepseek v4 flash）在 Codex 里表现远好于 hearth，证明是 **hearth fork 后把 loop 改坏**（加了写文件门禁、沉重 TaskGraph 规划器、全文件写入代替 diff），属**回归**非模型上限。B3 正确方向 = 对齐 Codex 开源 loop，不是换模型。

---

## 0. 真机证据（来自手工测试 v0.1.2）

| 现象 | 日志位置 | 根因 | 本任务书 |
|---|---|---|---|
| `20+20` 答对了却被强制 replan 3 次、9 步才 Done、冒出 `tool not found: bash` | 日志 235-314 行 | loop 门禁强制"写文件才算完成" | R1 |
| 几乎每个任务 `budget low 11% give up` / `budget exhausted` | 多处 | 预算过小 + replan 死循环烧预算 | R2 |
| grep `/home/wutao/codex_6d` → `path traversal denied` | 日志 1002-1014、1089 行 | grep 拒绝所有绝对路径 | R3 |
| `node --check game.js` → `exit code: -1`；`tail` → `-1` | 日志 176-189、1356 行 | 沙箱杀验证命令 | R4 |
| REPL 提示符行混入 WARN 日志 | 日志 625-636 行 | REPL 输出流未分流 | R5 |
| 象棋 plan 阶段 `read body failed` 放弃；planner `EOF at line 1 column 0` | 日志 62-69、79 行 | deepseek 传输/空响应（B3，本任务书不收） | 见 §4 |

---

## R1（🔴 B1）：解耦"任务完成"与"写文件"——最高优先级

**根因**：`crates/agent-core/src/loop.rs` 的 v20/v22/v24-post 门禁把"NO write executed"当作未完成信号，强制 replan：
- `:1217` v20 all_done gate: task graph complete but NO write executed — forcing replan
- `:1458` no tool_calls but NO write executed — forcing replan (v22 gate)
- `:1423` no write after N replans but acted — accepting Done（仅在第 3 次 replan 后才勉强接受）

**修复意图**（执行窗口细化实现，守门员只定判据）：
1. 引入**任务意图分类**：当一轮产生"最终文本回答给用户"且**无待执行 tool_calls** 时，对非产物类任务直接判 Done，**不再强制 replan 逼写文件**。
2. 仅在**产物类意图**下保留"必须写文件"约束：启发式——goal 含 写/创建/生成/文件/代码/保存 等词，或历史轮已写过文件，才要求 write。
3. 删除 v22 gate 对纯问答/内观/聊天的强制 replan；保留对"用户明确要产物但没写"的提醒。
4. 修复 `tool not found: bash` 的伴生症状（replan 多次后工具注册表异常）——dispatch 前确保工具集完整，replan 不重建/丢失工具注册。

**验收判据（守门员独立核验）**：
- `hearth chat "20+20 等于多少"` → **≤3 步**给出干净答案，**无 replan 循环**、**无 tool not found**，直接 Done。
- `hearth chat "你擅长什么"` 类纯问答同理，不再被逼写文件。
- 产物类任务（"写个贪吃蛇"）仍要求写出文件才算 Done（原有约束不丢）。

---

## R2（🔴 B4）：预算校准 + 杜绝 replan 烧预算

**修复意图**：
1. chat 默认 budget 从当前隐性上限（~16-20 步）提到 **40**；REPL 每轮预算合理化。
2. planner 空响应回退（`EOF at line 1 column 0`）**不计入有效 step 预算**且**短时缓存**（同 goal 5s 内不重复打一次 25s 的规划调用）。
3. replan 触发时若历史已含有效进展，不重置预算计数；连续 replan 无进展直接 fail-fast 给用户可行动错误，而非耗到 budget exhausted。

**验收判据**：
- 贪吃蛇类任务在预算内完成或给出明确"需更大预算"提示，而非 11% 静默 give up。
- planner 连续失败时单任务总时长显著下降（不再每次 25s×N）。

---

## R3（🟡 B5）：path traversal 放宽——允许项目/家目录内绝对路径

**根因**：`crates/tools-builtin/src/grep.rs:84-89` 对所有 `is_absolute()` 路径一律 `path traversal denied`。

**修复意图**：
1. 定义**允许根**（allowed_root）：项目目录（cwd 或其父项目根）或用户家目录 `~`。绝对路径若 canonicalize 后落在 allowed_root 内则放行；仅拒绝 escape 出根的路径（如 `/etc`、`/usr`）。
2. read/glob/write 等同理放宽（grep 先行，其余工具类比）。
3. 保留安全语义：仍拒绝越权读系统目录。

**验收判据**：
- `grep pattern: xxx path: /home/wutao/codex_6d` → 正常搜索（不再 denied）。
- `grep path: /etc/shadow` → 仍 denied（防越权）。

---

## R4（🟡 B6）：修复沙箱杀验证命令（exit -1）

**根因**：`node --check` / `tail` 在沙箱内返回 `exit code: -1`，疑似 seccomp KILL 或 bash 探针 profile 误杀（参考 `bash.rs:16` 注释：默认探针曾杀掉编译命令）。

**修复意图**：
1. 复现并定位：在沙箱内单独跑 `node --check x.js` / `tail -n 5 f`，用 strace/bpf 看是被 KILL 还是 bash 返回 -1。
2. 若 seccomp 缺 syscall（node 用到的如 `prlimit64`/`membarrier` 等）→ 补白名单；若 bash 探针 profile 误判 → 修正探针。
3. 确保验证类命令返回**真实退出码**，否则确定性验证层（v0.2）无法依赖。

**验收判据**：
- `bash cmd: node --check game.js` → 返回 0（语法 OK）或真实非 0（语法错），**不再 -1**。
- `bash cmd: tail -5 f` → 正常输出。

---

## R5（🟡 B7）：REPL 输出流分流，日志不污染提示符

**根因**：REPL 模式下 WARN/ERROR 日志直接混入 `hearth> ` 提示行（日志 625-636 行 `hearth> 〉2026-...WARN...`）。

**修复意图**：
1. 日志统一走 stderr 或独立渲染通道；REPL 提示符行只显示用户输入回显与 agent 输出。
2. 多行日志块不插入到用户输入行中间。

**验收判据**：REPL 中 WARN 日志出现在独立区域，提示符 `hearth> ` 行干净无夹杂。

---

## R6（通用门禁）
- `cargo fmt` / `clippy` / `build` 0 警告 0 错误。
- 现有单测全过 + 为 R1/R3 加回归单测（纯问答 ≤3 步完成；绝对路径在项目内放行）。
- 重跑真机五任务套件（象棋/五子棋/贪吃蛇/算术/问答）确认无回归。

---

## §4. v0.2 两项架构级修复（本任务书不收，但已排定优先级——按用户 2026-08-23 反馈）

### B2 REPL 连续性（v0.2 第一优先 —— 用户对齐 Codex/WorkBuddy 基线）
- **现状**：`repl.rs:96` 每轮 `create_session` → 无多轮记忆，"连续对话/继续任务"必败，agent 还骗用户"能保持会话内连续记忆"。
- **参考 Codex 开源实现**：Codex 用 **Thread / Turn / Item** 三层——Thread 持久化会话容器（可 resume/fork/archive），Turn = 一次用户输入触发的一整轮 agent 工作，Item = Turn 内原子事件（消息/工具调用/diff）。
- **修复方向**：REPL 复用单 Thread，把历史 Turn 喂进 context，消息历史落盘可 resume。直接对齐 Codex/Claude/WorkBuddy 的连续对话基线能力。
- **不做**：原稿提的 ThreadStore 10-20k 行超前基建（单人场景不需要 fork/archive 全套）——只做"单 Thread 复用 + 历史喂入 + 落盘 resume"，几百到一两千行。

### B3 loop 对齐 Codex 开源实现（v0.2 第二优先 —— 纠偏后方向）
- **纠偏**：原稿误判为"模型天花板/换模型"。实为 **hearth fork Codex 后把 loop 改坏**的回归。Codex CLI 已于 2026-08-19 全面开源（`github.com/openai/codex`，Apache-2.0，v0.149.0，codex-rs Rust 核心），hearth 即其 fork，VM 上 `/home/wutao/codex_cli` 就是该源码。
- **三个具体回归点（对照 Codex loop）**：
  1. **终止逻辑**：Codex 一轮只在模型发 `done` 事件时结束，无"必须写文件"门禁。hearth 的 v22/v24 写文件门禁（已归 B1/R1 修）是 fork 后新增的病。
  2. **规划器**：Codex **不做沉重 upfront TaskGraph 规划**，上下文按需 `rg`/`ls`/`cat` 拉取；简单任务（贪吃蛇）直接写。hearth 强塞结构化规划 → 50% 空响应（`EOF at line 1 column 0`）→ 象棋死在 plan 阶段。**修复 = 让规划可选/轻量，简单任务跳过 TaskGraph， planner 容错空响应不烧 25s**。
  3. **写入方式**：Codex 用 `apply_patch`（unified-diff）而非让模型吐整个文件 → 输出 token 少、无截断。hearth `write_file` 强吐全文件 → 截断病根。**中长期可考虑引入 diff 式编辑**，根治 write_file 截断类 bug（非 v0.1.3 范围）。
- **执行窗口动作**：diff `hearth crates/agent-core/src/loop.rs` 与 codex-rs 的 loop 实现，把上述三处回归点 re-import 上游证明可用的逻辑；必要时直接借鉴 codex-rs 对应模块（Apache-2.0 合规）。

---

## §5. 战略定调（用户拍板后的结论）
- **不换模型**：deepseek v4 flash 在 Codex 里能用，说明 hearth 集成层的问题，集中精力修集成/loop，不甩锅给模型。
- **对齐而非照搬**：保留 hearth 白盒四阶段状态机 + EnvelopedEvent（差异化优势），但把 loop 的终止/规划/写入三处回归对齐 Codex 开源实现。
- **先追平基线**：B2（连续对话）+ R1-R5（基础可用）先让 hearth 达到"Codex/WorkBuddy 正常功能"水位，再谈超越。

---

## 签发回执

| 项 | 内容 |
|---|---|
| 签发人 | 顶层守门员 |
| 签发日期 | 2026-08-23 |
| 接收窗口 | 执行窗口 |
| 接收确认 | ________________（执行窗口签收后回填） |
| 下发时序 | 先于 v0.2 能力对标任务书；R1-R5 落地后 v0.2 在其上补齐能力级对标 |
| 过闸提交 | R6 通用门禁全过 + 真机五任务套件无回归；守门员独立核验 |

> 本任务书为顶层正式签发件（快速修复，范围可控、ROI 高）。B2/B3 架构级修复已按用户 2026-08-23 反馈纠偏，列入 v0.2 第一/第二优先，不在本版范围。
