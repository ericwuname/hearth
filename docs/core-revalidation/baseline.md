# Node 00 — Baseline Lock & Freeze Status Reset（P4-REVALIDATION-01）

日期：2026-08-31　执行窗口：砺·执行

## Provenance 三查（+sha256）

| VM | source | binary（系统 PATH） | sha256（前 16） |
|---|---|---|---|
| .133（实验主机） | codex_t = v0.2.19 | **0.2.19 ✓**（18:54 顶层发现滞后后重装对齐） | `8ec501033c2a48ee` |
| .131（辅助） | codex = v0.2.19 | **0.2.19 ✓** | `730e17de4c7d3402` |
| 本机 | HEAD = P3 后续提交链 | — | — |

gate 基线 = **455**（t_gate_p3_final.log，四 RC=0）。磁盘 .133 = 54G free。

## 工程状态正式改写

**`CORE FREEZE SUSPENDED / CORE REVALIDATION IN PROGRESS`**
—— CLOSURE-01 的 CORE FREEZE 结论自 v0.2.18 盲测（28.6%、RC52 吸引子）起 SUSPENDED；
旧 Freeze 结论（freeze-decision.md）降为历史轨迹，**不得再被引用为有效验收结论**。
P4 结束时只产出 **CORE FREEZE CANDIDATE**（Node 15），最终裁决 = 人工+外部 AI 双窗。

## P3 复验剩余精确清单（4+1）→ decision-status.md

| 项 | 状态 |
|---|---|
| RC33 长会话 revision 对照 | NEEDS-RERUN |
| 复测包（A-5·A-36 连续性 / 12·18 跑矩阵） | NEEDS-RERUN |
| RC34 终端级快照 | NEEDS-RERUN（单元级 ✓） |
| RC13 service 会话真机 | NEEDS-RERUN（代码 ✓） |
| RC29 trust on | **NOT-IMPLEMENTED**（CLI 无 trust 命令 → ledger 修正项） |

## 状态索引（已读）

ledger v3 / P0 report / P3 report / SimUser v2.0（规格附录）/ 砺顶层总览——要点已并入本包 §0，P4 执行以本包为准。

## 观测仪器衔接

`HEARTH_DEBUG_PLANNER_INPUT=1` dump（prompt/graph/compact）已在树（P0 遗产）——Node 03 直接开启，不重造 ✓。
