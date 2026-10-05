//! D-157（P1-123）：限流 **429** 的统一错误形状门禁。
//!
//! 病灶（半接线 / B2 未覆盖中间件层）：契约 `docs/handoff-trunk-freeze.md` 声明
//! 「**所有** API 错误返回 `{"error":{"code","message"}}`」、`docs/governance.md` B2
//! 「**全部 handler** 返回 `Json<ErrorResponse>`」。但**限流中间件**
//! `routes::concurrency_limit` 超限时用 `rate_limit_response()` 直接构造
//! `Response::new(Body::from("too many requests (…)"))` ⇒ 429 体是**纯文本**、且无
//! `Content-Type: application/json`，绕过统一形状（客户端按 JSON 解析 429 错误体必失败）。
//! 同族：D-156（`Json` 提取器级拒绝）。
//!
//! 为什么用**进程内**（而非起真 service）测试：per-IP 在途上限=50、全局=500，要稳定
//! 触发 429 必须让 >50 个请求**同时**在途——用真实 HTTP 客户端难以确定性复现（快 handler
//! 的在途窗口极短）。此处以 `tower::ServiceExt::oneshot` + 一个**阻塞 handler** 精确控制：
//! 先占满 50 个名额（handler 全部阻塞），第 51 个请求必然被限流。
//!
//! 盲区（本套件**不**覆盖，勿误当全覆盖）：
//!   - 只锁 429 的**响应形状**与 `Retry-After`；不覆盖限流计数/归还语义（见 `routes::p0_05_tests`）。
//!   - 仅覆盖 per-IP 分支；全局分支走同一 `rate_limit_response()`，形状一致（未单列）。
//!   - `P1_MAX_PER_IP` 为私有常量（=50），本套件按 50 硬编码——该常量若调整，本门禁须同步。

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::Router;
use tokio::sync::Semaphore;
use tower::ServiceExt;

/// 与 `service::routes::P1_MAX_PER_IP` 保持一致（见文件头盲区）。
const MAX_PER_IP: usize = 50;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rate_limit_429_returns_unified_json_error() {
    // block：被放行的 handler 阻塞于此；entered：每进入一个 handler 放 1 许可，供主线程等"占满"。
    let block = Arc::new(Semaphore::new(0));
    let entered = Arc::new(Semaphore::new(0));

    let app = Router::new()
        .route(
            "/slow",
            get({
                let block = block.clone();
                let entered = entered.clone();
                move || {
                    let block = block.clone();
                    let entered = entered.clone();
                    async move {
                        entered.add_permits(1);
                        let _p = block.acquire().await;
                        "ok"
                    }
                }
            }),
        )
        .layer(axum::middleware::from_fn(
            service::routes::concurrency_limit,
        ));

    // 占满 per-IP 名额：50 个并发请求全部卡在 handler。
    let mut held = Vec::new();
    for _ in 0..MAX_PER_IP {
        let app = app.clone();
        held.push(tokio::spawn(async move {
            let mut req = Request::builder()
                .uri("/slow")
                .body(Body::empty())
                .expect("build req");
            req.extensions_mut()
                .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 40000))));
            app.oneshot(req).await
        }));
    }
    // 等待 50 个请求都已被放行（确定性：不靠 sleep 竞态）。
    let all = tokio::time::timeout(
        Duration::from_secs(10),
        entered.acquire_many(MAX_PER_IP as u32),
    )
    .await
    .expect("占满 50 个名额超时——限流常量或调度异常")
    .expect("entered semaphore closed");
    drop(all);

    // 第 51 个请求 → 必然 429。
    let mut req = Request::builder()
        .uri("/slow")
        .body(Body::empty())
        .expect("build req");
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 40000))));
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("51st request should be answered");

    assert_eq!(
        resp.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "第 51 个在途请求应被限流为 429"
    );
    assert_eq!(
        resp.headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok()),
        Some("5"),
        "429 必须带 Retry-After 头"
    );
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024)
        .await
        .expect("read 429 body");
    let body = String::from_utf8_lossy(&bytes).to_string();

    assert!(
        ct.contains("application/json"),
        "429 的 Content-Type 应为 application/json，实际 {ct:?}；body={body:?}"
    );
    let v: serde_json::Value = serde_json::from_str(&body)
        .unwrap_or_else(|e| panic!("429 体应为 JSON，解析失败: {e}；body={body:?}"));
    assert!(
        v["error"]["code"].is_string(),
        "429 缺 error.code：{body:?}"
    );
    assert!(
        v["error"]["message"].is_string(),
        "429 缺 error.message：{body:?}"
    );

    // 释放阻塞的 handler，避免任务悬挂。
    block.add_permits(MAX_PER_IP);
    for h in held {
        let _ = h.await;
    }
}
