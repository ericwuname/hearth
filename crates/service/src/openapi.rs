//! OpenAPI 文档构建（D-90，2026-10-02, traecode；**D-186 收口，2026-10-08**）。
//!
//! **病灶（D-90）**：`ApiDoc` 的 `#[openapi(...)]` 里只有 `components(schemas(...))`，
//! 注释写着 "path annotations deferred"。于是 `/openapi.json` 对外提供的是一份
//! **声明了 0 个端点的 OpenAPI 文档**——而服务实际注册了 28 条路由；`/swagger-ui`
//! （已 vendor 的交互 UI）读的正是这份文档 ⇒ 打开浏览器看到的是**空 API**。属
//! "半接线活特性"（同 D-48 的 civ 线 / D-79 的会话账本）。**处置：接线而非退役**——
//! 为不改动 28 个 handler（逐个手写 `#[utoipa::path]` 极易再次漂移），改为
//! **运行时注入 paths**：由 [`API_ROUTES`] 镜像表驱动 [`build_openapi_document`]。
//!
//! **D-186 收口（2026-10-08）**：D-90 只注入了 `paths` + `summary`，**每个 operation 的
//! `request_body` / `responses` 全为空**；而 `ApiDoc` 的 `components.schemas` 里的
//! `api::*` 类型**没有任何 `$ref` 指向它们**（components 与 operations 之间**零引用
//! 关系**）。本模块注释此前却自称"响应体仍以 `api` crate 的类型为准，已由 `ApiDoc`
//! 的 `components(schemas(...))` 声明"——**半真**（声明了 schema，但**未接线**）。
//! 后果：Swagger UI 打开每个端点**看不到请求/响应体**；按 json 生成的客户端方法**无类型**
//! ——正是 D-90 想治的"空 API"病灶的**残留形态**。
//!
//! 现据实分两层修复：
//! ① **已建模的操作**（[`OPERATION_IO`]）——为其建 `requestBody` / `responses`
//!    （`Ref::from_schema_name`），引用 `ApiDoc` 已声明的 `api::*` schema；
//! ② **尚未建模的操作**——**如实不声明** body（给一个 `default` 响应 + 描述，
//!    **不**编造 schema、**不**声称 content-type）。这不是"遗漏"：这些端点的响应体
//!    尚未收敛为 `api::*` 契约型，凭空编造 schema 只会制造**新的**"声称≠实现"。
//!
//! 两张表的职责与保真：
//! - [`API_ROUTES`]＝**路由镜像表**（path + method + summary）——保真由
//!   `crates/service/tests/openapi_route_gate.rs` 双向兜底（比对 `Router::route(...)`
//!   字面量：路由有、文档无 / 文档有、路由无）。
//! - [`OPERATION_IO`]＝**body 建模表**（仅列已有 `api::*` 契约型的操作）——与镜像表
//!   **解耦**（避免为补 body 而改动已稳定的镜像表）；二者 (path, method) 的一致性、
//!   以及"operation 引用的 schema 必须已被 `ApiDoc` 声明"由
//!   `crates/service/tests/openapi_operation_io_gate.rs` 兜底。
//! 路径写法：两表均用 **OpenAPI 形式** `{id}`，路由器用 axum 形式 `:id`，门禁负责归一化。

use utoipa::openapi::path::Operation;
use utoipa::openapi::request_body::RequestBody;
use utoipa::openapi::{Content, PathItem, Ref, Response, Responses};

