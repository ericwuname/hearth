# v23 顶层设计规划 · 三端口工程化

> **文件性质**：顶层设计与派工规划。发给**拆解层**（拆任务书与排期）与**执行层**（照办、跑门禁、交证据）。
> **基线**：HEAD `1255498` · 最新 tag `v22.0`（`8e39775`）· 24 crates · 214 tests · wiring 15/15
> **日期**：2026-08-02
> **前置文档**：`docs/requirements-ledger.md`（功能需求总账）· `docs/frontend-backend-split.md`（契约草案）· `docs/observer-os-design.md`（Observer 定版）· `docs/codex-desktop-demo.html`（v12 三端口原型）· **`docs/audit-findings-v22.md`（12 项 🔴，其 RT4 是 WP-0 的阻塞前置）**
>
> **本轮核心决策**：AI OS 与 Observer OS **立即开工**；Human OS **保持未定版**。
> 二者能并行的前提是先做一次架构动作（WP-0），**不做这个动作，前端每变一次后端就要动一次刀**。

---

## §1 核心问题裁决：Human OS 未定版会不会拖累后端

### 1.1 结论

**不会 —— 但当前后端形态会。** 这是两件事，必须分开。

风险不在"UI 没定"，在于**当前的人类中断机制是一次性定制的**。源码事实（已核）：

| 位置 | 现状 | 后果 |
|---|---|---|
| `crates/api/src/lib.rs:13` | `AgentEvent` 仅 8 变体：`Phase/Token/ToolCall/ToolResult/NeedApproval/Reflection/Done/Error` | 无信封字段（`ts`/`seq`/`span_id`/`parent_id`/`schema_version`），无法表达 span 树 |
| `crates/agent-core/src/loop.rs:41,46` | `Event::NeedApproval { id, action }` | 审批是**专用变体**，不是通用中断 |
| `crates/tool-runtime/src/dispatcher.rs:32,58` | `ApprovalState{NotRequired,Pending,Approved,Denied}` + 按 session 键控的 approval map | **暂停/续跑机制只为审批而生** |
| `crates/service/src/session.rs:594,602` | `submit_approval` / `resolve_approval` 专用路径 | 每种人类介入 = 一套独立管道 |

**推论（重要）**：当前架构**根本无法表达"规划期问人一个问题"**——澄清在后端不存在。
每新增一种人类介入，都要改 enum + 加状态机 + 加路由 + 在内核开新暂停点。**这就是 UI 变更穿透后端的真实机制。**

### 1.2 UI 变更四分类：只有两类会穿透

| 类 | 定义 | 例子 | 穿透后端 | 防御手段 |
|---|---|---|---|---|
| **A 呈现变更** | 同样的事实换个看法 | 配色、布局、折叠策略、动画、图标 | ❌ 零 | 公理 1.2「BE 给语义 FE 给呈现」 |
| **B 新视图复用旧事实** | 新界面但不要新数据 | 时间轴、diff 视图、搜索过滤、按 span 分组 | ❌ 零 | **事实粒度足够细 + 可回放** |
| **C 需要新事实** | 要后端没记的东西 | 每步 token 成本、文件级 diff、细粒度进度 | ⚠️ 中 | **超集原则 + 只增不改** |
| **D 需要新的人类介入点** | 新的暂停/续跑控制流 | 中途改计划、选执行分支、接管操作、给结果打分 | 🔴 **高** | **WP-0 通用交互原语** |

- **C 类防御 = 超集原则**：后端 emit 的事实**必须多于当前 UI 展示的**。宁可前端忽略字段，不要后端事后补录（补录 = 历史数据永久残缺）。
- **D 类防御 = WP-0**，见 §2。这是本规划唯一的架构性动作。

### 1.3 量化结论

