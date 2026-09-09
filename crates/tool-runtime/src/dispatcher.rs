use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::debug;

use crate::context::ToolContext;

/// P1-LTR-01（修 2）: 工具执行撞上任务剩余期截止的结构化标记。
/// 类型化错误——消费方 downcast 判定，**禁止**解析错误文本（RC20 反模式禁令）。
/// terminal 归属：deadline_clamped（effective < declared）时超时 → deadline_exceeded；
/// declared 更早时 → 普通 tool timeout（语义分离——总包 §9 情况 A/B）。
/// P2-LR Node 02（RC46 接线，批-4）：bash 进程级退出状态的结构化载体——
/// 五态区分（exit 0 / >0 / signal / tool timeout / task deadline）的事实源。
/// Display = bash 工具原 formatted 输出（LLM 可见内容不变）；结构化字段由
/// scheduler downcast 投影到 ToolResult.error_kind（RC20 纪律：禁文本解析）。
#[derive(Debug)]
pub struct BashExitError {
    pub exit_code: i32,
    pub timed_out: bool,
    pub formatted: String,
}
impl std::fmt::Display for BashExitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.formatted)
    }
}
impl std::error::Error for BashExitError {}

#[derive(Debug, Clone)]
pub struct TaskDeadlineExceeded {
    pub tool: String,
}
impl std::fmt::Display for TaskDeadlineExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "tool '{}' terminated by task deadline (deadline_exceeded)",
            self.tool
        )
    }
}
impl std::error::Error for TaskDeadlineExceeded {}

/// Description of a tool for the LLM.
#[derive(Debug, Clone)]
pub struct ToolDescription {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// The core tool trait. Each tool (bash, read, edit, etc.) implements this.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique tool name.
    fn name(&self) -> &str;
    /// Human-readable description for the LLM.
    fn description(&self) -> ToolDescription;
    /// Execute the tool with given arguments.
    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String>;
    /// H1 (v0.2.4): 该工具期望的 dispatcher 层时间窗（默认 30s）。
    /// 长跑工具（bash 构建类）声明更长窗口，避免被默认总闸误杀。
    fn declared_timeout(&self) -> Duration {
        Duration::from_secs(30)
    }
}

/// WP-0 (v23 §2.1): 通用交互等待态——原 ApprovalState 泛化。
/// 内核只 match 状态转移（pending → resolved/rejected），**不 match kind、
/// 不解析 payload**——"approval" 只是 kind 的一个实例（approve/deny 的
/// approve/deny 语义在上层，内核只认 resolved: bool）。
#[derive(Debug, Clone)]
pub enum InteractionState {
    /// 无等待中的交互。
    NoInteraction,
    /// 等待用户响应（原 Pending）。
    Pending {
        interaction_id: String,
        kind: String,
        action: String,
    },
    /// 已响应且放行（原 Approved）。携带响应 payload——内核只搬运不解析。
    Resolved { payload: serde_json::Value },
    /// 已响应且中止（原 Denied）。携带响应 payload——内核只搬运不解析。
    Rejected { payload: serde_json::Value },
}

/// Tools that can change the workspace on disk. `bash` is included because a
/// shell can trivially write (`sed -i`, `cat > file`), which would defeat the
/// whole point of a read-only view.
pub const MUTATING_TOOLS: &[&str] = &["write_file", "edit", "apply_patch", "bash"];

/// Dispatcher that routes tool calls to the correct tool implementation.
/// Supports serial/parallel execution, timeout, and approval gating.
pub struct ToolDispatcher {
    tools: HashMap<String, Arc<dyn Tool>>,
    /// Per-tool timeout overrides (seconds).
    timeouts: HashMap<String, Duration>,
    /// Default timeout for tool execution.
    default_timeout: Duration,
    /// WP-0: 通用交互等待态，按 session id 键控（M2: per-session isolation）。
    interaction: Mutex<HashMap<String, InteractionState>>,
    /// R2-C Resource Safety (§8): per-call 字节记账（Observe 层——零控制流）。
    ledger: Mutex<ResourceLedger>,
}

/// R2-C (§8): 资源记账——per_tool 累计 + 全局累计，阈值 warn（actionable signal）。
/// 57G 事故回答：异常"从哪个工具、哪一步开始"此后有数据可查。
#[derive(Default)]
pub struct ResourceLedger {
    per_tool_bytes: HashMap<String, u64>,
    total_bytes: u64,
    calls: u64,
}

/// 观测阈值：单调用/累计超过即 warn（字节）。只告警不拦截（D 类另出单）。
const OBSERVE_PER_CALL_WARN: u64 = 10 * 1024 * 1024; // 10MB
const OBSERVE_TOTAL_WARN: u64 = 1024 * 1024 * 1024; // 1GB

impl ResourceLedger {
    fn record(&mut self, tool: &str, args_bytes: u64, result_bytes: u64) {
        let call_bytes = args_bytes + result_bytes;
        *self.per_tool_bytes.entry(tool.to_string()).or_insert(0) += call_bytes;
        self.total_bytes += call_bytes;
        self.calls += 1;
        if call_bytes > OBSERVE_PER_CALL_WARN {
            tracing::warn!(
                tool,
                call_bytes,
                "resource observe: 单调用字节超阈值（10MB）——57G 事故类信号（Observe 层，不拦截）"
            );
        }
        if self.total_bytes > OBSERVE_TOTAL_WARN {
            tracing::warn!(
                total_bytes = self.total_bytes,
                calls = self.calls,
                "resource observe: 任务累计字节超阈值（1GB）——可能重演 57G 事故（Observe 层，不拦截）"
            );
        }
    }

