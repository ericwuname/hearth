# Hearth 核心问题清单（v0.2.23 更新版）

**生成时间**: 2026-08-27（初版 v0.2.22）  
**更新时间**: 2026-09-02（据两份手工测试记录补全）  
**版本**: v0.2.22 → 测试记录覆盖 v0.2.20 / v0.2.23  
**代码路径**: `crates/agent-core/src/loop.rs` (10,067 行)  
**审计范围**: 只读分析 + 两份手工测试记录（release/手工测试v0.2.20.txt、release/手工测试v0.2.23.txt）交叉印证  
**说明**: 本文档为初版 v0.2.22 的更新，新增内容以 `【新增】` 标注，证据增强以 `【实证更新】` 标注。

---

## ⚠️ 关键前置发现：二进制 / 源码漂移（已回源码二次核实，2026-09-02 修正）

手工测试记录与当前源码**确实存在版本差**，但本清单初版对若干断言的"源码侧判断"**有误**，已在源码中逐一核实修正：

| 日志现象（初版断言） | 当前源码实况（loop.rs / context.rs / patch.rs 核实） | 结论 |
|---|---|---|
| `stalled: 2 consecutive replans` 检测"当前源码已无"（v0.2.23:575 诊断） | **存在且带恢复**：loop.rs:2700-2827，阈值=2（`graph_stall_count>=2`）；且 `nodes.len()>1` 防护防止纯读任务误杀（2713-2716 注释） | 该诊断**错**——测试二进制比当前源码旧 |
| `steps_used` 恒 4/50 不递增（P1-5） | `inc_step()` 已实现并被调用（loop.rs:3373/3403/5007；context.rs:127） | 当前源码已接；日志现象疑为旧二进制残留 |
| 水位显示 40% 但"内部 94%"、压缩不触发（P0-β/P1-6） | `compact_char_threshold()` 默认走 provider-aware 注入值（Agnes 128K→~196k，context.rs:225-241）；`total_chars/196k≈40%` 正确；"94%"是 history/total 占比，非窗口水位 | 非失真；设计如此 |
| 中文 UTF-8 字符边界 panic（P0-ζ） | 压缩摘要 `summarize_turn` 已改 char 安全切片（context.rs:430 `.chars().take(60)`）；但 `apply_patch` 错误格式化 `&search[..search.len().min(80)]`（patch.rs:127/135/145）**仍按字节切，中文长 search 块在错误路径必崩** | 维护 P0-ζ，精确定位到 patch.rs 错误路径 |

**含义**：
1. 你手工测试跑的 **hearth 二进制比当前 `crates/` 源码旧**（尤其 T4 恢复逻辑、inc_step 接线在源码中已存在）。
2. 凡标注"来自运行日志"的，需回当前源码二次核实——本清单已对 T4 / steps_used / 水位 / char-boundary 四项完成核实并修正初版误判。
3. 项目内部另有自己的 RC/W/T/P1-8 等编号（如 patch.rs:100 标注 P1-8=宽松空白匹配）；本清单的 P0-α…P2-x 为**分析者编号**，与项目内部编号并行不冲突。

**仍需回源码确认的项**：P1-5（introspect 的 steps_used 与 RunReport 步数在旧二进制中口径不一致，源码侧 inc_step 已接，需确认两处是否同源）。

---

## 一、P0 级阻塞问题（直接导致任务失败）

### P0-α: 完成判定硬约束——必须写文件

**位置**: `loop.rs:3150-3250` (`completion_fact_check`)

**问题描述**:  
`completion_fact_check()` 强制要求 `!written_files.is_empty()`，否则返回 `Err("no artifact written this run")`。

**【实证更新】v0.2.23 运行日志直接复现**：
- `手工测试v0.2.23.txt:1254-1259`（run-011）：Agent 只做了 `ls`/`find`（只读），Reflect 决策 `Error（终止）；依据：连续错误 0、本轮写盘 0 个、跨轮产物记录 0 个` → `✗ Done (5 steps)`。
- `手工测试v0.2.23.txt:1838-1843`（run-014）：读取 `r21_matrix.log` 后同样因"写盘 0 个"触发 `give_up (6 steps)`。

