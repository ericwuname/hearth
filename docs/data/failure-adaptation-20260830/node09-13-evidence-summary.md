# Node 09/12/13/14 真机证据汇总（P1-FAILURE-ADAPTATION-01）

执行 VM：.131　binary：0.2.15（含 FA01 全部代码）　provider：Agnes　budget=50 / HEARTH_TASK_TIMEOUT_SECS=600 / HEARTH_ALLOW_NO_CGROUP=1
criteria 冻结全文：见 `node09-12-13-criteria-frozen.md`（批-5 归档）

## Node 09 calcpkg（TDD 受控失败）— 3 跑

| 跑 | 终态 | steps | 关键证据 |
|---|---|---|---|
| r1 | failed（T4 停滞旁路） | 43 | **红样本 1**：事实全完成（lib.rs 含 to_uppercase / RESULT.txt 正确）但 GiveUp 经 T4 停滞 Err，零核验 → 触发 T4 拦截机制修复 |
| r2 | failed（budget_exhausted headless abort） | 50 | **红样本 2**：同根旁路 → 触发预算拦截机制修复 |
| r3 | failed（deadline_exceeded 604s） | 42 | deadline 权威生效（§11，故意不拦截）；Agnes 10 次 backoff（28-55s 慢响应）；独立复验：lib.rs E0425 编译失败 → criteria 必败 → **系统未假报 completed**（INV-FA01-G） |

受控失败本体（r3 实证）：`exit code: 101`（cargo test 真失败，:73 行）→ 修复动作发生（write lib.rs 多次）→ 复测未能在 deadline 内收敛。**层归：model（修复未完成+未自报 Done）+ environment（Agnes 延迟）；mechanism 层（分类/拦截/权威终止）行为全部正确**。

## Node 12 mathlib（双受控失败 + 上轮 shout OPEN 承接）— 1 跑

- 49 步，terminal = failed。
- **机制层亮点（真机）**：GiveUp 拦截实际触发——`VERIFICATION_RESERVE(give_up interception) reserve_used=1` → 核验发现 `cmd failed` + `lib.rs 不含 to_uppercase` → `GIVE_UP_INTERCEPTED` 回喂修复 → Reserve 尽 → 诚实 failed。
- 独立复验：RESULT.txt=LONGRUN_RECOVERY_PASSED（模型写了标记）但 lib.rs E0425、无 to_uppercase——**上轮"STATUS 达标但 bug 未修"假阳性形态再次出现，本轮被 acceptance 层正确拒绝**（批-5 价值实证）。
- 层归：mechanism PASS（拒绝假完成）；model FAIL（修复未落地）。

## Node 13 wordcount（非 mathlib 样本）— 1 跑 ✅ 全闭环

- **46 步（≤52 判据；overhead=+4 vs 最优基线 42）**，terminal = **completed**。
- 真机链路：`GIVE_UP_INTERCEPTED 风险出现 → VERIFICATION_RESERVE → GIVE_UP_OVERRIDDEN: reserve verification passed → routing to completion (Case D)` —— **非 mathlib 样本上的第二个 GIVE_UP_OVERRIDDEN 真机实证**（同时满足 Node 15 Case D 回归要求）。
- 独立复验（执行窗口外部复跑）：
  - `cargo test`：**1 passed / 0 failed**（cmd criteria 独立通过）
  - main.rs 含 top_words（file criteria 独立通过）
  - RESULT.txt = WORDCOUNT_DONE（file criteria 独立通过）
  - `printf "b a b c b" | cargo run` 输出 `b 3 / a 1 / c 1`（**语义正确性**）

## Node 14 QA 负回归 — PASS

- Q1 "什么是所有权？"：2 步直答完成，无 replan/give_up/recovery 进入。
- Q2 "查看状态"：6 步 status 工具直答完成，无 failure side effect。
- 单测侧修 B 疑问句分类（INV-ED01-A）随 gate 全绿。

## 完整链路覆盖（§28 纪律对照）

| 链路环节 | 证据 |
|---|---|
| failure → classification | Node 09 r3 exit code 101；loop 观察级分类 fixtures（PlanFailure/Unknown） |
| classification → strategy | failure_strategy 纯函数矩阵 + scratch telemetry（Node 06） |
| strategy → recovery | Node 13：Reserve 核验 → passed → completed；Node 12：failed → 回喂 → 诚实终止 |
| recovery → retest | Node 13 cargo test 1 passed（独立复验） |
| retest → verification | acceptance 三 criteria 独立全过（Node 13） |
| verification → terminal | Node 13 completed（46 步）；Node 09/12 failed（诚实） |
| anti-loop | Node 09 r1 T4 有界停滞（bounded stop 生效）；replan cap 3 / graph stall cap 2 / Reserve ≤1 单测全绿 |
