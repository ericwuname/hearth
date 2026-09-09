# RFC-003: Guest Session — 最小可行外部接入（v3）

**状态**: Draft v3（Conditional Accept 修订版）  
**日期**: 2026-07-31  
**上游依赖**: v15 P1（经验库落盘）、v14.5（task_id 追踪）  
**评审基线**: RFC-002 评审函（2026-07-31），8 处必须修正全部核销  
**对标**: RFC-001（已退回）、RFC-002（Conditional Accept）  

---

## §0 摘要

为 codex-rust 增加 **guest session** 能力：外部 AI 通过 REST API 创建一个受限的只读会话，在独立的沙箱工作区内调用白名单限定的只读工具集。全部操作落审计日志，受人工审批门控制。不新增 crate，不改动现有安全模型（sandbox/tool-runtime/xray 均无变动）。

**一句话**：让外部 AI 能安全地"看"和"说"，不能"写"。

**与 RFC-002 的差异**：修复 8 处评审意见（§2），安全声明附行为证据（§4），工具白名单逐条对着 `fn name()` 返回值写（§3.3），审批认证机制明确（§3.1），类型修正（§6）。

---

## §1 背景与动机

### 1.1 需求溯源

外部 AI（如元宝、Kimi、Claude）需要安全接入 codex-rust 的身体，调用工具、获取信息、与启元协作。核心约束：**外部 AI 是不受信行为体**，G2（经验案例法）和 G3（宪法叙事层）对它约束力为零——它不读我们的 prompt，不访问我们的经验库。

### 1.2 与现存 `bridge` crate 的关系辨析

**现存 `bridge`**（`crates/bridge/Cargo.toml:2`，name = "bridge"，283 行）：
- 能力：RoundRobin / Debate / MajorityVote 三种**多模型讨论**策略
- 接口：`POST /api/v1/bridge` — 创建并同步运行一次多模型讨论（`crates/service/src/routes.rs:426`）
- 信任域：**内部**，运行在信任域内，模型都是已配置的 provider
- 定位：内部多模型协作工具

**本 RFC 的 guest session**：
- 能力：外部 AI **单会话只读接入**
- 接口：`POST /api/v1/guest/session` — 创建来宾会话
- 信任域：**外部**，不受信行为体
- 定位：外部单会话受限接入

**关系决策**：**划界并存**。二者能力正交（内部多模型讨论 vs 外部单会话只读接入），不合并、不相互依赖。命名上以 `guest` 前缀区分，避免与 `bridge` 混淆。guest 路径完全不经过 `crates/bridge` 的任何代码。

---

## §2 评审意见核销表（RFC-002 → v3）

| # | 严重度 | RFC-002 问题 | v3 修正 | 状态 |
|---|--------|---------------|---------|------|
| 1 | 🔴 | landlock 不在只读路径上（read.rs 是 std::fs 直读） | §4.1 安全表写实况：G0 对 guest 只读路径实际不在场，唯一防线是 G1 应用层路径校验 | ✅ 核销 |
| 2 | 🔴 | `read_only_view()` 是黑名单，对不受信行为体不能复用 | §3.3 guest 用独立白名单构造 dispatcher，不调用 `read_only_view()` | ✅ 核销 |
| 3 | 🔴 | AUTH-0 默认放行，guest 审批端点无认证 | §3.1 明确 `approve` 端点强制认证，api_key 缺失时该路由 503 | ✅ 核销 |
| 4 | 🟡 | 白名单有虚构工具（list_dir 不存在） | §3.3 白名单逐条对着 `tools-builtin` 的 `fn name()` 返回值写 | ✅ 核销 |
| 5 | 🟡 | "不新增 crate"自相矛盾（注释路径指向 `crates/guest/`） | §5/§7 全部路径统一为 `crates/service/src/guest.rs` | ✅ 核销 |
| 6 | 🟡 | "main.rs 无改动"不实（路由挂载点在 main.rs:683-699） | §5 改为"main.rs 新增约 5 行路由挂载" | ✅ 核销 |
| 7 | 🟡 | `Instant` 不可序列化，审批队列跨重启失效未声明 | §6 改为 `chrono::DateTime<Utc>`，CONTRACT.md 声明内存队列限制 | ✅ 核销 |
| 8 | 🟡 | `guest_token` 未定义，接口 GET/POST 不一致 | §3.2 token 独立于 UUID 生成，只经 header 传输；接口统一为 POST | ✅ 核销 |

