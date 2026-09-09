# P0 过闸确认 + P1 待办

## P0 闸门结论：✅ 正式过闸

| 指标 | 值 |
|------|-----|
| LOC | 2,974 |
| 单元测试 | 41 |
| 实现率 | 4/4 = 1.0 (A1/A2/A3/A4) |
| 🔴 偏离 | 0 |
| 🟡 偏离 | 1 |
| 🔵 偏离 | 2 |

## cargo test --workspace 日志

```
running 41 tests across 8 crates
result: 41 passed; 0 failed; 0 ignored
build: zero warnings
```

完整日志见上文。

---

## P1 强制待办（守门员要求）

### 🟡 send_message 丢弃 _req — 用户消息未进 loop

- **位置**: `crates/service/src/session.rs` — `send_message()` 方法
- **问题**: 接收 `_req: MessageReq` 但不使用其内容，session 是 goal-driven 单 run，用户无法追加消息
- **要求**: P1 必须将用户消息注入 agent loop 上下文

### 🔵 Done 事件语义模糊 — 任何终止都标 "completed"

- **位置**: `crates/service/src/session.rs:169`
- **问题**: budget 耗尽、error、自然完成 全部合成同样的 Done 事件
- **要求**: 区分 "completed" / "error" / "budget_exhausted" 等终止原因

### 🔵 MockProvider.chat 不推进脚本 — 依赖合成 Done

- **位置**: `crates/service/tests/integration_test.rs` — MockProvider 实现
- **问题**: `.first()` 不推进脚本，loop 自然数到 Done 未经证明
- **要求**: MockProvider 按脚本推进（pop 而非 peek），去掉对合成 Done 的依赖

---

## P1 阶段计划（待展开）

P1 范围 = execution-plan.md §4，待守门员确认 P1 详细需求后启动。
