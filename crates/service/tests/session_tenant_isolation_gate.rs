//! D-171（P1-136）：**多租户会话归属隔离**门禁。
//!
//! 背景（病灶）：D-169（P1-134）打开了 `HEARTH_USERS` 多租户（鉴权按 key 映射到 uid，
//! `create_session` 也把 uid 记为 `owner`），但**会话读写面完全没有归属条件**——
//! `list_sessions` 直接 `list_sessions_json()`、`get_session_status`/`send_message`/
//! `session_events_export`/`session_stream`/`open_artifact`/`open_external`/
//! `submit_approval`/`submit_interaction`/`cancel_session`/`get_session_history` 全按 id
//! 直取。⇒ 任一已认证租户只要拿到他人会话 id，即可**读其状态/事件/历史/工作区产物**
//! （含目标文本）、**向其注入消息/代其审批/停其会话**——跨租户数据泄露 + 完整性破坏
//! （BOLA/IDOR）。
//!
//! 修复口径：所有以会话 id 寻址的 handler 先过 `ensure_session_owner`——调用者 uid 必须
//! 等于会话 `owner`，否则 **404**（**不是 403**：403 会泄露"该 id 有效但归别人"，助长枚举；
//! 见 OWASP Multi-Tenant Security Cheat Sheet 与 BOLA 防护共识）。会话**不存在**与
//! **存在但非本人**走**同一 404 形状**，不可区分。归属元数据随会话持久化（`SessionRecord.owner`），
//! 重启后仍可判归属。
//!
//! 盲区（本套件**不**覆盖，勿误当全覆盖）：
//!   - 不覆盖"未设 `HEARTH_USERS` 时单租户行为不变"的回归（由既有集成测试覆盖；本档只锁
//!     多租户下跨租户不可见这一侧）。
//!   - 不覆盖重启后**持久化**会话的归属（`owner` 落盘 + 读回）——由 `agent-runtime` 的
//!     单测与 `memory` crate 的单测覆盖。
//!   - 不覆盖 SSE 长连接侧的越权（`session_stream`/`send_message` 在收到**首个事件前**即
//!     由同一 `ensure_session_owner` 拦截；此处只断言状态码 404，不解析流内容）。

use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const KEY_ALICE: &str = "k-alice";
const KEY_BOB: &str = "k-bob";

struct ServiceGuard(Child);