**根因**: 完成决策与进度追踪的语义不一致——进展可来自读/分析，但完成必须来自写。

**修复建议**:
1. 扩展 `goal_requires_product()` 分类边界，显式支持"输出到 stdout/stderr 的问答任务"。
2. 或为 QA 类任务添加独立完成路径（以 `verify_acceptance_criteria()` 的 Cmd 结果作为替代事实）。

**影响评估**: 高——所有非写入类任务均受此约束，且已在真实机高频触发 give_up。

---

### P0-β: 上下文压缩墓碑过短（水位显示为语义问题，非失真）【实证修正】

**位置**: `loop.rs:2779` + `context.rs` (`COMPACT_CHAR_THRESHOLD=32_000` + provider-aware 注入)

**问题描述（原）**: 超过阈值触发压缩，仅保留最后 2 轮 + 60 字符墓碑，丢失原始 goal 约束。

**【实证修正 2026-09-02，回源码核实】**：初版"水位显示失真 40% vs 内部 94%、压缩不触发"断言**不成立**：
- `compact_char_threshold()`（context.rs:225-241）优先级：env 覆盖 > `injected_compact_threshold`（provider-aware，Agnes 128K→~196k 字符）> 遗留常量 32k。
- 运行日志 `total_chars=79,712` / `196,000 ≈ 40%` —— **展示值 40% 正确**，是"距压缩阈值的接近程度"的真实水位。
- 日志里"内部 94%"=`history_chars 75,134 / total_chars 79,712`，是**历史占总量比例**，与窗口水位是两码事，不构成失真。
- "压缩不触发"是因为测试仅推到 ~80k 字符，未达 ~196k 注入阈值——设计如此，非 bug（需 `HEARTH_COMPACT_CHAR_THRESHOLD` 或注入更小阈值才会更早触发）。

**真正残留问题（保留）**：压缩墓碑仅 `summarize_turn` 截断 goal 为 **60 字符**（context.rs:418-434），丢失原始 goal 关键约束与中间结论——这是压缩后 LLM "失忆/replan 原地踏步" 的真实根因之一。

**修复建议**:
1. 墓碑写入 `original_goal（截断前保留关键约束）+ 关键中间结论 + written_files 路径`，而非 60 字符裸截断。
2. 报告/introspect 中明确标注 `compact_pressure_pct` 语义="距压缩阈值比例"及当前生效阈值来源（provider-aware / env / 常量），避免用户误读为"窗口填充率"。

**影响评估**: 中——墓碑过短是压缩后质量下降的真实诱因；水位"失真"初版断言已撤销。

---

### P0-γ: session_written_files 回填时机错误

**位置**: `loop.rs:2079-2118` (`rc52_route_done_if_session_artifacts`)

**问题描述**: `run()` 入口 `self.written_files.clear()` 导致跨 run 产物记录丢失；RC52 在 give_up 消费端回填，但若 Reflect 提前触发 GiveUp（如 `steps_without_progress >= 5`）回填可能未执行。

**现状**: RC52 已加，但测试 `test_rc52_cross_turn_artifact_hydration_on_giveup` 仍依赖"给定时机"。

**修复建议**: 回填逻辑提前到 `run_abort` 检查点之前，确保所有 give_up 出口都经过回填。

**影响评估**: 中等——真实机测试 A×5 全中、B×5 中 4，约 20% edge case 未覆盖。

---

### P0-δ: 完成报告不透明（"✓ Done (N steps)" 不解释做了什么）【新增】

**位置**: `loop.rs` 完成报告生成 / `RunReport` 输出（执行报告 markdown 模板）

**问题描述**: 终态标记 `✓ Done (21 steps)` / `✗ Done (5 steps)` 只给出结论性数字，**不说明**：
1. 这 N 步具体做了什么操作；
2. 结论是怎么得出的；
3. 用户如何验证结果。

