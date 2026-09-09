use agent_types::{ToolCall, ToolResult};
use std::sync::Arc;

use tool_runtime::{
    BashExitError, InteractionState, TaskDeadlineExceeded, ToolContext, ToolDispatcher,
};

/// P2-LR Node 02（RC46 接线，批-4）：五态区分——exit 0（None, is_error=false）/
/// exit >0 / signal / tool timeout / task deadline。类型化 downcast，禁文本解析。
fn classify_dispatch_error(e: &anyhow::Error) -> Option<agent_types::ToolErrorKind> {
    if e.downcast_ref::<TaskDeadlineExceeded>().is_some() {
        Some(agent_types::ToolErrorKind::DeadlineExceeded)
    } else if let Some(be) = e.downcast_ref::<BashExitError>() {
        if be.timed_out {
            Some(agent_types::ToolErrorKind::ToolTimeout)
        } else if be.exit_code < 0 {
            Some(agent_types::ToolErrorKind::ExitSignal(be.exit_code))
        } else {
            Some(agent_types::ToolErrorKind::ExitNonZero(be.exit_code))
        }
    } else {
        None
    }
}

/// Scheduler orchestrates tool calls: dispatch, collect results, handle retries.
/// Also handles approval gating for tools that require user confirmation.
pub struct Scheduler {
    dispatcher: Arc<ToolDispatcher>,
    ctx: ToolContext,
}

impl Scheduler {
    pub fn new(dispatcher: Arc<ToolDispatcher>, ctx: ToolContext) -> Self {
        Self { dispatcher, ctx }
    }

    /// Execute a batch of tool calls and return their results.
    /// Uses parallel dispatch when multiple calls are present.
    pub async fn execute_tool_calls(&self, calls: &[ToolCall]) -> Vec<ToolResult> {
        if calls.is_empty() {
            return vec![];
        }
        tracing::info!(count = calls.len(), tools = ?calls.iter().map(|c| &c.name).collect::<Vec<_>>(), "execute_tool_calls");
        if calls.len() == 1 {
            let call = &calls[0];
            // W4/RC20: is_error 由 Result 分支**结构化**决定——废除
            // `output.starts_with("error:")` 字符串猜测（错误消息不以 error:
            // 开头即漏判；正常输出以 error: 开头即误判）。
            let (output, is_error, error_kind) = match self
                .dispatcher
                .dispatch(&call.name, call.args.clone(), &self.ctx)
                .await
            {
                Ok(out) => {
                    tracing::info!(tool = %call.name, output_len = out.len(), "tool dispatch ok");
                    (out, false, None)
                }
                Err(e) => {
                    // P1-FAILURE-ADAPTATION-01 Node 06: 结构化错误类别投影
                    //（downcast 类型化判定，禁字符串解析——RC20 纪律）
                    let kind = classify_dispatch_error(&e);
                    tracing::error!(tool = %call.name, error = %e, "tool dispatch failed");
                    (format!("{e}"), true, kind)
                }
            };

            vec![ToolResult {
                call_id: call.call_id.clone(),
                is_error,
                output,
                artifacts: vec![],
                error_kind,
            }]
        } else {
            // Multiple calls: parallel dispatch
            let dispatch_calls: Vec<(String, serde_json::Value)> = calls
                .iter()
                .map(|c| (c.name.clone(), c.args.clone()))
                .collect();

            let outputs = self
                .dispatcher
                .dispatch_parallel(&dispatch_calls, &self.ctx)
                .await;

            calls
                .iter()
                .zip(outputs)
                .map(|(call, result)| {
                    let (output, is_error, error_kind) = match result {
                        Ok(out) => (out, false, None),
                        Err(e) => {
                            let kind = classify_dispatch_error(&e);
                            (format!("error: {e}"), true, kind)
                        }
                    };
                    ToolResult {
                        call_id: call.call_id.clone(),
                        is_error,
                        output,
                        artifacts: vec![],
                        error_kind,
                    }
                })
                .collect()
        }
    }

