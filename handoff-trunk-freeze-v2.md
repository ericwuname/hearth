# Trunk Freeze 执行方案 v2（handoff-to-executor，已修正）

> 基线：v2-gaps 定版
> 目标：关闭 5 项冻结阻塞/建议项（B1/B2/B3 + S1/S2），使主干可冻结为 v3.0
> 本版相对 `handoff-trunk-freeze.md` 修正了评审发现的 **3 🔴 + 2 🟡 + 文档不一致**
> 验收命令：`cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --all`

---

## 评审发现的原版阻塞（已修正）

| # | 原版问题 | 源码事实 | v2 修法 |
|---|---|---|---|
| 🔴1 | B1 假设 `Session` 有 `events` 字段 | `session.rs:18-39` 只有 `event_tx: broadcast::Sender<AgentEvent>`，**无 `events`** | 给 `Session` 加 `events: Vec<AgentEvent>`，在 9 个 `tx.send` 点同步 push |
| 🔴2 | B1 把 `inner.events` 当 `Vec<{event_type,payload}>` | `AgentEvent` 是 **enum**，无这两个字段 | `get_history` 用 `agent_event_kind(e)`(variant 名) + `serde_json::to_value(e)` 映射 |
| 🔴3 | B3 `#[openapi(paths(...))]` + `components` 含 `Budget` | handler 无 `#[utoipa::path]`；`SessionCreate.budget: Budget` 无 `ToSchema` | `ApiDoc` 只放 `components(schemas(...))` **不放 paths**；DTO 加 `ToSchema`，`agent_types::Budget` 也加 |
| 🟡1 | B1/B2 错误类型耦合 | B1 新路由返回 `(StatusCode,String)`，B2 改全部 handler 为 `(StatusCode,Json<ErrorResponse>)` | 合并一次改：`get_session_history` 直接用新错误类型 |
| 🟡2 | `require_api_key` 中间件签名 | `main.rs:236` 用 `from_fn_with_state`，返回 `Result<Response,(StatusCode,String)>` | Err 臂改 `api_err`，返回类型同步为 `(StatusCode,Json<ErrorResponse>)`（仍 `impl IntoResponse`） |
| ✅ S2 复核 | 原判 S2 需 LLM | `EvalRunner::run_all` 用 `MockProvider`（`eval.rs:310` `build_task_script`），本地跑；gate2.log:255 已 `ok` | S2 仅把 eval 显式暴露到 CI，无需 env gate |
| 文档 | 验收清单引用 G6/G7/T1/T2/T3 与不存在的 `trunk-freeze-branch-plan.md` | v2-gaps 已关 G1/G3/G4/G5/G9/G11 | 验收以本文件 B1..S2 打勾为准 |

---

## B1 — 历史回放端点（修正版）

### a) `crates/api/src/lib.rs` 加 DTO（在 `ModelInfo` 之后、`#[cfg(test)]` 之前）

```rust
/// B1 (trunk-freeze): session message history response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionHistory {
    pub session_id: String,
    pub goal: String,
    pub messages: Vec<HistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HistoryEntry {
    pub seq: u64,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: String,
}
```

### b) `crates/service/src/session.rs`

**b1. `Session` 加字段**（:26 `event_tx` 之后）：
```rust
    /// B1 (trunk-freeze): in-memory ring of broadcast events for history replay.
    pub events: Vec<AgentEvent>,
```

**b2. `create_session` 字面量（:153 `Session {`）加 `events: Vec::new(),`**（同处还有两处测试 `Session {` 字面量，也各加 `events: Vec::new(),`）。

**b3. 9 个 `tx.send` 点同步 push 到缓冲**（`tx` 为 `event_tx` 克隆、`session_clone` 为 `Arc<Mutex<Session>>`，均在作用域）：
- 主转发 `:297-298`：
  ```rust
  let api_evt = map_event(evt);
  let _ = tx.send(api_evt.clone());
  session_clone.lock().await.events.push(api_evt);
  ```
- `:304/310` cancel、`408/411` Ok(Err)、`415/419` Err、`434/438` no-agent：每处把
  `let _ = tx.send(AgentEvent::X { .. });` 改为先构造 `let ev = AgentEvent::X { .. }; let _ = tx.send(ev.clone()); session_clone.lock().await.events.push(ev);`

