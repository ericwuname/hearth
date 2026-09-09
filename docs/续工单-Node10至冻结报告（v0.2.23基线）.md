# 续工单 · Node 10 → Campaign → 冻结报告（v0.2.23 基线）

**日期**：2026-09-02｜**基线**：tag `v0.2.23`（commit `117a947`，.131 部署 md5 `34f6f807…`，门禁 486/0）
**前置状态**：P4+P5 代码收口全部完成（详见 `.workbuddy/memory/2026-09-01.md` §⑧-⑲ 与 `docs/P4-REVALIDATION-01 完成度审计与冻结就绪评估 v1.0.md`）。**从本单起不再改 crates/ 代码**。

## Step 1 · Node 10 SimUser Stage 2（冻结区外，纯测试基建）
- B1 相关矩阵（确定系数后写死进 sampler）→ 联合分布采样器 `tools/simuser/sampler.py`（G1 硬门禁：矩阵先评审再进 B2）。
- B2 persona 向量 schema + 种子 + **10 个矩阵场景**落 `tools/simuser/personas|scenarios/`（现空目录）。
- B3 检测器 13→15：补 `multi_intent_drop` / `paste_error_probe_quality` / `longpaste_truncation`。
- B4 四维验收（三形态 + 双模态分布对齐 + 任务域 + 结构覆盖）。
- Q11：campaign 报告头部必须声明语料来自单一人类主体，禁"真实用户成功率"表述。

## Step 2 · Node 11/12/14 campaign（绑定 v0.2.23，禁改码）
- 复用 `~/fa/p4-rerun2` harness（driver.py + run_rc52_matrix.py 改 OUT → 新目录 `~/fa/n14-final`）。
- ≥30 runs（含 Step3 10 跑不行——那些是 v0.2.21；v0.2.23 需全新 ≥30）+ 七场景 × 三样本（success/failure/counterexample）。
- 每跑四件套归档 + analyzer 全跑 + 判定器 `~/fa/n14/n14_judge.py` + hydration/truncation 计数（diagnostics.log 差值）。
- 环境：`HEARTH_ALLOW_NO_CGROUP=1`（跑批必设）；开工探活 Agnes（401=可用）；**workspace 全量测试挂起缺陷已登记——测试期避开 `cargo test --workspace` 全量口径**。

## Step 3 · 盲测包（Strong #10，需顶层本人执行）
- 从黄金集抽 10 条 + 5 条新场景，双列（hearth 输出 / 评分表）→ 用户按表打分 → 回收算 F0 缺陷率。

## Step 4 · Node 15 八文件（`docs/core-freeze-candidate/`）
architecture-status / behavioral-status / projection-status / simuser-status / open-deviations / evidence-index / independent-review-request / freeze-candidate。
（前三者+open-deviations 可依 `docs/p5-foundation/specs.md` 与审计文档先行起草；后五者依赖 Step 2/3 数据。）
最终裁决 = 人工窗 + 外部 AI 窗 + 独立证据复核；产物 = CORE FREEZE CANDIDATE（禁自行宣布 FREEZE）。

## 已知开放项（带入冻结材料）
- B1 判定歧义（PROVIDER_UNREACHABLE vs RC52_REPRO）→ 砺裁定。
- C 条件仍 BLOCKED（R-1 待砺解封）。
- 双 hang 取证（RT4-OOM 探针 socket 死锁）→ 缺陷登记待修。
- FZ-RFC-1/2b 双签追认待。
