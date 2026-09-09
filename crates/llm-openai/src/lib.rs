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

// ── OpenAI wire types ──

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAiTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenAiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    /// v24-post: deepseek thinking mode 回传必需。
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_content: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct OpenAiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OpenAiFunctionCall,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct OpenAiFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Serialize)]
struct OpenAiTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAiFunctionDef,
}

#[derive(Serialize)]
struct OpenAiFunctionDef {
    name: String,
    description: String,
    parameters: Value,
}

#[derive(Deserialize, Debug)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Deserialize, Debug)]
struct OpenAiChoice {
    message: Option<OpenAiMessage>,
    delta: Option<OpenAiDelta>,
    finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
struct OpenAiDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAiToolCallDelta>>,
}

#[derive(Deserialize, Debug)]
struct OpenAiToolCallDelta {
    #[allow(dead_code)]
    index: u64,
    #[serde(default)]
    id: Option<String>,
    function: Option<OpenAiFunctionDelta>,
}

#[derive(Deserialize, Debug)]
struct OpenAiFunctionDelta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Deserialize, Debug)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
    /// P1-1 (v0.2.4): DeepSeek 系扩展——缓存命中/未命中（缺失时 None，不误报 0）。
    #[serde(default)]
    prompt_cache_hit_tokens: Option<u32>,
    #[serde(default)]
    prompt_cache_miss_tokens: Option<u32>,
}

#[derive(Deserialize, Debug)]
struct OpenAiStreamChunk {
    choices: Vec<OpenAiChoice>,
}

#[derive(Serialize)]
struct OpenAiEmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct OpenAiEmbeddingResponse {
    data: Vec<OpenAiEmbeddingData>,
}

#[derive(Deserialize, Debug)]
struct OpenAiEmbeddingData {
    index: usize,
    embedding: Vec<f32>,
}

// ── SSE stream wrapper ──

