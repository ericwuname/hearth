# CHANGELOG v0.2.16 → v0.2.17 — P2-LONG-RUN-ACCEPTANCE-01（2026-08-31）

> 使命：Core Freeze 前长程可靠性——证明已完成的机制组合在长程真实任务中稳定协作。
> 证据：docs/data/long-run-20260831/ + Final Report。

## Node 01 — RC47 最小修复（criteria 空 + 已实质完成形态）

- 真机红样本（P2-MC Node 08）：续轮重写三文件后 planner give_up → failed
  （"有产物却 failed"假阴性）。根因：give_up 拦截链被 `!edd_checks.is_empty()`
  守门跳过（criteria 空时整链不生效，F9 只打标记不改终态）。
- 修复（give_up 消费端，禁动 planner schema）：criteria 空 + written_files 非空
  + 0 errors → 路由 Done（Done 相位盲区C 对产物做确定性校验兜底，INV-LR03）。
  criteria 非空的 FA01 Reserve 语义不变。
- 修 E 反例 fixture 语义细化：criteria 空 + **无产物**（纯 QA）→ failed 保留
  （QA 不被路由 completed）；有产物形态由 test_rc47 覆盖。

## Node 02 — bash exit code 五态接线（RC46，FA01 OPEN-1 闭环）

- 类型化载体 `BashExitError { exit_code, timed_out, formatted }`（tool-runtime，
  照抄 TaskDeadlineExceeded downcast 先例；Display = 原 formatted，LLM 可见内容不变）。
- `ToolErrorKind` 扩展：`ExitNonZero(i32)` / `ExitSignal(i32)` / `ToolTimeout`
  （serde 向后兼容，不新增 ToolResult 字段——批-4 纪律②）。
- scheduler 五态区分（exit 0 → None / >0 → ExitNonZero / <0 → ExitSignal /
  timed_out → ToolTimeout / task deadline → DeadlineExceeded），单/并行两路统一
  `classify_dispatch_error`。测试矩阵 + 集成投影测试（先红后绿）。
- 接线后 Unknown 只留给真正无信号场景（FA01 基线保持）。

## Node 05/07 — Memory 域最小修复

- **40 消息切片打标记**（批-1"打标记"分支）：切片发生时注入 System
  `[history note]`（如实告知模型更早 N 条未包含且未销毁）——保护对称性改善
  （压缩有 archive 先行，切片此前两样皆无）。入 archive 分支 DEFER（涉存储路径）。
- **chat 主路径 session 绑定**（P2-MC OPEN-4 闭环）：`run_local_continue` 在
  run() 前调 `set_session_id`（此前只有 rebuild/resume 路径绑定）→ 压缩归档
  落 `archive/<sid>.jsonl`（会话隔离恢复）。

## Node 03/04 — 审计结论（零代码）

- Node 03：Interaction 表面=分类器+结构化请求（修 B 后），历史切句形态已消失；测试覆盖 TaskControl/寒暄/终态投影行，interaction 触发正确性由 W8 fixtures 锁定。
- Node 04：progress 口径维持（FA01 §5 顶层裁决）——Case A（write 重置计数，W3 fixture）/ Case B（acceptance 回喂，FA01 fixtures）/ Case C（search-streak + T4 bounded）均有既有测试锁定，无假停滞真根因证据。

## 测试

新增 5（rc47 + 五态矩阵 + 集成投影 + slice marker + threshold 矩阵重构）；
agent-core **124 passed / 0 failed**。

## v0.2.17 → v0.2.18 — P2-CFR 补丁（同日）

- **压缩阈值优先级修正**（真机 COMPACT_DBG 实证）：provider-aware 注入值
  （Agnes caps 128K → 195,840 字符）在 v0.2.17 中压过 `HEARTH_COMPACT_CHAR_THRESHOLD`
  测试仪器，导致受控压缩实验静默失灵（探针 est=68 thr=195840）。修正优先级：
  **env（测试仪器）> injected（provider-aware）> 遗留常量**。生产语义不变
  （未设 env 仍走注入值）。阈值矩阵测试同步更新。
- 仪器纪律注记：P2-LR Node 13 的"双压缩"声明因本缺陷部分失真（压缩未实际发生，
  失败/修复/resume 链有效）——已在 CFR Final Report 如实更正。
