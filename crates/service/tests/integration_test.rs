/// Integration test: proves A1 (SSE event stream) + A3 (dual-provider switch)
/// Uses a MockProvider with scripted responses — no real API key needed.
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use api::SessionCreate;
use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use llm_gateway::{
    Capabilities, ChatRequest, ChatResponse, Embedding, LlmProvider, ProviderRegistry, StreamEvent,
    Usage,
};
use tool_runtime::{ToolContext, ToolDispatcher};
use tools_builtin::{BashTool, EditTool, GlobTool, GrepTool, ReadTool};

// ── MockProvider with script advancement (F3: pop instead of first) ──

struct MockProvider {
    name: String,
    script: Mutex<MockScriptState>,
}

#[derive(Clone)]
struct MockScript {
    chat_responses: Vec<ChatResponse>,
}

struct MockScriptState {
    responses: Vec<ChatResponse>,
    cursor: usize,
}

impl MockScriptState {
    fn new(script: MockScript) -> Self {
        Self {
            responses: script.chat_responses,
            cursor: 0,
        }
    }
}

impl MockProvider {
    fn new(name: &str, script: MockScript) -> Self {
        Self {
            name: name.to_string(),
            script: Mutex::new(MockScriptState::new(script)),
        }
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
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
            max_context_tokens: Some(4096),
        }
    }
    async fn chat(&self, _req: ChatRequest) -> Result<ChatResponse> {
        let mut state = self.script.lock().unwrap();
        // F3: advance cursor — pop-style: return current, then advance
        if state.cursor < state.responses.len() {
            let resp = state.responses[state.cursor].clone();
            state.cursor += 1;
            Ok(resp)
        } else {
            // Script exhausted — return a natural DONE to let loop converge
            Ok(ChatResponse {
                content: Some("DONE".into()),
                tool_calls: vec![],
                finish_reason: Some("stop".into()),
                usage: None,
                reasoning_content: None,
            })
        }
    }
    fn stream(&self, _req: ChatRequest) -> BoxStream<'static, Result<StreamEvent>> {
        Box::pin(stream::empty())
    }
    async fn embed(&self, _inputs: &[String]) -> Result<Vec<Embedding>> {
        Ok(vec![])
    }
}

fn build_script() -> MockScript {
    MockScript {
        chat_responses: vec![
            // P3: planner.decompose() — LLM response for task planning
            ChatResponse {
                content: Some("[{\"id\":\"execute\",\"description\":\"run the echo command\",\"deps\":[],\"delegable\":false}]".into()),
                tool_calls: vec![],
                finish_reason: Some("stop".into()),
                usage: Some(Usage { prompt_tokens: 30, completion_tokens: 15, total_tokens: 45, prompt_cache_hit_tokens: None, prompt_cache_miss_tokens: None }),
            reasoning_content: None,
            },
            // do_plan provider.chat — LLM response with tool call
            ChatResponse {
                content: Some("I will run the command.".into()),
                tool_calls: vec![agent_types::ToolCall {
                    call_id: "c1".into(),
                    name: "bash".into(),
                    args: serde_json::json!({"cmd": "echo hello_from_mock"}),
                }],
                finish_reason: Some("tool_calls".into()),
                usage: Some(Usage { prompt_tokens: 50, completion_tokens: 20, total_tokens: 70, prompt_cache_hit_tokens: None, prompt_cache_miss_tokens: None }),
            reasoning_content: None,
            },
            // P3: planner.reflect() — LLM response for reflection
            ChatResponse {
                content: Some("continue".into()),
                tool_calls: vec![],
                finish_reason: Some("stop".into()),
                usage: Some(Usage { prompt_tokens: 20, completion_tokens: 3, total_tokens: 23, prompt_cache_hit_tokens: None, prompt_cache_miss_tokens: None }),
            reasoning_content: None,
            },
            // P3: planner.decompose() second call
            ChatResponse {
                content: Some("[{\"id\":\"execute\",\"description\":\"run the echo command\",\"deps\":[],\"delegable\":false}]".into()),
                tool_calls: vec![],
                finish_reason: Some("stop".into()),
                usage: Some(Usage { prompt_tokens: 30, completion_tokens: 15, total_tokens: 45, prompt_cache_hit_tokens: None, prompt_cache_miss_tokens: None }),
            reasoning_content: None,
            },
            // do_plan provider.chat second call — DONE
            ChatResponse {
                content: Some("DONE".into()),
                tool_calls: vec![],
                finish_reason: Some("stop".into()),
                usage: Some(Usage { prompt_tokens: 30, completion_tokens: 5, total_tokens: 35, prompt_cache_hit_tokens: None, prompt_cache_miss_tokens: None }),
            reasoning_content: None,
            },
        ],
    }
}

fn build_manager() -> (Arc<service::session::SessionManager>, Arc<ProviderRegistry>) {
    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(MockProvider::new("mock-a", build_script())), true);
    registry.register(Arc::new(MockProvider::new("mock-b", build_script())), false);
    let registry = Arc::new(registry);

    let mut dispatcher = ToolDispatcher::new();
    dispatcher.register(Arc::new(BashTool::new()));
    dispatcher.register(Arc::new(ReadTool::new()));
    dispatcher.register(Arc::new(EditTool::new()));
    dispatcher.register(Arc::new(GlobTool::new()));
    dispatcher.register(Arc::new(GrepTool::new()));
    let dispatcher = Arc::new(dispatcher);

    let ctx = ToolContext::default();
    let mgr = Arc::new(service::session::SessionManager::new(
        registry.clone(),
        dispatcher,
        ctx,
    ));
    (mgr, registry)
}

