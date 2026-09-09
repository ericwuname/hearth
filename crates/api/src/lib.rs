use agent_types::Budget;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

// ── Northbound API contract ──

/// SSE event types pushed to the client.
/// TRACE-1: this enum IS the API contract (the referenced architecture doc
/// was not shipped with the repo). service/src/sse.rs must stay in sync.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    Phase {
        phase: String,
    },
    Token {
        delta: String,
    },
    ToolCall {
        call_id: String,
        name: String,
        args: Value,
    },
    ToolResult {
        call_id: String,
        is_error: bool,
        output: String,
    },
    NeedApproval {
        approval_id: String,
        action: String,
        /// B3-A (backend-intelligence): 交互载荷——clarification 携带问题内容
        /// {from, why}（用户可见"要确认什么"）；approval 为 null/空。契约补注。
        payload: Value,
    },
    Reflection {
        verdict: String,
    },
    Done {
        report: Value,
    },
    Error {
        message: String,
    },
    /// WP-1 (v23 phase3): span 生命周期事件——span_open 进入 / span_close 退出。
    /// span_id 由服务端分配（信封层事实），事件只带 name/t0/t1。
    SpanOpen {
        span_id: String,
        parent_id: Option<String>,
        name: String,
        t0: String,
    },
    SpanClose {
        span_id: String,
        t1: String,
        duration_ms: u64,
    },
    /// WP-5 (v23 phase4): 交互已响应——服务端记录响应事实（by/latency_ms）。
    /// 事件流只有请求（InteractionRequested）没有响应，Observer 无法算
    /// autonomy_rate / wait_ratio——响应必须入流（事实产生权）。
    /// kind 由 Observer 按 interaction_id 匹配请求（Observer 不解析 payload）。
    InteractionResolved {
        interaction_id: String,
        by: String,
        resolved: bool,
        latency_ms: Option<u64>,
    },
    /// WP-8 (v23 phase5): 产物登记——write_file/edit 成功时 BE emit。
    /// path 相对 workspace；delta_lines/size_bytes 为统计事实（不解析语义）。
    Artifact {
        path: String,
        kind: String,
        delta_lines: u64,
        size_bytes: u64,
    },
    /// WP-9 (v23 phase5): 思考摘要——相位完成时的人类可读描述。
    /// 默认确定性模板（零 LLM）；span_id 由信封自动携带（可关联到具体 span）。
    ThinkSummary {
        phase: String,
        text: String,
    },
    /// R2-D (批示 1 + 补充 2, v0.2.7): 目标修订——用户更换 current_goal 时 emit。
    /// 契约只增不改：FE 未知 kind 宽容（禁白屏）、Observer 默认忽略不炸、seq 单调不动。
    GoalChanged {
        revision: u64,
        new_goal: String,
    },
    /// B2 (backend taskbook #01): 规划草案——do_plan 完成时 emit。
    /// steps=规划步骤列表；gaps_found/gaps_to_ask/auto_assumed 为缺口审计事实
    /// （产品红线：每条 gap 必带 from+why，禁止"为问而问"）。
    PlanDraft {
        steps: Value,
        gaps_found: u64,
        gaps_to_ask: u64,
        auto_assumed: Value,
        /// B2-B: 将问的 blocking gap 明细（from+why）——契约 plan.draft 载荷。
        gaps_to_ask_details: Value,
        mode: String,
    },
}

/// WP-1 (v23 phase3): 事件信封——每个 SSE 事件携带的传输事实。
/// `#[serde(flatten)]` 保证 JSON 顶层为
/// `{schema_version, ts, seq, span_id, parent_id, type, phase, ...}`——
/// 旧客户端读 `type`/业务字段不受影响（向后兼容），新客户端读 seq/span_id。
/// 信封字段全部由**服务端产生**（事实产生权：BE 产生事实，FE 投影事实）。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EnvelopedEvent {
    /// 信封 schema 版本（当前 1）。
    pub schema_version: u32,
    /// 事件产生时刻（RFC3339，服务端时钟）。
    pub ts: String,
    /// 会话内单调递增序号——G4 断线续传锚点（Last-Event-ID）。
    pub seq: u64,
    /// 所属 span（无 span 上下文时为空串）。
    pub span_id: String,
    /// 父 span（根 span 为 None）。
    pub parent_id: Option<String>,
    #[serde(flatten)]
    pub event: AgentEvent,
}

/// Request to create a new session.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionCreate {
    pub provider: String,
    pub model: Option<String>,
    pub goal: String,
    pub budget: Option<Budget>,
}

/// Response after creating a session.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionCreateResponse {
    pub session_id: String,
    pub status: String,
}

/// Request to send a message in a session.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MessageReq {
    pub content: String,
}

/// Request to submit an approval decision.
/// WP-0: 保留为薄适配层（deprecated）——新契约走 InteractionResponse。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApprovalReq {
    pub approval_id: String,
    pub decision: String,
}

// ── WP-0: 通用交互原语（v23 §2.1 定版）──

