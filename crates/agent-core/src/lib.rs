// D-85（2026-10-01, traecode）：原 `orchestrator`（479 行）与 `replay`（51 行）两个
// 模块**已整段删除**——全仓**零生产消费者**：
// - `orchestrator`（`execute_plan` / `TaskOrchestrator` / `PipelineRunner` /
//   `TaskStep` / `TaskReport` / `TaskValidator`）：它是**已被拆除的 TaskGraph 的执行器**
//   （loop.rs 原位注释即载明"v10.4 orchestrator 分支已删——图本体消失，恒走顺序执行臂"），
//   故与 D-66（planner）/ D-83（子代理委派）同口径：生产者结构性消失 ⇒ 无从"接线"，删除。
//   连带删除 `Scheduler::dispatch_single`（其唯一用途就是给 orchestrator 当 runner closure）。
// - `replay`（`replay_session` / `ReplaySummary`）：仅有一行 `pub use` 再导出，无任何调用方。
// 若日后重启"预规划编排/会话回放"能力，随各自能力一并恢复。

pub mod constitution;
pub mod context;
pub mod r#loop;
pub mod scheduler;
pub mod talent;
pub mod terminal;

pub use constitution::constitution_prompt;
pub use context::ContextManager;
pub use r#loop::{
    Agent, AgentLoop, ApprovalPolicy, CivWriter, EgressPersistFn, Event, Goal, RunReport, StepNext,
    StepOutcome,
};
pub use scheduler::Scheduler;
pub use terminal::{is_terminal_state, normalize_terminal_state, TERMINAL_STATES};