**【实证】v0.2.23 用户三次质疑**：
- `手工测试v0.2.23.txt:1276`：用户指出"我上一轮输出了 ✓ Done (21 steps)，但我不知道我实际上做了什么"。
- `手工测试v0.2.23.txt:1331-1358`：Agent 自认"只运行了两次 bash（cargo test 触发 exit 159、cat lib.rs），没有写任何文件、没有完成任何实质性任务，却输出了 ✓ Done (21 steps)"——**空完成标记**。
- `手工测试v0.2.23.txt:2481`：改进项明确列入"改进 ✗ Done 报告，明确说明卡在哪里、为什么放弃"。

**根因**: 完成报告模板只渲染步数标记，缺少"操作清单 + 事实依据 + 验证指引"三段式结构。

**修复建议**: 完成/失败报告强制包含：
- 已执行工具调用清单（命令 + 结果摘要）；
- 完成判定的事实依据（写盘文件 / 验证命令输出）；
- 供用户复核的命令或文件路径。

**影响评估**: 高——直接破坏"用户知情权"，是沟通闭环断裂的核心表现之一。

---

### P0-ε: give_up 决策分散多出口 + 恢复不统一【新增，2026-09-02 回源码修正】

**位置**: `loop.rs` T4 plan 臂（`Err` 旁路，2820）+ Reflect 臂 + 预算臂

**问题描述（初版"无恢复路径"已修正）**：当前源码的 T4 停滞检测**并非无恢复**——它带三层恢复（loop.rs:2762-2818）：
1. `acceptance_criteria` 非空且已验证通过 → 转 `Done`（override give_up）；
2. `acceptance_criteria` 非空未验证 → 用 `acceptance_replan_count` reserve（仅 1 次）现场核验，通过则转 `Done`；
3. 无 criteria 且 `goal_requires_product` → 标记 `giveup_unverified` 并经 RC52 回填会话产物。

**真实结构性缺陷（仍成立）**：
- **give_up 决策分散在至少三个出口**（T4 plan 臂 `Err`、Reflect 臂、预算臂），各出口验证逻辑不一致；T4 的 `Err` 是"旁路"（loop.rs:2750 注释"T4 停滞 Err 是 GiveUp 的旁路——修 D 拦截只覆盖 Reflect GiveUp 臂，导致事实上已完成但图停滞的任务未核验即失败"）。
- RC52 回填逻辑被迫在 **plan 臂与 Reflect 臂重复**（loop.rs:2810-2818 注释"此前回填只在 Reflect 臂，流程在 plan 阶段终止到不了 reflect（集成测试 RED 实证）"）——这正是 P0-γ 的根因，也是"分散出口"的直接代价。
- 纯无-criteria 的 product 任务最终只标记 `giveup_unverified` 后放弃，无换策略/澄清/求助通道（用户的"报错就该重新思考新方案"诉求未被满足）。

**【实证】v0.2.23 用户质疑 + 数据**：
- `手工测试v0.2.23.txt:2289/2451`：用户"报错就 give_up 然后停止，难道不是接收报错后重新思考、规划新方案吗"。
- `手工测试v0.2.23.txt:2433-2468`：stalled 链 `2 consecutive replans → T4 → give_up`；run-014/015/020/022/023/026/034 反复 stalled。

**修复建议**:
1. **收口为单一 `should_give_up()` 裁决函数**，所有出口统一调用，内置"无验证不放弃 + 已验证通过不否决"原则（消除 Reflect 臂/plan 臂重复与遗漏，根治 P0-γ）。
2. 在中止前插入"恢复策略栈"：澄清 → 缩范围 → 换策略 → 求助用户，逐级降级。
3. 阈值维持 T4=2（总包禁令只锁阈值不锁消费端），但恢复逻辑统一化。

**影响评估**: 高——分散出口是真实机"已达成却被判失败 / 无恢复即弃"的结构性根因，且与 P0-γ 同源。

---

### P0-ζ: apply_patch 错误路径中文 UTF-8 字符边界 panic【新增，2026-09-02 精确定位】

**位置**: `crates/tools-builtin/src/patch.rs:127 / 135 / 145`（错误格式化 `&search[..search.len().min(80)]`）

**问题描述**: `apply_patch` 在 search 块**匹配失败/不唯一**并生成错误提示时，用 `&search[..80]` **按字节切片**截取 search 片段；若 search 为中文且第 80 字节落在多字节字符内部（如 '但' 的 bytes 79..82），即 panic `end byte index 80 is not a char boundary`。

