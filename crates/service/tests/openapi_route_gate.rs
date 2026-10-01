//! 门禁：**`/openapi.json` 的 paths 必须与真实注册的路由逐条一致**（D-90，2026-10-02, traecode）。
//!
//! 背景：该端点此前只声明了 schemas、**零 paths**，对外提供的是一份"声明了 0 个端点
//! 的 OpenAPI 文档"（而 `/swagger-ui` 读的正是它 ⇒ 打开即空接线列表）。现在由
//! `service::openapi::API_ROUTES` 镜像表注入 paths；本门禁负责**保真**：
//!
//! 1. **双向比对**：`API_ROUTES` ↔ `main.rs` 里 `Router::route("…", …)` 的字面量集合，
//!    任一侧多/少都报红（路由有、文档无 = 契约缺项；文档有、路由无 = 幽灵端点）。
//! 2. **内容断言**：构建出的文档 paths 条数必须等于镜像表条数（防止"注入逻辑被改坏/
//!    被绕开"后仍全绿——这正是本卡要治的"零 paths 也照样绿"）。
//!
//! 路径归一化：路由器用 axum 形式 `:id`，OpenAPI 用 `{id}`——比对前统一成 `{id}`。
//!
//! 文件头自报盲区：① 只解析 `.route("…")` 字面量，**mounted 的 `nest_service`**
//! （如 `/swagger-ui` 静态 UI）不算 API 端点、不参与比对；② 只比对**路径集合**，
//! 不比对方法与 handler 绑定（方法差异由镜像表自带、人工维护）。

use std::collections::BTreeSet;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// axum 路径形式（`:id`）→ OpenAPI 形式（`{id}`）。
fn normalize(path: &str) -> String {
    path.split('/')
        .map(|seg| {
            if let Some(name) = seg.strip_prefix(':') {
                format!("{{{name}}}")
            } else {
                seg.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// 从源码文本里抽出所有 `.route("<path>"` 的字面量路径。
fn router_paths(source: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = source;
    while let Some(i) = rest.find(".route(") {
        let after = &rest[i + ".route(".len()..];
        let trimmed = after.trim_start();
        if let Some(stripped) = trimmed.strip_prefix('"') {
            if let Some(end) = stripped.find('"') {
                out.insert(normalize(&stripped[..end]));
            }
        }
        rest = after;
    }
    out
}

#[test]
fn openapi_paths_match_router_routes() {
    let root = workspace_root();
    let main_rs = root
        .join("crates")
        .join("service")
        .join("src")
        .join("main.rs");
    let src = std::fs::read_to_string(&main_rs).expect("main.rs 必须存在");
    let routed = router_paths(&src);

    let documented: BTreeSet<String> = service::openapi::API_ROUTES
        .iter()
        .map(|(p, _)| p.to_string())
        .collect();

    assert!(
        routed.len() >= 20,
        "解析到的路由过少（{} 条）——门禁自身可能失效：{routed:?}",
        routed.len()
    );
    assert!(
        documented.len() >= 20,
        "镜像表过小（{} 条）——门禁自身可能失效",
        documented.len()
    );

    let missing_in_doc: Vec<&String> = routed.difference(&documented).collect();
    let ghost_in_doc: Vec<&String> = documented.difference(&routed).collect();
    assert!(
        missing_in_doc.is_empty() && ghost_in_doc.is_empty(),
        "OpenAPI 文档与真实路由**不一致**：\n\
         · 路由有、文档无（契约缺项，用户按 json 生成客户端会漏调）：{missing_in_doc:?}\n\
         · 文档有、路由无（幽灵端点，调用必 404）：{ghost_in_doc:?}\n\
         请在 `crates/service/src/openapi.rs` 的 `API_ROUTES` 与 `main.rs` 的 `.route(...)` 间对齐。"
    );

    // 内容断言：注入必须真的发生（本卡病灶 = "零 paths 也照样全绿"）。
    let doc = service::openapi::build_openapi_document(utoipa::openapi::OpenApi::default());
    assert_eq!(
        doc.paths.paths.len(),
        service::openapi::API_ROUTES.len(),
        "构建出的文档 paths 条数与镜像表不符——注入逻辑被改坏或绕开了\
         （修复前这里恒为 0：文档对外是一份空 API 契约）"
    );
    assert!(
        doc.paths.paths.contains_key("/api/v1/sessions"),
        "核心端点必须出现在文档里"
    );
}