impl Drop for ServiceGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_healthy(port: u16) -> ServiceGuard {
    let svc = env!("CARGO_BIN_EXE_service");
    let tmp = std::env::temp_dir().join(format!("wf-sess-iso-gate-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("create tmp memory dir");
    let child = Command::new(svc)
        .env("PORT", port.to_string())
        .env("MEMORY_DIR", &tmp)
        .env("ALLOW_NO_AUTH", "1")
        // D-171：两个 key → 两个独立 uid（多租户面）。
        .env("HEARTH_USERS", format!("{KEY_ALICE}=alice,{KEY_BOB}=bob"))
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

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()
        .expect("build http client")
}

#[test]
fn session_is_isolated_across_tenants() {
    let _svc = wait_healthy(3941);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let base = "http://127.0.0.1:3941";
    let c = client();

    // ① alice 建会话 → 201，取回 session_id
    let sid: String = rt.block_on(async {
        let r = c
            .post(format!("{base}/api/v1/sessions"))
            .header("Authorization", format!("Bearer {KEY_ALICE}"))
            .json(&serde_json::json!({
                "provider": "openai",
                "goal": "alice-secret-goal-d171",
                "budget": {"max_steps": 5}
            }))
            .send()
            .await
            .expect("alice create session");
        assert_eq!(r.status().as_u16(), 201, "① alice 建会话必须 201");
        let v: serde_json::Value = r.json().await.expect("create session json");
        v["session_id"]
            .as_str()
            .expect("session_id present")
            .to_string()
    });

    // ② alice 自己能读自己的会话状态 → 200（防修复误伤：归属者必须放行）
    let s_alice_own = rt.block_on(async {
        c.get(format!("{base}/api/v1/sessions/{sid}"))
            .header("Authorization", format!("Bearer {KEY_ALICE}"))
            .send()
            .await
            .expect("alice get own")
            .status()
            .as_u16()
    });
    assert_eq!(s_alice_own, 200, "② alice 读自己的会话必须 200（不得误伤）");

    // ③ alice 列表里能看到自己的会话
    let alice_list: String = rt.block_on(async {
        c.get(format!("{base}/api/v1/sessions"))
            .header("Authorization", format!("Bearer {KEY_ALICE}"))
            .send()
            .await
            .expect("alice list")
            .text()
            .await
            .unwrap_or_default()
    });
    assert!(
        alice_list.contains(&sid),
        "③ alice 列表应含自己的会话 {sid}；实得 {alice_list}"
    );

    // ④ bob 列表**不得**含 alice 的会话 id（修复前：全局可见 = RED）
    let bob_list: String = rt.block_on(async {
        c.get(format!("{base}/api/v1/sessions"))
            .header("Authorization", format!("Bearer {KEY_BOB}"))
            .send()
            .await
            .expect("bob list")
            .text()
            .await
            .unwrap_or_default()
    });
    assert!(
        !bob_list.contains(&sid),
        "④ **跨租户泄露**：bob 列表不应含 alice 的会话 {sid}（修复前全局可见）；实得 {bob_list}"
    );

    // ⑤ bob 直接按 id 读 alice 的会话状态 → 404（修复前 200 = RED）
    let s_bob_status = rt.block_on(async {
        c.get(format!("{base}/api/v1/sessions/{sid}"))
            .header("Authorization", format!("Bearer {KEY_BOB}"))
            .send()
            .await
            .expect("bob get status")
            .status()
            .as_u16()
    });
    assert_eq!(
        s_bob_status, 404,
        "⑤ **BOLA**：bob 读 alice 会话状态必须 404（非 403——不泄露 id 有效性；修复前 200）"
    );

    // ⑥ bob 读 alice 的消息历史 → 404（泄露目标文本的主路径）
    let s_bob_hist = rt.block_on(async {
        c.get(format!("{base}/api/v1/sessions/{sid}/messages"))
            .header("Authorization", format!("Bearer {KEY_BOB}"))
            .send()
            .await
            .expect("bob get history")
            .status()
            .as_u16()
    });
    assert_eq!(
        s_bob_hist, 404,
        "⑥ bob 读 alice 会话历史必须 404（修复前 200 ⇒ 跨租户目标文本泄露）"
    );

    // ⑦ bob 导出 alice 的事件 → 404
    let s_bob_events = rt.block_on(async {
        c.get(format!("{base}/api/v1/sessions/{sid}/events"))
            .header("Authorization", format!("Bearer {KEY_BOB}"))
            .send()
            .await
            .expect("bob get events")
            .status()
            .as_u16()
    });
    assert_eq!(
        s_bob_events, 404,
        "⑦ bob 导出 alice 会话事件必须 404（修复前 200 ⇒ 事件流泄露）"
    );

    // ⑧ bob 取消 alice 的会话 → 404（完整性：不得停他人会话）
    let s_bob_cancel = rt.block_on(async {
        c.post(format!("{base}/api/v1/sessions/{sid}/cancel"))
            .header("Authorization", format!("Bearer {KEY_BOB}"))
            .send()
            .await
            .expect("bob cancel")
            .status()
            .as_u16()
    });
    assert_eq!(
        s_bob_cancel, 404,
        "⑧ bob 取消 alice 会话必须 404（修复前 200 ⇒ 可 DoS 他人会话）"
    );

    // ⑨ 不存在的会话 id 与"存在但非本人"同形状（404）——不泄露存在性
    let s_missing = rt.block_on(async {
        c.get(format!("{base}/api/v1/sessions/no-such-session-d171"))
            .header("Authorization", format!("Bearer {KEY_BOB}"))
            .send()
            .await
            .expect("missing")
            .status()
            .as_u16()
    });
    assert_eq!(
        s_missing, 404,
        "⑨ 不存在的会话也必须 404（与'非本人'同形状，不可区分）"
    );

    eprintln!("D-171 PASS: 多租户会话读写面已按 owner 隔离（跨租户一律 404）");
}
