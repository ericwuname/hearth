//! D-179（P1-143）：**"仅持久化（非存活）会话"上的错误分类**门禁。
//!
//! 背景（病灶，D-171 引入的漂移）：`ensure_session_owner` 的"存在"是"内存**或**持久化"，
//! 但**实时**端点（开产物 / 系统打开 / 审批 / 交互）要的是**内存中活跃**的会话——仅持久化的
//! 会话（重启后未被加载）没有工作区句柄、没有 dispatcher 交互项。D-171 把这几处的"显式存在性
//! 检查"换成 `ensure_session_owner` 后，内层错误被硬编码成 400 ⇒ 对仅持久化会话返回
//! **400 `INVALID_PARAM`** 且 message 写着 `session not found: …`（**码与文案自相矛盾**）；
//! 而**同一 id** 在 `cancel_session` / `session_stream` / `send_message`（D-178）上正确报 **404**。
//!
//! 修复口径（与 D-166/D-178 同口径）：统一以 **"是否存活于内存"** 分类——
//! 不存活 ⇒ 404 `SESSION_NOT_FOUND`；存活而内层失败 ⇒ 400 `INVALID_PARAM`。
//!
//! 盲区（本套件**不**覆盖，勿误当全覆盖）：
//!   - 只锁"仅持久化"这一条边界；"存活会话内层失败仍 400"用 ⑦/⑧ 作防误伤对照（同一套件内）。
//!   - 不覆盖真实 artifact 内容路径（需真实会话工作区产物）。

use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// 仅持久化、未被加载的会话 id（预置 JSONL 文件名须等于该 id）。
const PERSISTED_ID: &str = "d179-persisted";

struct ServiceGuard(Child);