| 后端工作 | 是否被 Human OS 阻塞 |
|---|---|
| 事件信封 / span 树 / SSE / 录制重放 | ❌ 不阻塞 |
| **规划缺口推导器**（gap 三元组 + auto_assumed） | ❌ 不阻塞（gap 是**事实**不是 UI；怎么展示才是 UI） |
| Observer 全部（crate / 四锁 / 指标 / 规则 / 熔断） | ❌ 不阻塞 |
| 产物登记 | ❌ 不阻塞 |
| 通用交互原语本身 | ❌ 不阻塞 |
| 具体有哪些 interaction kind | ⚠️ 部分（但**有原语就不阻塞内核**，只影响前端与 payload 定义） |
| `think_summary` 的语气/长度风格 | ✅ 阻塞（但属打磨，P2） |
| 桌面壳 `open_artifact` 的具体行为 | ✅ 轻度阻塞（可等） |

> **约 95% 的后端工作现在即可开工**，前提是 **WP-0 必须先落地**。

---

## §2 WP-0：通用人类交互原语（本规划唯一的架构动作，最高优先级）

### 2.1 设计

不再为每种交互定制。内核只认**一个**原语：

```rust
// 建议落点：crates/agent-types/src/interaction.rs
pub struct InteractionRequest {
    pub id: String,
    pub kind: String,               // "clarification" | "approval" | 未来任意
    pub phase: LoopPhase,           // 发生在哪个相位
    pub blocking: bool,             // 不答能否继续
    pub timeout: Option<Duration>,
    pub on_timeout: TimeoutPolicy,  // Assume(Value) | Reject | Abort
    pub payload: Value,             // kind 专属；【内核不解释】
    pub schema_version: String,
}

pub struct InteractionResponse {
    pub id: String,
    pub by: Responder,              // Human | Policy(String)
    pub payload: Value,             // 【内核不解释】
    pub latency_ms: u64,            // Z-16：人类决策耗时，前端采集即回传
}
```

**铁律：内核只认 `blocking` / `timeout` / `id`，绝不 match `kind`，绝不解析 `payload`。**
任何 `match kind { "approval" => ..., "clarification" => ... }` 出现在 `agent-core` 或 `tool-runtime` 中，即判 WP-0 失败。

### 2.2 为什么这一个动作能解决 D 类风险

| 未来 Human OS 想加 | 有 WP-0 | 无 WP-0 |
|---|---|---|
| 中途改计划 | 加一个 kind + 前端渲染 | 改 Event enum + 新状态机 + 新路由 + 内核新暂停点 |
| 选执行分支 | 同上 | 同上 |
| 人工接管某一步 | 同上 | 同上 |
| 结果打分 / 反馈 | 同上 | 同上 |
| **内核改动量** | **0** | 每次一台手术 |

### 2.3 连带收益：Observer 也被泛化

象限②「人机交互」指标必须建在**通用原语**上，**禁止 match 具体 kind**：

| 指标 | 正确算法 | 错误算法（禁止） |
|---|---|---|
| `interrupt_count` | `count(InteractionRequest)` | `count(type=="need_approval") + count(type=="need_clarification")` |
| `wait_ratio` | `Σ response.latency_ms / ai_ms` | 前端本地累加 |
| `autonomy_rate` | `count(by=Policy)/count(all)` | 硬编码事件类型 |
| `clarify_skip_rate` | `kind=="clarification"` 的 payload 内 `skipped` 比例 | — |

> ⚠️ **demo v12 当前是硬编码 `type==='need_clarification'` 的**（原型允许）。执行层实现时**必须按通用原语重写**，否则每加一种交互 Observer 就漏统计。

### 2.4 ⚠️ 泛化前必须先修：审批原语本身是坏的（RT4）

**来自 `docs/audit-findings-v22.md` RT4（🔴 已确认）——这条直接决定 WP-0 的做法：**