// ── A1: SSE event stream via SessionManager ──

#[tokio::test]
async fn test_a3_dual_provider_switch() {
    let (mgr, registry) = build_manager();

    // Verify registry has both providers
    let providers = registry.list();
    eprintln!("Registered providers: {:?}", providers);
    assert!(providers.contains(&"mock-a".to_string()));
    assert!(providers.contains(&"mock-b".to_string()));

    // Create session with mock-a
    let resp_a = mgr
        .create_session(SessionCreate {
            provider: "mock-a".into(),
            model: None,
            goal: "test-a".into(),
            budget: None,
        })
        .await
        .unwrap();
    eprintln!("mock-a session: {}", resp_a.session_id);

    // Create session with mock-b
    let resp_b = mgr
        .create_session(SessionCreate {
            provider: "mock-b".into(),
            model: None,
            goal: "test-b".into(),
            budget: None,
        })
        .await
        .unwrap();
    eprintln!("mock-b session: {}", resp_b.session_id);

    assert_ne!(resp_a.session_id, resp_b.session_id, "distinct sessions");

    // Both sessions accessible
    let status_a = mgr.get_status(&resp_a.session_id).await.unwrap();
    let status_b = mgr.get_status(&resp_b.session_id).await.unwrap();
    assert_eq!(status_a.session_id, resp_a.session_id);
    assert_eq!(status_b.session_id, resp_b.session_id);

    // Send messages to both — each should produce SSE events
    for (sid, label) in [
        (&resp_a.session_id, "mock-a"),
        (&resp_b.session_id, "mock-b"),
    ] {
        let mut rx = mgr
            .send_message(
                sid,
                api::MessageReq {
                    content: "test".into(),
                },
            )
            .await
            .unwrap();

        let mut got_done = false;
        let timeout = tokio::time::sleep(std::time::Duration::from_secs(10));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok(evt) => {
                            if matches!(evt, api::AgentEvent::Done { .. }) { got_done = true; break; }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    }
                }
                _ = &mut timeout => break,
            }
        }

        eprintln!("{} got done event: {}", label, got_done);
        assert!(got_done, "{} should complete with Done event", label);
    }

    eprintln!("A3 PASS: dual provider switch works");
}

// ── P1: A1 — Real tool set (read/edit/glob/grep + bash via sandbox) ──

fn build_script_tool_test() -> MockScript {
    MockScript {
        chat_responses: vec![
            // P3: planner.decompose() — LLM response for task planning
            ChatResponse {
                content: Some("[{\"id\":\"create_file\",\"description\":\"Create a test file\",\"deps\":[],\"delegable\":false}]".into()),
                tool_calls: vec![],
                finish_reason: Some("stop".into()),
                usage: Some(Usage { prompt_tokens: 30, completion_tokens: 15, total_tokens: 45, prompt_cache_hit_tokens: None, prompt_cache_miss_tokens: None }),
            reasoning_content: None,
            },
            // First: write a file
            ChatResponse {
                content: Some("I will create a test file.".into()),
                tool_calls: vec![agent_types::ToolCall {
                    call_id: "c1".into(),
                    name: "write_file".into(),
                    args: serde_json::json!({"path": "p1_test.txt", "content": "hello from p1"}),
                }],
                finish_reason: Some("tool_calls".into()),
                usage: Some(Usage { prompt_tokens: 30, completion_tokens: 10, total_tokens: 40, prompt_cache_hit_tokens: None, prompt_cache_miss_tokens: None }),
            reasoning_content: None,
            },
            // P3: planner.reflect() → continue
            ChatResponse {
                content: Some("continue".into()),
                tool_calls: vec![],
                finish_reason: Some("stop".into()),
                usage: Some(Usage { prompt_tokens: 20, completion_tokens: 3, total_tokens: 23, prompt_cache_hit_tokens: None, prompt_cache_miss_tokens: None }),
            reasoning_content: None,
            },
            // P3: planner.decompose() second call
            ChatResponse {
                content: Some("[{\"id\":\"read_file\",\"description\":\"Read back the test file\",\"deps\":[],\"delegable\":false}]".into()),
                tool_calls: vec![],
                finish_reason: Some("stop".into()),
                usage: Some(Usage { prompt_tokens: 30, completion_tokens: 15, total_tokens: 45, prompt_cache_hit_tokens: None, prompt_cache_miss_tokens: None }),
            reasoning_content: None,
            },
            // Second: read it back
            ChatResponse {
                content: Some("Now I will read it back.".into()),
                tool_calls: vec![agent_types::ToolCall {
                    call_id: "c2".into(),
                    name: "read".into(),
                    args: serde_json::json!({"path": "p1_test.txt"}),
                }],
                finish_reason: Some("tool_calls".into()),
                usage: Some(Usage { prompt_tokens: 30, completion_tokens: 10, total_tokens: 40, prompt_cache_hit_tokens: None, prompt_cache_miss_tokens: None }),
            reasoning_content: None,
            },
            // P3: planner.reflect() second call → continue
            ChatResponse {
                content: Some("continue".into()),
                tool_calls: vec![],
                finish_reason: Some("stop".into()),
                usage: Some(Usage { prompt_tokens: 20, completion_tokens: 3, total_tokens: 23, prompt_cache_hit_tokens: None, prompt_cache_miss_tokens: None }),
            reasoning_content: None,
            },
            // P3: planner.decompose() third call
            ChatResponse {
                content: Some("[{\"id\":\"done\",\"description\":\"All tasks complete\",\"deps\":[],\"delegable\":false}]".into()),
                tool_calls: vec![],
                finish_reason: Some("stop".into()),
                usage: Some(Usage { prompt_tokens: 30, completion_tokens: 15, total_tokens: 45, prompt_cache_hit_tokens: None, prompt_cache_miss_tokens: None }),
            reasoning_content: None,
            },
            // Third: DONE
            ChatResponse {
                content: Some("DONE".into()),
                tool_calls: vec![],
                finish_reason: Some("stop".into()),
                usage: Some(Usage { prompt_tokens: 20, completion_tokens: 5, total_tokens: 25, prompt_cache_hit_tokens: None, prompt_cache_miss_tokens: None }),
            reasoning_content: None,
            },
        ],
    }
}