**b4. `get_history` 方法**（在 `load_persisted_session` 之后追加）：
```rust
pub async fn get_history(
    &self,
    session_id: &str,
) -> anyhow::Result<Option<SessionHistory>> {
    {
        let sessions = self.sessions.read().await;
        if let Some(s) = sessions.get(session_id) {
            let inner = s.lock().await;
            return Ok(Some(SessionHistory {
                session_id: session_id.to_string(),
                goal: inner.goal.clone(),
                messages: inner.events.iter().enumerate().map(|(i, e)| HistoryEntry {
                    seq: i as u64,
                    event_type: agent_event_kind(e),
                    payload: serde_json::to_value(e).unwrap_or(serde_json::Value::Null),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                }).collect(),
            }));
        }
    }
    match &self.memory_store {
        Some(store) => match store.load_session(session_id).await? {
            Some(record) => Ok(Some(SessionHistory {
                session_id: record.session_id,
                goal: record.goal,
                messages: record.events.into_iter().map(|e| HistoryEntry {
                    seq: e.seq,
                    event_type: e.event_type,
                    payload: e.payload,
                    timestamp: e.timestamp,
                }).collect(),
            })),
            None => Ok(None),
        },
        None => Ok(None),
    }
}
```
文件顶部 import 加 `use api::{HistoryEntry, SessionHistory};`。
同文件加自由函数：
```rust
/// B1: map an AgentEvent enum variant to a stable string kind.
fn agent_event_kind(e: &AgentEvent) -> String {
    match e {
        AgentEvent::Phase { .. } => "phase",
        AgentEvent::Token { .. } => "token",
        AgentEvent::ToolCall { .. } => "tool_call",
        AgentEvent::ToolResult { .. } => "tool_result",
        AgentEvent::NeedApproval { .. } => "need_approval",
        AgentEvent::Reflection { .. } => "reflection",
        AgentEvent::Done { .. } => "done",
        AgentEvent::Error { .. } => "error",
    }.to_string()
}
```

### c) `crates/service/src/routes.rs` 加路由（见 B2 段，错误类型用 `(StatusCode, Json<ErrorResponse>)`）

### d) `crates/service/src/main.rs` 挂路由（见 B2 段）

---

## B2 — 统一错误码体系（与 B1 合并改）

### a) `crates/api/src/lib.rs` 文件末尾追加
```rust
// ── B2 (trunk-freeze): unified error response ──
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse { pub error: ErrorBody }
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorBody { pub code: String, pub message: String }
impl ErrorResponse {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self { error: ErrorBody { code: code.into(), message: message.into() } }
    }
}
pub type ApiResult<T> = Result<T, (StatusCode, Json<ErrorResponse>)>;
pub const ERR_UNAUTHORIZED: &str = "UNAUTHORIZED";
pub const ERR_SESSION_NOT_FOUND: &str = "SESSION_NOT_FOUND";
pub const ERR_INVALID_PARAM: &str = "INVALID_PARAM";
pub const ERR_APPROVAL_TIMEOUT: &str = "APPROVAL_TIMEOUT";
pub const ERR_LLM_ERROR: &str = "LLM_ERROR";
pub const ERR_TOOL_ERROR: &str = "TOOL_ERROR";
pub const ERR_SANDBOX_ERROR: &str = "SANDBOX_ERROR";
pub const ERR_INTERNAL: &str = "INTERNAL";
```
（`Json` 来自 `axum`；`StatusCode` 已通过 `axum::http::StatusCode` 可用——若 `api` crate 未依赖 axum，则 `ApiResult` 里用 `axum::http::StatusCode` 需 `api` 依赖 axum。若不想给 `api` 加 axum 依赖，可把 `ApiResult` 类型别名定义在 `service` crate 而非 `api`。**本方案把 `ApiResult` 定义在 `service` 的 routes.rs 而非 api，避免 api 依赖 axum**；api 只放 `ErrorResponse/ErrorBody/常量`。）

### b) `crates/service/src/routes.rs`
- 加 `use api::{ErrorResponse, ERR_UNAUTHORIZED, ERR_INVALID_PARAM, ERR_SESSION_NOT_FOUND, ERR_INTERNAL};`（及已有 `use axum::Json;`）
- 加 helper：
```rust
fn api_err(code: &str, msg: impl Into<String>, status: StatusCode) -> (StatusCode, Json<ErrorResponse>) {
    (status, Json(ErrorResponse::new(code, msg)))
}
```
- 每个 handler 返回类型 `(StatusCode, String)` → `(StatusCode, Json<ErrorResponse>)`，每个 `Err((StatusCode::X, format!(..)))` → `Err(api_err(ERR_*, format!(..), StatusCode::X))`。
- `require_api_key`（返回 `Result<Response, (StatusCode, String)>`）同步改 `(StatusCode, Json<ErrorResponse>)`，内部 `Err` 用 `api_err(ERR_UNAUTHORIZED, .., StatusCode::UNAUTHORIZED)`。
- B1 的 `get_session_history`：
```rust
pub async fn get_session_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    match state.sessions.get_history(&id).await {
        Ok(Some(history)) => Ok((StatusCode::OK, Json(history))),
        Ok(None) => Err(api_err(ERR_SESSION_NOT_FOUND, format!("session not found: {id}"), StatusCode::NOT_FOUND)),
        Err(e) => Err(api_err(ERR_INTERNAL, format!("{e}"), StatusCode::INTERNAL_SERVER_ERROR)),
    }
}
```

### c) `crates/service/src/main.rs` 挂 B1 路由
```rust
.route("/api/v1/sessions/{id}/messages", get(routes::get_session_history))
```
（与已有的 `post(routes::send_message)` 同路径、不同方法，axum 合法）

---

## B3 — OpenAPI / TS 类型生成（fallback: components only）

