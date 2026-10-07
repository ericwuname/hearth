//! D-172（P1-137）：**webhook 投递的多租户隔离**门禁。
//!
//! 背景（病灶）：`WebhookManager` 是**进程级裸 `Vec`**（无 owner 维度，D-111 注释自承
//! "多用户隔离属未实现的能力"），而 `create_session` 在**任何**租户建会话时都
//! `fire_event("session_created", {"goal": …})` 并**无条件广播给全部 hook**。
//! ⇒ D-169 打开 `HEARTH_USERS` 多租户后：**alice 注册一个 hook，即可收割 bob 的会话
//! 目标文本**（跨租户数据泄露，与 D-171 同族）。
//!
//! 修复口径：`WebhookConfig` 增 `owner`（由 `register_webhook` 按调用者 uid **服务端强制
//! 写入**，请求体不可伪造）；`fire_event(event, payload, owner)` 只投给**同一 owner** 的 hook。
//! 单租户（owner 恒 `"default"`）行为不变。
//!
//! 本门禁**起真实 service** 并在**本机抓包口**（临时端口 HTTP 监听）观察真实投递：
//!   ① 正控：alice 的 hook 必须能收到**她自己**建会话的 `session_created`（证明投递链路 + owner
//!      命中都正常——否则下面的否定断言会因"根本没投递"而**假绿**）；
//!   ③ 负控：bob 建会话时 alice 的 hook **不得**收到 bob 的 goal（修复前会收到）。
//!
//! 盲区（本套件**不**覆盖，勿误当全覆盖）：
//!   - 不覆盖"未设 `HEARTH_USERS` 时单租户广播行为不变"的回归（由 `webhook.rs` 单测
//!     `test_d172_select_hooks_is_owner_scoped` 的 `"default"` 等价性 + 既有测试覆盖）。
//!   - 依赖本机 `curl`（`fire` 用 curl 子进程投递）；curl 缺失时 ① 正控会失败并**显式报错**，
//!     不产生假绿。

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
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

/// 起一个临时端口的 HTTP 抓包口：把收到的请求**原文**塞进共享 `sink`，再回 200。
fn start_capture() -> (u16, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind capture listener");
    let port = listener.local_addr().expect("capture addr").port();
    let sink = Arc::new(Mutex::new(Vec::new()));
    let sink2 = sink.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut s) = stream else { continue };
            // 读超时 300ms：curl 一次性把请求写完，首读即可拿到；随后阻塞至超时即认为到齐。
            let _ = s.set_read_timeout(Some(Duration::from_millis(300)));
            let mut acc: Vec<u8> = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                match s.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => acc.extend_from_slice(&buf[..n]),
                    Err(_) => break,
                }
            }
            sink2
                .lock()
                .unwrap()
                .push(String::from_utf8_lossy(&acc).to_string());
            let _ =
                s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
            let _ = s.flush();
        }
    });
    (port, sink)
}

fn wait_for(sink: &Arc<Mutex<Vec<String>>>, needle: &str, timeout: Duration) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if sink.lock().unwrap().iter().any(|r| r.contains(needle)) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

fn wait_healthy(port: u16) -> ServiceGuard {
    let svc = env!("CARGO_BIN_EXE_service");
    let tmp = std::env::temp_dir().join(format!("wf-wh-iso-gate-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("create tmp memory dir");
    let child = Command::new(svc)
        .env("PORT", port.to_string())
        .env("MEMORY_DIR", &tmp)
        .env("ALLOW_NO_AUTH", "1")
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
fn webhook_fire_is_tenant_scoped() {
    let (cap_port, sink) = start_capture();
    let _svc = wait_healthy(3942);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let base = "http://127.0.0.1:3942";
    let c = client();

    // ① alice 注册 webhook，指向本机抓包口
    let s_reg = rt.block_on(async {
        c.post(format!("{base}/api/v1/webhooks"))
            .header("Authorization", format!("Bearer {KEY_ALICE}"))
            .json(&serde_json::json!({
                "url": format!("http://127.0.0.1:{cap_port}/hook"),
                "events": ["session_created"]
            }))
            .send()
            .await
            .expect("alice register webhook")
            .status()
            .as_u16()
    });
    assert_eq!(s_reg, 200, "① alice 注册 webhook 应 200");

    // ② 正控：alice 建会话 → 抓包口应收到**她自己**的 goal（证明投递链路 + owner 命中均正常）
    let a_marker = "alice-marker-d172";
    let s_a = rt.block_on(async {
        c.post(format!("{base}/api/v1/sessions"))
            .header("Authorization", format!("Bearer {KEY_ALICE}"))
            .json(&serde_json::json!({
                "provider": "openai", "goal": a_marker, "budget": {"max_steps": 5}
            }))
            .send()
            .await
            .expect("alice create session")
            .status()
            .as_u16()
    });
    assert_eq!(s_a, 201, "② alice 建会话应 201");
    assert!(
        wait_for(&sink, a_marker, Duration::from_secs(8)),
        "② 正控失败：alice 的 hook 未收到自己的 session_created ⇒ 投递链路未通（curl 缺失？），\
         此时③的否定断言无意义"
    );

    // 清空抓包，确保③看到的只可能是 bob 触发的新请求
    sink.lock().unwrap().clear();

    // ③ 负控：bob 建会话 → alice 的 hook **不得**收到 bob 的 goal
    let b_marker = "bob-marker-d172";
    let s_b = rt.block_on(async {
        c.post(format!("{base}/api/v1/sessions"))
            .header("Authorization", format!("Bearer {KEY_BOB}"))
            .json(&serde_json::json!({
                "provider": "openai", "goal": b_marker, "budget": {"max_steps": 5}
            }))
            .send()
            .await
            .expect("bob create session")
            .status()
            .as_u16()
    });
    assert_eq!(
        s_b, 201,
        "③ bob 建会话应 201（否则事件根本没触发，否定断言会假绿）"
    );

    std::thread::sleep(Duration::from_secs(3));
    let got = sink.lock().unwrap().clone();
    assert!(
        !got.iter().any(|r| r.contains(b_marker)),
        "③ **跨租户泄露**：bob 建会话不得投给 alice 的 hook（修复前对全部 hook 广播）；实得 {got:?}"
    );
    assert!(
        got.is_empty(),
        "③ bob 建会话时 alice 的 hook 不应被触发（注册表里 bob 无 hook）；实得 {got:?}"
    );

    eprintln!("D-172 PASS: webhook 投递已按 owner 隔离（跨租户不广播）");
}
