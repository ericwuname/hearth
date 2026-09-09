# Projection Reality Audit — P4 Node 02（先证明问题，再决定修复）

日期：2026-08-31　执行窗口：砺·执行　基线：v0.2.19

## A. Isolation（终端直写点全库清点）

| 直写点 | 内容 | 定性 |
|---|---|---|
| tracing subscriber → **stderr**（lib.rs:264-266，filter hearth=info,error） | **系统性**——全部 ERROR/WARN tracing 直达终端 | **RC51-A 核心违约**（P0-A 已定位 do_plan_inner 一条；此处为系统性清单确认） |
| loop.rs:2381 `do_plan_inner failed` | T4 stall ERROR | P0-A 已证 |
| scheduler tool dispatch failed（ERROR） | 工具失败 | 同上族 |
| lib.rs:279/292（RC16 弃用警告） | **有意诊断**（P3 新增，env-gated 语义） | 排除（sanctioned） |
| lib.rs:347（RC18 端点警告） | **有意诊断** | 排除（sanctioned） |
| lib.rs:385/387/428（render::error ✗ 红） | 有意 UI（错误投影，走 render） | 排除 |

**结论**：tracing→stderr 为**系统性隔离违约**——非单点。修法方向（Node 05）：subscriber writer 收敛（如写入独立诊断文件/降级 filter），终端侧只保留 render 层输出。

## B. Completeness（internal vs user-visible diff）

- BUG-012 族（盲测 run-010 实证 + 检测器 user_visible_completion 在盲测日志命中 5 处）：内部报告正文（run-XXX.md 数 KB）vs 终端仅 `✓ Done (N steps)` + 路径指针。
- **修法方向（Node 05-B）**：Done 投影追加"结果摘要"（终态事实→用户可理解投影，如产物清单+验证结果+一句结论），**非内部日志倾倒**。

## C. Correctness

- failed≠completed / error≠success：v0.2.19 五态接线 + RC15 被拒→Task failed 复验 ✓（P3 Node 04）。
- give_up≠hidden：give_up/completed 共存 = FALSE POSITIVE（P0 Q-C）但 OVERRIDDEN 标记不进终端 → completeness 残余（并入 B）。

## D. Debug 泄漏

- 盲测 16 行 compacted Debug 外泄（L2810/L2843 同文重复）——源：`format!("{:?}"...)` 于摘要/日志路径。P4 Node 09 复核 compact 路径后并入修复批。

## E. 截断族专项审计（8 处实测，三问：有标记？可感知？可恢复？）

| # | 位置 | 截断 | 有标记 | 可感知 | 可恢复 | 判级 |
|---|---|---|---|---|---|---|
| 1 | constitution.rs:80（MAX_CONSTITUTION_CHARS） | take 无后缀 | **否** | 否 | 否 | **F1 候选**（静默截断——宪法被裁用户不知） |
| 2 | context.rs:431（goal 60 字 + "…"） | take+"…" | 是 | 是（goal_short 每轮可见） | 否（原文在 state.goal） | bounded declared ✓ |
| 3 | loop.rs:1269（final_statement 2000） | take 无标记 | **否** | 部分 | 否 | F1 候选 |
| 4 | loop.rs:1665（tool result 80 字） | take（上下文待核） | 待核 | — | — | 待核（Node 09 并入） |
| 5 | loop.rs:3752（results_text 200） | take 无标记 | **否** | 否 | 否 | F1 候选 |
| 6 | agent-types:565（MAX_INJECTED_CHARS） | take 无标记 | **否** | 否 | 否 | F1 候选 |
| 7 | client.rs:343（msg 160） | take 无标记 | **否** | 部分 | 否 | F2（错误消息场景） |
| 8 | lib.rs:661（40 字） | take 无标记 | **否** | 否 | 否 | F1 候选 |

**静默截断（无标记）6/8**——修复方向（Node 05/13）：统一截断标记规范（`…[N 字已截断]`），先红后绿 fixture。

## 处置

**先证明问题 ✓**（本文件）。修复（Node 05）：仅 A（tracing writer 收敛）与 B（Done 摘要投影）进入 v0.2.20；E 截断标记规范进 Node 13 修复批（v0.2.21）。
