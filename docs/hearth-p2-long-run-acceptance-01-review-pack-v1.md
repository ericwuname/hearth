# Hearth P2-LONG-RUN-ACCEPTANCE-01 评审包 v1（整合版）

> **用途**：单文件自足评审材料——供砺·评审窗口与外部 AI（ChatGPT）交叉评审；同时作为 **Core Freeze Review** 的 Evidence 输入。
> **执行窗口**：砺·执行　**日期**：2026-08-31　**代码终态**：**v0.2.17**（tag v0.2.17，HEAD `2aa32da` 含 docs；代码基线 `aee97aa`）
> **结论**：**PASS WITH DEVIATIONS**（偏差与 OPEN 见 §13；是否进入 CORE FREEZE CANDIDATE 由评审/顶层裁定——执行窗口不自宣 Freeze Ready）
> **核验纪律**：所有锚点均经 grep/实测复核；§15 给出评审方可独立执行的核验命令清单。禁止采信 commit message 与"已验证"自述。

---

## 1. Executive Summary

**使命**：不再堆叠能力——证明已完成的机制（Intent/Plan/Execution/Fact/Verification/Decision/Recovery/Memory/Resume）组合后在长程真实任务中稳定协作。

- **Structural**：Final Gate v0.2.17 四 RC 全 0，**447 passed / 0 failed**（`.133:~/t_gate_lr_final.log`；443 基线 + 本轮 4）。
- **Behavioral（mechanism 层）**：10 场景 **10/10 有界正确**——零假完成、零无限循环、零沙箱绕过、零静默丢失、零 GoalMutation。
- **Behavioral（behavior 层）**：7/10 completed——3 个 failed 全部层归 model（规划停滞 ×2 / 未收敛修复 ×1），机制层有界且产物部分有效。
- **Long-run**：Node 08 **单跑成链 compact→stop→resume→continue→complete**（P2-MC Deviation① 正式闭环，S-4）；Node 13 双压缩 stress completed（≥2 压缩 + 修复 + resume）。
- **代码改动四项**（全部先红后绿，零扩架构）：RC47 最小修复 / exit code 五态接线（RC46 闭环）/ 40 切片标记 / chat 主路径 session 绑定。

**Deviation**：① B 档（archive grep）模型仍 0 主动使用——C 未证明但机制在（批-2 三档口径）；② Product 跑 false stop 2 例（n12：修复完成后规划停滞终止，mechanism 有界但终态与事实不符）；③ interaction 8 类样本未全部真机覆盖（分类器单测锁定）。

## 2. Node 00 — Provenance（S-1 落实）

- **双 VM binary 对齐**：.133 曾 0.2.15 脱节（守门员实测）→ 重建+安装 **0.2.16→0.2.17**；.131 = 0.2.17 ✓。
- **.git 重建口径声明**（砺批-7）：**历史连续性以 CHANGELOG + docs 为准；git 历史自 `463b315` 起可信**；tag v0.2.16/v0.2.17 在库、HEAD 链一致。
- **源码备份**：`/home/wutao/hearth-v0.2.16-backup-20260831.tar.gz`。
- **df 保险丝**（<10G 中止）全程未触发（最低 36G）；target symlink 检查：.133 已重建为真实目录（P2-MC 事故闭环）、.131 独立缓存。

## 3. Node 01 — Core Decision/Terminal 映射 + RC47 修复

映射表：Continue→循环；Replan→needs_decompose（cap 3）；Complete→Done（盲区C 产物校验 + acceptance）；Escalate→InteractionRequested（**delegation 不能消除 semantic uncertainty**）；Stop/GiveUp→Error（拦截链前置）。三层区分 Stop/Escalate/GiveUp/Completion 清晰无混乱。

