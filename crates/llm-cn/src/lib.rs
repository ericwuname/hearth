//! llm-cn: Tencent Hunyuan (混元) LLM provider.
//!
//! ## Key differences from OpenAI
//! - Auth: `Authorization: Bearer <key>` (same as OpenAI), but uses
//!   `hunyuan.cloud.tencent.com` as the base endpoint.
//! - SSE: standard `data: ...` lines, same format as OpenAI.
//! - Tool calling: OpenAI-compatible `tool_calls` in responses.
//! - Error responses: use a `Response.Error.Message` field (vs OpenAI's
//!   `error.message`).
//!
//! ## Wire-format testing
//! All provider tests use mock HTTP servers (axum) that assert on request
//! bodies and serve controlled responses.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::channel::mpsc;
use futures::stream::{BoxStream, StreamExt};
use futures::SinkExt;
use llm_gateway::{
    Capabilities, ChatRequest, ChatResponse as GatewayChatResponse, Embedding, LlmProvider,
    StreamEvent, Usage,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Hunyuan wire types ──
// Hunyuan uses OpenAI-compatible wire format with a few differences:
// - Base URL: https://hunyuan.cloud.tencent.com/hyllm/v1
// - Error format: { "Response": { "Error": { "Message": "..." } } }

#[derive(Serialize)]
struct HunyuanRequest {
    model: String,
    messages: Vec<HunyuanMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<HunyuanTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    stream: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct HunyuanMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<HunyuanToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct HunyuanToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: HunyuanFunctionCall,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct HunyuanFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Serialize)]
struct HunyuanTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: HunyuanFunctionDef,
}

#[derive(Serialize)]
struct HunyuanFunctionDef {
    name: String,
    description: String,
    parameters: Value,
}

#[derive(Deserialize, Debug)]
struct HunyuanResponse {
    choices: Vec<HunyuanChoice>,
    usage: Option<HunyuanUsage>,
}

#[derive(Deserialize, Debug)]
struct HunyuanChoice {
    message: Option<HunyuanMessage>,
    delta: Option<HunyuanDelta>,
    finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
struct HunyuanDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<HunyuanToolCallDelta>>,
}

#[derive(Deserialize, Debug)]
struct HunyuanToolCallDelta {
    #[allow(dead_code)]
    index: u64,
    #[serde(default)]
    id: Option<String>,
    function: Option<HunyuanFunctionDelta>,
}

#[derive(Deserialize, Debug)]
struct HunyuanFunctionDelta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Deserialize, Debug)]
struct HunyuanUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Deserialize, Debug)]
struct HunyuanStreamChunk {
    choices: Vec<HunyuanChoice>,
}

/// Hunyuan-specific error wrapper.
#[derive(Deserialize, Debug)]
struct HunyuanErrorWrapper {
    #[serde(rename = "Response")]
    response: HunyuanErrorResponse,
}