/// WP-0: 交互请求——内核/服务端发出的通用人类介入点。
/// 内核只认 `id` / `blocking` / `timeout`，**绝不 match `kind`、绝不解析 `payload`**。
/// `kind` 必须是开放字符串（"approval" / "clarification" / 自定义）——
/// 做成 Rust enum 等于把 UI 焊进内核。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InteractionRequest {
    /// 唯一 id——响应必须携带（R1 强校验 + R2 一次性消费）。
    pub id: String,
    /// 开放字符串。内核不 match。
    pub kind: String,
    /// true=阻塞等待响应（agent 暂停）；false=通知型（不阻塞）。
    pub blocking: bool,
    /// 超时秒数；None=无限等待。
    pub timeout: Option<u64>,
    /// 超时行为提示（"abort" / "continue" 等）——内核按原样透传，不解析。
    pub on_timeout: Option<String>,
    /// kind 专属负载——内核不解析。
    pub payload: serde_json::Value,
}

/// WP-0: 交互响应——客户端 → POST /api/v1/sessions/{id}/interaction/{iid}。
/// 服务端只校验 id 匹配 + 一次性消费；payload 内容不解析（kind 语义由上层决定）。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InteractionResponse {
    /// 被响应的 InteractionRequest.id（R1：不匹配必拒）。
    pub id: String,
    /// 响应者标识（"human" / "system" / 客户端名）。
    pub by: String,
    /// 通用传输结果：true=放行（approval: approve）；false=中止（approval: deny）。
    /// **内核只读这个布尔决定放行/中止——不解析 payload**（v23 §2.1 必要补充：
    /// 原定版 Response 无此字段，但内核必须能区分"放行/中止"）。
    pub resolved: bool,
    /// kind 专属响应负载（approval: {"decision":"approve"}）——内核不解析。
    pub payload: serde_json::Value,
    /// 响应时延（毫秒，客户端采集上报）。
    pub latency_ms: Option<u64>,
}

/// Session status information.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionStatus {
    pub session_id: String,
    pub phase: String,
    pub steps: u64,
    pub budget_remaining: Option<u64>,
}

/// Available model information.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelInfo {
    pub provider: String,
    pub model: String,
}

// ── B1 (trunk-freeze): session message history response ──
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionHistory {
    pub session_id: String,
    pub goal: String,
    pub messages: Vec<HistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HistoryEntry {
    pub seq: u64,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: String,
}

// ── B2 (trunk-freeze): unified error response ──
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

impl ErrorResponse {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: ErrorBody {
                code: code.into(),
                message: message.into(),
            },
        }
    }
}

pub const ERR_UNAUTHORIZED: &str = "UNAUTHORIZED";
pub const ERR_FORBIDDEN: &str = "FORBIDDEN";
pub const ERR_SESSION_NOT_FOUND: &str = "SESSION_NOT_FOUND";
pub const ERR_INVALID_PARAM: &str = "INVALID_PARAM";
pub const ERR_APPROVAL_TIMEOUT: &str = "APPROVAL_TIMEOUT";
pub const ERR_LLM_ERROR: &str = "LLM_ERROR";
pub const ERR_TOOL_ERROR: &str = "TOOL_ERROR";
pub const ERR_SANDBOX_ERROR: &str = "SANDBOX_ERROR";
pub const ERR_INTERNAL: &str = "INTERNAL";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_event_serde() {
        let evt = AgentEvent::Phase {
            phase: "Plan".into(),
        };
        let json = serde_json::to_string(&evt).unwrap();
        assert!(json.contains("phase"));
        assert!(json.contains("Plan"));
    }

    #[test]
    fn test_session_create() {
        let sc = SessionCreate {
            provider: "openai".into(),
            model: Some("gpt-4o".into()),
            goal: "write tests".into(),
            budget: None,
        };
        let json = serde_json::to_string(&sc).unwrap();
        let back: SessionCreate = serde_json::from_str(&json).unwrap();
        assert_eq!(back.provider, "openai");
    }

    #[test]
    fn test_agent_event_all_variants() {
        let events = vec![
            AgentEvent::Phase {
                phase: "Init".into(),
            },
            AgentEvent::Token { delta: "fn".into() },
            AgentEvent::ToolCall {
                call_id: "c1".into(),
                name: "bash".into(),
                args: serde_json::json!({"cmd":"ls"}),
            },
            AgentEvent::ToolResult {
                call_id: "c1".into(),
                is_error: false,
                output: "ok".into(),
            },
            AgentEvent::NeedApproval {
                approval_id: "a1".into(),
                action: "write".into(),
                payload: serde_json::Value::Null,
            },
            AgentEvent::Reflection {
                verdict: "continue".into(),
            },
            AgentEvent::Done {
                report: serde_json::json!({"steps":3}),
            },
            AgentEvent::Error {
                message: "oops".into(),
            },
        ];
        assert_eq!(events.len(), 8);
        for evt in events {
            let json = serde_json::to_string(&evt).unwrap();
            let _: AgentEvent = serde_json::from_str(&json).unwrap();
        }
    }
}
