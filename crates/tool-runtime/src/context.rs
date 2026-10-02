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
    //
    // D-98（2026-10-02, traecode）：**删除 `deadline_clamped` 字段**——它的唯一写入点
    // 是 `dispatcher.rs`（`ctx.deadline_clamped = …`），而**全仓无任何读取方**：
    // dispatcher 判定"超时归属是 deadline 还是工具自身"用的是**同名的局部变量**
    // （`Err(_) if deadline_clamped => TaskDeadlineExceeded`），工具侧只消费
    // `effective_timeout`。即它是"写给人看、机器不用"的字段（同 D-88 的
    // `Turn.actions` 口径）。真需要让**工具**区分超时归属时，应把结论**随错误类型**
    // 传（现已是 `TaskDeadlineExceeded` 类型化错误），而不是另加一个只写字段。
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            cwd: PathBuf::from("."),
            env: HashMap::new(),
            scratch: serde_json::Value::Object(Default::default()),
            task_deadline: None,
            effective_timeout: None,
        }
    }
}

// D-98：`remaining_task_time()` 已删除——全仓零调用方；唯一需要该算法的
// dispatcher 是**内联**写的（它还要同时区分"已过期→直接拒绝"与"被收紧→置
// clamped"，形态不同），且该处注释已声明是"effective timeout **单点**计算"。
// 留一个平行的 unused pub 方法只会与那处单点实现漂移（同 D-57 删 `coerce_args`
// 的理由：并行且从未接线的旧机制）。