**【实证】v0.2.20 真实崩溃 + 源码确认**：
- `手工测试v0.2.20.txt:336-339`：`panicked at crates/tools-builtin/src/patch.rs:128:32: end byte index 80 is not a char boundary; it is inside '但' (bytes 79..82)`。
- 当前源码 patch.rs:127/135/145 均为 `&search[..search.len().min(80)]`——**该 bug 在当前源码仍存在**（仅在不匹配的错误路径触发；正常匹配走 `content.matches`/`replacen`，char 安全）。
- 注：压缩摘要 `summarize_turn`（context.rs:428-434）**已改** `.chars().take(60)`，不再崩——故崩溃点不在压缩而在 apply_patch 错误格式化。

**根因**: 错误提示构造用字节索引切片字符串，未处理多字节字符边界。

**修复建议**: 改用 char 安全切片，如 `let snippet: String = search.chars().take(80).collect();` 或 `&search[..search.char_indices().nth(80).map_or(search.len(), |(i,_)| i)]`。

**影响评估**: 高（触发条件明确）——中文环境下，含中文且 ≥80 字节的 search 块一旦匹配失败即 panic，整个 agent 线程崩溃（虽隔离重建，但丢本轮进度）；且会掩盖"search 未匹配"这一本应正常返回的错误信息。

---

## 二、P1 级结构性缺陷（影响稳定性与可扩展性）

### P1-1: 进度追踪语义不一致

**位置**: `loop.rs:4700-4800` (main loop) + `loop.rs:~3000` (do_act)

**问题描述**: `steps_without_progress` 仅在工具调用失败或 TaskGraph 无事实进展时递增；read/glob/bash 成功、write 成功均不递增 → "只读探索型"任务 progress 永远为 0，直到预算耗尽才 give_up。此为**故意设计**（代码注释："pwd/ls/read 类成功不算 progress"），但与用户直觉相悖。

**影响评估**: 高——许多 give_up 场景的隐性根因。

---

### P1-2: 停滞检测的图签名粒度（恢复已存在，分散出口见 P0-ε）【实证修正】

**位置**: `loop.rs:2700-2827` (T4 stall detection，已核实存在)

**原问题**: 全量 hash 比较过粗（同结构不同子任务误判停滞）/过细（描述同义词微调漏判）；阈值=2（`graph_stall_count>=2`）。

**【实证修正 2026-09-02，回源码核实】**：初版"当前源码已无此检查"断言**错**——T4 检测**存在且带恢复**（见 P0-ε 三层恢复）。修正后的待改进点：
- 全量 `id+description+deps+status` hash 仍偏粗/偏细（同结构不同子任务误判；描述同义词漏判）；
- 恢复逻辑正确但**分散在 plan 臂**（见 P0-ε），未收口到单一裁决。

**修复建议**:
1. 引入"稳定期"：连续 N 轮 graph 不变才判停滞。
2. 比较"status 变化节点集"而非全量 hash。
3. 与 P0-ε 的 `should_give_up()` 收口联动。

**影响评估**: 中等——检测与恢复均在，但粒度与出口一致性待优化。

---

### P1-3: acceptance_criteria 验证不完整

**位置**: `loop.rs:3400-3550` (`verify_acceptance_criteria`)

**缺失能力**: 无 `FileMatchesRegex`、无 `CommandOutputContains`、无 `PortListening`、无 `ExitCode` 检查（仅检查命令是否执行）。

**影响评估**: 中等——复杂任务无法精确指定完成条件。

---

### P1-4: 审批策略权限模型过于简单

**位置**: `loop.rs:3400-3550` (approval policy)

**缺失能力**: 无白名单命令、无风险分级、无上下文感知。

**影响评估**: 低——当前满足基本需求，扩展性受限。

---

### P1-5: steps_used 指标口径（旧二进制残留，待源码确认）【新增→降级，2026-09-02 修正】

**位置**: `ContextManager` / 主循环步数追踪（`introspect` 输出 `steps_used` vs `RunReport.steps`）

