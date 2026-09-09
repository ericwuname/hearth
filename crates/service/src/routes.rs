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
    /// 6C: shared civilization store for all sessions.
    pub civ_store: Arc<memory::CivilizationStore>,
    /// v10.5: Tool registry for search/install.
    pub tool_registry: Arc<tool_runtime::ToolRegistry>,
    /// 6D: shared work line store.
    pub workline_store: Arc<memory::WorkLineStore>,
    /// v7.0: shared telemetry collector.
    pub telemetry: Arc<TelemetryCollector>,
    /// WP-4 (v23 phase4): Observer（第三权）——只读事件流，零执行权。
    /// L2 fail-closed：构造失败 → service 拒启（内核不 import observer）。
    pub observer: Arc<observer::Observer>,
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
pub async fn create_session(
    State(state): State<Arc<AppState>>,
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
    let _ = state.civ_store.append(civ_entry);

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
            let replay = state.sessions.session_events(&id).await;
            Ok(sse::sse_stream_with_replay(rx, replay, last_seq))
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
    let replay = state.sessions.session_events(&id).await;
    Ok(sse::sse_stream_with_replay(rx, replay, last_seq))
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
            ERR_SESSION_NOT_FOUND,
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
    match state.sessions.open_external(&id, &rel).await {
        Ok(msg) => Ok((StatusCode::OK, msg)),
        Err(e) => Err(api_err(
            ERR_SESSION_NOT_FOUND,
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
    if let Some(ip) = peer_ip {
        let mut map = PER_IP_INFLIGHT
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .unwrap();
        let n = map.entry(ip).or_insert(0);
        if *n >= P1_MAX_PER_IP {
            return Ok(rate_limit_response(&format!("per-ip limit ({ip})")));
        }
        *n += 1;
    }
    // 全局总量（洪水防护，上限放宽到 per-IP 的 10 倍）
    let total = P1_TOTAL_INFLIGHT.fetch_add(1, Ordering::Relaxed);
    if total >= P1_MAX_TOTAL {
        P1_TOTAL_INFLIGHT.fetch_sub(1, Ordering::Relaxed);
        if let Some(ip) = peer_ip {
            let mut map = PER_IP_INFLIGHT
                .get_or_init(|| Mutex::new(HashMap::new()))
                .lock()
                .unwrap();
            if let Some(v) = map.get_mut(&ip) {
                if *v > 1 {
                    *v -= 1;
                } else {
                    map.remove(&ip);
                }
            }
        }
        return Ok(rate_limit_response("global limit"));
    }
    let resp = next.run(req).await;
    P1_TOTAL_INFLIGHT.fetch_sub(1, Ordering::Relaxed);
    if let Some(ip) = peer_ip {
        let mut map = PER_IP_INFLIGHT
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .unwrap();
        if let Some(v) = map.get_mut(&ip) {
            if *v > 1 {
                *v -= 1;
            } else {
                map.remove(&ip);
            }
        }
    }
    Ok(resp)
}

// ── 6C v6.0: Civilization Line handlers ──

use agent_types::{CivAuthor, CivCategory, CivEntry};

/// GET /api/v1/civilization — recent civ entries.
pub async fn get_civ_feed(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let uid = get_user_id(&headers, &state.user_store);
    let entries = state.per_user.civ_for(&uid).recent(50);
    Json(serde_json::json!({ "entries": entries }))
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
    state.per_user.civ_for(&uid).append(entry).map_err(|e| {
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
pub async fn get_workline(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let uid = get_user_id(&headers, &state.user_store);
    let nodes = state.per_user.workline_for(&uid).list(None);
    Json(serde_json::json!({ "nodes": nodes }))
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
    let tools: &[&str] = &[
        "bash",
        "read_file",
        "write_file",
        "edit_file",
        "grep",
        "glob",
        "lsp_diagnostics",
        "lsp_hover",
        "code_index_search",
        "web_search",
        "web_fetch",
        "send_message",
        "approve",
    ];
    Json(serde_json::json!({ "tools": tools }))
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
        Err(e) => Err(api_err(ERR_INTERNAL, e, StatusCode::BAD_REQUEST)),
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