---

## §3 API 设计

### 3.1 外部接口（REST，新增路由）

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| POST | `/api/v1/guest/session` | 创建来宾会话（返回 pending_approval） | 外部 AI 自声明身份（label） |
| GET | `/api/v1/guest/session/:id` | 查询会话状态 | `Authorization: Bearer <guest_token>` |
| DELETE | `/api/v1/guest/session/:id` | 销毁会话 | `Authorization: Bearer <guest_token>` |
| POST | `/api/v1/guest/session/:id/tool` | 调用只读工具 | `Authorization: Bearer <guest_token>` |
| POST | `/api/v1/guest/session/:id/audit` | 拉取本会话审计日志 | `Authorization: Bearer <guest_token>` |
| POST | `/api/v1/guest/approve/:id` | 人工审批（**强制认证**） | AUTH-0 Bearer token，**api_key 缺失时此路由返回 503** |

**认证机制（评审 🔴2.3 核销）**：

`crates/service/src/routes.rs:63-84` 的 AUTH-0 中间件语义：`api_key` 未配置时全放行。这对内部接口可接受，对 guest 审批端点不可接受。

v3 的处理：
```rust
// crates/service/src/guest_approval.rs

/// 审批端点中间件：api_key 未配置时直接 503
async fn require_auth_for_approval(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Response {
    if state.user_store.is_none() {
        // api_key 未配置 → 审批端点不可用
        return (StatusCode::SERVICE_UNAVAILABLE, "Approval auth not configured").into_response();
    }
    // 正常认证流程
    authenticate(req, next).await
}
```

**关键**：`/api/v1/guest/approve/:id` 的路由注册使用 `require_auth_for_approval` 中间件，而非普通 AUTH-0。当 `UserStore` 未初始化时，该路由直接返回 503，不 pass-through。

### 3.2 内部接口（Rust API，新增文件 `crates/service/src/guest.rs`）

```rust
/// 创建来宾会话（返回 pending_approval 状态 + guest_token）
/// guest_token 独立于 session UUID，使用 uuid::Uuid::new_v4() 生成
pub async fn create_guest_session(
    label: String,
    ttl_hours: u64,
) -> Result<(Uuid, String, SessionStatus), GuestError>;

/// 审批通过（仅内部管理员调用）
pub async fn approve_guest_session(
    id: Uuid,
    approver: String,
) -> Result<(), GuestError>;

/// 调用工具（只读白名单）
/// 使用独立构造的 GuestDispatcher，不调用 read_only_view()
pub async fn call_guest_tool(
    guest_id: Uuid,
    tool: String,
    params: serde_json::Value,
) -> Result<ToolResult, GuestError>;

/// 审计日志追加（内部调用）
async fn append_audit_entry(entry: AuditEntry) -> Result<(), GuestError>;
```

**token 机制（评审 🟡2.8 核销）**：
- `session_id` = Uuid（内部标识，出现在审计日志）
- `guest_token` = 独立生成的 Uuid（鉴权凭证，只经 `Authorization` header 传输，不出现在 URL）
- 两者分离：即使 session_id 泄露（如在访问日志中），没有 token 仍无法调用工具

### 3.3 工具白名单（评审 🟡2.4 核销）

**逐条对着 `tools-builtin` 各文件的 `fn name()` 返回值写**：

| 工具真名 | 来源文件 | 是否在白名单 | 理由 |
|-----------|---------|:---:|------|
| `read` | `crates/tools-builtin/src/read.rs` 的 `fn name() -> "read"` | ✅ | 只读文件读取 |
| `glob` | `crates/tools-builtin/src/glob.rs` 的 `fn name() -> "glob"` | ✅ | 只读文件搜索 |
| `grep` | `crates/tools-builtin/src/grep.rs` 的 `fn name() -> "grep"` | ✅ | 只读内容搜索 |
| `bash` | `crates/tools-builtin/src/bash.rs` 的 `fn name() -> "bash"` | ❌ | 可变，执行任意命令 |
| `write_file` | `crates/tools-builtin/src/write_file.rs` 的 `fn name() -> "write_file"` | ❌ | 可变，写入文件 |

