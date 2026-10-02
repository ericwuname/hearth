use crate::session::SessionManager;
use crate::sse;
use api::{
    ApprovalReq, ErrorResponse, MessageReq, SessionCreate, ERR_FORBIDDEN, ERR_INTERNAL,
    ERR_INVALID_PARAM, ERR_SESSION_NOT_FOUND, ERR_UNAUTHORIZED,
};
use axum::{
    extract::{Path, Query, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

/// B2 (trunk-freeze): unified JSON error helper.
fn api_err(
    code: &str,
    msg: impl Into<String>,
    status: StatusCode,
) -> (StatusCode, Json<ErrorResponse>) {
    (status, Json(ErrorResponse::new(code, msg)))
}

/// Shared application state.
pub struct AppState {
    pub sessions: Arc<SessionManager>,
    pub api_key: Option<String>,
    /// P0-1 (audit-fix): 显式选择加入无鉴权模式（ALLOW_NO_AUTH=1），此时只绑 127.0.0.1。
    pub allow_no_auth: bool,
    /// P1-4 (audit-fix): civ 写失败计数（连续失败 >5 → readyz 503）。
    pub civ_write_failures: std::sync::Arc<std::sync::atomic::AtomicU64>,
    // D-108：原 `civ_store` / `workline_store` 两字段**已删**——它们在整个
    // `routes.rs` 里零读取方（`civ_store` 仅在 `create_session` 里被 append，
    // 而该写入 D-108 已改投 per-user store；`workline_store` 从未被任何 handler 读，
    // 路由读的是 `per_user.workline_for()`）。两个 store 本体仍由 `main.rs` 的
    // 组合根持有并供各自的后台任务/适配器使用，只是不再塞进请求态（"只写不读"字段）。
    /// v10.5: Tool registry for search/install.
    pub tool_registry: Arc<tool_runtime::ToolRegistry>,
    /// v7.0: shared telemetry collector.
    pub telemetry: Arc<TelemetryCollector>,
    // D-111②：原 `observer: Arc<observer::Observer>` 字段**已删**——它在整个
    // `routes.rs` 里零读取方（真正的 Observer 接线是组合根对 **SessionManager** 的
    // `sessions.set_observer(...)`，会话收尾时生效）。保留一份请求态副本只会让读代码的
    // 人以为"每个请求都在过 Observer"（"声称≠实现"）。
    /// v8.0: multi-user identity mapping.
    pub user_store: Arc<crate::user::UserStore>,
    /// v8.0: agent template manager.
    pub template_manager: Arc<crate::templates::TemplateManager>,
    /// v8.0: webhook registry.
    pub webhooks: Arc<crate::webhook::WebhookManager>,
    /// v8.0: per-user store multiplexer (v10.0: now actively wired).
    pub per_user: Arc<crate::per_user::PerUserStore>,
    /// v11.0: Experience store for the self-evolution loop.
    pub experience_store: Arc<experience::ExperienceStore>,
    /// v10.0: instance identity string.
    pub instance_id: String,
}

/// Extract user_id from Authorization header, defaulting to "default".
fn get_user_id(headers: &axum::http::HeaderMap, user_store: &crate::user::UserStore) -> String {
    headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .and_then(|key| user_store.find(key))
        .unwrap_or_else(|| "default".into())
}

/// AUTH-0: Bearer-token auth middleware.
///
/// P0-1 (audit-fix): 默认安全反转——未配置 key 时**只有显式 ALLOW_NO_AUTH=1** 才放行
/// （且启动时只绑 127.0.0.1，见 main.rs）。有 key 时：无凭据 → 401；凭据错误 → 403。
pub async fn require_api_key(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    // P4: skip auth for health check endpoints
    if req.uri().path() == "/healthz" || req.uri().path() == "/readyz" {
        return Ok(next.run(req).await);
    }
    if let Some(ref expected) = state.api_key {
        let provided = req
            .headers()
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "));
        let ok = provided
            .map(|p| {
                p.len() == expected.len()
                    && p.bytes()
                        .zip(expected.bytes())
                        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
                        == 0
            })
            .unwrap_or(false);
        if provided.is_none() {
            // 缺凭据 → 401
            return Err(api_err(
                ERR_UNAUTHORIZED,
                "missing API key (Authorization: Bearer <API_KEY>)",
                StatusCode::UNAUTHORIZED,
            ));
        }
        if !ok {
            // 凭据错误 → 403
            return Err(api_err(
                ERR_FORBIDDEN,
                "invalid API key",
                StatusCode::FORBIDDEN,
            ));
        }
    } else if !state.allow_no_auth {
        // P0-1: 无 key 且未显式选择无鉴权 → 拒绝（fail closed）
        return Err(api_err(
            ERR_UNAUTHORIZED,
            "API auth required: set API_KEY or ALLOW_NO_AUTH=1 (loopback only)",
            StatusCode::UNAUTHORIZED,
        ));
    }
    Ok(next.run(req).await)
}

/// POST /api/v1/sessions
///
/// D-108（2026-10-02, traecode）：**"会话创建"公告必须写进 API 真正读取的那个 store**。
///
/// 病灶：读接口 `get_civ_feed` 读的是 `per_user.civ_for(uid)`（v8.0 多用户隔离改造后
/// 的**唯一可见** store，落在 `MEMORY_DIR/<uid>/civ.jsonl`），而本 handler 原写**全局**
/// 文明线 store（`MEMORY_DIR/civilization.jsonl`）——两者是**不同文件**，于是这条公告
/// （以及 agent-loop 经 `CivWriterAdapter` 写入的同款条目）写进了**无人读取的档**：
/// 用户 `hearth civ feed` / `GET /api/v1/civilization` 永远看不到（"写了但不可见"，
/// D-48 重接线的可见面因此仍未真正生效）。
///
/// 注意**不能**反过来"让读侧合并全局 store"——全局档含各用户的目标文本（
/// `goal={req.goal}` 与 run 叙述），合并即跨租户泄露。故唯一安全修法是**写侧对齐读侧**。
///
/// 写失败**不阻断建会话**（公告是 best-effort），但必须**留痕**（不再 `let _ =` 静默）。
/// 防复发：`crates/service/tests/civ_visibility_gate.rs` 钉住本函数的 store 选择。
pub async fn create_session(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(req): Json<SessionCreate>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // v10.3: Real telemetry — increment session counter
    state
        .telemetry
        .session_count
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // v10.4: Fire webhooks on session create
    let _ = state
        .webhooks
        .fire_event("session_created", &serde_json::json!({"goal": req.goal}))
        .await;

    // v10.4: CIV auto-write — record session creation on civilization line
    let civ_entry = agent_types::CivEntry {
        id: format!("civ-{}", chrono::Utc::now().timestamp_millis()),
        author: agent_types::CivAuthor {
            provider_model: "codex-inst-v10".into(),
            session_id: "auto".into(),
            bridge_id: None,
        },
        content: format!("session created: goal={}", req.goal),
        category: agent_types::CivCategory::Announcement,
        context: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        tags: vec!["session_create".into()],
    };
    // D-108：写入**可见的** per-user 文明线档（病灶与理由见本函数上方文档）。
    let uid = get_user_id(&headers, &state.user_store);
    match state.per_user.civ_for(&uid) {
        Ok(store) => {
            if let Err(e) = store.append(civ_entry) {
                tracing::warn!(user = %uid, "civ 公告写入失败（不阻断建会话）: {e}");
            }
        }
        Err(e) => tracing::warn!(user = %uid, "per-user civ store 不可用，公告未写入: {e}"),
    }

    match state.sessions.create_session(req).await {
        Ok(resp) => Ok((StatusCode::CREATED, Json(resp))),
        Err(e) => Err(api_err(
            ERR_INVALID_PARAM,
            format!("failed to create session: {e}"),
            StatusCode::BAD_REQUEST,
        )),
    }
}

/// P0-3 (audit-fix): GET /api/v1/sessions — 会话列表（内存活跃 + 持久化并集）。
pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let sessions = state.sessions.list_sessions_json().await;
    Ok(Json(serde_json::json!({ "sessions": sessions })))
}

/// GET /api/v1/sessions/:id
pub async fn get_session_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    match state.sessions.get_status(&id).await {
        Some(status) => Ok((StatusCode::OK, Json(status))),
        None => {
            // P1-1 (audit-fix): 内存 miss → 回落持久化读路径（修"写了不读"：
            // 重启后会话全 404，但磁盘上 JSONL 明明在）。
            match state.sessions.load_persisted_session(&id).await {
                Ok(Some(rec)) => {
                    let status = api::SessionStatus {
                        session_id: rec.session_id,
                        phase: "persisted".to_string(),
                        steps: 0,
                        budget_remaining: None,
                    };
                    Ok((StatusCode::OK, Json(status)))
                }
                _ => Err(api_err(
                    ERR_SESSION_NOT_FOUND,
                    format!("session not found: {id}"),
                    StatusCode::NOT_FOUND,
                )),
            }
        }
    }
}