#[derive(Deserialize, Debug)]
struct HunyuanErrorResponse {
    #[serde(rename = "Error")]
    error: HunyuanErrorDetail,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct HunyuanErrorDetail {
    #[serde(rename = "Message")]
    message: String,
    #[serde(rename = "Code")]
    code: Option<String>,
}

#[derive(Serialize)]
#[allow(dead_code)]
struct HunyuanEmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct HunyuanEmbeddingResponse {
    data: Vec<HunyuanEmbeddingData>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct HunyuanEmbeddingData {
    index: usize,
    embedding: Vec<f32>,
}

// ── SSE parsing ──

fn parse_hunyuan_sse_line(line: &str) -> Option<Result<StreamEvent>> {
    let data = line.strip_prefix("data: ")?.trim();
    if data == "[DONE]" {
        return Some(Ok(StreamEvent::Finish {
            finish_reason: Some("stop".into()),
            usage: None,
        }));
    }
    let chunk: HunyuanStreamChunk = serde_json::from_str(data).ok()?;

    for choice in chunk.choices {
        if let Some(delta) = &choice.delta {
            if let Some(content) = &delta.content {
                if !content.is_empty() {
                    return Some(Ok(StreamEvent::Token(content.clone())));
                }
            }
            if let Some(tc_deltas) = &delta.tool_calls {
                for tc in tc_deltas {
                    if let Some(ref func) = tc.function {
                        return Some(Ok(StreamEvent::ToolCallDelta {
                            call_id: tc.id.clone().unwrap_or_default(),
                            name: func.name.clone(),
                            args_delta: func.arguments.clone().unwrap_or_default(),
                        }));
                    }
                }
            }
        }
        if let Some(finish) = &choice.finish_reason {
            if finish != "null" && !finish.is_empty() {
                return Some(Ok(StreamEvent::Finish {
                    finish_reason: Some(finish.clone()),
                    usage: None,
                }));
            }
        }
    }
    None
}

// ── Helper: consume SSE bytes ──

async fn consume_sse_stream(
    resp: reqwest::Response,
    tx: &mut mpsc::Sender<Result<StreamEvent>>,
    parse_fn: impl Fn(&str) -> Option<Result<StreamEvent>>,
) {
    let mut byte_stream = Box::pin(resp.bytes_stream());
    let mut buffer = String::new();

    loop {
        match byte_stream.as_mut().next().await {
            Some(Ok(bytes)) => {
                buffer.push_str(&String::from_utf8_lossy(&bytes));
                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim().to_string();
                    buffer = buffer[pos + 1..].to_string();
                    if let Some(event) = parse_fn(&line) {
                        if tx.send(event).await.is_err() {
                            return;
                        }
                    }
                }
            }
            Some(Err(e)) => {
                let _ = tx.send(Err(anyhow!("stream read error: {e}"))).await;
                return;
            }
            None => return,
        }
    }
}

// ── HunyuanProvider ──

pub struct HunyuanProvider {
    name: String,
    model: String,
    client: Client,
    base_url: String,
    api_key: String,
}

impl HunyuanProvider {
    /// Create a new Hunyuan provider.
    ///
    /// `base_url` defaults to `https://hunyuan.cloud.tencent.com/hyllm/v1`.
    pub fn new(
        name: impl Into<String>,
        model: impl Into<String>,
        base_url: Option<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            model: model.into(),
            // LLM-1: default reqwest client has no timeout.
            client: Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap_or_else(|_| Client::new()),
            // URL-1: trim trailing '/' to avoid double slashes in endpoints.
            base_url: base_url
                .unwrap_or_else(|| "https://hunyuan.cloud.tencent.com/hyllm/v1".into())
                .trim_end_matches('/')
                .to_string(),
            api_key: api_key.into(),
        }
    }

