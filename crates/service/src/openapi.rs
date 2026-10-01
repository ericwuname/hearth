//! OpenAPI 文档构建（D-90，2026-10-02, traecode）。
//!
//! **病灶**：`ApiDoc` 的 `#[openapi(...)]` 里只有 `components(schemas(...))`，注释写着
//! "path annotations deferred"。于是 `/openapi.json` 对外提供的是一份**声明了 0 个端点
//! 的 OpenAPI 文档**——而服务实际注册了 27 条路由；`/swagger-ui`（已 vendor 的交互 UI）
//! 读的正是这份文档 ⇒ 打开浏览器看到的是**空 API**。属"半接线活特性"
//! （同 D-48 的 civ 线 / D-79 的会话账本）：端点活着、消费者也活着，缺的只是**内容**。
//!
//! **处置：接线而非退役**——补全 `paths`。为不改动 27 个 handler（逐个手写
//! `#[utoipa::path]` 极易再次漂移），改为**运行时注入 paths**：由 [`API_ROUTES`]
//! 镜像表驱动 [`build_openapi_document`]。
//!
//! [`API_ROUTES`] 是**镜像表而非唯一事实源**（axum 各 handler 函数类型互不相同，
//! 无法用同一张表同时注册路由与生成文档），所以它的保真由**门禁**兜底：
//! `crates/service/tests/openapi_route_gate.rs` 逐条比对它与 `Router::route(...)`
//! 的字面量，**双向**报告缺失（路由有、文档无 / 文档有、路由无）。
//! 路径写法：本表用 **OpenAPI 形式** `{id}`，路由器用 axum 形式 `:id`，门禁负责归一化。

use utoipa::openapi::path::Operation;
use utoipa::openapi::PathItem;

/// 与 axum 路由器逐条对应的（路径，[(方法, 摘要)]）镜像表。
///
/// 只声明"路径 + 方法 + 摘要"——**不含响应 schema**（响应体仍以 `api` crate 的类型为准，
/// 已由 `ApiDoc` 的 `components(schemas(...))` 声明）。
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
    ("/api/v1/experience/metrics", &[("get", "经验库指标")]),
];

/// 把 [`API_ROUTES`] 注入 `ApiDoc` 生成的文档。
///
/// `base` 由调用方传入（`ApiDoc::openapi()` 位于二进制 crate `main.rs`，
/// 故通过参数反转依赖，避免 lib 反向依赖 bin）。
pub fn build_openapi_document(mut base: utoipa::openapi::OpenApi) -> utoipa::openapi::OpenApi {
    for (path, ops) in API_ROUTES {
        let mut item = PathItem::default();
        for (method, summary) in *ops {
            let mut op = Operation::default();
            op.summary = Some((*summary).to_string());
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
