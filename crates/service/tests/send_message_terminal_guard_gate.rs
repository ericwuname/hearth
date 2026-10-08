//! D-178（P1-142）：**终态会话不得再被 `send_message` 改写状态**（承 D-159「终态不可变」+ D-166「错误分类」）。
//!
//! 背景（病灶）：`POST /api/v1/sessions/:id/messages` 走 `SessionManager::send_message`，
//! 而该函数是**单发**设计——首次调用会把 `s.agent` **取走**。对**已落终态**
//! （`finished_at.is_some()`，如已 cancel / 已 done）或**在途**（`running`）的会话再发一次，
//! 会走进 `agent None` 分支并：
//!   ① 把 `phase` **改写为 `"error"`**、`finished_at` 重置 —— 与 D-159 同族（"终态不可变"被破坏：
//!      丢掉真实完成时间、重置 TTL 淘汰时钟）；
//!   ② 把**已成功完成**的会话对外报成 error（假故障）。
//! 且 handler 的 `Err` 一律映射 `404 SESSION_NOT_FOUND`（会话其实存在 —— 与 D-166 同族）。
//! CLI 侧早已**自行**兜底（`lib.rs` 对 `status==done/cancelled` 拒绝 resume、提示"会话已结束"）
//! —— 恰因服务端缺这道守卫。
//!
//! 修复口径：`send_message` 在**任何副作用之前**判 `running || finished_at.is_some()` → 直接拒绝；
//! handler 按"会话是否存活于内存"分类错误：存活 ⇒ **400 `INVALID_PARAM`**；不存活 ⇒ 404。
//!
//! 盲区（本套件**不**覆盖）：
//!   - 不覆盖"首条消息正常跑通"（需真实 LLM）；只锁**终态守卫**与**错误分类**。
//!   - 用 `OPENAI_BASE_URL` 指向已关闭端口，使**若守卫失效**（修复前）走到 agent 时快速失败、
//!     不会拖慢测试。

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
    let tmp = std::env::temp_dir().join(format!("wf-sendmsg-guard-gate-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("create tmp memory dir");
    let child = Command::new(svc)
        .env("PORT", port.to_string())
        .env("MEMORY_DIR", &tmp)
        .env("ALLOW_NO_AUTH", "1")
        .env("OPENAI_API_KEY", "sk-test-placeholder-for-contract-test")
        .env("OPENAI_BASE_URL", "http://127.0.0.1:1")
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
fn send_message_on_terminal_session_is_rejected_without_state_rewrite() {
    let _svc = wait_healthy(3947);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let base = "http://127.0.0.1:3947";
    let c = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()
        .expect("build http client");

    // ① 建会话
    let sid: String = rt.block_on(async {
        let r = c
            .post(format!("{base}/api/v1/sessions"))
            .json(&serde_json::json!({
                "provider": "openai", "goal": "d178", "budget": {"max_steps": 5}
            }))
            .send()
            .await
            .expect("create session");
        assert_eq!(r.status().as_u16(), 201, "① 建会话应 201");
        let v: serde_json::Value = r.json().await.expect("create json");
        v["session_id"].as_str().expect("session_id").to_string()
    });

    // ② 取消 → 落终态（从未启动 ⇒ 由 cancel_session 落 phase="cancelled" + finished_at）
    let st_cancel = rt.block_on(async {
        c.post(format!("{base}/api/v1/sessions/{sid}/cancel"))
            .send()
            .await
            .expect("cancel")
            .status()
            .as_u16()
    });
    assert_eq!(st_cancel, 200, "② cancel 应 200");

    let phase = |c: reqwest::Client, sid: String| {
        rt.block_on(async move {
            let v: serde_json::Value = c
                .get(format!("{base}/api/v1/sessions/{sid}"))
                .send()
                .await
                .expect("get status")
                .json()
                .await
                .expect("status json");
            v["phase"].as_str().unwrap_or("?").to_string()
        })
    };
    let p_before = phase(c.clone(), sid.clone());
    assert_eq!(p_before, "cancelled", "② 取消后 phase 应为 cancelled");

    // ③ 向**终态**会话再发消息 → 必须 400（修复前：200 + SSE，并把 phase 改写为 error）
    let (st_send, body_send) = rt.block_on(async {
        let r = c
            .post(format!("{base}/api/v1/sessions/{sid}/messages"))
            .json(&serde_json::json!({"content": "again"}))
            .send()
            .await
            .expect("send message");
        let st = r.status().as_u16();
        // 仅当不是 200 流时才读体（避免对 SSE 做无谓读取）。
        let body = if st == 200 {
            String::new()
        } else {
            r.text().await.unwrap_or_default()
        };
        (st, body)
    });
    assert_eq!(
        st_send, 400,
        "③ 对**已终态**会话发消息必须 400 `INVALID_PARAM`（修复前 200 + 改写终态）；body={body_send}"
    );
    assert!(
        body_send.contains("error") && body_send.contains("code"),
        "③ 400 应为统一 JSON 错误形状；body={body_send}"
    );

    // ④ **终态不可变**：phase 必须仍为 cancelled（修复前被改写成 running/error/done）
    let p_after = phase(c.clone(), sid.clone());
    assert_eq!(
        p_after, p_before,
        "④ **终态被改写**：发消息后 phase 必须保持 {p_before}（D-159 同族：终态不可变）；实得 {p_after}"
    );

    // ⑤ 分类不得误伤：**不存在**的会话仍应 404（而非 400）
    let st_missing = rt.block_on(async {
        c.post(format!("{base}/api/v1/sessions/no-such-d178/messages"))
            .json(&serde_json::json!({"content": "x"}))
            .send()
            .await
            .expect("send to missing")
            .status()
            .as_u16()
    });
    assert_eq!(st_missing, 404, "⑤ 不存在的会话应 404（分类不得误伤）");

    eprintln!("D-178 PASS: 终态会话拒绝收消息（400）且状态不被改写；不存在的会话仍 404");
}
