//! D-156（P1-122）：提取器级拒绝的**统一错误形状**门禁。
//!
//! 契约口径（本轮取证）：
//!   - `docs/handoff-trunk-freeze.md`：「**所有** API 错误返回
//!     `{"error":{"code":"...","message":"..."}}` 格式」；
//!   - `docs/governance.md` B2：「**全部 handler** 返回 `Json<ErrorResponse>`」。
//!
//! 病灶（"声称≠实现" / "半接线"）：B2 统一形状只覆盖了 **handler 内部** 主动返回的
//! 错误（`(StatusCode, Json<ErrorResponse>)`），**未覆盖 axum 提取器级拒绝**——缺/错
//! `Content-Type` → 415、畸形 JSON → 400，二者直接吐**纯文本**体 + `text/plain`
//! Content-Type。按 JSON 解析错误体的客户端（Swagger UI / 第三方 SDK）在 4xx 上必失败。
//!
//! 本套件两部分：
//!   ① 运行时门禁：起**真实 service**，断言**提取器级**（415/400/413）与**路由级**
//!      （未知路由 404 / 方法不允许 405，D-158）错误的响应体是可解析的 JSON
//!      `{"error":{"code","message"}}`，且 `Content-Type` 为 `application/json`。
//!   ② 源码级回归锁：钉住 `routes.rs` 不再出现裸 `Json<T>` 提取器（防新 handler 回退），
//!      并断言 `ApiJson` 包装确实被采用。
//!
//! 盲区（本套件**不**覆盖，勿误当全覆盖）：
//!   - 运行时只探代表性端点（sessions / civilization）；其余 handler 由 ② 源码级锁定代偿。
//!   - **`DefaultBodyLimit` 超限（413）实为同一条 `Json` 拒绝链**（`JsonRejection::BytesRejection`），
//!     已被 `ApiJson` 一并归一，并有断言 ④ 锁定 ⇒ 不再列为"盲区"。
//!   - 路径/查询参数拒绝（`PathRejection`/`QueryRejection`）：本服务所有 `Path` 均为
//!     `String`/`(String,String)`、所有 `Query` 均为 `HashMap<String,String>` ⇒ **永不拒绝**
//!     （无红侧、未纳入本卡；将来若引入强类型 Path/Query，须同法收口）。
//!   - auth 中间件的 401/403 本就走 B2 统一形状；并发限流 429 曾返回**纯文本**，已由
//!     **D-157** 归一并由 `rate_limit_error_shape_gate.rs` 锁定（本套件不重复覆盖）。

use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// RAII 守卫：无论测试如何退出（含断言 panic / 提前 return），都回收子进程。
///
/// 必要性（本轮踩到）：断言失败时若只靠末尾 `kill()`，panic 会让子进程成为孤儿并
/// **占用 `target/debug/service.exe`**，导致后续 `cargo build` 报 `os error 5 拒绝访问`；
/// 且子进程若继承 stdin 会吊住上层的管道。故：① `stdin/stdout/stderr` 全部 null；
/// ② 用 `Drop` 兜底 kill。
struct ServiceGuard(Child);

