# Trunk Freeze 执行方案（handoff-to-executor）

> 基线：v2-gaps 定版
> 目标：关闭 5 项冻结阻塞/建议项（B1/B2/B3 + S1/S2），使主干可冻结为 v3.0
> 估量：3–5 天
> 格式：每项 文件:行号 → 精确补丁 / 验证命令 / 提交门槛

---

## 前置条件

- Linux VM `wutao@192.168.220.131`（Ubuntu 24.04，landlock/seccomp 可用）
- 致命坑：tar mtime 问题 → 解压后 `find ~/codex/crates -name '*.rs' -exec touch {} +`
- 验收命令：`cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --all`

---

## B1 — 历史回放端点

### 目标
`GET /api/v1/sessions/{id}/messages` → 返回该 session 的完整消息历史 JSON 数组。

### 改动清单

#### a) 新增 API DTO

**文件**: `crates/api/src/lib.rs`，在 `SessionStatus` struct 之后追加：

```rust
/// B1 (trunk-freeze): session message history response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHistory {
    pub session_id: String,
    pub goal: String,
    pub messages: Vec<HistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub seq: u64,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: String,
}
```

#### b) SessionManager 加 get_history

**文件**: `crates/service/src/session.rs`，在 `load_persisted_session` 方法后追加：

```rust
    /// B1 (trunk-freeze): Load session history for the history-replay endpoint.
    /// Falls back to in-memory events for active sessions, then to MemoryStore
    /// for completed/persisted sessions.
    pub async fn get_history(
        &self,
        session_id: &str,
    ) -> anyhow::Result<Option<SessionHistory>> {
        // Check in-memory session first (has live events)
        {
            let sessions = self.sessions.read().await;
            if let Some(s) = sessions.get(session_id) {
                let inner = s.lock().await;
                return Ok(Some(SessionHistory {
                    session_id: session_id.to_string(),
                    goal: inner.goal.clone(),
                    messages: inner.events.iter().enumerate().map(|(i, e)| {
                        HistoryEntry {
                            seq: i as u64,
                            event_type: e.event_type.clone(),
                            payload: e.payload.clone(),
                            timestamp: chrono::Utc::now().to_rfc3339(),
                        }
                    }).collect(),
                }));
            }
        }
        // Fall back to MemoryStore for completed sessions
        match &self.memory_store {
            Some(store) => {
                match store.load_session(session_id).await? {
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
                }
            }
            None => Ok(None),
        }
    }
```

**需要在 session.rs 顶部加 import**（确保 `HistoryEntry`, `SessionHistory` 可用）：
```rust
use api::{HistoryEntry, SessionHistory};
```

#### c) 新增路由

**文件**: `crates/service/src/routes.rs`，在 `cancel_session` 之后追加：

```rust
/// B1 (trunk-freeze): GET /api/v1/sessions/:id/messages
pub async fn get_session_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match state.sessions.get_history(&id).await {
        Ok(Some(history)) => Ok((StatusCode::OK, Json(history))),
        Ok(None) => Err((StatusCode::NOT_FOUND, format!("session not found: {id}"))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}"))),
    }
}
```

**在文件头部加 import**（如果未导入）：
```rust
use api::SessionHistory;
```

#### d) 挂载路由

**文件**: `crates/service/src/main.rs`，在现有路由注册 `get_session_status` 行之后加：

```rust
.route("/api/v1/sessions/{id}/messages", get(routes::get_session_history))
```

注意：`GET /api/v1/sessions/{id}/messages` 与 `POST /api/v1/sessions/{id}/messages` 路径相同、方法不同，axum 支持。

### 验证
```bash
# 创建 session 并发一条消息
curl -s -X POST localhost:3000/api/v1/sessions \
  -H 'Content-Type: application/json' \
  -d '{"provider":"openai","goal":"say hello","budget":null}'
# → {"session_id":"...","status":"created"}

# 查历史（应为空或含初始事件）
curl -s localhost:3000/api/v1/sessions/<id>/messages
# → {"session_id":"...","goal":"say hello","messages":[...]}
```

---

## B2 — 统一错误码体系

### 目标
所有 API 错误返回 `{"error":{"code":"...","message":"..."}}` 格式，替换当前 `(StatusCode, String)` 裸元组。

### 改动清单

#### a) 新增 ErrorResponse 类型 + 错误码常量

**文件**: `crates/api/src/lib.rs`，在文件末尾（`SessionHistory` 之后）追加：