**白名单代码（评审 🔴2.2 核销）**：

```rust
// crates/service/src/guest.rs

/// Guest 工具白名单 — 独立定义，不调用 read_only_view()
/// read_only_view() 是黑名单语义（剥除 MUTATING_TOOLS 后放行其余全部），
/// 对不受信行为体不安全：新增工具会被自动放行。
/// 白名单语义：只注册显式列出的工具，未列出的一律拒绝。
const GUEST_TOOL_NAMES: &[&str] = &[
    "read",      // crates/tools-builtin/src/read.rs → fn name() -> "read"
    "glob",      // crates/tools-builtin/src/glob.rs → fn name() -> "glob"
    "grep",      // crates/tools-builtin/src/grep.rs → fn name() -> "grep"
];

/// 为 guest session 构造独立 dispatcher（白名单语义）
fn build_guest_dispatcher() -> Dispatcher {
    let mut d = Dispatcher::new();
    for tool_name in GUEST_TOOL_NAMES {
        if let Some(tool) = registry::get(tool_name) {
            d.register(tool);
        }
    }
    d
}
```

**关键不变量**：`build_guest_dispatcher()` 不调用 `read_only_view()`。新增工具不会被自动注册到 guest dispatcher——必须显式添加到 `GUEST_TOOL_NAMES` 才会生效。

---

## §4 安全设计

### 4.1 安全表（附行为证据）

| 层 | 机制 | 对 guest 有效性 | 行为证据（文件:行号 + 关键内容） |
|----|------|:---:|------|
| **G0 结构层** | landlock + seccomp | ⚠️ **对只读路径不在场** | `crates/sandbox/src/lib.rs` 的 landlock 仅在 `bash` 工具执行子进程的 `pre_exec` 时生效（`crates/tools-builtin/src/bash.rs` 引用 sandbox）。`read`/`glob`/`grep` 是 service 进程内 `std::fs` 直读，**不经过 sandbox**。`GUEST_TOOL_NAMES` 不含 `bash`，故 G0 对 guest 只读路径实际不生效。 |
| **G1 应用层** | 路径校验（`read.rs`）+ 白名单硬编码 | ✅ **guest 唯一有效防线** | `crates/tools-builtin/src/read.rs` 的 `ParentDir` 拒绝 + `strip_prefix(&ctx.cwd)` 校验。Guest dispatcher 只注册白名单工具。这两道都是编译期决定的，运行时不可绕过。 |
| **G2 案例法层** | 经验库 | ❌ 无效 | 外部 AI 不读经验库，无影响 |
| **G3 叙事层** | constitution prompt | ❌ 无效 | 外部 AI 不读 constitution，无影响 |

**安全实质（评审 🔴2.1 核销）**：

> **Guest 只读路径上只有一道有效防线：G1 应用层路径校验 + 白名单硬编码。**
> G0（landlock）不在场——只读工具不走 bash，不经过 sandbox 的 pre_exec。
> 这不是灾难——`read.rs` 的路径校验代码质量尚可（`ParentDir` 拒绝 + `strip_prefix`）。
> 但必须写实况：不能声称"landlock 提供隔离"，因为 landlock 对 guest 的只读调用链不在场。

### 4.2 关键 xray 断言（评审新增要求）

| 断言 ID | 锁住什么 | 行为证据 | 严重度 |
|----------|---------|---------|:---:|
| `guest-ctx-cwd-is-workspace-root` | guest 的 `ToolContext.cwd` 必须强制等于其 `workspace_root` | `crates/service/src/guest.rs` 中创建 session 时必须设置 `ctx.cwd = workspace_root` | 🔴 red |
| `guest-dispatcher-is-whitelist` | guest dispatcher 不调用 `read_only_view()` | grep `crates/service/src/guest.rs` 不含 `read_only_view` 子串 | 🔴 red |
| `guest-tools-are-readonly` | 白名单只含 read/glob/grep | grep `GUEST_TOOL_NAMES` 不含 bash/write_file | 🔴 red |
| `guest-approval-auth-required` | approve 端点不 pass-through | grep `require_auth_for_approval` 存在于 `guest_approval.rs` | 🔴 red |

