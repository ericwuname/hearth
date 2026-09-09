# v23 阶段三施工计划 — 事件信封 + 缺口推导 + SSE（WP-1/2/3）

> 基线：WP-0 完成（222 tests），WP-1~WP-10 全部解封
> 上一阶段最大成果：**加新交互 = 0 行内核改动。** 从此 Human OS 再怎么变都不需要动 agent-core。

---

## 一、当前状态

```
WP-0  ✅  通用交互原语（222 tests，内核不认 kind）
WP-1  ⬜  事件信封与 span 树
WP-2  ⬜  SSE 出口与录制重放
WP-3  ⬜  规划缺口推导器
WP-4  ⬜  Observer 地基
WP-5  ⬜  确定性指标引擎
WP-6  ⬜  规则引擎与 Finding
WP-7  ⬜  G0 红线熔断
WP-8  ⬜  产物登记
WP-9  ⬜  思考摘要
WP-10 ⬜  前端接真流
```

---

## 二、阶段3 施工：WP-1 + WP-3 并行 → WP-2

从 v23 规划 §5.2 的依赖图，WP-1 和 WP-3 可以并行开工（都只依赖 WP-0），WP-2 依赖 WP-1。

### WP-1：事件信封 + span 树（3h）

| step | 文件 | 动作 | 独立验证 |
|---|---|---|---|
| ① 信封扩展 | `crates/api/src/lib.rs` | `AgentEvent` 加信封字段 `{schema_version, ts, seq, span_id, parent_id}`，不删任何现有变体 | `cargo check` |
| ② span 钩子 | `crates/agent-core/src/loop.rs` | 每个相位进入/退出 emit `span_open/close`（`parent_id` 继承 + 真实 `t0/t1`）；子代理 span 嵌套 | 事件流 gzip → zcat → 还原 span 树 |
| ③ 离线还原 | 新测试 | 录制一段含嵌套子代理的会话 → 离线还原 span 树 → 深度 ≥ 3 | 测试过 |
| ④ 向后兼容 | `codex-cli` | SSE 事件格式扩展为带信封，旧字段保留别名 | CLI 渲染不变 |

### WP-3：规划缺口推导器（3h）

| step | 文件 | 动作 | 独立验证 |
|---|---|---|---|
| ① gap 类型 | `agent-types` | `Gap{from, why, blocking, auto_assumed}` 结构体 | `cargo check` |
| ② 推导逻辑 | `planner/src/lib.rs` | plan 产出后扫描缺口：目标无 from → gap(from="missing_goal_source")；方案有未澄清项 → gap(why="ambiguous_option")；`blocking=false` 项 100% 填 `assume` | 单元测试 |
| ③ 通用中断 | `loop.rs` | gap 为 `blocking=true` → emit `InteractionRequested{kind="clarification"}` | replay 跑通澄清→暂停→回答→继续 |
| ④ 门禁 | 测试 | 抽 5 个真实计划，gap 100% 带 `from+why`，blocking=false 100% 带 `assume` | 缺 from/why → 退回 |

**注意**：WP-3 使用 WP-0 的通用交互原语做澄清——gap 产出 blocking 问题时走 `InteractionRequested{kind="clarification"}`，内核不加一行分支代码。

### WP-2：SSE 出口与录制重放（2h，WP-1 完成后）

| step | 文件 | 动作 | 独立验证 |
|---|---|---|---|
| ① schema 字段 | `service/sse.rs` | SSE 流每事件带 `schema_version` + `seq` 编号 | curl 看 SSE 流含 schema 字段 |
| ② Last-Event-ID | `service/sse.rs` | 客户端断线重连时发 `Last-Event-ID: <seq>` → 服务端从该 seq+1 续发 | 断线 3 秒 → 重连 → 事件连续 |
| ③ 录制 | 新工具 | 实时会话的事件流写为 JSONL（每行一个事件） | replay 工具读 JSONL → 渲染一致 |
| ④ G2 重放 | 测试 | 离线 JSONL 重放渲染 = 实时渲染 | 测试过 |

---

## 三、执行顺序

```
WP-0 (done) ──┬── WP-1 (3h) ── WP-2 (2h) ──→ 进入阶段4
              │
              └── WP-3 (3h) ─────────────────→ 并行完成
```

WP-1 和 WP-3 **可以并行**——它们改不同的 crate（WP-1 改 api + loop.rs，WP-3 改 agent-types + planner + loop.rs），但改 loop.rs 时有冲突风险。建议**不同人**并行或同一人**先 WP-1 再 WP-3**（loop.rs 序列化改完后再加缺口逻辑）。

---

## 四、门禁

| WP | 门禁 |
|---|---|
| WP-1 | 嵌套 span 树可从事件流完整还原（深度 ≥ 3）；222 tests 不得回归 |
| WP-2 | 录制 JSONL → 离线重放 → 渲染一致；断线重连不丢事件 |
| WP-3 | 抽 5 个真实计划 → gap 100% 带 from+why；blocking=false 100% 带 assume |
| 全 | fmt 0 / clippy 0 / test 全绿 |

---

## 五、阶段4 预览（本轮不展开）

WP-1/2/3 完成后 → WP-4（Observer 地基，新建 crate）→ WP-5/6/7（指标/规则/熔断）。预期下下批。