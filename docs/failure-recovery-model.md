# Recovery State Model — Node 05（P1-FAILURE-ADAPTATION-01）

日期：2026-08-30　依据：Node 01 源码审计。**不改粒度（复-3 裁决 2：Reserve 维持 run-level 共用）**。

## 1. Reserve 粒度显式登记（v1.1 必答，砺批-2）

**裁决：run-level 共用，本轮不翻案。** 源码事实（v0.2.14 实测锚点）：

```text
acceptance_replan_count   loop.rs:935
  ├─ 写点 1：GiveUp 拦截 Reserve（:3972-3976，<1）
  ├─ 写点 2：Done 相位 Reserve（:4456-4458，<1）
  ├─ 清零点：无（仅跨 run 重建时随结构体重置）
  └─ 语义：单 run 单 Reserve（≤1 次），拦截与 Done 核验跨相位零和

verify_replan_count       loop.rs:953
  ├─ 写点 1：Act done-gate 产物缺失回喂（:2646-2670，<1）
  ├─ 写点 2：Done 相位 written_files 缺失回喂（:4402-4403，<3）
  ├─ 清零点：fresh run（:4199）
  └─ 语义：⚠️ 同一计数器、两个不同上限（<1 与 <3）——Act 先消耗则 Done 只剩
     2 次名额。有界性成立；粒度 = 跨 Act/Done 共用（与 Reserve 同构的零和，
     此前未登记，Node 01 已补入污染表）。

graph_stall_count         loop.rs:963
  ├─ 写点：do_plan 同图停滞（:2494，cap 2 → Err）
  ├─ 清零点：图变化（:2507）、v20 gate 重入（:3706）
  └─ 语义：独立于前两者，无共用。
```

## 2. Node 07 若调整粒度的影响分析（预案，非本轮动作）

- **acceptance_replan_count 拆分（phase-level）**：GiveUp 拦截与 Done 核验各 ≤1 →
  最坏恢复步数 +1；效果 = Done 相位核验失败不再直接 verify_failed（拦截先耗过
  也有第二次机会）。代价 = 每任务最多多 1 次核验+修复回路（步数 +2~4）。
  连带：**无对 verify_replan_count / graph_stall_count 的影响**（二者独立）。
  若启用，须修 D 同级顶层批准 + Case D fixture 先红后绿回归。
- **verify_replan_count 统一上限**：Act/Done 共用一个 <N——影响 = Done 名额被
  Act 预支的零和消失。低风险，但仍是控制流变更，不在本轮边界内。

## 3. Recovery 状态五问（用既有模型回答，不新建第四套）

| 问 | 答 |
|---|---|
| 一次 failure attempt 是什么 | 一次 Reflect 观察（pending_results）中出现 is_error 事实的 Act 步 |
| 什么时候算 recovery attempt | failure 之后任何改变恢复态势的动作：Repair（修复+复测）、Replan（重分解）、Reserve 核验。计数载体 = 既有计数器（§1），**不新增 recovery_attempts 字段**（总包："必须先证明必要"——既有计数器已承载，新增=平行事实源） |
| 什么时候算 recovery 成功 | 仅三层事实：L1 cmd exit 0（acceptance cmd 核验）/ L2 产物在场非空 / L3 verify_acceptance_criteria passed → acceptance_result="passed"。LLM 自报无通道（INV-FA01-G） |
| 什么时候算重复策略 | same_tool_repeat≥2（F7 判据）+ graph_stall_count≥2（同图重分解）——两个既有信号即"same failure + same state + same strategy"的确定性检测器 |
| 什么时候必须换策略 | F7 命中（同工具反复）→ planner Replan 臂换分解；graph 停滞 → 强制 GiveUp（终态兜底）；Reserve 尽 → verify_failed |

## 4. 反重复原则（INV-FA01-C）落点

`same failure + same state + same strategy` 的循环上界由四层既有机制合成：
Replan cap 3（planner）→ graph_stall cap 2（do_plan Err）→ Reserve ≤1（run-level）
→ budget/deadline 硬上限（WS9 ask 边界 + dispatcher deadline）。**任何一层都不
提供无界恢复**；Node 08 F5/F7 fixture 锁定该上界。

## 5. Node 07 — Budget-aware Recovery（Case A-F 判定映射）

不提高全局 budget cap；deadline 保持权威（dispatcher 单点 `effective=min(declared,remaining)`）。
各 Case 在现有源码中的确定性落点与证明（测试）：

| Case | 场景 | 系统行为（源码落点） | 证明 |
|---|---|---|---|
| A | budget plenty + transient | provider 层有界重试；loop 不盲试（test_provider_transient_retry_capped / test_t2_transient_retry_capped） | 既有测试 ✅ |
| B | budget plenty + test failure | F5→Repair：错误回喂修复（:4008/:4468 回喂通道），Reserve 未耗则修复后复测 | test_node05_case_g / test_node04_verify_failed_e2e ✅ |
| C | budget low + transient | planner budget-low GiveUp 臂只在 `noprogress≥2` 时触发；transient（无错步清零 consecutive_errors 且不累积 noprogress）不误触 | test_execdec_node01_budgetlow_giveup_ignores_acceptance 反例臂 ✅ |
| D | budget low + repair possible | 修 D 拦截：GiveUp 前花 Reserve（≤1）核验→passed 则 completed / failed 则回喂修复 | test_node05_case_d_giveup_intercepted_verified_passed + 上轮真机 46 步样本 ✅ |
| E | budget low + verification pending | Done 相位 Reserve 同上；verification="pending" → REQUIRES_VERIFICATION（readiness 纯函数） | completion_readiness 矩阵测试 ✅ |
| F | budget low + no new evidence | 反重复上界（§4）：Replan×3 / graph stall×2 / Reserve≤1 逐层收敛到 GiveUp；资源耗尽→F8→Stop | test_p3_stuck_replans_or_gives_up / test_t4_semantic_stall_gives_up / test_t6_replan_hard_cap ✅ |

**新增（本单 Node 06/08）**：budget_exhausted 作为分类输入（F8）+ Reserve 消耗
scratch 留痕 + `failure_strategy(F8)=Stop` 单测锁定——资源耗尽不得进入任何
恢复策略（INV-FA01-E）。

**open（如实登记）**：C/F 的 budget-aware 判定目前是**既有常量**（BUDGET_LOW_THRESHOLD=0.15
等，修 A 备而未用），本轮未标定未改动——标定须顶层批准（复-3 裁决 2）。
