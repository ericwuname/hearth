use agent_types::{Message, ToolCall};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A chat completion request sent to an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<Message>,
    pub tools: Vec<ToolSchema>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
}

/// A chat completion response from an LLM provider (non-streaming).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: Option<String>,
    pub usage: Option<Usage>,
    /// v24-post: deepseek thinking mode 的 reasoning_content——必须原样回传，
    /// 否则 API 400（'reasoning_content in thinking mode must be passed back'）。
    pub reasoning_content: Option<String>,
}

/// A streaming event from an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamEvent {
    Token(String),
    ToolCallDelta {
        call_id: String,
        name: Option<String>,
        args_delta: String,
    },
    Finish {
        finish_reason: Option<String>,
        usage: Option<Usage>,
    },
    Error(String),
}

/// An embedding vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub index: usize,
    pub values: Vec<f32>,
}

/// 供应商错误分类（hearth-harness-review-supplement 补充2 映射表落地）。
///
/// - `Transient`: 网络超时 / 连接重置 / 读 body 中断 / 429 速率限制 → **retry**
///   （短退避，同请求上限 2 次；第 3 次升级为不可恢复——"象棋 6 次卡死 245s"直接药方）。
/// - `Param`: HTTP 400 / 参数校验失败 / 工具参数缺字段 → **replan**（改参数重来，不盲目重试）。
/// - `Fatal`: HTTP 500 / 供应商内部错误 / 内容策略拒绝 / 不可恢复 → **give_up**（标记原因，不空转）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorClass {
    Transient,
    Param,
    Fatal,
}

/// 结构化供应商错误——携带分类，loop 层按 class 分流（retry / replan / give_up），
/// 不再"所有错误一视同仁重试 6 次"。
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("transient provider error: {0}")]
    Transient(String),
    #[error("request parameter error: {0}")]
    Param(String),
    #[error("provider unrecoverable error: {0}")]
    Fatal(String),
}

impl LlmError {
    pub fn class(&self) -> ErrorClass {
        match self {
            LlmError::Transient(_) => ErrorClass::Transient,
            LlmError::Param(_) => ErrorClass::Param,
            LlmError::Fatal(_) => ErrorClass::Fatal,
        }
    }
}

/// 把 anyhow::Error 分类（loop 层消费）。
/// ① 结构化优先：downcast LlmError 直接取 class；
/// ② 兜底启发式：字符串特征匹配（旧 provider 或第三方错误未带 class 时也能正确分流）。
pub fn classify_anyhow(e: &anyhow::Error) -> ErrorClass {
    if let Some(le) = e.downcast_ref::<LlmError>() {
        return le.class();
    }
    let msg = format!("{e}");
    let low = msg.to_lowercase();
    // Tier3 T3: 连续 read-body 失败（provider 层标记）→ Fatal 快速失败。
    // 规则必须**先于** "read body"→Transient 检查（否则字符串含 "read body"
    // 被误判 Transient——B04/B06 类错误重试不会自愈）。非 LlmError 包装的
    // 同类字符串错误（其他来源/测试 mock）同样覆盖。
    if low.contains("read body failed twice")
        || low.contains("read body failed twice consecutively")
    {
        return ErrorClass::Fatal;
    }
    // T2 (v0.2.3): 端点死（通道下线）≠ 瞬时抖动——connection refused/unreachable/DNS
    // 是"通道已死"，重试无意义 → Fatal（提示换通道）；此前一律 Transient 导致死通道无限重试。
    if low.contains("connection refused")
        || low.contains("unreachable")
        || low.contains("name or service not known")
        || low.contains("dns")
        || low.contains("temporarily down")
    {
        return ErrorClass::Fatal;
    }
    // 瞬时：传输/连接/超时/速率限制
    if low.contains("read body")
        || low.contains("request failed")
        || low.contains("timed out")
        || low.contains("timeout")
        || low.contains("connection")
        || low.contains("reset")
        || low.contains("network")
        || low.contains("send request")
        || low.contains("429")
        || low.contains("1302")
        || low.contains("eof")
    {
        return ErrorClass::Transient;
    }
    // 参数：4xx 客户端错误（401/403 由 loop 层单独特判，这里归 Param）
    if low.contains("400")
        || low.contains("missing")
        || low.contains("invalid")
        || low.contains("argument")
        || low.contains("parse response")
        || low.contains("no choices")
    {
        return ErrorClass::Param;
    }
    // 其余（含 500/策略拒绝）→ 不可恢复
    ErrorClass::Fatal
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    /// P1-1 (v0.2.4): 缓存命中 token 数（OpenAI 兼容通道扩展字段，
    /// DeepSeek 返回 prompt_cache_hit_tokens）。不支持该字段的 provider 为 None——
    /// 缺失≠0（区分"未上报"与"真零命中"，避免误导命中率判断）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_cache_hit_tokens: Option<u32>,
    /// P1-1: 缓存未命中 token 数（DeepSeek prompt_cache_miss_tokens）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_cache_miss_tokens: Option<u32>,
}