**问题描述（初版）**: `steps_used` 在整轮会话中**始终为 4/50**，无论实际执行多少次工具调用。

**【实证修正 2026-09-02，回源码核实】**：当前源码 `inc_step()` 已实现且被调用（loop.rs:3373/3403/5007；context.rs:127 `+= 1`），`budget_exhausted` 也依赖它（context.rs:86）。因此"恒显 4/50"**极可能是旧二进制残留**——但日志中暴露一个**真实不一致**：
- 旧二进制里 **RunReport 步数递增**（5/6/9/22/25…）而 **introspect `steps_used` 恒 4/50**——两处计数疑似不同源。

**待确认**: 回当前源码核对 `update_body_state`（loop.rs:1389 用 `ctx_mgr.steps_used()`）与 RunReport（loop.rs:4263 同字段）是否同源；若同源，则纯旧二进制残留，本项可关闭。

**修复建议（仅当源码确证两处不一致时）**: 统一两处步数口径到 `ctx_mgr.steps_used()`。

**影响评估**: 低（疑似旧二进制残留）——但需在 CI 加一条断言：introspect 与 RunReport 步数一致。

---

### P1-6: 水位指标语义澄清（非失真，合并自 P0-β）【实证修正】

**位置**: `loop.rs:1372-1399` (`update_body_state` / `compact_pressure_pct`)

**问题描述（初版"不一致"已修正）**: `compact_pressure_pct = total_chars / 生效阈值`（loop.rs:1382-1386）。初版称"展示 40% 但内部 94%、压缩不触发"——**已核实非失真**（见 P0-β：40% 正确，94% 是 history/total 占比）。

**真实待改进点（语义清晰度）**: 字段名/报告未说明"距压缩阈值比例"语义与**当前生效阈值来源**（provider-aware ~196k / env / 32k 常量）。用户易误读为"上下文窗口填充率"而低估/高估风险。

**修复建议**: 在 introspect/报告里附 `compact_threshold` 实际值与本语义说明（与 P0-β 修复 2 同做）。

**影响评估**: 低（清晰度，非正确性）。

---

### P1-7: 写前目标校验守卫误报（extract_mentioned_files 启发式过粗）【新增，2026-09-02 源码确认】

**位置**: `loop.rs:683` (`extract_mentioned_files`) + `loop.rs:1406-1451` (`check_write_target_mismatch`)

**问题描述**: `extract_mentioned_files` 将用户文本按空白切分，凡"含 `.` + 长度>3 + 含字母 + 全为字母数字/./_/-"的 token 即当作文件名。这会把**英文句末带点的普通词**（如 `call.` / `done.` / `glob.` / `task.` / `re-planned.`）误判为"用户提到的文件"。

**【实证】v0.2.20:820-821**：Agent 写 `conversation_result.md` 时收到警示
> "[写前目标校验] 用户消息提到文件 call./done./glob./re-planned./task.，但本次 write_file 目标为 /home/wutao/conversation_result.md——若非有意为之请核对写入路径"

——显然是误报（用户消息里这些是英文指令词，不是文件名）。

**根因**: 启发式不要求真实扩展名（`.rs`/`.py`/`.md` 等）或路径分隔符 `/`，仅凭"字母词+句点"即判定，句末标点被当扩展名。

**修复建议**: 仅当 token 含已知源码/文档扩展名，或含 `/` 路径分隔符时才视为文件引用；或对候选做常见英文停用词过滤。

**影响评估**: 低（非阻塞——守卫只发 `ThinkSummary` 警示不拦执行），但**噪声会侵蚀用户对校验信号的信任**，且可能误导 Agent 自我怀疑写入路径。

---

### P1-8: give_up 决策分散多出口（可维护性 + 验证一致性风险）【新增，2026-09-02 源码确认】

**位置**: `loop.rs` 多处——T4 plan 臂 `Err`(2820)、Reflect 臂、预算臂；RC52 回填在 plan 臂(2816) 与 Reflect 臂重复

