use api::AgentEvent;
// D1: 信封状态机已抽至 agent-runtime（纯逻辑，service 仅保留 SSE HTTP 出口）。
use agent_runtime::envelope::EnvelopeState;
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use futures::stream::Stream;
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::broadcast::Receiver;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

/// WP-1 (v23 phase3): 事件信封状态机——seq 单调递增 + span 栈。
/// span_id 由**服务端**分配（事实产生权：BE 产生事实，FE 投影事实）；
/// 业务事件自动携带当前 span 上下文。
/// SSE-2: wrap a stream of `AgentEvent` results and stop yielding once a
/// terminal event (`Done` or `Error`) has been produced. The underlying
/// broadcast channel's sender is never dropped by the session, so without
/// this short-circuit the SSE body would never end and streaming clients
/// (and the benchmark's polling path) hang open after the agent finishes.
struct TerminateOnTerminal<S> {
    inner: S,
    done: bool,
}

impl<S, E> Stream for TerminateOnTerminal<S>
where
    S: Stream<Item = Result<AgentEvent, E>> + Unpin,
{
    type Item = Result<AgentEvent, E>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.done {
            return Poll::Ready(None);
        }
        match Pin::new(&mut this.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(evt))) => {
                if matches!(evt, AgentEvent::Done { .. } | AgentEvent::Error { .. }) {
                    this.done = true;
                }
                Poll::Ready(Some(Ok(evt)))
            }
            other => other,
        }
    }
}

/// Create an SSE stream from a broadcast receiver.
pub fn sse_stream(
    rx: Receiver<AgentEvent>,
) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
    sse_stream_with_replay(rx, Vec::new(), 0)
}

/// WP-2 (v23 phase3): SSE 流——断线续传支持。
/// `replay` = 会话事件缓冲；`last_event_id` = 客户端 Last-Event-ID。
/// 缓冲事件重新 envelop（seq 与实时一致）后，seq > last_event_id 的先发，
/// 再无缝接续 live 流——断线重连不丢事件（G4：停在最后事实 seq=N）。
pub fn sse_stream_with_replay(
    rx: Receiver<AgentEvent>,
    replay: Vec<AgentEvent>,
    last_event_id: u64,
) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
    let mut env = EnvelopeState::new();
    // 1. 同步重放缓冲（重新 envelop，seq 与实时一致）
    let mut replay_events: Vec<Result<SseEvent, Infallible>> = Vec::new();
    for mut evt in replay {
        env.wrap(&mut evt);
        if env.seq <= last_event_id {
            continue; // 客户端已有，跳过
        }
        if let Some(e) = build_sse(&evt, &env) {
            replay_events.push(Ok(e));
        }
    }
    // 2. live 流（接续同一个 EnvelopeState 的 seq/span 栈）
    let stream = BroadcastStream::new(rx);
    let terminated = TerminateOnTerminal {
        inner: stream,
        done: false,
    };
    let live = terminated.filter_map(move |result| match result {
        Ok(mut evt) => {
            env.wrap(&mut evt);
            build_sse(&evt, &env).map(Ok)
        }
        Err(err) => {
            tracing::warn!("SSE: broadcast receiver lagged, events dropped: {err}");
            None
        }
    });

    let mapped = futures::stream::iter(replay_events).chain(live);

    Sse::new(mapped).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