/// POST /api/v1/sessions/:id/messages
/// WP-2 (v23 phase3): 支持 `Last-Event-ID` 头——断线重连从 seq+1 续发（G4）。
pub async fn send_message(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<MessageReq>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Last-Event-ID: 客户端已收到的最后 seq → 重放缓冲中 seq 之后的事件
    let last_seq = headers
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    match state.sessions.send_message(&id, req).await {
        Ok(rx) => {
            let (base, replay) = state.sessions.session_events_with_base(&id).await;
            Ok(sse::sse_stream_with_replay(rx, replay, base, last_seq))
        }
        Err(e) => Err(api_err(
            ERR_SESSION_NOT_FOUND,
            format!("{e}"),
            StatusCode::NOT_FOUND,
        )),
    }
}

/// WP-2 (v23 phase3): GET /api/v1/sessions/:id/events —— 录制导出（JSONL，每行一个信封事件）。
pub async fn session_events_export(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let lines = state.sessions.session_events_jsonl(&id).await;
    if lines.is_empty() && state.sessions.get_session(&id).await.is_none() {
        return Err(api_err(
            ERR_SESSION_NOT_FOUND,
            format!("session not found: {id}"),
            StatusCode::NOT_FOUND,
        ));
    }
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/x-ndjson")],
        lines.join("\n"),
    ))
}

