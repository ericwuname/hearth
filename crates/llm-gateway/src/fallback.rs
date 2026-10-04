//! FallbackChain: degrade through a chain of providers on failure.
//!
//! ## 何时切换通道（**S9 定版语义，以本节为准**）
//!
//! **只在通道级不可恢复错误（fatal/param：401/403/400/畸形流/策略拒绝）时才切下一
//! 通道**；**transient（网络抖动/超时/429/5xx）不切**——原样上抛，交回上层 S7 长退避
//! （同通道等待重试）。理由：瞬时抖动即换模型层会污染"同尺对照"（换 provider＝换模型层，
//! 基准不可比）。
//!
//! > 本模块头旧文曾写「主通道一出错即自动试链中下一通道」——那是 **S9 之前的语义，已作废**。
//! > 维护者勿照旧文把"transient 不切"当作 bug 去"修"。D-147 据代码订正，行为锁见
//! > `tests::test_s9_transient_does_not_switch_chain` / `tests::test_s9_fatal_switches_next_with_callback`。
//!
//! ## `stream()` **不参与**降级
//!
//! `stream()` 只走**首通道（primary）**，失败不上抛切换（`capabilities().stream` 亦据此
//! 为 `false`）。走整链降级的只有非流式 `chat()` 与 `embed()`（行为锁 `tests::test_stream_uses_primary`）。
//!
//! ## Bounded retry
//! Each provider is tried at most once per call. There is no unbounded
//! retry loop — the chain length is the hard limit.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::classify_anyhow;
use crate::provider::LlmProvider;
use crate::types::{Capabilities, ChatRequest, ChatResponse, Embedding, ErrorClass, StreamEvent};

/// S9（手术包二）：通道切换通知回调——(失败通道, 下一通道)。CLI/上层据此
/// 投影 `[fallback] provider: a → b`（降级数据必须标注通道：换 provider =
/// 换模型层，基准对照不可比）。
pub type SwitchFn = Arc<dyn Fn(&str, &str) + Send + Sync>;

/// A chain of LLM providers used for graceful degradation.
///
/// Providers are tried in order. On success, the result is returned.
/// On error, the next provider is tried. If all providers fail,
/// the last error is returned.
///
/// S9（手术包二）语义：**只有通道级不可恢复错误（fatal/param：401/403/400/
/// 畸形流/策略拒绝）才切下一通道**；transient（网络抖动/超时/429/5xx）交回
/// 上层 S7 长退避（等 1 分钟重试同通道）——避免"主通道瞬时抖动即换模型层"
/// 污染同尺对照。窗口耗尽时的自动切换需链与 S7 窗口联动，属后续项（申报）。
pub struct FallbackChain {
    /// Ordered list of (display_name, provider) pairs.
    providers: Vec<(String, Arc<dyn LlmProvider>)>,
    /// S9：切换通知（可选）。
    on_switch: Option<SwitchFn>,
}

impl FallbackChain {
    /// Create a new fallback chain from an ordered list of providers.
    ///
    /// The first provider is the primary; subsequent ones are fallbacks.
    pub fn new(providers: Vec<(String, Arc<dyn LlmProvider>)>) -> Self {
        Self {
            providers,
            on_switch: None,
        }
    }

    /// S9：挂切换回调（投影 [fallback] 用）。
    pub fn with_switch_callback(mut self, cb: SwitchFn) -> Self {
        self.on_switch = Some(cb);
        self
    }

