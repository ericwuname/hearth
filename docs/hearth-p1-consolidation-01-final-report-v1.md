# Hearth P1-CONSOLIDATION-01 Final Report v1（2026-08-30）

## 1. Executive Summary

# **PASS WITH DEVIATIONS**

收敛达成：**Sandbox/Seccomp/Landlock（Node 01）、Task Routing（Node 02）、Verify（Node 04）、Projection（Node 05）四域 CLOSED + DEV-2 部分收敛（observe 标记）+ Deadline/Resume/Context/RC24/W8/P1-LTR 全量回归无回归**（收口合计六项：O-3、Q-3、OPEN-W8-1、verify_failed、N-3 + FCHDIR 安全修正）；唯一偏差 = Node 07 长程任务**任务完成度**（机制链 PASS、模型执行链未闭环——Agnes flash 多步编辑能力边界，如实归属 DEV-2 家族，详见 §5）。

## 2. Commit Graph（Node → commit）

```text
Node 00 Baseline Lock      → 61667dc（gate 补票四 RC 全 0 + /tmp 证据迁移 8 文件）
Node 01 G0 Capability      → 7b53800（O-3 加白 + FCHDIR 错位安全修复 + Q-3 结论）
Node 02+03 Routing/DEV-2   → 2c0c1b9（Intent×Constraint 分离 + QA 跳过 decompose）
Node 03+05 Conflict/N-3    → 6d1d66a（REFLECT_FACT_CONFLICT + 审批委托投影）
Node 04 verify_failed      → 117cf02（端到端 fixture）
v0.2.12 资产               → a907fcc（bump + CHANGELOG + ledger 回填）
lint 收敛                  → 7ad56cc（HEAD）
tag: v0.2.12（→ 7ad56cc）
```

## 3. Implementation

| 修改点 | 文件 | 内容 |
|---|---|---|
| seccomp 加白+修正 | sandbox/lib.rs | `SYS_MKDIRAT=258`（probe 实证）、`SYS_UMASK=95`（strace 差集）、**`SYS_FCHDIR 133→81`（ausyscall 反查：133=x86_64 mknod 误放行——安全级发现）**；数组 121→123 |
| Intent×Constraint 分离 | agent-core/loop.rs | `goal_requires_product` 负向短语剥离→residual 判 Intent（禁关键词堆叠——Constraint 分离机制） |
| QA 跳过 decompose | agent-core/loop.rs | QA/论述任务跳过 planner decompose（T4 stall 根除，终态语义零变化——Node 03 最小诊断修复） |
| REFLECT_FACT_CONFLICT | agent-core/loop.rs | give_up 时事实进展冲突标记（tracing + summary.reflect_fact_conflict；修 4 落法，observe-only） |
| N-3 投影 | codex-cli/report.rs + run_local.rs | `RunReportInput.approval_delegated` + 「## 审批委托」节（四象限） |
| verify_failed fixture | agent-core/loop.rs（test） | 写→空覆盖→DONE×3→replan≤3→verify_failed（零 production 改动） |

## 4. Test Evidence

- **先红后绿**：Node 02 A-G 矩阵（旧负向短路下 C/F/T3 红）；Node 01 位运算/probe 红证据。
- **unit**：agent-core 95（+node02 A-G +node04）；tool-runtime 21（P1-LTR 继承）。
- **VM gate（四 RC 全 0，增 6 路径登记）**：
  - Node 00：`.133:/home/wutao/t_gate_consol_node00.log`（411/0 补票——修 1）
  - Node 01：`.133:/home/wutao/t_gate_consol_node01.log`（四 RC 0）
  - Node 02：`.133:/home/wutao/t_gate_consol_node02.log`（**417 passed / 0 failed / 1 ignored**，ignored=既有 RT4 人工项）
  - Final（修 5 覆盖最终态）：`.133:/home/wutao/t_gate_consol_final.log`（**417/0/1 四 RC 全 0**）
- **real-machine**：Node 01 三测（mkdir OK/mknod 159 拒/读写 OK）+ Node 02 五 cases（hash 保全）+ A/T3 修复复验。

## 5. Long-run（Node 07，如偏差实报）

- **Product 长程**（建 mathlib→受控失败→修复→复测→RESULT，budget 30/50 两轮）：29 步 / 42 步均 `failed`——**受控失败→修复→复测链条断**（修复实际成功但复测时序混乱；planner budget-low give_up）。机制链全程无故障（规划/工具/写盘/审批/deadline/telemetry）；**假完成样本**：RESULT.txt 写 TEST_PASSED 而实际 test FAILED（**O-4 候选**：语义级 completion_fact_check 适用性研究）。
- **Discussion**（10+ turn REPL，Rust 主题 10 问）：两轮 56/63 hits ✅——零 TaskGraph 循环（W8 回归）。

## 6. Regression

R2-C / W3/W4 / RC24（TC-8/9 测试原样绿）/ W8（A-G + 十论述回归）/ P1-LTR（deadline 测试原样绿）/ Sandbox（N1-SBX 三测 6.3 复核）——全量 gate 417/0 覆盖，零回归。

## 7. Security

- seccomp：+2 加白（mkdirat/umask，证据驱动最小集）+ **1 收紧**（FCHDIR 错位修复关闭 mknod 误放行）——净效果**能力收敛**。
- landlock / sandbox / approval：零改动。

## 8. OPEN / UNKNOWN / DEFER

- **OPEN**：DEV-2 model judgment 残余（REFLECT_FACT_CONFLICT 持续观察）；**O-4 候选**（长程任务自述成功与事实不符→语义级验证研究）；Q-3 kernel≥5 新位评估（另立）。
- **UNKNOWN**：无。
- **DEFER**：Bridge / Subagent / TUI / MCP（维持）。
- 观察记录：telemetry `phase` 已分桶（main/aux），细分 role 延后（Node 08 审计结论）。

## 9. VM Provenance（系统 PATH 口径——修 5）

| host | source path（**分窗登记**，vm-version-sync.md 2026-08-30 修订） | HEAD | binary path | version | gate log |
|---|---|---|---|---|---|
| .133（评审 VM） | ~/codex_t | 7ad56cc | /usr/local/bin/hearth | **0.2.12** | t_gate_consol_final.log |
| .131（执行 VM） | **~/codex**（执行窗口自有施工树——分窗登记；非 ~/codex_t） | 同源 tarball（git archive） | /usr/local/bin/hearth | **0.2.12** | （build 实证 55s） |

> 勘误（顶层 §五 + 守门员裁决 1）：初版".131 source = ~/codex 与 W1 规程冲突"的表述**撤回**——~/codex 禁写是 .133 侧规则被错误泛化；.131 的 ~/codex 为执行窗口自有施工树（0.2.12 binary 构建源），按 **per-host 分窗登记**口径为准。.131 的 ~/codex_t（0.2.9 陈旧树）标 DEPRECATED 防误用。

## 10. Final Recommendation

# **PASS WITH DEVIATIONS**

（偏差 = Node 07 长程任务完成度，机制验收全过；O-4 立项建议已列。）本轮停止，等顶层终验。