**RC47（批-3 命中，活样本闭环）**：give_up 拦截链被 `!edd_checks.is_empty()` 守门跳过——criteria 空 + 已实质完成形态（written_files 非空 + 0 errors）→ 旧路径直接 failed（真机：续轮重写三文件后 give_up → failed）。**最小修复**（消费端，禁动 planner schema）：该形态路由 Done，由 Done 相位盲区C 做产物确定性校验兜底（INV-LR03）。红（单元复现 Error）→ 绿。修 E 反例细化：criteria 空 + **无产物**（纯 QA）→ failed 保留（QA 不被路由 completed）。

## 4. Node 02 — bash exit code 五态接线（FA01 OPEN-1/RC46 闭环）

| 态 | 载体 | ToolErrorKind |
|---|---|---|
| exit 0 | Ok | None |
| exit >0 | `BashExitError{exit_code>0}` | `ExitNonZero(code)` |
| signal | `BashExitError{exit_code<0}` | `ExitSignal(code)` |
| tool timeout | `BashExitError{timed_out}` | `ToolTimeout` |
| task deadline | `TaskDeadlineExceeded`（既有先例） | `DeadlineExceeded` |

- `BashExitError`（tool-runtime 新类型，Display = 原 formatted，LLM 可见内容不变）；scheduler 单/并行两路统一 `classify_dispatch_error`。
- **勿新增 ToolResult 字段**（批-4 纪律②）——仅扩展 ToolErrorKind 枚举（serde 兼容）。
- 测试：五态矩阵 + Unknown 不冒充（FA01 基线保持）+ 集成投影（真实 bash "exit 3" → ExitNonZero(3)）。

## 5. Node 03 — Interaction 表面审计

分类器（classify_user_input）TaskControl/寒暄/终态投影行类别 + 修 B 疑问句已覆盖历史切句形态；interaction 触发正确性由 W8 fixtures 锁定。8 类样本中 6 类有单测/真机覆盖，纯情绪/长混合 2 类仅单测——**测试补全 DEFER**（无真实 bug 证据）。

## 6. Node 04 — Progress 语义审计（批-5 衔接 FA01 §5 裁决）

| Case | 现有表现 | 锁定测试 | 结论 |
|---|---|---|---|
| A write→10 次验证 | write 重置 steps_without_progress | W3 fixture | 维持 |
| B test fail→repair→pass | consecutive_errors + acceptance 回喂 | FA01 fixtures | 维持 |
| C read-only 探索 | search-streak + T4 bounded stop | FA01 T4 fixtures | 维持 |

**无假停滞真根因证据**（真机 n12 false stop 为 T4/规划停滞，非 progress 计数）→ **口径维持不改**（顶层裁决）。二阶效应三条逐条对照：①拦截回喂步已计入 progress（真机观察无漂移事故）②零和竞争已由 Reserve ≤1 约束 ③graph_stall 无交互（已排除）。

## 7. Node 05 — 40 消息切片：两条路径保护对称性（批-1 主战场）

| 维度 | Compaction 路径 | MAX_HISTORY_MSGS=40 切片路径 |
|---|---|---|
| 触发条件 | est ≥ 动态阈值（provider-aware） | 消息数 > 40（每次 build_messages） |
| 修复前标记 | F9/giveup 场景有 | **无** |
| 修复后标记 | — | **✅ System `[history note]`（N 条被裁如实告知）** |
| 归档 | ✅ archive 先行 | **无（DEFER）** |
| 恢复能力 | B 档 grep | **零恢复通道**（消息留 state.history 但 LLM 不可见，archive 无切片内容——批-1 预期答案证实） |

最小改善（"打标记"分支）已落地 + `test_history_slice_marker_present` 锁定；入 archive 分支 DEFER（涉存储路径，不扩架构）。

## 8. Node 06 — Archive Recoverability（C 级判定，批-2 口径预写）

判定口径（开工前冻结）：确定性判定——压缩 FACT（secret.txt=CODE-7788）后询问，回复做 substring 比对，不采信自述。

