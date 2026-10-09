use crate::session::SessionManager;
use crate::sse;
use api::{
    ApprovalReq, ErrorResponse, MessageReq, SessionCreate, ERR_FORBIDDEN, ERR_INTERNAL,
    ERR_INVALID_PARAM, ERR_LLM_ERROR, ERR_SESSION_NOT_FOUND, ERR_UNAUTHORIZED,
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

/// D-156（P1-122）：统一错误形状的**提取器侧**收口。
///
/// 契约口径：`docs/handoff-trunk-freeze.md`「**所有** API 错误返回
/// `{"error":{"code":"...","message":"..."}}` 格式」；`docs/governance.md` B2
/// 「**全部 handler** 返回 `Json<ErrorResponse>`」。但 axum 内建 `Json` 提取器在
/// **拒绝**时（缺/错 `Content-Type` → 415；畸形 JSON → 400）直接吐**纯文本**体 +
/// `text/plain` Content-Type，绕过 B2 统一形状 ⇒ 按 JSON 解析错误体的客户端在 4xx
/// 上必失败（"声称≠实现"/"半接线"）。
///
/// 本包装把 `JsonRejection` 归一为统一 `ErrorResponse`（保留原状态码与脱敏后的
/// `body_text`）；所有 handler 一律用 `ApiJson<T>` 取代 `Json<T>`。
/// 回归锁：`crates/service/tests/api_error_shape_gate.rs`。
pub struct ApiJson<T>(pub T);

#[async_trait::async_trait]
impl<T, S> axum::extract::FromRequest<S> for ApiJson<T>
where
    T: serde::de::DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ErrorResponse>);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match <Json<T> as axum::extract::FromRequest<S>>::from_request(req, state).await {
            Ok(Json(v)) => Ok(ApiJson(v)),
            Err(rej) => {
                let status = rej.status();
                Err(api_err(ERR_INVALID_PARAM, rej.body_text(), status))
            }
        }
    }
}

/// D-158（2026-10-05, traecode）：未知路由 → 统一 JSON 404。
///
/// axum 默认 fallback 返回**空体**且无 `Content-Type: application/json`，绕过契约
/// 「**所有** API 错误返回 `{"error":{"code","message"}}`」（路由层未被 B2/`ApiJson` 覆盖）。
/// 由 `main.rs` 挂到 `Router::fallback`。回归锁：`tests/api_error_shape_gate.rs` ⑤。
pub async fn not_found(req: Request) -> impl IntoResponse {
    api_err(
        api::ERR_NOT_FOUND,
        format!("no route for {} {}", req.method(), req.uri().path()),
        StatusCode::NOT_FOUND,
    )
}

