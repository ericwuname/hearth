//! llm-local: Local LLM providers (Ollama native protocol + vLLM OpenAI-compatible).
//!
//! ## Providers
//! - `OllamaProvider` — speaks Ollama's native `/api/chat` and `/api/embeddings` wire format.
//! - `VllmProvider` — thin wrapper that reuses `OpenAiProvider`-compatible logic with a
//!   custom base_url pointing to a vLLM server.
//!
//! ## Wire-format testing
//! All provider tests use mock HTTP servers (axum) that assert on request bodies and
//! serve controlled responses. Trait-level mocks are supplementary only.

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

// ────────────────────────────────────────────────────────────────────────────
// Ollama native wire types
// ────────────────────────────────────────────────────────────────────────────

/// Ollama `/api/chat` request body.
#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OllamaTool>>,
}

#[derive(Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OllamaToolCall>>,
}

#[derive(Serialize)]
struct OllamaToolCall {
    function: OllamaFunctionCall,
}

#[derive(Serialize)]
struct OllamaFunctionCall {
    name: String,
    arguments: Value,
}

#[derive(Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
}

/// Ollama `/api/chat` response (non-streaming).
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct OllamaChatResponse {
    message: OllamaResponseMessage,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    total_duration: Option<u64>,
    #[serde(default)]
    eval_count: Option<u32>,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct OllamaResponseMessage {
    #[serde(default)]
    role: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    tool_calls: Option<Vec<OllamaResponseToolCall>>,
}

#[derive(Deserialize, Debug)]
struct OllamaResponseToolCall {
    function: OllamaResponseFunction,
}

#[derive(Deserialize, Debug)]
struct OllamaResponseFunction {
    name: String,
    arguments: Value,
}

/// Ollama SSE stream chunk.
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct OllamaStreamChunk {
    #[serde(default)]
    message: OllamaStreamMessage,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    total_duration: Option<u64>,
    #[serde(default)]
    eval_count: Option<u32>,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
}

#[derive(Deserialize, Debug, Default)]
#[allow(dead_code)]
struct OllamaStreamMessage {
    #[serde(default)]
    role: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    tool_calls: Option<Vec<OllamaResponseToolCall>>,
}

/// Ollama `/api/embeddings` request.
#[derive(Serialize)]
struct OllamaEmbeddingRequest {
    model: String,
    input: Vec<String>,
}

/// Ollama `/api/embeddings` response.
#[derive(Deserialize, Debug)]
struct OllamaEmbeddingResponse {
    embeddings: Vec<Vec<f32>>,
}

/// Ollama tool definition in request.
#[derive(Serialize)]
struct OllamaTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OllamaFunctionDef,
}

#[derive(Serialize)]
struct OllamaFunctionDef {
    name: String,
    description: String,
    parameters: Value,
}

// ────────────────────────────────────────────────────────────────────────────
// SSE parsing (shared between Ollama and vLLM)
// ────────────────────────────────────────────────────────────────────────────

/// Parse a single SSE `data: ...` line into a StreamEvent for Ollama format.
fn parse_ollama_sse_line(line: &str) -> Option<Result<StreamEvent>> {
    let data = line.strip_prefix("data: ")?.trim();
    if data == "[DONE]" {
        return Some(Ok(StreamEvent::Finish {
            finish_reason: Some("stop".into()),
            usage: None,
        }));
    }
    let chunk: OllamaStreamChunk = serde_json::from_str(data).ok()?;

    // Emit content token
    if !chunk.message.content.is_empty() {
        return Some(Ok(StreamEvent::Token(chunk.message.content)));
    }

    // Emit tool call deltas
    if let Some(tool_calls) = &chunk.message.tool_calls {
        if let Some(tc) = tool_calls.iter().next() {
            return Some(Ok(StreamEvent::ToolCallDelta {
                call_id: String::new(), // Ollama doesn't provide call_id in stream
                name: Some(tc.function.name.clone()),
                args_delta: serde_json::to_string(&tc.function.arguments).unwrap_or_default(),
            }));
        }
    }

    // Finish
    if chunk.done {
        return Some(Ok(StreamEvent::Finish {
            finish_reason: Some("stop".into()),
            usage: chunk.eval_count.map(|completion_tokens| Usage {
                prompt_tokens: chunk.prompt_eval_count.unwrap_or(0),
                completion_tokens,
                total_tokens: chunk.prompt_eval_count.unwrap_or(0) + completion_tokens,
                prompt_cache_hit_tokens: None,
                prompt_cache_miss_tokens: None,
            }),
        }));
    }

    None
}

