# P1-FAILURE-ADAPTATION-01 Final Report

**日期**：2026-08-30　**执行窗口**：砺·执行　**基线**：v0.2.14（`2fe0688`）　**终态**：**v0.2.15**（`236f7a8`，tag v0.2.15）

## 1. Executive Summary

## **PASS WITH DEVIATIONS**

- **能识别**：Failure taxonomy F1-F10 十类纯函数落码（`classify_failure`），每类 ≥1 正例 + 4 组相邻边界反例，Unknown 真实存在。真机实证：exit code 101 受控失败被结构化捕获。
- **能改变策略**：策略矩阵代码化（`failure_strategy`，Retry/Repair/Replan/Verify/Escalate/Stop 六策略），Retry 仅 Transient 一类（INV-FA01-A），单测锁边界。
- **能真正恢复**：**Node 13 wordcount 全闭环**（46 步 ≤52 判据，overhead +4）：受控失败 → 拦截 → Reserve 核验 → `GIVE_UP_OVERRIDDEN` → completed，三 criteria 独立复验全过 + 二进制输出语义正确。**且为非 mathlib 样本**（不过拟合实证，同时充当 Node 15 Case D 回归真机样本）。
- **不无限浪费资源**：反重复上界四层合成（Replan cap 3 / graph stall cap 2 / Reserve ≤1 / budget+deadline 硬上限）单测+真机双重锁定。
- **不破坏既有语义**：Final Gate v0.2.15 = 四 RC 全 0，437/0（基线 429 + 新增 8）。
- **Deviation**：Node 09 / Node 12 真机样本 terminal=failed——层归 model（修复未收敛/未自报 Done）+ environment（Agnes 10 次 backoff、28-55s 慢响应）；mechanism 层全部正确（见 §9/§10）。§26 分层归因条款适用。

## 2. Failure Lifecycle

见 `docs/failure-adaptation-flow.md`（T1 产生 → T2 观察 → T3 计数 → T4 决策 → T5 消费 → T6 验证 → T7 终态，全锚点）。**真机补遗（§4.1）**：GiveUp 实际有 5 条产生路径，本轮盘点并处置：

| # | 路径 | 处置 |
|---|---|---|
| 1 | planner GiveUp 臂 | 修 D 拦截（上轮）✅ |
| 2 | nervous Abandon | 保持观察 |
| 3 | **T4 停滞 Err**（真机红样本发现） | **本轮加拦截**（Reserve 核验后才允许停滞放弃） |
| 4 | **WS9 预算 headless abort**（真机红样本发现） | **本轮加拦截**（核验 passed → 路由 Done，不延长执行预算） |
| 5 | **deadline abort** | **故意不拦截**——Safety>Deadline（INV-FA01-D） |

**criteria 空路径**（砺批-1）：三条可拦截路径全部补 `giveup_unverified` 结构化标记 + 失败报告 `error_detail` 不再折叠为 "loop error"——未核验的放弃必须可审计，且不得伪装成正确终态。

## 3. Failure Taxonomy

`terminal.rs::classify_failure`（纯函数，8 结构化输入）：F1 Transient / F2 ToolFailure / F3 EnvironmentFailure / F4 PermissionFailure / F5 AssertionFailure / F6 VerificationFailure / F7 PlanFailure / F8 ResourceFailure / F9 ModelJudgmentFailure / F10 Unknown。**新结构化通道**：`ToolResult.error_kind`（serde-default，向后兼容）承载 dispatcher `TaskDeadlineExceeded` downcast 投影。**OPEN**：bash exit code 尚未接入 ToolResult（F2/F5 运行时判定受限，loop 层无 exit code 证据时诚实归 Unknown，不冒充）。

## 4. Strategy Matrix

`docs/failure-strategy-matrix.md` + `failure_strategy` 纯函数。边界单测：Retry 仅 F1；F5/F2→Repair；F6/F7→Replan（INV-FA01-F）；F9→Verify（INV-FA01-G）；F4/F3/F10→Escalate；F8→Stop（INV-FA01-E）。

## 5. Progress Semantics