| # | 缺陷 | 锚点 |
|---|---|---|
| ① | 默认 `API_KEY=None` → `require_api_key` **全放行** | `routes.rs:68-103`, `main.rs:526-545` |
| ② | CLI 发 `{"approved":bool}`，服务端 `ApprovalReq` 要 `decision:String` → **422，approve/deny 子命令完全不可用** | `api/src/lib.rs:69-72`, `codex-cli/src/client.rs:135-138`, `routes.rs:181-194` |
| ③ | **`approval_id` 不参与校验** | `dispatcher.rs:171-197` |

**架构判断：泛化一个坏原语 = 把 bug 泛化到所有交互类型。**

③ 尤其致命：现在只是"任意审批响应能顶替任意 pending 审批"；一旦泛化成通用原语，就变成**任意人类响应可以顶替任意 pending 请求**——回答 A 问题的内容被当成 B 问题的答案，且无法察觉。**这是 WP-0 的阻塞前置，不是可选项。**

因此 WP-0 **必须内含**三项修复：

| 修 | 要求 |
|---|---|
| **R1 id 强校验** | `InteractionResponse.id` 必须匹配一个 **pending** 请求，否则拒绝 |
| **R2 一次性消费** | 响应消费后 pending 项立即失效，**重复提交必须失败**（防重放） |
| **R3 契约单一** | 客户端与服务端**共用同一份类型定义**，消除 ② 的 `approved` / `decision` 双写 |

> ① 的鉴权默认值属于部署安全，不在 WP-0 范围，但**必须在 §8 挂账并单独修**——不要顺手改，避免 WP-0 范围失控。

### 2.5 迁移路径

1. 新增 `InteractionRequest/Response` 类型与通用暂停/续跑管道，**内含 R1/R2/R3**。
2. 把现有审批**改写为 `kind="approval"` 的一个实例**（`ApprovalState` 退化为通用等待态的特例）。
3. 新增 `POST /session/{id}/interaction/{iid}` 通用路由。
4. `POST /approval` 旧路由保留为薄适配层并标记 **deprecated** ——但**必须按 R3 修正契约**（当前 CLI 走这条路是 422，"保持兼容"等于保持坏掉，无意义）。
5. `codex-cli` 的 approve/deny 子命令随之修好（RT4-② 顺带闭环）。
6. 门禁：现有审批相关测试**全部不改一行**仍须通过；**并新增 R1/R2 的失败用例**（错 id 必拒、重复提交必拒）。

---

## §3 三道隔离层（Human OS 与后端之间的防火墙）

| 层 | 机制 | 防住什么 | 验证 |
|---|---|---|---|
| **I1 事实超集** | 后端 emit 的事实严格多于 UI 所需；宁可前端忽略，不要后端补录 | C 类穿透 | 事件字段数 ≥ 契约要求；新增 UI 需求时 grep 历史事件流能否满足 |
| **I2 通用交互原语** | WP-0 | D 类穿透 | `agent-core`/`tool-runtime` 内 grep 不到 `"approval"` / `"clarification"` 字面量 |
| **I3 单一写入通道** | 前端**只能**通过白名单写后端 | 前端绕过契约制造事实 | 路由白名单审计 |

**I3 写入白名单（除此之外 FE 一律只读）**：

| 通道 | 用途 |
|---|---|
| `POST /session` | 创建会话 |
| `POST /session/{id}/message` | 人类发言 |
| `POST /session/{id}/interaction/{iid}` | **所有**人类决策（含 latency_ms 回传） |
| `POST /session/{id}/artifact/open` | 请求打开产物（BE 执行） |
| `POST /session/{id}/cancel` | 中止 |

> 前端**没有**任何其他写口。新增写口必须走顶层批准 —— 这是防止"前端悄悄长出业务逻辑"的结构锁。

---

## §4 冻结 / 留口矩阵（执行层据此判断哪些能定死）

### 4.1 现在就定死（Human OS 再怎么变都不动）

