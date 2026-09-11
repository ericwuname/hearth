# P2-LONG-RUN-ACCEPTANCE-01 Final Report v1

**日期**：2026-08-31　**执行窗口**：砺·执行　**基线**：v0.2.16　**终态**：**v0.2.17**（tag 待打于 gate 后；CHANGELOG 已先行）

## 1. Executive Summary

```text
PASS WITH DEVIATIONS
```

- **Structural**：Final Gate **四 RC 全 0，447 passed / 0 failed**（`.133:~/t_gate_lr_final.log`，v0.2.17；443 基线 + 本轮 4 新测试）。
- **Behavioral**：10 项 Long-run 场景全执行；mechanism 层 **10/10 有界正确**（零假完成、零无限循环、零沙箱绕过、零静默丢失）；behavior 层 7/10 completed（3 个 failed 全部层归 model/决策残留，无一条属于 Memory/Context 机制）。
- **Long-run**：Node 08 单跑成链 **compact→stop→resume→continue→complete**（P2-MC Deviation① 正式闭环，S-4）；Node 13 双压缩 stress completed（≥2 压缩 + 修复 + resume）。
- **Independent**：全部产物独立复跑（6 个 cargo test + 3 个 marker 文件），失败层归逐条给锚点。
- 代码改动四项（RC47 修复 / exit code 五态接线 / 40 切片标记 / session 绑定），全部先红后绿，未扩架构（§3 禁区零触碰）。

## 2. Baseline / Provenance

见 `docs/data/long-run-20260831/node00-provenance.md`。要点：双 VM binary **0.2.17 对齐**（.133 曾 0.2.15 脱节，S-1 已修）；.git 重建口径声明（**历史连续性以 CHANGELOG + docs 为准；git 历史自 `463b315` 起可信**，tag→HEAD 链一致已核）；源码备份 `/home/wutao/hearth-v0.2.16-backup-20260831.tar.gz`；df 保险丝全程未触发（最低 36G free）；target symlink 已确认为真实目录。

## 3. Core Decision Audit（Node 01）

Decision/Terminal 映射表（源码实测）：

| Decision | Terminal | 等待用户 | delegation 影响 | 备注 |
|---|---|---|---|---|
| Continue | 继续循环（→Plan/Act） | 否 | — | reflect verdict |
| Replan | needs_decompose → Plan | 否 | — | cap 3 + acceptance_replan 零和 |
| Complete | Done（盲区C 产物校验 + acceptance） | 否 | — | v20 gate：product 需 write_attempted |
| Escalate | InteractionRequested（语义不确定） | 视 channel | **不能消除 semantic uncertainty** | delegation 只解 operational approval |
| Stop/GiveUp | Error（给_up 拦截链前置） | 否 | — | **Stop ≠ GiveUp ≠ Completion** ✓ |

**RC47 审计与修复（批-3 命中）**：give_up 拦截链被 `!edd_checks.is_empty()` 守门跳过——criteria 空 + 已实质完成形态（written_files 非空 + 0 errors）→ 旧路径直接 failed（真机 RC47：续轮重写三文件后 give_up → failed）。**最小修复**（消费端，禁动 planner schema）：该形态路由 Done，由 Done 相位盲区C 做产物确定性校验兜底（INV-LR03 合规）。红（单元复现 Error give_up 形态）→ 绿。修 E 反例细化：criteria 空 + **无产物**（纯 QA）→ failed 保留。三层区分 Stop/Escalate/GiveUp/Completion 语义清晰，无命名混乱。

## 4. Tool Error Closure（Node 02，FA01 OPEN-1/RC46 闭环）

五态接线（照抄 TaskDeadlineExceeded downcast 先例，零文本解析）：

| 态 | 载体 | ToolErrorKind |
|---|---|---|
| exit 0 | Ok | None |
| exit >0 | `BashExitError{exit_code>0}` | `ExitNonZero(code)` |
| signal | `BashExitError{exit_code<0}` | `ExitSignal(code)` |
| tool timeout | `BashExitError{timed_out}` | `ToolTimeout` |
| task deadline | `TaskDeadlineExceeded`（既有） | `DeadlineExceeded` |