/// WP-10 (v23 phase5): GET /api/v1/sessions/:id/stream —— 实时 SSE 流（EventSource 用）。
/// 复用 sse_stream_with_replay：Last-Event-ID 断线续传（G4）。
/// 注：/events 是一次性 JSONL 录制导出（非流），EventSource 需要本 GET 流。
pub async fn session_stream(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let last_seq = headers
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let rx = match state.sessions.stream_rx(&id).await {
        Some(rx) => rx,
        None => {
            return Err(api_err(
                ERR_SESSION_NOT_FOUND,
                format!("session not found: {id}"),
                StatusCode::NOT_FOUND,
            ))
        }
    };
    let (base, replay) = state.sessions.session_events_with_base(&id).await;
    Ok(sse::sse_stream_with_replay(rx, replay, base, last_seq))
}

/// WP-8 (v23 phase5): GET /api/v1/sessions/:id/artifact/open?path=... —— 预览产物内容。
/// 路径校验在 SessionManager::open_artifact（workspace 内）。
pub async fn open_artifact(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let rel = params.get("path").cloned().unwrap_or_default();
    if rel.is_empty() {
        return Err(api_err(
            api::ERR_INVALID_PARAM,
            "missing 'path' query param",
            StatusCode::BAD_REQUEST,
        ));
    }
    // D-111：同 `open_external`——先判存在性（404），再谈产物错误（400）。
    // 原实现把两者都写成 `ERR_SESSION_NOT_FOUND` + 400（码与状态自相矛盾，
    // 且"路径穿越被拒"被报成"会话不存在"）。
    if state.sessions.get_session(&id).await.is_none() {
        return Err(api_err(
            ERR_SESSION_NOT_FOUND,
            format!("session not found: {id}"),
            StatusCode::NOT_FOUND,
        ));
    }
    match state.sessions.open_artifact(&id, &rel).await {
        Ok(content) => Ok((
            StatusCode::OK,
            [(
                axum::http::header::CONTENT_TYPE,
                "text/plain; charset=utf-8",
            )],
            content,
        )),
        Err(e) => Err(api_err(
            ERR_INVALID_PARAM,
            format!("{e}"),
            StatusCode::BAD_REQUEST,
        )),
    }
}

/// B3-3 (backend taskbook #01): 系统打开产物——file → xdg-open/默认程序，
/// url → 默认浏览器。路径校验在 SessionManager::open_external（workspace 内）。
pub async fn open_external(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let rel = params.get("path").cloned().unwrap_or_default();
    if rel.is_empty() {
        return Err(api_err(
            api::ERR_INVALID_PARAM,
            "missing 'path' query param",
            StatusCode::BAD_REQUEST,
        ));
    }
    // D-111（2026-10-02, traecode）：**先显式判会话是否存在**，再区分错误语义。
    //
    // 病灶：原实现的 `Err` 一律映射成 `ERR_SESSION_NOT_FOUND` + **400**——于是
    // "路径穿越被拒 / 产物读失败"（客户端错）与"会话不存在"（应 404）混成同一个码，
    // 且错误码与状态码自相矛盾（"未找到"配 400）。而 `SessionManager` 的错误**只能**
    // 靠错误文本区分，本项目**明令禁止**文本判定（RC20 反模式禁令，见 dispatcher.rs）。
    // ⇒ 用"先查存在性"消除歧义：不存在 → 404 SESSION_NOT_FOUND；其余 → 400 INVALID_PARAM。
    // （同款存在性检查已是本文件 `session_stream` 的既有做法。）
    if state.sessions.get_session(&id).await.is_none() {
        return Err(api_err(
            ERR_SESSION_NOT_FOUND,
            format!("session not found: {id}"),
            StatusCode::NOT_FOUND,
        ));
    }
    match state.sessions.open_external(&id, &rel).await {
        Ok(msg) => Ok((StatusCode::OK, msg)),
        Err(e) => Err(api_err(
            ERR_INVALID_PARAM,
            format!("{e}"),
            StatusCode::BAD_REQUEST,
        )),
    }
}

/// POST /api/v1/sessions/:id/approvals
/// WP-0: 保留为薄适配层（deprecated）——新契约走 POST /interaction/{iid}。
pub async fn submit_approval(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ApprovalReq>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    match state.sessions.submit_approval(&id, req).await {
        Ok(()) => Ok((StatusCode::OK, Json(serde_json::json!({"status": "ok"})))),
        Err(e) => Err(api_err(
            ERR_SESSION_NOT_FOUND,
            format!("{e}"),
            StatusCode::NOT_FOUND,
        )),
    }
}

/// WP-0 (v23 §2.1): POST /api/v1/sessions/:id/interaction/:iid —— 通用交互响应。
/// R1 id 强校验 + R2 一次性消费由 dispatcher 保证；payload 内容内核不解析。
pub async fn submit_interaction(
    State(state): State<Arc<AppState>>,
    Path((id, iid)): Path<(String, String)>,
    Json(req): Json<api::InteractionResponse>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    if req.id != iid {
        return Err(api_err(
            api::ERR_INVALID_PARAM,
            format!("path iid {iid} != body id {}", req.id),
            StatusCode::BAD_REQUEST,
        ));
    }
    match state
        .sessions
        .submit_interaction(
            &id,
            &iid,
            req.resolved,
            &req.by,
            req.payload,
            req.latency_ms,
        )
        .await
    {
        Ok(()) => Ok((StatusCode::OK, Json(serde_json::json!({"status": "ok"})))),
        Err(e) => Err(api_err(
            ERR_SESSION_NOT_FOUND,
            format!("{e}"),
            StatusCode::NOT_FOUND,
        )),
    }
}

