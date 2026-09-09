# Hearth R2-C 基线重同步（P0 · 2026-08-28）

> 批示 §一 P0：只记录当前真实状态，不复述旧报告。全部数据实测（本地 git + VM 门禁/构建）。

## 版本与门禁

| 项 | 实测值 |
|---|---|
| HEAD | `db925d1`（ux-polish-01；其上另有他窗口 commit `d81087b`/`0ccbe26` 叠加） |
| Cargo version | **0.2.7**（R2-C 本轮施工后 bump 0.2.8） |
| VM 门禁 | fmt/clippy/test 全 RC=0，**370 passed / 0 failed**（~/t_gate.log，R2-D 收敛轮） |
| VM /usr/local/bin/hearth | **已升级 0.2.3 → 0.2.7**（用户手工测试通道，2026-08-28 16:15） |
| VM ~/codex_t/target/release/hearth | 0.2.7 |

## 前序审计状态的当前核实（逐 crate）

| crate | v0.2.5-v0.2.7 变化 | 当前状态 |
|---|---|---|
| agent-core | terminal.rs（九态+映射）新建；loop.rs +egress 审批/apply_turn_goal/restore_taskgoal/task_continuity_message/acceptance_verification_status；context.rs +session_id 归档隔离 | 4646 行级核心，编译 0 warn |
| agent-runtime | session.rs map_event +GoalChanged 映射 + "goal_changed" type 名 | 契约只增 |
| agent-types | RunState +4 字段（original_goal/goal_revision/constraints/acceptance_criteria，serde default）；TaskGraph +next_action_deterministic/completed_titles/remaining_titles | 旧会话文件兼容 |
| api | AgentEvent +GoalChanged 变体 | 只增不改 |
| session_store（codex-cli） | +save/load_taskgoal、save/load_graph_with_revision（同 state_revision） | 旧格式兼容 |
| llm-gateway | +cache_telemetry（自 v0.2.7-R2C 迁入）+ TelemetryProvider 装饰器（v2 采集器，本轮新增） | env 开关零开销 |
| resource-monitor | **未动**——105 行相位体检器（sysinfo：memory_percent>80 或 disk_free_gb<1.0 才告警）；**与工具写盘路径零接线**（补充 3 实锤） | R2-C §8 断点确认 |
| tool-runtime | dispatcher +ResourceLedger（per-call/per-tool 字节记账，Observe 层——本轮新增） | 10MB/1GB 阈值 warn |
| sandbox | 未动（限路径不限体积——57G 事故盲区仍在） | D 类候选 |
| observer | 未动（未知事件默认忽略——GoalChanged 兼容） | 消费侧扩展挂账 |
| bridge | 未动——284 行 "前端桥接"，无 production 调用 | §十 DEFER 裁决（见设计单） |

## 测试资产

| 套件 | 数量 |
|---|---|
| R2-D T1-T7（TaskGoal） | 7（agent-types r2d_tests / loop tests / session_store r2d_tests） |
| R2-F egress | 2 |
| R2-1 terminal/cache_telemetry v1 | 5（其中 cache_telemetry 3 个随模块迁至 gateway） |
| 既有基线 | 356 |
| **合计** | **370**（R2-C 施工后另 +ledger 测试） |

## 环境事实

- VM Ubuntu 24.04 / 4C8G / 磁盘 118G（8-28 曾被失控写盘塞满至 100%，清理后 49%——**磁盘健康进采集前置检查**）
- LLM 通道：Agnes（`agnes-2.5-flash`，config provider=openai url=agnes——用户授权预算）
- telemetry 采集开关：`HEARTH_CACHE_TELEMETRY=<jsonl>`；session 归属：`HEARTH_TELEMETRY_SID`