    fn build_request(&self, req: ChatRequest, stream: bool) -> HunyuanRequest {
        let messages: Vec<HunyuanMessage> = req
            .messages
            .into_iter()
            .map(|m| {
                let (role, content, tool_calls, tool_call_id) = match m.role {
                    agent_types::Role::System => {
                        ("system".to_string(), m.content_to_text(), None, None)
                    }
                    agent_types::Role::User => {
                        ("user".to_string(), m.content_to_text(), None, None)
                    }
                    agent_types::Role::Assistant => match &m.content {
                        agent_types::MessageContent::Text(t) => {
                            ("assistant".to_string(), Some(t.clone()), None, None)
                        }
                        agent_types::MessageContent::ToolCalls(tcs) => {
                            let h_tcs: Vec<HunyuanToolCall> = tcs
                                .iter()
                                .map(|tc| HunyuanToolCall {
                                    id: tc.call_id.clone(),
                                    call_type: "function".into(),
                                    function: HunyuanFunctionCall {
                                        name: tc.name.clone(),
                                        arguments: serde_json::to_string(&tc.args)
                                            .unwrap_or_default(),
                                    },
                                })
                                .collect();
                            ("assistant".to_string(), None, Some(h_tcs), None)
                        }
                        agent_types::MessageContent::ToolResults(_) => (
                            "assistant".to_string(),
                            Some("[tool results omitted]".into()),
                            None,
                            None,
                        ),
                    },
                    agent_types::Role::Tool => {
                        let output = match &m.content {
                            agent_types::MessageContent::Text(t) => t.clone(),
                            agent_types::MessageContent::ToolResults(trs) => trs
                                .iter()
                                .map(|tr| tr.output.clone())
                                .collect::<Vec<_>>()
                                .join("\n"),
                            _ => String::new(),
                        };
                        let cid = m.meta.source.clone().unwrap_or_default();
                        ("tool".to_string(), Some(output), None, Some(cid))
                    }
                };

                HunyuanMessage {
                    role,
                    content,
                    tool_calls,
                    tool_call_id,
                }
            })
            .collect();

        let tools = if req.tools.is_empty() {
            None
        } else {
            Some(
                req.tools
                    .iter()
                    .map(|t| HunyuanTool {
                        tool_type: "function".into(),
                        function: HunyuanFunctionDef {
                            name: t.name.clone(),
                            description: t.description.clone(),
                            parameters: t.parameters.clone(),
                        },
                    })
                    .collect(),
            )
        };

        HunyuanRequest {
            model: self.model.clone(),
            messages,
            tools,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            stream,
        }
    }

    /// Try to extract a Hunyuan-specific error message from the response body.
    fn try_extract_hunyuan_error(&self, text: &str, status: reqwest::StatusCode) -> anyhow::Error {
        // Try Hunyuan error format first
        if let Ok(err_wrapper) = serde_json::from_str::<HunyuanErrorWrapper>(text) {
            return anyhow!(
                "Hunyuan error {}: {}",
                status,
                err_wrapper.response.error.message
            );
        }
        // Fallback: raw text
        anyhow!("Hunyuan error {status}: {text}")
    }
}

#[async_trait]
impl LlmProvider for HunyuanProvider {
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
            embeddings: false, // Hunyuan doesn't expose public embeddings API
            max_context_tokens: Some(32_000),
        }
    }

    async fn chat(&self, req: ChatRequest) -> Result<GatewayChatResponse> {
        let body = self.build_request(req, false);

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Hunyuan request failed: {e}"))?;

        let status = resp.status();
        let text = resp.text().await.map_err(|e| anyhow!("read body: {e}"))?;

        if !status.is_success() {
            return Err(self.try_extract_hunyuan_error(&text, status));
        }

        let parsed: HunyuanResponse = serde_json::from_str(&text)
            .map_err(|e| anyhow!("parse Hunyuan response: {e}: {text}"))?;

        let choice = parsed
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("no choices in response"))?;

        let msg = choice.message.unwrap_or(HunyuanMessage {
            role: "assistant".into(),
            content: None,
            tool_calls: None,
            tool_call_id: None,
        });

        let tool_calls = msg
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|tc| agent_types::ToolCall {
                call_id: tc.id,
                name: tc.function.name.clone(),
                // LLM-2: warn instead of silently degrading to a raw string.
                args: serde_json::from_str(&tc.function.arguments).unwrap_or_else(|e| {
                    tracing::warn!(
                        "tool call '{}' has non-JSON arguments ({e}); passing as raw string",
                        tc.function.name
                    );
                    Value::String(tc.function.arguments)
                }),
            })
            .collect();

        Ok(GatewayChatResponse {
            content: msg.content,
            tool_calls,
            finish_reason: choice.finish_reason,
            reasoning_content: None,
            usage: parsed.usage.map(|u| Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
                prompt_cache_hit_tokens: None,
                prompt_cache_miss_tokens: None,
            }),
        })
    }

    fn stream(&self, req: ChatRequest) -> BoxStream<'static, Result<StreamEvent>> {
        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let api_key = self.api_key.clone();
        let body = self.build_request(req, true);

        let (mut tx, rx) = mpsc::channel::<Result<StreamEvent>>(64);

        // STREAM-2: abort the producer task when the returned stream is dropped.
        let producer = tokio::spawn(async move {
            let resp = match client
                .post(format!("{}/chat/completions", base_url))
                .header("Authorization", format!("Bearer {api_key}"))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx
                        .send(Err(anyhow!("Hunyuan stream request failed: {e}")))
                        .await;
                    return;
                }
            };

            let status = resp.status();
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                let _ = tx
                    .send(Err(anyhow!("Hunyuan stream error {status}: {text}")))
                    .await;
                return;
            }

            consume_sse_stream(resp, &mut tx, parse_hunyuan_sse_line).await;
        });

        Box::pin(llm_gateway::AbortOnDropStream::new(
            rx,
            producer.abort_handle(),
        ))
    }

    async fn embed(&self, _inputs: &[String]) -> Result<Vec<Embedding>> {
        Err(anyhow!(
            "Hunyuan does not support embeddings via public API"
        ))
    }
}

