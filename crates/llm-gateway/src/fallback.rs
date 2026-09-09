//! FallbackChain: degrade through a chain of providers on failure.
//!
//! When the primary provider fails (returns an error), FallbackChain
//! automatically tries the next provider in the chain. It stops when
//! a provider succeeds or the chain is exhausted.
//!
//! ## Bounded retry
//! Each provider is tried at most once per call. There is no unbounded
//! retry loop — the chain length is the hard limit.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::provider::LlmProvider;
use crate::types::{Capabilities, ChatRequest, ChatResponse, Embedding, StreamEvent};

/// A chain of LLM providers used for graceful degradation.
///
/// Providers are tried in order. On success, the result is returned.
/// On error, the next provider is tried. If all providers fail,
/// the last error is returned.
pub struct FallbackChain {
    /// Ordered list of (display_name, provider) pairs.
    providers: Vec<(String, Arc<dyn LlmProvider>)>,
}

impl FallbackChain {
    /// Create a new fallback chain from an ordered list of providers.
    ///
    /// The first provider is the primary; subsequent ones are fallbacks.
    pub fn new(providers: Vec<(String, Arc<dyn LlmProvider>)>) -> Self {
        Self { providers }
    }

    /// Number of providers in the chain.
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// Whether the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    /// Get the name of the primary provider.
    pub fn primary_name(&self) -> Option<&str> {
        self.providers.first().map(|(name, _)| name.as_str())
    }

    /// List all provider names in order.
    pub fn provider_names(&self) -> Vec<&str> {
        self.providers
            .iter()
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Execute a chat completion through the fallback chain.
    ///
    /// Returns the first successful response, or the last error.
    pub async fn chat(&self, req: ChatRequest) -> Result<(String, ChatResponse)> {
        let mut last_err: Option<anyhow::Error> = None;

        for (name, provider) in &self.providers {
            match provider.chat(req.clone()).await {
                Ok(resp) => return Ok((name.clone(), resp)),
                Err(e) => {
                    tracing::warn!(
                        provider = %name,
                        error = %e,
                        "fallback: provider failed, trying next"
                    );
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("fallback chain is empty")))
    }

    /// Execute a streaming chat completion through the fallback chain.
    ///
    /// Streaming fallback is NOT transparent — we return the stream
    /// from the first provider that accepts the request. If streaming
    /// fails mid-stream, the caller should retry via `chat()`.
    pub fn stream(&self, req: ChatRequest) -> BoxStream<'static, Result<StreamEvent>> {
        // For streaming, we only try the primary provider.
        // Mid-stream failures can't be transparently retried.
        if let Some((name, provider)) = self.providers.first() {
            tracing::debug!(primary = %name, "streaming via primary provider");
            provider.stream(req)
        } else {
            Box::pin(futures::stream::once(async {
                Err(anyhow::anyhow!("fallback chain is empty"))
            }))
        }
    }

    /// Execute embedding through the fallback chain.
    pub async fn embed(&self, inputs: &[String]) -> Result<(String, Vec<Embedding>)> {
        let mut last_err: Option<anyhow::Error> = None;

        for (name, provider) in &self.providers {
            match provider.embed(inputs).await {
                Ok(resp) => return Ok((name.clone(), resp)),
                Err(e) => {
                    tracing::warn!(
                        provider = %name,
                        error = %e,
                        "fallback: embed failed, trying next"
                    );
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("fallback chain is empty")))
    }
}

// P5: FallbackChain implements LlmProvider so it can be registered
// as a provider in the ProviderRegistry and used transparently.
#[async_trait]
impl LlmProvider for FallbackChain {
    fn name(&self) -> &str {
        "fallback"
    }

    fn model(&self) -> &str {
        self.primary_name().unwrap_or("unknown")
    }

    fn capabilities(&self) -> Capabilities {
        // Union of all providers' capabilities (pessimistic: only claim what all support)
        let mut caps = Capabilities {
            chat: true,
            stream: false, // streaming fallback not transparent
            function_calling: true,
            embeddings: true,
            max_context_tokens: None,
        };
        for (_, p) in &self.providers {
            let pc = p.capabilities();
            caps.function_calling = caps.function_calling && pc.function_calling;
            caps.embeddings = caps.embeddings && pc.embeddings;
            caps.max_context_tokens = match (caps.max_context_tokens, pc.max_context_tokens) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) => Some(a),
                (None, _) => None,
            };
        }
        caps
    }

    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        let mut last_err: Option<anyhow::Error> = None;

        for (name, provider) in &self.providers {
            match provider.chat(req.clone()).await {
                Ok(resp) => {
                    tracing::debug!(provider = %name, "fallback: chat succeeded via {}", name);
                    return Ok(resp);
                }
                Err(e) => {
                    tracing::warn!(provider = %name, error = %e, "fallback: provider failed, trying next");
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("fallback chain is empty")))
    }

    fn stream(&self, req: ChatRequest) -> BoxStream<'static, Result<StreamEvent>> {
        if let Some((name, provider)) = self.providers.first() {
            tracing::debug!(primary = %name, "fallback: streaming via primary provider");
            provider.stream(req)
        } else {
            Box::pin(futures::stream::once(async {
                Err(anyhow::anyhow!("fallback chain is empty"))
            }))
        }
    }

    async fn embed(&self, inputs: &[String]) -> Result<Vec<Embedding>> {
        let mut last_err: Option<anyhow::Error> = None;

        for (name, provider) in &self.providers {
            match provider.embed(inputs).await {
                Ok(resp) => {
                    tracing::debug!(provider = %name, "fallback: embed succeeded via {}", name);
                    return Ok(resp);
                }
                Err(e) => {
                    tracing::warn!(provider = %name, error = %e, "fallback: embed failed, trying next");
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("fallback chain is empty")))
    }
}