#[tokio::test]
async fn test_p1_budget_exhausted_error() {
    let mut registry = ProviderRegistry::new();
    // Script that keeps suggesting tool calls — will exhaust budget
    // Multiple identical entries so cursor advances past budget limit
    let mut infinite_responses = Vec::new();
    for _ in 0..20 {
        infinite_responses.push(ChatResponse {
            content: Some("I will keep working.".into()),
            tool_calls: vec![agent_types::ToolCall {
                call_id: format!("cx{}", infinite_responses.len()),
                name: "bash".into(),
                args: serde_json::json!({"cmd": "echo working"}),
            }],
            finish_reason: Some("tool_calls".into()),
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
                prompt_cache_hit_tokens: None,
                prompt_cache_miss_tokens: None,
            }),
            reasoning_content: None,
        });
    }
    let infinite_script = MockScript {
        chat_responses: infinite_responses,
    };
    registry.register(
        Arc::new(MockProvider::new("mock-budget", infinite_script)),
        true,
    );
    let registry = Arc::new(registry);

    let mut dispatcher = ToolDispatcher::new();
    dispatcher.register(Arc::new(BashTool::new()));
    dispatcher.register(Arc::new(ReadTool::new()));
    dispatcher.register(Arc::new(EditTool::new()));
    let dispatcher = Arc::new(dispatcher);

    let mgr = Arc::new(service::session::SessionManager::new(
        registry,
        dispatcher,
        ToolContext::default(),
    ));

    let resp = mgr
        .create_session(SessionCreate {
            provider: "mock-budget".into(),
            model: None,
            goal: "run forever".into(),
            // Very tight budget — only 3 steps
            budget: Some(agent_types::Budget {
                max_steps: 5,
                max_tokens: None,
                max_time_secs: None,
                ..Default::default()
            }),
        })
        .await
        .unwrap();

    let mut rx = mgr
        .send_message(
            &resp.session_id,
            api::MessageReq {
                content: "go".into(),
            },
        )
        .await
        .unwrap();

    let mut events: Vec<api::AgentEvent> = Vec::new();
    let timeout = tokio::time::sleep(std::time::Duration::from_secs(10));
    tokio::pin!(timeout);
    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(evt) => {
                        let is_done = matches!(evt, api::AgentEvent::Done { .. });
                        events.push(evt);
                        if is_done { break; }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                }
            }
            _ = &mut timeout => break,
        }
    }

    // Should have an error event about budget
    let has_error = events
        .iter()
        .any(|e| matches!(e, api::AgentEvent::Error { .. }));
    eprintln!(
        "Budget test events: {} total, has_error={}",
        events.len(),
        has_error
    );
    assert!(has_error, "budget exhaustion should produce Error event");

    // The Done event should indicate failure, not "completed"
    let done_event = events
        .iter()
        .find(|e| matches!(e, api::AgentEvent::Done { .. }));
    assert!(done_event.is_some(), "should have Done event");
    if let Some(api::AgentEvent::Done { report }) = done_event {
        let json = serde_json::to_string(report).unwrap();
        eprintln!("Done report: {}", json);
        // F2: budget exhaustion should NOT say "completed"
        assert!(
            !json.contains("\"completed\""),
            "budget exhaustion must not claim 'completed'"
        );
    }

    // Session should be in error state
    let status = mgr.get_status(&resp.session_id).await.unwrap();
    eprintln!("Final session phase: {}", status.phase);

    eprintln!("P1-A5 PASS: budget exhaustion produces error termination");
}

// ── P1: A5 — Natural done (ok:true) ──

#[tokio::test]
async fn test_p1_natural_done() {
    let (mgr, _) = build_manager();

    // W8/A1: goal 用论述信号措辞——无信号输入默认 product（EC-03 安全方向），
    // product 交卷需真实 write；本测试验证 natural done，走 QA 直答路径。
    let resp = mgr
        .create_session(SessionCreate {
            provider: "mock-a".into(),
            model: None,
            goal: "explain echo hello".into(),
            budget: Some(agent_types::Budget {
                max_steps: 10,
                max_tokens: None,
                max_time_secs: None,
                ..Default::default()
            }),
        })
        .await
        .unwrap();

    let mut rx = mgr
        .send_message(
            &resp.session_id,
            api::MessageReq {
                content: "echo".into(),
            },
        )
        .await
        .unwrap();

    let mut events: Vec<api::AgentEvent> = Vec::new();
    let timeout = tokio::time::sleep(std::time::Duration::from_secs(10));
    tokio::pin!(timeout);
    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(evt) => {
                        let is_done = matches!(evt, api::AgentEvent::Done { .. });
                        events.push(evt);
                        if is_done { break; }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                }
            }
            _ = &mut timeout => break,
        }
    }

    // F3: natural done — the Done event should come from agent loop, not synthesized
    let done_event = events
        .iter()
        .find(|e| matches!(e, api::AgentEvent::Done { .. }));
    assert!(done_event.is_some(), "should have Done event");
    if let Some(api::AgentEvent::Done { report }) = done_event {
        let json = serde_json::to_string(report).unwrap();
        eprintln!("Natural Done report: {}", json);
        // F2: natural completion should have ok:true
        assert!(
            json.contains("\"ok\":true"),
            "natural done should have ok:true"
        );
    }

    eprintln!("P1-A5 PASS: natural done ok:true");
}

