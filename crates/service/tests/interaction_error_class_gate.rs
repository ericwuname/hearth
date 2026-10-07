//! D-166（P1-132）：交互响应的**错误分类**门禁。
//!
//! 病灶（错误分类 / B2，承 D-161 同族）：`routes::submit_approval` 与 `routes::submit_interaction`
//! 把**任何**失败一律映射成 **404 + `ERR_SESSION_NOT_FOUND`**。但
//! `tool_runtime::ToolDispatcher::resolve_interaction` 的失败**全是请求级**（`interaction_id`
//! 不匹配 / 无 pending 交互 / 无交互态），`submit_approval` 另有一条"非法 decision"——这些都会
//! 把**客户端拼错 / 重复提交**冒充成"**会话不存在**"（客户端按 `code` 分支会误判会话丢失）。
//! 正确对照：`get_session_history` 已正确区分 `Ok(None)`→404 与 `Err`→500。
//!
//! 修复口径：**只有会话真的不存在**才是 404/`SESSION_NOT_FOUND`；会话存在时，上述失败一律
//! **400 `INVALID_PARAM`**（判据复用 `SessionManager::get_session`，与 manager 同一判定）。
//!
//! 盲区（本套件**不**覆盖）：
//!   - 不覆盖"重复提交已消费交互"应有的 **409** 语义（本卡统一归 400；细分需 manager 返回类型化错误）。
//!   - 不覆盖 dispatcher 内部错误（`resolve_interaction` 当前**没有**内部失败分支，见其实现）。

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
    let tmp = std::env::temp_dir().join(format!("wf-interact-gate-{}", std::process::id()));
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

/// 返回 (status, code)。
async fn post_json_code(
    client: &reqwest::Client,
    url: String,
    body: serde_json::Value,
) -> (u16, String) {
    let resp = client.post(url).json(&body).send().await.expect("send");
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    let code = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| v["error"]["code"].as_str().map(str::to_string))
        .unwrap_or_else(|| format!("<no-json-error:{text}>"));
    (status, code)
}

#[test]
fn interaction_failures_are_not_all_session_not_found() {
    let _svc = wait_healthy(3938);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let base = "http://127.0.0.1:3938";
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()
        .expect("build http client");

    // 建一个**存在**的会话
    let sid: String = rt.block_on(async {
        let v: serde_json::Value = client
            .post(format!("{base}/api/v1/sessions"))
            .json(&serde_json::json!({"provider": "openai", "goal": "d166"}))
            .send()
            .await
            .expect("create session")
            .json()
            .await
            .expect("parse");
        v["session_id"].as_str().expect("session_id").to_string()
    });

    // ① 会话存在 + 非法 decision → 400 INVALID_PARAM（修复前：404 SESSION_NOT_FOUND）
    let (s1, c1) = rt.block_on(post_json_code(
        &client,
        format!("{base}/api/v1/sessions/{sid}/approvals"),
        serde_json::json!({"approval_id": "nope", "decision": "yes"}),
    ));
    assert_eq!(
        s1, 400,
        "① 会话存在时的非法 decision 应 400（客户端错误），实得 {s1}（code={c1}）"
    );
    assert_ne!(
        c1, "SESSION_NOT_FOUND",
        "① 会话**存在**，不得报 `SESSION_NOT_FOUND`（客户端按 code 会误判会话丢失）"
    );

    // ② 会话存在 + 无 pending 交互 → 400（修复前：404 SESSION_NOT_FOUND）
    let weird = "no-such-interaction-d166";
    let (s2, c2) = rt.block_on(post_json_code(
        &client,
        format!("{base}/api/v1/sessions/{sid}/interaction/{weird}"),
        serde_json::json!({"id": weird, "by": "t", "resolved": true, "payload": {}}),
    ));
    assert_eq!(
        s2, 400,
        "② 会话存在但无 pending 交互应 400，实得 {s2}（code={c2}）"
    );
    assert_ne!(
        c2, "SESSION_NOT_FOUND",
        "② 会话**存在**，不得报 `SESSION_NOT_FOUND`"
    );

    // ③ 回归守卫：**会话真不存在**时仍须 404 + SESSION_NOT_FOUND
    let (s3, c3) = rt.block_on(post_json_code(
        &client,
        format!("{base}/api/v1/sessions/no-such-session-d166/approvals"),
        serde_json::json!({"approval_id": "x", "decision": "approve"}),
    ));
    assert_eq!(s3, 404, "③ 会话不存在应 404，实得 {s3}");
    assert_eq!(
        c3, "SESSION_NOT_FOUND",
        "③ 会话不存在时仍须 `SESSION_NOT_FOUND`（防修复误伤），实得 {c3}"
    );

    eprintln!("D-166 PASS: 交互失败按『会话是否存在』分类（存在→400 / 不存在→404）");
}