/// Provider capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Capabilities {
    pub chat: bool,
    pub stream: bool,
    pub function_calling: bool,
    pub embeddings: bool,
    pub max_context_tokens: Option<u32>,
}

/// Schema for a tool / function that can be called.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// A JSON schema property.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySchema {
    #[serde(rename = "type")]
    pub prop_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<std::collections::HashMap<String, PropertySchema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_request_serde() {
        let req = ChatRequest {
            messages: vec![],
            tools: vec![],
            temperature: Some(0.7),
            max_tokens: Some(4096),
            stream: true,
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: ChatRequest = serde_json::from_str(&json).unwrap();
        assert!(back.stream);
        assert_eq!(back.temperature, Some(0.7));
    }

    #[test]
    fn test_capabilities_default() {
        let c = Capabilities::default();
        assert!(!c.chat);
    }

    /// 回归（hearth-harness-review-supplement 补充2 表内 test_error_classification_table）：
    /// 400→Param（replan）/ 500→Fatal（give_up）/ timeout·read body→Transient（retry）。
    /// 防止"错误分类退化回一视同仁重试"（象棋 6 次卡死 245s 的直接药方）。
    #[test]
    fn test_error_classification_table() {
        use crate::classify_anyhow;
        use crate::ErrorClass::{Fatal, Param, Transient};

        let cases = [
            // 网络超时 / 连接重置 / 读 body 中断 / 429 → Transient（retry）
            ("read body: error decoding response body", Transient),
            ("OpenAI request failed: connection reset", Transient),
            ("request timed out", Transient),
            ("HTTP 429: rate limited (code 1302)", Transient),
            // T2 (v0.2.3): network unreachable = 通道死（Fatal，重试无意义）——旧表判 Transient 误导死通道无限重试
            ("send request error: network unreachable", Fatal),
            ("connection refused", Fatal),
            ("name or service not known", Fatal),
            // 瞬时（timeout/429）仍 Transient——可重试
            ("request timed out", Transient),
            ("HTTP 429 rate limit", Transient),
            // Tier3 T3 (handoff): 连续 2 次 read-body 失败 → provider 层标 Fatal
            // （同一畸形流重试不会自愈）——loop 收到 Fatal 不再重试，快速失败
            // 切 FallbackChain。B04/B06（200s 挂起）的正面阻断。
            (
                "read body failed twice consecutively (stream likely broken): error decoding response body — not retrying same provider",
                Fatal,
            ),
            // HTTP 400 / 参数缺失 / JSON 解析失败 → Param（replan，不盲目重试）
            ("HTTP 400: invalid request body", Param),
            ("missing 'content' argument", Param),
            ("parse response: expected value at line 1", Param),
            ("OpenAI error 400: bad parameter", Param),
            // HTTP 500 / 供应商内部错误 / 策略拒绝 → Fatal（give_up，不空转）
            ("HTTP 500: internal server error", Fatal),
            ("OpenAI error 500: upstream failure", Fatal),
            ("content policy violation", Fatal),
            ("unknown provider error", Fatal),
        ];
        for (msg, expected) in cases {
            let e = anyhow::anyhow!("{msg}");
            let got = classify_anyhow(&e);
            assert_eq!(
                got, expected,
                "classify({msg:?}) 应= {expected:?}，实际 {got:?}"
            );
        }

        // 结构化优先：LlmError 直接取 class（不依赖字符串特征）
        assert_eq!(
            classify_anyhow(&anyhow::Error::new(LlmError::Transient("x".into()))),
            Transient
        );
        assert_eq!(
            classify_anyhow(&anyhow::Error::new(LlmError::Param("x".into()))),
            Param
        );
        assert_eq!(
            classify_anyhow(&anyhow::Error::new(LlmError::Fatal("x".into()))),
            Fatal
        );
    }
}