// ── P1: F1 proof — Multi-turn user messages enter LLM context ──

#[tokio::test]
async fn test_p1_f1_multi_turn_user_message_in_context() {
    // This test proves F1: user message content from send_message reaches build_messages()
    // by checking that the MockProvider's chat() receives it in the ChatRequest.
    use std::sync::Mutex as StdMutex;

    struct SpyingProvider {
        inner: MockProvider,
        last_messages: StdMutex<Vec<String>>,
    }

    #[async_trait]
    impl LlmProvider for SpyingProvider {
        fn name(&self) -> &str {
            self.inner.name()
        }
        fn model(&self) -> &str {
            self.inner.model()
        }
        fn capabilities(&self) -> Capabilities {
            self.inner.capabilities()
        }
        async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
            // Spy: capture all message content
            let contents: Vec<String> = req
                .messages
                .iter()
                .map(|m| format!("{:?}", m.content))
                .collect();
            {
                let mut lm = self.last_messages.lock().unwrap();
                *lm = contents.clone();
            }
            // Delegate to inner
            self.inner.chat(req).await
        }
        fn stream(&self, req: ChatRequest) -> BoxStream<'static, Result<StreamEvent>> {
            self.inner.stream(req)
        }
        async fn embed(&self, inputs: &[String]) -> Result<Vec<Embedding>> {
            self.inner.embed(inputs).await
        }
    }

    let _last_messages = Arc::new(StdMutex::new(Vec::<String>::new()));

    let spy = Arc::new(SpyingProvider {
        inner: MockProvider::new("spy", build_script()),
        last_messages: StdMutex::new(Vec::new()),
    });

    let mut registry = ProviderRegistry::new();
    registry.register(spy.clone(), true);
    let registry = Arc::new(registry);

    let mut dispatcher = ToolDispatcher::new();
    dispatcher.register(Arc::new(BashTool::new()));
    dispatcher.register(Arc::new(ReadTool::new()));
    dispatcher.register(Arc::new(EditTool::new()));
    let dispatcher = Arc::new(dispatcher);

    let mgr = Arc::new(service::session::SessionManager::new(
        registry,
        dispatcher,
        ToolContext::default(),
    ));

    let resp = mgr
        .create_session(SessionCreate {
            provider: "spy".into(),
            model: None,
            goal: "echo hello".into(),
            budget: Some(agent_types::Budget {
                max_steps: 10,
                max_tokens: None,
                max_time_secs: None,
                ..Default::default()
            }),
        })
        .await
        .unwrap();

    // Send message with unique content that we can grep for
    let unique_msg = "F1_PROOF_MARKER_42";
    let mut rx = mgr
        .send_message(
            &resp.session_id,
            api::MessageReq {
                content: unique_msg.into(),
            },
        )
        .await
        .unwrap();

    // Drain events
    let timeout = tokio::time::sleep(std::time::Duration::from_secs(10));
    tokio::pin!(timeout);
    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(evt) => {
                        if matches!(evt, api::AgentEvent::Done { .. }) { break; }
                    }
                    Err(_) => break,
                }
            }
            _ = &mut timeout => break,
        }
    }

    // Check that the spy captured the user message
    let captured = spy.last_messages.lock().unwrap();
    let all_text: String = captured.join(" ||| ");
    eprintln!("F1 spy captured messages: {}", all_text);
    assert!(
        all_text.contains(unique_msg),
        "F1 FAIL: user message '{}' not found in ChatRequest messages. Captured: {:?}",
        unique_msg,
        captured
    );

    eprintln!("P1-F1 PASS: user message reaches LLM context");
}

// ── P2: A4 — LSP diagnostics flow through observe → SSE ──