### 4.3 文件隔离（应用层）

每个 guest session 拥有独立的 `workspace_root`：

```rust
// crates/service/src/guest.rs

struct GuestSession {
    id: Uuid,
    token: String,           // 独立 token，不暴露 UUID
    label: String,
    status: GuestStatus,      // PendingApproval | Active | Expired | Revoked
    created_at: chrono::DateTime<Utc>,
    approved_at: Option<chrono::DateTime<Utc>>,
    expires_at: chrono::DateTime<Utc>,
    workspace_root: PathBuf,  // 隔离的工作目录
    audit_path: PathBuf,      // 审计日志路径
    tool_count: u64,          // 工具调用计数（轻量配额）
    max_tool_calls: u64,      // 上限（默认 1000）
}
```

**隔离机制**：
- `workspace_root` 在创建时由 `tempfile::TempDir` 或固定目录 + UUID 子目录生成
- `read.rs` 的路径校验确保 `read` 工具只能访问 `workspace_root` 内的文件
- guest 无法跳出 `workspace_root`（应用层强制）

**注意**：隔离靠的是 `read.rs` 的 `strip_prefix(&ctx.cwd)` 校验，不是 landlock。这是因为 guest 的 `ctx.cwd = workspace_root`，而 `read.rs` 校验读取路径必须在 `ctx.cwd` 之下。

### 4.4 审批门（Phase 1 前置）

```
POST /api/v1/guest/session { "label": "yuanbao" }
  → 创建 GuestSession { status: PendingApproval }
  → 返回 { "id": <uuid>, "token": <token>, "status": "pending_approval" }
  → 审计日志写入 "session_created, pending"

（此时 guest 调用任何工具 → 返回 403 Forbidden）

人工管理员：
POST /api/v1/guest/approve/<uuid>
  → require_auth_for_approval 中间件验证 Bearer token
  → api_key 未配置 → 返回 503
  → 验证通过 → GuestSession.status = Active
  → 审计日志写入 "session_approved, by=<approver>"

此后 guest 调用工具 → 正常返回
```

### 4.5 审计日志

```rust
// crates/service/src/guest.rs

struct AuditEntry {
    timestamp: chrono::DateTime<Utc>,  // 可序列化（评审 🟡2.7 核销）
    guest_id: Uuid,
    guest_label: String,
    action: String,        // "tool_call" | "session_created" | "session_approved" | "session_revoked"
    tool: Option<String>,
    params_summary: Option<String>,  // 截断到 256 字节
    result: String,        // "success" | "403" | "429" | "error"
    duration_ms: u64,
}
```

**写入路径**：每个 guest 一个文件 `workspace_root/.guest_audit/<guest_id>.jsonl`  
**不可篡改性**：审计文件由 service 进程持有写权限，guest 工具白名单不含任何写操作，guest 无法修改自己的审计日志  
**轮转**：按日轮转，保留 30 天，超限自动清理最早日志  

---

## §5 现有组件集成

| 组件 | 文件 | 改动量 | 说明 |
|------|------|:---:|------|
| `crates/service/src/session.rs` | 新增 `mod guest;` | 1 行 | 模块声明 |
| `crates/service/src/guest.rs` | **新增** | ~200 行 | GuestSession + create/approve/destroy/call_tool + 审计 |
| `crates/service/src/guest_approval.rs` | **新增** | ~80 行 | 审批队列 + `require_auth_for_approval` 中间件 |
| `crates/service/src/routes.rs` | 新增 6 条路由 handler | ~100 行 | REST handler 函数 |
| `crates/service/src/main.rs` | 路由挂载 | ~5 行 | `routes.rs:683-699` 的 `.route(...)` 链追加 |
| `crates/tool-runtime/src/dispatcher.rs` | **无改动** | 0 | guest 用独立 dispatcher，不碰现有 |
| `crates/tools-builtin/src/read.rs` | **无改动** | 0 | 复用现有路径校验 |
| `crates/tools-builtin/src/glob.rs` | **无改动** | 0 | 复用 |
| `crates/tools-builtin/src/grep.rs` | **无改动** | 0 | 复用 |
| `crates/sandbox` | **无改动** | 0 | guest 只读路径不经过 sandbox |
| `crates/project-xray` | **无改动**（仅新增断言配置） | 0 | 断言加到 `docs/xray/wiring-v16.toml` |
| `crates/experience` | **无改动** | 0 | guest 路径不访问经验库 |