/// Generic SSE byte-stream consumer. Reads lines, calls `parse_fn` on each "data:" line.
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

// ────────────────────────────────────────────────────────────────────────────
// OllamaProvider
// ────────────────────────────────────────────────────────────────────────────

pub struct OllamaProvider {
    name: String,
    model: String,
    client: Client,
    base_url: String,
}

impl OllamaProvider {
    /// Create a new Ollama provider.
    ///
    /// `base_url` defaults to `http://localhost:11434` (standard Ollama default).
    pub fn new(
        name: impl Into<String>,
        model: impl Into<String>,
        base_url: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            model: model.into(),
            // LLM-1: default reqwest client has no timeout. Local models can
            // be slow, so use a generous cap rather than none at all.
            client: Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(600))
                .build()
                .unwrap_or_else(|_| Client::new()),
            // URL-1: trim trailing '/' to avoid double slashes in endpoints.
            base_url: base_url
                .unwrap_or_else(|| "http://localhost:11434".into())
                .trim_end_matches('/')
                .to_string(),
        }
    }

    fn build_chat_request(&self, req: ChatRequest, stream: bool) -> OllamaChatRequest {
        let messages: Vec<OllamaMessage> = req
            .messages
            .into_iter()
            .map(|m| {
                let role = match m.role {
                    agent_types::Role::System => "system",
                    agent_types::Role::User => "user",
                    agent_types::Role::Assistant => "assistant",
                    agent_types::Role::Tool => "tool",
                };
                let content = match &m.content {
                    agent_types::MessageContent::Text(t) => t.clone(),
                    agent_types::MessageContent::ToolCalls(tcs) => tcs
                        .iter()
                        .map(|tc| {
                            format!(
                                "call {}: {}",
                                tc.name,
                                serde_json::to_string(&tc.args).unwrap_or_default()
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                    agent_types::MessageContent::ToolResults(trs) => trs
                        .iter()
                        .map(|tr| tr.output.clone())
                        .collect::<Vec<_>>()
                        .join("\n"),
                };

                // Ollama doesn't support tool_calls in request messages natively,
                // but some versions do. We include them for compatibility.
                let tool_calls = match &m.content {
                    agent_types::MessageContent::ToolCalls(tcs) => Some(
                        tcs.iter()
                            .map(|tc| OllamaToolCall {
                                function: OllamaFunctionCall {
                                    name: tc.name.clone(),
                                    arguments: tc.args.clone(),
                                },
                            })
                            .collect(),
                    ),
                    _ => None,
                };

                OllamaMessage {
                    role: role.to_string(),
                    content,
                    tool_calls,
                }
            })
            .collect();

        let options = Some(OllamaOptions {
            temperature: req.temperature,
            num_predict: req.max_tokens,
        });

        let tools = if req.tools.is_empty() {
            None
        } else {
            Some(
                req.tools
                    .iter()
                    .map(|t| OllamaTool {
                        tool_type: "function".into(),
                        function: OllamaFunctionDef {
                            name: t.name.clone(),
                            description: t.description.clone(),
                            parameters: t.parameters.clone(),
                        },
                    })
                    .collect(),
            )
        };

        OllamaChatRequest {
            model: self.model.clone(),
            messages,
            stream,
            options,
            tools,
        }
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
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
            embeddings: true,
            max_context_tokens: Some(128_000),
        }
    }

    async fn chat(&self, req: ChatRequest) -> Result<GatewayChatResponse> {
        let body = self.build_chat_request(req, false);

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Ollama request failed: {e}"))?;

        let status = resp.status();
        let text = resp.text().await.map_err(|e| anyhow!("read body: {e}"))?;

        if !status.is_success() {
            return Err(anyhow!("Ollama error {status}: {text}"));
        }

        let parsed: OllamaChatResponse = serde_json::from_str(&text)
            .map_err(|e| anyhow!("parse Ollama response: {e}: {text}"))?;

        let tool_calls = parsed
            .message
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|tc| agent_types::ToolCall {
                call_id: String::new(),
                name: tc.function.name,
                args: tc.function.arguments,
            })
            .collect();

        let content = if parsed.message.content.is_empty() {
            None
        } else {
            Some(parsed.message.content)
        };

        Ok(GatewayChatResponse {
            content,
            tool_calls,
            finish_reason: if parsed.done {
                Some("stop".into())
            } else {
                None
            },
            reasoning_content: None,
            usage: parsed.eval_count.map(|completion_tokens| Usage {
                prompt_tokens: parsed.prompt_eval_count.unwrap_or(0),
                completion_tokens,
                total_tokens: parsed.prompt_eval_count.unwrap_or(0) + completion_tokens,
                prompt_cache_hit_tokens: None,
                prompt_cache_miss_tokens: None,
            }),
        })
    }

    fn stream(&self, req: ChatRequest) -> BoxStream<'static, Result<StreamEvent>> {
        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let body = self.build_chat_request(req, true);

        let (mut tx, rx) = mpsc::channel::<Result<StreamEvent>>(64);

        // STREAM-3: abort the producer task when the returned stream is dropped.
        let producer = tokio::spawn(async move {
            let resp = match client
                .post(format!("{}/api/chat", base_url))
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx
                        .send(Err(anyhow!("Ollama stream request failed: {e}")))
                        .await;
                    return;
                }
            };

            let status = resp.status();
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                let _ = tx
                    .send(Err(anyhow!("Ollama stream error {status}: {text}")))
                    .await;
                return;
            }

            consume_sse_stream(resp, &mut tx, parse_ollama_sse_line).await;
        });

        Box::pin(llm_gateway::AbortOnDropStream::new(
            rx,
            producer.abort_handle(),
        ))
    }

    async fn embed(&self, inputs: &[String]) -> Result<Vec<Embedding>> {
        let body = OllamaEmbeddingRequest {
            model: self.model.clone(),
            input: inputs.to_vec(),
        };

        let resp = self
            .client
            .post(format!("{}/api/embeddings", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Ollama embed request failed: {e}"))?;

        let status = resp.status();
        let text = resp.text().await.map_err(|e| anyhow!("read body: {e}"))?;

        if !status.is_success() {
            return Err(anyhow!("Ollama embed error {status}: {text}"));
        }

        let parsed: OllamaEmbeddingResponse =
            serde_json::from_str(&text).map_err(|e| anyhow!("parse embed: {e}"))?;

        Ok(parsed
            .embeddings
            .into_iter()
            .enumerate()
            .map(|(i, values)| Embedding { index: i, values })
            .collect())
    }
}

// ────────────────────────────────────────────────────────────────────────────
// VllmProvider (thin OpenAI-compatible wrapper)
// ────────────────────────────────────────────────────────────────────────────
//
// vLLM exposes an OpenAI-compatible `/v1/chat/completions` endpoint, so we can
// reuse the same wire types and SSE parsing from `llm-openai`. To avoid a hard
// dependency on `llm-openai`, we inline the minimal necessary types and
// SSE parsing here.
// ────────────────────────────────────────────────────────────────────────────

// ── Inlined OpenAI-compatible wire types (duplicated from llm-openai) ──

#[derive(Serialize)]
struct VllmRequest {
    model: String,
    messages: Vec<VllmMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<VllmTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    stream: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct VllmMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<VllmToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct VllmToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: VllmFunctionCall,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct VllmFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Serialize)]