**问题描述**: give_up / Done 的最终裁决散落在多个出口，各处验证逻辑不统一：
- T4 停滞在 **plan 阶段**直接 `return Err(...)`（loop.rs:2820），**绕过 Reflect 臂**；
- RC52 产物回填逻辑因此在 **plan 臂与 Reflect 臂各写一份**（loop.rs:2810-2818 注释"此前回填只在 Reflect 臂，流程在 plan 阶段终止到不了 reflect（集成测试 RED 实证）"）；
- 预算臂、Reflect 臂各有独立中止条件。

**根因**: 历史上"边发现边补丁"——每发现一个漏掉的 give_up 出口就就地加一段，未收口到统一裁决。

**风险**: 验证一致性难保证（某臂漏校验即"已完成却判失败"）；新增中止条件需改多处，易遗漏（P0-γ 即此代价）。

**修复建议**: 抽象单一 `fn decide_termination() -> Termination` 或 `should_give_up()`，所有出口统一调用，内置"无验证不放弃 + 已验证通过不否决"原则；RC52 回填只在该函数内做一次。

**影响评估**: 中（架构/一致性，非单点崩溃）——但它是 P0-γ、P0-ε 反复"补丁式修复"的根因，收口后显著降低回归风险。

---

## 三、P2 级优化机会（提升体验与可靠性）

### P2-1: LLM 调用单点出口限制

**位置**: `loop.rs:3100-3176`

**问题描述**: 每 `step(phase)` 最多一次 `provider.chat()`，无法 chain-of-thought。

**对比**: Claude Code / Codex CLI 支持单轮内多步推理。

**修复建议**: 引入 `ReasoningLoop` trait，允许单 step 内多次调用（带 max_turns 上限）。

**影响评估**: 中等。

---

### P2-2: experience store 未自动写入

**位置**: `crates/experience/src/lib.rs`

**问题描述**: embedding + 余弦相似度检索已实现，自动写入机制未实现（无任务完成后的经验提取、失败记录、跨会话迁移）。

**影响评估**: 低。

---

### P2-3: 沙箱权限过度严格【实证更新】

**位置**: `crates/sandbox/src/lib.rs`

**原问题**: Landlock 默认拒绝所有写入，需显式白名单；错误提示不清晰。

**【实证更新】v0.2.20 Cap Sweep 实测**：
- `手工测试v0.2.20.txt:398`：38 条命令中 7 成功、2 因 `RC=159（沙箱拦截）`失败（`cp`、`mv`、`sed -i`）、29 未执行。
- `手工测试v0.2.20.txt:777`：沙箱疑似同时拦截出站网络连接（与 cp/mv 同类 RC=159）。

**修复建议**: 增加友好错误（区分"沙箱拒绝"vs"系统拒绝"）；评估常用命令（cp/mv/sed -i）是否应纳入白名单。

**影响评估**: 低——Landlock FS_RW bug 已修（`sandbox/lib.rs:293`），UX 仍可改进。

---

### P2-4: 网络出网被完全拦截【新增】

**位置**: 沙箱 egress / `HEARTH_EGRESS_ALLOWLIST`

**问题描述**: 即便配置了出网白名单，所有外连均超时；`web_fetch` 在 bash 内不可用。

**【实证】v0.2.20 联网实测**：
- `手工测试v0.2.20.txt:755-765`：`HEARTH_EGRESS_ALLOWLIST=rust-lang.org,crates.io,docs.rs,doc.rust-lang.org,github.com` 已配，但 `curl` 五个域名**全部 timeout**。
- `手工测试v0.2.20.txt:751-752`：`web_fetch` 工具在 bash 环境无法直接调用（需工具层支持）。
- `手工测试v0.2.20.txt:773`：可能原因——虚拟机网络未启用 / 沙箱拦截出站 / 防火墙阻止。

**根因（待定）**: 沙箱 egress 策略或 VM 网络配置，需环境侧排查（非纯代码缺陷）。

**修复建议**: 确认 VM 网络是否启用；沙箱 egress 白名单是否真正放行；提供 `web_fetch` 作为可调用工具而非仅依赖 bash curl。

**影响评估**: 低（能力缺失，非崩溃）——但联网检索类任务完全不可用。

---

### P2-5: 沟通闭环未形成（流程缺陷）【新增】

**位置**: 流程层（BUG_LEDGER.md → 开发窗口）

