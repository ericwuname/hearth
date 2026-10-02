use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

use crate::provider::LlmProvider;

/// Registry of LLM providers, keyed by "provider:model".
/// Aliases (e.g. "openai" → "openai:gpt-4o") provide backward-compatible lookups.
/// Allows runtime switching between providers (P0 A3 + 6A v2 requirement).
///
/// D-94（2026-10-02, traecode）：**删除了 `default` 字段与 `set_default` 参数**。
/// 病灶：响应体里根本没有"默认 provider"概念——service 的 `SessionCreate.provider`
/// 是**必填**，`SessionManager::create_session` 直接 `registry.get(&provider)`；
/// 而 `get_default()` 全仓**唯一调用者是本文件的单测**。即启动期那几处
/// `register(_, true)` 只是往一个永不读取的字段里写值，纯死语义。
/// 更糟的是 main.rs 把该参数注成 "required" 并宣称 "OpenAI primary"——三重不实。
/// 依据"声称≠实现"纪律（D-23/D-45/D-73 同族）删除，**零行为变更**。
#[derive(Default)]
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn LlmProvider>>,
    aliases: HashMap<String, String>, // alias → canonical key
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            aliases: HashMap::new(),
        }
    }

    /// Register a provider with canonical "provider:model" key.
    pub fn register_with_key(
        &mut self,
        provider_name: &str,
        model: &str,
        provider: Arc<dyn LlmProvider>,
    ) {
        let key = format!("{provider_name}:{model}");
        self.providers.insert(key, provider);
    }

    /// Register an alias (e.g. "openai" → "openai:gpt-4o").
    pub fn register_alias(&mut self, alias: &str, canonical_key: &str) {
        self.aliases
            .insert(alias.to_string(), canonical_key.to_string());
    }

    /// Register a provider (backward-compatible: uses provider.name() as key).
    pub fn register(&mut self, provider: Arc<dyn LlmProvider>) {
        let name = provider.name().to_string();
        self.providers.insert(name, provider);
    }

    /// Get a provider by name, alias, or "provider:model" key.
    pub fn get(&self, name: &str) -> Result<Arc<dyn LlmProvider>> {
        // 1) direct key lookup
        if let Some(p) = self.providers.get(name) {
            return Ok(p.clone());
        }
        // 2) alias resolution
        if let Some(canonical) = self.aliases.get(name) {
            if let Some(p) = self.providers.get(canonical) {
                return Ok(p.clone());
            }
        }
        // 3) partial match: "provider:" prefix → find first matching key
        if name.ends_with(':') {
            let prefix = name; // e.g. "openai:"
            for (key, p) in &self.providers {
                if key.starts_with(prefix) {
                    return Ok(p.clone());
                }
            }
        }
        Err(anyhow::anyhow!("provider not found: {}", name))
    }

    /// List all registered provider names.
    pub fn list(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    /// List provider names with models.
    pub fn list_detailed(&self) -> Vec<(String, String)> {
        self.providers
            .iter()
            .map(|(name, p)| (name.clone(), p.model().to_string()))
            .collect()
    }
}

impl std::fmt::Debug for ProviderRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderRegistry")
            .field("providers", &self.providers.keys().collect::<Vec<_>>())
            .field("aliases", &self.aliases)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::LlmProvider;
    use crate::types::{Capabilities, ChatRequest, ChatResponse, Embedding, StreamEvent};
    use async_trait::async_trait;
    use futures::stream::{self, BoxStream};

    struct MockProvider {
        name: String,
        model: String,
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
            Capabilities::default()
        }
        async fn chat(&self, _req: ChatRequest) -> Result<ChatResponse> {
            Ok(ChatResponse {
                content: Some("mock".into()),
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

    #[test]
    fn test_register_and_get() {
        let mut reg = ProviderRegistry::new();
        reg.register(Arc::new(MockProvider {
            name: "test".into(),
            model: "m1".into(),
        }));
        let p = reg.get("test").unwrap();
        assert_eq!(p.name(), "test");
        assert_eq!(p.model(), "m1");
    }

    // D-94：`test_get_default` 已随死语义 `get_default()`/`default` 字段一并删除
    // （生产零调用方，保留测试等于给死代码续命）。

    #[test]
    fn test_missing_provider() {
        let reg = ProviderRegistry::new();
        assert!(reg.get("nope").is_err());
    }

    #[test]
    fn test_list_detailed() {
        let mut reg = ProviderRegistry::new();
        reg.register(Arc::new(MockProvider {
            name: "x".into(),
            model: "mx".into(),
        }));
        let list = reg.list_detailed();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0], ("x".to_string(), "mx".to_string()));
    }
}
