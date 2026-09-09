# 测试窗口施工任务书 · Step 2 Campaign（v0.2.23 基线）

> **签发**：执行窗口　**日期**：2026-09-02　**收件**：测试窗口（砺·评审代行）
> **授权链**：放行令 v1.0（四步批准）→ 五件送审物审批裁决书（§4 测试窗口指令）→ 本任务书
> **一句话**：**campaign 前置已全部清零（F-3/OD-11 已解除，gate 486/0 已在 VM 复跑确认），本单 = Step 2 可直接开工的完整施工参数。**

---

## 〇、开工前置状态（全部 ✅，2026-09-02 15:40-15:55 执行窗完成）

| # | 前置 | 状态 | 证据 |
|---|---|---|---|
| 1 | COND-1 harness 源归一 | ✅ repo `tools/simuser/run_rc52_matrix.py:16` OUT → `~/fa/n14-final`；VM 树已同步 | commit `606cd02` |
| 2 | VM 源树（T-4/F-3） | ✅ .131 `~/codex` 实为新鲜树（T-4 NO_CODEX = `git log` 误报，tar 同步无 .git）；今日 simuser 资产已增量同步；`.133 ~/codex_t` 同步刷新 | `~/gate-rerun-0902.log` |
| 3 | COND-5 gate 486/0 复跑 | ✅ **.131 实跑确认：agent-core 130/0 + workspace --exclude agent-core 356/0**，RC 双 0 | `/tmp/gate-ac.log`（2 段）/ `/tmp/gate-ws.log`（59 段） |
| 4 | binary 探活 | ✅ .131 hearth 0.2.23 md5 `34f6f807…`（砺 T-3 已验） | — |
| 5 | simuser 资产（sampler/driver/检测器 16） | ✅ 双 VM 在位；**G1 已 CONDITIONAL PASS、修正回执待砺复核（G1-C3）——B2 资产按裁决书 §3 解锁，campaign 可引用** | G1 材料 v1.1 |
| 6 | 判定器 `~/fa/n14/n14_judge.py` | ⚠ COND-4：**give_up 主导性检查须在 campaign 判定前加入**（give_up ≥阈值且终态 failed → RC52_REPRO，provider 字符串降注记），B1 原始日志做回归用例 | 砺评审 §裁定二 |

## 一、施工内容

### 1.1 主矩阵：RC52 三臂（绑 tag v0.2.23，禁改码）

- 复用 `~/fa/p4-rerun2` harness，OUT 已指向 `~/fa/n14-final`；
- **A fresh / B 前缀重放 / C resume** 三臂；**C1 单跑先行**（COND-3：J-1/J-2/J-3 首跑验证通过才放 C2/C3），C 数据单独标注 **"v0.2.23 首次闭合"**；
- 判定**以 continue 终态为准**，driver marker 仅辅助（B1 教训）；禁并发跑矩阵（`_newest_changed_session` 按 mtime 取最新）。

### 1.2 ≥30 runs 全新 campaign（v0.2.23）

- **不得混入** v0.2.21 的 10 runs（S6 跨标签纪律）；每跑四件套归档（终端侧全量 log + driver events + planner dump + session JSONL 路径记录）；
- analyzer 全跑（16 检测器口径，`~/codex/tools/simuser/analyzer.py`）；hydration/truncation 计数用 diagnostics.log 差值；
- 环境模板：**跑批必设 `HEARTH_ALLOW_NO_CGROUP=1`**（与 gate 作用域区分，勿混）；探活 Agnes（401=可用）；provider 不可达样本必须出失败率分母。

### 1.3 七场景 × 三样本（Node 12）

- 场景：Fresh / Long-session / After-stall / After-compaction / Resume / Continue / Failure-diagnostic；
- 每类**必须留 success + failure + counterexample 三样本**（禁只存失败——幸存者偏差）；
- 场景资产：`~/codex/tools/simuser/scenarios/s01..s10.yaml`（Node 12 七类 = s01-s07）；persona：`personas/p01..p12.yaml`（G2 已解锁）；采样：`sampler.py --amplify`（Q11 声明随报告头部输出）。

### 1.4 报告头部强制（Q11 + 口径）

- Q11 声明逐字引用 `sampler.py::Q11_DECLARATION`（单一人类主体，禁"真实用户成功率"）；
- 与顶层熟悉版盲测（42 轮）、A/B 首正式轮数据**分表列示，禁混表**（RC51 教训）。

## 二、交付物

| # | 交付 | 落点 |
|---|---|---|
| 1 | campaign 报告（≥30 runs + 七场景三样本 + Q11 头部 + 混表禁令遵守声明） | `docs/core-revalidation/` |
| 2 | COND-3：J-1/J-2/J-3 首跑验证记录（C1 单跑） | 报告附录 |
| 3 | COND-4：判定器 give_up 主导性检查 + B1 回归用例 | `~/fa/n14/n14_judge.py` + 报告 |
| 4 | 四件套原始数据 | `~/fa/n14-final/` |
| 5 | hydration/truncation 差值计数 | 报告表格 |

## 三、边界与红线

1. **禁改令**：`crates/` 零改动；发现缺陷只登记（进统一台账 `docs/core-freeze-candidate/unified-ledger.md`，禁另立登记文件——可用性任务书 T-4 并账纪律）；
2. **并发纪律**：.131 跑 campaign 期间严禁同机并发任何 LLM 任务；Phase D 在 .133 并行允许（可用性任务书 §二），两机分工互不占用；
3. 判读分离：campaign 数据判读 = 砺；测试窗交数据与初步归因，终判归砺/顶层；
4. 版本冻结：绑 tag v0.2.23（commit `117a947`），campaign 内禁边测边改。

## 四、关键文件地址（.131 VM）

| 文件 | 路径 |
|---|---|
| harness runner | `~/codex/tools/simuser/run_rc52_matrix.py`（OUT=~/fa/n14-final） |
| PTY 驱动 | `~/codex/tools/simuser/driver.py` |
| analyzer（16 检测器） | `~/codex/tools/simuser/analyzer.py` |
| 采样器 / 资产 | `~/codex/tools/simuser/sampler.py`、`personas/`、`scenarios/` |
| 判定器 | `~/fa/n14/n14_judge.py`（COND-4 修改点） |
| gate 复跑日志 | `~/gate-rerun-0902.log`、`/tmp/gate-ac.log`、`/tmp/gate-ws.log` |
| 上一轮参考数据 | `~/fa/p4-rerun2/`（v0.2.21 基线，禁混表只作对照） |
| 统一台账（新缺陷登记处） | `docs/core-freeze-candidate/unified-ledger.md`（repo 侧） |