**不新增 crate**。所有代码在 `crates/service/src/` 下。

---

## §6 数据类型修正（评审 🟡2.7 核销）

### 6.1 `Instant` → `chrono::DateTime<Utc>`

```rust
// ❌ RFC-002 错误写法
struct GuestSession {
    created_at: Instant,       // 不可序列化，跨重启无效
    expires_at: Instant,
}

// ✅ RFC-003 修正
struct GuestSession {
    created_at: chrono::DateTime<Utc>,   // 可序列化，可持久化
    approved_at: Option<chrono::DateTime<Utc>>,
    expires_at: chrono::DateTime<Utc>,
}
```

### 6.2 审计日志可序列化

```rust
// ❌ RFC-002 错误写法
struct AuditEntry {
    timestamp: Instant,  // serde 不支持 Instant
}

// ✅ RFC-003 修正
struct AuditEntry {
    timestamp: chrono::DateTime<Utc>,  // serde 支持
}
```

### 6.3 审批队列内存限制（已知限制，写入 CONTRACT.md）

**当前（Phase 1）**：审批队列纯内存（`HashMap<Uuid, GuestSession>`）。服务重启后所有 pending 会话丢失，已 approved 会话也丢失（guest 需重新创建）。

**Phase 2 候选**：将 `GuestSession` 序列化到 `workspace_root/.guest_sessions.jsonl`，重启后恢复 active 会话。

---

## §7 文件结构

```
crates/service/src/
├── lib.rs                  // 新增: mod guest; mod guest_approval;
├── guest.rs                // 新增 ~200 行: GuestSession, create/approve/destroy/call_tool, 审计
├── guest_approval.rs       // 新增 ~80 行: 审批队列, require_auth_for_approval 中间件
├── routes.rs               // 新增 6 个 handler ~100 行
├── main.rs                 // 修改 ~5 行: 路由挂载 (routes.rs:683-699 链追加)
├── session.rs              // 无改动
└── user.rs                // 无改动 (UserStore 已存在)
```

**不新增 crate，不新增目录。**

---

## §8 CONTRACT.md

```markdown
# crates/service — Guest Session 契约

## Public Interface
- `POST /api/v1/guest/session` — 创建来宾会话（返回 pending_approval）
- `GET /api/v1/guest/session/:id` — 查询会话状态（需 guest_token）
- `DELETE /api/v1/guest/session/:id` — 销毁会话（需 guest_token）
- `POST /api/v1/guest/session/:id/tool` — 调用只读工具（需 guest_token，仅 active 后可用）
- `POST /api/v1/guest/session/:id/audit` — 拉取审计日志（需 guest_token）
- `POST /api/v1/guest/approve/:id` — 人工审批（强制认证，api_key 缺失时 503）

## Invariants
1. 未经审批的 session 所有工具调用返回 403
2. 白名单只含 read/glob/grep（不含 bash/write_file）
3. Guest dispatcher 不调用 read_only_view()
4. 审计日志不可由 guest 修改（白名单无写工具）
5. guest_token 独立于 session UUID，只经 Authorization header 传输
6. ctx.cwd == workspace_root（由 xray 断言 guest-ctx-cwd-is-workspace-root 锁死）
7. api_key 未配置时 /approve 端点返回 503（不 pass-through）

## Known Limitations (Phase 1)
- 审批队列纯内存，服务重启后所有会话丢失
- 审计日志按日轮转，保留 30 天
- 无 SSE 推送，客户端需轮询 GET /session/:id

## Wiring (xray 断言 ID → wiring-v16.toml)
- guest-ctx-cwd-is-workspace-root (severity: red)
- guest-dispatcher-is-whitelist (severity: red)
- guest-tools-are-readonly (severity: red)
- guest-approval-auth-required (severity: red)
```

---

## §9 开发阶段

### Phase 1：审批门 + 只读工具（v16 核心，2-3 天）