    /// WP-0: 等待交互解决（原 check_approval 泛化）。
    /// 只认状态转移（Resolved=放行 / Rejected=中止）——不解析 kind/payload。
    /// Returns true if resolved (放行), false if rejected (中止). Scoped to `session_id` (M2).
    pub async fn check_interaction(&self, session_id: &str) -> bool {
        let state = self.dispatcher.interaction_state(session_id).await;
        match state {
            InteractionState::NoInteraction | InteractionState::Resolved { .. } => true,
            InteractionState::Rejected { .. } => false,
            InteractionState::Pending { .. } => {
                // Poll until resolved (with timeout to avoid hanging forever)
                let mut waited = 0u64;
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    waited += 100;
                    if waited > 60_000 {
                        // 60 second timeout
                        return false;
                    }
                    let state = self.dispatcher.interaction_state(session_id).await;
                    match state {
                        InteractionState::Resolved { .. } => return true,
                        InteractionState::Rejected { .. } => return false,
                        InteractionState::NoInteraction => return true,
                        InteractionState::Pending { .. } => continue,
                    }
                }
            }
        }
    }

    /// WP-0: 登记等待中的交互（原 set_approval_pending）。
    pub async fn set_interaction_pending(
        &self,
        session_id: String,
        interaction_id: String,
        kind: String,
        action: String,
    ) {
        self.dispatcher
            .set_interaction_pending(&session_id, interaction_id, kind, action)
            .await;
    }

    /// WP-3: 一次性取回已解决交互的响应（内核只搬运不解析）。
    pub async fn take_interaction_response(
        &self,
        session_id: &str,
    ) -> Option<(bool, serde_json::Value)> {
        self.dispatcher.take_interaction_response(session_id).await
    }

    /// Get tool descriptions for the LLM.
    pub fn tool_schemas(&self) -> Vec<llm_gateway::ToolSchema> {
        self.dispatcher
            .list_tools()
            .into_iter()
            .map(|d| llm_gateway::ToolSchema {
                name: d.name,
                description: d.description,
                parameters: d.parameters,
            })
            .collect()
    }

    /// Get reference to the tool context.
    pub fn tool_context(&self) -> &ToolContext {
        &self.ctx
    }

    /// WS8 (v0.2): 写 scratch（body 状态注入——introspect 工具读取）。
    /// scratch 须为 JSON object（ToolContext::default 保证）。
    pub fn set_scratch(&mut self, key: &str, value: serde_json::Value) {
        if let Some(obj) = self.ctx.scratch.as_object_mut() {
            obj.insert(key.to_string(), value);
        }
    }

    /// R2-F (v0.2.6): 运行时更新工具环境变量（egress 审批放行后追加白名单——
    /// ToolContext.env 是 web_fetch 的唯一读取来源，T10 契约不变）。
    pub fn set_env_var(&mut self, key: &str, value: &str) {
        self.ctx.env.insert(key.to_string(), value.to_string());
    }

    /// R2-F: 读取工具环境变量。
    pub fn env_var(&self, key: &str) -> Option<&String> {
        self.ctx.env.get(key)
    }

    /// Get reference to the dispatcher.
    pub fn dispatcher(&self) -> &Arc<ToolDispatcher> {
        &self.dispatcher
    }

    /// v10.4: Expose tool context for orchestrator integration.
    /// P1-LTR-01: 注入任务级绝对截止（Phase 2）——run() 开头调用（H2 同源），
    /// 每轮 continue_turn 重建；None = 无 deadline（旧行为）。
    pub fn set_task_deadline(&mut self, deadline: Option<std::time::Instant>) {
        self.ctx.task_deadline = deadline;
    }

    pub fn ctx(&self) -> &ToolContext {
        &self.ctx
    }

    /// v10.4: Dispatch a single tool and return a JSON value result,
    /// for use as the runner closure in orchestrator::execute_plan.
    pub async fn dispatch_single(
        &self,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<String, String> {
        self.dispatcher
            .dispatch(tool_name, args, &self.ctx)
            .await
            .map_err(|e| format!("{e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P2-LR Node 02（RC46 接线，批-4）：五态可区分测试矩阵——
    /// exit 0 / exit >0 / signal / tool timeout / task deadline。
    #[test]
    fn test_five_state_error_classification() {
        // task deadline（既有先例语义）
        let deadline = anyhow::Error::new(TaskDeadlineExceeded {
            tool: "bash".into(),
        });
        assert_eq!(
            classify_dispatch_error(&deadline),
            Some(agent_types::ToolErrorKind::DeadlineExceeded)
        );
        // exit >0（断言/测试失败族）
        let nz = anyhow::Error::new(BashExitError {
            exit_code: 1,
            timed_out: false,
            formatted: "exit code: 1".into(),
        });
        assert_eq!(
            classify_dispatch_error(&nz),
            Some(agent_types::ToolErrorKind::ExitNonZero(1))
        );
        // signal（sandbox 报告负退出码——seccomp KILL 等）
        let sig = anyhow::Error::new(BashExitError {
            exit_code: -1,
            timed_out: false,
            formatted: "exit code: -1".into(),
        });
        assert_eq!(
            classify_dispatch_error(&sig),
            Some(agent_types::ToolErrorKind::ExitSignal(-1))
        );
        // tool timeout（sandbox 层 timed_out）
        let to = anyhow::Error::new(BashExitError {
            exit_code: 0,
            timed_out: true,
            formatted: "error: command timed out".into(),
        });
        assert_eq!(
            classify_dispatch_error(&to),
            Some(agent_types::ToolErrorKind::ToolTimeout)
        );
        // 未知来源 → None（不冒充，FA01 基线）
        let unknown = anyhow::anyhow!("some other failure");
        assert_eq!(classify_dispatch_error(&unknown), None);
    }

    /// 集成：真实 bash 工具 dispatch "exit 1" → ToolResult.error_kind =
    /// ExitNonZero(1)（修复前 None——文本只进 output）。
    #[tokio::test]
    async fn test_bash_exit_code_structured_projection() {
        let mut dispatcher = ToolDispatcher::new();
        dispatcher.register(Arc::new(tools_builtin::BashTool::new()));
        let sched = Scheduler::new(Arc::new(dispatcher), tool_runtime::ToolContext::default());
        let calls = vec![agent_types::ToolCall {
            call_id: "c1".into(),
            name: "bash".into(),
            args: serde_json::json!({"cmd": "exit 3"}),
        }];
        let results = sched.execute_tool_calls(&calls).await;
        assert_eq!(results.len(), 1);
        assert!(results[0].is_error);
        assert_eq!(
            results[0].error_kind,
            Some(agent_types::ToolErrorKind::ExitNonZero(3))
        );
    }
}