#[tokio::test]
async fn test_p4_three_backend_switch() {
    use async_trait::async_trait;
    use futures::stream::{self, BoxStream};
    use llm_gateway::{
        Capabilities, ChatRequest, ChatResponse, Embedding, LlmProvider, StreamEvent,
    };
    use std::sync::Arc as StdArc;
    use tokio::sync::Mutex as TokioMutex;

    // ── Hit-tracked mock provider wrapping a script ──
    struct HitMock {
        name: String,
        model: String,
        hit: StdArc<TokioMutex<bool>>,
        script: TokioMutex<Vec<ChatResponse>>,
    }

    impl HitMock {
        fn new(
            name: &str,
            model: &str,
            hit: StdArc<TokioMutex<bool>>,
            script: Vec<ChatResponse>,
        ) -> Self {
            Self {
                name: name.into(),
                model: model.into(),
                hit,
                script: TokioMutex::new(script),
            }
        }
    }

    #[async_trait]
    impl LlmProvider for HitMock {
        fn name(&self) -> &str {
            &self.name
        }
        fn model(&self) -> &str {
            &self.model
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities {
                chat: true,
                stream: true,
                function_calling: true,
                embeddings: false,
                max_context_tokens: Some(4096),
            }
        }
        async fn chat(&self, _req: ChatRequest) -> anyhow::Result<ChatResponse> {
            *self.hit.lock().await = true;
            let mut s = self.script.lock().await;
            if let Some(resp) = s.pop() {
                Ok(resp)
            } else {
                // Script exhausted → return DONE to converge
                Ok(ChatResponse {
                    content: Some("DONE".into()),
                    tool_calls: vec![],
                    finish_reason: Some("stop".into()),
                    usage: None,
                    reasoning_content: None,
                })
            }
        }
        fn stream(&self, _req: ChatRequest) -> BoxStream<'static, anyhow::Result<StreamEvent>> {
            Box::pin(stream::empty())
        }
        async fn embed(&self, _inputs: &[String]) -> anyhow::Result<Vec<Embedding>> {
            Ok(vec![])
        }
    }

    // ── Shared hit flags ──
    let ollama_hit = StdArc::new(TokioMutex::new(false));
    let vllm_hit = StdArc::new(TokioMutex::new(false));
    let hunyuan_hit = StdArc::new(TokioMutex::new(false));

    // ── Script: drives agent loop Plan→Act→Observe→Reflect→Done ──
    // Agent loop calls: decompose(LLM) → chat(LLM with tool_call) → reflect(LLM) → decompose(LLM) → chat(LLM→DONE)
    fn build_converge_script() -> Vec<ChatResponse> {
        vec![
            // 1. decompose: return a task
            ChatResponse {
                content: Some(
                    "[{\"id\":\"t1\",\"description\":\"echo\",\"deps\":[],\"delegable\":false}]"
                        .into(),
                ),
                tool_calls: vec![],
                finish_reason: Some("stop".into()),
                usage: None,
                reasoning_content: None,
            },
            // 2. chat with tool_call: do the work
            ChatResponse {
                content: Some("I'll run echo.".into()),
                tool_calls: vec![agent_types::ToolCall {
                    call_id: "c1".into(),
                    name: "bash".into(),
                    args: serde_json::json!({"cmd": "echo p4_test"}),
                }],
                finish_reason: Some("tool_calls".into()),
                usage: None,
                reasoning_content: None,
            },
            // 3. reflect: continue
            ChatResponse {
                content: Some("continue".into()),
                tool_calls: vec![],
                finish_reason: Some("stop".into()),
                usage: None,
                reasoning_content: None,
            },
            // 4. decompose again
            ChatResponse {
                content: Some(
                    "[{\"id\":\"t2\",\"description\":\"done\",\"deps\":[],\"delegable\":false}]"
                        .into(),
                ),
                tool_calls: vec![],
                finish_reason: Some("stop".into()),
                usage: None,
                reasoning_content: None,
            },
            // 5. chat → DONE
            ChatResponse {
                content: Some("DONE".into()),
                tool_calls: vec![],
                finish_reason: Some("stop".into()),
                usage: None,
                reasoning_content: None,
            },
            // 6. reflect → continue (will be followed by script-exhausted DONE)
            ChatResponse {
                content: Some("continue".into()),
                tool_calls: vec![],
                finish_reason: Some("stop".into()),
                usage: None,
                reasoning_content: None,
            },
        ]
    }

    // Build three HitMock providers (reverse order since we pop)
    let script = build_converge_script();
    let ollama = Arc::new(HitMock::new("ollama", "llama3.2", ollama_hit.clone(), {
        let mut s = script.clone();
        s.reverse();
        s
    }));
    let vllm = Arc::new(HitMock::new("vllm", "mistral-7b", vllm_hit.clone(), {
        let mut s = script.clone();
        s.reverse();
        s
    }));
    let hunyuan = Arc::new(HitMock::new(
        "hunyuan",
        "hunyuan-pro",
        hunyuan_hit.clone(),
        {
            let mut s = script.clone();
            s.reverse();
            s
        },
    ));

    // ── Register all three providers ──
    let mut registry = ProviderRegistry::new();
    registry.register(ollama, true);
    registry.register(vllm, false);
    registry.register(hunyuan, false);
    let registry = Arc::new(registry);

    let mut dispatcher = ToolDispatcher::new();
    dispatcher.register(Arc::new(BashTool::new()));
    dispatcher.register(Arc::new(ReadTool::new()));
    dispatcher.register(Arc::new(EditTool::new()));
    dispatcher.register(Arc::new(GlobTool::new()));
    dispatcher.register(Arc::new(GrepTool::new()));
    let dispatcher = Arc::new(dispatcher);

    let mgr = Arc::new(service::session::SessionManager::new(
        registry.clone(),
        dispatcher,
        ToolContext::default(),
    ));

    let budget = agent_types::Budget {
        max_steps: 10,
        max_tokens: None,
        max_time_secs: None,
        ..Default::default()
    };

    // ── Run all three sessions ──
    // W8/A1: goal 加论述信号（explain）——保持 QA 直答 natural done 路径
    let backends = [
        ("ollama", "explain echo via ollama"),
        ("vllm", "explain echo via vllm"),
        ("hunyuan", "explain echo via hunyuan"),
    ];

    for (provider_name, goal) in &backends {
        eprintln!("\n=== P4-A3: testing backend '{provider_name}' ===");

        let resp = mgr
            .create_session(SessionCreate {
                provider: provider_name.to_string(),
                model: None,
                goal: goal.to_string(),
                budget: Some(budget.clone()),
            })
            .await
            .unwrap();
        eprintln!("  session_id: {}", resp.session_id);

        let mut rx = mgr
            .send_message(
                &resp.session_id,
                api::MessageReq {
                    content: "echo hello".into(),
                },
            )
            .await
            .unwrap();

        let mut events: Vec<api::AgentEvent> = Vec::new();
        let mut got_done = false;
        let timeout = tokio::time::sleep(std::time::Duration::from_secs(15));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok(evt) => {
                            let is_done = matches!(evt, api::AgentEvent::Done { .. });
                            let json = serde_json::to_string(&evt).unwrap_or_default();
                            // char-safe 截断（中文 JSON 字节切片会 panic）
                            let preview = if json.chars().count() > 150 {
                                format!("{}...", json.chars().take(147).collect::<String>())
                            } else {
                                json
                            };
                            eprintln!("    event: {preview}");
                            events.push(evt);
                            if is_done { got_done = true; break; }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    }
                }
                _ = &mut timeout => break,
            }
        }

        // A3 assertion: every backend must reach Done
        assert!(
            got_done,
            "P4-A3 FAIL: backend '{provider_name}' did not reach Done"
        );

        // Verify the Done report has ok:true
        if let Some(api::AgentEvent::Done { report }) = events
            .iter()
            .find(|e| matches!(e, api::AgentEvent::Done { .. }))
        {
            let json = serde_json::to_string(report).unwrap();
            assert!(
                json.contains("\"ok\":true"),
                "backend '{provider_name}' Done should have ok:true, got: {json}"
            );
        }

        eprintln!(
            "  backend '{provider_name}' → Done with {} events",
            events.len()
        );
    }

    // ── A3 core assertion: every mock was actually hit ──
    assert!(
        *ollama_hit.lock().await,
        "P4-A3 FAIL: Ollama mock was NEVER called"
    );
    assert!(
        *vllm_hit.lock().await,
        "P4-A3 FAIL: vLLM mock was NEVER called"
    );
    assert!(
        *hunyuan_hit.lock().await,
        "P4-A3 FAIL: Hunyuan mock was NEVER called"
    );

    eprintln!("\nP4-A3 PASS: three-backend switch — all 3 sessions reached Done, all 3 providers received real calls");
}