**问题描述**: 住户 AI 发现问题登记到 `BUG_LEDGER.md`，但**无人评审/修复/回写**，闭环断裂。

**【实证】v0.2.20:264-298**：
- 全链路 `住户AI → BUG_LEDGER.md → 文件落盘 →（无人读取）→ 开发AI无法获知`。
- 缺失环节：Bug 评审 ❌、修复分配 ❌、测试验证 ❌、反馈闭环 ❌。
- 证据：`.hearth/reports/` 有 101 个 session 目录，但无证据表明它们在读 bug 台账；住户 AI"从未收到任何'你的 bug 已修复'的反馈"。

**关联台账**: `BUG_LEDGER.md` 登记 BUG-009/010（写入成功读失败）、BUG-011/013（resume core dump）等 13 项，5 已确认 8 待验证。

**修复建议**: 建立自动评审机制（定期读台账 → 分配 → 修复 → 回归 → 回写）；或将台账接入 issue 系统。

**影响评估**: 低（流程）但高（积累技术债）——手动测试发现的问题长期无人消费。

---

## 四、已验证通过的能力（正面证据）

> 来自 `手工测试v0.2.23.txt:1628-1635` 的安全/韧性测试矩阵，**全部 PASS**：

| 编号 | 验证项 | 结果 | 证据 |
|------|--------|------|------|
| P0-1 | fail-closed（无 key 无 allow 拒绝启动）+ 请求鉴权（无 key=401/正确=201/错=403） | ✅ PASS | refused_start=True; nokey=401; ok=201; wrong=403 |
| P0-3 | `GET /api/v1/sessions` = 200 | ✅ PASS | — |
| P0-4 | baseline EXIT=0, mutated EXIT=1 | ✅ PASS | — |
| P1-1 | 重启后可读，磁盘文件 = 4 | ✅ PASS | disk files=4 |
| P1-2 | 依赖销毁时 `/readyz` 断流 | ✅ PASS | /readyz breaks when deps destroyed |
| P1-3 | 错误 `MEMORY_DIR` 不 panic（需人工确认优雅退出） | ✅ PASS（部分） | bad MEMORY_DIR no panic |
| P2-1 | 10 次 429 + Retry-After 重试 | ✅ PASS | 10 429s with Retry-After |

**结论**: 安全启动、鉴权、健康检查、崩溃恢复（不 panic）、限流重试等**基础设施层质量已达标**；问题集中在**任务完成判定、停滞恢复、报告透明度、中文补丁、指标失真**等"任务执行智能"层面。

---

## 五、已修复问题回顾（验证清单）

| ID | 问题 | 修复状态 | 验证测试 |
|----|------|----------|----------|
| RC36 | `classify_user_input()` 缺少问句分类 | ✅ 已修复 (line 376-410) | `test_classify_question_input` |
| RC37 | 压缩墓碑过短 | ⚠️ 部分修复（保留元信息，未增内容） | 需回归测试 |
| RC47 | session_written_files 清空导致假阴性 | ✅ 已修复 (RC52) | `test_rc52_cross_turn_artifact_hydration_on_giveup` |
| RC52 | 统一 give_up 消费端回填 | ✅ 已实现 | 见上 |
| T4 | 语义级停滞检测（含三层恢复） | ✅ 存在且带恢复（loop.rs:2700-2827）；初版"源码已移除"断言已撤销 | `test_t4_semantic_stall_detection` + 恢复 reserve 测试（建议补"无 criteria product 任务"分支） |
| W3 | 完成事实校验 | ✅ 已实现 | `test_w3_unrelated_artifact_rejects_done` |
| Landlock FS_RW | 位掩码 bug | ✅ 已修复 (sandbox/lib.rs:293) | 编译时断言 |

---

## 六、优先级建议（更新，2026-09-02 修正）

### 立即修复（阻塞质量可用）
1. **P0-α**: 扩展 QA 任务完成路径（最小改动，最大 ROI）——已被真实机高频触发。
2. **P0-δ**: 完成报告强制三段式（操作清单 + 事实依据 + 验证指引）——用户知情权。
3. **P0-ζ**: apply_patch 错误格式化改 char 安全切片（patch.rs:127/135/145）——中文环境错误路径必崩。
4. **P0-ε + P1-8**: 收口单一 `should_give_up()` 裁决 + 恢复策略栈——根治分散出口与"已达成却判失败"。

