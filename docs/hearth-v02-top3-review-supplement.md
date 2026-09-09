# Hearth v0.2 任务书 · 对标 Top-3 评审补充

> 补 `hearth-v02-taskbook.md` + `hearth-harness-hardening-v013-taskbook.md`。
> 两份任务书方向正确、覆盖完整（R1-R5 快速修复 + WS1-WS6 能力对标）。
> 本补充聚焦：**对标 top3 层面真正缺失、且手工日志有实证**的四个缺口，不重复任务书已有内容。

---

## 补充1：工具集对标——hearth 只有 5 个工具，top3 有 15+

**这是最显著、最被低估的差距。** 手工日志里 agent 从头到尾只用了 5 个工具：

```
glob, grep, read, write_file, bash
```

对照 top3 的核心工具集，hearth 缺了一整层"结构化验证"工具：

| 工具 | 用途 | codex | Claude | Aider | hearth |
|---|---|---|---|---|---|
| `apply_patch` | diff 编辑（WS2 要做） | ✅ | ✅ | ✅ | ❌ |
| `read_lints` | 读结构化编译/语法错误 | ✅ | ✅ | — | ❌ |
| `list_files` | 列目录（替代 ls 原始输出） | ✅ | ✅ | ✅ | ❌ |
| `search` | 语义搜索 | ✅ | ✅ | ✅ | ❌ |
| `web_search`/`web_fetch` | 联网 | ✅ | ✅ | — | ❌ |

**手工日志证据**：贪吃蛇任务 agent 用 `node --check game.js` 验证 JS 语法（返 `exit -1`），内观任务用 `sed -n`、`grep -ril` 手搓搜索——**没有 read_lints，agent 只能靠 bash 原始输出自己 parse 编译错误**。这是 top3 和 hearth 最根本的体验鸿沟：top3 写代码后能精确读到"哪一行、什么错"，hearth 只能看一坨 cargo 输出。

**建议新增 WS（或并入 WS2）**：`read_lints` 工具——跑 cargo/tsc/eslint 后把错误**结构化回喂**（file:line + message + severity），让 loop 直接消费，而非让模型自己 parse 原始 stderr。

---

## 补充2：deepseek prompt 格式适配——从"提一句"升级为"独立 WS"

任务书 §6 自查 #4 只提了一句"deepseek prompt 格式适配"。但**手工日志里 `EOF at line 1 column 0` 是最高频故障**：

```
象棋 plan 阶段 → planner EOF at line 1 column 0（日志 79 行）
贪吃蛇 plan → Failed to parse LLM task plan as JSON: EOF（日志 79 行）
内观任务 → 同样 EOF（日志 840 行）
续接任务 → 同样 EOF（日志 1128/1278 行）
```

**这不是偶发，是 deepseek v4 flash 对当前 JSON 规划 prompt 的结构性不兼容**。四个任务三个撞上，说明 planner 的 JSON 输出约束对 deepseek 失效了——模型不吐 JSON，hearth 就 fallback 单节点，浪费一整轮 20-25s 的规划调用。

**建议**：把"deepseek prompt 适配"从 §6 自查升级为**独立 WS（或并入 WS3 planner 瘦身）**，具体动作：
1. 实测 deepseek 对 JSON 规划 prompt 的响应率（小样本，20 次里几次吐合法 JSON）。
2. 若响应率 < 50%，改 prompt 格式（如用 markdown 结构化而非 JSON、或加 few-shot 示例、或用 tool_calling 强制 schema）。
3. planner 空响应不只是"静默降级"，还要**短时缓存**（同 goal 5s 内不重复打 25s 规划调用）——这在 R2 提了，但要和 prompt 适配合并看。

---

## 补充3：回归保护——任务书通篇没提（锻造九轮最核心教训）

两份任务书（R1-R5 + WS1-WS6）都**没有把"每修一个 bug 挂一条回归测试"作为显式门禁**。

这是 codex-rust 锻造九轮用 20 个版本换来的血泪：**没有断言的修复，等于没修复**——下个版本就无声退化。

**建议**：在 R6 通用门禁和 v0.2 每个 WS 门禁里，强制加一条：

| 修复项 | 回归测试（先红后绿） |
|---|---|
| R1 纯问答 ≤3 步 | `test_chat_qa_no_forced_replan`——20+20 不触发写文件闸 |
| R3 路径放宽 | `test_absolute_path_within_root_allowed`——`/home/wutao/codex_6d` 放行，`/etc/shadow` 拒绝 |
| R4 沙箱验证命令 | `test_node_check_returns_real_exit_code`——不返 -1 |
| WS2 diff 编辑 | `test_diff_applier_no_truncation`——11834 字节完整落盘 |
| WS4 compaction | `test_compaction_preserves_arch_decision` |

**验收口径（复用锻造期 V3）**：新测试必须**先红后绿**——在修复前代码上跑 FAIL，修复后 PASS。只交绿的测试不接受。

---

## 补充4：80-90 分的"测法"没定义——需要一套对标 benchmark

任务书目标"top3 95-100 分，hearth 达 80-90 分"，但**全文没定义怎么测出这个分数**。V2-8 只有"贪吃蛇/象棋/小 web 应用稳定完成"，这不足以支撑"80-90 分等效"的结论。

**建议**：补一个**对标 benchmark 定义**（可放附录或独立文档）：

| 层级 | 任务 | 判据 | 对应能力维度 |
|---|---|---|---|
| L1 问答 | 20+20、自我介绍、概念解释 | ≤3 步干净回答 | 终止 |
| L2 单文件编辑 | 改常量、修 off-by-one、加函数 | 完整落盘 + 测试绿 | 编辑 |
| L3 多文件构建 | 贪吃蛇/象棋（HTML+CSS+JS） | 三文件齐全可运行 | 编辑+验证 |
| L4 仓库级 | 在现有项目改 bug（跨文件） | 定位+改+验证 | 上下文+工具 |
| L5 长会话 | 连续对话 + 续接 | 引用前文 | 续接+compaction |

**每层 N 次里 M 次**（如 L3 贪吃蛇 5 次≥4 全绿）。这是唯一能把"80-90 分"从口号变成可测分数的方式——否则守门员无法独立判"达没达标"。

---

## 补充汇总：对任务书的四处改动

| # | 任务书现状 | 补充定版 |
|---|---|---|
| 1 | 工具集只有 5 个，未提对标 | 补 `read_lints`（结构化编译错误）+ `list_files` + `search` 工具集对标 |
| 2 | deepseek prompt 适配只提一句 | 升级为独立 WS，实测响应率 + 改 prompt + 空响应缓存 |
| 3 | 无回归保护 | 每修一个 bug 挂一条回归测试，先红后绿 |
| 4 | 80-90 分无测法 | 补 L1-L5 对标 benchmark + N/M 判据 |

---

## 净影响

四处补充不改变任务书的骨架（R1-R5 + WS1-WS6 仍然成立），但把"对标 top3"从**方向性口号**变成**可施工、可测、可防回归**的工程目标。

最要命的是补充1——hearth 现在只有 5 个工具，agent 每次验证都要自己 parse 原始 bash 输出。top3 的 `read_lints` 让 agent 精确读到"哪行错了"，这是"能用"和"好用"之间最宽的一条沟。任务书修了终止、预算、路径、diff、compaction，但**没修"agent 看不见编译错误的结构化信息"这个根本体验**。

*"对标 top3"不是把每个 WS 做完就达成的——是让 agent 拥有和 top3 一样的"眼睛"（read_lints）和"手"（apply_patch），而不是用 5 个工具硬凑。*