// ────────────────────────────────────────────────────────────────────────────
// P5: A3② — FallbackChain in request path
//
// Primary mock fails → FallbackChain auto-switches to secondary mock.
// Verifies: primary receives request and fails, secondary receives request
// and succeeds, session reaches Done.
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_p5_fallback_in_request_path() {
    use std::sync::Arc as StdArc;
    use tokio::sync::Mutex as TokioMutex;

    // Hit-tracked mock: records whether it was called
    struct HitMock {
        name: String,
        should_fail: bool,
        hit: StdArc<TokioMutex<bool>>,
    }

    #[async_trait]
    impl LlmProvider for HitMock {
        fn name(&self) -> &str {
            &self.name
        }
        fn model(&self) -> &str {
            "mock"
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities {
                chat: true,
                stream: false,
                function_calling: true,
                embeddings: false,
                max_context_tokens: Some(4096),
            }
        }
        async fn chat(&self, _req: ChatRequest) -> Result<ChatResponse> {
            *self.hit.lock().await = true;
            if self.should_fail {
                Err(anyhow::anyhow!("{} intentionally failed", self.name))
            } else {
                Ok(ChatResponse {
                    content: Some("DONE".into()),
                    tool_calls: vec![],
                    finish_reason: Some("stop".into()),
                    usage: None,
                    reasoning_content: None,
                })
            }
        }
        fn stream(&self, _req: ChatRequest) -> BoxStream<'static, Result<StreamEvent>> {
            Box::pin(stream::empty())
        }
        async fn embed(&self, _inputs: &[String]) -> Result<Vec<Embedding>> {
            Ok(vec![])
        }
    }

    let primary_hit = StdArc::new(TokioMutex::new(false));
    let secondary_hit = StdArc::new(TokioMutex::new(false));

    let primary = Arc::new(HitMock {
        name: "primary".into(),
        should_fail: true,
        hit: primary_hit.clone(),
    });
    let secondary = Arc::new(HitMock {
        name: "secondary".into(),
        should_fail: false,
        hit: secondary_hit.clone(),
    });

    // Build FallbackChain and register it
    let chain = Arc::new(llm_gateway::FallbackChain::new(vec![
        ("primary".into(), primary.clone() as Arc<dyn LlmProvider>),
        (
            "secondary".into(),
            secondary.clone() as Arc<dyn LlmProvider>,
        ),
    ]));

    let mut registry = ProviderRegistry::new();
    registry.register(chain, true);
    let registry = Arc::new(registry);

    let mut dispatcher = ToolDispatcher::new();
    dispatcher.register(Arc::new(BashTool::new()));
    dispatcher.register(Arc::new(ReadTool::new()));
    dispatcher.register(Arc::new(EditTool::new()));
    dispatcher.register(Arc::new(GlobTool::new()));
    dispatcher.register(Arc::new(GrepTool::new()));
    let dispatcher = Arc::new(dispatcher);

    let mgr = Arc::new(service::session::SessionManager::new(
        registry,
        dispatcher,
        ToolContext::default(),
    ));

    let resp = mgr
        .create_session(SessionCreate {
            provider: "fallback".into(),
            model: None,
            goal: "test fallback".into(),
            budget: Some(agent_types::Budget {
                max_steps: 5,
                max_tokens: None,
                max_time_secs: None,
                ..Default::default()
            }),
        })
        .await
        .unwrap();

    let mut rx = mgr
        .send_message(
            &resp.session_id,
            api::MessageReq {
                content: "test".into(),
            },
        )
        .await
        .unwrap();

    let mut got_done = false;
    let timeout = tokio::time::sleep(std::time::Duration::from_secs(10));
    tokio::pin!(timeout);
    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(evt) => {
                        if matches!(evt, api::AgentEvent::Done { .. }) { got_done = true; break; }
                    }
                    Err(_) => break,
                }
            }
            _ = &mut timeout => break,
        }
    }

    // Hard assertions
    assert!(
        *primary_hit.lock().await,
        "P5 FAIL: primary mock was NEVER called"
    );
    assert!(
        *secondary_hit.lock().await,
        "P5 FAIL: secondary mock was NEVER called (fallback not triggered)"
    );
    assert!(
        got_done,
        "P5 FAIL: session did not reach Done via fallback chain"
    );

    eprintln!(
        "P5 A3② PASS: fallback chain — primary failed, secondary succeeded, session reached Done"
    );
}

