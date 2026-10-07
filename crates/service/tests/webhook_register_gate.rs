//! D-170（P1-135）：`POST /api/v1/webhooks` **注册时**的 url 校验门禁。
//!
//! 病灶（假成功 / 校验只在一侧，承 D-161/D-164 同族）：
//! `WebhookManager::register(&self, cfg)` **只 push、恒成功**；url 校验只在 `fire()` 里做
//! （不通过则 `tracing::warn` 跳过）。于是 `POST /api/v1/webhooks` 对**永远投不出去**的 url
//! 也回 `{"ok": true}` —— 客户端以为注册成功、却永远收不到回调，且**无从知晓**。
//! 最严重的一类 url 是 curl 选项注入形（`-K/path`，可读宿主文件）：它在**注册**这一步就被
//! 放行入库，安全性完全依赖 `fire()` 那条 warn 分支，边界（注册面）本身没有防线。
//!
//! 修复口径：注册时即校验（复用 `validate_webhook_url` 与 `fire` **同一份** allowlist 口径），
//! 不合法 → **400 INVALID_PARAM**；合法 → 仍回 `{"ok": true}`。
//!
//! 盲区（本套件**不**覆盖）：
//!   - 不覆盖"合法 url 但投递失败"（fire 侧超时/非零退出，属运行期，已有 warn 留痕）。
//!   - 不覆盖 allowlist 生效时的域名单（那由 `HEARTH_EGRESS_ALLOWLIST` 决定，本套件不设它）。

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
    let tmp = std::env::temp_dir().join(format!("wf-hook-gate-{}", std::process::id()));
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
fn webhook_registration_validates_url() {
    let _svc = wait_healthy(3940);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let base = "http://127.0.0.1:3940";
    let c = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()
        .expect("build http client");

    let post = |url: &'static str| {
        let c = c.clone();
        async move {
            let r = c
                .post(format!("{base}/api/v1/webhooks"))
                .json(&serde_json::json!({ "url": url, "events": ["session_created"] }))
                .send()
                .await
                .expect("post webhook");
            let st = r.status().as_u16();
            let body = r.text().await.unwrap_or_default();
            (st, body)
        }
    };

    // ① curl 选项注入形（可读宿主文件）——**注册**即须拒
    let (s1, b1) = rt.block_on(post("-K/etc/passwd"));
    assert_eq!(
        s1, 400,
        "① curl 选项注入形 url 必须 400（修复前入库并回 ok:true，安全性只靠 fire 的 warn 兜底）；body={b1}"
    );

    // ② 非 http(s) scheme —— 须拒
    let (s2, b2) = rt.block_on(post("ftp://example.com/hook"));
    assert_eq!(s2, 400, "② 非 http(s) url 必须 400；body={b2}");

    // ③ 合法 http url —— 仍须放行（防修复误伤）
    let (s3, b3) = rt.block_on(post("http://example.com/hook"));
    assert_eq!(s3, 200, "③ 合法 http(s) url 应 200；body={b3}");
    assert!(
        b3.contains("\"ok\":true") || b3.contains("\"ok\": true"),
        "③ 合法注册应回 ok:true；body={b3}"
    );

    eprintln!("D-170 PASS: webhook 注册面已校验 url（非法 400 / 合法放行）");
}