| 项 | 理由 |
|---|---|
| 事件统一信封 `{schema_version, ts, seq, type, span_id, parent_id, data}` | 与 UI 无关 |
| span 树模型（`parent_id` + 真实 `t0/t1` + `status`） | 与 UI 无关 |
| **通用交互原语结构** | 正是为了抗变而设 |
| SSE 传输 + `Last-Event-ID` 断线续传 | 与 UI 无关 |
| 事件流录制 / 重放（JSONL） | 基础设施 |
| Observer 四锁 L1/L2/L3/L4 | 结构性保证 |
| Observer 指标**口径与算法** | 必须唯一，否则 G3 挂 |
| gap 三元组 `{from, why, blocking, assume}` | 是事实结构，不是 UI |

### 4.2 现在必须留口（禁止定死）

| 项 | 留口方式 |
|---|---|
| interaction 的 `kind` 集合 | 开放字符串，**不做 enum**（做成 Rust enum 就等于把 UI 焊死在内核里） |
| `payload` 内部结构 | 每个 kind 自带 `schema_version`；内核透传 |
| Observer 规则阈值 | **配置化下发**（`observer-rules.toml`），不硬编码，不放前端（G5） |
| 报告呈现形态（徽章/抽屉/toast） | BE 只给 `health` + Finding 结构 |
| 产物卡片展示字段 | BE emit 全量元数据，FE 选显 |
| `think_summary` 语气与长度 | 生成能力先做，风格参数化 |

### 4.3 现在不要做（会白干）

| 不要做 | 原因 |
|---|---|
| 任何针对特定 UI 组件的后端定制接口 | UI 未定版，必废 |
| 后端返回颜色 / HTML / 像素坐标 | 违反公理 1.2 |
| 为"当前 demo 长什么样"做的后端缓存或聚合 | B 类视图应由前端派生 |
| 把 interaction kind 写成 Rust enum | 4.2 已述 |
| 前端本地计算任何会进报告的数字 | 违反 G5；腐化起点 |

---

## §5 工作包与排期

### 5.1 工作包定义

| WP | 名称 | 内容 | 依赖 | 阻塞于 Human OS | 优先级 |
|---|---|---|---|---|---|
| **WP-0** | **通用交互原语** | §2 全部 + 审批迁移 + 旧路由适配 | — | ❌ | **P0 最高** |
| **WP-1** | 事件信封与 span 树 | 扩展 `AgentEvent` 为带信封的统一结构；`loop.rs` 相位/任务/子循环 emit `span_open/close`（`parent_id` + 真实 `t0/t1`） | — | ❌ | P0 |
| **WP-2** | SSE 出口与录制重放 | `schema_version`、`Last-Event-ID` 续传、JSONL 落盘 + 重放器 | WP-1 | ❌ | P0 |
| **WP-3** | 规划缺口推导器 | planner 产 `plan.draft` 后做缺口扫描，输出 gap 三元组 + `auto_assumed[]`；**禁止无 from/why 的通用问题** | WP-0 | ❌ | P0 |
| **WP-4** | 观察者地基 | **新建 `crates/observer`** + L1 只读抽头 + L2 fail-closed + 测试 | WP-2 | ❌ | P0 |
| **WP-5** | 确定性指标引擎 | 四象限全指标，**纯确定性代码，禁 LLM**；象限②建在通用原语上 | WP-4 | ❌ | P1 |
| **WP-6** | 规则引擎与 Finding | 规则配置化下发 + L3 证据强制 + L4 元观察 + 双投影报告 | WP-5 | ❌ | P1 |
| **WP-7** | G0 红线熔断 | 仅预算/沙箱/重试风暴；**只拉闸**；熔断入流可审 | WP-6 | ❌ | P1 |
| **WP-8** | 产物登记 | `artifact` emit（path/delta/kind）+ `open_artifact` 执行 | WP-1 | ⚠️ 轻度 | P1 |
| **WP-9** | 思考摘要 | span 关闭时生成 `think_summary`，风格参数化 | WP-1 | ⚠️ 风格待定 | P2 |
| **WP-10** | 前端接真流 | demo 换真实 SSE | 全部 | — | 最后 |

### 5.2 排期