struct VllmTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: VllmFunctionDef,
}

#[derive(Serialize)]
struct VllmFunctionDef {
    name: String,
    description: String,
    parameters: Value,
}

#[derive(Deserialize, Debug)]
struct VllmResponse {
    choices: Vec<VllmChoice>,
    usage: Option<VllmUsage>,
}

#[derive(Deserialize, Debug)]
struct VllmChoice {
    message: Option<VllmMessage>,
    delta: Option<VllmDelta>,
    finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
struct VllmDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<VllmToolCallDelta>>,
}

#[derive(Deserialize, Debug)]
struct VllmToolCallDelta {
    #[allow(dead_code)]
    index: u64,
    #[serde(default)]
    id: Option<String>,
    function: Option<VllmFunctionDelta>,
}

#[derive(Deserialize, Debug)]
struct VllmFunctionDelta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Deserialize, Debug)]
struct VllmUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Deserialize, Debug)]
struct VllmStreamChunk {
    choices: Vec<VllmChoice>,
}

#[derive(Serialize)]
struct VllmEmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct VllmEmbeddingResponse {
    data: Vec<VllmEmbeddingData>,
}

#[derive(Deserialize, Debug)]
struct VllmEmbeddingData {
    index: usize,
    embedding: Vec<f32>,
}