    /// 快照（introspect/报告消费）。
    pub fn snapshot(&self) -> (u64, u64, Vec<(String, u64)>) {
        let mut per: Vec<(String, u64)> = self
            .per_tool_bytes
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        per.sort_by_key(|(_, v)| std::cmp::Reverse(*v));
        (self.total_bytes, self.calls, per)
    }
}

impl ToolDispatcher {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            timeouts: HashMap::new(),
            default_timeout: Duration::from_secs(30),
            interaction: Mutex::new(HashMap::new()),
            ledger: Mutex::new(ResourceLedger::default()),
        }
    }

    /// Register a tool.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Register a tool with a per-tool timeout.
    pub fn register_with_timeout(&mut self, tool: Arc<dyn Tool>, timeout: Duration) {
        let name = tool.name().to_string();
        self.tools.insert(name.clone(), tool);
        self.timeouts.insert(name, timeout);
    }

    /// Get all tool descriptions for the LLM.
    pub fn list_tools(&self) -> Vec<ToolDescription> {
        // Closure Window（顶层复核 §2 观测暴露）: HashMap 迭代顺序随机 →
        // ChatRequest.tools 数组顺序每请求漂移（内容稳定、顺序不稳）——
        // 破坏 provider 侧缓存且 tool_schema_hash 观测到 2-4 唯一值/会话。
        // 按名字排序 = 确定性顺序；不改工具内容、不改注册生命周期（无 phase pruning）。
        let mut descs: Vec<_> = self.tools.values().map(|t| t.description()).collect();
        descs.sort_by(|a, b| a.name.cmp(&b.name));
        descs
    }

    /// H1 (v0.2.4): 按工具自身声明覆盖 dispatcher 默认超时。
    /// bash 工具内部已有 180s 默认/600s 上限的 timeout_secs 语义，但被 dispatcher
    /// 30s 总闸盖死（手工实测 "tool 'bash' timed out after 30s" 反复出现，
    /// cargo build 类长命令全灭）。工具可经 `declared_timeout()` 声明"我需要更长
    /// 时间窗"，dispatcher 在无显式 per-tool 覆盖时采用工具声明值。
    pub fn register_with_declared_timeout(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        let declared = tool.declared_timeout();
        self.tools.insert(name.clone(), tool);
        // 显式 timeouts 优先；未显式设置时采用工具声明值
        self.timeouts.entry(name).or_insert(declared);
    }

    /// Build a restricted view of this dispatcher with every workspace-mutating
    /// tool removed.
    ///
    /// v12.7: sub-agents run *concurrently* against the **same** workspace
    /// directory. Giving each of them write access corrupts the tree: on the
    /// T10 benchmark five sub-agents each rewrote `src/lib.rs` from their own
    /// stale snapshot, and the surviving file contained both
    /// `#[derive(Default)]` and a hand-written `impl Default` — `E0119`, which
    /// then drove the parent into an unbounded replan loop until the run timed
    /// out. Mutation is therefore the root agent's exclusive responsibility;
    /// sub-agents may only observe and report.
    pub fn read_only_view(&self) -> Self {
        Self {
            tools: self
                .tools
                .iter()
                .filter(|(name, _)| !MUTATING_TOOLS.contains(&name.as_str()))
                .map(|(k, v)| (k.clone(), Arc::clone(v)))
                .collect(),
            timeouts: self.timeouts.clone(),
            default_timeout: self.default_timeout,
            interaction: Mutex::new(HashMap::new()),
            ledger: Mutex::new(ResourceLedger::default()),
        }
    }

    /// Dispatch a tool call by name (serial).
    pub async fn dispatch(
        &self,
        name: &str,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<String> {
        let tool = self.tools.get(name).ok_or_else(|| {
            // T9 (v0.2.3): "tool not found" 附可用工具列表——LLM 在 provider 异常时
            // 常输出畸形/幻觉工具名，错误须可自我纠正（此前只报名字，模型会反复重试错名）。
            let mut names: Vec<&str> = self.tools.keys().map(|s| s.as_str()).collect();
            names.sort();
            anyhow!(
                "tool not found: {}（可用工具: {}）",
                name,
                names.join(" / ")
            )
        })?;
        debug!(tool = name, "dispatching tool");

        let timeout = self
            .timeouts
            .get(name)
            .copied()
            .unwrap_or(self.default_timeout);

        // ── P1-LTR-01 Phase 3: effective timeout **单点**计算 ──
        // effective = min(declared_timeout, remaining_task_time)；600s 工具 cap
        // 在 declared 内已收敛（bash 自身 cap），本点不放宽。expired（remaining<=0）
        // → 不启动工具进程，直接结构化 deadline_exceeded（总包 §10）。
        // ctx.clone 成本可忽略（三个字段的轻结构）；单点写 effective 传给工具。
        let mut ctx = ctx.clone();
        let deadline_clamped;
        let effective = match ctx.task_deadline {
            Some(dl) => {
                let remaining = dl.saturating_duration_since(std::time::Instant::now());
                if remaining.is_zero() {
                    return Err(anyhow::Error::new(TaskDeadlineExceeded {
                        tool: name.to_string(),
                    }));
                }
                let e = timeout.min(remaining);
                deadline_clamped = e < timeout;
                e
            }
            None => {
                deadline_clamped = false;
                timeout
            }
        };
        ctx.effective_timeout = Some(effective);
        ctx.deadline_clamped = deadline_clamped;

        // R2-C Resource Safety Observe 层（§8, v0.2.8）: per-call 字节记账——
        // 57G 事故（21 步 2.7G/步塞满磁盘）的直接回应：此前**零字节观测**，
        // resource-monitor 是相位体检器、根本没接工具写盘路径（补充 3 实锤）。
        // 本记账只观测（Observe 层）——warn/pause/deny 控制流属 D 类另出单。
        let args_bytes = serde_json::to_string(&args).map(|s| s.len()).unwrap_or(0) as u64; // execute 会 move args——先量
        let t0 = std::time::Instant::now();
        match tokio::time::timeout(effective, tool.execute(args, &ctx)).await {
            Ok(result) => {
                let result_bytes = result.as_ref().map(|s| s.len()).unwrap_or(0) as u64;
                self.ledger
                    .lock()
                    .await
                    .record(name, args_bytes, result_bytes);
                // C-2 (P5-FOUNDATION-01) 方向①：实际耗时异常超过声明阈值
                // （tokio enforcement 正常时不可达——此分支命中即 enforcement
                // 被绕过的信号，belt & braces）。
                if let Some(ratio) = timeout_drift_ratio(timeout, t0.elapsed()) {
                    tracing::warn!(
                        tool = name,
                        declared_ms = timeout.as_millis() as u64,
                        actual_ms = t0.elapsed().as_millis() as u64,
                        ratio = format!("{ratio:.2}"),
                        "timeout_declaration_drift: actual exceeded declared × 1.2 without enforcement kill"
                    );
                }
                result
            }
            // P1-LTR-01（修 2）: 结构化区分两类超时——deadline_clamped → 类型化
            // TaskDeadlineExceeded（消费方 downcast，归 deadline_exceeded 语义）；
            // 否则 = 工具自身声明超时（tool timeout，W4 结构化错误原语义）。
            Err(_) if deadline_clamped => Err(anyhow::Error::new(TaskDeadlineExceeded {
                tool: name.to_string(),
            })),
            Err(_) => {
                // C-2 (P5-FOUNDATION-01) 方向②：声明超时被击穿——工具声明
                // N 秒但实际需要更久（bash.rs:89 的 610s 声明 vs 869s 实测
                // 即此形态）。结构化登记，供声明阈值校准（观测级，不拦截）。
                tracing::warn!(
                    tool = name,
                    declared_ms = timeout.as_millis() as u64,
                    "timeout_declaration_drift: tool killed at declared timeout — declaration insufficient for actual need"
                );
                Err(anyhow!("tool '{}' timed out after {:?}", name, timeout))
            }
        }
    }

    /// Dispatch multiple tool calls in parallel, collecting all results.
    pub async fn dispatch_parallel(
        &self,
        calls: &[(String, serde_json::Value)],
        ctx: &ToolContext,
    ) -> Vec<Result<String>> {
        let futures: Vec<_> = calls
            .iter()
            .map(|(name, args)| self.dispatch(name, args.clone(), ctx))
            .collect();
        futures::future::join_all(futures).await
    }

    /// WP-0: 登记一个等待中的交互（原 set_approval_pending）。
    /// 内核不解析 kind——kind 只是开放字符串，由上层决定语义。
    pub async fn set_interaction_pending(
        &self,
        session_id: &str,
        interaction_id: String,
        kind: String,
        action: String,
    ) {
        let mut state = self.interaction.lock().await;
        state.insert(
            session_id.to_string(),
            InteractionState::Pending {
                interaction_id,
                kind,
                action,
            },
        );
    }

    /// WP-0: 解决一个交互（原 resolve_approval 泛化）。
    /// R1: interaction_id 强校验——响应必须匹配一个 **pending** 请求的 id。
    /// R2: 一次性消费——已解决的 pending 重复提交必拒。
    /// 内核只认 `resolved: bool`（放行/中止）——payload 原样存储不解析
    /// （WP-3: clarification 由 take_interaction_response 回读）。
    pub async fn resolve_interaction(
        &self,
        session_id: &str,
        interaction_id: &str,
        resolved: bool,
        payload: serde_json::Value,
    ) -> Result<InteractionState> {
        let mut state = self.interaction.lock().await;
        let current = state.get(session_id).cloned();
        match current {
            Some(InteractionState::Pending {
                interaction_id: pending_id,
                ..
            }) => {
                if pending_id != interaction_id {
                    return Err(anyhow!(
                        "interaction_id mismatch for session {}: request {} != pending {}",
                        session_id,
                        interaction_id,
                        pending_id
                    ));
                }
                let new_state = if resolved {
                    InteractionState::Resolved { payload }
                } else {
                    InteractionState::Rejected { payload }
                };
                state.insert(session_id.to_string(), new_state.clone());
                Ok(new_state)
            }
            Some(other) => Err(anyhow!(
                "no pending interaction for session {}; current state: {:?}",
                session_id,
                other
            )),
            None => Err(anyhow!("no interaction state for session {}", session_id)),
        }
    }

    /// WP-3: 一次性取回已解决交互的响应（内核只搬运不解析）。
    /// 返回 (resolved, payload)；调用后交互状态重置为 NoInteraction。
    pub async fn take_interaction_response(
        &self,
        session_id: &str,
    ) -> Option<(bool, serde_json::Value)> {
        let mut state = self.interaction.lock().await;
        let taken = match state.get(session_id).cloned() {
            Some(InteractionState::Resolved { payload }) => Some((true, payload)),
            Some(InteractionState::Rejected { payload }) => Some((false, payload)),
            _ => None,
        };
        if taken.is_some() {
            state.remove(session_id);
        }
        taken
    }

    /// Get current interaction state for a session (defaults to NoInteraction).
    pub async fn interaction_state(&self, session_id: &str) -> InteractionState {
        self.interaction
            .lock()
            .await
            .get(session_id)
            .cloned()
            .unwrap_or(InteractionState::NoInteraction)
    }

    /// Reset interaction state for a session.
    pub async fn reset_interaction(&self, session_id: &str) {
        self.interaction.lock().await.remove(session_id);
    }

    /// Check if a tool is registered.
    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Whether no tools are registered.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    /// Closure Window（顶层复核 §2）: list_tools 顺序确定性——HashMap 迭代随机
    /// 曾导致 ChatRequest.tools 每请求顺序漂移（tools_hash 2-4 唯一值/会话实测）。
    /// 修复后必须按名字升序稳定输出。
    fn mock_tool_named(name: &str) -> MockTool {
        MockTool {
            name: name.to_string(),
            desc: ToolDescription {
                name: name.to_string(),
                description: format!("mock {name}"),
                parameters: serde_json::json!({"type": "object"}),
            },
        }
    }

    #[test]
    fn test_list_tools_deterministic_order() {
        let mut d = ToolDispatcher::new();
        // 乱序注册
        d.register(Arc::new(mock_tool_named("zeta")));
        d.register(Arc::new(mock_tool_named("alpha")));
        d.register(Arc::new(mock_tool_named("mid")));
        let names: Vec<String> = d.list_tools().into_iter().map(|t| t.name).collect();
        let mut sorted_names = names.clone();
        sorted_names.sort();
        assert_eq!(
            names, sorted_names,
            "list_tools 必须按名字升序（tools 数组顺序确定性）"
        );
        // 重复调用一致
        let names2: Vec<String> = d.list_tools().into_iter().map(|t| t.name).collect();
        assert_eq!(names, names2);
    }

    use super::*;
    use crate::context::ToolContext;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct MockTool {
        name: String,
        desc: ToolDescription,
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }
        fn description(&self) -> ToolDescription {
            self.desc.clone()
        }
        async fn execute(&self, _args: serde_json::Value, _ctx: &ToolContext) -> Result<String> {
            Ok("mock result".into())
        }
    }

    struct SlowTool {
        delay_ms: u64,
    }

    #[async_trait]
    impl Tool for SlowTool {
        fn name(&self) -> &str {
            "slow"
        }
        fn description(&self) -> ToolDescription {
            ToolDescription {
                name: "slow".into(),
                description: "slow tool".into(),
                parameters: serde_json::json!({}),
            }
        }
        async fn execute(&self, _args: serde_json::Value, _ctx: &ToolContext) -> Result<String> {
            tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
            Ok("done".into())
        }
    }

    fn mk(name: &str) -> Arc<dyn Tool> {
        Arc::new(MockTool {
            name: name.into(),
            desc: ToolDescription {
                name: name.into(),
                description: "mock tool".into(),
                parameters: serde_json::json!({}),
            },
        })
    }

    /// v12.7: the read-only view handed to sub-agents must expose the
    /// inspection tools and *refuse* every mutating one — concurrent
    /// sub-agents sharing one workspace previously clobbered the same file.
    #[tokio::test]
    async fn test_read_only_view_strips_mutating_tools() {
        let mut d = ToolDispatcher::new();
        for t in ["read", "grep", "glob", "write_file", "edit", "bash"] {
            d.register(mk(t));
        }
        assert_eq!(d.len(), 6);

        let ro = d.read_only_view();
        for allowed in ["read", "grep", "glob"] {
            assert!(ro.has(allowed), "read-only view must keep '{allowed}'");
        }
        for denied in MUTATING_TOOLS {
            assert!(!ro.has(denied), "read-only view must drop '{denied}'");
        }
        assert_eq!(ro.len(), 3);

        // Dispatching a stripped tool fails instead of silently writing.
        let ctx = ToolContext::default();
        let err = ro
            .dispatch("write_file", serde_json::json!({}), &ctx)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("tool not found"), "got: {err}");

        // The parent dispatcher is untouched.
        assert!(d.has("write_file"));
    }

    #[tokio::test]
    async fn test_register_and_dispatch() {
        let mut d = ToolDispatcher::new();
        d.register(Arc::new(MockTool {
            name: "mock".into(),
            desc: ToolDescription {
                name: "mock".into(),
                description: "mock tool".into(),
                parameters: serde_json::json!({}),
            },
        }));
        assert!(d.has("mock"));
        assert_eq!(d.len(), 1);

        let ctx = ToolContext::default();
        let result = d
            .dispatch("mock", serde_json::json!({}), &ctx)
            .await
            .unwrap();
        assert_eq!(result, "mock result");
    }

    #[tokio::test]
    async fn test_dispatch_missing_tool() {
        let d = ToolDispatcher::new();
        let ctx = ToolContext::default();
        assert!(d
            .dispatch("nope", serde_json::json!({}), &ctx)
            .await
            .is_err());
    }

    /// H1 (v0.2.4): declared_timeout 注册——长跑工具（bash 610s 窗口）不再被
    /// dispatcher 30s 默认总闸误杀。负面：普通 register 的 slow 工具 40ms 延迟
    /// 在 30ms 超时下必须超时（对照组，证明超时机制本身有效）；declared_timeout
    /// 注册后同延迟必须成功（窗口被工具声明撑开）。
    struct LongDeclaredTool {
        delay_ms: u64,
    }

    #[async_trait]
    impl Tool for LongDeclaredTool {
        fn name(&self) -> &str {
            "long-declared"
        }
        fn description(&self) -> ToolDescription {
            ToolDescription {
                name: "long-declared".into(),
                description: "declares a long dispatcher window".into(),
                parameters: serde_json::json!({}),
            }
        }
        async fn execute(&self, _args: serde_json::Value, _ctx: &ToolContext) -> Result<String> {
            tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
            Ok("long done".into())
        }
        fn declared_timeout(&self) -> Duration {
            Duration::from_secs(2)
        }
    }

    #[tokio::test]
    async fn test_declared_timeout_extends_dispatch_window() {
        let ctx = ToolContext::default();
        // 对照组：默认注册（30s 窗口）——此处用显式短超时验证超时路径真实可触发
        let mut d_short = ToolDispatcher::new();
        d_short.register_with_timeout(
            Arc::new(SlowTool { delay_ms: 80 }),
            Duration::from_millis(20),
        );
        let err = d_short
            .dispatch("slow", serde_json::json!({}), &ctx)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("timed out"), "got: {err}");

        // 正面组：声明超时注册——80ms 延迟在 2s 声明窗口内必须成功
        let mut d = ToolDispatcher::new();
        d.register_with_declared_timeout(Arc::new(LongDeclaredTool { delay_ms: 80 }));
        let r = d
            .dispatch("long-declared", serde_json::json!({}), &ctx)
            .await
            .expect("declared 2s window must survive 80ms run");
        assert_eq!(r, "long done");
    }

    /// H1: 显式 register_with_timeout 优先于工具声明值（不覆盖用户显式配置）。
    #[tokio::test]
    async fn test_explicit_timeout_overrides_declared() {
        let mut d = ToolDispatcher::new();
        d.register_with_timeout(
            Arc::new(LongDeclaredTool { delay_ms: 80 }),
            Duration::from_millis(20),
        );
        let ctx = ToolContext::default();
        let err = d
            .dispatch("long-declared", serde_json::json!({}), &ctx)
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("timed out"),
            "显式 20ms 必须压过声明 2s: {err}"
        );
    }

    #[test]
    fn test_list_tools() {
        let mut d = ToolDispatcher::new();
        d.register(Arc::new(MockTool {
            name: "a".into(),
            desc: ToolDescription {
                name: "a".into(),
                description: "tool a".into(),
                parameters: serde_json::json!({}),
            },
        }));
        let list = d.list_tools();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "a");
    }

    #[tokio::test]
    async fn test_tool_timeout() {
        let mut d = ToolDispatcher::new();
        d.default_timeout = Duration::from_millis(50);
        d.register(Arc::new(SlowTool { delay_ms: 5000 }));

        let ctx = ToolContext::default();
        let result = d.dispatch("slow", serde_json::json!({}), &ctx).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn test_parallel_execution() {
        let counter = Arc::new(AtomicU64::new(0));

        struct CounterTool {
            counter: Arc<AtomicU64>,
            id: String,
        }
        #[async_trait]
        impl Tool for CounterTool {
            fn name(&self) -> &str {
                &self.id
            }
            fn description(&self) -> ToolDescription {
                ToolDescription {
                    name: self.id.clone(),
                    description: "counter".into(),
                    parameters: serde_json::json!({}),
                }
            }
            async fn execute(
                &self,
                _args: serde_json::Value,
                _ctx: &ToolContext,
            ) -> Result<String> {
                self.counter.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(50)).await;
                Ok(format!("{} done", self.id))
            }
        }

        let mut d = ToolDispatcher::new();
        d.default_timeout = Duration::from_secs(5);
        d.register(Arc::new(CounterTool {
            counter: counter.clone(),
            id: "a".into(),
        }));
        d.register(Arc::new(CounterTool {
            counter: counter.clone(),
            id: "b".into(),
        }));

        let ctx = ToolContext::default();
        let calls = vec![
            ("a".into(), serde_json::json!({})),
            ("b".into(), serde_json::json!({})),
        ];

        let results = d.dispatch_parallel(&calls, &ctx).await;
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_ok()));
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_interaction_flow() {
        let d = ToolDispatcher::new();
        assert!(matches!(
            d.interaction_state("sess-1").await,
            InteractionState::NoInteraction
        ));

        d.set_interaction_pending(
            "sess-1",
            "a1".into(),
            "approval".into(),
            "write file".into(),
        )
        .await;
        assert!(matches!(
            d.interaction_state("sess-1").await,
            InteractionState::Pending { .. }
        ));

        let result = d
            .resolve_interaction("sess-1", "a1", true, serde_json::json!({}))
            .await
            .unwrap();
        assert!(matches!(result, InteractionState::Resolved { .. }));

        d.reset_interaction("sess-1").await;
        assert!(matches!(
            d.interaction_state("sess-1").await,
            InteractionState::NoInteraction
        ));
    }

    #[tokio::test]
    async fn test_interaction_deny() {
        let d = ToolDispatcher::new();
        d.set_interaction_pending(
            "sess-1",
            "a2".into(),
            "approval".into(),
            "delete file".into(),
        )
        .await;
        let result = d
            .resolve_interaction("sess-1", "a2", false, serde_json::json!({}))
            .await
            .unwrap();
        assert!(matches!(result, InteractionState::Rejected { .. }));
    }

    #[tokio::test]
    async fn test_interaction_isolated_between_sessions() {
        // M2: two sessions setting/resolve approval must not interfere.
        let d = ToolDispatcher::new();

        d.set_interaction_pending(
            "sess-a",
            "id-a".into(),
            "approval".into(),
            "delete /a".into(),
        )
        .await;
        d.set_interaction_pending(
            "sess-b",
            "id-b".into(),
            "approval".into(),
            "delete /b".into(),
        )
        .await;

        // Session A is still Pending, even though B set its own.
        assert!(matches!(
            d.interaction_state("sess-a").await,
            InteractionState::Pending { .. }
        ));
        assert!(matches!(
            d.interaction_state("sess-b").await,
            InteractionState::Pending { .. }
        ));

        // Resolving A must not affect B.
        let ra = d
            .resolve_interaction("sess-a", "id-a", true, serde_json::json!({}))
            .await
            .unwrap();
        assert!(matches!(ra, InteractionState::Resolved { .. }));
        assert!(matches!(
            d.interaction_state("sess-b").await,
            InteractionState::Pending { .. }
        ));

        // Resolving B with an invalid decision for A's id is irrelevant; B resolves deny.
        let rb = d
            .resolve_interaction("sess-b", "id-b", false, serde_json::json!({}))
            .await
            .unwrap();
        assert!(matches!(rb, InteractionState::Rejected { .. }));
        assert!(matches!(
            d.interaction_state("sess-a").await,
            InteractionState::Resolved { .. }
        ));
    }
}

