//! 门禁：**`/openapi.json` 的 operations 必须真的接上 request/response body**（D-186，2026-10-08, traecode）。
//!
//! 背景：D-90 只注入了 `paths` + `summary`——每个 operation 的 `request_body` /
//! `responses` **全为空**，`components.schemas` 里的 `api::*` 类型**没有任何 `$ref`
//! 指向它们**（components 与 operations 之间**零引用关系**）。Swagger UI 打开每个
//! 端点看不到请求/响应体；按 json 生成的客户端方法无类型。
//!
//! 本门禁对 `build_openapi_document` 的**产物**（序列化后的 JSON）做断言：
//! 1. **已建模操作必须接线**：`/api/v1/models`、`POST /api/v1/sessions`、
//!    `POST .../{id}/messages` 等——其 `responses` / `requestBody` 必须带
//!    `$ref` 指向对应 `api::*` schema（修复前这里恒为空 ⇒ 红）。
//! 2. **引用不得悬空**：产物里出现的每个 `#/components/schemas/<Name>`，其 `<Name>`
//!    必须在 `main.rs` 的 `ApiDoc` `components(schemas(...))` 里**已声明**（防止
//!    "接线了但 schema 没登记" → Swagger UI 解析失败）。
//! 3. **未建模操作不得编造**：如 `/healthz`——只有 `default` 响应、**无 content**
//!    （如实不声明，而非伪造 schema）。
//!
//! 文件头自报盲区：① 只断言**已列入 `OPERATION_IO` 的少数端点**的接线，其余端点
//! 的响应体**尚未建模**（本门禁不覆盖其形状正确性）；② schema 声明检查是**源码文本
//! 匹配**（`main.rs` 含 `api::<Name>`），不校验 schema 内容本身。

use std::collections::BTreeSet;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// 构建文档并序列化为 JSON（`base` 用默认 OpenApi —— schema 声明由 `main.rs` 的
/// `ApiDoc` 负责，本门禁只验 operations 的接线与引用名）。
fn built_doc_json() -> serde_json::Value {
    let doc = service::openapi::build_openapi_document(utoipa::openapi::OpenApi::default());
    let text = doc.to_json().expect("OpenAPI 文档必须能序列化");
    serde_json::from_str(&text).expect("序列化结果必须是合法 JSON")
}

fn get<'a>(v: &'a serde_json::Value, ptr: &str) -> Option<&'a serde_json::Value> {
    v.pointer(ptr)
}

#[test]
fn openapi_operations_carry_request_response_bodies() {
    let doc = built_doc_json();

    // ① 已建模操作：响应体带 `$ref` → ModelsResponse。
    assert_eq!(
        get(
            &doc,
            "/paths/~1api~1v1~1models/get/responses/200/content/application~1json/schema/$ref"
        )
        .and_then(|v| v.as_str()),
        Some("#/components/schemas/ModelsResponse"),
        "GET /api/v1/models 的 200 响应必须引用 ModelsResponse（修复前 responses 为空）"
    );

    // ② POST /api/v1/sessions：请求体 SessionCreate + 201 响应 SessionCreateResponse。
    assert_eq!(
        get(
            &doc,
            "/paths/~1api~1v1~1sessions/post/requestBody/content/application~1json/schema/$ref"
        )
        .and_then(|v| v.as_str()),
        Some("#/components/schemas/SessionCreate"),
        "POST /api/v1/sessions 的 requestBody 必须引用 SessionCreate"
    );
    assert_eq!(
        get(
            &doc,
            "/paths/~1api~1v1~1sessions/post/responses/201/content/application~1json/schema/$ref"
        )
        .and_then(|v| v.as_str()),
        Some("#/components/schemas/SessionCreateResponse"),
        "POST /api/v1/sessions 的 201 响应必须引用 SessionCreateResponse"
    );

    // ③ POST .../{id}/messages：请求体 MessageReq；响应媒体类型为 SSE（无 JSON schema）。
    assert_eq!(
        get(
            &doc,
            "/paths/~1api~1v1~1sessions~1{id}~1messages/post/requestBody/content/application~1json/schema/$ref"
        )
        .and_then(|v| v.as_str()),
        Some("#/components/schemas/MessageReq"),
        "POST .../messages 的 requestBody 必须引用 MessageReq"
    );
    assert!(
        get(
            &doc,
            "/paths/~1api~1v1~1sessions~1{id}~1messages/post/responses/200/content/text~1event-stream"
        )
        .is_some(),
        "POST .../messages 的 200 响应必须如实声明 text/event-stream 媒体类型"
    );

    // ④ GET .../{id}/messages：历史回放 SessionHistory。
    assert_eq!(
        get(
            &doc,
            "/paths/~1api~1v1~1sessions~1{id}~1messages/get/responses/200/content/application~1json/schema/$ref"
        )
        .and_then(|v| v.as_str()),
        Some("#/components/schemas/SessionHistory"),
        "GET .../messages 的 200 响应必须引用 SessionHistory"
    );

    // ⑤ 未建模操作（/healthz）：只有 default 响应、**无 content**（如实不声明）。
    let healthz =
        get(&doc, "/paths/~1healthz/get/responses").expect("/healthz 的 get 操作必须有 responses");
    assert!(
        healthz.get("default").is_some(),
        "/healthz 未建模，应给 default 响应：{healthz}"
    );
    assert!(
        healthz.get("default").unwrap().get("content").is_none(),
        "/healthz 未建模，**不得**编造 content/schema：{healthz}"
    );
}

#[test]
fn openapi_refs_resolve_to_declared_schemas() {
    // 产物里出现的每个 `#/components/schemas/<Name>` 都必须在 main.rs 的
    // ApiDoc `components(schemas(...))` 里声明——否则引用悬空（Swagger UI 解析失败）。
    let json = built_doc_json().to_string();
    let marker = "#/components/schemas/";
    let mut names = BTreeSet::new();
    let mut rest = json.as_str();
    while let Some(i) = rest.find(marker) {
        let after = &rest[i + marker.len()..];
        let end = after.find(['"', '}']).unwrap_or(after.len());
        if end > 0 {
            names.insert(after[..end].to_string());
        }
        rest = after;
    }
    assert!(
        !names.is_empty(),
        "产物里一个 schema 引用都没有——D-186 接线未生效"
    );

    let main_rs = workspace_root()
        .join("crates")
        .join("service")
        .join("src")
        .join("main.rs");
    let src = std::fs::read_to_string(&main_rs).expect("main.rs 必须存在");
    let undeclared: Vec<&String> = names
        .iter()
        .filter(|name| !src.contains(&format!("api::{name}")))
        .collect();
    assert!(
        undeclared.is_empty(),
        "operations 引用了**未在 ApiDoc components(schemas(...)) 声明**的 schema：{undeclared:?}\n\
         请在 `crates/service/src/main.rs` 的 ApiDoc schemas 列表补上对应 `api::<Type>`。"
    );
}
