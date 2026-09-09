# CHANGELOG v14.0 — 淬火轮

## Fixed
- **v14-1 毒债**：子代理预算 `max_steps/2`（7 步饿死，v13 FLAKY 根因）→ `(parent_max_steps-2).max(8)`（`agent-core/src/loop.rs:939`）。S4b 补测实证：4 条 15 步截断失败在新预算下全部 PASS。

## Added
- **wiring 第 8 条红线断言** `sub-budget-not-halved`（severity=red）：修复被回退即 CI 断裂（`docs/xray/wiring-v13.toml`）。
- **应力场工装** `bench/stress_v14.py`：ST1-ST8 × 3（token 饥饿/恶意目标/空项目/300并发burst/磁盘满tmpfs/乱码输入/会话隔离/审批门），全部判据经源码核实（无 `/events` 端点——ST8 走 POST /messages SSE 流捕获 `need_approval`）。**结果 24/24 零 panic**。
- **回放素材录制** `bench/archive_replay_v14.py`：31 条 PASS session 完整历史归档 `bench/replay/fixtures/`（replay.py 留 v15）。
- **按题预算** `bench/runner.py`：`meta.json max_steps` 覆盖全局 BUDGET；L4/L5 全体 + T13 现为 20 步。
- **链驱动** `bench/chain_v14.py` / `bench/s4b_v14.py`：S4→S7→S5/S6→S4b 全自动无人值守。

## Changed
- 夹具 v2（`fixture_version:2`）：T14 预置 serde_json dev-dep、T19 强调 `parse_positive` 函数名、T15 改动清单化；v13 原版存 `bench/tasks/_archive_v13/`。

## Metrics
- zhipu 20×2 主表 31/40=77.5%；预算修正口径 35/40=**87.5%**（≥85% 红线过、90% 目标未达）。
- 应力场 24/24 零 panic；会话隔离零泄漏 3/3；审批门 deny 语义实证 3/3。
- VM 四门全绿（fmt/clippy/test/wiring 8/8）。

## Known Debt（v15）
1. 90% 缺口：T14-add-serde 0/2（derive 宏）、T19-merge-duplicate 0/2（泛型合并）、T09 FLAKY 50%。
2. replay.py 真回放防线（素材已备）。
3. ST7 判据拆分 isolation_ok / task_ok。
