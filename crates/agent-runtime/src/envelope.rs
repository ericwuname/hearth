//! 事件信封状态机（D1: 从 service::sse 抽出，供 service SSE 出口与 CLI 直跑共用）。
//!
//! 契约：`docs/ai-os-event-contract-v1.md` 信封 v1——seq 单调递增 + span 树
//! (span_open/close 分配/回填 span_id)。纯逻辑，零 HTTP 依赖。

use api::AgentEvent;

/// 信封 v1 状态：seq 递增 + span 栈（span 树分配）。
pub struct EnvelopeState {
    pub seq: u64,
    pub span_stack: Vec<(String, Option<String>)>,
}

impl EnvelopeState {
    pub fn new() -> Self {
        Self {
            seq: 0,
            span_stack: Vec::new(),
        }
    }

    /// 对 span_open/close 事件分配/回填 span_id；业务事件只累加 seq。
    pub fn wrap(&mut self, evt: &mut AgentEvent) {
        self.seq += 1;
        match evt {
            AgentEvent::SpanOpen {
                span_id, parent_id, ..
            } => {
                let sid = uuid::Uuid::new_v4().to_string();
                let parent = self.span_stack.last().map(|(id, _)| id.clone());
                *span_id = sid.clone();
                *parent_id = parent.clone();
                self.span_stack.push((sid, parent));
            }
            AgentEvent::SpanClose { span_id, .. } => {
                *span_id = self.span_stack.pop().map(|(id, _)| id).unwrap_or_default();
            }
            _ => {}
        }
    }
}

impl Default for EnvelopeState {
    fn default() -> Self {
        Self::new()
    }
}
