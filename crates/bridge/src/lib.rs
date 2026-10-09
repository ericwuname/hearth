//! Bridge crate — multi-model real-time discussion and consensus.
//! 6B v6.0: BridgeSession manages peer discussion across multiple LLM providers.

use agent_types::{Message, MessageContent, MessageMeta, Role};
use chrono::Utc;
use llm_gateway::types::ChatRequest;
use llm_gateway::ProviderRegistry;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// D-103（2026-10-02, traecode）：把一条投票回复判为赞成 / 反对 / 无法判定。
///
/// 病灶：原实现只做 `content.to_lowercase().contains("agree")`——而 **"disagree"
/// 里就含 "agree"**，于是每一张反对票都被计成**赞成票**，多数票结论可能整个反向。
///
/// 判据：**先看否定词，再退回肯定词**；两者都没有 ⇒ `None`——调用方按"保守计反对 +
/// warn 留痕"处理，不假装读懂了模型的回复（也不静默吞掉）。
fn classify_vote(content: &str) -> Option<bool> {
    let l = content.to_lowercase();
    if l.contains("disagree") {
        Some(false)
    } else if l.contains("agree") {
        Some(true)
    } else {
        None
    }
}

/// Helper: build a system/user message with text content.
fn msg(role: Role, text: &str) -> Message {
    Message {
        id: Uuid::new_v4().to_string(),
        role,
        content: MessageContent::Text(text.to_string()),
        created_at: Utc::now(),
        meta: MessageMeta::default(),
        reasoning_content: None,
    }
}

/// A single turn from one model in a bridge discussion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub index: u32,
    pub provider_key: String,
    pub model: String,
    pub content: String,
}

/// Discussion strategy.
///
/// D-181（2026-10-08, traecode）：原第四变体 `SingleSpeaker { speaker }` **已退役**——它
/// **全仓零构造方**（HTTP 层 `routes::create_bridge` 只产 3 种；`create_session` 亦不构造），
/// `run_single` 因而**不可达**；且"单模型发言"**不是多模型讨论**（业界多智能体协作模式＝
/// debate / voting / expert-panel / hierarchical / round-robin，无 "single"；单模型诉求走
/// `/api/v1/sessions` 即可）⇒ 按 D-78/D-111 纪律删除（"只有定义、无任何构造方" = 漂移陷阱）。
/// 防回潮：`crates/bridge/tests/retired_single_speaker_gate.rs`（源码级 pin + 在用策略反向对照）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BridgeStrategy {
    RoundRobin,
    Debate,
    MajorityVote,
}

#[allow(clippy::derivable_impls)]
impl Default for BridgeStrategy {
    fn default() -> Self {
        BridgeStrategy::RoundRobin
    }
}

/// A bridge session — multi-model peer discussion.
pub struct BridgeSession {
    pub id: String,
    pub topic: String,
    pub strategy: BridgeStrategy,
    pub turns: Vec<Turn>,
    pub participants: Vec<String>,
    pub registry: Arc<ProviderRegistry>,
}