// ── Helper trait for text extraction ──

trait ContentToText {
    fn content_to_text(&self) -> Option<String>;
}

impl ContentToText for agent_types::Message {
    fn content_to_text(&self) -> Option<String> {
        match &self.content {
            agent_types::MessageContent::Text(t) => Some(t.clone()),
            agent_types::MessageContent::ToolResults(trs) => Some(
                trs.iter()
                    .map(|tr| tr.output.clone())
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            _ => None,
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Wire-format tests (mock HTTP server, axum)
// ────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::post, Json, Router};
    use llm_gateway::ToolSchema;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::net::TcpListener;
    use tokio::sync::Mutex;

    /// Helper: create a user message.
    fn user_msg(text: &str) -> agent_types::Message {
        agent_types::Message::new(
            uuid::Uuid::new_v4().to_string(),
            agent_types::Role::User,
            agent_types::MessageContent::Text(text.into()),
        )
    }

    /// Start an axum server on a random port, return the base URL.
    async fn start_mock(app: Router) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{}", addr);
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        url
    }

    // ── Hunyuan wire tests ──

    #[tokio::test]
    async fn test_hunyuan_chat_wire_format() {
        let received = Arc::new(Mutex::new(None::<serde_json::Value>));
        let r = received.clone();

        let app = Router::new().route(
            "/chat/completions",
            post(move |Json(body): Json<serde_json::Value>| {
                let r = r.clone();
                async move {
                    *r.lock().await = Some(body);
                    Json(json!({
                        "choices": [{
                            "message": {
                                "role": "assistant",
                                "content": "你好，我是混元助手"
                            },
                            "finish_reason": "stop"
                        }],
                        "usage": {
                            "prompt_tokens": 20,
                            "completion_tokens": 8,
                            "total_tokens": 28
                        }
                    }))
                }
            }),
        );

        let base_url = start_mock(app).await;
        let provider = HunyuanProvider::new("hunyuan", "hunyuan-pro", Some(base_url), "sk-test");

        let req = ChatRequest {
            messages: vec![user_msg("你好")],
            tools: vec![],
            temperature: Some(0.7),
            max_tokens: Some(2048),
            stream: false,
        };

        let resp = provider.chat(req).await.unwrap();
        assert_eq!(resp.content.unwrap(), "你好，我是混元助手");
        assert_eq!(resp.usage.unwrap().total_tokens, 28);

        // Assert wire format
        let sent = received.lock().await.take().unwrap();
        assert_eq!(sent["model"], "hunyuan-pro");
        assert!(!sent["stream"].as_bool().unwrap());
        assert_eq!(sent["temperature"], 0.7);
        assert_eq!(sent["messages"][0]["content"], "你好");
    }

    #[tokio::test]
    async fn test_hunyuan_chat_with_tools() {
        let received = Arc::new(Mutex::new(None::<serde_json::Value>));
        let r = received.clone();

        let app = Router::new().route(
            "/chat/completions",
            post(move |Json(body): Json<serde_json::Value>| {
                let r = r.clone();
                async move {
                    *r.lock().await = Some(body);
                    Json(json!({
                        "choices": [{
                            "message": {
                                "role": "assistant",
                                "content": null,
                                "tool_calls": [{
                                    "id": "call_001",
                                    "type": "function",
                                    "function": {
                                        "name": "search",
                                        "arguments": "{\"query\":\"天气\"}"
                                    }
                                }]
                            },
                            "finish_reason": "tool_calls"
                        }],
                        "usage": {
                            "prompt_tokens": 30,
                            "completion_tokens": 15,
                            "total_tokens": 45
                        }
                    }))
                }
            }),
        );

        let base_url = start_mock(app).await;
        let provider = HunyuanProvider::new("hunyuan", "hunyuan-pro", Some(base_url), "sk-test");

        let req = ChatRequest {
            messages: vec![user_msg("今天天气怎么样")],
            tools: vec![ToolSchema {
                name: "search".into(),
                description: "搜索信息".into(),
                parameters: json!({"type": "object", "properties": {"query": {"type": "string"}}}),
            }],
            temperature: None,
            max_tokens: None,
            stream: false,
        };

        let resp = provider.chat(req).await.unwrap();
        assert!(resp.content.is_none());
        assert_eq!(resp.tool_calls.len(), 1);
        assert_eq!(resp.tool_calls[0].name, "search");
        assert_eq!(resp.tool_calls[0].args["query"], "天气");
        assert_eq!(resp.finish_reason.unwrap(), "tool_calls");

        let sent = received.lock().await.take().unwrap();
        let tools = sent["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["function"]["name"], "search");
    }

    #[tokio::test]
    async fn test_hunyuan_error_hunyuan_format() {
        let app = Router::new().route(
            "/chat/completions",
            post(|| async {
                (
                    axum::http::StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "Response": {
                            "Error": {
                                "Message": "认证失败，请检查API密钥",
                                "Code": "AuthFailure"
                            }
                        }
                    })),
                )
            }),
        );

        let base_url = start_mock(app).await;
        let provider = HunyuanProvider::new("hunyuan", "hunyuan-pro", Some(base_url), "bad-key");

        let req = ChatRequest {
            messages: vec![user_msg("test")],
            tools: vec![],
            temperature: None,
            max_tokens: None,
            stream: false,
        };

        let err = provider.chat(req).await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("认证失败"), "got: {msg}");
        assert!(msg.contains("401"), "got: {msg}");
    }

    #[tokio::test]
    async fn test_hunyuan_embed_unsupported() {
        let provider = HunyuanProvider::new("hunyuan", "hunyuan-pro", None, "sk-test");
        let err = provider.embed(&["test".into()]).await.unwrap_err();
        assert!(err.to_string().contains("not support"));
    }

    #[tokio::test]
    async fn test_hunyuan_error_generic_format() {
        // Fallback: when error doesn't match Hunyuan wrapper, use raw text
        let app = Router::new().route(
            "/chat/completions",
            post(|| async {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "internal error",
                )
            }),
        );

        let base_url = start_mock(app).await;
        let provider = HunyuanProvider::new("hunyuan", "hunyuan-pro", Some(base_url), "sk-test");

        let req = ChatRequest {
            messages: vec![user_msg("test")],
            tools: vec![],
            temperature: None,
            max_tokens: None,
            stream: false,
        };

        let err = provider.chat(req).await.unwrap_err();
        assert!(err.to_string().contains("500"), "got: {err}");
    }

    // ── Unit tests (no wire) ──

    #[test]
    fn test_hunyuan_provider_creation() {
        let p = HunyuanProvider::new("cn-hunyuan", "hunyuan-pro", None, "sk-abc");
        assert_eq!(p.name(), "cn-hunyuan");
        assert_eq!(p.model(), "hunyuan-pro");
        let caps = p.capabilities();
        assert!(caps.chat);
        assert!(caps.stream);
        assert!(caps.function_calling);
        assert!(!caps.embeddings); // Hunyuan doesn't support public embeddings
    }
}