fn parse_sse_line(line: &str) -> Option<Result<StreamEvent>> {
    let data = {
        let d = line.strip_prefix("data: ")?;
        d.trim()
    };

    if data == "[DONE]" {
        return Some(Ok(StreamEvent::Finish {
            finish_reason: Some("stop".into()),
            usage: None,
        }));
    }

    let chunk: OpenAiStreamChunk = match serde_json::from_str(data) {
        Ok(c) => c,
        Err(_) => return None,
    };

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

// ── Provider implementation ──

/// v12.4: process-wide throttle for outbound LLM calls.
///
/// The agent fans out sub-agents that all talk to the same provider account, so
/// a task like "change PAGE_SIZE" produced a burst of concurrent requests and
/// ZhiPu answered HTTP 429 code 1302 ("您的账户已达到速率限制"). Retrying alone does
/// not help when the burst itself is the problem: cap concurrency and enforce a
/// minimum gap between request starts.
///
/// Tunable via `LLM_MAX_CONCURRENCY` (default 2) and `LLM_MIN_INTERVAL_MS`
/// (default 700).
struct RateGate {
    permits: tokio::sync::Semaphore,
    last_start: tokio::sync::Mutex<Option<std::time::Instant>>,
    min_interval: std::time::Duration,
}

impl RateGate {
    fn global() -> &'static RateGate {
        static GATE: std::sync::OnceLock<RateGate> = std::sync::OnceLock::new();
        GATE.get_or_init(|| {
            let concurrency = std::env::var("LLM_MAX_CONCURRENCY")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .filter(|v| *v > 0)
                .unwrap_or(2);
            let interval_ms = std::env::var("LLM_MIN_INTERVAL_MS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(700);
            tracing::info!(concurrency, interval_ms, "LLM rate gate initialised");
            RateGate {
                permits: tokio::sync::Semaphore::new(concurrency),
                last_start: tokio::sync::Mutex::new(None),
                min_interval: std::time::Duration::from_millis(interval_ms),
            }
        })
    }

    /// Acquire a slot. The returned permit must be held for the request's
    /// lifetime; dropping it releases the slot.
    async fn enter(&'static self) -> tokio::sync::SemaphorePermit<'static> {
        let permit = self
            .permits
            .acquire()
            .await
            .expect("rate gate semaphore is never closed");
        let mut last = self.last_start.lock().await;
        if let Some(prev) = *last {
            let elapsed = prev.elapsed();
            if elapsed < self.min_interval {
                tokio::time::sleep(self.min_interval - elapsed).await;
            }
        }
        *last = Some(std::time::Instant::now());
        permit
    }
}

pub struct OpenAiProvider {
    name: String,
    model: String,
    client: Client,
    base_url: String,
    api_key: String,
}

impl OpenAiProvider {
    pub fn new(
        name: impl Into<String>,
        model: impl Into<String>,
        base_url: Option<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            model: model.into(),
            // LLM-1: Client::new() has NO timeout by default — a hung server
            // would block the agent loop forever.
            // R7 (v0.1.4): 300s 挂起超时是"deepseek 卡死 62s 撞 60s cap"的一半根因
            // （服务端挂起时客户端死等 5 分钟）。改为 30s——正常 LLM 响应远快于此，
            // 挂起快速失败交给 loop 层重试（2s/4s 退避 + 60s cap）。
            client: Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| Client::new()),
            // URL-1: trim trailing '/' so "{base}/chat/completions" never
            // produces a double slash.
            base_url: base_url
                .unwrap_or_else(|| "https://api.openai.com/v1".into())
                .trim_end_matches('/')
                .to_string(),
            api_key: api_key.into(),
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
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
        let body = self.build_request(req, false)?;
        tracing::info!(provider = %self.name, model = %body.model, msg_count = body.messages.len(), "chat request");
        if body.messages.len() <= 3 {
            tracing::info!(req_body = ?serde_json::to_string(&body).unwrap_or_default(), "chat request body (<=3 msgs)");
        }
        // Hold the throttle slot for the whole round-trip.
        let _slot = RateGate::global().enter().await;
        // B1 (v0.1.2): read body 失败 provider 内重试——R9-B2 曾放宽到 3 次，
        // 但 Tier3 遥测 B04/B06 实测（handoff §6 补充 1）：3×30s≈90s/次 chat，
        // 与 loop 层 4×/120s 嵌套叠加穿透 TOTAL_RETRY_CAP（200s 外部 timeout 才杀）。
        // T2 修复（删嵌套优先——守门员补充 2）: 重试 3→1 次（偶发传输中断值得 1 次
        // 快速重试），第 2 次连续失败直接 Fatal 上抛（同一畸形流重试不会自愈）——
        // 单次 chat 上限从 ~90s 降到 ~60s，B04/B06 类故障 ~60s 快速失败进 FallbackChain。
        let mut body_err: Option<anyhow::Error> = None;
        let mut body_fail_count: u32 = 0;
        for attempt in 0..2u32 {
            match self
                .client
                .post(format!("{}/chat/completions", self.base_url))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
            {
                Ok(resp) => {
                    let status = resp.status();
                    let text = match resp.text().await {
                        Ok(t) => t,
                        Err(e) => {
                            body_fail_count += 1;
                            body_err = Some(anyhow!("read body: {e}"));
                            tracing::warn!(
                                provider = %self.name,
                                attempt,
                                error = %e,
                                "read body failed — retrying with backoff"
                            );
                            // T3 (handoff §6): 连续 2 次 read-body 失败 → Fatal 快速失败
                            // （同一畸形/断开的流重试不会自愈—— unlike 偶发传输中断）。
                            if body_fail_count >= 2 {
                                return Err(llm_gateway::LlmError::Fatal(format!(
                                    "read body failed twice consecutively (stream likely broken): {e} — not retrying same provider"
                                ))
                                .into());
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(
                                500 * (1 << attempt),
                            ))
                            .await;
                            continue;
                        }
                    };

                    if !status.is_success() {
                        // v24-post (backend-intelligence): 失败路径不打 req_body 全文——6000 字符
                        // 含完整 system prompt 造成日志噪音；错误摘要（text）已含具体原因。
                        // 需要完整 body 调试时用 RUST_LOG=llm_openai=debug。
                        tracing::debug!(req_body = %serde_json::to_string(&body).unwrap_or_default(), "rejected request body (debug)");
                        tracing::warn!(
                            status = %status,
                            "chat NON-success — {text}"
                        );
                        // B2 (v0.1.2 错误分类映射表): 429 速率限制=瞬时（稍等重试有意义）；
                        // 其他 4xx=参数问题（replan，不重试）；5xx=供应商内部错误（give_up）。
                        let code = status.as_u16();
                        return if code == 429 {
                            Err(
                                llm_gateway::LlmError::Transient(format!("HTTP 429: {text}"))
                                    .into(),
                            )
                        } else if code >= 500 {
                            Err(llm_gateway::LlmError::Fatal(format!("HTTP {code}: {text}")).into())
                        } else {
                            Err(llm_gateway::LlmError::Param(format!("HTTP {code}: {text}")).into())
                        };
                    }

                    let parsed: OpenAiResponse = serde_json::from_str(&text)
                        .map_err(|e| anyhow!("parse response: {e}: {text}"))?;

                    let choice = parsed
                        .choices
                        .into_iter()
                        .next()
                        .ok_or_else(|| anyhow!("no choices in response"))?;

                    let msg = choice.message.unwrap_or(OpenAiMessage {
                        role: "assistant".into(),
                        content: None,
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    });

                    let tool_calls = msg
                        .tool_calls
                        .unwrap_or_default()
                        .into_iter()
                        .map(|tc| agent_types::ToolCall {
                            call_id: tc.id,
                            name: tc.function.name.clone(),
                            args: Self::parse_tool_args(&tc.function.arguments, &tc.function.name),
                        })
                        .collect();

                    return Ok(GatewayChatResponse {
                        content: msg.content,
                        tool_calls,
                        finish_reason: choice.finish_reason,
                        reasoning_content: msg.reasoning_content,
                        usage: parsed.usage.map(|u| Usage {
                            prompt_tokens: u.prompt_tokens,
                            completion_tokens: u.completion_tokens,
                            total_tokens: u.total_tokens,
                            prompt_cache_hit_tokens: u.prompt_cache_hit_tokens,
                            prompt_cache_miss_tokens: u.prompt_cache_miss_tokens,
                        }),
                    });
                }
                Err(e) => {
                    body_err = Some(anyhow!("OpenAI request failed: {e}"));
                    tracing::warn!(
                        provider = %self.name,
                        attempt,
                        error = %e,
                        "send request failed — retrying with backoff"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(500 * (1 << attempt)))
                        .await;
                }
            }
        }
        // 多次都失败：区分"端点死"（connection refused/unreachable/DNS——Fatal，重试无意义）
        // 与"瞬时抖动"（timeout/429——Transient，可重试）。T2 (v0.2.3)。
        let detail = body_err
            .map(|e| format!("{e:#}"))
            .unwrap_or_else(|| "unknown transport error".into());
        let low = detail.to_lowercase();
        let fatal = low.contains("connection refused")
            || low.contains("unreachable")
            || low.contains("name or service not known")
            || low.contains("dns");
        if fatal {
            Err(llm_gateway::LlmError::Fatal(format!("通道不可达（端点死）: {detail}")).into())
        } else {
            Err(llm_gateway::LlmError::Transient(detail).into())
        }
    }

    fn stream(&self, req: ChatRequest) -> BoxStream<'static, Result<StreamEvent>> {
        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let api_key = self.api_key.clone();

        let body = match self.build_request(req, true) {
            Ok(b) => b,
            Err(e) => return Box::pin(futures::stream::once(async move { Err(e) })),
        };

        let (mut tx, rx) = mpsc::channel::<Result<StreamEvent>>(64);

        // STREAM-1: keep the AbortHandle and tie it to the returned stream so
        // dropping the stream (session cancel) aborts this producer task
        // instead of leaving it running until the next failed send.
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
                    let _ = tx.send(Err(anyhow!("stream request failed: {e}"))).await;
                    return;
                }
            };

            let status = resp.status();
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                let _ = tx.send(Err(anyhow!("OpenAI error {status}: {text}"))).await;
                return;
            }

            let mut byte_stream = Box::pin(resp.bytes_stream());
            let mut buffer = String::new();

            loop {
                match byte_stream.as_mut().next().await {
                    Some(Ok(bytes)) => {
                        let chunk_str = String::from_utf8_lossy(&bytes);
                        buffer.push_str(&chunk_str);
                        while let Some(pos) = buffer.find('\n') {
                            let line = buffer[..pos].trim().to_string();
                            buffer = buffer[pos + 1..].to_string();
                            if let Some(event) = parse_sse_line(&line) {
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
        });

        Box::pin(llm_gateway::AbortOnDropStream::new(
            rx,
            producer.abort_handle(),
        ))
    }

    async fn embed(&self, inputs: &[String]) -> Result<Vec<Embedding>> {
        let body = OpenAiEmbeddingRequest {
            model: "text-embedding-ada-002".into(),
            input: inputs.to_vec(),
        };

        let resp = self
            .client
            .post(format!("{}/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("embed request failed: {e}"))?;

        let status = resp.status();
        let text = resp.text().await.map_err(|e| anyhow!("read body: {e}"))?;

        if !status.is_success() {
            return Err(anyhow!("OpenAI embed error {status}: {text}"));
        }

        let parsed: OpenAiEmbeddingResponse =
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

impl OpenAiProvider {
    /// R1-A (v0.1.1 用户真机): 工具参数解析——LLM 流式吐超长参数（如整段 HTML）被截断时，
    /// JSON 不完整（EOF）。三层处理：
    /// ① 正常 parse → 直接返回；
    /// ② 以 `{` 开头但缺尾 `}`（截断特征）→ 补 `}` 重试（治标）；
    /// ③ 仍失败 → 保留 raw string + **醒目 WARN（含截断长度）**（不静默降级）。
    fn parse_tool_args(raw: &str, name: &str) -> serde_json::Value {
        match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(e) => {
                let trimmed = raw.trim();
                if trimmed.starts_with('{') && !trimmed.ends_with('}') {
                    let repaired = format!("{trimmed}}}");
                    if let Ok(v) = serde_json::from_str(&repaired) {
                        tracing::warn!(
                            "tool call '{name}' JSON truncated at {} chars — repaired by appending '}}'",
                            raw.len()
                        );
                        return v;
                    }
                }
                tracing::warn!(
                    "tool call '{name}' has non-JSON arguments ({e}) at {} chars; passing as raw string",
                    raw.len()
                );
                serde_json::Value::String(raw.to_string())
            }
        }
    }

    fn build_request(&self, req: ChatRequest, stream: bool) -> Result<OpenAiRequest> {
        let messages: Vec<OpenAiMessage> = req
            .messages
            .into_iter()
            .map(|m| {
                let (role, content, tool_calls, tool_call_id, reasoning) = match m.role {
                    agent_types::Role::System => {
                        ("system".to_string(), m.content_to_text(), None, None, None)
                    }
                    agent_types::Role::User => {
                        ("user".to_string(), m.content_to_text(), None, None, None)
                    }
                    agent_types::Role::Assistant => match &m.content {
                        agent_types::MessageContent::Text(t) => (
                            "assistant".to_string(),
                            Some(t.clone()),
                            None,
                            None,
                            m.reasoning_content.clone(),
                        ),
                        agent_types::MessageContent::ToolCalls(tcs) => {
                            let oai_tcs: Vec<OpenAiToolCall> = tcs
                                .iter()
                                .map(|tc| OpenAiToolCall {
                                    id: tc.call_id.clone(),
                                    call_type: "function".into(),
                                    function: OpenAiFunctionCall {
                                        name: tc.name.clone(),
                                        arguments: serde_json::to_string(&tc.args)
                                            .unwrap_or_default(),
                                    },
                                })
                                .collect();
                            ("assistant".to_string(), None, Some(oai_tcs), None, None)
                        }
                        agent_types::MessageContent::ToolResults(_) => (
                            "assistant".to_string(),
                            Some("[tool results omitted]".into()),
                            None,
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
                        ("tool".to_string(), Some(output), None, Some(cid), None)
                    }
                };

                OpenAiMessage {
                    role,
                    content,
                    tool_calls,
                    tool_call_id,
                    reasoning_content: reasoning,
                }
            })
            .collect();

        // v12.4: last-resort guards for OpenAI-compatible backends that are
        // stricter than OpenAI itself (ZhiPu GLM answers HTTP 400 code 1214
        // "messages 参数非法"). Two shapes were observed to kill live sessions:
        //
        //   1. an assistant turn whose content is blank ("\n") and which carries
        //      no tool_calls — the model emitted nothing, yet we echoed it back;
        //   2. a conversation with no user turn at all.
        let mut messages: Vec<OpenAiMessage> = messages
            .into_iter()
            .filter(|m| {
                !(m.role == "assistant"
                    && m.tool_calls.is_none()
                    && m.content.as_deref().unwrap_or("").trim().is_empty())
            })
            .collect();

        if !messages.iter().any(|m| m.role == "user") {
            tracing::warn!("request carried no user message — injecting one (1214 guard)");
            let insert_at = usize::from(messages.first().is_some_and(|m| m.role == "system"));
            messages.insert(
                insert_at,
                OpenAiMessage {
                    role: "user".into(),
                    content: Some("Proceed.".into()),
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: None,
                },
            );
        }

        let tools = if req.tools.is_empty() {
            None
        } else {
            Some(
                req.tools
                    .iter()
                    .map(|t| OpenAiTool {
                        tool_type: "function".into(),
                        function: OpenAiFunctionDef {
                            name: t.name.clone(),
                            description: t.description.clone(),
                            parameters: t.parameters.clone(),
                        },
                    })
                    .collect(),
            )
        };

        Ok(OpenAiRequest {
            model: self.model.clone(),
            messages,
            tools,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            stream,
            tool_choice: None,
        })
    }
}

/// Helper trait for extracting text from MessageContent.
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

#[cfg(test)]
mod tests {
    use super::OpenAiProvider;

    #[test]
    fn test_parse_tool_args_repaired_truncation() {
        // R1 (v0.1.1): 截断修复——以 { 开头缺尾 }（LLM 流式截断特征，字符串已闭合）→ 补 } 重试成功
        // 真实场景：content 完整输出完但外层对象缺尾 }（column 2350 EOF）
        // 注意 raw string 语义：`"#` 是 r#"..."# 的结束标记——内容结尾的 content 闭合
        // 引号须单独写（`""#` = 内容 `"` + 结束 `"#`）。这才是真实截断场景（字符串闭合、
        // 外层对象缺尾 }）。
        let raw =
            r#"{"path": "snake_game/index.html", "content": "<html><script>...</script></html>""#;
        let v = OpenAiProvider::parse_tool_args(raw, "write_file");
        assert!(v.is_object(), "截断修复后应为 Object, got {v:?}");
        assert_eq!(
            v.get("path").and_then(|p| p.as_str()),
            Some("snake_game/index.html")
        );
    }

    #[test]
    fn test_parse_tool_args_keeps_raw_on_failure() {
        // 无法修复（非截断特征）→ 保留 raw string + 不 panic
        let raw = "not json at all {{{";
        let v = OpenAiProvider::parse_tool_args(raw, "bash");
        assert!(v.is_string(), "不可修复时保留 raw string");
        assert_eq!(v.as_str().unwrap(), raw);
    }

    #[test]
    fn test_parse_tool_args_normal() {
        let v =
            OpenAiProvider::parse_tool_args(r#"{"path": "a.rs", "content": "x"}"#, "write_file");
        assert!(v.is_object());
        assert_eq!(v["content"], "x");
    }

    /// P1-1 (v0.2.4): usage 反序列化兼容——DeepSeek 扩展字段（prompt_cache_hit_tokens/
    /// prompt_cache_miss_tokens）存在时解析出来；缺失（标准 OpenAI/Agnes/Gemini）时
    /// 为 None 且不 panic。这是"装仪表"——命中率可观测是后续前缀稳定性诊断的前提。
    #[test]
    fn test_usage_cache_fields_deserialize() {
        // DeepSeek 风格（含缓存字段）
        let with_cache = r#"{
            "prompt_tokens": 1000,
            "completion_tokens": 50,
            "total_tokens": 1050,
            "prompt_cache_hit_tokens": 800,
            "prompt_cache_miss_tokens": 200
        }"#;
        let u: OpenAiUsage = serde_json::from_str(with_cache).expect("含缓存字段必须可解析");
        assert_eq!(u.prompt_cache_hit_tokens, Some(800));
        assert_eq!(u.prompt_cache_miss_tokens, Some(200));

        // 标准 OpenAI 风格（无缓存字段）——不报错，字段为 None
        let without_cache = r#"{
            "prompt_tokens": 100,
            "completion_tokens": 20,
            "total_tokens": 120
        }"#;
        let u: OpenAiUsage =
            serde_json::from_str(without_cache).expect("无缓存字段必须可解析（不 panic）");
        assert_eq!(u.prompt_cache_hit_tokens, None, "缺失≠0（未上报）");
        assert_eq!(u.prompt_cache_miss_tokens, None);
    }
    use super::*;
    use llm_gateway::ToolSchema;

    #[test]
    fn test_provider_creation() {
        let p = OpenAiProvider::new("openai", "gpt-4o", None, "sk-test");
        assert_eq!(p.name(), "openai");
        assert_eq!(p.model(), "gpt-4o");
        let caps = p.capabilities();
        assert!(caps.chat);
        assert!(caps.stream);
        assert!(caps.function_calling);
    }

    #[test]
    fn test_build_request_basic() {
        let p = OpenAiProvider::new("openai", "gpt-4o", None, "sk-test");
        let req = ChatRequest {
            messages: vec![],
            tools: vec![],
            temperature: None,
            max_tokens: None,
            stream: false,
        };
        let body = p.build_request(req, false).unwrap();
        assert_eq!(body.model, "gpt-4o");
        assert!(!body.stream);
    }

    #[test]
    fn test_build_request_with_tools() {
        let p = OpenAiProvider::new("openai", "gpt-4o", None, "sk-test");
        let req = ChatRequest {
            messages: vec![],
            tools: vec![ToolSchema {
                name: "bash".into(),
                description: "run command".into(),
                parameters: serde_json::json!({}),
            }],
            temperature: Some(0.5),
            max_tokens: Some(1000),
            stream: true,
        };
        let body = p.build_request(req, true).unwrap();
        assert!(body.stream);
        assert_eq!(body.temperature, Some(0.5));
        assert_eq!(body.tools.unwrap().len(), 1);
    }

    #[test]
    fn test_parse_sse_token() {
        let line = r#"data: {"choices":[{"delta":{"content":"hello"},"finish_reason":null}]}"#;
        let result = parse_sse_line(line);
        assert!(result.is_some());
        if let Some(Ok(StreamEvent::Token(t))) = result {
            assert_eq!(t, "hello");
        } else {
            panic!("expected token event");
        }
    }

    #[test]
    fn test_parse_sse_done() {
        let result = parse_sse_line("data: [DONE]");
        assert!(result.is_some());
    }
}
