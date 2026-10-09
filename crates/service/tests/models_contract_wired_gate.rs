//! D-187（P1-149）：`/api/v1/models` 的**契约↔实现接线**门禁（源码级）。
//!
//! 背景（病灶，"声称≠实现" + "临时 json! 冒充契约"）：
//!   · `api::ModelInfo { provider, model }` **被 `ApiDoc` 的 `components(schemas(…))` 声明**
//!     （`service/src/main.rs`），即对外宣称"这是模型信息的契约型"；
//!   · 但全仓**零构造**——`SessionManager::list_providers` 手搓
//!     `serde_json::json!({"provider": name, "model": model})` 并返回 `Vec<serde_json::Value>`，
//!     `routes::list_models` 再包成 `{"providers": …}` 的**临时** JSON。
//!   ⇒ 契约里有一个"真形状"的类型（`ModelInfo` 的字段与手搓 JSON **逐字段一致**），却**从未被用**；
//!     而 `ApiDoc` 声明的是**元素**类型、真实响应外层是 `{"providers": […]}`——**缺一层类型**。
//!
//! 修复口径（**接线而非退役**——因为手搓 JSON 与 `ModelInfo` 字段完全一致，是纯粹"没用类型"）：
//!   ① `api` 新增 `ModelsResponse { providers: Vec<ModelInfo> }`（真实响应外层）；
//!   ② `SessionManager::list_providers` 返回 `Vec<api::ModelInfo>`（不再手搓 JSON）；
//!   ③ `routes::list_models` 返回 `api::ModelsResponse`；
//!   ④ `ApiDoc` schemas 收录 `ModelsResponse`（`ModelInfo` 保留）。
//!   **序列化形状不变**（`{"providers":[{"provider":…,"model":…}]}`）⇒ 对现有客户端**非破坏**。
//!
//! 本门禁是**源码级**（修复前后都能编译）：断言 ① `api` 有 `ModelsResponse`；
//! ② `list_providers` 不再返回 `Vec<serde_json::Value>`；③ `main.rs` schemas 收录 `ModelsResponse`；
//! ④ 反向对照：`ModelInfo` **仍存在**（它是真形状，不得被误删）。
//!
//! 盲区（本套件**不**覆盖，勿误当全覆盖）：
//!   - 只 pin 接线点；运行时**形状不变**由 `models_response_shape_gate.rs` 锁。
//!   - 剥注释用行级 `//`（与既有源码级门禁同口径）。

use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// 读源文件并逐行剥掉 `//` 之后的注释（保留代码）。
fn code_only(src: &str) -> String {
    src.lines()
        .map(|l| match l.find("//") {
            Some(i) => &l[..i],
            None => l,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn read_code(path: &std::path::Path) -> String {
    code_only(
        &std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("读 {} 失败: {e}", path.display())),
    )
}

#[test]
fn models_endpoint_is_wired_to_contract_types() {
    let root = workspace_root();
    let api = read_code(&root.join("crates/api/src/lib.rs"));
    let runtime = read_code(&root.join("crates/agent-runtime/src/session.rs"));
    let service_main = read_code(&root.join("crates/service/src/main.rs"));

    // ① 真实响应外层类型必须存在
    assert!(
        api.contains("pub struct ModelsResponse"),
        "D-187：`api` 必须有 `ModelsResponse`（`/api/v1/models` 的**真实**响应外层 \
         `{{\"providers\": […]}}`）——否则契约里缺一层，Swagger/codegen 拿不到外层形状"
    );
    // ② 不得再手搓临时 JSON 冒充契约
    assert!(
        !runtime.contains("pub fn list_providers(&self) -> Vec<serde_json::Value>"),
        "D-187 回潮：`list_providers` 又返回 `Vec<serde_json::Value>`（手搓 `json!`）了——\
         应返回 `Vec<api::ModelInfo>`（其字段与手搓 JSON 逐字段一致，即\"没用上现成契约型\"）"
    );
    // ③ 契约文档必须收录真实外层类型
    assert!(
        service_main.contains("api::ModelsResponse"),
        "D-187：`ApiDoc` 的 `components(schemas(…))` 必须收录 `api::ModelsResponse`\
         （否则明明类型化了、文档仍缺该形状）"
    );
    // ④ 反向对照：`ModelInfo` 是**真形状**（元素型），必须保留
    assert!(
        api.contains("pub struct ModelInfo"),
        "对照失败：`ModelInfo` 是 `/api/v1/models` 的**元素**真形状，必须保留（不得误删）"
    );
}
