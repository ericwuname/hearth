//! D-165（P1-131）：**优雅停机**（graceful shutdown）。
//!
//! 病灶：`main.rs` 原先是裸 `axum::serve(listener, app).await`——**没有任何停机路径**；
//! 编排方（k8s/docker）发 SIGTERM 时进程被默认处理**立即终止** ⇒ 在途请求 / SSE 流被
//! **硬切断**（客户端看到连接错误），与"高可用"目标相悖。
//!
//! 分工口径（2026-10-05 联网核实，非拍脑袋）：应用负责「**停止接受新连接 + 排空在途**」；
//! 排空的**墙钟上界交编排方**（`terminationGracePeriodSeconds`，超时才 SIGKILL）。故本实现
//! **不**自设硬杀超时——避免在库内 `process::exit` 带来的副作用（且不可单测）。
//!
//! 抽成 lib 函数的目的：让"停机顺序"可被**确定性**单测（`tests/graceful_shutdown_gate.rs`
//! 用 `oneshot` 手动触发停机，不依赖真实信号与平台）。
//!
//! 盲区：不覆盖"就绪探针翻 draining"（LB 摘除优化）；Windows 无 SIGTERM，仅 Ctrl-C。

use std::future::Future;
use std::net::SocketAddr;

use axum::Router;
use tokio::net::TcpListener;

/// 起服务，并在 `shutdown` future 完成时**停止接受新连接**、**排空在途请求**后返回。
///
/// `ConnectInfo<SocketAddr>` 仍按原样注入（限流中间件的 per-IP 维度依赖它，见 P2-1）。
pub async fn serve_with_shutdown(
    app: Router,
    listener: TcpListener,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown)
    .await
}

/// 停机信号：Ctrl-C（SIGINT）；Unix 上并听 SIGTERM（k8s/docker 的默认终止信号）。
pub async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(e) => {
                // 拿不到 SIGTERM 监听时**不要**让它立即返回（那会变成"秒退"），挂起即可。
                tracing::warn!("cannot install SIGTERM handler ({e}); falling back to Ctrl-C only");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!(
        "shutdown signal received — draining in-flight requests, accepting no new connections"
    );
}
