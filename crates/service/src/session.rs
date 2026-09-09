//! D1: 会话管理已抽至 agent-runtime——本模块为薄 re-export（HTTP 包装层保持
//! routes.rs / integration_test.rs 的 service::session::* 路径不变）。

pub use agent_runtime::session::*;