审计结论（`failure-adaptation-flow.md` §6）：verification 步不计 progress 的口径**维持不变**——修 D 拦截已做等价补偿且真机实证；扩展 = 控制流变更 + budget 燃烧节奏漂移，按修 A 先例须顶层批准。6.3 节二阶效应清单（counter 清零→GiveUp 臂漂移→Reserve 零和加剧）已登记为未来启用 checklist。

## 6. Recovery Model

`docs/failure-recovery-model.md`：Reserve 粒度 = run-level 共用（复-3 裁决不翻案）；**新登记** verify_replan_count 跨相位双上限（Act <1 / Done <3）零和实例；Node 07 Case A-F 判定映射表（每 Case 指向既有测试证据）。

## 7. Changes（逐文件）

| 文件 | 变更 |
|---|---|
| `crates/agent-core/src/terminal.rs` | F1-F10 taxonomy + RecoveryStrategy + failure_strategy + 边界测试 |
| `crates/agent-core/src/loop.rs` | 观察级分类消费端（same_tool_repeat/last_error_tool/approval_denied_flag 字段 + scratch telemetry）；F9 giveup_unverified 标记（3 路径）；Error 相位 summary 投影 error_detail；**T4 停滞拦截**；**预算耗尽拦截**（fa01_budget_intercepted 一次性）；F1-F9 fixtures（StallPlanner/LoopReadLlm） |
| `crates/agent-core/src/scheduler.rs` | dispatch Err downcast → error_kind 投影（单/并行两路） |
| `crates/agent-types/src/lib.rs` | ToolErrorKind + ToolResult.error_kind（serde default，不进 LLM 可见内容） |
| `crates/tool-runtime/src/lib.rs` | TaskDeadlineExceeded 重导出 |
| `Cargo.toml` | 0.2.14 → 0.2.15 |
| docs | failure-adaptation-flow.md / failure-strategy-matrix.md / failure-recovery-model.md / data/failure-adaptation-20260830/*（8 个证据文件） |

## 8. Tests

- **先红后绿记录**：Node 09 真机 r1（T4 红样本：事实全完成零核验 failed）→ T4 拦截落码 → fixture 绿；Node 09 r2（预算红样本）→ 预算拦截 → 既有测试全绿。fixture `test_fa01_t4_stall_verification_failed_still_stops` 锁"核验 failed 仍有界停止"。
- 新增 8：taxonomy 十类 + 策略矩阵边界 + F9 两例 + 同工具重复两例 + T4 两例。
- Final Gate（`.133:~/run_gate_r2c.sh` → `/home/wutao/t_gate_fa_final.log`，v0.2.15）：**FMT=0 / CLIPPY=0 / RT4_SOLO=0 / TEST=0，437 passed / 0 failed / 0 ignored**（基线 429 + 8，63 条 test-result 行）。

## 9. Real-machine（两个 long-run 样本）

| 样本 | 终态 | 层归 |
|---|---|---|
| Node 12 mathlib（双受控失败，49 步） | failed（诚实） | mechanism **PASS**：拦截真实触发，拒绝假 completed（模型写 RESULT.txt 标记但 lib.rs E0425——上轮假阳性形态被核验层拦下）；model FAIL |
| Node 13 wordcount（46 步） | **completed ✅** | 全闭环：受控失败→修复→复测→Reserve 核验→GIVE_UP_OVERRIDDEN→completed，三 criteria + 二进制输出独立复验全过 |

Node 09 calcpkg 3 跑：r1/r2 = 两条旁路的红样本来源；r3 = 604s deadline_exceeded（权威终止，INV-FA01-D）。

## 10. Regression

- Final Gate 437/0 覆盖全部历史主线单测（Intent routing / TaskGraph / Readiness / O-4 acceptance / 修 D Case D-G / Deadline / Approval / Sandbox / Resume / Terminal / Projection）。
- Case D 真机样本：Node 13 `GIVE_UP_OVERRIDDEN`（非 mathlib，46 步）✅。
- INV-ED01-A~G 全绿（"本轮只新增不回改"遵守）；INV-FA01-A~H 状态见 `failure-recovery-model.md` / `failure-strategy-matrix.md`（H=QA 负回归真机 PASS，E=Reserve/预算拦截不延长预算）。

## 11. Governance

INV-ED01-*（上轮）与 INV-FA01-*（本轮）前缀已区分；两套单测全绿。新增可执行 invariant：未核验放弃必标记（F9）/ T4、预算旁路核验前置 / deadline 豁免有据。

## 12. OPEN / UNKNOWN / DEFER

| 项 | 等级 |
|---|---|
| bash exit code 未接入 ToolResult → loop 层 F2/F5 判定受限（纯函数层已就绪） | open |
| Node 09/12 真机未收敛到 completed（model 未自报 Done + 修复不完整；Agnes 高延迟） | open（层归已记录） |
| progress 语义扩展候选（四类 progress 模型） | defer（须顶层批准，§5 二阶效应清单在案） |
| verify_replan_count 跨相位双上限统一 | defer（控制流变更，预案在 Node 05 §2） |
| 修 A 常量标定（BUDGET_LOW_THRESHOLD 等） | defer（备而未用，复-3 裁决 2） |
| Node 11 Resource/Failure 交互：deadline 权威真机实证（604s abort），Safety>Recovery 顺序未发现违例；大输出×低预算组合未单独压测 | open（低风险） |

## 13. Provenance

| VM | source | version | binary | gate log |
|---|---|---|---|---|
| .133（评审） | `/home/wutao/codex_t`（tar 同步树，无 .git） | 0.2.15 | `/usr/local/bin/hearth` = 0.2.15（本轮重建安装） | `/home/wutao/t_gate_fa_final.log`（v0.2.15，437/0，四 RC=0）；基线 `/home/wutao/t_gate_fa_baseline.log`（429/0） |
| .131（执行） | `/home/wutao/codex`（tar 同步树，无 .git） | 0.2.15 | `/usr/local/bin/hearth` = 0.2.15（本轮重建安装） | —（真机日志 `~/fa/node09_r3.log` / `node12.log` / `node13.log` / `node14.log`，已归档 `docs/data/failure-adaptation-20260830/`） |
| 本机 | HEAD `236f7a8`，tag **v0.2.15**；提交链 eb77bbc → 81d39d4 → 236f7a8 | 0.2.15 | — | — |

**criteria 冻结全文**（批-5）：`docs/data/failure-adaptation-20260830/node09-12-13-criteria-frozen.md`（run 前冻结，未修改）。

---

## 14. Addendum（v1.1，2026-08-30 复测补录）

Node 12 按执行协议"retest"复跑 2 次（脚本/criteria 冻结不变，Agnes 延迟恢复后错峰执行）：

| 跑 | 终态 | 关键证据 |
|---|---|---|
| r1（原报） | failed 49 步 | 拦截拒绝假 completed（见 §9） |
| r2 | failed 50 步（budget_exhausted） | **拦截再触发**：acceptance cmd 失败详情捕获完整 E0425 编译诊断（测试模块缺 `use super::*`）+ read_lints 结构化提取 2 条——acceptance 层成为真实诊断通道；Reserve 已尽 → 预算拦截正确拒绝二次核验（INV-FA01-E 零和语义） |
| **r3** | **completed（21 步，tokens 24965）** | 拦截 → Reserve 核验 passed → `GIVE_UP_OVERRIDDEN`（Case D）。独立复验：`add`=a+b（bug 已修）、`shout`=to_uppercase+感叹号（**上轮 shout OPEN 行为级闭环**）、`use super::*` 在位、**cargo test 2 passed/0 failed**、RESULT.txt=LONGRUN_RECOVERY_PASSED——三 criteria 全部独立通过 |

**判定影响**：
- §25.A-E 现全部有真机实证（C"至少一个闭环"现为**两个**：Node 12 r3 + Node 13）；
- 步数 21 ≤ 52 判据（overhead 相对 42 基线为 **-21**，无惩罚项）；
- 偏差收敛至：Node 09 单样本 model 层未收敛（deadline 权威终止，机制正确）+ §12 OPEN 工程项；
- **总判定维持 PASS WITH DEVIATIONS**（不自行升级为 PASS——剩余偏差与 OPEN 项由砺·评审/顶层裁定；本附录只登记证据）。
