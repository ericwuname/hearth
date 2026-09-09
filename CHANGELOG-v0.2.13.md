# CHANGELOG v0.2.12 → v0.2.13 — P1-TASK-TRUTH-01（2026-08-30）

> 证据链：`docs/hearth-p1-task-truth-01-construction-order-v1.md` + Final Report + 真机 `/tmp/node07_v2_summary.log`（@.133）。
> 门禁：417 → **418+ passed / 0 failed**（final gate 路径见 Final Report）。

## O-4 Deterministic Acceptance Verification（更名自 O-4 Semantic Completion——顶层修正 5：本实现为确定性验证，语义解释仍归 DEV-2）（commit 1e22203）

- **criteria 结构化前缀约定**（兼容扩展，不改 schema）：`cmd: <命令>`（命令型）| `file: <p> contains <text>`（内容型）| `file: <p> nonempty`（存在型）；自由文本不核验（仅 Continuity 注入）。
- **cmd verifier**：走 dispatcher bash 通道（审批/sandbox/landlock 边界完整继承，不产生新执行能力）+ 副作用禁名单粗滤（修正 1）+ **写盘快照确定性检测**（守门员增 2：验证命令不得产生工作产物——禁名单漏花样漏不过快照）。
- **file verifier**：workspace 白名单（绝对路径/越权判 invalid 不读——增 1）+ 64KB 上限。
- **"passed" 生产者补齐**（O-4 根因修复）：Done 相位核验通过 → `acceptance_verification="passed"`（此前恒 none/pending）。
- **Verification Reserve**（顶层修正 2）：acceptance 核验失败回喂重试 ≤1（对齐源码 verify_replan 真实额度），telemetry 记账（增 4），不突破预算总量；耗尽 → verify_failed（acceptance_failures 明细）。
- **pending 桥**：init_taskgoal 与 run() 内 ContextManager::new 重建之间的 criteria 传递（否则 --acceptance 静默丢失——R2-D 时代无生产者未暴露）。
- **CLI `--acceptance` 生产入口接线**（Producer Audit：字段/三态骨架/provenance/resume 完好，唯一缺口=CLI 未接线——修复而非 STOP-8）。
- **真机 O-4 全链**（mathlib 复跑）：模型自述"PASSED"→ acceptance 核验以 L1 证据判定 → **外部独立复跑确证** lib 修复 + test ok + RESULT 一致——artifact 自述与事实的矛盾被结构化证据链约束。

## 顶层修正/裁决落实

- 修正 1：Producer Audit（criteria 唯一权威生产入口=init_taskgoal，CLI 未接线为唯一缺口——修复而非 STOP-8）；cmd: 定性=验证命令非工作命令（禁名单+快照+三道边界）。
- 修正 2：Reserve 替代"免费预算豁免"（不突破预算总量）。
- 修正 3：冲突样本 natural/replay 分开记录（Node 04）。
- 修正 4：Node 07 硬条件改"no unresolved REFLECT_FACT_CONFLICT"（conflict→verification→消解→completed 判成功）；"零假声明"降为观察指标。
- 修正 5：O-4 更名 Deterministic Acceptance Verification（semantic interpretation 仍归 DEV-2）。
- 守门员增 3：criteria revision-mismatch 沿用 R2-B stale 标注裁决（restore 路径 note 已覆盖，Audit 确认）。
- 守门员裁决 1：provenance 分窗登记（.131=~/codex 自有施工树；.133=~/codex_t；.131 ~/codex_t DEPRECATED）——vm-version-sync.md 修订 + Final Report 勘误。

## OPEN

- DEV-2 model judgment 残余（Node 07 42 步样本：修复成功但复测时序混乱+planner budget-low give_up 抢在任务实质完成前——Node 05 交互矩阵继续观察）。
- Q-3 kernel≥5 新位评估（另立）。
