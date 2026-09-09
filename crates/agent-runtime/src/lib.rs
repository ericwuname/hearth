//! agent-runtime — Hearth 内核运行时（D1: 从 service 抽出）。
//!
//! 职责：会话管理 / agent loop 驱动 / LLM 调度 / 沙箱调用（原 `service::session`）。
//! 零 HTTP 依赖——service 是它的 HTTP 包装层；CLI（hearth）进程内直跑。
//!
//! 红线（docs/hearth-cli-design.md §2.1）：不动 `docs/ai-os-event-contract-v1.md`
//! 事件结构；不动 `crates/sandbox` 内部；不删 service 独立部署能力。

pub mod envelope;
pub mod session;

pub use envelope::EnvelopeState;
/// CLI 直跑复用：agent 事件 → 契约 AgentEvent（原 service::session 私有）。
pub use session::map_event;