```
WP-0 ──┬── WP-1 ── WP-2 ──┬── WP-4 ── WP-5 ── WP-6 ── WP-7 ──┐
       │                  │                                  ├── WP-10
       └── WP-3           └── WP-8                           │
                              WP-9 ─────────────────────────┘
```

- **第一批（可立即并行开工）**：WP-0 → 完成后 WP-1 与 WP-3 并行。
- **第二批**：WP-2 完成后 WP-4 与 WP-8 并行。
- **WP-9** 可随时插入，不阻塞任何人。
- **WP-10** 等 Human OS 定版后再做（**这是唯一真正等 Human OS 的工作包**）。

### 5.3 crate 落点

| WP | 落点 |
|---|---|
| WP-0 | `agent-types`（类型）+ `agent-core`（暂停/续跑）+ `service`（路由） |
| WP-1 | `api`（信封）+ `agent-core/loop.rs`（span 钩子）+ `service/session.rs`（映射） |
| WP-2 | `service/sse.rs` + 新增录制/重放工具 |
| WP-3 | `planner` |
| WP-4~7 | **新建 `crates/observer`** |
| WP-8 | `tool-runtime` + `service` |

---

## §6 门禁（每个 WP 的验收，不达标一律退回）

| WP | 门禁 |
|---|---|
| WP-0 | ① `agent-core`/`tool-runtime` grep 不到 `"approval"`/`"clarification"` 字面量分支；② 现有审批测试**零改动**全过；③ 新增一个 dummy kind，内核代码改动 = **0 行**；④ **R1**：错误 id 的响应必须被拒（有测试）；⑤ **R2**：同一 id 重复提交必须被拒（有测试）；⑥ **R3**：`codex-cli` approve/deny 端到端可用（当前是 422） |
| WP-1 | 嵌套子循环的 span 树可从事件流完整还原（深度 ≥ 3 的用例） |
| WP-2 | **G2 重放**：录制 JSONL 离线重放，渲染结果与实时一致；断线重连不丢事件 |
| WP-3 | 抽 10 个真实任务，gap **100% 带可追溯 `from` + 具体 `why`**；`blocking=false` 项 100% 带 `assume` 默认值。不满足 = 判"为问而问"退回 |
| WP-4 | ① observer **是独立 crate**；② 杀掉观察者，内核**拒绝继续执行**（有测试）；③ 内核中不存在 `observer.disable()` |
| WP-5 | 同一事件流跑两次，指标**字节级相同**；observer 内 grep 不到 LLM 调用 |
| WP-6 | ① 构造空 evidence 的 Finding **必须失败**；② **随机抽 10 条 Finding，100% 能凭 evidence 复现结论**（中立性总闸门，不达标判不中立退回） |
| WP-7 | 熔断只产生停机，**不改计划不换工具**；熔断动作本身在事件流中可查 |
| WP-8 | 前端**零文件系统调用**（grep 验证） |
| WP-10 | **A-08 总闸门：demo 零改动消费真实事件流即跑通** |

**全局门禁（每个 WP 都要过）**：`cargo fmt` = 0 · `cargo clippy` = 0 · 全部测试通过 · 现有 214 tests 不得回归。

**架构门禁（建议进 CI）**：

| 检查 | 命令形态 |
|---|---|
| G5 前端零阈值 | 前端源码 grep 业务阈值常量 → 有即红 |
| I2 内核不认 kind | `agent-core`/`tool-runtime` grep 交互 kind 字面量 → 有即红 |
| I3 写入白名单 | 路由清单 diff → 出现白名单外写口即红 |
| observer 无 LLM | `crates/observer` grep `llm` → 有即红 |

---

## §7 禁止清单（执行层不得自由发挥）