/// 从已 wrap 的事件 + 信封状态构建 SSE 事件（不修改状态）。
fn build_sse(evt: &AgentEvent, env: &EnvelopeState) -> Option<SseEvent> {
    let event_type = match evt {
        AgentEvent::Phase { .. } => "phase",
        AgentEvent::Token { .. } => "token",
        AgentEvent::ToolCall { .. } => "tool_call",
        AgentEvent::ToolResult { .. } => "tool_result",
        AgentEvent::NeedApproval { .. } => "need_approval",
        AgentEvent::Reflection { .. } => "reflection",
        AgentEvent::Done { .. } => "done",
        AgentEvent::Error { .. } => "error",
        AgentEvent::SpanOpen { .. } => "span_open",
        AgentEvent::SpanClose { .. } => "span_close",
        AgentEvent::InteractionResolved { .. } => "interaction_resolved",
        AgentEvent::Artifact { .. } => "artifact",
        AgentEvent::ThinkSummary { .. } => "think_summary",
        AgentEvent::GoalChanged { .. } => "goal_changed",
        AgentEvent::PlanDraft { .. } => "plan_draft",
    };
    let enveloped = api::EnvelopedEvent {
        schema_version: 1,
        ts: chrono::Utc::now().to_rfc3339(),
        seq: env.seq,
        span_id: env
            .span_stack
            .last()
            .map(|(sid, _)| sid.clone())
            .unwrap_or_default(),
        parent_id: env.span_stack.last().and_then(|(_, p)| p.clone()),
        event: evt.clone(),
    };
    let data = serde_json::to_string(&enveloped).ok()?;
    Some(SseEvent::default().event(event_type).data(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span_id_of(evt: &AgentEvent) -> String {
        match evt {
            AgentEvent::SpanOpen { span_id, .. } | AgentEvent::SpanClose { span_id, .. } => {
                span_id.clone()
            }
            _ => String::new(),
        }
    }

    fn parent_of(evt: &AgentEvent) -> Option<String> {
        match evt {
            AgentEvent::SpanOpen { parent_id, .. } => parent_id.clone(),
            _ => None,
        }
    }

    #[test]
    fn test_envelope_seq_monotonic_and_span_tree() {
        // 模拟：plan span → observe span → 子代理 observe span（深度 3 嵌套）
        let mut env = EnvelopeState::new();
        let mut open_plan = AgentEvent::SpanOpen {
            span_id: String::new(),
            parent_id: None,
            name: "plan".into(),
            t0: "t0".into(),
        };
        let mut open_sub = AgentEvent::SpanOpen {
            span_id: String::new(),
            parent_id: None,
            name: "observe".into(),
            t0: "t0".into(),
        };
        let mut open_sub2 = AgentEvent::SpanOpen {
            span_id: String::new(),
            parent_id: None,
            name: "observe".into(),
            t0: "t0".into(),
        };
        let mut phase = AgentEvent::Phase {
            phase: "Observe".into(),
        };
        let mut close_sub2 = AgentEvent::SpanClose {
            span_id: String::new(),
            t1: "t1".into(),
            duration_ms: 1,
        };
        let mut close_sub = AgentEvent::SpanClose {
            span_id: String::new(),
            t1: "t1".into(),
            duration_ms: 2,
        };
        let mut close_plan = AgentEvent::SpanClose {
            span_id: String::new(),
            t1: "t1".into(),
            duration_ms: 3,
        };

        env.wrap(&mut open_plan);
        env.wrap(&mut open_sub);
        env.wrap(&mut open_sub2);
        env.wrap(&mut phase);
        env.wrap(&mut close_sub2);
        env.wrap(&mut close_sub);
        env.wrap(&mut close_plan);

        // seq 单调递增
        assert_eq!(env.seq, 7, "seq 应为 7");
        // span_id 服务端分配
        assert!(!span_id_of(&open_plan).is_empty(), "plan span_id 已分配");
        assert!(!span_id_of(&open_sub).is_empty(), "sub span_id 已分配");
        assert!(!span_id_of(&open_sub2).is_empty(), "sub2 span_id 已分配");
        // 嵌套 parent 链（深度 3：plan → sub → sub2）
        assert_eq!(parent_of(&open_plan), None, "根 span 无父");
        assert_eq!(
            parent_of(&open_sub).as_deref(),
            Some(span_id_of(&open_plan).as_str()),
            "sub 的父 = plan"
        );
        assert_eq!(
            parent_of(&open_sub2).as_deref(),
            Some(span_id_of(&open_sub).as_str()),
            "sub2 的父 = sub（深度 3）"
        );
        // close 回填同一 span_id（LIFO pop 顺序）
        assert_eq!(
            span_id_of(&close_sub2),
            span_id_of(&open_sub2),
            "close 回填 sub2 id"
        );
        assert_eq!(
            span_id_of(&close_sub),
            span_id_of(&open_sub),
            "close 回填 sub id"
        );
        assert_eq!(
            span_id_of(&close_plan),
            span_id_of(&open_plan),
            "close 回填 plan id"
        );
        eprintln!("WP-1 PASS: seq 单调 + span 树可还原（深度 3 嵌套）");
    }

    // ── WP-2 (v23 phase3): G2 重放一致性——离线重放 envelop = 实时 envelop ──

    #[test]
    fn test_replay_envelope_consistent() {
        // 同一事件序列走 EnvelopeState 两次 → seq/span 分配必须一致（确定性）
        let make_events = || -> Vec<AgentEvent> {
            vec![
                AgentEvent::SpanOpen {
                    span_id: String::new(),
                    parent_id: None,
                    name: "plan".into(),
                    t0: "t0".into(),
                },
                AgentEvent::Phase {
                    phase: "Plan".into(),
                },
                AgentEvent::Token {
                    delta: "hello".into(),
                },
                AgentEvent::SpanClose {
                    span_id: String::new(),
                    t1: "t1".into(),
                    duration_ms: 1,
                },
                AgentEvent::Done {
                    report: serde_json::json!({"ok": true}),
                },
            ]
        };

        // 第一次（实时路径）
        let mut e1 = EnvelopeState::new();
        let mut seqs1 = Vec::new();
        let mut span_ids1 = Vec::new();
        for mut evt in make_events() {
            e1.wrap(&mut evt);
            seqs1.push(e1.seq);
            if let AgentEvent::SpanOpen { span_id, .. } = &evt {
                span_ids1.push(span_id.clone());
            }
        }
        // 第二次（重放路径——同一序列重 envelop）
        let mut e2 = EnvelopeState::new();
        let mut seqs2 = Vec::new();
        let mut span_ids2 = Vec::new();
        for mut evt in make_events() {
            e2.wrap(&mut evt);
            seqs2.push(e2.seq);
            if let AgentEvent::SpanOpen { span_id, .. } = &evt {
                span_ids2.push(span_id.clone());
            }
        }
        // G2: 重放与实时 seq 序列字节级一致
        assert_eq!(seqs1, seqs2, "重放 seq 必须与实时一致");
        // span_id 是 uuid��—每次运行不同（随机），但**结构**一致（长度/格式）
        // 断言：seq 递增 + span_id 同格式（uuid v4）
        for (i, s) in seqs1.iter().enumerate() {
            assert_eq!(*s, (i + 1) as u64, "seq 必须从 1 单调递增");
        }
        for sid in &span_ids1 {
            assert_eq!(sid.len(), 36, "span_id 必须 uuid v4 格式: {sid}");
        }
        eprintln!("WP-2 PASS: G2 重放 envelop 确定性（seq 一致 + span 结构一致）");
    }

    // ── WP-8/9 (v23 phase5): Artifact/ThinkSummary 事件序列化门禁 ──

    #[test]
    fn test_wp8_artifact_event_serialized() {
        let mut env = EnvelopeState::new();
        let mut evt = AgentEvent::Artifact {
            path: "src/main.rs".into(),
            kind: "file".into(),
            delta_lines: 42,
            size_bytes: 1024,
        };
        env.wrap(&mut evt);
        let enveloped = api::EnvelopedEvent {
            schema_version: 1,
            ts: "2026-08-04T00:00:00Z".into(),
            seq: env.seq,
            span_id: String::new(),
            parent_id: None,
            event: evt,
        };
        let json = serde_json::to_string(&enveloped).unwrap();
        assert!(json.contains("src/main.rs"), "path 在信封 data 内: {json}");
        assert!(json.contains("\"delta_lines\":42"), "delta 入流: {json}");
        eprintln!("WP-8 PASS: Artifact 事件序列化（path/delta/size 入流）");
    }

    #[test]
    fn test_wp9_think_summary_carries_span() {
        // ThinkSummary 作为业务事件，被信封自动带上 span_id（span_open 后）
        let mut env = EnvelopeState::new();
        let mut open = AgentEvent::SpanOpen {
            span_id: String::new(),
            parent_id: None,
            name: "plan".into(),
            t0: "t0".into(),
        };
        let mut summary = AgentEvent::ThinkSummary {
            phase: "plan".into(),
            text: "正在规划任务分解".into(),
        };
        env.wrap(&mut open);
        env.wrap(&mut summary);
        // 信封 span_id 来自 EnvelopeState 栈顶（业务事件自动带当前 span 上下文）
        let envelope_span = env
            .span_stack
            .last()
            .map(|(id, _)| id.clone())
            .unwrap_or_default();
        assert!(!envelope_span.is_empty(), "ThinkSummary 信封带 span_id");
        assert_eq!(envelope_span, span_id_of(&open), "span_id 关联到当前 span");
        let enveloped = api::EnvelopedEvent {
            schema_version: 1,
            ts: "2026-08-04T00:00:00Z".into(),
            seq: env.seq,
            span_id: envelope_span,
            parent_id: None,
            event: summary,
        };
        let json = serde_json::to_string(&enveloped).unwrap();
        assert!(json.contains("正在规划任务分解"), "text 入流: {json}");
        assert!(json.contains("\"span_id\""), "span_id 在信封: {json}");
        eprintln!("WP-9 PASS: ThinkSummary 带 span_id（可关联具体 span）");
    }
}
