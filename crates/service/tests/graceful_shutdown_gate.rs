//! D-165（P1-131）：**优雅停机**门禁。
//!
//! 病灶（能力缺失，**先设计后接线**）：`main.rs` 原先是裸 `axum::serve(listener, app).await`
//! ——**没有任何停机路径**。编排方（k8s/docker）发 SIGTERM 时进程被默认处理**立即终止**
//! ⇒ 在途请求 / SSE 流被**硬切断**（客户端连接错误），与"高可用"目标相悖。
//! 业界口径（2026-10-05 联网核实）：应用负责「**停止接受新连接 + 排空在途**」，
//! 排空的**墙钟上界交编排方**（`terminationGracePeriodSeconds`，超时方 SIGKILL）；
//! 故本实现**不**自设硬杀超时（避免进程自决退出）。
//!
//! 本套件三部分：
//!   ① 源码级：`main.rs` 必须走 `serve::serve_with_shutdown(..)`，**不得**再裸调 `axum::serve(`；
//!      `serve.rs` 必须真的用 `with_graceful_shutdown`。
//!   ② 运行时：进程内起 `serve_with_shutdown`（自由端口）+ 一个**阻塞 handler**，
//!      手动触发停机后断言——**新连接被拒**，而**在途请求仍完成 200**（排空语义）。
//!   ③ 信号函数存在性：`serve::shutdown_signal` 必须可被引用（Ctrl-C / Unix SIGTERM）。
//!
//! 盲区（本套件**不**覆盖）：
//!   - 不覆盖"就绪探针翻 draining"（LB 摘除优化，见驱动文档 D-165 后续选项）。
//!   - Windows 无 SIGTERM，仅 Ctrl-C；真信号路径（非手动 `oneshot`）不在此单测内。

use std::path::PathBuf;

/// 读源码并**逐行剥 `//` 注释**——避免"注释里点名旧写法 ⇒ 自命中"（D-147/151/153 反复踩到）。
fn read_stripped(rel: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    let raw = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("读不到 {}：{e}", p.display()));
    raw.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn composition_root_serves_with_graceful_shutdown() {
    let main_src = read_stripped("src/main.rs");
    assert!(
        main_src.contains("serve_with_shutdown("),
        "`main.rs` 必须通过 `serve::serve_with_shutdown(..)` 起服务——否则 SIGTERM/SIGINT \
         会**立即终止**进程、硬切断在途请求与 SSE 流（D-165 病灶）。"
    );
    assert!(
        !main_src.contains("axum::serve("),
        "`main.rs` 不得再裸调 `axum::serve(`（无停机路径）；请改用 `serve::serve_with_shutdown`。"
    );

    let serve_src = read_stripped("src/serve.rs");
    assert!(
        serve_src.contains("with_graceful_shutdown"),
        "`serve.rs` 必须真的调用 `with_graceful_shutdown`（停止接受新连接 + 排空在途）。"
    );
    assert!(
        serve_src.contains("fn shutdown_signal"),
        "`serve.rs` 必须提供 `shutdown_signal`（Ctrl-C + Unix SIGTERM）。"
    );
}

/// 运行时：停机触发后，**在途请求仍被排空**（完成 200），而**新连接被拒**。
///
/// 确定性做法：进程内起 `serve_with_shutdown`（自由端口）+ 一个**阻塞 handler**，
/// 用 `oneshot` 手动触发停机（不依赖真实信号与平台）。
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn inflight_drains_and_new_connections_are_refused() {
    use axum::routing::get;
    use axum::Router;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Semaphore;

    let block = Arc::new(Semaphore::new(0));
    let entered = Arc::new(Semaphore::new(0));
    let app = Router::new().route(
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
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral");
    let addr = listener.local_addr().expect("addr");
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        let _ = service::serve::serve_with_shutdown(app, listener, async move {
            let _ = rx.await;
        })
        .await;
    });

    // ① 发一个**在途**请求（handler 阻塞住）
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()
        .expect("client");
    let inflight = tokio::spawn({
        let client = client.clone();
        async move { client.get(format!("http://{addr}/slow")).send().await }
    });
    let _entered_permit = tokio::time::timeout(Duration::from_secs(5), entered.acquire())
        .await
        .expect("在途请求未到达 handler（超时）")
        .expect("semaphore closed");

    // ② 触发停机
    let _ = tx.send(());

    // ③ 停机后新连接应被拒（accept 循环退出有微小竞态 ⇒ 给 5s 窗口，观察到一次失败即可）
    let mut refused = false;
    for _ in 0..50 {
        if tokio::net::TcpStream::connect(addr).await.is_err() {
            refused = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        refused,
        "停机后不应再接受新连接（5s 内未观察到一次连接被拒）"
    );

    // ④ 放开阻塞 ⇒ 在途请求必须**仍能完成 200**（这正是"排空"而非"硬切断"）
    block.add_permits(1);
    let resp = tokio::time::timeout(Duration::from_secs(5), inflight)
        .await
        .expect("在途请求未被排空（超时）")
        .expect("join")
        .expect("在途请求被切断了（应为 200）");
    assert_eq!(resp.status().as_u16(), 200, "在途请求应 200");

    let _ = tokio::time::timeout(Duration::from_secs(5), server).await;
}