```rust
// ── B2 (trunk-freeze): unified error response ──

/// Standard error response for all API endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

impl ErrorResponse {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: ErrorBody {
                code: code.into(),
                message: message.into(),
            },
        }
    }
}

/// Typed API result — replaces bare `(StatusCode, String)` tuples.
pub type ApiResult<T> = Result<T, (StatusCode, Json<ErrorResponse>)>;

// ── Error code constants ──
pub const ERR_UNAUTHORIZED: &str = "UNAUTHORIZED";
pub const ERR_SESSION_NOT_FOUND: &str = "SESSION_NOT_FOUND";
pub const ERR_INVALID_PARAM: &str = "INVALID_PARAM";
pub const ERR_APPROVAL_TIMEOUT: &str = "APPROVAL_TIMEOUT";
pub const ERR_LLM_ERROR: &str = "LLM_ERROR";
pub const ERR_TOOL_ERROR: &str = "TOOL_ERROR";
pub const ERR_SANDBOX_ERROR: &str = "SANDBOX_ERROR";
pub const ERR_INTERNAL: &str = "INTERNAL";
```

#### b) routes.rs 错误返回替换

**文件**: `crates/service/src/routes.rs`

全局规则：每个 `(StatusCode, String)` 返回 → `(StatusCode, Json(ErrorResponse::new(code, msg)))`。
为减少代码膨胀，先加一个辅助函数在 `require_api_key` 之前：

```rust
// ── B2: error helper ──
use api::{ErrorResponse, ERR_INTERNAL, ERR_INVALID_PARAM, ERR_SESSION_NOT_FOUND, ERR_UNAUTHORIZED};

fn api_err(code: &str, msg: impl Into<String>, status: StatusCode) -> (StatusCode, Json<ErrorResponse>) {
    (status, Json(ErrorResponse::new(code, msg)))
}
```

然后每处旧 `Err((status, format!(...)))` 替换为 `Err(api_err(ERR_*, ..., status))`。关键替换清单：

| 旧代码 | 新代码 |
|---|---|
| `Err((StatusCode::UNAUTHORIZED, "missing or invalid API key".to_string()))` | `Err(api_err(ERR_UNAUTHORIZED, "missing or invalid API key", StatusCode::UNAUTHORIZED))` |
| `Err((StatusCode::BAD_REQUEST, format!("failed to create session: {e}")))` | `Err(api_err(ERR_INVALID_PARAM, format!("failed to create session: {e}"), StatusCode::BAD_REQUEST))` |
| `Err((StatusCode::NOT_FOUND, format!("session not found: {id}")))` | `Err(api_err(ERR_SESSION_NOT_FOUND, format!("session not found: {id}"), StatusCode::NOT_FOUND))` |
| `Err((StatusCode::NOT_FOUND, format!("{e}")))` | `Err(api_err(ERR_SESSION_NOT_FOUND, format!("{e}"), StatusCode::NOT_FOUND))` |
| `Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))` | `Err(api_err(ERR_INTERNAL, format!("{e}"), StatusCode::INTERNAL_SERVER_ERROR))` |

**注意**：`create_session` 生成 `SessionCreateResponse`（201），这个正常返回不需要动。只改 `Err` 分支。

**send_message / submit_approval 的 Err 分支**中 `format!("{e}")` 根据上游错误类型决定错误码。如上游区分 NotFound vs 权限 → 两个分支。当前简洁做法：`anyhow` 错误 → 统一 `ERR_INTERNAL`。

### 验证
```bash
curl -s -w '\n%{http_code}' localhost:3000/api/v1/sessions/nonexistent
# → {"error":{"code":"SESSION_NOT_FOUND","message":"..."}}
# → 404

curl -s -w '\n%{http_code}' -H 'Authorization: Bearer wrong' \
  localhost:3000/api/v1/sessions
# → {"error":{"code":"UNAUTHORIZED","message":"..."}}
# → 401 （需要先设 API_KEY=xxx 启动服务）
```

---

## B3 — OpenAPI / TS 类型生成

### 目标
`GET /openapi.json` 返回 OpenAPI 3.0 规范文档；前端可据此自动生成 TS 类型。

### 改动清单

#### a) 加 utoipa 依赖

**文件**: `crates/api/Cargo.toml`，`[dependencies]` 追加：
```toml
utoipa = { version = "5", features = ["axum_extras"] }
```

**文件**: `crates/service/Cargo.toml`，`[dependencies]` 追加：
```toml
utoipa = { version = "5", features = ["axum_extras"] }
utoipa-swagger-ui = { version = "7", features = ["axum"] }
```

#### b) api 类型加 derive

所有 DTO 与 `AgentEvent` 加 `#[derive(ToSchema)]`（`utoipa::ToSchema`）。

**文件**: `crates/api/src/lib.rs`，头部加 `use utoipa::ToSchema;`，然后对以下类型逐一加 `ToSchema`：
- `AgentEvent`（已有 `Serialize,Deserialize` → 改为 `#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]`）
- `SessionCreate`, `SessionCreateResponse`, `MessageReq`, `ApprovalReq`, `SessionStatus`, `ModelInfo`
- `ErrorResponse`, `ErrorBody`（B2 新增的）
- `SessionHistory`, `HistoryEntry`（B1 新增的）

#### c) service 挂 OpenAPI 端点