    fn fire_switch(&self, from: &str, to: &str) {
        if let Some(cb) = &self.on_switch {
            cb(from, to);
        }
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

    // D-115（2026-10-02, traecode）：原固有的 `stream()` **已删除**（D-78 余项收口）
    // ——它与下方 `impl LlmProvider for FallbackChain` 的 `stream()` **函数体逐字重复**，
    // 且全仓零生产调用方（生产路径一律以 `Arc<dyn LlmProvider>` 注册、走 trait 方法）。
    // 保留两份同体实现是一个漂移陷阱（改一处忘另一处即行为分叉）。删除后，任何
    // `FallbackChain` 的具体类型调用 `stream()` 由 trait 实现兜底（语义完全一致）。
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

        for (idx, (name, provider)) in self.providers.iter().enumerate() {
            match provider.chat(req.clone()).await {
                Ok(resp) => {
                    tracing::debug!(provider = %name, "fallback: chat succeeded via {}", name);
                    return Ok(resp);
                }
                Err(e) => {
                    let class = classify_anyhow(&e);
                    tracing::warn!(provider = %name, error = %e, class = ?class, "fallback: provider failed");
                    let has_next = idx + 1 < self.providers.len();
                    // S9：transient 不切（交回上层 S7 长退避）；fatal/param 且有
                    // 下一通道 → 切（投影 [fallback] 由回调完成）。
                    if matches!(class, ErrorClass::Transient) || !has_next {
                        return Err(e);
                    }
                    let next = self.providers[idx + 1].0.clone();
                    self.fire_switch(name, &next);
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

        /// S9：自定义失败消息（分级测试：fatal vs transient）。
        fn with_message(name: &str, msg: &str) -> Self {
            Self {
                name: name.into(),
                model: "mock".into(),
                should_fail: true,
                fail_message: msg.into(),
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

        let resp = chain.chat(test_req()).await.unwrap();
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

        let resp = chain.chat(test_req()).await.unwrap();
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

        let embeds = chain.embed(&["test".into()]).await.unwrap();
        assert_eq!(embeds[0].values, vec![0.1, 0.2]);
    }

    /// S9（手术包二）：fatal（401/通道级不可恢复）→ 自动切下一通道 + 切换回调
    /// 触发（[fallback] 投影源）。
    #[tokio::test]
    async fn test_s9_fatal_switches_next_with_callback() {
        let switches = Arc::new(std::sync::Mutex::new(Vec::<(String, String)>::new()));
        let s = switches.clone();
        let chain = FallbackChain::new(vec![
            (
                "agnes".into(),
                Arc::new(MockProvider::with_message("agnes", "HTTP 401 unauthorized")),
            ),
            ("zhipu".into(), Arc::new(MockProvider::new("zhipu", false))),
        ])
        .with_switch_callback(Arc::new(move |a: &str, b: &str| {
            s.lock().unwrap().push((a.to_string(), b.to_string()));
        }));

        let resp = chain.chat(test_req()).await.unwrap();
        assert_eq!(
            resp.content.unwrap(),
            "response from zhipu",
            "fatal 后必须切到次通道完成任务（单通道死等=全挂的病灶）"
        );
        let sw = switches.lock().unwrap().clone();
        assert_eq!(sw.len(), 1, "切换必须触发回调（[fallback] 投影源）");
        assert_eq!(sw[0], ("agnes".to_string(), "zhipu".to_string()));
    }

    /// S9：transient（网络抖动/超时/429）**不切通道**——交回上层 S7 长退避
    ///（同通道等待重试；瞬时抖动换模型层会污染同尺对照）。
    #[tokio::test]
    async fn test_s9_transient_does_not_switch_chain() {
        let chain = FallbackChain::new(vec![
            (
                "agnes".into(),
                Arc::new(MockProvider::with_message("agnes", "request timed out")),
            ),
            ("zhipu".into(), Arc::new(MockProvider::new("zhipu", false))),
        ]);
        let err = chain.chat(test_req()).await.unwrap_err();
        assert!(
            err.to_string().contains("timed out"),
            "transient 必须原样上抛（不切换通道）: {err:#}"
        );
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

    /// D-147 回归锁（**先红后绿**）：模块头文档**不得回退**到 S9 之前的过时语义
    /// （"任一出错即自动试下一通道"）。依据：`chat()` 的 transient 分支直接
    /// `return Err`（见 `test_s9_transient_does_not_switch_chain`），模块头若再宣称
    /// "任一出错即切"即与实现矛盾（声称≠实现）；`stream()` 更**从不**降级。
    #[test]
    fn test_d147_module_doc_matches_s9_semantics() {
        // 注：禁词在测试内**拼接**而非写成整句字面量——否则 `include_str!` 会把本测试
        // 自身那句字面量也算进去、断言恒红（自指陷阱）。
        let banned = [
            "automatically",
            "tries",
            "the",
            "next",
            "provider",
            "in",
            "the",
            "chain",
        ]
        .join(" ");
        let src = include_str!("fallback.rs");
        assert!(
            !src.contains(&banned),
            "模块头又变回过时语义：S9 之后**仅** fatal/param 才切通道，transient 不切\
             ——请与 `chat()` 实现及 `test_s9_transient_does_not_switch_chain` 对齐。"
        );
        assert!(
            src.contains("transient") && src.contains("不切"),
            "模块头必须写明「transient 不切通道」这一 S9 语义（否则维护者会照旧文误判为 bug 去'修'）。"
        );
    }
}
