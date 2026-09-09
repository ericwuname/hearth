# FZ-RFC-2b · RC52 give_up 三臂统一消费端收口（T4 stall 臂 + budget 臂扩展）

**日期**：2026-09-01｜**提出**：执行窗｜**授权**：顶层（2026-09-01 23:44 全权委托 + 当日三条指令追加授权 B 相二次修复）
**状态**：已实施（v0.2.23 工作树），**待双签追认**（人工窗签发=顶层本人口头批准留痕；外部 AI 窗追认待）

---

## 1. 申报内容

**触达冻结区文件**：`crates/agent-core/src/loop.rs`（T4 stall 臂 + budget exhausted 臂 + Reflect GiveUp 臂统一重构）。

**变更**：把 Node 13 的 RC52 回填+RC47 路由（原仅 Reflect GiveUp 臂）抽为统一 helper
`rc52_route_done_if_session_artifacts()`，并接线到全部三条 give_up 生产出口：
①Reflect GiveUp 判定臂（原有语义等价重构）；②T4 stall 臂（**新增**）；③budget exhausted 臂（**新增**）。

**语义（三臂一致）**：criteria 空 + 本轮 0 errors 时——run 边界清空的产物事实从 session 级记录回填
（只读投影，不动 planner schema——STOP-1/2 防线），有产物即路由 Done；
文件存活性由 Done 相位盲区C 确定性校验裁决（INV-LR03 不变）。

## 2. 为什么必须扩到三臂（证据链）

| 证据 | 内容 |
|---|---|
| 集成测试 RED | `test_rc52_hydration_integration_through_run_boundary`（commit 286b428）：穿 run() 全路径后 turn2 give_up 仍 failed——Node 13 修复在生产路径不可达 |
| diagnostics 实证 | 失败走 T4 stall 臂：`do_plan_inner failed error=stalled: 2 consecutive replans...`；集成测试 turn2 实走 budget exhausted 臂（16 步耗尽，criteria/0-errors/session 产物三条件全满足仍 failed） |
| 测试窗 B 相 | Node 14 B 相（前缀重放）**100% 复现**（4/4），日志含 `stalled ... Tier3 T4` |
| Node 13 修复局限 | 回填只在 Reflect GiveUp 臂 → hydration=0（生产从未触发）+ pilot A1 failed |

## 3. 不变量（自查清单）

- [x] **T4 阈值=2 不动**（总包 §1 禁令只锁阈值，本 RFC 只加消费端路由）
- [x] planner schema 零改动（STOP-1/2）
- [x] criteria 非空路径零改动（FA01 Reserve 语义维持）
- [x] 权威不复制（回填是只读投影；存活性裁决仍在 Done 盲区C）
- [x] 有界停止保留：criteria 空但**无产物**或**有错误**时仍走原 Err/budget 终止
- [x] 非交互审批语义不变（budget 臂路由 Done 在交互 ask **之前**——完成事实已核验，无需延预算，对齐 4770 GIVE_UP_OVERRIDDEN 先例）

## 4. 验收

- 集成测试移除 ignore 后**必须绿**（先红后绿闭环）：`test_rc52_hydration_integration_through_run_boundary`
- 既有 T4 臂反例测试不破（criteria 核验 failed → 仍 stalled Err）
- workspace gate：fmt/clippy/test 全绿

## 5. 双签

| 签位 | 状态 |
|---|---|
| 顶层（人工窗） | 口头批准 2026-09-01（"B 相二次修复批准 + 全权委托"），**书面追认待** |
| 外部 AI 窗 | 待 |