/// POST /api/v1/sessions/:id/cancel
pub async fn cancel_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    match state.sessions.cancel_session(&id).await {
        Ok(()) => Ok((
            StatusCode::OK,
            Json(serde_json::json!({"status": "cancelled"})),
        )),
        Err(e) => Err(api_err(
            ERR_SESSION_NOT_FOUND,
            format!("{e}"),
            StatusCode::NOT_FOUND,
        )),
    }
}

/// GET /api/v1/models
pub async fn list_models(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // P4: expose all registered providers from the registry
    let sessions = state.sessions.clone();
    // SessionManager doesn't directly expose registry, but we can list
    // registered providers from the models that were registered at startup.
    // For now, return a dynamically discovered list via the session manager.
    let providers = sessions.list_providers();
    Json(serde_json::json!({
        "providers": providers,
    }))
}

/// B1 (trunk-freeze): GET /api/v1/sessions/:id/messages
pub async fn get_session_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    match state.sessions.get_history(&id).await {
        Ok(Some(history)) => Ok((StatusCode::OK, Json(history))),
        Ok(None) => Err(api_err(
            ERR_SESSION_NOT_FOUND,
            format!("session not found: {id}"),
            StatusCode::NOT_FOUND,
        )),
        Err(e) => Err(api_err(
            ERR_INTERNAL,
            format!("{e}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

// ── P4 v4.1: health check / readiness probe ──

/// GET /healthz — liveness probe (always 200 if the server is alive).
pub async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// P1-4 gate (acceptance-gatekeeper-v22 §四-1): civ 写失败计数 >5 → 存储降级。
/// 独立 pub 判定函数——让 readyz 降级逻辑可被集成测试动态断言（而非仅源码确认）。
pub fn civ_store_degraded(failures: u64) -> bool {
    failures > 5
}

/// GET /readyz — readiness probe (verifies session manager is accessible).
/// EPIC-B（audit-breakdown-v22 B1a）：真实探活——访问 session store 列表，
/// 依赖不可用（I/O 失败/内部错误）返回 503，可用返回 200。
pub async fn readyz(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // P1-2 (audit-fix): 读探活——list_sessions 错误已上抛（无 store / 目录不可读 → Err → 503）
    if let Err(e) = state.sessions.list_persisted_sessions().await {
        tracing::warn!("readyz: session store unavailable: {e}");
        return (StatusCode::SERVICE_UNAVAILABLE, "session store unavailable");
    }
    // P1-4: civ 连续写失败 >5 → 存储降级，不判就绪。
    if civ_store_degraded(
        state
            .civ_write_failures
            .load(std::sync::atomic::Ordering::SeqCst),
    ) {
        tracing::warn!("readyz: civ write failures > 5 — store degraded");
        return (StatusCode::SERVICE_UNAVAILABLE, "civ store degraded");
    }
    // P1-2: 写探活——只读成功不等于就绪（chmod 000 后 read 也可能碰巧过）。
    if let Ok(mdir) = std::env::var("MEMORY_DIR") {
        if mdir.is_empty() {
            return (StatusCode::OK, "OK");
        }
        let probe = std::path::Path::new(&mdir).join(".readyz_probe");
        match std::fs::write(&probe, b"ok").and_then(|_| std::fs::remove_file(&probe)) {
            Ok(_) => (StatusCode::OK, "OK"),
            Err(e) => {
                tracing::warn!("readyz: write probe failed: {e}");
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "session store not writable",
                )
            }
        }
    } else {
        (StatusCode::OK, "OK")
    }
}

// ── P1 v4.1: concurrency limiter middleware ──
// P2-1 (audit-fix): ① 429 补 Retry-After；② per-IP 在途维度（单客户端不再
// 吃满全局额度）；③ /healthz /readyz 豁免限流（探针不应被限流拒）。

use crate::lock::recover;
use std::net::IpAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

/// P2-1: per-IP 在途请求计数（单客户端上限，替代单一全局计数）。
static PER_IP_INFLIGHT: OnceLock<Mutex<HashMap<IpAddr, usize>>> = OnceLock::new();
const P1_MAX_PER_IP: usize = 50;
/// P2-1: 全局总量上限（防总洪水；远大于 per-IP，单客户端不再吃满全局）。
static P1_TOTAL_INFLIGHT: AtomicUsize = AtomicUsize::new(0);
const P1_MAX_TOTAL: usize = 500;

/// P2-1: 429 响应——带 Retry-After 头（门禁自动判定项）。
fn rate_limit_response(reason: &str) -> Response {
    let mut resp = Response::new(axum::body::Body::from(format!(
        "too many requests ({reason})"
    )));
    *resp.status_mut() = StatusCode::TOO_MANY_REQUESTS;
    resp.headers_mut().insert(
        axum::http::header::RETRY_AFTER,
        axum::http::HeaderValue::from_static("5"),
    );
    resp
}

/// P0-05: 限流拒绝原因（用于 429 文案与单测断言）。
#[derive(Debug, PartialEq, Eq)]
enum LimitKind {
    PerIp,
    Global,
}

/// 在途计数的 **RAII 守卫**（P0-05, 2026-10-01, traecode）。
///
/// 病灶（原实现）：在函数尾部**手动** `fetch_sub` / 回退 per-IP 计数。
/// 一旦 `next.run(req).await` **panic 并 unwind**（handler 内任何 panic，例如
/// 修复前的 `per_user.civ_for` `.expect()`），这些递减语句**永不执行** →
/// `PER_IP_INFLIGHT[ip]` 与全局 `P1_TOTAL_INFLIGHT` **双双单调泄漏**。
/// 放大链：同一 IP 泄漏到 50 → 该 IP **永久 429**；全局泄漏到 500 → **整站永久 429**
/// （且健康端点虽豁免，业务面已全灭）。即"一个可复现的 handler panic"被放大成全局 DoS。
///
/// `Drop` 在 unwind 展开时**仍会执行**，故改为守卫持有计数、析构时成对归还。
struct InflightGuard {
    ip: Option<IpAddr>,
}

/// 归还一个 IP 的在途计数（计数为 1 时移除条目，避免 map 无界增长）。
fn release_ip(ip: Option<IpAddr>) {
    if let Some(ip) = ip {
        let mut map = recover(
            PER_IP_INFLIGHT
                .get_or_init(|| Mutex::new(HashMap::new()))
                .lock(),
        );
        if let Some(v) = map.get_mut(&ip) {
            if *v > 1 {
                *v -= 1;
            } else {
                map.remove(&ip);
            }
        }
    }
}

impl Drop for InflightGuard {
    fn drop(&mut self) {
        P1_TOTAL_INFLIGHT.fetch_sub(1, Ordering::Relaxed);
        release_ip(self.ip);
    }
}

/// 尝试占用一个在途名额；成功返回守卫，超限返回原因。
///
/// P0-05：锁访问改走 `recover`（中毒不升级为"每个请求都 panic"，见 D-25）。
fn enter(ip: Option<IpAddr>) -> Result<InflightGuard, LimitKind> {
    if let Some(ip) = ip {
        let mut map = recover(
            PER_IP_INFLIGHT
                .get_or_init(|| Mutex::new(HashMap::new()))
                .lock(),
        );
        let n = map.entry(ip).or_insert(0);
        if *n >= P1_MAX_PER_IP {
            return Err(LimitKind::PerIp);
        }
        *n += 1;
    }
    // 全局总量（洪水防护，上限放宽到 per-IP 的 10 倍）
    let total = P1_TOTAL_INFLIGHT.fetch_add(1, Ordering::Relaxed);
    if total >= P1_MAX_TOTAL {
        P1_TOTAL_INFLIGHT.fetch_sub(1, Ordering::Relaxed);
        // 全局拒绝时回退已占用的 per-IP 名额（保持原有语义）
        release_ip(ip);
        return Err(LimitKind::Global);
    }
    Ok(InflightGuard { ip })
}

/// P1: per-IP + 全局并发限流——容量内透传，超限 429 + Retry-After。
pub async fn concurrency_limit(req: Request, next: Next) -> Result<Response, StatusCode> {
    // P2-1: 健康端点豁免限流（探针必须可到达）
    let path = req.uri().path();
    if path == "/healthz" || path == "/readyz" {
        return Ok(next.run(req).await);
    }
    // per-IP 在途检查（需要 serve 注入 ConnectInfo<SocketAddr>）
    let peer_ip: Option<IpAddr> = req
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip());
    // P0-05：名额由守卫持有，handler panic 时也能归还（原先会泄漏 → 永久 429）。
    let _guard = match enter(peer_ip) {
        Ok(g) => g,
        Err(LimitKind::PerIp) => {
            return Ok(rate_limit_response(&format!(
                "per-ip limit ({})",
                peer_ip.map(|i| i.to_string()).unwrap_or_default()
            )));
        }
        Err(LimitKind::Global) => return Ok(rate_limit_response("global limit")),
    };
    Ok(next.run(req).await)
}

#[cfg(test)]
mod p0_05_tests {
    use super::*;

    /// 先红后绿：修复前计数在函数尾部**手动**归还，`next.run(req).await` panic 展开时
    /// 那段代码不会执行 → per-IP 与全局两个计数**单调泄漏** → 累积到上限即
    /// "该 IP 永久 429 / 整站永久 429"。此处直接对守卫做 unwind 断言。
    ///
    /// 三个断言写在**同一个**测试函数里：它们共享全局静态计数器，拆成多个测试
    /// 并行跑会互相干扰。
    #[test]
    fn test_p0_05_inflight_guard_releases_on_unwind() {
        fn per_ip_of(ip: IpAddr) -> usize {
            recover(
                PER_IP_INFLIGHT
                    .get_or_init(|| Mutex::new(HashMap::new()))
                    .lock(),
            )
            .get(&ip)
            .copied()
            .unwrap_or(0)
        }

        let ip: IpAddr = "10.99.0.1".parse().unwrap();

        // ① 正常路径：占用后归还（含 per-IP 条目清理，防 map 无界增长）。
        let base_total = P1_TOTAL_INFLIGHT.load(Ordering::SeqCst);
        {
            let _g = enter(Some(ip)).expect("容量内应放行");
            assert_eq!(per_ip_of(ip), 1, "占用时应计数 1");
            assert_eq!(P1_TOTAL_INFLIGHT.load(Ordering::SeqCst), base_total + 1);
        }
        assert_eq!(per_ip_of(ip), 0, "归还后 per-IP 条目应被移除");
        assert_eq!(
            P1_TOTAL_INFLIGHT.load(Ordering::SeqCst),
            base_total,
            "归还后全局计数应回到基线"
        );

        // ② panic / unwind 路径（本卡核心缺陷）。
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _g = enter(Some(ip)).expect("容量内应放行");
            panic!("simulated handler panic");
        }));
        assert!(r.is_err(), "前置条件：闭包确实 panic 了");
        assert_eq!(
            per_ip_of(ip),
            0,
            "panic 后 per-IP 计数必须归还（修复前泄漏 → 累积 50 次即该 IP 永久 429）"
        );
        assert_eq!(
            P1_TOTAL_INFLIGHT.load(Ordering::SeqCst),
            base_total,
            "panic 后全局计数必须归还（修复前泄漏 → 累积 500 次即整站永久 429）"
        );

        // ③ per-IP 上限语义未被本次改动破坏：容量内放行、超限拒绝、释放后清零。
        let ip2: IpAddr = "10.99.0.2".parse().unwrap();
        let mut held = Vec::new();
        for i in 0..P1_MAX_PER_IP {
            match enter(Some(ip2)) {
                Ok(g) => held.push(g),
                Err(e) => panic!("第 {i} 个名额应放行，实得 {e:?}"),
            }
        }
        assert!(
            matches!(enter(Some(ip2)), Err(LimitKind::PerIp)),
            "超出 per-IP 上限必须拒绝"
        );
        drop(held);
        assert_eq!(per_ip_of(ip2), 0, "全部释放后应清空");
    }
}