/// 与 axum 路由器逐条对应的（路径，[(方法, 摘要)]）镜像表。
///
/// 只声明"路径 + 方法 + 摘要"——**不含请求/响应 schema**。D-186：body 的接线**不在此表**，
/// 而在 [`OPERATION_IO`]（仅列已有 `api::*` 契约型的操作）；未列入者**如实不声明** body。
pub const API_ROUTES: &[(&str, &[(&str, &str)])] = &[
    ("/healthz", &[("get", "存活探针（免鉴权）")]),
    (
        "/readyz",
        &[("get", "就绪探针（含 civ 写失败降级计数；>5 次 → 503）")],
    ),
    (
        "/api/v1/sessions",
        &[("get", "列出会话"), ("post", "创建会话")],
    ),
    ("/api/v1/sessions/{id}", &[("get", "查询会话状态")]),
    (
        "/api/v1/sessions/{id}/messages",
        &[("post", "发送用户消息"), ("get", "读取会话历史回放")],
    ),
    (
        "/api/v1/sessions/{id}/approvals",
        &[("post", "提交审批结果")],
    ),
    (
        "/api/v1/sessions/{id}/interaction/{iid}",
        &[("post", "提交通用交互响应（approvals 的泛化契约）")],
    ),
    ("/api/v1/sessions/{id}/cancel", &[("post", "取消会话")]),
    (
        "/api/v1/sessions/{id}/events",
        &[("get", "录制导出（JSONL 信封事件流）")],
    ),
    ("/api/v1/sessions/{id}/stream", &[("get", "实时 SSE 流")]),
    (
        "/api/v1/sessions/{id}/artifact/open",
        &[("get", "产物预览（路径校验限定 workspace 内）")],
    ),
    (
        "/api/v1/sessions/{id}/artifact/open-external",
        &[("get", "用系统程序打开产物（file→xdg-open / url→浏览器）")],
    ),
    ("/api/v1/models", &[("get", "列出可用模型")]),
    ("/openapi.json", &[("get", "本 OpenAPI 文档")]),
    (
        "/api/v1/civilization",
        &[("get", "文明线：读取条目流"), ("post", "文明线：追加条目")],
    ),
    ("/api/v1/workline", &[("get", "工作线：读取任务板")]),
    ("/api/v1/workline/nodes", &[("post", "工作线：新建节点")]),
    (
        "/api/v1/workline/nodes/{id}",
        &[("patch", "工作线：更新节点")],
    ),
    ("/api/v1/bridge", &[("post", "桥接讨论")]),
    ("/api/v1/telemetry", &[("get", "遥测快照")]),
    ("/api/v1/templates", &[("get", "列出提示模板")]),
    ("/api/v1/webhooks", &[("post", "注册 webhook")]),
    ("/api/v1/resources", &[("get", "资源占用快照")]),
    ("/api/v1/tools", &[("get", "列出已注册工具")]),
    ("/api/v1/tools/search", &[("get", "搜索工具生态")]),
    ("/api/v1/tools/install", &[("post", "安装工具")]),
    (
        "/api/v1/tool-registry",
        &[("get", "列出已登记工具清单（含 TOOLS_DIR 自动发现的）")],
    ),
    // D-107：经验库为**只写审计档**（追加 / 剪枝 / 真实计数）；指标只含
    // 条数 / 高质量率 / 失败率——原先对外暴露的 `reuse_rate` 已删（它读的
    // `reference_count` 无写入方、恒 0，属"把结构性 0 当业务指标"）。
    (
        "/api/v1/experience/metrics",
        &[("get", "经验库指标（审计档：条数 / 高质量率 / 失败率）")],
    ),
];

/// [`OPERATION_IO`] 的元素：
/// `(path, method, request_schema, response_schema, success_status, content_type)`。
pub type OperationIo = (
    &'static str,
    &'static str,
    Option<&'static str>,
    Option<&'static str>,
    &'static str,
    &'static str,
);

