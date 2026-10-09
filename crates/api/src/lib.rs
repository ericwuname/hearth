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

// D-185（2026-10-08, traecode）：原 `pub struct InteractionRequest { id, kind, blocking,
// timeout, on_timeout, payload }`（WP-0 曾规划的"交互请求"契约型）**已删除**——它**零构造、
// 零读取、未入契约**（`ApiDoc` 的 `components(schemas(…))` 从未列出它）。原因：内核与
// 服务端**实际发出**的人类介入点是 **SSE 事件** `Event::InteractionRequested { id, kind, … }`
// （见 `agent-core`），请求型的"结构体契约"从未落地——与在用兄弟类型 `InteractionResponse`
// 形成**不对称**（响应型在用、请求型从未存在）。按 D-111/D-78 纪律删除（"只有定义、
// 无任何消费者" = 漂移陷阱）。
// 防回潮：`crates/api/tests/retired_interaction_request_gate.rs`（源码级 pin +
// `InteractionResponse` 反向对照）。若日后确需该结构（如新增轮询式交互查询端点），
// 请**先接线消费者**再立卡加回。

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

/// D-187（2026-10-08, traecode）：`GET /api/v1/models` 的**真实**响应外层。
///
/// 病灶：`ApiDoc` 只声明了**元素**型 `ModelInfo`，而真实响应是 `{"providers": [...]}`——
/// 外层**缺一层类型**；且 `SessionManager::list_providers` 一直**手搓**
/// `json!({"provider": …, "model": …})`（字段与 `ModelInfo` **逐字段一致**），
/// 即"有现成契约型却没用上"。现收敛为类型化返回：**序列化形状不变**（对现有客户端非破坏）。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelsResponse {
    /// 已注册（含未配 key 的空 provider，客户端据此提示）的 provider 列表。
    pub providers: Vec<ModelInfo>,
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

// ── 对外错误码词表（`ErrorResponse.error.code`；客户端据此分支，值本身是契约）──
//
// 现役（服务端**真实发出**）：`ERR_UNAUTHORIZED` / `ERR_FORBIDDEN` /
// `ERR_SESSION_NOT_FOUND` / `ERR_INVALID_PARAM` / `ERR_RATE_LIMITED` /
// `ERR_NOT_FOUND` / `ERR_METHOD_NOT_ALLOWED` / `ERR_INTERNAL`。
pub const ERR_UNAUTHORIZED: &str = "UNAUTHORIZED";
pub const ERR_FORBIDDEN: &str = "FORBIDDEN";
pub const ERR_SESSION_NOT_FOUND: &str = "SESSION_NOT_FOUND";
pub const ERR_INVALID_PARAM: &str = "INVALID_PARAM";
/// D-157（2026-10-05, traecode）：限流中间件 429 的错误码（此前 429 返回纯文本、无 code）。
pub const ERR_RATE_LIMITED: &str = "RATE_LIMITED";
/// D-158（2026-10-05, traecode）：未知路由（404）的错误码（此前为空体、无 code）。
pub const ERR_NOT_FOUND: &str = "NOT_FOUND";
/// D-158（2026-10-05, traecode）：路径存在但方法不允许（405）的错误码（此前为空体、无 code）。
pub const ERR_METHOD_NOT_ALLOWED: &str = "METHOD_NOT_ALLOWED";

// D-111②（2026-10-02, traecode）：以下 4 个**当前无任何生产者**——服务端从不发出它们
// （全仓仅有定义行）。此处**保留而非删除**：它们是**对外错误码词表**的一部分（客户端
// 可能已按这些字符串分支），删掉等于单方缩小契约面且不可逆；故按"如实留痕"处置：
// **启用其中任何一项时，必须同时在服务端把它接上线**，否则客户端永远等不到该码。
// 这与"内部死代码直接删"（D-78/D-88）不同类：契约常量的价值在于**对外可用性**。
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
