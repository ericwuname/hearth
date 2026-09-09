use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Execution context provided to each tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContext {
    /// Current working directory for tool execution.
    pub cwd: PathBuf,
    /// Environment variables available to tools.
    pub env: HashMap<String, String>,
    /// Scratch space shared across tool invocations in the same run.
    pub scratch: serde_json::Value,

    /// P1-LTR-01: 任务级**绝对截止时刻**（共享时刻戳，非剩余秒数——免疫逐层
    /// 传参漂移）。来源 = H2 run_started_at + budget.max_time_secs（同源派生，
    /// 不建第二套 deadline 类型）；每轮 continue_turn 重置；**不持久化**（6.3）。
    /// None = 无任务时间上限——所有路径行为与 v0.2.10 完全一致（T2 回归保证）。
    #[serde(default, skip)]
    pub task_deadline: Option<std::time::Instant>,
    /// P1-LTR-01: dispatcher **单点**计算的 effective timeout
    /// = min(declared_timeout, remaining_task_time)。由 dispatch() 写入本副本后
    /// 传给工具；工具（bash→sandbox.spawn）优先消费本值。None = 未计算（旧行为）。
    #[serde(default, skip)]
    pub effective_timeout: Option<std::time::Duration>,
    /// P1-LTR-01: effective 被任务 deadline 收紧的结构化标记（修 2——
    /// 禁 stderr 字符串判定；true = effective < declared，超时归 deadline_exceeded）。
    #[serde(default, skip)]
    pub deadline_clamped: bool,
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            cwd: PathBuf::from("."),
            env: HashMap::new(),
            scratch: serde_json::Value::Object(Default::default()),
            task_deadline: None,
            effective_timeout: None,
            deadline_clamped: false,
        }
    }
}

impl ToolContext {
    /// P1-LTR-01: 剩余任务时间（deadline 已过 → 零值；无 deadline → None）。
    pub fn remaining_task_time(&self) -> Option<std::time::Duration> {
        self.task_deadline
            .map(|dl| dl.saturating_duration_since(std::time::Instant::now()))
    }
}