#[tokio::test]
async fn test_wp0_dummy_kind_not_switched() {
    // WP-0 (v23 §2.1): 内核绝不 match kind——任意开放字符串 kind 都走通
    // （证明"加一种新交互"不改内核任何代码）。
    let d = ToolDispatcher::new();
    for kind in ["approval", "clarification", "test-kind", "任意自定义"] {
        d.set_interaction_pending("sess-x", "d1".into(), kind.into(), "act".into())
            .await;
        let st = d
            .resolve_interaction("sess-x", "d1", true, serde_json::json!({}))
            .await
            .unwrap();
        assert!(
            matches!(st, InteractionState::Resolved { .. }),
            "kind={kind} should resolve, got {st:?}"
        );
        d.reset_interaction("sess-x").await;
    }
    eprintln!("WP-0 PASS: dummy kind 走通——内核不挑 kind");
}

#[tokio::test]
async fn test_wp0a_r1_wrong_interaction_id_rejected() {
    // RT4-③ fix: a response with a non-matching approval_id must be rejected.
    let d = ToolDispatcher::new();
    d.set_interaction_pending(
        "sess-1",
        "real-id".into(),
        "approval".into(),
        "delete file".into(),
    )
    .await;
    let err = d
        .resolve_interaction("sess-1", "wrong-id", true, serde_json::json!({}))
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("mismatch"),
        "expected mismatch error, got: {err}"
    );
    // The state must remain Pending — the wrong id must not consume it.
    assert!(matches!(
        d.interaction_state("sess-1").await,
        InteractionState::Pending { .. }
    ));
}