impl Drop for ServiceGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// 起真实 service（ALLOW_NO_AUTH=1 回环模式），端口可连即视为健康。
fn wait_healthy(port: u16) -> ServiceGuard {
    let svc = env!("CARGO_BIN_EXE_service");
    let tmp = std::env::temp_dir().join(format!("wf-api-err-shape-{}", std::process::id()));
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

/// 断言一个 4xx 响应体是统一 JSON 错误形状，且 Content-Type 为 JSON。
fn assert_unified_error_shape(what: &str, ct: &str, body: &str) {
    assert!(
        ct.contains("application/json"),
        "[{what}] 提取器级错误的 Content-Type 应为 application/json，实际 {ct:?}；body={body:?}"
    );
    let v: serde_json::Value = serde_json::from_str(body).unwrap_or_else(|e| {
        panic!("[{what}] 提取器级错误体应为 JSON，解析失败: {e}；body={body:?}")
    });
    assert!(
        v["error"]["code"].is_string(),
        "[{what}] 缺 error.code（契约形状 {{\"error\":{{\"code\",\"message\"}}}}）：{body:?}"
    );
    assert!(
        v["error"]["message"].is_string(),
        "[{what}] 缺 error.message：{body:?}"
    );
}

#[test]
fn extractor_rejections_return_unified_json_error() {
    let _svc = wait_healthy(3933);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let base = "http://127.0.0.1:3933";
    // 关闭空闲连接复用：④ 的 3 MiB 体被 413 拒绝后服务端未排空该体、会关连接，
    // 复用该池化连接会让 ⑤ 报 `ConnectionAborted`（客户端侧噪声，非被测行为）。
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()
        .expect("build http client");

    // ① 错 Content-Type → 415：此前体为纯文本
    //    `Expected request with ` + backtick + `Content-Type: application/json` + backtick
    let r = rt.block_on(async {
        client
            .post(format!("{base}/api/v1/sessions"))
            .header("content-type", "text/plain")
            .body(r#"{"goal":"x"}"#)
            .send()
            .await
            .expect("send ①")
    });
    assert!(
        r.status().is_client_error(),
        "① 错 Content-Type 应 4xx，实际 {}",
        r.status()
    );
    let ct = r
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body = rt.block_on(r.text()).expect("read body ①");
    assert_unified_error_shape("①text/plain on /sessions", &ct, &body);

    // ② 畸形 JSON → 400：此前体为纯文本 `Failed to parse the request body as JSON: ...`
    let r2 = rt.block_on(async {
        client
            .post(format!("{base}/api/v1/sessions"))
            .header("content-type", "application/json")
            .body("{bad json")
            .send()
            .await
            .expect("send ②")
    });
    assert!(
        r2.status().is_client_error(),
        "② 畸形 JSON 应 4xx，实际 {}",
        r2.status()
    );
    let ct2 = r2
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body2 = rt.block_on(r2.text()).expect("read body ②");
    assert_unified_error_shape("②malformed json on /sessions", &ct2, &body2);

    // ③ 第二个端点（civilization）同族复现，证明不是某 handler 的个例
    let r3 = rt.block_on(async {
        client
            .post(format!("{base}/api/v1/civilization"))
            .header("content-type", "application/json")
            .body("{bad json")
            .send()
            .await
            .expect("send ③")
    });
    assert!(
        r3.status().is_client_error(),
        "③ 畸形 JSON 应 4xx，实际 {}",
        r3.status()
    );
    let ct3 = r3
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body3 = rt.block_on(r3.text()).expect("read body ③");
    assert_unified_error_shape("③malformed json on /civilization", &ct3, &body3);

    // ④ 超 DefaultBodyLimit（2 MiB）→ 413：同属 `Json` 拒绝链（BytesRejection），应被同一包装归一
    let big = format!("{{\"goal\":\"{}\"}}", "A".repeat(3 * 1024 * 1024));
    let r4 = rt.block_on(async {
        client
            .post(format!("{base}/api/v1/sessions"))
            .header("content-type", "application/json")
            .body(big)
            .send()
            .await
            .expect("send ④")
    });
    assert_eq!(
        r4.status().as_u16(),
        413,
        "④ 超限体应 413，实际 {}",
        r4.status()
    );
    let ct4 = r4
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body4 = rt.block_on(r4.text()).expect("read body ④");
    assert_unified_error_shape("④oversize body on /sessions", &ct4, &body4);

    // ⑤ 未知路由 → 统一 JSON 404（此前 axum 默认：空体、无 Content-Type）
    let r5 = rt.block_on(async {
        client
            .get(format!("{base}/api/v1/definitely-not-a-route"))
            .send()
            .await
            .expect("send ⑤")
    });
    assert_eq!(r5.status().as_u16(), 404, "⑤ 未知路由应 404");
    let ct5 = r5
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body5 = rt.block_on(r5.text()).expect("read body ⑤");
    assert_unified_error_shape("⑤unknown route", &ct5, &body5);

    // ⑥ 路径存在但方法不允许 → 统一 JSON 405（此前 axum 默认：空体）
    let r6 = rt.block_on(async {
        client
            .delete(format!("{base}/api/v1/sessions"))
            .send()
            .await
            .expect("send ⑥")
    });
    assert_eq!(r6.status().as_u16(), 405, "⑥ 方法不允许应 405");
    // RFC 7231：405 必须带 Allow；自定义 fallback 不得把它弄丢（axum 在 route 层回填）。
    assert!(
        r6.headers().get("allow").is_some(),
        "⑥ 405 应保留 Allow 头（自定义 fallback 不得丢失）"
    );
    let ct6 = r6
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body6 = rt.block_on(r6.text()).expect("read body ⑥");
    assert_unified_error_shape("⑥method not allowed", &ct6, &body6);

    eprintln!("D-156/158 PASS: 提取器级 + 路由级（404/405）错误均为统一 JSON 形状");
}

/// 源码级回归锁：`routes.rs` 的 handler 不得再用裸 `Json<T>` 提取器。
///
/// 判据：提取器位置的特征是 `ident: Json<...>`；函数**返回类型**是 `-> Json<...>`
/// （不含子串 `: Json<`），故 `: Json<` 可精确区分"取"与"返"。
/// 逐行先剥 `//` 注释再判，避免"注释点名旧写法 ⇒ 自命中"（D-147/D-151/D-153 反复踩到）。
#[test]
fn routes_use_api_json_extractor_not_raw_json() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = std::fs::read_to_string(root.join("src/routes.rs")).expect("read routes.rs");
    let mut offenders: Vec<String> = Vec::new();
    for (i, raw) in src.lines().enumerate() {
        let line = match raw.find("//") {
            Some(p) => &raw[..p],
            None => raw,
        };
        if line.contains(": Json<") {
            offenders.push(format!("{}: {}", i + 1, raw.trim()));
        }
    }
    assert!(
        offenders.is_empty(),
        "routes.rs 仍在使用裸 Json<T> 提取器（提取器级拒绝会绕过 B2 统一形状）；\
         请改用 ApiJson<T>。违规行：{offenders:?}"
    );
    assert!(
        src.contains("pub struct ApiJson<"),
        "routes.rs 缺少 ApiJson 包装提取器定义"
    );
}