// ────────────────────────────────────────────────────────────────────────────
// P5: A2 — Session survives restart (MemoryStore persistence)
//
// Creates a session, runs it to completion, then verifies the session record
// can be loaded from JsonlMemoryStore (simulating a service restart).
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_p5_session_survives_restart() {
    use memory::{JsonlMemoryStore, MemoryStore};

    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(JsonlMemoryStore::new(tmp.path()));
    assert!(
        store.is_available(),
        "P5 A2: memory store should be available"
    );

    // Build SessionManager with memory store
    let (_mgr, _registry) = build_manager();
    // We need a mutable SessionManager to set_memory_store
    // rebuild with memory store injected
    let mut registry = ProviderRegistry::new();
    registry.register(
        Arc::new(MockProvider::new("mock-mem", build_script())),
        true,
    );
    let registry = Arc::new(registry);

    let mut dispatcher = ToolDispatcher::new();
    dispatcher.register(Arc::new(BashTool::new()));
    dispatcher.register(Arc::new(ReadTool::new()));
    dispatcher.register(Arc::new(EditTool::new()));
    dispatcher.register(Arc::new(GlobTool::new()));
    dispatcher.register(Arc::new(GrepTool::new()));
    let dispatcher = Arc::new(dispatcher);

    let mut mgr =
        service::session::SessionManager::new(registry, dispatcher, ToolContext::default());
    mgr.set_memory_store(store.clone());
    let mgr = Arc::new(mgr);

    // Create and run a session
    let resp = mgr
        .create_session(SessionCreate {
            provider: "mock-mem".into(),
            model: None,
            goal: "explain persist test".into(),
            budget: Some(agent_types::Budget {
                max_steps: 10,
                max_tokens: None,
                max_time_secs: None,
                ..Default::default()
            }),
        })
        .await
        .unwrap();
    let session_id = resp.session_id.clone();
    eprintln!("P5 A2: created session {}", session_id);

    let mut rx = mgr
        .send_message(
            &session_id,
            api::MessageReq {
                content: "test".into(),
            },
        )
        .await
        .unwrap();

    // Drain events
    let mut got_done = false;
    let timeout = tokio::time::sleep(std::time::Duration::from_secs(10));
    tokio::pin!(timeout);
    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(evt) => {
                        if matches!(evt, api::AgentEvent::Done { .. }) {
                            got_done = true;
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            _ = &mut timeout => break,
        }
    }
    assert!(got_done, "P5 A2: session should reach Done");

    // Wait a moment for async persistence to complete
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // P5 A2: Verify session was persisted to MemoryStore
    let sessions = store.list_sessions().await.unwrap();
    eprintln!("P5 A2: persisted sessions: {:?}", sessions);
    assert!(
        sessions.contains(&session_id),
        "P5 A2 FAIL: session {} not found in MemoryStore. Sessions: {:?}",
        session_id,
        sessions
    );

    // P5 A2: Load the persisted session and verify its fields
    let loaded = store.load_session(&session_id).await.unwrap();
    assert!(
        loaded.is_some(),
        "P5 A2 FAIL: load_session returned None for {}",
        session_id
    );
    let record = loaded.unwrap();
    assert_eq!(
        record.session_id, session_id,
        "P5 A2: persisted session_id mismatch"
    );
    assert_eq!(
        record.provider_name, "mock-mem",
        "P5 A2: persisted provider_name mismatch"
    );
    assert_eq!(
        record.goal, "explain persist test",
        "P5 A2: persisted goal mismatch"
    );
    assert!(
        !record.events.is_empty(),
        "P5 A2: persisted session should have at least one event (done)"
    );
    let done_event = record
        .events
        .iter()
        .find(|e| e.event_type == "done")
        .expect("P5 A2: persisted session should contain a 'done' event");
    let payload = &done_event.payload;
    assert_eq!(
        payload["ok"], true,
        "P5 A2: persisted 'done' event should have ok=true, got: {}",
        payload
    );

    eprintln!(
        "P5 A2 PASS: session {} survived restart — loaded from MemoryStore with {} events",
        session_id,
        record.events.len()
    );
}