/// D-158（2026-10-05, traecode）：路径存在但方法不允许 → 统一 JSON 405。
///
/// 由 `main.rs` 挂到 `Router::method_not_allowed_fallback`；`Allow` 头由 axum 在 route 层
/// 自动回填（见 `api_error_shape_gate.rs` ⑥）。回归锁同上。
pub async fn method_not_allowed(req: Request) -> impl IntoResponse {
    api_err(
        api::ERR_METHOD_NOT_ALLOWED,
        format!(
            "method {} not allowed for {}",
            req.method(),
            req.uri().path()
        ),
        StatusCode::METHOD_NOT_ALLOWED,
    )
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
    // D-175（2026-10-08, traecode）：原 `experience_store: Arc<ExperienceStore>` 字段**已删**
    // ——经验库已按 uid 分区（`per_user.experience_for(uid)`），`experience_metrics` 改读
    // per-user 档后此字段**零读取方**（真正生效的是组合根的 `set_experience_factory`）。
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

/// D-171（2026-10-08, traecode）：会话不存在（或**不属于调用者**）的统一 404 形状。
fn session_not_found(id: &str) -> (StatusCode, Json<ErrorResponse>) {
    api_err(
        ERR_SESSION_NOT_FOUND,
        format!("session not found: {id}"),
        StatusCode::NOT_FOUND,
    )
}

/// D-171（2026-10-08, traecode）：**多租户会话归属校验**——以会话 id 寻址的 handler
/// 的**统一入口**。
///
/// 病灶：D-169 打开了 `HEARTH_USERS` 多租户（鉴权按 key 映射到 uid、`create_session`
/// 也把 uid 记为 `owner`），但**会话读/写面完全没有归属条件**——任一已认证租户只要拿到
/// 另一租户的会话 id，即可 `GET /sessions/<id>` 读其状态、`GET .../events|messages` 读其
/// 事件与目标文本、`POST .../messages` 往其会话注入、`POST .../cancel` 停其会话、
/// `.../artifact/open` 读其工作区文件 ⇒ 跨租户数据泄露 + 完整性破坏（BOLA/IDOR）。
///
/// 口径（`owner != uid` ⇒ **404** 而非 403）：对标 OWASP Multi-Tenant Security Cheat Sheet
/// 与跨租户 BOLA 防护共识——**不要用 403 泄露"该 id 有效但归别人"**，否则攻击者可据
/// 状态码差异枚举有效会话 id。会话**不存在**与**存在但非本人**走**同一 404 形状**，
/// 不可区分。归属未知（无 store / 读失败）一律按不存在处理（fail closed）。
async fn ensure_session_owner(
    state: &AppState,
    headers: &axum::http::HeaderMap,
    id: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let uid = get_user_id(headers, &state.user_store);
    match state.sessions.owner_of(id).await {
        Some(owner) if owner == uid => Ok(()),
        _ => Err(session_not_found(id)),
    }
}

/// D-179（2026-10-08, traecode）：**"存活于内存"判据**——`ensure_session_owner` 的"存在"
/// 是"内存**或**持久化"，但**实时**端点（开产物 / 系统打开 / 审批 / 交互 / 发消息）要的是
/// **内存中活跃**的会话：仅持久化的会话（重启后未被加载）没有工作区句柄、没有 dispatcher
/// 交互项 ⇒ 应如实 **404**。
///
/// 病灶（D-171 引入的漂移）：D-171 把这几处的"显式存在性检查"换成 `ensure_session_owner`
/// 后，对**仅持久化**的会话，其内层错误被硬编码成 400 ⇒ 返回 **400 `INVALID_PARAM`**
/// 且 message 写着 `session not found: …`（**码与文案自相矛盾**）；而**同一 id** 在
/// `cancel_session` / `session_stream` / `send_message`（D-178）上正确报 **404**。
/// 本函数把这些端点的判据统一到"是否存活于内存"（与 D-166/D-178 同口径）。
async fn session_live(state: &AppState, id: &str) -> bool {
    state.sessions.get_session(id).await.is_some()
}

/// D-179（收编 D-166 的 `classify_interaction_error`）：按 **"是否存活于内存"** 把内层错误
/// 分类——存活 ⇒ 400 `INVALID_PARAM`（请求与当前状态不符）；不存活 ⇒ 404 `SESSION_NOT_FOUND`。
///
/// D-166 原病灶（本函数的前身只服务 approvals/interaction）：`submit_approval` /
/// `submit_interaction` 把**任何**失败一律映射为 **404 + `ERR_SESSION_NOT_FOUND`**——而
/// `ToolDispatcher::resolve_interaction` 的失败**全是请求级**（`interaction_id` 不匹配 /
/// 无 pending 交互 / 无交互态；`submit_approval` 另有"非法 decision"），于是把**客户端拼错或
/// 重复提交**冒充成"**会话不存在**"（客户端按 `code` 分支会误判会话丢失）。
/// ⇒ 口径：**只有"会话不在"才是 404**；"在但不匹配当前状态"一律 400。
/// D-179 把这一口径**推广到全部"实时"端点**（开产物 / 系统打开 / 审批 / 交互 / 发消息），
/// 并把判据从 D-171 的"存在（内存**或**持久化）"**收紧为"存活于内存"**。
fn classify_live_error(
    err: &dyn std::fmt::Display,
    live: bool,
    id: &str,
) -> (StatusCode, Json<ErrorResponse>) {
    if live {
        api_err(ERR_INVALID_PARAM, format!("{err}"), StatusCode::BAD_REQUEST)
    } else {
        session_not_found(id)
    }
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
    // D-169（2026-10-05, traecode）：鉴权来源＝「**已注册 key 集**」`∪` 显式 `API_KEY`。
    //
    // 修复前只看 `state.api_key`：只配 `HEARTH_USERS`（多租户）而不配 `API_KEY` 时，
    // "已配 key"被误判成"未配 key" ⇒ 要么放行（`ALLOW_NO_AUTH=1`，等于无鉴权）、
    // 要么整站 401。既有单 key 亦经 `main.rs` 的 `register(key,"default")` 入册，
    // 故两种配置方式在**同一判定**下统一（多租户隔离由此真正可用）。
    if state.api_key.is_some() || state.user_store.has_keys() {
        let provided = req
            .headers()
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "));
        let Some(p) = provided else {
            // 缺凭据 → 401
            return Err(api_err(
                ERR_UNAUTHORIZED,
                "missing API key (Authorization: Bearer <API_KEY>)",
                StatusCode::UNAUTHORIZED,
            ));
        };
        // 命中**任一**已注册 key 即通过（逐 key 常数时间、不早退）；保留单 `API_KEY` 显式比对。
        let ok = state.user_store.authenticates(p)
            || state
                .api_key
                .as_deref()
                .map(|expected| {
                    p.len() == expected.len()
                        && p.bytes()
                            .zip(expected.bytes())
                            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
                            == 0
                })
                .unwrap_or(false);
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

/// D-163（P1-129）：API 可覆盖的会话步数预算**上界**。
///
/// 服务端会话由后台任务跑、**没有墙钟上限** ⇒ `budget.max_steps` 是其**唯一**的时间/成本界。
/// `Budget::default()`（env 路径）本有护栏（`HEARTH_MAX_STEPS`，`0`/非法回落 50，注释明写
/// "护栏不得被配没"），但客户端**自带**的 `budget.max_steps` 此前被**原样**采纳 ⇒ 可发
/// `1e8` 绕过护栏（单会话近无界运行 = 成本/资源 DoS）；发 `0` 则 `is_budget_exhausted()`
/// （`steps_used >= 0`）**开局即耗尽**（同 D-160 的"0 语义"陷阱）。取 1000：默认 50 的 20 倍，
/// 够用且封顶。
const MAX_SESSION_STEPS: u64 = 1000;

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
    ApiJson(req): ApiJson<SessionCreate>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // D-163：入参边界校验放在**任何副作用之前**（遥测/webhook/公告都不该为非法请求动）。
    if let Some(b) = req.budget.as_ref() {
        if b.max_steps == 0 || b.max_steps > MAX_SESSION_STEPS {
            return Err(api_err(
                ERR_INVALID_PARAM,
                format!(
                    "budget.max_steps must be in 1..={MAX_SESSION_STEPS} (got {})",
                    b.max_steps
                ),
                StatusCode::BAD_REQUEST,
            ));
        }
    }
    // D-172：调用者 uid 提前取——webhook 投递（下方）与 civ 公告、会话归属都要用它。
    let uid = get_user_id(&headers, &state.user_store);
    // v10.3: Real telemetry — increment session counter
    state
        .telemetry
        .session_count
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // v10.4: Fire webhooks on session create
    // D-172：按**发起者 uid** 过滤——只投给同租户的 hook（修复前对全部 hook 广播 ⇒
    // 多租户下 alice 可收割 bob 的会话目标文本）。
    let _ = state
        .webhooks
        .fire_event(
            "session_created",
            &serde_json::json!({"goal": req.goal}),
            &uid,
        )
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
    match state.per_user.civ_for(&uid) {
        Ok(store) => {
            if let Err(e) = store.append(civ_entry) {
                tracing::warn!(user = %uid, "civ 公告写入失败（不阻断建会话）: {e}");
            }
        }
        Err(e) => tracing::warn!(user = %uid, "per-user civ store 不可用，公告未写入: {e}"),
    }

    // D-109（2026-10-02, traecode）：把 uid 一并交给会话创建——会话归属该用户后，
    // agent-loop 的文明线**自动写入**（经组合根注册的写入器工厂）也会落到
    // `per_user.civ_for(uid)`，与本函数的公告及 API 读侧**同一个档**（此前自动写入
    // 落全局档 ⇒ `hearth civ feed` 看不到；见 main.rs `CivWriterAdapter`）。
    match state.sessions.create_session_with_owner(req, &uid).await {
        Ok(resp) => Ok((StatusCode::CREATED, Json(resp))),
        Err(e) => Err(api_err(
            ERR_INVALID_PARAM,
            format!("failed to create session: {e}"),
            StatusCode::BAD_REQUEST,
        )),
    }
}

/// P0-3 (audit-fix): GET /api/v1/sessions — 会话列表（内存活跃 + 持久化并集）。
///
/// D-171：**按调用者 uid 过滤**（多租户隔离）——修复前无归属条件，任一租户可见全部会话 id。
pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let uid = get_user_id(&headers, &state.user_store);
    let sessions = state.sessions.list_sessions_json_for(&uid).await;
    Ok(Json(serde_json::json!({ "sessions": sessions })))
}

/// GET /api/v1/sessions/:id
pub async fn get_session_status(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // D-171：归属校验（不存在 / 非本人 → 404，同一形状）。
    ensure_session_owner(&state, &headers, &id).await?;
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
                _ => Err(session_not_found(&id)),
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
    ApiJson(req): ApiJson<MessageReq>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // D-171：归属校验（不存在 / 非本人 → 404）——在任何副作用之前。
    ensure_session_owner(&state, &headers, &id).await?;
    // Last-Event-ID: 客户端已收到的最后 seq → 重放缓冲中 seq 之后的事件
    let last_seq = headers
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    // D-178（2026-10-08, traecode）：错误**分类**——修复前 Err 一律映射
    // `404 SESSION_NOT_FOUND`（与 D-166 同族：会话其实存在，却报"不存在"；且把
    // "会话已结束/在跑"这种**客户端时序错误**冒充成"目标不存在"）。
    // 现按"会话是否**存活于内存**"分类：存活 ⇒ 400 `INVALID_PARAM`（请求与当前状态不符）；
    // 不存活（仅持久化/未知）⇒ 404。判据与 D-166 对 approvals/interaction 的处置同款。
    let live = session_live(&state, &id).await;
    match state.sessions.send_message(&id, req).await {
        Ok(rx) => {
            let (base, replay) = state.sessions.session_events_with_base(&id).await;
            Ok(sse::sse_stream_with_replay(rx, replay, base, last_seq))
        }
        Err(e) => Err(if live {
            api_err(ERR_INVALID_PARAM, format!("{e}"), StatusCode::BAD_REQUEST)
        } else {
            session_not_found(&id)
        }),
    }
}

/// WP-2 (v23 phase3): GET /api/v1/sessions/:id/events —— 录制导出（JSONL，每行一个信封事件）。
pub async fn session_events_export(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // D-171：归属校验（不存在 / 非本人 → 404）——不得导出他人会话事件（含目标文本）。
    ensure_session_owner(&state, &headers, &id).await?;
    // D-174（2026-10-08, traecode）：`ensure_session_owner` **已保证会话存在**（内存或持久化）
    // ⇒ 此处**不得**再因"内存缓冲为空"而报 404（D-166：会话存在即不得报 SESSION_NOT_FOUND；
    // 修复前对**重启后的持久化会话**导出恒 404）。导出 = 实时内存缓冲；非存活会话由
    // `session_events_jsonl` 回落**持久化录制**（两态同形状）。
    let lines = state.sessions.session_events_jsonl(&id).await;
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
    // D-171：归属校验（不存在 / 非本人 → 404）——不得订阅他人会话实时流。
    ensure_session_owner(&state, &headers, &id).await?;
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
    headers: axum::http::HeaderMap,
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
    // D-171：归属校验（不存在 / 非本人 → 404）——不得读他人会话工作区产物。
    // D-179：**存活**判据——仅持久化的会话（无工作区句柄）应 404，不得报 400。
    ensure_session_owner(&state, &headers, &id).await?;
    let live = session_live(&state, &id).await;
    match state.sessions.open_artifact(&id, &rel).await {
        Ok(content) => Ok((
            StatusCode::OK,
            [(
                axum::http::header::CONTENT_TYPE,
                "text/plain; charset=utf-8",
            )],
            content,
        )),
        Err(e) => Err(classify_live_error(&e, live, &id)),
    }
}

/// B3-3 (backend taskbook #01): 系统打开产物——file → xdg-open/默认程序，
/// url → 默认浏览器。路径校验在 SessionManager::open_external（workspace 内）。
pub async fn open_external(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
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
    // D-171：归属校验（不存在 / 非本人 → 404）——不得对他人会话工作区发起系统打开。
    // D-179：**存活**判据——仅持久化的会话应 404（不得报 400 且文案自相矛盾）。
    ensure_session_owner(&state, &headers, &id).await?;
    let live = session_live(&state, &id).await;
    match state.sessions.open_external(&id, &rel).await {
        Ok(msg) => Ok((StatusCode::OK, msg)),
        Err(e) => Err(classify_live_error(&e, live, &id)),
    }
}

/// D-166（2026-10-05, traecode）：交互响应的**错误分类**收口。
///
/// D-179（2026-10-08, traecode）：本函数已**并入** [`classify_live_error`]（同一逻辑，
/// 且判据由"存在"收紧为"存活"）——原 `classify_interaction_error(&e, true, &id)` 的**硬编码
/// `true`** 正是"仅持久化会话被误报 400"的根因。此处不再保留独立函数（避免逐字重复=漂移陷阱）。
///
/// POST /api/v1/sessions/:id/approvals
/// WP-0: 保留为薄适配层（deprecated）——新契约走 POST /interaction/{iid}。
pub async fn submit_approval(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<ApprovalReq>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // D-171：归属校验（不存在 / 非本人 → 404）——不得代他人会话审批。
    ensure_session_owner(&state, &headers, &id).await?;
    // D-166 + D-179：按"会话是否**存活于内存**"分类——存活 ⇒ 400 `INVALID_PARAM`（请求级）；
    // 仅持久化 / 未加载 ⇒ 404（无 dispatcher 交互项，D-171 后不得再硬编码成 400）。
    let live = session_live(&state, &id).await;
    match state.sessions.submit_approval(&id, req).await {
        Ok(()) => Ok((StatusCode::OK, Json(serde_json::json!({"status": "ok"})))),
        Err(e) => Err(classify_live_error(&e, live, &id)),
    }
}

/// WP-0 (v23 §2.1): POST /api/v1/sessions/:id/interaction/:iid —— 通用交互响应。
/// R1 id 强校验 + R2 一次性消费由 dispatcher 保证；payload 内容内核不解析。
pub async fn submit_interaction(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path((id, iid)): Path<(String, String)>,
    ApiJson(req): ApiJson<api::InteractionResponse>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    if req.id != iid {
        return Err(api_err(
            api::ERR_INVALID_PARAM,
            format!("path iid {iid} != body id {}", req.id),
            StatusCode::BAD_REQUEST,
        ));
    }
    // D-171：归属校验（不存在 / 非本人 → 404）——不得代他人会话应答交互。
    ensure_session_owner(&state, &headers, &id).await?;
    // D-166 + D-179：按"存活于内存"分类（同 `submit_approval`）。
    let live = session_live(&state, &id).await;
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
        Err(e) => Err(classify_live_error(&e, live, &id)),
    }
}