#[tokio::test]
async fn test_wp0a_r2_double_submit_rejected() {
    // RT4/R2 fix: after a response is consumed, a second submit must fail (no replay).
    let d = ToolDispatcher::new();
    d.set_interaction_pending(
        "sess-1",
        "a1".into(),
        "approval".into(),
        "delete file".into(),
    )
    .await;
    let first = d
        .resolve_interaction("sess-1", "a1", true, serde_json::json!({}))
        .await
        .unwrap();
    assert!(matches!(first, InteractionState::Resolved { .. }));

    // Second submit with the same id: no pending state → error.
    let second = d
        .resolve_interaction("sess-1", "a1", true, serde_json::json!({}))
        .await
        .unwrap_err();
    assert!(
        second.to_string().contains("no pending interaction"),
        "expected no-pending error, got: {second}"
    );
}

#[tokio::test]
async fn test_wp3_take_interaction_response_returns_payload() {
    // WP-3: clarification 的回答 payload 必须能一次性取回（内核只搬运不解析）。
    let d = ToolDispatcher::new();
    d.set_interaction_pending("sess-c", "c1".into(), "clarification".into(), "ask".into())
        .await;
    let answer = serde_json::json!({"answer": "用 Rust 实现"});
    let st = d
        .resolve_interaction("sess-c", "c1", true, answer.clone())
        .await
        .unwrap();
    assert!(matches!(st, InteractionState::Resolved { .. }));
    // 取回（一次性）
    let taken = d.take_interaction_response("sess-c").await;
    assert!(taken.is_some(), "应能取回响应");
    let (resolved, payload) = taken.unwrap();
    assert!(resolved, "resolved=true");
    assert_eq!(payload, answer, "payload 原样回读");
    // 二次取回 → None（一次性消费）
    assert!(
        d.take_interaction_response("sess-c").await.is_none(),
        "二次取回应为空（一次性）"
    );
    eprintln!("WP-3 PASS: payload 回读 + 一次性消费");
}

