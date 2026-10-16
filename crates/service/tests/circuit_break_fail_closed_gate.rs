//! D-208（2026-10-08, traecode）**行为级**门禁：G0 红线熔断必须被**执行方消费**并 fail-closed 置终态。
//!
//! 病灶：`Observer::run_and_report` 返回的 `Vec<CircuitBreak>` 在**唯一调用点**
//! （`agent-runtime/src/session.rs`）被**丢弃** ⇒ "红线即停机"从未生效：一次**越权读**
//! （工具产出确定性护栏拒绝串）跑完后，会话对外仍是 `done`（**正常完成**），
//! 客户端/用户只在"没人自动读"的 observer 报告文件里才能看到这条红线。
//!
//! 本门禁：脚本化 provider 让 agent 发起 `read {"path":"../escape.txt"}`
//! （`read` **不需审批**，工具的路径护栏直接产出 `path traversal denied`）
//! ⇒ observer 判 `sandbox_violation` ⇒ **修复后会话终态必须是 `blocked`**；
//! 修复前恒 `done` ⇒ **RED**。
//!
//! 盲区（自报）：只覆盖**安全级**红线（`sandbox_violation`）这一条被判定的路径；
//! `retry_storm`/`budget_exhausted` 按设计**不翻终态**（见 `red_line_terminal_phase` 文档）。

use std::sync::{Arc, Mutex};

use anyhow::Result;
use api::SessionCreate;
use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use llm_gateway::{
    Capabilities, ChatRequest, ChatResponse, Embedding, LlmProvider, ProviderRegistry, StreamEvent,
};
use tool_runtime::{ToolContext, ToolDispatcher};
use tools_builtin::{EditTool, GlobTool, GrepTool, ReadTool};

// ── 脚本化 provider（无需真实 key；镜像 integration_test 的 MockProvider 形状） ──

struct ScriptedProvider {
    name: String,
    responses: Mutex<Vec<ChatResponse>>,
}

impl ScriptedProvider {
    fn new(name: &str, responses: Vec<ChatResponse>) -> Self {
        Self {
            name: name.to_string(),
            responses: Mutex::new(responses),
        }
    }
}

#[async_trait]
impl LlmProvider for ScriptedProvider {
    fn name(&self) -> &str {
        &self.name
    }
    fn model(&self) -> &str {
        "mock-model"
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            chat: true,
            stream: true,
            function_calling: true,
            embeddings: false,
            max_context_tokens: Some(8192),
        }
    }
    async fn chat(&self, _req: ChatRequest) -> Result<ChatResponse> {
        let mut q = self.responses.lock().unwrap();
        if !q.is_empty() {
            return Ok(q.remove(0));
        }
        // 脚本耗尽 → 自然收尾
        Ok(ChatResponse {
            content: Some("DONE".into()),
            tool_calls: vec![],
            finish_reason: Some("stop".into()),
            usage: None,
            reasoning_content: None,
        })
    }
    fn stream(&self, _req: ChatRequest) -> BoxStream<'static, Result<StreamEvent>> {
        Box::pin(stream::empty())
    }
    async fn embed(&self, _inputs: &[String]) -> Result<Vec<Embedding>> {
        Ok(vec![])
    }
}

fn stop(content: &str) -> ChatResponse {
    ChatResponse {
        content: Some(content.into()),
        tool_calls: vec![],
        finish_reason: Some("stop".into()),
        usage: None,
        reasoning_content: None,
    }
}

fn red_line_script() -> Vec<ChatResponse> {
    vec![
        // 首个 chat 即给**工具调用**（实测：无需先给 plan JSON，loop 也会派发 `read`）
        ChatResponse {
            content: Some("I will read the file.".into()),
            tool_calls: vec![agent_types::ToolCall {
                call_id: "c1".into(),
                name: "read".into(),
                args: serde_json::json!({"path": "../escape.txt"}),
            }],
            finish_reason: Some("tool_calls".into()),
            usage: None,
            reasoning_content: None,
        },
        stop("DONE"),
        stop("DONE"),
        stop("DONE"),
        stop("DONE"),
    ]
}

#[tokio::test]
async fn sandbox_red_line_fail_closed_sets_blocked_terminal_phase() {
    // observer 报告写临时目录（不污染仓库）。本文件只有一个测试 ⇒ 无 env 竞争。
    let obs_dir = std::env::temp_dir().join(format!("hearth-d208-{}", uuid::Uuid::new_v4()));
    std::env::set_var("CODEX_OBSERVER_DIR", &obs_dir);

    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(ScriptedProvider::new(
        "mock-redline",
        red_line_script(),
    )));
    let registry = Arc::new(registry);

    let mut dispatcher = ToolDispatcher::new();
    dispatcher.register(Arc::new(ReadTool::new()));
    dispatcher.register(Arc::new(EditTool::new()));
    dispatcher.register(Arc::new(GlobTool::new()));
    dispatcher.register(Arc::new(GrepTool::new()));
    let dispatcher = Arc::new(dispatcher);

    let mut mgr =
        service::session::SessionManager::new(registry, dispatcher, ToolContext::default());
    // Q3/D-208：接上 Observer（生产同款接线见 service/main.rs）
    mgr.set_observer(Arc::new(observer::Observer::new()));
    let mgr = Arc::new(mgr);

    let created = mgr
        .create_session(SessionCreate {
            provider: "mock-redline".into(),
            model: None,
            goal: "explain reading a file".into(),
            budget: None,
        })
        .await
        .expect("建会话");
    let sid = created.session_id;
    let mut rx = mgr
        .send_message(
            &sid,
            api::MessageReq {
                content: "read it".into(),
            },
        )
        .await
        .expect("发消息");

    // 等 Done（或流关闭 / 超时兜底）
    let timeout = tokio::time::sleep(std::time::Duration::from_secs(30));
    tokio::pin!(timeout);
    loop {
        tokio::select! {
            r = rx.recv() => match r {
                Ok(evt) => {
                    if matches!(evt, api::AgentEvent::Done { .. }) { break; }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                Err(_) => continue, // Lagged 等：继续收
            },
            _ = &mut timeout => break,
        }
    }

    let status = mgr.get_status(&sid).await.expect("会话应仍在内存中");
    assert_eq!(
        status.phase, "blocked",
        "D-208：越权读触发的 sandbox 红线必须 fail-closed 置终态 `blocked` \
         （修复前恒 `done`——红线事实被丢弃，对外呈现为正常完成）；实得 phase={}",
        status.phase
    );

    let _ = std::fs::remove_dir_all(&obs_dir);
    std::env::remove_var("CODEX_OBSERVER_DIR");
}
