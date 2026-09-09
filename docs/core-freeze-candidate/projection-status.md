# FREEZE CANDIDATE · projection-status（先行件）

> **Node 15 八文件之一（先行起草）**｜执行窗 2026-09-02｜基线 tag `v0.2.23`
> **状态**：DRAFT——Step 2/3 数据回填后定版
> **设计公理**：事实产生权在 BE，FE（投影层）只呈现不产生事实；BE 禁返回颜色/HTML。

## 1. 投影层现状清单（代码级锚点）

| 能力 | 实现 | 状态 |
|---|---|---|
| 推理显式显示（用户指令 #3） | `render.rs` ThinkSummary `phase=="reasoning"` 分支「💭 推理（模型自述）」；`loop.rs` reflect 决策依据投影 `emit_think_summary("reflect", Some(&why))`；reasoning_content 投影（`HEARTH_SHOW_REASONING=0` 可关，MAX_REASONING_CHARS=2000） | ✅ v0.2.22 落地；盲测 468 次投影在场 |
| UNVERIFIED 终态标注（C-1/C-12） | `verification_state()` getter + completed 投影「目标达成（未验证）」；`is_verification_command()` 分类器（READ_ONLY 15 项不算验证） | ✅ v0.2.23；盲测 30 轮标注在场 |
| 报告指针 | 每轮终态附 `📄 执行报告: <path>/run-NNN.md` | ✅（既有） |
| 裸 tracing 隔离 | projection_leak 检测器在场（analyzer#1）；v0.2.23 盲测未发现新增裸 ERROR 直达（本轮 ERROR 13 处均为 grep 内容/误贴回显） | ✅ 维持 |
| 截断标记 | 4 处标记 + 2 豁免（Node 13 收口）；宪法 sanitize 截断标记（constitution.rs:79-86） | ✅ |
| ✗ Done 语义混乱 | 「✗ Done (N steps)」组合标记（failed 也打 Done 记号）——**渲染层缺陷，登记未修**（禁改令；RC20 渲染层家族） | ⛔ 开放 |
| PLAN 阶段 stall 无依据投影 | `do_plan_inner` 失败仅裸错误行，无 💭 依据投影（reflect 路径有） | ⛔ 开放（登记） |

## 2. 盲测 v0.2.23 投影层证据（执行窗分析）

- 💭 投影 468 次（plan/act/observe/reflect 四相位均有覆盖）。
- 「目标达成（未验证）」30 轮——含 RC52 三臂路由完成的轮次（完成语义显式标注，对齐 v0.2.18 主诉"我需要猜你的结果"）。
- 需要正式盲测复验的投影观察点：B2（推理可读性主观分）、A6（"需要猜"复验）、A5/A7（诊断投影完整性）。

## 3. Projection 隔离遗留

- tracing→stderr 系统性隔离违约（lib.rs:264-266"装摄像头对着墙"）已在 P4 Node 05 修复（ 见 P4 报告）；本轮盲测未见复发。
- 剩余开放项全部进 open-deviations.md，不带病进 CANDIDATE。