impl Drop for ServiceGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_healthy(port: u16, tmp: &std::path::Path) -> ServiceGuard {
    let svc = env!("CARGO_BIN_EXE_service");
    let child = Command::new(svc)
        .env("PORT", port.to_string())
        .env("MEMORY_DIR", tmp)
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
fn persisted_only_session_errors_are_session_not_found() {
    let tmp = std::env::temp_dir().join(format!("wf-persisted-class-gate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("create tmp memory dir");

    // 预置**仅持久化**会话（owner=default ⇒ 单租户 ALLOW_NO_AUTH 下 uid 亦为 default）。
    let meta = format!(
        "{{\"type\":\"session_meta\",\"session_id\":\"{PERSISTED_ID}\",\"provider_name\":\"openai\",\
         \"model\":\"gpt-4o\",\"goal\":\"g\",\"owner\":\"default\",\
         \"created_at\":\"2026-01-01T00:00:00Z\"}}\n"
    );
    std::fs::write(tmp.join(format!("{PERSISTED_ID}.jsonl")), meta)
        .expect("seed persisted session");

    let _svc = wait_healthy(3948, &tmp);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let base = "http://127.0.0.1:3948";
    let c = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()
        .expect("build http client");

    // 发一次请求，返回 (状态码, body 文本)。
    let req = |method: reqwest::Method, path: String, body: Option<serde_json::Value>| {
        let c = c.clone();
        rt.block_on(async move {
            let mut r = c.request(method, format!("{base}{path}"));
            if let Some(b) = body {
                r = r.json(&b);
            }
            let resp = r.send().await.expect("send");
            let st = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            (st, text)
        })
    };

    // ① 对照：该端点目标是"**记录**"（持久化即算存在）⇒ 200 phase=persisted
    let (s1, b1) = req(
        reqwest::Method::GET,
        format!("/api/v1/sessions/{PERSISTED_ID}"),
        None,
    );
    assert_eq!(
        s1, 200,
        "① 仅持久化会话的状态查询应 200（判据按端点语义）；body={b1}"
    );
    assert!(
        b1.contains("persisted"),
        "① 应报 phase=persisted；body={b1}"
    );

    // ② artifact/open → 404（修复前 400 INVALID_PARAM，文案却写 session not found）
    let (s2, b2) = req(
        reqwest::Method::GET,
        format!("/api/v1/sessions/{PERSISTED_ID}/artifact/open?path=x.txt"),
        None,
    );
    assert_eq!(
        s2, 404,
        "② 仅持久化会话开产物应 404（修复前 400）；body={b2}"
    );
    assert!(
        b2.contains("SESSION_NOT_FOUND"),
        "② 错误码应为 SESSION_NOT_FOUND（修复前 INVALID_PARAM——码与文案自相矛盾）；body={b2}"
    );

    // ③ artifact/open-external → 404
    let (s3, b3) = req(
        reqwest::Method::GET,
        format!("/api/v1/sessions/{PERSISTED_ID}/artifact/open-external?path=x.txt"),
        None,
    );
    assert_eq!(
        s3, 404,
        "③ 仅持久化会话系统打开应 404（修复前 400）；body={b3}"
    );

    // ④ approvals → 404
    let (s4, b4) = req(
        reqwest::Method::POST,
        format!("/api/v1/sessions/{PERSISTED_ID}/approvals"),
        Some(serde_json::json!({"approval_id": "a1", "decision": "approve"})),
    );
    assert_eq!(s4, 404, "④ 仅持久化会话审批应 404（修复前 400）；body={b4}");
    assert!(
        b4.contains("SESSION_NOT_FOUND"),
        "④ 错误码应为 SESSION_NOT_FOUND；body={b4}"
    );

    // ⑤ interaction/:iid → 404
    let (s5, b5) = req(
        reqwest::Method::POST,
        format!("/api/v1/sessions/{PERSISTED_ID}/interaction/i1"),
        Some(serde_json::json!({
            "id": "i1", "by": "human", "resolved": true,
            "payload": {}, "latency_ms": null
        })),
    );
    assert_eq!(
        s5, 404,
        "⑤ 仅持久化会话应答交互应 404（修复前 400）；body={b5}"
    );

    // ⑥ 对照：**本就不漂移**的两个端点（应仍 404）
    let (s6a, _) = req(
        reqwest::Method::POST,
        format!("/api/v1/sessions/{PERSISTED_ID}/cancel"),
        None,
    );
    let (s6b, _) = req(
        reqwest::Method::GET,
        format!("/api/v1/sessions/{PERSISTED_ID}/stream"),
        None,
    );
    assert_eq!(s6a, 404, "⑥ cancel 对仅持久化会话应 404（对照）");
    assert_eq!(s6b, 404, "⑥ stream 对仅持久化会话应 404（对照）");

    // ⑦/⑧ 防误伤：**存活**会话上内层失败仍必须 400（不得一刀切成 404）
    let live_id: String = rt.block_on(async {
        let r = c
            .post(format!("{base}/api/v1/sessions"))
            .json(&serde_json::json!({
                "provider": "openai", "goal": "d179-live", "budget": {"max_steps": 5}
            }))
            .send()
            .await
            .expect("create live session");
        assert_eq!(r.status().as_u16(), 201, "⑦ 建存活会话应 201");
        let v: serde_json::Value = r.json().await.expect("json");
        v["session_id"].as_str().expect("session_id").to_string()
    });
    let (s7, b7) = req(
        reqwest::Method::GET,
        format!("/api/v1/sessions/{live_id}/artifact/open?path=definitely-missing.txt"),
        None,
    );
    assert_eq!(
        s7, 400,
        "⑦ **存活**会话的产物缺失仍是 400（分类不得误伤为 404）；body={b7}"
    );
    let (s8, b8) = req(
        reqwest::Method::POST,
        format!("/api/v1/sessions/{live_id}/approvals"),
        Some(serde_json::json!({"approval_id": "a1", "decision": "approve"})),
    );
    assert_eq!(
        s8, 400,
        "⑧ **存活**会话无 pending 交互仍是 400（分类不得误伤）；body={b8}"
    );

    eprintln!("D-179 PASS: 仅持久化会话的实时端点错误已归位为 404；存活会话仍 400");
    let _ = std::fs::remove_dir_all(&tmp);
}