**结果：C 未证明但机制在**（三档中的中间档）：
- A（archive 有记录）✅：compacted.jsonl 真实落盘增长（934KB→1187KB 实测）；
- B（系统知道 archive 存在）✅：检索提示注入（fixture 锁定）；
- C（模型实际找得到）：**答案确定性正确（CODE-7788 在回复中）但恢复路径=保留上下文而非 archive grep**（cat secret=0 / read_file=0 / archive grep=0）→ 措辞登记 "**archive preserved but model recoverability unproven**"。禁 A+B 冒充 M5。

## 9. Node 07 — Session Archive Isolation（P2-MC OPEN-4 闭环）

根因：主路径 `run_local_continue` 从不调 `set_session_id`（仅 rebuild/resume 路径有）→ 首轮压缩归档落共享 `compacted.jsonl`。**一行修复**（run 前 `agent.set_session_id(...)`）。真机验证：Node 08/13 的压缩归档均落 `archive/<sid>.jsonl`，A↔B 交叉污染检查通过。

## 10. Node 08 — Compact + Resume 单跑成链（S-4：P2-MC Deviation① 正式闭环）

chainlib 受控 bug（dbl 乘 3）→ 测试失败 → 修复（乘 2）→ 复测 → 阈值 8000 强制压缩 → deadline 中断（10 步）→ **resume（空 goal）→ completed（16 步）**。独立复验：chainlib cargo test **1 passed**、CHAIN_RECOVERED ✓。**re-teach=0 / 无重复破坏性工作 / 无 goal mutation / 无假完成**——单跑链完整，Final Report 显式声明闭环 P2-MC 偏差之首。

## 11. Node 09/10 — Long-run Product Task A/B

| | Task A textkit | Task B geoutil |
|---|---|---|
| 拓扑 | 单文件双函数（逻辑错误） | 跨文件依赖（point→polygon 连带失败） |
| 终态 | failed（48 步，T4 stall 7-node ×2） | **completed（76 步，override 真机触发 1 次）** |
| 独立复验 | 仍 1 failed（模型未收敛） | **1 passed**（sqrt 修复在位） |
| 层归 | model（未收敛） | mechanism ✓ |

## 12. Node 11 — QA 15 轮（砺批-3 逐轮记录）

16 输入（含"继续"/"查看状态"/"你刚才说的是什么"/反例/修正）：**completed；QA 零进 TaskGraph、零 GoalMutation、零 recovery loop；"继续"轮三件事全零**（0 give_up / 0 GoalMutation / 0 duplicate——RC47 修复后形态健康）。

## 13. Node 12/13 — Controlled Failure ×2 与 Cross-compaction Stress

- **Node 12** sortlib 边界错误 ×2：r1 failed（21 步 T4 stall——**独立复验 sortlib 1 passed：修复已完成**，failed 发生在修复后的规划停滞）/ r2 failed（33 步 approval_denied——模型误触破坏性操作被沙箱正确拒绝）。两轮均诊断→定向修复→复测，零盲重试。**false stop 2 例登记**。
- **Node 13** mathnotes 双 bug → 修复 → 阈值 6000 压缩#1 → deadline 中断 → resume → 压缩#2 → 完成：**双跑 completed**；独立复验 **2 passed**、STRESS_ALL_OK ✓；give_up override 真机触发（路由正确）。压缩计数 = archive created_at UTC 窗口（批-6，禁日志 grep）。

## 14. Node 14 — Reliability Matrix（三层成功口径）

