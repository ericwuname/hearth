//! D-187（P1-149）：`GET /api/v1/models` 的**运行时形状**门禁（契约↔实现对齐 + 非破坏）。
//!
//! 背景：本端点原来返回**临时** `json!({"providers": providers})`，而 `ApiDoc` 只声明了
//! **元素**型 `api::ModelInfo`（真实外层 `{"providers": […]}` **无类型**、且 `ModelInfo` 零构造）。
//! 修复把两端收敛到契约型 `api::ModelsResponse { providers: Vec<ModelInfo> }`。
//!
//! 本套件锁两件事：
//!   ① **非破坏**：修复改变了返回类型，但**序列化形状必须逐字段不变**
//!      （顶层仅 `providers`；元素仅 `provider` / `model`）——否则现有客户端会碎。
//!   ② **对齐**：响应体能**反序列化进契约型** `api::ModelsResponse`（证明"文档里的类型 = 真实形状"）。
//!
//! 盲区（本套件**不**覆盖）：
//!   - 不校验 `providers` 的**内容**（那取决于启动期注册了哪些 provider；本套件只锁**形状**）。
//!   - 不覆盖 OpenAPI 文档侧的收录（由源码级 `models_contract_wired_gate.rs` 锁）。

use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

struct ServiceGuard(Child);

impl Drop for ServiceGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_healthy(port: u16) -> ServiceGuard {
    let svc = env!("CARGO_BIN_EXE_service");
    let tmp = std::env::temp_dir().join(format!("wf-models-shape-gate-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("create tmp memory dir");
    let child = Command::new(svc)
        .env("PORT", port.to_string())
        .env("MEMORY_DIR", &tmp)
        .env("ALLOW_NO_AUTH", "1")
        .env("OPENAI_API_KEY", "sk-test-placeholder-for-contract-test")
        .env("RUST_LOG", "warn")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn service");
    let guard = ServiceGuard(child);
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            std::thread::sleep(Duration::from_millis(300));
            return guard;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("service did not become healthy on port {port}");
}

#[test]
fn models_response_shape_unchanged_and_matches_contract_type() {
    let _svc = wait_healthy(3951);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let base = "http://127.0.0.1:3951";
    let c = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()
        .expect("build http client");

    let (status, body) = rt.block_on(async {
        let r = c
            .get(format!("{base}/api/v1/models"))
            .send()
            .await
            .expect("get models");
        let st = r.status().as_u16();
        let text = r.text().await.unwrap_or_default();
        (st, text)
    });
    assert_eq!(status, 200, "/api/v1/models 应 200；body={body}");

    let v: serde_json::Value = serde_json::from_str(&body).expect("必须是 JSON");

    // ① 顶层**只有** `providers`（D-187 之前也是这个形状——非破坏要求）。
    let obj = v.as_object().expect("顶层必须是对象");
    assert_eq!(
        obj.keys().collect::<Vec<_>>(),
        vec!["providers"],
        "顶层字段必须**恰好**是 `providers`（序列化形状不得因本次类型化而改变）；实得 {body}"
    );

    // ② 元素**只有** `provider` / `model`（与 `ModelInfo` 的字段集一致）。
    //    注意：JSON 对象的**键序无意义**（`serde_json` 的 Map 默认按字典序），故按**集合**比较。
    if let Some(arr) = obj["providers"].as_array() {
        for item in arr {
            let keys: std::collections::BTreeSet<&str> = item
                .as_object()
                .unwrap_or_else(|| panic!("元素必须是对象：{item}"))
                .keys()
                .map(|k| k.as_str())
                .collect();
            let expected: std::collections::BTreeSet<&str> =
                ["provider", "model"].into_iter().collect();
            assert_eq!(
                keys, expected,
                "元素字段集必须恰好是 provider/model（`ModelInfo` 的字段）；实得 {item}"
            );
        }
    } else {
        panic!("`providers` 必须是数组；实得 {}", obj["providers"]);
    }

    // ③ **对齐**：响应体必须能反序列化进契约型 `api::ModelsResponse`。
    let typed: api::ModelsResponse =
        serde_json::from_str(&body).expect("响应必须可反序列化为契约型 api::ModelsResponse");
    assert_eq!(
        typed.providers.len(),
        obj["providers"].as_array().map(|a| a.len()).unwrap_or(0),
        "契约型解析出的条数必须与 JSON 一致"
    );

    eprintln!("D-187 PASS: /api/v1/models 形状不变且与契约型 ModelsResponse 对齐");
}
