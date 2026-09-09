#![allow(
    clippy::empty_line_after_doc_comments,
    clippy::doc_lazy_continuation,
    clippy::doc_markdown
)]

pub mod constitution;
pub mod context;
pub mod r#loop;
pub mod orchestrator;
pub mod replay;
pub mod scheduler;
pub mod talent;
pub mod terminal;

pub use constitution::constitution_prompt;
pub use context::ContextManager;
pub use orchestrator::{
    execute_plan, PipelineRunner, TaskOrchestrator, TaskReport, TaskStep, TaskValidator,
};
pub use r#loop::{
    Agent, AgentLoop, ApprovalPolicy, CivWriter, EgressPersistFn, Event, Goal, LoopPhase,
    RunReport, StepOutcome,
};
pub use replay::{replay_session, ReplaySummary};
pub use scheduler::Scheduler;
pub use terminal::{is_terminal_state, normalize_terminal_state, TERMINAL_STATES};