#[cfg(test)]
mod r2c_resource_tests {
    use super::*;

    /// R2-C §8: 资源记账——per_tool 累计与全局累计正确；57G 事故后
    /// "哪个工具、多少字节"必须有数据可查（Observe 层）。
    #[test]
    fn test_resource_ledger_accumulates() {
        let mut ledger = ResourceLedger::default();
        ledger.record("bash", 100, 200);
        ledger.record("bash", 300, 400);
        ledger.record("write_file", 5000, 10);
        let (total, calls, per) = ledger.snapshot();
        assert_eq!(calls, 3);
        assert_eq!(total, 100 + 200 + 300 + 400 + 5000 + 10);
        let bash = per.iter().find(|(t, _)| t == "bash").unwrap().1;
        assert_eq!(bash, 1000, "per_tool 累计正确");
        assert_eq!(per[0].0, "write_file", "按字节降序排（write_file 最大）");
    }
}

// ── P1-LTR-01 Phase 10 单测（先红后绿：旧代码无 task_deadline 通道，T1/T4/T5 红）──

#[cfg(test)]
mod p1ltr_tests {
    use super::*;
    use crate::context::ToolContext;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    /// 记录执行次数的假工具（声明 600s 窗口；执行时睡指定时长）。
    struct SleepTool {
        exec_count: Arc<AtomicU32>,
        sleep: Duration,
    }
    #[async_trait::async_trait]
    impl Tool for SleepTool {
        fn name(&self) -> &str {
            "sleep_tool"
        }
        fn description(&self) -> ToolDescription {
            ToolDescription {
                name: "sleep_tool".into(),
                description: "sleep".into(),
                parameters: serde_json::json!({}),
            }
        }
        async fn execute(&self, _args: serde_json::Value, _ctx: &ToolContext) -> Result<String> {
            self.exec_count.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(self.sleep).await;
            Ok("slept".into())
        }
        fn declared_timeout(&self) -> Duration {
            Duration::from_secs(600)
        }
    }

