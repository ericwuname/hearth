# CHANGELOG v0.2.14 → v0.2.15 — P1-FAILURE-ADAPTATION-01（2026-08-30）

> 使命：Failure Adaptation——失败不是 retry 指令，是"发生了什么、为什么、该换什么策略、恢复是否真的成功"。
> 证据：docs/hearth-p1-failure-adaptation-01-final-report-v1.md（v1.1）+ review-pack-v1 + docs/data/failure-adaptation-20260830/（11 个证据文件）。

## 核心修复：GiveUp 五路径全景收口

上轮修 D 只拦截了 GiveUp 五条产生路径中的一条。本轮盘点全景并收口：

- **T4 停滞旁路拦截**（真机红样本：43 步任务事实全完成但未核验即 failed）——
  套修 D 原则：停滞放弃前花 Reserve 核验，passed → 路由 Done；failed → 有界停止不变。
- **WS9 预算 headless abort 旁路拦截**（真机红样本：50 步同根）——预算耗尽先核验，
  passed → 路由 Done（**不延长执行预算**，INV-FA01-E）；Reserve 尽 → 原 WS9 ask/终止路径不变。
- **deadline 旁路故意不拦截**——Safety > Deadline > Recovery（§11），
  Node 09 r3 604s 权威终止实证（INV-FA01-D）。
- **F9（砺批-1）**：criteria 为空的 Product 任务放弃必须显式打
  `giveup_unverified` 结构化标记（scratch + RunReport.summary），失败报告
  `error_detail` 不再折叠为 "loop error"——未核验的放弃可审计、不得伪装正确终态。

## Failure Taxonomy F1-F10 + 策略矩阵代码化

- `classify_failure` 七分类 → **十类**：+PermissionFailure（审批拒绝，优先于
  exit code）/ +ResourceFailure（budget 耗尽，优先于 F7）/ +Unknown（exit_code=None
  无其他信号，不冒充）。每类正例 + 4 组相邻边界反例，禁 LLM judge（STOP-9）。
- `failure_strategy` 六策略纯函数（Retry/Repair/Replan/Verify/Escalate/Stop）：
  **Retry 仅 Transient 一类**（INV-FA01-A）；F6/F7→Replan（INV-FA01-F）；
  F8→Stop；F9→Verify（INV-FA01-G）。
- 新结构化通道：`ToolResult.error_kind`（serde-default 向后兼容）承载
  dispatcher `TaskDeadlineExceeded` downcast 投影——RC20 纪律（禁错误文本解析）。
- loop 观察级消费端：reflect 错误步分类落 scratch telemetry
  （`last_failure_class` / `last_recovery_strategy`），同一工具连续失败计数。

## 真机实证（budget=50 / 600s / Agnes，criteria 冻结）

- **Node 12 mathlib**：r1 拦截拒绝假 completed（模型写 RESULT.txt 标记但 bug
  未修——上轮假阳性形态被核验层正确拦下）；r2 拦截捕获完整 E0425 结构化诊断；
  **r3 completed 21 步**（GIVE_UP_OVERRIDDEN Case D），独立复验 cargo test
  **2 passed**——**上轮 shout 假阳性 OPEN 正式行为级闭环**。
- **Node 13 wordcount（非 mathlib）**：46 步 completed 全闭环，三 criteria +
  二进制输出（`b 3 / a 1 / c 1`）独立复验全过。
- **Node 09 calcpkg**：3 跑均 failed（两条旁路的红样本来源 + deadline 权威终止），
  层归 model+environment；机制层全部正确，无假 completed。
- **Node 14 QA 负回归**：2/6 步直答，零 recovery 进入。

## 测试

新增 8（taxonomy 十类+边界反例 / 策略矩阵边界 / F9 两例 / 同工具重复两例 /
T4 正反例）。Final Gate v0.2.15：四 RC 全 0，**437 passed / 0 failed**
（基线 429+8）；双 VM binary 0.2.15。

## Governance

两套 INV 前缀区分：`INV-ED01-*`（上轮，单测未回改）/ `INV-FA01-*`（本轮
A-H）。新不变式：**凡未核验的放弃，要么先核验（Reserve ≤1 零和），要么显式标记**。

## OPEN（移交顶层）

① bash exit code 接入 ToolResult（F2/F5 运行时判定补全）；② progress 语义
扩展批准件（四类 progress 候选 + 二阶效应清单）；③ verify_replan_count 跨相位
双上限（Act<1/Done<3）统一；④ Node 09 单样本 model 层收敛（换模型/错峰对照）。