/// 已建模"请求/响应体"的操作（D-186，2026-10-08, traecode）。
///
/// 每项 = `(path, method, request_schema, response_schema, success_status, content_type)`：
/// - `request_schema` / `response_schema`：`ApiDoc` 的 `components(schemas(...))` **已声明**的
///   `api::*` 类型名（`Ref::from_schema_name` 的目标）。`None` 表示该操作**无请求体**，或
///   响应体**尚未收敛为契约型**——此时**如实不声明**，**绝不编造 schema**（凭空造 schema
///   且与真实响应不符，只会制造新的"声称≠实现"）。
/// - `success_status`：成功响应状态码（`"200"` / `"201"`，据各 handler 实际返回）。
/// - `content_type`：成功响应体媒体类型（JSON / SSE / NDJSON）。
///
/// **未列入本表的操作**：`build_openapi_document` 只给一个 `default` 响应 + 描述，**不含 body**。
/// 逐步补建模时，请**同时**把类型加进 `main.rs` 的 `ApiDoc` `components(schemas(...))`，
/// 否则 `openapi_operation_io_gate` 会报"引用了未声明的 schema"。
pub const OPERATION_IO: &[OperationIo] = &[
    // GET /api/v1/models —— D-187 起响应已收敛为契约型 `ModelsResponse`。
    (
        "/api/v1/models",
        "get",
        None,
        Some("ModelsResponse"),
        "200",
        "application/json",
    ),
    // POST /api/v1/sessions —— 请求 `SessionCreate`；成功 **201** `SessionCreateResponse`。
    (
        "/api/v1/sessions",
        "post",
        Some("SessionCreate"),
        Some("SessionCreateResponse"),
        "201",
        "application/json",
    ),
    // GET /api/v1/sessions/{id} —— 状态 `SessionStatus`（内存命中或持久化回落，均 200）。
    (
        "/api/v1/sessions/{id}",
        "get",
        None,
        Some("SessionStatus"),
        "200",
        "application/json",
    ),
    // GET /api/v1/sessions/{id}/messages —— 历史回放 `SessionHistory`。
    (
        "/api/v1/sessions/{id}/messages",
        "get",
        None,
        Some("SessionHistory"),
        "200",
        "application/json",
    ),
    // POST /api/v1/sessions/{id}/messages —— 请求 `MessageReq`；响应为 **SSE 流**（无 JSON schema）。
    (
        "/api/v1/sessions/{id}/messages",
        "post",
        Some("MessageReq"),
        None,
        "200",
        "text/event-stream",
    ),
    // POST /api/v1/sessions/{id}/approvals —— 请求 `ApprovalReq`（响应为精简 `{"status":"ok"}`，未建模）。
    (
        "/api/v1/sessions/{id}/approvals",
        "post",
        Some("ApprovalReq"),
        None,
        "200",
        "application/json",
    ),
    // POST /api/v1/sessions/{id}/interaction/{iid} —— 请求 `InteractionResponse`（同上）。
    (
        "/api/v1/sessions/{id}/interaction/{iid}",
        "post",
        Some("InteractionResponse"),
        None,
        "200",
        "application/json",
    ),
    // GET /api/v1/sessions/{id}/events —— JSONL 录制导出（每行一个信封事件）。
    (
        "/api/v1/sessions/{id}/events",
        "get",
        None,
        None,
        "200",
        "application/x-ndjson",
    ),
    // GET /api/v1/sessions/{id}/stream —— 实时 SSE 流。
    (
        "/api/v1/sessions/{id}/stream",
        "get",
        None,
        None,
        "200",
        "text/event-stream",
    ),
];

/// 把 [`API_ROUTES`] 注入 `ApiDoc` 生成的文档，并按 [`OPERATION_IO`] 接线每个 operation 的
/// 请求/响应体（D-186）。
///
/// `base` 由调用方传入（`ApiDoc::openapi()` 位于二进制 crate `main.rs`，
/// 故通过参数反转依赖，避免 lib 反向依赖 bin）。
pub fn build_openapi_document(mut base: utoipa::openapi::OpenApi) -> utoipa::openapi::OpenApi {
    for (path, ops) in API_ROUTES {
        let mut item = PathItem::default();
        for (method, summary) in *ops {
            let op = build_operation(path, method, summary);
            match *method {
                "get" => item.get = Some(op),
                "post" => item.post = Some(op),
                "patch" => item.patch = Some(op),
                "put" => item.put = Some(op),
                "delete" => item.delete = Some(op),
                other => {
                    // 镜像表里写了本函数不认识的方法：显式失败，绝不静默丢端点
                    panic!("API_ROUTES 出现未支持的方法 {other}（{path}）——请同步 build_openapi_document");
                }
            }
        }
        base.paths.paths.insert((*path).to_string(), item);
    }
    base
}

/// 组装单个 operation：`summary` +（若已建模）`requestBody` / `responses`（D-186）。
///
/// - 命中 [`OPERATION_IO`]：按建模表接 `requestBody` 与 `"<status>"` 响应（含 content ref）；
/// - 未命中：给一个 `default` 响应 + 如实描述，**不含 body**（不编造 schema / content-type）。
fn build_operation(path: &str, method: &str, summary: &str) -> Operation {
    let mut op = Operation::default();
    op.summary = Some(summary.to_string());
    op.responses = match OPERATION_IO
        .iter()
        .find(|(p, m, ..)| *p == path && *m == method)
    {
        Some((_, _, req, resp, status, content_type)) => {
            if let Some(req_schema) = req {
                op.request_body = Some(
                    RequestBody::builder()
                        .content(
                            "application/json",
                            Content::new(Some(Ref::from_schema_name(*req_schema))),
                        )
                        .build(),
                );
            }
            let content = match resp {
                Some(schema) => Content::new(Some(Ref::from_schema_name(*schema))),
                None => Content::new(None::<Ref>),
            };
            Responses::builder()
                .response(
                    *status,
                    Response::builder()
                        .description("成功响应")
                        .content(*content_type, content)
                        .build(),
                )
                .build()
        }
        None => Responses::builder()
            .response(
                "default",
                Response::builder()
                    .description(
                        "（响应体尚未收敛为契约型；详见 crates/service/src/openapi.rs 的 OPERATION_IO）",
                    )
                    .build(),
            )
            .build(),
    };
    op
}