impl BridgeSession {
    pub fn new(
        topic: String,
        participants: Vec<String>,
        strategy: BridgeStrategy,
        registry: Arc<ProviderRegistry>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            topic,
            strategy,
            turns: Vec::new(),
            participants,
            registry,
        }
    }

    pub async fn run(&mut self, max_rounds: u32) -> anyhow::Result<String> {
        match &self.strategy {
            BridgeStrategy::RoundRobin => self.run_round_robin(max_rounds).await,
            BridgeStrategy::MajorityVote => self.run_majority(max_rounds).await,
            BridgeStrategy::Debate => self.run_debate(max_rounds).await,
        }
    }

    async fn run_round_robin(&mut self, max_rounds: u32) -> anyhow::Result<String> {
        let mut summaries = Vec::new();
        for _round in 0..max_rounds {
            for participant in &self.participants.clone() {
                let provider = self.registry.get(participant)?;
                let context = self.build_context(participant);
                let req = ChatRequest {
                    messages: vec![msg(
                        Role::System,
                        &format!(
                            "You are {participant} in a round-robin discussion.\nTopic: {topic}\nContext: {context}\nRespond with your thoughts.",
                            topic = self.topic
                        ),
                    )],
                    tools: vec![],
                    temperature: Some(0.7),
                    max_tokens: Some(1024),
                    stream: false,
                };
                let resp = provider.chat(req).await?;
                let content = resp.content.unwrap_or_default();
                let turn = Turn {
                    index: self.turns.len() as u32,
                    provider_key: participant.clone(),
                    model: provider.model().to_string(),
                    content: content.clone(),
                };
                self.turns.push(turn);
                summaries.push(content);
            }
        }
        let result = format!(
            "Bridge round-robin complete ({} turns).\n{}",
            self.turns.len(),
            summaries.join("\n\n---\n\n")
        );
        Ok(result)
    }

    async fn run_majority(&mut self, max_rounds: u32) -> anyhow::Result<String> {
        for round in 0..max_rounds {
            let mut votes_for = 0;
            let mut votes_against = 0;
            for participant in &self.participants.clone() {
                let provider = self.registry.get(participant)?;
                let req = ChatRequest {
                    messages: vec![msg(
                        Role::System,
                        &format!(
                            "Topic: {topic}\nCast your vote (agree/disagree with reason).",
                            topic = self.topic
                        ),
                    )],
                    tools: vec![],
                    temperature: Some(0.5),
                    max_tokens: Some(512),
                    stream: false,
                };
                let resp = provider.chat(req).await?;
                let content = resp.content.unwrap_or_default();
                // D-103：原先单条件 `contains("agree")` 把 "disagree" 也算赞成
                //（子串包含），多数票方向可能整个反过来；现按 classify_vote 判定，
                // 无法判定者**保守计反对并留痕**（不静默、也不假装读懂了）。
                match classify_vote(&content) {
                    Some(true) => votes_for += 1,
                    Some(false) => votes_against += 1,
                    None => {
                        tracing::warn!(
                            provider = %participant,
                            "majority_vote: 回复既无 agree 也无 disagree，计为反对（不静默）"
                        );
                        votes_against += 1;
                    }
                }
                let turn = Turn {
                    index: self.turns.len() as u32,
                    provider_key: participant.clone(),
                    model: provider.model().to_string(),
                    content,
                };
                self.turns.push(turn);
            }
            if votes_for > votes_against || round >= max_rounds - 1 {
                let result = format!("Majority: {votes_for} agree, {votes_against} disagree");
                return Ok(result);
            }
        }
        Ok("No consensus reached".into())
    }

    async fn run_debate(&mut self, max_rounds: u32) -> anyhow::Result<String> {
        let mut previous = format!("Topic: {}", self.topic);
        for _round in 0..max_rounds {
            for participant in &self.participants.clone() {
                let provider = self.registry.get(participant)?;
                let req = ChatRequest {
                    messages: vec![msg(
                        Role::System,
                        &format!("Previous argument:\n{previous}\n\nRespond as {participant}."),
                    )],
                    tools: vec![],
                    temperature: Some(0.8),
                    max_tokens: Some(1024),
                    stream: false,
                };
                let resp = provider.chat(req).await?;
                let content = resp.content.unwrap_or_default();
                previous = content.clone();
                let turn = Turn {
                    index: self.turns.len() as u32,
                    provider_key: participant.clone(),
                    model: provider.model().to_string(),
                    content,
                };
                self.turns.push(turn);
            }
        }
        let result = format!("Debate complete — {} turns.", self.turns.len());
        Ok(result)
    }

    fn build_context(&self, current: &str) -> String {
        let mut ctx = String::new();
        for t in self.turns.iter().rev().take(5) {
            if t.provider_key != current {
                ctx.push_str(&format!("[{}] {}\n", t.provider_key, t.content));
            }
        }
        if ctx.is_empty() {
            ctx = "(no prior context)".into();
        }
        ctx
    }
}

#[cfg(test)]
mod tests {
    use super::classify_vote;

    /// D-103 回归锁：多数票的**反对票不得被计成赞成票**。
    ///
    /// 修复前判据是 `content.to_lowercase().contains("agree")`，而 **"disagree"
    /// 里就含 "agree"**——所有反对票都落进"赞成"分支（红侧：这些断言取到 Some(true)）。
    #[test]
    fn test_d103_disagree_not_counted_as_agree() {
        assert_eq!(classify_vote("I disagree"), Some(false));
        assert_eq!(classify_vote("DISAGREE — reasons below"), Some(false));
        assert_eq!(classify_vote("I agree"), Some(true));
        assert_eq!(classify_vote("Agree, with caveats"), Some(true));
        // 两者同现时以**否定**为准（保守：需明确赞成才算赞成）
        assert_eq!(
            classify_vote("I don't agree, in fact I disagree"),
            Some(false)
        );
        // 都无法判定 ⇒ None（调用方计反对并 warn，不假装读懂）
        assert_eq!(classify_vote("no opinion"), None);
        assert_eq!(classify_vote(""), None);
    }
}