/// Parse SSE for vLLM (OpenAI-compatible format).
fn parse_vllm_sse_line(line: &str) -> Option<Result<StreamEvent>> {
    let data = line.strip_prefix("data: ")?.trim();
    if data == "[DONE]" {
        return Some(Ok(StreamEvent::Finish {
            finish_reason: Some("stop".into()),
            usage: None,
        }));
    }
    let chunk: VllmStreamChunk = serde_json::from_str(data).ok()?;

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

// ── VllmProvider ──

pub struct VllmProvider {
    name: String,
    model: String,
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

impl VllmProvider {
    /// Create a new vLLM provider.
    ///
    /// `base_url` should point to the vLLM OpenAI-compatible endpoint,
    /// e.g. `http://localhost:8000/v1`. If an `api_key` is provided, it
    /// will be sent as a Bearer token; otherwise no auth header is sent.
    pub fn new(
        name: impl Into<String>,
        model: impl Into<String>,
        base_url: Option<String>,
        api_key: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            model: model.into(),
            // LLM-1: default reqwest client has no timeout.
            client: Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(600))
                .build()
                .unwrap_or_else(|_| Client::new()),
            // URL-1: trim trailing '/' to avoid double slashes in endpoints.
            base_url: base_url
                .unwrap_or_else(|| "http://localhost:8000/v1".into())
                .trim_end_matches('/')
                .to_string(),
            api_key,
        }
    }

    fn build_request(&self, req: ChatRequest, stream: bool) -> VllmRequest {
        let messages: Vec<VllmMessage> = req
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
                            let v_tcs: Vec<VllmToolCall> = tcs
                                .iter()
                                .map(|tc| VllmToolCall {
                                    id: tc.call_id.clone(),
                                    call_type: "function".into(),
                                    function: VllmFunctionCall {
                                        name: tc.name.clone(),
                                        arguments: serde_json::to_string(&tc.args)
                                            .unwrap_or_default(),
                                    },
                                })
                                .collect();
                            ("assistant".to_string(), None, Some(v_tcs), None)
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

                VllmMessage {
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
                    .map(|t| VllmTool {
                        tool_type: "function".into(),
                        function: VllmFunctionDef {
                            name: t.name.clone(),
                            description: t.description.clone(),
                            parameters: t.parameters.clone(),
                        },
                    })
                    .collect(),
            )
        };

        VllmRequest {
            model: self.model.clone(),
            messages,
            tools,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            stream,
        }
    }
}

