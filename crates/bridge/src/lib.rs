//! Bridge crate — multi-model real-time discussion and consensus.
//! 6B v6.0: BridgeSession manages peer discussion across multiple LLM providers.

use agent_types::{Message, MessageContent, MessageMeta, Role};
use chrono::Utc;
use llm_gateway::types::ChatRequest;
use llm_gateway::ProviderRegistry;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

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
    pub vote: Option<Vote>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Vote {
    Agree,
    Disagree(String),
    Abstain,
}

/// Discussion strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BridgeStrategy {
    RoundRobin,
    Debate,
    MajorityVote,
    SingleSpeaker { speaker: String },
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
    pub event_tx: broadcast::Sender<BridgeEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BridgeEvent {
    Turn(Turn),
    Consensus { agreed: bool, summary: String },
    Error(String),
}

impl BridgeSession {
    pub fn new(
        topic: String,
        participants: Vec<String>,
        strategy: BridgeStrategy,
        registry: Arc<ProviderRegistry>,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(64);
        Self {
            id: Uuid::new_v4().to_string(),
            topic,
            strategy,
            turns: Vec::new(),
            participants,
            registry,
            event_tx,
        }
    }

    pub async fn run(&mut self, max_rounds: u32) -> anyhow::Result<String> {
        match &self.strategy {
            BridgeStrategy::RoundRobin => self.run_round_robin(max_rounds).await,
            BridgeStrategy::MajorityVote => self.run_majority(max_rounds).await,
            BridgeStrategy::Debate => self.run_debate(max_rounds).await,
            BridgeStrategy::SingleSpeaker { .. } => self.run_single(max_rounds).await,
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
                    vote: None,
                };
                let _ = self.event_tx.send(BridgeEvent::Turn(turn.clone()));
                self.turns.push(turn);
                summaries.push(content);
            }
        }
        let result = format!(
            "Bridge round-robin complete ({} turns).\n{}",
            self.turns.len(),
            summaries.join("\n\n---\n\n")
        );
        let _ = self.event_tx.send(BridgeEvent::Consensus {
            agreed: true,
            summary: result.clone(),
        });
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
                let vote = if content.to_lowercase().contains("agree") {
                    votes_for += 1;
                    Vote::Agree
                } else {
                    votes_against += 1;
                    Vote::Disagree(content.clone())
                };
                let turn = Turn {
                    index: self.turns.len() as u32,
                    provider_key: participant.clone(),
                    model: provider.model().to_string(),
                    content,
                    vote: Some(vote),
                };
                let _ = self.event_tx.send(BridgeEvent::Turn(turn.clone()));
                self.turns.push(turn);
            }
            if votes_for > votes_against || round >= max_rounds - 1 {
                let agreed = votes_for > votes_against;
                let result = format!("Majority: {votes_for} agree, {votes_against} disagree");
                let _ = self.event_tx.send(BridgeEvent::Consensus {
                    agreed,
                    summary: result.clone(),
                });
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
                    vote: None,
                };
                let _ = self.event_tx.send(BridgeEvent::Turn(turn.clone()));
                self.turns.push(turn);
            }
        }
        let result = format!("Debate complete — {} turns.", self.turns.len());
        let _ = self.event_tx.send(BridgeEvent::Consensus {
            agreed: true,
            summary: result.clone(),
        });
        Ok(result)
    }

    async fn run_single(&mut self, _max_rounds: u32) -> anyhow::Result<String> {
        let speaker = if let BridgeStrategy::SingleSpeaker { speaker } = &self.strategy {
            speaker.clone()
        } else {
            self.participants.first().cloned().unwrap_or_default()
        };
        let provider = self.registry.get(&speaker)?;
        let req = ChatRequest {
            messages: vec![msg(
                Role::System,
                &format!("Topic: {topic}\nProvide your analysis.", topic = self.topic),
            )],
            tools: vec![],
            temperature: Some(0.5),
            max_tokens: Some(2048),
            stream: false,
        };
        let resp = provider.chat(req).await?;
        let content = resp.content.unwrap_or_default();
        let turn = Turn {
            index: 0,
            provider_key: speaker.clone(),
            model: provider.model().to_string(),
            content: content.clone(),
            vote: None,
        };
        let _ = self.event_tx.send(BridgeEvent::Turn(turn.clone()));
        self.turns.push(turn.clone());
        let _ = self.event_tx.send(BridgeEvent::Consensus {
            agreed: true,
            summary: content.clone(),
        });
        Ok(content)
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