// ── 6C v6.0: Civilization Line handlers ──

use agent_types::{CivAuthor, CivCategory, CivEntry};

/// D-49（2026-10-01）：civ 检索的**唯一事实源**——`GET /api/v1/civilization` 的
/// 选择逻辑抽为纯函数以便单测（`AppState` 有 13 字段、仅 `main.rs` 可构造，无法
/// 在测试里起真路由；处置同 D-44）。
///
/// 带非空 `?search=` 时按关键词过滤，否则回退最近 50 条。此前 handler **硬编码**
/// `store.recent(50)`、完全忽略查询参数 → CLI `hearth civ search <q>` 实为
/// `civ feed`（返回最近条目，与关键词无关），且 `CivilizationStore::search`
/// 全仓零调用方（与 D-44「声称 ≠ 实现」同型）。
pub fn civ_feed_entries(
    store: &memory::CivilizationStore,
    search: Option<&str>,
) -> Vec<agent_types::CivEntry> {
    match search.map(str::trim).filter(|s| !s.is_empty()) {
        Some(q) => store.search(q, 50),
        None => store.recent(50),
    }
}

/// GET /api/v1/civilization — recent civ entries（`?search=` 时按关键词过滤）。
///
/// P0-05：`civ_for` 失败改为 500（**不再**在请求路径 panic —— 那会毒掉 civ 锁，
/// 把一次 I/O 错误放大成该接口全站永久不可用）。
pub async fn get_civ_feed(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let uid = get_user_id(&headers, &state.user_store);
    let store = state.per_user.civ_for(&uid).map_err(|e| {
        api_err(
            ERR_INTERNAL,
            format!("{e}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;
    let entries = civ_feed_entries(&store, params.get("search").map(|s| s.as_str()));
    Ok(Json(serde_json::json!({ "entries": entries })))
}

/// POST /api/v1/civilization — manual civ post.
pub async fn post_civ_entry(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let content = body["content"].as_str().unwrap_or("").to_string();
    if content.is_empty() {
        return Err(api_err(
            ERR_INVALID_PARAM,
            "content is required",
            StatusCode::BAD_REQUEST,
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let entry = CivEntry {
        id: id.clone(),
        author: CivAuthor {
            provider_model: "direct".into(),
            session_id: "manual".into(),
            bridge_id: None,
        },
        content,
        category: CivCategory::Announcement,
        context: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        tags: vec![],
    };
    let uid = get_user_id(&headers, &state.user_store);
    state
        .per_user
        .civ_for(&uid)
        .map_err(|e| {
            api_err(
                ERR_INTERNAL,
                format!("{e}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?
        .append(entry)
        .map_err(|e| {
            api_err(
                ERR_INTERNAL,
                format!("{e}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

// ── 6D v6.0: Work Line handlers ──

use agent_types::{WorkCategory, WorkNode, WorkStatus};

/// GET /api/v1/workline — list tasks.
/// P0-05：同 `get_civ_feed`——失败返回 500，不在请求路径 panic（防锁中毒 DoS）。
pub async fn get_workline(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let uid = get_user_id(&headers, &state.user_store);
    let store = state.per_user.workline_for(&uid).map_err(|e| {
        api_err(
            ERR_INTERNAL,
            format!("{e}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;
    let nodes = store.list(None);
    Ok(Json(serde_json::json!({ "nodes": nodes })))
}

/// POST /api/v1/workline/nodes — create task.
pub async fn create_work_node(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let desc = body["description"].as_str().unwrap_or("").to_string();
    if desc.is_empty() {
        return Err(api_err(
            ERR_INVALID_PARAM,
            "description required",
            StatusCode::BAD_REQUEST,
        ));
    }
    let now = chrono::Utc::now().to_rfc3339();
    let node = WorkNode {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        description: desc,
        status: WorkStatus::Pending,
        progress: 0.0,
        category: WorkCategory::ShortTerm,
        assignee: None,
        deps: vec![],
        created_at: now.clone(),
        updated_at: now,
        completed_at: None,
        notes: vec![],
    };
    let uid = get_user_id(&headers, &state.user_store);
    state
        .per_user
        .workline_for(&uid)
        .map_err(|e| {
            api_err(
                ERR_INTERNAL,
                format!("{e}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?
        .add(node.clone())
        .map_err(|e| {
            api_err(
                ERR_INTERNAL,
                format!("{e}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": node.id })),
    ))
}

/// PATCH /api/v1/workline/nodes/:id — update progress/status.
pub async fn update_work_node(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let progress = body["progress"].as_f64().unwrap_or(0.0) as f32;
    let status = body["status"].as_str().map(|s| match s {
        "completed" => WorkStatus::Completed,
        "in_progress" => WorkStatus::InProgress,
        "blocked" => WorkStatus::Blocked,
        "review" => WorkStatus::Review,
        _ => WorkStatus::Pending,
    });
    let uid = get_user_id(&headers, &state.user_store);
    state
        .per_user
        .workline_for(&uid)
        .map_err(|e| {
            api_err(
                ERR_INTERNAL,
                format!("{e}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?
        .update(&id, progress, status)
        .map_err(|e| {
            api_err(
                ERR_INTERNAL,
                format!("{e}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
    Ok(StatusCode::OK)
}

// ── v6.1 L1: Bridge integration ──

/// POST /api/v1/bridge — create and run a multi-model bridge discussion.
pub async fn create_bridge(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let topic = body["topic"].as_str().unwrap_or("unnamed").to_string();
    let participants: Vec<String> = body["participants"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    if participants.len() < 2 {
        return Err(api_err(
            ERR_INVALID_PARAM,
            "need >= 2 participants",
            StatusCode::BAD_REQUEST,
        ));
    }
    let strategy = match body["strategy"].as_str().unwrap_or("round_robin") {
        "debate" => bridge::BridgeStrategy::Debate,
        "majority" => bridge::BridgeStrategy::MajorityVote,
        _ => bridge::BridgeStrategy::RoundRobin,
    };
    let result = state
        .sessions
        .create_bridge(topic, participants, strategy)
        .await
        .map_err(|e| {
            api_err(
                ERR_INTERNAL,
                format!("{e}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
    Ok(Json(
        serde_json::json!({"id": result.0, "summary": result.1, "turns": result.2}),
    ))
}

// ── v7.0: Telemetry ──

use std::sync::atomic::AtomicU64;

/// v7.0: Simple in-memory telemetry collector.
pub struct TelemetryCollector {
    pub session_count: AtomicU64,
    pub error_count: AtomicU64,
    pub steps_total: AtomicU64,
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        Self {
            session_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            steps_total: AtomicU64::new(0),
        }
    }
}

/// GET /api/v1/telemetry — real-time metrics.
pub async fn get_telemetry(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tc = &state.telemetry;
    Json(serde_json::json!({
        "session_count": tc.session_count.load(Ordering::Relaxed),
        "error_count": tc.error_count.load(Ordering::Relaxed),
        "steps_total": tc.steps_total.load(Ordering::Relaxed),
    }))
}

/// v8.0: GET /api/v1/templates — list available agent templates.
pub async fn list_templates(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let templates: Vec<_> = state.template_manager.list().into_iter().cloned().collect();
    Json(serde_json::json!({ "templates": templates }))
}

/// v8.0: POST /api/v1/webhooks — register a webhook.
pub async fn register_webhook(
    State(state): State<Arc<AppState>>,
    Json(cfg): Json<crate::webhook::WebhookConfig>,
) -> impl IntoResponse {
    state.webhooks.register(cfg);
    Json(serde_json::json!({"ok": true}))
}

/// v10.0: GET /api/v1/resources — system resource snapshot.
pub async fn get_resources(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let pid = std::process::id();
    let snap = resource_monitor::snapshot(pid);
    Json(serde_json::json!({
        "instance_id": &state.instance_id,
        "snapshot": snap,
        "sessions_active": state.telemetry.session_count.load(std::sync::atomic::Ordering::Relaxed),
    }))
}

/// v10.0: GET /api/v1/tools — list registered tools.
pub async fn list_tools() -> impl IntoResponse {
    Json(serde_json::json!({ "tools": tool_names() }))
}

/// `GET /api/v1/tools` 的载荷：本服务内置工具名。
///
/// 唯一事实源 = [`builtin_tools()`]（与启动注册同一张表）——
/// 工具增删只需改那一处，路由与注册**自动同步**，不可能再漂移。
pub fn tool_names() -> Vec<String> {
    builtin_tools()
        .into_iter()
        .map(|(_, t)| t.name().to_string())
        .collect()
}

/// P1-11（D-44）：**内置工具表 = 唯一事实源**。
///
/// 启动注册（`main.rs`）与 `GET /api/v1/tools` 都从这里取，杜绝
/// "API 声称 ≠ 实际注册"。首元素是 `HEARTH_DISABLED_TOOLS` 的**键**——
/// 其中 `edit`→`write_file` 是历史别名，**勿改**（改了会变配置语义）。
pub fn builtin_tools() -> Vec<(&'static str, Arc<dyn tool_runtime::Tool>)> {
    vec![
        ("bash", Arc::new(tools_builtin::BashTool::new())),
        ("read", Arc::new(tools_builtin::ReadTool::new())),
        ("edit", Arc::new(tools_builtin::EditTool::new())),
        ("apply_patch", Arc::new(tools_builtin::PatchTool::new())),
        ("glob", Arc::new(tools_builtin::GlobTool::new())),
        ("grep", Arc::new(tools_builtin::GrepTool::new())),
    ]
}

#[cfg(test)]
mod d44_tests {
    use super::*;

    /// 先红后绿（D-44）：`GET /api/v1/tools` 此前硬编码 13 个名字，与真实注册完全不符——
    /// ① 名字错：`read_file` / `edit_file` 不是工具名（真名 `read` / `write_file`）；
    /// ② 幽灵工具：`lsp_diagnostics` / `lsp_hover` / `code_index_search` 全仓从未注册；
    /// ③ 非工具：`send_message` / `approve` 是 API 动作，不在 dispatcher 里。
    /// 修复后名单由 `builtin_tools()` 派生（与启动注册**同一张表**）→ 不可能漂移。
    #[test]
    fn test_p1_11_reported_tools_match_registered_builtins() {
        let reported = tool_names();
        for ghost in [
            "lsp_diagnostics",
            "lsp_hover",
            "code_index_search",
            "send_message",
            "approve",
            "read_file",
            "edit_file",
        ] {
            assert!(
                !reported.iter().any(|n| n == ghost),
                "API 声称了从未注册的工具名 `{ghost}`（声称 ≠ 存在）"
            );
        }
        let real: Vec<String> = builtin_tools()
            .into_iter()
            .map(|(_, t)| t.name().to_string())
            .collect();
        assert_eq!(
            reported.len(),
            real.len(),
            "名单条数与真实注册不一致：reported={reported:?} real={real:?}"
        );
        for n in &real {
            assert!(
                reported.iter().any(|r| r == n),
                "真实注册的 `{n}` 未出现在 /api/v1/tools"
            );
        }
    }
}

#[cfg(test)]
mod d49_tests {
    use super::*;

    fn store_with(contents: &[&str]) -> (tempfile::TempDir, memory::CivilizationStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = memory::CivilizationStore::new(dir.path(), "civ.jsonl").unwrap();
        for (i, c) in contents.iter().enumerate() {
            store
                .append(agent_types::CivEntry {
                    id: format!("e{i}"),
                    author: agent_types::CivAuthor {
                        provider_model: "test".into(),
                        session_id: "s".into(),
                        bridge_id: None,
                    },
                    content: (*c).to_string(),
                    category: agent_types::CivCategory::Insight,
                    context: None,
                    created_at: "t".into(),
                    tags: vec![],
                })
                .unwrap();
        }
        (dir, store)
    }

    /// 先红后绿（D-49）：`?search=` 必须按关键词过滤。修复前 handler 硬编码
    /// `store.recent(50)` → 非匹配条目也返回（CLI `hearth civ search` 形同 `civ feed`）。
    #[test]
    fn test_d49_civ_feed_search_filters() {
        let (_dir, store) = store_with(&["alpha needle", "beta unrelated"]);
        // 无 search（或缺省）→ 全部返回。
        assert_eq!(civ_feed_entries(&store, None).len(), 2);
        assert_eq!(
            civ_feed_entries(&store, Some("")).len(),
            2,
            "空串视为未检索"
        );
        assert_eq!(
            civ_feed_entries(&store, Some("   ")).len(),
            2,
            "纯空白视为未检索"
        );
        // 有 search → 只返回匹配（修复前会返回 2 条）。
        let hits = civ_feed_entries(&store, Some("needle"));
        assert_eq!(hits.len(), 1, "检索必须过滤掉不匹配条目：{hits:?}");
        assert!(hits[0].content.contains("needle"));
        // 大小写不敏感（沿用 CivilizationStore::search 语义）。
        assert_eq!(civ_feed_entries(&store, Some("NEEDLE")).len(), 1);
    }
}

// ─── v10.5: Tool ecosystem routes ───

/// GET /api/v1/tools/search?q=query
pub async fn search_tools(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let q = params.get("q").map(|s| s.as_str()).unwrap_or("");
    let tools = state.tool_registry.search(q);
    Json(serde_json::json!({ "tools": tools }))
}

/// POST /api/v1/tools/install
#[derive(Debug, Deserialize)]
pub struct InstallToolReq {
    pub manifest: tool_runtime::ToolManifest,
}

pub async fn install_tool(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InstallToolReq>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let name = req.manifest.name.clone();
    match state.tool_registry.install(req.manifest) {
        Ok(true) => Ok((
            StatusCode::CREATED,
            Json(serde_json::json!({"status":"installed","tool":name})),
        )),
        Ok(false) => Ok((
            StatusCode::OK,
            Json(serde_json::json!({"status":"already_installed","tool":name})),
        )),
        Err(e) => Err(api_err(
            // D-111：`ToolRegistry::install` 的失败是**清单本身不合法/重复冲突**（客户端错），
            // 原映射 `ERR_INTERNAL` + 400 自相矛盾（"内部错误"配客户端状态码）。
            ERR_INVALID_PARAM,
            e,
            StatusCode::BAD_REQUEST,
        )),
    }
}

/// GET /api/v1/tool-registry
pub async fn registry_list(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let tools = state.tool_registry.list();
    Json(serde_json::json!({ "tools": tools, "count": tools.len() }))
}

/// GET /api/v1/experience/metrics
pub async fn experience_metrics(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let m = state.experience_store.metrics().await;
    Json(serde_json::to_value(m).unwrap_or_default())
}