/// POST /api/v1/sessions/:id/cancel
pub async fn cancel_session(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // D-171：归属校验（不存在 / 非本人 → 404）——不得停他人会话。
    ensure_session_owner(&state, &headers, &id).await?;
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
///
/// D-187（2026-10-08, traecode）：改为返回**契约型** `api::ModelsResponse`（原为临时
/// `json!({"providers": providers})`）——序列化形状不变（`{"providers":[{"provider":…,"model":…}]}`），
/// 但契约与实现自此对齐（`ApiDoc` schemas 收录 `ModelsResponse`）。
pub async fn list_models(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // P4: expose all registered providers from the registry
    let sessions = state.sessions.clone();
    // SessionManager doesn't directly expose registry, but we can list
    // registered providers from the models that were registered at startup.
    // For now, return a dynamically discovered list via the session manager.
    let providers = sessions.list_providers();
    Json(api::ModelsResponse { providers })
}

/// B1 (trunk-freeze): GET /api/v1/sessions/:id/messages
pub async fn get_session_history(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // D-171：归属校验（不存在 / 非本人 → 404）——不得读他人会话消息历史（含目标文本）。
    ensure_session_owner(&state, &headers, &id).await?;
    match state.sessions.get_history(&id).await {
        Ok(Some(history)) => Ok((StatusCode::OK, Json(history))),
        Ok(None) => Err(session_not_found(&id)),
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

/// D-182（2026-10-08, traecode）：civ 写**结果**记账——成功**清零**、失败自增。
///
/// 病灶（HA/可用性）：`civ_write_failures` 原先**只增不减**（`CivWriterAdapter` 两处
/// `fetch_add`、全仓无清零），而 `/readyz` 在 `>5` 时判 503 ⇒ 与适配器注释自称的
/// "**连续**失败"不符（实现是**累计**失败）：一次**瞬时**抖动累计过阈后，该实例
/// **永久**不 ready（负载均衡永久摘除，直到进程重启），即便故障早已恢复——把"可自愈的
/// 抖动"放大成"实例下线"。
///
/// 现收敛为**真·连续**语义：任一成功清零。抽成本函数以便单测——适配器在**二进制 crate**
/// （`main.rs`）内，集成测试无法直接触达其计数逻辑。
pub fn record_civ_write(failures: &std::sync::atomic::AtomicU64, ok: bool) {
    use std::sync::atomic::Ordering;
    if ok {
        failures.store(0, Ordering::SeqCst);
    } else {
        failures.fetch_add(1, Ordering::SeqCst);
    }
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
///
/// D-157（2026-10-05, traecode）：本响应此前是**纯文本** `too many requests (…)` 且无
/// `Content-Type: application/json`，绕过契约「**所有** API 错误返回
/// `{"error":{"code","message"}}`」/ B2「全部 handler 返回 `Json<ErrorResponse>`」
/// （中间件层未被 B2 覆盖）。现归一为统一 `ErrorResponse`，`code = RATE_LIMITED`。
/// 回归锁：`crates/service/tests/rate_limit_error_shape_gate.rs`。
fn rate_limit_response(reason: &str) -> Response {
    let mut resp = (
        StatusCode::TOO_MANY_REQUESTS,
        Json(ErrorResponse::new(
            api::ERR_RATE_LIMITED,
            format!("too many requests ({reason})"),
        )),
    )
        .into_response();
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
    ApiJson(body): ApiJson<serde_json::Value>,
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
    ApiJson(body): ApiJson<serde_json::Value>,
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
    ApiJson(body): ApiJson<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // D-164（2026-10-05, traecode）：收紧 PATCH 语义——`progress`/`status` 均为**部分更新**：
    // 键缺失 = 不改；键在则必须合法。修复前三处病灶：① 未知 status 静默回落 `Pending`
    // （客户端拼错被吞）；② `progress` 是必填 f32 ⇒ 只改 status 的请求把进度**静默重置为 0**
    // （"Completed 但 0%" 自相矛盾）；③ 节点不存在也返回 200（假成功，见下方 `hit`）。
    let progress: Option<f32> = match body.get("progress") {
        None => None,
        Some(v) => {
            let f = v.as_f64().map(|x| x as f32).ok_or_else(|| {
                api_err(
                    ERR_INVALID_PARAM,
                    "progress must be a number",
                    StatusCode::BAD_REQUEST,
                )
            })?;
            // D-162：非有限值（如 `1e40` 溢出 f32）会污染为 `null` 并致重载丢节点。
            if !f.is_finite() {
                return Err(api_err(
                    ERR_INVALID_PARAM,
                    "progress must be a finite number",
                    StatusCode::BAD_REQUEST,
                ));
            }
            Some(f)
        }
    };
    let status: Option<WorkStatus> = match body.get("status") {
        None => None,
        Some(v) => {
            let s = v.as_str().ok_or_else(|| {
                api_err(
                    ERR_INVALID_PARAM,
                    "status must be a string",
                    StatusCode::BAD_REQUEST,
                )
            })?;
            Some(match s {
                "pending" => WorkStatus::Pending,
                "in_progress" => WorkStatus::InProgress,
                "blocked" => WorkStatus::Blocked,
                "review" => WorkStatus::Review,
                "completed" => WorkStatus::Completed,
                other => {
                    return Err(api_err(
                        ERR_INVALID_PARAM,
                        format!(
                            "unknown status: {other} \
                             (expected pending|in_progress|blocked|review|completed)"
                        ),
                        StatusCode::BAD_REQUEST,
                    ))
                }
            })
        }
    };
    let uid = get_user_id(&headers, &state.user_store);
    let hit = state
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
    if !hit {
        // D-164：节点不存在必须是 404（修复前静默跳过 + 200 假成功）。
        return Err(api_err(
            api::ERR_NOT_FOUND,
            format!("workline node not found: {id}"),
            StatusCode::NOT_FOUND,
        ));
    }
    Ok(StatusCode::OK)
}

// ── v6.1 L1: Bridge integration ──

/// POST /api/v1/bridge — create and run a multi-model bridge discussion.
/// D-161（P1-127）：bridge 参与者的**硬上界**。
///
/// `bridge::BridgeSession` 对每个参与者**每轮**发一次 LLM 调用（`max_rounds` 在
/// `SessionManager::create_bridge` 内固定为 3）⇒ 无上界时，一份 2 MiB 体（`DefaultBodyLimit`）
/// 塞满同名**已注册** provider 即可驱动约 210 万次顺序调用（成本/资源 DoS，认证客户端亦可为）。
/// 取 8：多模型讨论的合理规模，最坏 8×3=24 次调用/请求。
const MAX_BRIDGE_PARTICIPANTS: usize = 8;

pub async fn create_bridge(
    State(state): State<Arc<AppState>>,
    ApiJson(body): ApiJson<serde_json::Value>,
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
    // D-161：上界校验——把 `participants` 当不可信入参（无界扇出的闸门）。
    if participants.len() > MAX_BRIDGE_PARTICIPANTS {
        return Err(api_err(
            ERR_INVALID_PARAM,
            format!(
                "too many participants ({}, max {MAX_BRIDGE_PARTICIPANTS})",
                participants.len()
            ),
            StatusCode::BAD_REQUEST,
        ));
    }
    // D-161：名字必须先能被 registry 解析——否则是**客户端**拼错 provider 名，返回 400；
    // 修复前一路进 bridge，`registry.get()` 失败被下面的 `map_err` 冒充成 500 INTERNAL。
    if let Some(bad) = participants
        .iter()
        .find(|p| !state.sessions.has_provider(p))
    {
        return Err(api_err(
            ERR_INVALID_PARAM,
            format!("unknown provider: {bad}"),
            StatusCode::BAD_REQUEST,
        ));
    }
    // D-177（2026-10-08, traecode）：`strategy` 的**严格校验**——修复前 `_ => RoundRobin`
    // 把**未知/拼错的策略名静默当成 round_robin**（客户端 `"voting"`/`"debate "` 被吞，
    // 讨论按错误策略跑完且**无从知晓**；与 D-164「未知 workline status 静默回落 Pending」
    // 同族）。口径：键**缺失**才取默认（文档语义），键**在**则必须是合法字符串且属
    // `{round_robin,debate,majority}`，否则 **400 INVALID_PARAM**（非字符串同样 400，
    // 与 D-164「status must be a string」一致）。
    let strategy = match body.get("strategy") {
        None => bridge::BridgeStrategy::RoundRobin,
        Some(v) => {
            let s = v.as_str().ok_or_else(|| {
                api_err(
                    ERR_INVALID_PARAM,
                    "strategy must be a string",
                    StatusCode::BAD_REQUEST,
                )
            })?;
            match s {
                "round_robin" => bridge::BridgeStrategy::RoundRobin,
                "debate" => bridge::BridgeStrategy::Debate,
                "majority" => bridge::BridgeStrategy::MajorityVote,
                other => {
                    return Err(api_err(
                        ERR_INVALID_PARAM,
                        format!(
                            "unknown strategy: {other} \
                             (expected round_robin|debate|majority)"
                        ),
                        StatusCode::BAD_REQUEST,
                    ))
                }
            }
        }
    };
    // D-183（2026-10-08, traecode）：**上游失败分类**——此处已是**唯一**剩余错误源
    // （入参/归属校验全部前置），其失败即 bridge 真去调 provider 失败 ⇒ 契约表的
    // `LLM_ERROR`→**502**。修复前一律 `500 INTERNAL`：① 与契约表不符（该码从不发出，
    // 客户端按 code/状态码分支永远走不到）；② 语义上把"**上游可重试**"与"**服务端缺陷**"
    // 混为一谈（文案本就写着 "transient provider error"，码却说 internal——自相矛盾）。
    let result = state
        .sessions
        .create_bridge(topic, participants, strategy)
        .await
        .map_err(|e| api_err(ERR_LLM_ERROR, format!("{e}"), StatusCode::BAD_GATEWAY))?;
    Ok(Json(
        serde_json::json!({"id": result.0, "summary": result.1, "turns": result.2}),
    ))
}

// ── v7.0: Telemetry ──

use std::sync::atomic::AtomicU64;

/// v7.0: Simple in-memory telemetry collector.
///
/// D-176（2026-10-08, traecode）：原三字段中 `error_count` / `steps_total` **全仓零
/// `fetch_add`**（唯一写入方 `session_count` 在 `create_session` 自增）⇒ `/api/v1/telemetry`
/// 恒报两个**结构性 0**，却被命名为"实时指标"（`global-panorama-v10.1.md` 已记录该现象，
/// 驱动文档此前未跟踪）。按 D-107 先例（退役 `reuse_rate`——"拿结构性 0 当业务指标比不报更糟"），
/// **删除**这两个无写入方的字段；若日后要报，须**先接线到消费点**再谈。
pub struct TelemetryCollector {
    /// 累计**创建**的会话数（`create_session` 自增；单调不减——**非**"活跃数"）。
    pub session_count: AtomicU64,
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        Self {
            session_count: AtomicU64::new(0),
        }
    }
}

/// GET /api/v1/telemetry — real-time metrics.
///
/// D-176：只报**有真实写入方**的计数（`session_count`）。原 `error_count`/`steps_total`
/// 恒 0（无写入方）已删——不再对外提供假指标。
pub async fn get_telemetry(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tc = &state.telemetry;
    Json(serde_json::json!({
        "session_count": tc.session_count.load(Ordering::Relaxed),
    }))
}

/// v8.0: GET /api/v1/templates — list available agent templates.
pub async fn list_templates(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let templates: Vec<_> = state.template_manager.list().into_iter().cloned().collect();
    Json(serde_json::json!({ "templates": templates }))
}

/// v8.0: POST /api/v1/webhooks — register a webhook.
///
/// D-170（2026-10-05, traecode）：**注册时校验** url——修复前 `register` 只 push、恒成功，
/// 对"永远投不出去"的 url 也回 `{"ok":true}`（假成功），且 curl 选项注入形 url 在注册面即被放行。
pub async fn register_webhook(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    ApiJson(mut cfg): ApiJson<crate::webhook::WebhookConfig>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // D-172：owner 由**服务端**按调用者 uid 强制写入（请求体即便带了 `owner` 也被覆盖，
    // 不可伪造）——否则任一租户可伪造 owner 去订阅他人事件。
    cfg.owner = get_user_id(&headers, &state.user_store);
    match state.webhooks.try_register(cfg) {
        Ok(()) => Ok(Json(serde_json::json!({"ok": true}))),
        Err(e) => Err(api_err(
            ERR_INVALID_PARAM,
            format!("invalid webhook url: {e}"),
            StatusCode::BAD_REQUEST,
        )),
    }
}

/// v10.0: GET /api/v1/resources — system resource snapshot.
///
/// D-176（2026-10-08, traecode）：原键名 `sessions_active` 读的是**累计**创建数
/// （`session_count`，单调不减、从不回退）——**名不副实**（"活跃"意味着有回退的当前量）。
/// 已如实改名为 `sessions_created_total`（CLI `whoami` 只读 `instance_id`，不受影响）。
pub async fn get_resources(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let pid = std::process::id();
    let snap = resource_monitor::snapshot(pid);
    Json(serde_json::json!({
        "instance_id": &state.instance_id,
        "snapshot": snap,
        "sessions_created_total": state.telemetry.session_count.load(std::sync::atomic::Ordering::Relaxed),
    }))
}

/// v10.0: GET /api/v1/tools — list registered tools.
pub async fn list_tools() -> impl IntoResponse {
    Json(serde_json::json!({ "tools": tool_names() }))
}

/// D-149（2026-10-04, traecode）：`CODEX_DISABLE_TOOLS`（逗号分隔、忽略大小写）→ 禁用**键**集合。
///
/// 此前这段解析只存在于 `main.rs` 的注册循环里，而本文件旧注释把 env 名误写成
/// `HEARTH_DISABLED_TOOLS`——照注释配置即**静默失效**（工具照旧启用）。现收敛到本函数，
/// **注册与 `GET /api/v1/tools` 共用同一份判定**。
pub fn disabled_tools_from_env() -> std::collections::HashSet<String> {
    std::env::var("CODEX_DISABLE_TOOLS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

/// P1-11（D-44）：**内置工具表 = 唯一事实源**。
///
/// 首元素是 `CODEX_DISABLE_TOOLS` 的**键**（其中 `edit`→`write_file` 是历史别名，
/// **勿改**——改了会变配置语义：操作者按显示名 `write_file` 写会静默失效）。
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

/// D-149：**实际启用**的内置工具——`builtin_tools()` 过滤掉 `disabled` 里的键。
///
/// 注册（`main.rs`）与 `GET /api/v1/tools` 都必须经此：否则会出现
/// "禁用已生效、API 却仍报它可用"（D-44 同族复发——D-44 只修了**硬编码名单**漂移，
/// 未修**禁用过滤**这一路；实测 `CODEX_DISABLE_TOOLS=bash` 下 `/api/v1/tools` 仍列出 bash）。
pub fn enabled_builtin_tools(
    disabled: &std::collections::HashSet<String>,
) -> Vec<(&'static str, Arc<dyn tool_runtime::Tool>)> {
    builtin_tools()
        .into_iter()
        .filter(|(key, _)| !disabled.iter().any(|d| d == key))
        .collect()
}

/// `GET /api/v1/tools` 的载荷：**实际启用**的内置工具名。
///
/// 名字取 `Tool::name()`（与 LLM/dispatcher 所见一致；`edit` 键对应显示名 `write_file`）。
/// 唯一事实源 = `enabled_builtin_tools()`（与启动注册同一份判定）→ 工具增删/禁用
/// 只需改那一处，路由与注册自动同步。
pub fn tool_names() -> Vec<String> {
    enabled_builtin_tools(&disabled_tools_from_env())
        .into_iter()
        .map(|(_, t)| t.name().to_string())
        .collect()
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

    /// D-149 回归锁：`GET /api/v1/tools` 的载荷必须反映**禁用过滤**。
    ///
    /// 修复前 `tool_names()` 直接映射静态 `builtin_tools()`、**无视** `CODEX_DISABLE_TOOLS`
    /// ⇒ 被禁用的工具仍被"声称可用"（**活体实证**：`CODEX_DISABLE_TOOLS=bash` 起服务后
    /// `hearth tools` 仍列出 `bash`）。本测试用**显式入参**（不碰进程 env）以免并行竞态。
    #[test]
    fn test_d149_enabled_tools_respect_disable_filter() {
        use std::collections::HashSet;

        let names = |d: &HashSet<String>| -> Vec<String> {
            enabled_builtin_tools(d)
                .into_iter()
                .map(|(_, t)| t.name().to_string())
                .collect()
        };

        let none: HashSet<String> = HashSet::new();
        let all = names(&none);
        assert!(all.iter().any(|n| n == "bash"), "前置：未禁用时含 bash");

        let bash_off: HashSet<String> = ["bash".to_string()].into_iter().collect();
        let f1 = names(&bash_off);
        assert!(
            !f1.iter().any(|n| n == "bash"),
            "禁用 `bash` 后仍报 bash 可用：{f1:?}"
        );
        assert!(f1.iter().any(|n| n == "read"), "禁用过滤误伤了其它工具");

        // 历史别名：禁用**键**是 `edit`，而**显示名**是 `write_file`。
        let edit_off: HashSet<String> = ["edit".to_string()].into_iter().collect();
        let f2 = names(&edit_off);
        assert!(
            !f2.iter().any(|n| n == "write_file"),
            "按注册键 `edit` 禁用后仍报 write_file 可用：{f2:?}"
        );

        // 记录别名语义（防被当 bug 改掉）：按**显示名** `write_file` 写**不生效**。
        let by_display: HashSet<String> = ["write_file".to_string()].into_iter().collect();
        assert_eq!(
            names(&by_display).len(),
            all.len(),
            "`write_file` 不是注册键、不应生效——若此处失败说明 `edit`→`write_file` 别名语义被改了"
        );
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

#[cfg(test)]
mod d182_tests {
    use super::{civ_store_degraded, record_civ_write};
    use std::sync::atomic::{AtomicU64, Ordering};

    /// D-182 回归锁（**先红后绿**）：civ 写失败计数必须是**连续**语义——任一成功**清零**。
    ///
    /// 修复前该计数**只增不减**（`CivWriterAdapter` 两处 `fetch_add`、全仓无清零）：
    /// 6 次瞬时失败后 `/readyz` **永久** 503（HA：LB 永久摘除该实例），即便故障已恢复。
    #[test]
    fn test_d182_civ_write_failures_reset_on_success() {
        let f = AtomicU64::new(0);

        // 连续 6 次失败 → 越过阈值（>5）→ degraded（与既有 readyz 口径一致）。
        for _ in 0..6 {
            record_civ_write(&f, false);
        }
        assert_eq!(f.load(Ordering::SeqCst), 6);
        assert!(
            civ_store_degraded(f.load(Ordering::SeqCst)),
            "应判 degraded"
        );

        // 一次**成功**即清零 ⇒ 恢复 ready（修复前此处仍为 6 ⇒ 永久 503）。
        record_civ_write(&f, true);
        assert_eq!(
            f.load(Ordering::SeqCst),
            0,
            "**连续**语义：成功必须清零（修复前只增不减 ⇒ 永久 degraded）"
        );
        assert!(
            !civ_store_degraded(f.load(Ordering::SeqCst)),
            "清零后必须恢复 ready"
        );

        // 单次失败仍会自增（不破坏"暴露连续失败"的本意）。
        record_civ_write(&f, false);
        assert_eq!(f.load(Ordering::SeqCst), 1);
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
    ApiJson(req): ApiJson<InstallToolReq>,
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
///
/// D-175（2026-10-08, traecode）：**按调用者 uid** 读该租户的经验档（`per_user.experience_for`），
/// 而不是全局单例——修复前所有租户共用一份指标（且会话也写进同一文件）。
pub async fn experience_metrics(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Json<serde_json::Value> {
    let uid = get_user_id(&headers, &state.user_store);
    let m = state.per_user.experience_for(&uid).metrics().await;
    Json(serde_json::to_value(m).unwrap_or_default())
}
