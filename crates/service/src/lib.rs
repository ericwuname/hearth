/// P0-05（2026-10-01, traecode）：锁中毒恢复（见模块注释）。
pub mod lock;
/// D-90（2026-10-02, traecode）：OpenAPI 文档的路径镜像表 + 构建（见模块注释）。
pub mod openapi;
pub mod per_user;
pub mod routes;
pub mod serve;
pub mod session;
pub mod sse;
/// v8.0: Agent templates.
pub mod templates;
/// v8.0: Multi-user identity mapping.
pub mod user;
/// v8.0: Webhook notifications.
pub mod webhook;