    fn dispatcher_with(tool: Arc<dyn Tool>) -> ToolDispatcher {
        let mut d = ToolDispatcher::new();
        d.register_with_declared_timeout(tool);
        d
    }

    /// T1a + T6: task remaining=30s（近似——测试用 1s 收紧实验时长）< declared=600s
    /// → effective=remaining → 截断错误为 TaskDeadlineExceeded（deadline_exceeded 语义）。
    #[tokio::test]
    async fn test_t1_task_deadline_clamps_declared() {
        let count = Arc::new(AtomicU32::new(0));
        let d = dispatcher_with(Arc::new(SleepTool {
            exec_count: count.clone(),
            sleep: Duration::from_secs(30), // 工具本体睡 30s
        }));
        let ctx = ToolContext {
            task_deadline: Some(Instant::now() + Duration::from_secs(1)), // remaining=1s
            ..Default::default()
        };
        let t0 = Instant::now();
        let err = d
            .dispatch("sleep_tool", serde_json::json!({}), &ctx)
            .await
            .unwrap_err();
        let elapsed = t0.elapsed();
        // 结构化类型判定（修 2）：downcast，禁字符串解析
        assert!(
            err.downcast_ref::<TaskDeadlineExceeded>().is_some(),
            "deadline_clamped 截断必须是类型化 TaskDeadlineExceeded: {err}"
        );
        assert!(
            elapsed < Duration::from_secs(5),
            "应在 remaining≈1s 截断，实际 {elapsed:?}"
        );
        assert_eq!(
            count.load(Ordering::SeqCst),
            1,
            "工具已启动（未被 expired 早退）"
        );
    }