impl std::fmt::Debug for FallbackChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FallbackChain")
            .field("providers", &self.provider_names())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Embedding, StreamEvent};
    use crate::Capabilities;
    use async_trait::async_trait;

    /// A mock provider that can be configured to succeed or fail.
    struct MockProvider {
        name: String,
        model: String,
        should_fail: bool,
        fail_message: String,
    }

    impl MockProvider {
        fn new(name: &str, should_fail: bool) -> Self {
            Self {
                name: name.into(),
                model: "mock".into(),
                should_fail,
                fail_message: format!("{name} failed"),
            }
        }
    }

    #[async_trait]
    impl LlmProvider for MockProvider {
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
                function_calling: false,
                embeddings: true,
                max_context_tokens: None,
            }
        }
        async fn chat(&self, _req: ChatRequest) -> Result<ChatResponse> {
            if self.should_fail {
                Err(anyhow::anyhow!("{}", self.fail_message))
            } else {
                Ok(ChatResponse {
                    content: Some(format!("response from {}", self.name)),
                    tool_calls: vec![],
                    finish_reason: Some("stop".into()),
                    usage: None,
                    reasoning_content: None,
                })
            }
        }
        fn stream(&self, _req: ChatRequest) -> BoxStream<'static, Result<StreamEvent>> {
            let should_fail = self.should_fail;
            let fail_message = self.fail_message.clone();
            let name = self.name.clone();
            if should_fail {
                Box::pin(futures::stream::once(async move {
                    Err(anyhow::anyhow!("{}", fail_message))
                }))
            } else {
                Box::pin(futures::stream::once(async move {
                    Ok(StreamEvent::Token(format!("stream from {}", name)))
                }))
            }
        }
        async fn embed(&self, _inputs: &[String]) -> Result<Vec<Embedding>> {
            if self.should_fail {
                Err(anyhow::anyhow!("{}", self.fail_message))
            } else {
                Ok(vec![Embedding {
                    index: 0,
                    values: vec![0.1, 0.2],
                }])
            }
        }
    }

    fn test_req() -> ChatRequest {
        ChatRequest {
            messages: vec![],
            tools: vec![],
            temperature: None,
            max_tokens: None,
            stream: false,
        }
    }

    #[tokio::test]
    async fn test_primary_succeeds() {
        let chain = FallbackChain::new(vec![
            (
                "primary".into(),
                Arc::new(MockProvider::new("primary", false)),
            ),
            (
                "fallback".into(),
                Arc::new(MockProvider::new("fallback", false)),
            ),
        ]);

        let (name, resp) = chain.chat(test_req()).await.unwrap();
        assert_eq!(name, "primary");
        assert_eq!(resp.content.unwrap(), "response from primary");
    }

    #[tokio::test]
    async fn test_fallback_after_primary_fails() {
        let chain = FallbackChain::new(vec![
            (
                "primary".into(),
                Arc::new(MockProvider::new("primary", true)),
            ),
            (
                "fallback".into(),
                Arc::new(MockProvider::new("fallback", false)),
            ),
        ]);

        let (name, resp) = chain.chat(test_req()).await.unwrap();
        assert_eq!(name, "fallback");
        assert_eq!(resp.content.unwrap(), "response from fallback");
    }

    #[tokio::test]
    async fn test_all_fail() {
        let chain = FallbackChain::new(vec![
            ("a".into(), Arc::new(MockProvider::new("a", true))),
            ("b".into(), Arc::new(MockProvider::new("b", true))),
        ]);

        let err = chain.chat(test_req()).await.unwrap_err();
        assert!(err.to_string().contains("b failed"));
    }

    #[tokio::test]
    async fn test_empty_chain() {
        let chain = FallbackChain::new(vec![]);
        assert!(chain.is_empty());
        let err = chain.chat(test_req()).await.unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[tokio::test]
    async fn test_embed_fallback() {
        let chain = FallbackChain::new(vec![
            ("a".into(), Arc::new(MockProvider::new("a", true))),
            ("b".into(), Arc::new(MockProvider::new("b", false))),
        ]);

        let (name, embeds) = chain.embed(&["test".into()]).await.unwrap();
        assert_eq!(name, "b");
        assert_eq!(embeds[0].values, vec![0.1, 0.2]);
    }

    #[test]
    fn test_chain_metadata() {
        let chain = FallbackChain::new(vec![
            (
                "primary".into(),
                Arc::new(MockProvider::new("primary", false)),
            ),
            (
                "fallback".into(),
                Arc::new(MockProvider::new("fallback", false)),
            ),
        ]);
        assert_eq!(chain.len(), 2);
        assert!(!chain.is_empty());
        assert_eq!(chain.primary_name(), Some("primary"));
        assert_eq!(chain.provider_names(), vec!["primary", "fallback"]);
    }

    #[tokio::test]
    async fn test_stream_uses_primary() {
        let chain = FallbackChain::new(vec![
            (
                "primary".into(),
                Arc::new(MockProvider::new("primary", false)),
            ),
            (
                "fallback".into(),
                Arc::new(MockProvider::new("fallback", false)),
            ),
        ]);

        use futures::StreamExt;
        let mut stream = chain.stream(test_req());
        let event = stream.next().await.unwrap().unwrap();
        if let StreamEvent::Token(t) = event {
            assert_eq!(t, "stream from primary");
        } else {
            panic!("expected token");
        }
    }
}