### 短期优化（提升稳定性）
5. **P0-β / P1-6**: 墓碑补 original_goal 关键约束 + introspect 标注水位语义与阈值来源。
6. **P1-5**: 回源码确认 introspect 与 RunReport 步数同源（疑似旧二进制残留，确认后加一致性断言）。
7. **P1-1 / P1-2**: 多类型 progress 计数 + 停滞稳定期。

### 中期规划（架构升级）
8. **P2-1**: 引入 reasoning loop。
9. **P2-2**: experience auto-write。
10. **P2-4 / P2-5**: 网络出网排查 + 沟通闭环机制。
11. **P1-7**: extract_mentioned_files 启发式收敛（去误报）。

---

## 七、测试覆盖缺口（更新，2026-09-02 修正）

| 场景 | 当前覆盖 | 缺口 |
|------|----------|------|
| 问答类任务（无文件写入） | ⚠️ 部分（R1 豁免） | 真实机仍触发 give_up（P0-α 实证） |
| 长上下文压缩（>64KB） | ❌ 无 | 墓碑内容验证缺失（P0-β）；水位语义已核实正确（P1-6） |
| 跨 run 产物继承 | ⚠️ 单测通过 | 真实机 80% 覆盖 |
| 多轮推理（chain-of-thought） | ❌ 无 | 架构不支持 |
| **中文 apply_patch（错误路径）** | ❌ 无 | **P0-ζ 当前源码仍崩**（patch.rs:127/135/145） |
| **步数指标一致性** | ❌ 无 | **P1-5 待确认——疑旧二进制残留，非确证 bug** |
| **写前校验误报** | ❌ 无 | **P1-7 未覆盖（call./done. 误判）** |
| **give_up 出口统一** | ❌ 无 | **P1-8 分散出口无集成测试** |
| **网络出网** | ❌ 无 | **P2-4 全失败** |
| **沟通闭环** | ❌ 无 | **P2-5 流程断链** |

---

## 八、测试记录来源与 BUG 台账交叉引用

**手工测试记录**：
- `release/手工测试v0.2.23.txt`（4,304 行）：能力自检、stalled/give_up 深度诊断、安全矩阵 PASS、完成报告不透明投诉。
- `release/手工测试v0.2.20.txt`（14,458 行）：BUG_LEDGER 沟通闭环分析、apply_patch panic、Cap Sweep 沙箱拦截、联网超时、50 轮记忆压测、步数/水位指标异常。

**BUG_LEDGER.md 关联项**（住户 AI 登记，待消费）：
- BUG-009/010：写入成功但读取失败（压测中初步复现后恢复，最终未系统性复现）。
- BUG-011/013：`hearth resume` core dump（待验证）。
- 13 项总计：5 已确认 / 8 待验证。

---

**结论（2026-09-02 修正版）**：v0.2.22 已解决大部分结构性问题（RC36/RC47/RC52/W3 + 安全矩阵全 PASS），真实机测试揭示的**真阻塞层**：P0-α（完成硬约束，高频 give_up）、P0-δ（报告不透明）、P0-ζ（apply_patch 中文错误路径崩溃）、P0-ε/P1-8（give_up 决策分散多出口 + 恢复不统一）。

**本轮回源码核实修正了初版 4 处事实错误**：
1. `stalled`/T4 检测**当前源码存在且带恢复**（初版"源码已无"错）；
2. `steps_used` 当前源码 `inc_step` 已接（初版"恒 4/50"疑为旧二进制残留）；
3. 水位 40% **正确**（provider-aware ~196k 阈值），非失真；
4. 中文 char-boundary 崩溃点**精确定位到 patch.rs 错误格式化**（压缩代码 context.rs 已修）。

新增 P1-7（写前校验误报）、P1-8（give_up 分散出口）。建议优先修复 **P0-α / P0-δ / P0-ζ / P0-ε+P1-8** 四项，再补 P0-β 墓碑与 P1-5/P1-7。