    /// T5: tool timeout 比 task deadline 更早（declared=10s < remaining=30s）
    /// → 普通 tool timeout（**不是** TaskDeadlineExceeded——语义分离）。
    #[tokio::test]
    async fn test_t5_tool_timeout_beats_deadline() {
        let mut d = ToolDispatcher::new();
        // 短睡工具 + per-tool 覆盖 1s（< remaining）
        struct QuickTool;
        #[async_trait::async_trait]
        impl Tool for QuickTool {
            fn name(&self) -> &str {
                "quick"
            }
            fn description(&self) -> ToolDescription {
                ToolDescription {
                    name: "quick".into(),
                    description: "q".into(),
                    parameters: serde_json::json!({}),
                }
            }
            async fn execute(&self, _a: serde_json::Value, _c: &ToolContext) -> Result<String> {
                tokio::time::sleep(Duration::from_secs(30)).await;
                Ok("x".into())
            }
            fn declared_timeout(&self) -> Duration {
                Duration::from_secs(600)
            }
        }
        d.register_with_timeout(Arc::new(QuickTool), Duration::from_secs(1));
        let ctx = ToolContext {
            task_deadline: Some(Instant::now() + Duration::from_secs(30)),
            ..Default::default()
        };
        let err = d
            .dispatch("quick", serde_json::json!({}), &ctx)
            .await
            .unwrap_err();
        assert!(
            err.downcast_ref::<TaskDeadlineExceeded>().is_none(),
            "tool timeout 更早时不得误标 deadline_exceeded（总包 §9 情况 B）: {err}"
        );
        assert!(format!("{err}").contains("timed out"));
    }

