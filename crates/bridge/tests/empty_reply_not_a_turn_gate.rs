//! D-209（2026-10-08, traecode）行为门禁：bridge 不得把 provider 的"空回复"当成一次有效发言。
//!
//! 病灶：`run_round_robin`/`run_debate`/`run_majority` 用 `resp.content.unwrap_or_default()`——
//! provider 返回 `None`/空白（tool-call-only、内容策略拒绝、仅 reasoning）时静默得到 `""`：
//! `run_round_robin` 把**空段**计入讨论与汇总（参与者**看似发了言**，结果串里出现空小节）；
//! `run_debate` 更用空串**覆盖 `previous`** ⇒ 后续轮次 "Previous argument:" 变空、讨论上下文被
//! **静默清空**（连锁影响后面每个参与者）。属"静默回落/声称≠实现"族。

use async_trait::async_trait;
use bridge::{BridgeSession, BridgeStrategy};
use llm_gateway::types::{Capabilities, ChatRequest, ChatResponse, Embedding, StreamEvent};
use llm_gateway::{LlmProvider, ProviderRegistry};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// 可编程假 provider：按序返回预置正文（可为 `None`），并**记录**每次收到的提示文本。
struct FakeProvider {
    name: String,
    model: String,
    replies: Mutex<VecDeque<Option<String>>>,
    prompts: Mutex<Vec<String>>,
}

impl FakeProvider {
    fn new(name: &str, replies: Vec<Option<String>>) -> Self {
        Self {
            name: name.into(),
            model: format!("{name}-model"),
            replies: Mutex::new(replies.into()),
            prompts: Mutex::new(Vec::new()),
        }
    }
    fn seen_prompts(&self) -> Vec<String> {
        self.prompts.lock().unwrap().clone()
    }
}

#[async_trait]
impl LlmProvider for FakeProvider {
    fn name(&self) -> &str {
        &self.name
    }
    fn model(&self) -> &str {
        &self.model
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            chat: true,
            stream: false,
            function_calling: false,
            embeddings: false,
            max_context_tokens: None,
        }
    }
    async fn chat(&self, req: ChatRequest) -> anyhow::Result<ChatResponse> {
        let text = req
            .messages
            .last()
            .map(|m| match &m.content {
                agent_types::MessageContent::Text(t) => t.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();
        self.prompts.lock().unwrap().push(text);
        let content = self.replies.lock().unwrap().pop_front().unwrap_or(None);
        Ok(ChatResponse {
            content,
            tool_calls: vec![],
            finish_reason: Some("stop".into()),
            usage: None,
            reasoning_content: None,
        })
    }
    fn stream(
        &self,
        _req: ChatRequest,
    ) -> futures::stream::BoxStream<'static, anyhow::Result<StreamEvent>> {
        Box::pin(futures::stream::empty())
    }
    async fn embed(&self, _inputs: &[String]) -> anyhow::Result<Vec<Embedding>> {
        Ok(vec![])
    }
}

/// 空回复**不得**伪造一次发言（round-robin：turns 不得含空 Turn、汇总不得出现空小节）。
#[tokio::test]
async fn empty_reply_is_not_a_turn() {
    let mut reg = ProviderRegistry::new();
    reg.register(Arc::new(FakeProvider::new("p1", vec![None])));
    let mut s = BridgeSession::new(
        "T".into(),
        vec!["p1".into()],
        BridgeStrategy::RoundRobin,
        Arc::new(reg),
    );
    s.run(1).await.unwrap();
    assert!(
        s.turns.is_empty(),
        "D-209：provider 空回复不得伪造一次发言（turns 不得含空 Turn）；实得 {} 个，内容 {:?}",
        s.turns.len(),
        s.turns.iter().map(|t| &t.content).collect::<Vec<_>>()
    );
}

/// 空回复**不得清空**讨论上下文（debate：后续参与者仍应看到上一**有效**发言）。
#[tokio::test]
async fn empty_reply_does_not_wipe_debate_context() {
    let mut reg = ProviderRegistry::new();
    let p1 = Arc::new(FakeProvider::new("p1", vec![Some("ARG-ONE".into())]));
    let p2 = Arc::new(FakeProvider::new("p2", vec![None]));
    let p3 = Arc::new(FakeProvider::new("p3", vec![Some("last".into())]));
    reg.register(p1);
    reg.register(p2);
    reg.register(p3.clone());
    let mut s = BridgeSession::new(
        "T".into(),
        vec!["p1".into(), "p2".into(), "p3".into()],
        BridgeStrategy::Debate,
        Arc::new(reg),
    );
    s.run(1).await.unwrap();
    let seen = p3.seen_prompts();
    assert_eq!(seen.len(), 1, "p3 应恰好被调用一次");
    assert!(
        seen[0].contains("ARG-ONE"),
        "D-209：中间方空回复不得**清空**讨论上下文——p3 仍应看到上一有效发言。实得提示: {:?}",
        seen[0]
    );
}