**文件**: `crates/service/src/main.rs`，头部加：
```rust
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
```

在 `Router::new()` 之前加：
```rust
    #[derive(OpenApi)]
    #[openapi(
        paths(
            routes::create_session,
            routes::get_session_status,
            routes::send_message,
            routes::submit_approval,
            routes::cancel_session,
            routes::list_models,
            routes::get_session_history,        // B1
        ),
        components(schemas(
            api::AgentEvent,
            api::SessionCreate,
            api::SessionCreateResponse,
            api::MessageReq,
            api::ApprovalReq,
            api::SessionStatus,
            api::ModelInfo,
            api::ErrorResponse,
            api::SessionHistory,                // B1
        )),
    )]
    struct ApiDoc;

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/openapi.json", ApiDoc::openapi()))
        .route(...)
        // ... 其余不变
```

**注意**：`utoipa` 的 `#[openapi]` 宏要求 paths 中的函数签名返回 `impl IntoResponse`（当前已是）。若编译报类型不匹配，需在所有 handler 签名中加 `#[utoipa::path(...)]` 属性宏。如时间紧张，可分两期：先挂 `/openapi.json` 用空 `ApiDoc`（仅 components 不含 paths），paths 注解后续补。

### 验证
```bash
curl localhost:3000/openapi.json | head -20
# → {"openapi":"3.0.3","info":{...},"paths":{...}}

curl localhost:3000/swagger-ui  # 浏览器访问
```

---

## S1 — 配置规范化（env-varify TTL）

### 改动

**文件**: `crates/service/src/main.rs` @174-176

**改动前**:
```rust
    sessions.start_cleanup_task(
        std::time::Duration::from_secs(3600),
        std::time::Duration::from_secs(300),
    );
```

**改动后**:
```rust
    // S1 (trunk-freeze): TTL configurable via env, defaults preserved.
    let oom_ttl = std::env::var("OOM_TTL_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(std::time::Duration::from_secs)
        .unwrap_or(std::time::Duration::from_secs(3600));
    let oom_sweep = std::env::var("OOM_SWEEP_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(std::time::Duration::from_secs)
        .unwrap_or(std::time::Duration::from_secs(300));
    sessions.start_cleanup_task(oom_ttl, oom_sweep);
```

### 验证
`OOM_TTL_SECS=60 OOM_SWEEP_INTERVAL_SECS=10 cargo run` → 启动日志无异常，TTL 短时 session 更快被驱逐。

---

## S2 — telemetry eval 接 CI

### 改动

**文件**: `.github/workflows/ci.yml`，在 test 步骤之后追加：

```yaml
      - name: eval
        run: cargo test -p telemetry -- --nocapture
```

### 验证
`cargo test -p telemetry -- --nocapture` → `test_p5_eval_harness_regression ... ok`

---

## 改动文件汇总

| 文件 | 改动内容 |
|---|---|
| `crates/api/src/lib.rs` | B1: `SessionHistory`+`HistoryEntry` DTO；B2: `ErrorResponse`+`ErrorBody`+`ApiResult`+错误码常量；B3: 各 struct 加 `ToSchema` derive |
| `crates/api/Cargo.toml` | B3: 加 `utoipa` dep |
| `crates/service/src/session.rs` | B1: 加 `get_history()` 方法 |
| `crates/service/src/routes.rs` | B1: 加 `get_session_history` 路由；B2: 加 `api_err` helper + 所有 Err 分支改错误码 |
| `crates/service/src/main.rs` | B1: 挂 GET messages 路由；B3: 挂 `/openapi.json`+`/swagger-ui`；S1: env-varify TTL |
| `crates/service/Cargo.toml` | B3: 加 `utoipa`+`utoipa-swagger-ui` dep |
| `.github/workflows/ci.yml` | S2: 加 eval step |

---

## 提交门槛（gate）

在 Linux VM `wutao@192.168.220.131` 执行：

```bash
cd ~/codex && source $HOME/.cargo/env
find ~/codex/crates -name '*.rs' -exec touch {} +
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --all
echo $?  # 期望 rc=0
```

**期望**：fmt 无差异、clippy 无 warn、所有测试 passed（含 B1 新增）。

---

## 执行后验收清单

- [ ] `curl localhost:3000/api/v1/sessions/{id}/messages` → 返回 JSON 历史（B1）
- [ ] `curl localhost:3000/api/v1/sessions/nonexistent` → `{"error":{"code":"SESSION_NOT_FOUND",...}}`（B2）
- [ ] `curl localhost:3000/openapi.json` → 返回有效 OpenAPI 文档（B3）
- [ ] `cargo test --all` 全绿，含 `test_p5_eval_harness_regression`（S2）
- [ ] `top-level-design.md` §13 G6/G7 标记 ✅，§14 T1/T2/T3/T8/T10 标记 ✅
- [ ] `trunk-freeze-branch-plan.md` B1/B2/B3/S1/S2 标记 ✅