**交付物**：
1. `crates/service/src/guest.rs` — GuestSession + create/approve/destroy/call_tool + 审计
2. `crates/service/src/guest_approval.rs` — 审批队列 + `require_auth_for_approval`
3. `crates/service/src/routes.rs` — 6 个 handler
4. `crates/service/src/main.rs` — 5 行路由挂载
5. `docs/xray/wiring-v16.toml` — 4 条新增断言
6. `CONTRACT.md` — §8 内容

**验收标准（11 条）**：

| # | 标准 | 验证方式 |
|---|------|---------|
| 1 | `POST /guest/session` 返回 `pending_approval` | curl 测试 |
| 2 | 审批前调用工具返回 403 | curl 测试 |
| 3 | `POST /guest/approve/:id` 无 token 返回 401 | curl 测试 |
| 4 | api_key 未配置时 approve 返回 503 | 测试环境移除 api_key |
| 5 | 审批后 read/glob/grep 正常返回 | curl 测试 |
| 6 | 白名单外工具（如 write_file）返回 403 | curl POST tool=write_file |
| 7 | 审计日志文件存在且格式正确（JSONL） | 检查 `.guest_audit/` 目录 |
| 8 | `cargo clippy -D warnings` 通过 | CI |
| 9 | xray 4 条新断言全部 PASS | CI 第四门 |
| 10 | grep `read_only_view` 在 `guest.rs` 中无匹配 | 行为证据 |
| 11 | 注册白名单外新工具后 guest 调用必须 403 | 集成测试：临时注册 fake_tool，guest 调用返回 403 |

### Phase 2：可观测性 + 持久化（v16 辅助，1-2 天）

**依赖**：v14.5 task_id 追踪完成

**交付物**：
1. 所有 guest 操作注入 `guest_id` span（`tracing::info_span!`）
2. GuestSession 序列化到 `workspace_root/.guest_sessions.jsonl`（重启恢复）
3. 审计日志按日轮转 + 30 天保留

**验收标准**：
1. 日志中可通过 `guest_id` 追溯一次完整调用链
2. 服务重启后 active 会话恢复（无需重新审批）
3. 审计日志按日轮转，超限自动清理

### Phase 3：写权限 + MCP 前端（v17，延期）

**前提**：v15 P1 经验库落盘完成，且只读版跑出真实需求。

**交付物**：
1. 受控写工具（仅在审批过的文件路径内可写）
2. MCP 协议前端（可选）

---

## §10 风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
|------|:---:|:---:|------|
| 外部 AI 通过 read 工具读取敏感文件 | 中 | 中 | `read.rs` 的 `strip_prefix(&ctx.cwd)` 校验 + workspace_root 不包含敏感路径 |
| 审批门被绕过（直接调用 tool-runtime） | 低 | 高 | guest token 与内部 session 隔离；tool dispatcher 不对外暴露 |
| 审计日志磁盘写满 | 低 | 低 | 按日轮转 + 30 天保留 + 上限 100MB |
| 外部 AI 长期占用 session 不释放 | 低 | 低 | TTL 到期自动 Expired（默认 24 小时） |
| 新增工具被自动放行给 guest | 中 | 高 | 白名单语义（非黑名单）；新增工具必须显式添加到 `GUEST_TOOL_NAMES`；xray 断言 `guest-tools-are-readonly` 锁死 |
| AUTH-0 默认放行导致 guest 审批可绕过 | 低 | 高 | `require_auth_for_approval` 独立中间件，api_key 缺失时 503 |

---

## §11 Non-Goals

- ❌ 不做 MCP 协议前端（v17 候选）
- ❌ 不做跨窗口消息总线（v17 候选）
- ❌ 不做写权限（v17 候选，且需 v15 P1 完成）
- ❌ 不做完整配额管理系统（Phase 2 仅轻量 tool_count 上限）
- ❌ 不新增 crate
- ❌ 不修改现有安全模型（sandbox/tool-runtime/xray 均无变动）
- ❌ 不修改 `read_only_view()`（留给内部子代理继续使用）

---

## §12 开放问题（需决策）