### a) 依赖
- `crates/api/Cargo.toml` `[dependencies]` 加 `utoipa = "5"`
- `crates/service/Cargo.toml` `[dependencies]` 加 `utoipa = "5"` 与 `utoipa-swagger-ui = { version = "7", features = ["axum"] }`

### b) `ToSchema` derive
- `crates/api/src/lib.rs` 顶部加 `use utoipa::ToSchema;`，给 `AgentEvent / SessionCreate / SessionCreateResponse / MessageReq / ApprovalReq / SessionStatus / ModelInfo / ErrorResponse / ErrorBody / SessionHistory / HistoryEntry` 加 `#[derive(..., ToSchema)]`。
- `crates/agent-types/src/lib.rs`：`Budget`(struct, :187) 加 `#[derive(..., ToSchema)]` + 顶部 `use utoipa::ToSchema;`（其字段均为 `u64`/`Option<u64>`，ToSchema 安全）。

### c) `crates/service/src/main.rs` 挂端点（**不放 paths，避免 `#[utoipa::path]` 要求**）
```rust
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(components(schemas(
    api::AgentEvent, api::SessionCreate, api::SessionCreateResponse,
    api::MessageReq, api::ApprovalReq, api::SessionStatus, api::ModelInfo,
    api::ErrorResponse, api::ErrorBody, api::SessionHistory, api::HistoryEntry,
)))]
struct ApiDoc;

// 在 let app = Router::new() 之后加：
let app = app.merge(SwaggerUi::new("/swagger-ui").url("/openapi.json", ApiDoc::openapi()));
```
> 说明：此 fallback 生成 **有效 OpenAPI 3.0 文档（含全部组件 schema）**，paths 为空。完整 paths 注解（带 `#[utoipa::path]`）留作后续，不在本冻结范围。

---

## S1 — 配置规范化（env-varify TTL）

`crates/service/src/main.rs` :174-176 改为：
```rust
    // S1 (trunk-freeze): TTL configurable via env, defaults preserved.
    let oom_ttl = std::env::var("OOM_TTL_SECS")
        .ok().and_then(|s| s.parse().ok())
        .map(std::time::Duration::from_secs)
        .unwrap_or(std::time::Duration::from_secs(3600));
    let oom_sweep = std::env::var("OOM_SWEEP_INTERVAL_SECS")
        .ok().and_then(|s| s.parse().ok())
        .map(std::time::Duration::from_secs)
        .unwrap_or(std::time::Duration::from_secs(300));
    sessions.start_cleanup_task(oom_ttl, oom_sweep);
```
（`start_cleanup_task(self: &Arc<Self>, ttl, interval)` 签名见 `session.rs:515`，匹配。）

## S2 — telemetry eval 接 CI

`crates/.github/workflows/ci.yml`（test 步骤后追加）：
```yaml
      - name: eval
        run: cargo test -p telemetry -- --nocapture
```
> eval harness 用 `MockProvider`（`eval.rs:310 build_task_script`），全程本地、不联网，无需 env gate。

---

## 改动文件汇总

| 文件 | 改动 |
|---|---|
| `crates/api/src/lib.rs` | B1 DTO(SessionHistory/HistoryEntry) + B2(ErrorResponse/ErrorBody/常量) + B3(各 struct ToSchema) |
| `crates/api/Cargo.toml` | B3 加 utoipa |
| `crates/agent-types/src/lib.rs` | B3 Budget 加 ToSchema |
| `crates/service/Cargo.toml` | B3 加 utoipa + utoipa-swagger-ui |
| `crates/service/src/session.rs` | B1 events 字段+缓冲+get_history+agent_event_kind；两处测试 Session 字面量加 events |
| `crates/service/src/routes.rs` | B1 get_session_history；B2 api_err + 所有 Err 改错误码 |
| `crates/service/src/main.rs` | B1 挂 GET messages；B3 挂 /openapi.json+/swagger-ui；S1 env TTL |
| `.github/workflows/ci.yml` | S2 eval step |

---

## 提交门槛（gate）

```bash
cd ~/codex && source $HOME/.cargo/env
# 致命坑：tar 保留本机 mtime 早于 VM target 产物 → cargo 跳过重编复用旧缓存
find ~/codex/crates -name '*.rs' -exec touch {} +
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --all
echo $?  # 期望 rc=0，且测试数 ≥ 146（B1 新增逻辑不增测试数，但结构须过）
```

## 验收清单

- [ ] `curl localhost:3000/api/v1/sessions/{id}/messages` → 返回 JSON 历史（含 phase/token/tool_call/.../done）
- [ ] `curl localhost:3000/api/v1/sessions/nonexistent` → `{"error":{"code":"SESSION_NOT_FOUND",...}}`
- [ ] `curl localhost:3000/openapi.json` → 有效 OpenAPI 3.0 文档（含 components.schemas）
- [ ] `cargo test --all` 全绿（含 `test_p5_eval_harness_regression`，本地 mock）
- [ ] `OOM_TTL_SECS=60 OOM_SWEEP_INTERVAL_SECS=10` 启动无异常
- [ ] 本文件 B1/B2/B3/S1/S2 标记 ✅