#[async_trait]
impl LlmProvider for VllmProvider {
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
            embeddings: true,
            max_context_tokens: Some(128_000),
        }
    }

    async fn chat(&self, req: ChatRequest) -> Result<GatewayChatResponse> {
        let body = self.build_request(req, false);

        let mut builder = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Content-Type", "application/json");

        if let Some(ref key) = self.api_key {
            builder = builder.header("Authorization", format!("Bearer {key}"));
        }

        let resp = builder
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("vLLM request failed: {e}"))?;

        let status = resp.status();
        let text = resp.text().await.map_err(|e| anyhow!("read body: {e}"))?;

        if !status.is_success() {
            return Err(anyhow!("vLLM error {status}: {text}"));
        }

        let parsed: VllmResponse =
            serde_json::from_str(&text).map_err(|e| anyhow!("parse vLLM response: {e}: {text}"))?;

        let choice = parsed
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("no choices in response"))?;

        let msg = choice.message.unwrap_or(VllmMessage {
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

        // STREAM-4: abort the producer task when the returned stream is dropped.
        let producer = tokio::spawn(async move {
            let mut builder = client
                .post(format!("{}/chat/completions", base_url))
                .header("Content-Type", "application/json");

            if let Some(ref key) = api_key {
                builder = builder.header("Authorization", format!("Bearer {key}"));
            }

            let resp = match builder.json(&body).send().await {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx
                        .send(Err(anyhow!("vLLM stream request failed: {e}")))
                        .await;
                    return;
                }
            };

            let status = resp.status();
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                let _ = tx
                    .send(Err(anyhow!("vLLM stream error {status}: {text}")))
                    .await;
                return;
            }

            consume_sse_stream(resp, &mut tx, parse_vllm_sse_line).await;
        });

        Box::pin(llm_gateway::AbortOnDropStream::new(
            rx,
            producer.abort_handle(),
        ))
    }

    async fn embed(&self, inputs: &[String]) -> Result<Vec<Embedding>> {
        let body = VllmEmbeddingRequest {
            model: self.model.clone(),
            input: inputs.to_vec(),
        };

        let mut builder = self
            .client
            .post(format!("{}/embeddings", self.base_url))
            .header("Content-Type", "application/json");

        if let Some(ref key) = self.api_key {
            builder = builder.header("Authorization", format!("Bearer {key}"));
        }

        let resp = builder
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("vLLM embed request failed: {e}"))?;

        let status = resp.status();
        let text = resp.text().await.map_err(|e| anyhow!("read body: {e}"))?;

        if !status.is_success() {
            return Err(anyhow!("vLLM embed error {status}: {text}"));
        }

        let parsed: VllmEmbeddingResponse =
            serde_json::from_str(&text).map_err(|e| anyhow!("parse embed: {e}"))?;

        Ok(parsed
            .data
            .into_iter()
            .map(|d| Embedding {
                index: d.index,
                values: d.embedding,
            })
            .collect())
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
        // Give the server a moment to start
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        url
    }

    // ── Ollama wire tests ──

    #[tokio::test]
    async fn test_ollama_chat_wire_format() {
        // Record the request body the mock receives
        let received = Arc::new(Mutex::new(None::<serde_json::Value>));
        let r = received.clone();

        let app = Router::new().route(
            "/api/chat",
            post(move |Json(body): Json<serde_json::Value>| {
                let r = r.clone();
                async move {
                    *r.lock().await = Some(body);
                    Json(json!({
                        "model": "llama3.2",
                        "created_at": "2025-01-01T00:00:00Z",
                        "message": {
                            "role": "assistant",
                            "content": "Hello from Ollama!"
                        },
                        "done": true,
                        "total_duration": 1_000_000_000,
                        "eval_count": 15,
                        "prompt_eval_count": 10
                    }))
                }
            }),
        );

        let base_url = start_mock(app).await;
        let provider = OllamaProvider::new("ollama", "llama3.2", Some(base_url));

        let req = ChatRequest {
            messages: vec![user_msg("Hi")],
            tools: vec![],
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: false,
        };

        let resp = provider.chat(req).await.unwrap();
        assert_eq!(resp.content.unwrap(), "Hello from Ollama!");
        assert_eq!(resp.finish_reason.unwrap(), "stop");
        assert_eq!(resp.usage.unwrap().completion_tokens, 15);

        // Assert wire format
        let sent = received.lock().await.take().unwrap();
        assert_eq!(sent["model"], "llama3.2");
        assert!(!sent["stream"].as_bool().unwrap());
        assert_eq!(sent["messages"][0]["role"], "user");
        assert_eq!(sent["messages"][0]["content"], "Hi");
    }

    #[tokio::test]
    async fn test_ollama_chat_with_tools_wire_format() {
        let received = Arc::new(Mutex::new(None::<serde_json::Value>));
        let r = received.clone();

        let app = Router::new().route(
            "/api/chat",
            post(move |Json(body): Json<serde_json::Value>| {
                let r = r.clone();
                async move {
                    *r.lock().await = Some(body);
                    Json(json!({
                        "model": "llama3.2",
                        "message": {
                            "role": "assistant",
                            "content": "",
                            "tool_calls": [{
                                "function": {
                                    "name": "bash",
                                    "arguments": {"cmd": "ls"}
                                }
                            }]
                        },
                        "done": true,
                        "eval_count": 20,
                        "prompt_eval_count": 5
                    }))
                }
            }),
        );

        let base_url = start_mock(app).await;
        let provider = OllamaProvider::new("ollama", "llama3.2", Some(base_url));

        let req = ChatRequest {
            messages: vec![user_msg("list files")],
            tools: vec![ToolSchema {
                name: "bash".into(),
                description: "run command".into(),
                parameters: json!({"type": "object", "properties": {"cmd": {"type": "string"}}}),
            }],
            temperature: None,
            max_tokens: None,
            stream: false,
        };

        let resp = provider.chat(req).await.unwrap();
        assert_eq!(resp.tool_calls.len(), 1);
        assert_eq!(resp.tool_calls[0].name, "bash");
        assert_eq!(resp.tool_calls[0].args["cmd"], "ls");

        // Assert wire: tools field present
        let sent = received.lock().await.take().unwrap();
        let tools = sent["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["function"]["name"], "bash");
    }

    #[tokio::test]
    async fn test_ollama_embed_wire_format() {
        let received = Arc::new(Mutex::new(None::<serde_json::Value>));
        let r = received.clone();

        let app = Router::new().route(
            "/api/embeddings",
            post(move |Json(body): Json<serde_json::Value>| {
                let r = r.clone();
                async move {
                    *r.lock().await = Some(body);
                    Json(json!({
                        "embeddings": [[0.1, 0.2, 0.3], [0.4, 0.5, 0.6]]
                    }))
                }
            }),
        );

        let base_url = start_mock(app).await;
        let provider = OllamaProvider::new("ollama", "nomic-embed-text", Some(base_url));

        let embeds = provider
            .embed(&["hello".into(), "world".into()])
            .await
            .unwrap();
        assert_eq!(embeds.len(), 2);
        assert_eq!(embeds[0].values, vec![0.1, 0.2, 0.3]);
        assert_eq!(embeds[1].values, vec![0.4, 0.5, 0.6]);

        let sent = received.lock().await.take().unwrap();
        assert_eq!(sent["model"], "nomic-embed-text");
        assert_eq!(sent["input"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_ollama_chat_error_status() {
        let app = Router::new().route(
            "/api/chat",
            post(|| async {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "model not found",
                )
            }),
        );

        let base_url = start_mock(app).await;
        let provider = OllamaProvider::new("ollama", "nonexistent", Some(base_url));

        let req = ChatRequest {
            messages: vec![user_msg("Hi")],
            tools: vec![],
            temperature: None,
            max_tokens: None,
            stream: false,
        };

        let err = provider.chat(req).await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Ollama error 500"), "got: {msg}");
    }

    // ── vLLM wire tests ──

    #[tokio::test]
    async fn test_vllm_chat_wire_format() {
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
                                "content": "Hello from vLLM!"
                            },
                            "finish_reason": "stop"
                        }],
                        "usage": {
                            "prompt_tokens": 10,
                            "completion_tokens": 5,
                            "total_tokens": 15
                        }
                    }))
                }
            }),
        );

        let base_url = start_mock(app).await;
        let provider = VllmProvider::new("vllm", "mistral-7b", Some(base_url), None);

        let req = ChatRequest {
            messages: vec![user_msg("Hello")],
            tools: vec![],
            temperature: Some(0.3),
            max_tokens: Some(512),
            stream: false,
        };

        let resp = provider.chat(req).await.unwrap();
        assert_eq!(resp.content.unwrap(), "Hello from vLLM!");
        assert_eq!(resp.usage.unwrap().total_tokens, 15);

        // Assert wire format
        let sent = received.lock().await.take().unwrap();
        assert_eq!(sent["model"], "mistral-7b");
        assert!(!sent["stream"].as_bool().unwrap());
        assert_eq!(sent["temperature"], 0.3);
    }

    #[tokio::test]
    async fn test_vllm_chat_with_auth() {
        let received = Arc::new(Mutex::new(None::<String>));
        let r = received.clone();

        let app = Router::new().route(
            "/chat/completions",
            post(
                move |headers: axum::http::HeaderMap, Json(_body): Json<serde_json::Value>| {
                    let r = r.clone();
                    async move {
                        let auth = headers
                            .get("Authorization")
                            .and_then(|v| v.to_str().ok())
                            .map(|s| s.to_string());
                        *r.lock().await = auth;
                        Json(json!({
                            "choices": [{
                                "message": {"role": "assistant", "content": "ok"},
                                "finish_reason": "stop"
                            }]
                        }))
                    }
                },
            ),
        );

        let base_url = start_mock(app).await;
        let provider = VllmProvider::new(
            "vllm",
            "mistral-7b",
            Some(base_url),
            Some("secret-key".into()),
        );

        let req = ChatRequest {
            messages: vec![user_msg("test")],
            tools: vec![],
            temperature: None,
            max_tokens: None,
            stream: false,
        };

        let resp = provider.chat(req).await.unwrap();
        assert_eq!(resp.content.unwrap(), "ok");

        let auth = received.lock().await.take();
        assert_eq!(auth.as_deref(), Some("Bearer secret-key"));
    }

    #[tokio::test]
    async fn test_vllm_embed_wire_format() {
        let received = Arc::new(Mutex::new(None::<serde_json::Value>));
        let r = received.clone();

        let app = Router::new().route(
            "/embeddings",
            post(move |Json(body): Json<serde_json::Value>| {
                let r = r.clone();
                async move {
                    *r.lock().await = Some(body);
                    Json(json!({
                        "data": [
                            {"index": 0, "embedding": [1.0, 2.0]},
                            {"index": 1, "embedding": [3.0, 4.0]}
                        ]
                    }))
                }
            }),
        );

        let base_url = start_mock(app).await;
        let provider = VllmProvider::new("vllm", "mistral-7b", Some(base_url), None);

        let embeds = provider.embed(&["a".into(), "b".into()]).await.unwrap();
        assert_eq!(embeds.len(), 2);
        assert_eq!(embeds[0].values, vec![1.0, 2.0]);
        assert_eq!(embeds[1].values, vec![3.0, 4.0]);

        let sent = received.lock().await.take().unwrap();
        assert_eq!(sent["input"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_vllm_chat_error_status() {
        let app = Router::new().route(
            "/chat/completions",
            post(|| async { (axum::http::StatusCode::BAD_GATEWAY, "upstream error") }),
        );

        let base_url = start_mock(app).await;
        let provider = VllmProvider::new("vllm", "mistral-7b", Some(base_url), None);

        let req = ChatRequest {
            messages: vec![user_msg("test")],
            tools: vec![],
            temperature: None,
            max_tokens: None,
            stream: false,
        };

        let err = provider.chat(req).await.unwrap_err();
        assert!(err.to_string().contains("vLLM error 502"), "got: {err}");
    }

    // ── Unit tests (no wire) ──

    #[test]
    fn test_ollama_provider_creation() {
        let p = OllamaProvider::new("local-ollama", "llama3.2", None);
        assert_eq!(p.name(), "local-ollama");
        assert_eq!(p.model(), "llama3.2");
        let caps = p.capabilities();
        assert!(caps.chat);
        assert!(caps.stream);
        assert!(caps.function_calling);
        assert!(caps.embeddings);
    }

    #[test]
    fn test_vllm_provider_creation() {
        let p = VllmProvider::new("local-vllm", "mistral-7b", None, Some("key".into()));
        assert_eq!(p.name(), "local-vllm");
        assert_eq!(p.model(), "mistral-7b");
        let caps = p.capabilities();
        assert!(caps.chat);
        assert!(caps.stream);
        assert!(caps.function_calling);
    }

    #[test]
    fn test_vllm_provider_no_auth() {
        let p = VllmProvider::new("local-vllm", "mistral-7b", None, None);
        assert_eq!(p.name(), "local-vllm");
    }
}
