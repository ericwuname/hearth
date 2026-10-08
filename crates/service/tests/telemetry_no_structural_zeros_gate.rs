//! D-176（P1-140）：**可观测端点不得对外提供"结构性 0"假指标 / 名不副实标签**。
//!
//! 背景（病灶）：
//!   · `TelemetryCollector` 三字段中 `error_count` / `steps_total` **全仓零 `fetch_add`**
//!     （唯一写入方是 `create_session` 对 `session_count` 的自增）⇒ `GET /api/v1/telemetry`
//!     恒报两个 **0**，却被命名为"实时指标"（`docs/global-panorama-v10.1.md:75` 已记录该现象，
//!     驱动文档此前未跟踪）。
//!   · `GET /api/v1/resources` 的键 `sessions_active` 读的是**累计**创建数（单调不减、从不回退）
//!     —— **名不副实**（"活跃"意味着有回退的当前量）。
//!
//! 修复口径（承 D-107「拿结构性 0 当业务指标比不报更糟」）：① 删除无写入方的
//! `error_count` / `steps_total`（字段 + 响应）；② `sessions_active` 如实改名
//! `sessions_created_total`。
//!
//! 盲区（本套件**不**覆盖）：不校验 `snapshot` 内部字段（由 `resource-monitor` 自测覆盖）；
//! 只锁"无写入方字段不得对外报告"这条不变量。

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
    let tmp = std::env::temp_dir().join(format!("wf-telemetry-gate-{}", std::process::id()));
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
fn telemetry_reports_no_structural_zeros() {
    let _svc = wait_healthy(3945);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let base = "http://127.0.0.1:3945";
    let c = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()
        .expect("build http client");

    let get_json = |path: &str| {
        let path = path.to_string();
        let c = c.clone();
        rt.block_on(async move {
            let r = c
                .get(format!("{base}{path}"))
                .send()
                .await
                .expect("get json");
            assert_eq!(r.status().as_u16(), 200, "{path} 应 200");
            r.json::<serde_json::Value>().await.expect("json body")
        })
    };

    // ① 结构性 0 不得对外报告
    let t = get_json("/api/v1/telemetry");
    assert!(
        t.get("session_count").is_some(),
        "① telemetry 必须报有真实写入方的 session_count；实得 {t}"
    );
    assert!(
        t.get("error_count").is_none(),
        "① `error_count` **全仓零写入方**（恒 0）⇒ 不得当「实时指标」对外报告；实得 {t}"
    );
    assert!(
        t.get("steps_total").is_none(),
        "① `steps_total` **全仓零写入方**（恒 0）⇒ 不得当「实时指标」对外报告；实得 {t}"
    );

    // ② 有写入方的计数必须是真的：建 2 个会话后应为 2
    for i in 0..2 {
        let st = rt.block_on(async {
            c.post(format!("{base}/api/v1/sessions"))
                .json(&serde_json::json!({
                    "provider": "openai",
                    "goal": format!("d176-{i}"),
                    "budget": {"max_steps": 5}
                }))
                .send()
                .await
                .expect("create session")
                .status()
                .as_u16()
        });
        assert_eq!(st, 201, "② 建会话应 201");
    }
    let t2 = get_json("/api/v1/telemetry");
    assert_eq!(
        t2["session_count"].as_u64(),
        Some(2),
        "② `session_count` 必须反映真实创建数（2）；实得 {t2}"
    );

    // ③ `sessions_active` 名不副实（值实为累计、从不回退）⇒ 必须如实改名
    let res = get_json("/api/v1/resources");
    assert!(
        res.get("sessions_active").is_none(),
        "③ `sessions_active` 读的是**累计**计数（单调不减）——名不副实，应改名；实得 {res}"
    );
    assert_eq!(
        res["sessions_created_total"].as_u64(),
        Some(2),
        "③ 应如实给出 `sessions_created_total` = 2；实得 {res}"
    );

    eprintln!("D-176 PASS: telemetry/resources 不再提供结构性 0 假指标，标签与事实一致");
}
