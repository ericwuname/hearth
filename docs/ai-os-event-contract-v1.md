# AI OS 事件契约 v1（ai-os-event-contract-v1.md）

> **版本**：`schema_version=1`（信封）· 契约 `v1`
> **状态**：❄ 冻结（2026-08-21，后端任务书 #01 B1-3）
> **性质**：前后端（Human OS / CLI / Observer OS）解耦的**唯一契约**。
> 内核（AI OS）改事件结构必须升 `schema_version`，不得静默破坏消费方。

---

## 1. 传输层

### 1.1 SSE 出口

| 端点 | 语义 |
|---|---|
| `GET /api/v1/sessions/:id/stream` | 实时 SSE 流（EventSource 兼容）——`text/event-stream`，每事件一行信封 JSON |
| `GET /api/v1/sessions/:id/events` | 一次性 JSONL 录制导出（非流）——断线重放源 |

### 1.2 断线续传（G4）

- 客户端在 `Last-Event-ID` 头携带最后收到的 `seq`。
- 服务端重放缓冲中 `seq` 之后的事件，保证**事实序不丢**（G4：断线重连不丢事件）。

## 2. 信封（EnvelopedEvent）

每个 SSE 事件 JSON 顶层固定字段（`#[serde(flatten)]` 保证）：

```json
{
  "schema_version": 1,
  "ts": "2026-08-21T00:00:00Z",
  "seq": 42,
  "span_id": "1f2e...",
  "parent_id": null,
  "type": "tool_call",
  "...": "业务字段（见 §3）"
}
```

| 字段 | 类型 | 产生方 | 语义 |
|---|---|---|---|
| `schema_version` | u32 | 服务端 | 信封版本（当前 1）。升版本=消费方可识别破坏 |
| `ts` | RFC3339 | 服务端时钟 | 事件产生时刻 |
| `seq` | u64 | 服务端 | 会话内单调递增——G4 续传锚点 |
| `span_id` | String | 服务端 | 所属 span（无上下文为空串） |
| `parent_id` | Option\<String> | 服务端 | 父 span（根为 null） |
| `type` | String | serde tag | 事件类型（§3 枚举，13 个出流类型） |

**事实产生权**：信封字段全部服务端产生；消费方（前端/CLI/Observer）只投影，禁止回写。

## 3. 事件类型（14 变体）

| type | 载荷要点 | 语义 |
|---|---|---|
| `phase` | `phase` | 相位切换（Init/Plan/Act/Observe/Reflect/Done/Error） |
| `token` | `delta` | LLM 输出增量（流式） |
| `tool_call` | `call_id,name,args` | 工具调用发起 |
| `tool_result` | `call_id,is_error,output` | 工具执行结果 |
| `need_approval` | `approval_id,action,payload` | 审批/澄清请求（危险操作前/规划歧义）——见 §5；`payload`（v1.1 补注）在 `action="clarification"` 时携带问题内容 `{from,why}` |
| `reflection` | `verdict` | 反思结论（continue/replan/give_up） |
| `done` | `report` | 会话完成（含 usage/steps） |
| `error` | `message` | 会话错误 |
| `span_open` | `span_id,parent_id,name,t0` | span 生命周期进入 |
| `span_close` | `span_id,t1,duration_ms` | span 生命周期退出 |
| `interaction_resolved` | `interaction_id,by,resolved,latency_ms` | 交互已响应（事实入流，Observer 计算 autonomy_rate/wait_ratio） |
| `artifact` | `path,kind,delta_lines,size_bytes` | 产物登记（write_file/edit 成功） |
| `think_summary` | `phase,text` | 思考摘要（确定性模板，零 LLM） |
| `plan_draft` | `steps,gaps_found,gaps_to_ask,auto_assumed,gaps_to_ask_details,mode` | 规划草案（do_plan 完成时 emit）——见 §8 |
| `sandbox_violation` | `reason,exit_code,syscall_hint` | （v1.2 补注）sandbox 子进程被 seccomp KILL / cgroup 限制 / readonly 拒写时 BE emit 的事实——Observer/审计可记安全事件 |

> v24-post (backend-intelligence) 补注：`plan_draft` 由后端任务书 #01 B2 引入，
> 本版契约补入（新增事件类型，按 §7 升级规则**不升 schema_version**）。
> 载荷：`steps`=任务数组；`gaps_to_ask`=将问的 blocking gap 数；
> `gaps_to_ask_details`=blocking gap 明细（每项 `{from,why}`——产品红线"AI 不得问废话"的可见化）；
> `auto_assumed`=非阻塞 gap 明细（每项 `{from,why,assume}`）。

> 注：内核内部事件 `InteractionRequested`（通用交互，kind 开放字符串）在 **API 层映射为
> `need_approval`**（`session.rs map_event`）——即审批/澄清请求统一以 `need_approval` 出流；
> 响应统一走 `POST /interaction/:iid` → `interaction_resolved` 回注。消费方无需区分二者。

## 4. Span 树

- `span_open` 携带 `span_id`（服务端分配）+ `parent_id`——构成 span 树。
- `span_close` 携带 `t1` + `duration_ms`——时序事实由服务端记录（`loop.rs do_act` 相位内嵌）。
- 深度无硬限制；消费方按 `parent_id` 缩进还原。

## 5. 交互协议（clarify / approval）

### 5.1 请求入流

- **审批**：危险操作（write_file / shell / bind_tcp）前 emit `need_approval { approval_id, action }`。
- **澄清**：规划缺口 emit `interaction_requested { kind:"clarification", blocking, timeout, on_timeout, payload }`。
- **内核只认 `blocking`/`timeout`/`id`**——不 match kind、不解析 payload（WP-0 铁律）。

### 5.2 响应

- `POST /api/v1/sessions/:id/interaction/:iid` → `InteractionResponse { id, by, resolved, payload, latency_ms }`
  - R1：`id` 不匹配必拒；R2：一次性消费（防重放）。
- 响应事实以 `interaction_resolved` 事件**回注事件流**（Observer 依赖）。
- 遗留：`POST /approvals` 为薄适配层（deprecated，不删）。

### 5.3 默认超时语义

- blocking 交互默认 `on_timeout="abort"`（fail-safe；内核透传不解析）。

## 6. 消费方约束

1. **只投影**：渲染颜色/缩进/坐标由消费方决定；BE 禁返回颜色/HTML/坐标。
2. **按 seq 单调消费**：`seq` 断档 = 流不可信（Observer L2 fail-closed）。
3. **schema_version 检查**：不识别版本 → 拒绝消费（避免静默错读）。

## 7. 升级规则

- 新增事件类型：可加（`type` 为开放字符串，消费方按需处理，未知 type 忽略）。
- **修改现有事件载荷字段**：必须 `schema_version += 1` 并更新本文档。
- 删除/重命名事件类型：禁止（向后兼容；用新 type 替代并保留旧 type 占位）。

---

## 附：版本历史

| 版本 | 日期 | 变更 |
|---|---|---|
| v1 | 2026-08-21 | 冻结：14 事件类型 + 信封 v1 + 交互协议（后端任务书 #01 B1-3） |
| v1.1 | 2026-08-22 | 补注 `plan_draft` 事件定义（B2 引入未入契约的滞后修正；新增类型不升版本）+ `need_approval.payload`（B3-A 澄清问题内容，字段新增不升版本） |
| v1.2 | 2026-08-22 | 补注 `sandbox_violation` 事件（RT4 安全纵深——子进程被 KILL/限制时的事实；新增类型不升版本） |
