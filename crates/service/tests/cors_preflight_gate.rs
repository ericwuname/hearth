//! D-173（P1-138）：**CORS 预检方法白名单**门禁——本 API 暴露的**唯一 PATCH 路由**
//! 必须能被浏览器（允许源）调用。
//!
//! 背景（病灶）：`main.rs` 的 `CorsLayer::allow_methods([GET, POST, OPTIONS])` 缺少
//! **PATCH**，而本 API 暴露 `PATCH /api/v1/workline/nodes/:id`（`update_work_node`）。
//! 浏览器对 PATCH 会先发**预检**（非简单方法）——白名单缺 PATCH 时预检回道
//! `Access-Control-Allow-Methods: GET,POST,OPTIONS` ⇒ 浏览器**拦截**该请求。于是即便
//! 默认允许源 `http://localhost:5173`（本项目自带前端的默认来源）也**调不动**这个端点：
//! "路由存在但浏览器不可达" 的接线缺口（承 D-92「handler 实现了但没挂路由」同族）。
//!
//! 修复口径：`allow_methods` 增 `PATCH`（不引入 `Any`）。既有文档 `docs/configuration.md`
//! 的方法白名单描述同步更新。
//!
//! 盲区（本套件**不**覆盖，勿误当全覆盖）：
//!   - 只锁 PATCH 这一个代表性缺口 + "不得放宽为 Any" 守卫；不逐一枚举所有方法。
//!   - 只验证**预检响应头**，不驱动真实浏览器（浏览器侧拦截行为由规范决定）。

use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// 与 `main.rs` 默认允许源一致（测试**不**设 `CORS_ORIGIN`，走默认）。
const ORIGIN: &str = "http://localhost:5173";

struct ServiceGuard(Child);

impl Drop for ServiceGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_healthy(port: u16) -> ServiceGuard {
    let svc = env!("CARGO_BIN_EXE_service");
    let tmp = std::env::temp_dir().join(format!("wf-cors-gate-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("create tmp memory dir");
    let child = Command::new(svc)
        .env("PORT", port.to_string())
        .env("MEMORY_DIR", &tmp)
        .env("ALLOW_NO_AUTH", "1")
        .env("OPENAI_API_KEY", "sk-test-placeholder-for-contract-test")
        // 注意：**不设** CORS_ORIGIN ⇒ 走默认 http://localhost:5173（与 ORIGIN 一致）。
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

/// 发一次预检请求（`OPTIONS` + `Origin` + `Access-Control-Request-Method`），
/// 返回 `(状态码, Access-Control-Allow-Methods 头)`。
async fn preflight(c: &reqwest::Client, url: &str, method: &str) -> (u16, String) {
    let r = c
        .request(reqwest::Method::OPTIONS, url)
        .header("Origin", ORIGIN)
        .header("Access-Control-Request-Method", method)
        .send()
        .await
        .expect("send preflight");
    let st = r.status().as_u16();
    let allow = r
        .headers()
        .get("access-control-allow-methods")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    (st, allow)
}

#[test]
fn cors_preflight_allows_patch_but_stays_allowlisted() {
    let _svc = wait_healthy(3943);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let base = "http://127.0.0.1:3943";
    let url = format!("{base}/api/v1/workline/nodes/x");
    let c = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()
        .expect("build http client");

    // ① PATCH 预检必须放行（修复前 allow-methods 缺 PATCH ⇒ 浏览器拦截唯一 PATCH 端点）
    let (st, allow) = rt.block_on(preflight(&c, &url, "PATCH"));
    assert_eq!(st, 200, "① PATCH 预检应 200");
    assert!(
        allow.to_uppercase().contains("PATCH"),
        "① CORS 方法白名单必须含 PATCH（修复前 = `GET,POST,OPTIONS` ⇒ 浏览器拦截 \
         `PATCH /api/v1/workline/nodes/:id`）；实得 `{allow}`"
    );

    // ② 既有方法不得被本次改动误删
    for m in ["GET", "POST", "OPTIONS"] {
        assert!(
            allow.to_uppercase().contains(m),
            "② {m} 必须仍被允许（防修复误伤）；实得 `{allow}`"
        );
    }

    // ③ 不得为省事把白名单放宽成任意方法（未列出的 TRACE 不得出现）
    let (_st_t, allow_trace) = rt.block_on(preflight(&c, &url, "TRACE"));
    assert!(
        !allow_trace.to_uppercase().contains("TRACE"),
        "③ 白名单**不得**被放宽为任意方法（TRACE 不该出现）；实得 `{allow_trace}`"
    );

    eprintln!("D-173 PASS: CORS 预检已放行 PATCH 且仍为显式白名单（非 Any）");
}