测试矩阵（五态 + Unknown 不冒充）+ 集成投影测试全绿；未新增 ToolResult 字段（批-4 纪律②）。

## 5. Memory / Compact Closure（Node 05，批-1 主战场）

**两条路径保护对称性对照表**：

| 维度 | Compaction 路径 | MAX_HISTORY_MSGS=40 切片路径 |
|---|---|---|
| 触发条件 | est ≥ 动态阈值 | 消息数 > 40（每次 build_messages） |
| 修复前标记 | giveup/F9 场景有 | **无** |
| 修复后标记 | —（不变） | **✅ System `[history note]`（N 条被裁告知）** |
| 归档 | ✅ archive 先行 | **无（DEFER：涉存储路径）** |
| 恢复能力 | B 档 grep（模型 0 使用，Node 06） | **零恢复通道**（消息留 state.history 但 LLM 不可见，archive 无切片内容） |

结论：切片路径零恢复能力证实（批-1 预期答案成立）→ 已实施最小改善（打标记）；入 archive 分支 DEFER。

## 6. Archive Isolation（Node 07，P2-MC OPEN-4 闭环）

根因：主路径 `run_local_continue` 从不调 `set_session_id`（仅 rebuild/resume 路径有）→ 首轮压缩归档落共享 `compacted.jsonl`。**一行修复**（run 前 `agent.set_session_id(...)`）。真机验证：Node 08/13 的压缩归档均落 `archive/<sid>.jsonl`（§11）；A↔B 交叉污染检查通过（各 session 归档文件独立）。

## 7. Long-run Task A（Node 09）

textkit 双函数库（normalize_whitespace 逻辑错误拓扑）：**failed（48 步，T4 stall 7-node ×2）**——模型未收敛修复（独立复验 textkit 仍 1 failed），层归 model；mechanism 有界正确（T4 bounded stop，INV-LR04 ✓）。

## 8. Long-run Task B（Node 10）

geoutil 跨文件依赖拓扑（point→polygon 连带失败）：**completed（76 步）**；独立复验 cargo test **1 passed**（sqrt 修复在位）；give_up override 真机触发 1 次（Guard 正确）。

## 9. QA / Discussion（Node 11）

15 轮 REPL（含"继续"/"查看状态"/"你刚才说的是什么"）：**completed，QA 零进 TaskGraph、零 GoalMutation、零 recovery loop**；"继续"轮逐轮记录（砺批-3）：0 give_up / 0 GoalMutation / 0 duplicate（RC47 修复后形态健康）。

## 10. Controlled Failure Recovery（Node 12）

sortlib 边界错误 ×2 独立跑：r1 failed（21 步，T4 stall——**但独立复验 sortlib 1 passed：修复已完成**，failed 发生在修复后的规划停滞，mechanism 有界）；r2 failed（33 步，approval_denied_noninteractive——**模型试图破坏性操作被沙箱策略正确拒绝**，INV：不可绕过 ✓）。两轮均无盲重试（strategy change：诊断→定向修复→复测）。层归：model（规划停滞/审批误触）。

## 11. Multi-compaction Stress（Node 13）

mathnotes 双 bug（add+shout）→ 双失败 → 修复 → 阈值 6000 强制压缩#1 → deadline 中断 → resume → seq 轰炸压缩#2 → 完成：**双跑 completed**；独立复验 **2 passed**、STRESS_ALL_OK ✓；give_up override 真机触发（模型 give_up 被 GIVE_UP_ROUTED/override 正确处置）。压缩计数口径 = archive created_at UTC 窗口（批-6）。

## 12. Reliability Matrix（Node 14）