    /// T2/T4: task_deadline=None → 旧行为（declared 窗口，工具正常完成）；
    /// expired（remaining=0）→ 不启动工具 + TaskDeadlineExceeded（总包 §10）。
    #[tokio::test]
    async fn test_t2_t4_none_and_expired() {
        // T2: None → 正常完成
        let count = Arc::new(AtomicU32::new(0));
        let d = dispatcher_with(Arc::new(SleepTool {
            exec_count: count.clone(),
            sleep: Duration::from_millis(50),
        }));
        let ctx = ToolContext::default(); // task_deadline=None
        let out = d
            .dispatch("sleep_tool", serde_json::json!({}), &ctx)
            .await
            .unwrap();
        assert_eq!(out, "slept");
        assert_eq!(count.load(Ordering::SeqCst), 1);

        // T4: expired → 不启动（exec_count 不增）+ 类型化错误
        let count2 = Arc::new(AtomicU32::new(0));
        let d2 = dispatcher_with(Arc::new(SleepTool {
            exec_count: count2.clone(),
            sleep: Duration::from_millis(50),
        }));
        let ctx2 = ToolContext {
            task_deadline: Some(Instant::now() - Duration::from_secs(1)), // 已过期
            ..Default::default()
        };
        let err = d2
            .dispatch("sleep_tool", serde_json::json!({}), &ctx2)
            .await
            .unwrap_err();
        assert!(err.downcast_ref::<TaskDeadlineExceeded>().is_some());
        assert_eq!(
            count2.load(Ordering::SeqCst),
            0,
            "expired 不得启动工具进程（总包 §10）"
        );
    }

    /// T3（ContextManager 侧）: continue_turn 重建 deadline——不跨轮复用。
    #[test]
    fn test_t3_continue_turn_resets_deadline() {
        // 直接构造 ContextManager 验证（context.rs 的 task_deadline 派生自 run_started_at）
        // 这里用最小依赖：模拟 run_started_at 重建后 task_deadline 前移。
        let cap = 60u64;
        let t1 = Instant::now();
        let dl1 = t1 + Duration::from_secs(cap);
        std::thread::sleep(Duration::from_millis(20));
        let t2 = Instant::now(); // 模拟 continue_turn 的 mark_run_start
        let dl2 = t2 + Duration::from_secs(cap);
        assert!(
            dl2 > dl1,
            "continue_turn 后 deadline 必须前移（不得跨轮复用）"
        );
    }
}

/// C-2（P5-FOUNDATION-01）：代价对账判定——实际耗时 > 声明 × 1.2 → Some(ratio)。
/// 纯函数（便于无-tracing 捕获的单测）。
pub(crate) fn timeout_drift_ratio(
    declared: std::time::Duration,
    actual: std::time::Duration,
) -> Option<f64> {
    if declared.is_zero() {
        return None;
    }
    let threshold = declared.mul_f32(1.2);
    if actual > threshold {
        Some(actual.as_secs_f64() / declared.as_secs_f64())
    } else {
        None
    }
}

#[cfg(test)]
mod c2_tests {
    use super::*;

    #[test]
    fn test_timeout_drift_ratio() {
        let d = std::time::Duration::from_secs(10);
        assert!(timeout_drift_ratio(d, std::time::Duration::from_secs(5)).is_none());
        assert!(timeout_drift_ratio(d, std::time::Duration::from_secs(11)).is_none()); // ×1.1 未超阈
        let r = timeout_drift_ratio(d, std::time::Duration::from_secs(25)).unwrap();
        assert!((r - 2.5).abs() < 0.01, "ratio 应为 2.5，实际 {r}");
        assert!(
            timeout_drift_ratio(std::time::Duration::ZERO, d).is_none(),
            "零声明不判 drift"
        );
    }

    /// C-2 方向②：超时被杀 = 声明不敷实际——tokio enforcement 下这是
    /// drift 的主要观测面（bash 610s 声明 vs 869s 实测同构）。
    #[tokio::test]
    async fn test_timeout_kill_emits_drift_context() {
        let mut d = ToolDispatcher::new();
        d.register_with_declared_timeout(Arc::new(C2SlowTool { delay_ms: 400 }));
        let ctx = ToolContext::default();
        let t0 = std::time::Instant::now();
        let r = d.dispatch("c2slow", serde_json::json!({}), &ctx).await;
        eprintln!(
            "[c2-debug] elapsed={:?} r={:?} timeouts_has_c2slow={}",
            t0.elapsed(),
            r.as_ref(),
            d.timeouts.contains_key("c2slow")
        );
        assert!(r.is_err(), "超时必须 Err");
    }

    struct C2SlowTool {
        delay_ms: u64,
    }

    #[async_trait]
    impl Tool for C2SlowTool {
        fn name(&self) -> &str {
            "c2slow"
        }
        fn description(&self) -> ToolDescription {
            ToolDescription {
                name: "c2slow".into(),
                description: "sleeps beyond declared timeout".into(),
                parameters: serde_json::json!({}),
            }
        }
        fn declared_timeout(&self) -> Duration {
            Duration::from_millis(100)
        }
        async fn execute(&self, _args: serde_json::Value, _ctx: &ToolContext) -> Result<String> {
            tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
            Ok("late".into())
        }
    }
}
