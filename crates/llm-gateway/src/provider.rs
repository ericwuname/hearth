use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::types::{Capabilities, ChatRequest, ChatResponse, Embedding, StreamEvent};

/// The core LLM provider trait.
/// All concrete providers (OpenAI, local, CN) must implement this.
/// `agent-core` depends ONLY on this trait, never on a specific provider crate.
// D-91（2026-10-02, traecode）：**此处刻意不加 `#[allow(clippy::double_must_use)]`**。
// 该 lint 是 1.99.0 新增，会把工作区内**每一个** `#[async_trait]` + `Result` 的
// trait 方法都判为"重复 must_use"（`agent_types`/`subconscious`/`tool-runtime`/
// `sandbox`/`memory`/`llm-gateway` 全线命中）——纯属宏产物误报。处置不是撒 allow，
// 而是**把工具链钉在 1.98.1**（见仓库根 `rust-toolchain.toml` 的升级仪式）。
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