| 场景 | 跑数 | mechanism | behavior | evidence |
|---|---:|---|---|---|
| Product long-run（A/B） | 2 | 2/2 有界 | 1/2 completed | Task B 产物✓；Task A 未收敛（model） |
| Controlled failure ×2 | 2 | 2/2 分类+有界 | 0/2 completed（修复完成但终态 failed——false stop 2 例） | **独立复验 sortlib 2/2 passed** |
| QA/Discussion 15+ | 1 | 1/1 | 1/1 | 零 TaskGraph 进入 |
| Compact+resume 单跑链 | 1 | 1/1 | 1/1 | chainlib 1 passed + CHAIN_RECOVERED ✓ |
| Multi-compaction stress | 1 | 1/1 | 1/1 | mathnotes 2 passed + STRESS_ALL_OK ✓ |
| Archive C-probe | 1 | 1/1（A+B 证） | 1/1（答案确定正确） | C 未证明（路径=保留上下文，非 grep） |
| InteractionRequest | 1 | （Node 11 内观察） | — | — |
| Approval/denial | 1 | 1/1 拒绝正确 | 0/1 | 沙箱不可绕过 ✓ |
| Deadline | 2 | 2/2 权威 | — | INV-FA01-D 保持 |

**False 指标**：false completion **0** / false giveup **0**（RC47 修复后 give_up 形态正确路由）/ false replan **0** / **false stop 2**（n12 双跑：修复完成后规划停滞终止——mechanism 有界但终态与事实不符，登记为 Node 01 后续改进项）/ duplicate execution：n11 QA 零；product 跑内重执行记录在案 / reteach **0**。

## 13. Metrics

long_run_completion_rate 7/10（70%，层归后 mechanism 100%）｜false_completion 0｜fact_retention 9/10 精确（§6 P2-MC 口径）｜goal_retention 10/10（零 GoalMutation）｜artifact_retention 6/6（独立复验）｜compact_count 8+（archive 归因）｜reteach 0｜deterministic_verification_ratio ≈ 0.9（生产任务验收全部确定性命令）。

## 14. Final Gate + Regression（Node 15）

- **Final Gate**：`.133:~/run_gate_r2c.sh` → `/home/wutao/t_gate_lr_final.log`——四 RC 全 0，**447 passed / 0 failed**（443 基线 + 本轮 4：rc47 / 五态矩阵 / 集成投影 / slice marker）。
- **S-5 机械回归清单**（P2-MC 评审包 §17 六组命令）：①turn 粒度锚点 ✓ ②honest counting ✓ ③provider-aware 三 env ✓ ④compact_pressure_pct 改名+旧名清零 ✓ ⑤INV-M01 双 fixture ✓ ⑥全量 gate ✓——输出已归档 `docs/data/long-run-20260831/`。

## 15. Deviations / OPEN / UNKNOWN / DEFER

| # | 项 | 层 |
|---|---|---|
| 1 | Product 跑 model 未收敛 3 例（n09 未修复 / n12 终态 false stop ×2） | open（model 层） |
| 2 | B 档 archive grep：模型仍 0 主动使用（C 未证明，机制在） | open（措辞："archive preserved but model recoverability unproven"） |
| 3 | MAX_HISTORY_MSGS 切片入 archive | defer（已打标记，入档涉存储） |
| 4 | interaction 8 类样本未全部真机覆盖（分类器单测覆盖） | defer |
| 5 | n12r2 模型误触审批（非交互拒绝） | open（model 层，策略正确） |

## 16. Security

沙箱（RT4 fail-closed）全程在位；审批拒绝正确（n12r2 实证）；egress 白名单无绕过；零 STOP 触发。

## 17. Independent Review Package

本报告 + `docs/data/long-run-20260831/`（provenance/criteria 冻结/全量日志/矩阵 JSON/复验脚本）+ P2-MC 评审包 §17 六组命令输出。评审主体 = 人工独立评审窗口 + 外部 AI 双窗（执行窗口不自宣 Freeze Ready）。

## 18. Final Acceptance Recommendation

**建议：HEARTH CORE FREEZE CANDIDATE（有条件）**——五层结构中 Structural/Behavioral(mechanism)/Long-run/Evidence 四层成立；Independent Review 待人工+外部 AI 双窗完成后进入 Freeze Review。残余 OPEN（B 档模型检索、false stop 2 例、model 收敛方差）不阻塞 Freeze 边界（均为 model/provider 层或已有界），但需在 Freeze Review 中逐条裁定。