// ────────────────────────────────────────────────────────────────────────────
// P5: A3① — CostMeter records real usage from ChatResponse.usage
//
// Runs a session through SessionManager with a script that carries explicit
// Usage values, then checks that the CostMeter accumulated those tokens.
// ────────────────────────────────────────────────────────────────────────────

// ── EPIC-B (audit-breakdown-v22 B1a): /readyz real liveness probe ──

/// MemoryStore mock that always fails — for the unhealthy readyz path.
struct FailingMemoryStore;

#[async_trait]
impl memory::MemoryStore for FailingMemoryStore {
    async fn save_session(&self, _r: &memory::SessionRecord) -> Result<()> {
        Ok(())
    }
    async fn load_session(&self, _id: &str) -> Result<Option<memory::SessionRecord>> {
        Ok(None)
    }
    async fn list_sessions(&self) -> Result<Vec<String>> {
        Err(anyhow::anyhow!("store unavailable"))
    }
    async fn delete_session(&self, _id: &str) -> Result<()> {
        Ok(())
    }
    async fn append_events(&self, _id: &str, _e: &[memory::StoredEvent]) -> Result<()> {
        Ok(())
    }
    fn is_available(&self) -> bool {
        false
    }
}

/// EPIC-B probe semantics: readyz maps list_persisted_sessions Ok -> 200,
/// Err -> 503. We verify the probe's dependency path directly (SessionManager
/// store access), since AppState carries 13 fields only built by main.rs.

#[tokio::test]
async fn test_epic_b_readyz_probe_semantics() {
    // P1-2 (audit-fix) 语义更新：
    // ① 无 memory store → list_sessions **Err**（503）——「没有存储」≠「就绪」，
    //    旧测试把「无 store → Ok」当 healthy 写进预期（审计 RT1/P1-2：测试把弱点写成预期）。
    let healthy = Arc::new(service::session::SessionManager::new(
        Arc::new(ProviderRegistry::new()),
        Arc::new(ToolDispatcher::new()),
        ToolContext::default(),
    ));
    let r_none = healthy.list_persisted_sessions().await;
    assert!(r_none.is_err(), "no store should be Err (503), got Ok");
    eprintln!("P1-2 PASS: no store -> readyz 503 path (list Err)");

    // ② 真实 JsonlMemoryStore 指向可读临时目录 → Ok（200）
    let tmp = std::env::temp_dir().join(format!("wf-readyz-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("create tmp memory dir");
    let real_store = Arc::new(memory::JsonlMemoryStore::new(tmp.clone()));
    let mut ok = service::session::SessionManager::new(
        Arc::new(ProviderRegistry::new()),
        Arc::new(ToolDispatcher::new()),
        ToolContext::default(),
    );
    ok.set_memory_store(real_store);
    let r_ok = ok.list_persisted_sessions().await;
    assert!(r_ok.is_ok(), "real readable store should be Ok");
    eprintln!("EPIC-B PASS: real store -> readyz 200 path (list Ok)");

    // ③ 真实 JsonlMemoryStore 指向**不可读**目录 → Err（503，任务书验收：用真实 store 非 mock）
    let bad_dir = tmp.join("chmod-000-sub");
    std::fs::create_dir_all(&bad_dir).expect("create bad dir");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bad_dir, std::fs::Permissions::from_mode(0o000))
            .expect("chmod 000");
    }
    let bad_store = Arc::new(memory::JsonlMemoryStore::new(bad_dir.clone()));
    let mut bad = service::session::SessionManager::new(
        Arc::new(ProviderRegistry::new()),
        Arc::new(ToolDispatcher::new()),
        ToolContext::default(),
    );
    bad.set_memory_store(bad_store);
    let r_err = bad.list_persisted_sessions().await;
    #[cfg(unix)]
    assert!(
        r_err.is_err(),
        "unreadable dir should be Err (503), got {r_err:?}"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&bad_dir, std::fs::Permissions::from_mode(0o755));
    }
    eprintln!("EPIC-B PASS: unreadable real store -> readyz 503 path (list Err)");

    // ④ FailingStore mock → Err（保留原覆盖）
    let mut bad2 = service::session::SessionManager::new(
        Arc::new(ProviderRegistry::new()),
        Arc::new(ToolDispatcher::new()),
        ToolContext::default(),
    );
    bad2.set_memory_store(Arc::new(FailingMemoryStore));
    let r_err2 = bad2.list_persisted_sessions().await;
    assert!(r_err2.is_err(), "failing store should be Err");
    eprintln!("EPIC-B PASS: failing store -> readyz 503 path (list Err)");
}

/// P1-4 gate (acceptance-gatekeeper-v22 §四-1): civ 写失败计数 >5 → readyz 降级。
/// 守门员点名 P1-4 缺动态红绿——这里把「降级阈值」做成动态断言：
/// 0/5 → 健康（不降级），6 → 降级（503）。判定逻辑独立成 pub 函数便于测试。
#[test]
fn test_p14_readyz_civ_degraded_threshold() {
    use service::routes::civ_store_degraded;
    assert!(!civ_store_degraded(0), "0 failures -> healthy");
    assert!(
        !civ_store_degraded(5),
        "5 failures -> threshold boundary healthy"
    );
    assert!(civ_store_degraded(6), "6 failures -> degraded (readyz 503)");
    assert!(civ_store_degraded(100), "100 failures -> degraded");
    eprintln!("P1-4 PASS: civ failure threshold flips at >5 (readyz 503)");
}