| 场景 | 跑数 | mechanism | behavior | evidence |
|---|---:|---|---|---|
| Product long-run | 2 | 2/2 有界 | 1/2 | Task B 1 passed ✓；Task A 未收敛（model） |
| Controlled failure ×2 | 2 | 2/2 | 0/2（false stop） | **sortlib 2/2 passed（独立）** |
| QA 15+ | 1 | 1/1 | 1/1 | 零 TaskGraph 进入 |
| Compact+resume 单跑链 | 1 | 1/1 | 1/1 | chainlib 1 passed ✓ |
| Multi-compaction | 1 | 1/1 | 1/1 | mathnotes 2 passed ✓ |
| Archive C-probe | 1 | 1/1（A+B） | 1/1（答案正确） | C 未证明 |
| Approval/denial | 1 | 1/1 拒绝正确 | 0/1 | 沙箱不可绕过 ✓ |
| Deadline | 2 | 2/2 权威 | — | INV-FA01-D 保持 |

**False 指标**：false completion **0** / false giveup **0**（RC47 路由正确）/ false replan **0** / **false stop 2**（登记）/ duplicate execution：n11 零、product 跑内重执行记录在案 / reteach **0**。

## 15. OPEN / UNKNOWN / DEFER

| # | 项 | 层 |
|---|---|---|
| 1 | B 档 archive grep 模型 0 主动使用 | open（"archive preserved but model recoverability unproven"） |
| 2 | false stop ×2（修复完成后规划停滞终止） | open（Node 01 后续：规划停滞消费端复核） |
| 3 | Product 跑 model 未收敛方差（n09） | open（model） |
| 4 | MAX_HISTORY_MSGS 切片入 archive | defer（已打标记） |
| 5 | interaction 8 类样本真机补全 | defer |
| 6 | n12r2 模型误触审批 | open（model 层，策略正确） |

## 16. Security

沙箱 RT4 fail-closed 全程在位；审批拒绝正确（n12r2 实证）；egress 白名单无绕过；**零 STOP-1~7 触发**；§3 禁区零触碰（无新 TaskGraph/Memory Model/Verification Authority/Subagents 等）。

## 17. 评审核验锚点与命令清单

`.133:~/codex_t`：

```bash
# 1) RC47 修复
grep -n "GIVE_UP_ROUTED_TO_DONE" crates/agent-core/src/loop.rs
grep -n "test_rc47_artifacts_route_done_on_empty_criteria_giveup" crates/agent-core/src/loop.rs
# 2) exit code 五态
grep -n "BashExitError" crates/tool-runtime/src/dispatcher.rs crates/agent-core/src/scheduler.rs crates/tools-builtin/src/bash.rs
grep -n "ExitNonZero\|ExitSignal\|ToolTimeout" crates/agent-types/src/lib.rs
# 3) 切片标记
grep -n "history-slice-note\|history note" crates/agent-core/src/loop.rs
# 4) session 绑定
grep -n "set_session_id" crates/codex-cli/src/run_local.rs
# 5) 测试
cargo test -p agent-core --lib test_rc47                 # 1 passed
cargo test -p agent-core --lib test_five_state           # 1 passed
cargo test -p agent-core --lib test_bash_exit_code       # 1 passed
cargo test -p agent-core --lib test_history_slice_marker # 1 passed
bash ~/run_gate_r2c.sh                                   # 四 RC=0, 447/0
```

`.131` 真机日志（已归档本仓库 `docs/data/long-run-20260831/` + `~/fa/lr_*.log`）：

```bash
grep -aoE "Task (completed|failed)" ~/fa/lr_n10.log ~/fa/lr_n13a.log ~/fa/lr_n13b.log
cat /tmp/lr_n08/CHAIN_RESULT.txt /tmp/lr_n13/STRESS_RESULT.txt /tmp/lr_n06/secret.txt
cargo test --manifest-path /tmp/lr_n13/mathnotes/Cargo.toml   # 2 passed（独立复验）
python3 /home/wutao/fa/collect_lr.py                           # Reliability Matrix 重算
```

## 18. 防伪声明

本包所有"completed"均可独立复跑验证（§17 命令）；所有"failed"如实层归（model/decision/mechanism 三层）；"压缩发生"以 archive 落盘为权威信号；clean 样本与"重跑混入"样本分开列示；false stop ×2 主动披露；无一次成功外推为"全部可靠"。