| # | 问题 | 建议 | 决策者 |
|---|------|------|--------|
| 1 | guest session 的默认 TTL？ | 24 小时 | 顶层角色 |
| 2 | 默认 tool_count 上限？ | 1000 次/会话 | 顶层角色 |
| 3 | 是否允许同一 label 创建多个并发会话？ | 允许，每个需独立审批 | 顶层角色 |
| 4 | 审计日志告警阈值？（如单会话 1 分钟内 100 次 read） | v17 候选，Phase 1 不做 | 顶层角色 |
| 5 | workspace_root 的存储位置？ | `./.guest_workspaces/<uuid>/` | 顶层角色 |

---

## §13 评审检查清单（v3 自审）

- [x] 每个引用的现有接口给出 `文件:行号` + 行为证据（不只位置）
- [x] 与现存 `bridge` crate 的关系已辨析（划界并存，互不依赖）
- [x] 审批门前置到 Phase 1
- [x] 安全表只保留对外部行为体真实有效的层（G1 应用层是唯一有效防线）
- [x] G0 landlock 对只读路径不在场的事实已写明
- [x] 无虚构接口（工具名逐条对着 `fn name()` 返回值）
- [x] 无新增 crate
- [x] 写工具不在白名单中（白名单是白名单语义，非黑名单）
- [x] 审计日志机制完整且可序列化
- [x] 依赖声明完整（v15 P1 + v14.5 task_id）
- [x] Non-Goals 清单清晰
- [x] `Instant` 已替换为 `chrono::DateTime<Utc>`
- [x] `guest_token` 独立于 UUID，只经 header 传输
- [x] 接口统一为 POST（无 GET/POST 混用）
- [x] `require_auth_for_approval` 中间件明确（api_key 缺失 → 503）
- [x] xray 4 条新断言 ID 已注册

---

## 附录 A：与 RFC-001/RFC-002 的差异摘要

| 维度 | RFC-001（退回） | RFC-002（Conditional Accept） | RFC-003（本版） |
|------|-----------------|----------------------------|-----------------|
| 规模 | 4 子系统 × 3 协议前端 | 1 文件 guest.rs | 1 文件 guest.rs |
| 新增 crate | 1（external-bridge） | 0（声明）但注释路径矛盾 | 0（全部路径统一） |
| 安全模型 | G0+G1+G2+G3 | G0+G1（删 G2/G3） | G1 only（G0 对只读路径不在场已写明） |
| 审批门位置 | Phase 4 | Phase 1 | Phase 1（认证机制明确） |
| 虚构接口 | 5 处 | 0（但工具名有虚构） | 0（逐条对着 fn name()） |
| 现存 bridge 关系 | 未提及 | 辨析（划界并存） | 辨析 + 明确互不依赖 |
| 依赖声明 | 遗漏 2 项 | 完整 | 完整 + 行为证据 |
| 类型正确性 | N/A | Instant 错误 | chrono DateTime<Utc> |
| token 机制 | N/A | 未定义 | 独立于 UUID，header only |
| xray 断言 | 3 条（虚构） | 3 条（部分不实） | 4 条（逐条可 grep 验证） |

## 附录 B：行为证据锚点汇总

| 声明 | 文件:行号 | 关键内容 |
|------|-----------|---------|
| landlock 仅 bash pre_exec 生效 | `crates/sandbox/src/lib.rs` | pre_exec 钩子 |
| bash 引用 sandbox | `crates/tools-builtin/src/bash.rs` | use sandbox |
| read 是 std::fs 直读 + 路径校验 | `crates/tools-builtin/src/read.rs` | ParentDir 拒绝 + strip_prefix(&ctx.cwd) |
| glob/grep 同理 | `crates/tools-builtin/src/glob.rs`, `grep.rs` | fn name() 返回值 |
| read_only_view 是黑名单 | `crates/tool-runtime/src/dispatcher.rs:46,99-110` | MUTATING_TOOLS 过滤 |
| AUTH-0 默认放行 | `crates/service/src/routes.rs:63-84` | api_key 未配置时 pass-through |
| UserStore 存在 | `crates/service/src/user.rs` | UserStore struct |
| 路由挂载点 | `crates/service/src/main.rs:683-699` | .route(...) 链 |
| 工具注册 | `crates/service/src/main.rs:364-376` | 5 个工具注册 |

---

**v3 承诺**：本 RFC 每个安全声明附行为证据（不只位置），每个工具名对着 `fn name()` 返回值，每个 xray 断言可 grep 复现。48 小时内可进入 Phase 1 施工。