1. **禁止**把 Observer 塞进 `nervous-system`。后者 `NerveAction::{Simplify,ReduceSteps,Abandon,DeliverAndQuit}` 有连续干预执行权（调用点 `loop.rs:1589`），是交感神经；Observer 必须零执行权。**必须两个 crate、两条数据通路。**
2. **禁止**在 Observer 中调用 LLM 计算指标。LLM 只允许把**已算好的数字**改写成人话。
3. **禁止**内核 `match` interaction 的 `kind`。
4. **禁止**前端计算任何会进报告的数字（G5）。
5. **禁止**"前端先本地算着用、以后再挪后端"——解耦架构腐化的起点。
6. **禁止**改字段语义或删字段（major 变更需顶层批准）；只增不改。
7. **禁止**为无证据的 Finding 开后门（L3 无例外）。
8. **禁止**后端返回颜色 / HTML / 坐标。
9. **禁止**给 Observer 增加"拉闸"以外的执行权。
10. **禁止**在 Human OS 定版前做 WP-10 之外的任何 UI 定制后端接口。

---

## §8 挂账未决（拆解层遇到直接上报顶层，不要自行决定）

| # | 问题 | 现状 |
|---|---|---|
| Q1 | Observer 规则阈值持有方 | 已定方向：BE 配置化下发（`observer-rules.toml`），前端零阈值。**具体阈值数值待定** |
| Q2 | 规则能否按项目自定义 / 覆盖 | 未决 |
| Q3 | Observer 报告持久化位置；与现有 `observer/daily-*.jsonl`（**无消费者**的历史债）如何合并 | 未决 |
| Q4 | 时间轴 scrubber 是否需要真实回滚（需 BE 快照接口） | 未决，当前仅设计为只读回放 |
| Q5 | CLI/TUI 是否 v1 就要，还是仅保留契约能力 | 未决（`codex-cli` 已存在，注意 WP-0 迁移不要打破它） |
| Q6 | Human OS UI 定版时机 | 用户明确：**未定版，继续打磨** |
| Q7 | 熔断后的恢复流程（人工解除？重启会话？） | 未决 |
| Q8 | interaction 超时策略默认值（`on_timeout`） | 未决，建议 blocking 项默认 `Abort`，非 blocking 默认 `Assume` |
| Q9 | **RT4-① 默认 `API_KEY=None` 全放行** | 🔴 部署安全缺陷，**不并入 WP-0**（避免范围失控），需单独排修复 |
| Q10 | 本规划与 `docs/audit-findings-v22.md` 的 12 项 🔴 如何排序 | 未决。**建议：RT4-③（id 不校验）随 WP-0 修；RT9 重启会话 404 影响 WP-2 持久化设计，需在 WP-2 前定；其余走独立修复线** |
| Q11 | `/readyz` 状态冲突 | `MEMORY.md` 记「已真修」，但 RT1 指 `session.rs:128-133` 无 store 时返回 `Ok(vec![])` → 永远 200。**需复核，勿凭记忆下结论** |

---

## §9 给拆解层的交付要求

1. 按 §5 工作包逐个拆成任务书，每份任务书必须含：**精确 file:line 锚点**、patch 方案、验证命令、§6 对应门禁。
2. **WP-0 必须排在最前**，其余按 §5.2 依赖图排。
3. 每份任务书标注基线 commit（当前 `1255498`）。
4. 遇到 §8 未决问题**上报顶层**，不自行决定。
5. 任务书中的行号锚点**必须重新核对源码**——本文档引用的行号基于 `1255498`，若基线推进需重新 grep。

## §10 给执行层的纪律

1. **不信计划信源码**：本文档所有 file:line 均须下笔前重新核对，出入以源码为准。
2. 每个 WP 完成后交**证据**（测试输出、grep 结果、门禁日志），不是"已完成"的口头报告。
3. 现有 **214 tests 不得回归**，这是硬红线。
4. 真实验证环境是 Linux VM（Windows 验证无效）；VM 可达 crates.io **不可达 github.com**。
5. 不确定归属时，按 `docs/requirements-ledger.md` §3 灰区裁决表 Z-01~Z-18 判定；表里没有的**上报**，不要自己发明。
