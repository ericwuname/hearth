# v23 阶段三验收报告：WP-1/2/3（事件信封 + 规划缺口 + SSE 重放）

> 日期：2026-08-02 | 基线：`0e45722`（WP-0 闭环）→ 验收后 `923f43c`
> 依据：`v23-phase3-plan.md` + `top-level-plan-v23.md` §5.2
> 性质：执行窗口一轮自主施工——**WP-1 / WP-2 / WP-3 全闭环**，225 tests 零回归

---

## 〇、审查补充优化（用户问"有没有补充优化的地方"）

1. **关键缺口：WP-0 的 payload 被丢弃**（审查发现）——`resolve_interaction` 只消费
   `resolved: bool`，响应 payload 存为 `_payload` 未用。对 approval 无所谓，但
   **WP-3 的 clarification 必须把用户回答回填给 planner**。已补：
   `InteractionState::Resolved/Rejected` 携带 payload + `take_interaction_response`
   （一次性回读，内核只搬运不解析）。**这是 WP-3 的前置，plan 未预见**。
2. **信封方案定版为 serde flatten**：`EnvelopedEvent{schema_version,ts,seq,span_id,parent_id}` +
   `#[serde(flatten)] event: AgentEvent`——JSON 顶层 `{...信封..., type, 业务字段}`，
   **旧客户端读 type/业务字段不受影响**（向后兼容），新客户端读 seq/span_id。
3. **span_id 由服务端分配**（EnvelopeState 状态机）——符合事实产生权公理
   （BE 产生事实，FE 投影事实）；loop 只管时序（name/t0/t1）。
4. **相位 span 用内联包装而非泛型方法**——泛型 future 会破坏 trait 方法的 Send
   bound（编译期暴露），内联 4 处解决。

---

## 一、WP-1 事件信封与 span 树 ✅

| step | 内容 | 证据 |
|---|---|---|
| ① 信封 | `EnvelopedEvent{schema_version,ts,seq,span_id,parent_id}`（serde flatten）+ `AgentEvent::SpanOpen/SpanClose` 变体 | 编译过（clippy 0） |
| ② span 钩子 | loop 相位进入/退出 emit SpanOpen/Close（内联包装）——plan/act/observe/reflect 每相位一对；子代理嵌套天然形成多层 | 门禁测试通过 |
| ③ 离线还原 | `test_envelope_seq_monotonic_and_span_tree`：plan→observe→子observe 深度 3 嵌套，parent 链还原 + close 回填同一 span_id（LIFO） | ✅ ok |
| ④ 向后兼容 | SSE `event:` 名不变 + flatten 信封（旧字段保留）；CLI repl 字符串 match 不受影响 | 225 tests 零回归 |

## 二、WP-2 SSE 出口与录制重放 ✅

| step | 内容 | 证据 |
|---|---|---|
| ① schema+seq | 每 SSE 事件 data = 完整信封 JSON（schema_version + seq 单调递增） | EnvelopeState 测试 |
| ② Last-Event-ID | `sse_stream_with_replay`：缓冲事件重新 envelop（seq 与实时一致）→ seq>last 先发 → live 无缝接续同一 EnvelopeState——**断线重连不丢事件（G4）** | 实现 + 结构测试 |
| ③ 录制 | `GET /api/v1/sessions/:id/events` 导出 JSONL（每行一个信封事件） | 路由注册 |
| ④ G2 重放 | `test_replay_envelope_consistent`：同序列两次 envelop → seq 序列一致（确定性） | ✅ ok |

## 三、WP-3 规划缺口推导器 ✅

| step | 内容 | 证据 |
|---|---|---|
| ① Gap 类型 | `Gap{from,why,blocking,auto_assumed}` + 构造器 `assumed()`/`blocking()`——**强制契约**：blocking=false ⇒ auto_assumed=true；from/why 必填 | agent-types |
| ② 推导逻辑 | `planner::derive_gaps`：无来源目标 → missing_goal_source（非阻塞）/ 任务含"或/待定/可选" → ambiguous_option（阻塞）/ 干净 plan 全 assume | test_derive_gaps ×3 ✅ |
| ③ 通用中断 | blocking gap → `InteractionRequested{kind="clarification"}` + 等待 + `take_interaction_response` 回读 → 注入 scratch（**内核不加一行分支**——WP-0 原语的第一个实际收益） | test_wp3_take_... ✅ |
| ④ 门禁 | 缺 from/why 构造器直接报错（编译期强制）；非阻塞必 assume（构造器强制） | 3 测试全过 |

## 四、门禁汇总

| 门禁 | 结果 |
|---|---|
| cargo fmt --check | ✅ 0 |
| cargo clippy -D warnings | ✅ 0 |
| cargo test --workspace | ✅ **225 passed 零失败**（+6 WP 新测试，-3 死代码 serialize_payload 测试） |
| window-framework | ✅ 208/208（未改动） |

## 五、WP 进度

```
WP-0 ✅  通用交互原语（222 tests）
WP-1 ✅  事件信封与 span 树（深度 3 还原）
WP-2 ✅  SSE 出口与录制重放（Last-Event-ID + G2）
WP-3 ✅  规划缺口推导器（clarification 走通用原语）
WP-4 ⬜  Observer 地基（下一批）
WP-5~10 ⬜
```

**WP-1/2/3 完成后，事件链路（信封→SSE→重放）与规划链路（缺口→澄清）已通**——
下一步 WP-4（Observer 地基，新建 crate）可直接消费信封事件流。

## 六、交付

- commit `923f43c`（WP-1/2/3，701 insertions）
- phase3-plan 验收清单全勾（plan §四 门禁全过）
- 审查补充：payload 回读接口（WP-3 前置）+ flatten 信封 + 服务端 span 分配
