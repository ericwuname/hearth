use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::types::{Capabilities, ChatRequest, ChatResponse, Embedding, StreamEvent};

/// The core LLM provider trait.
/// All concrete providers (OpenAI, local, CN) must implement this.
/// `agent-core` depends ONLY on this trait, never on a specific provider crate.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Human-readable provider name (e.g. "openai", "ollama").
    fn name(&self) -> &str;

    /// Currently active model name.
    fn model(&self) -> &str;

    /// What this provider can do.
    fn capabilities(&self) -> Capabilities;

    /// Non-streaming chat completion.
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse>;

    /// Streaming chat completion.
    fn stream(&self, req: ChatRequest) -> BoxStream<'static, Result<StreamEvent>>;

    /// Generate embeddings for the given inputs.
    async fn embed(&self, inputs: &[String]) -> Result<Vec<Embedding>>;
}
