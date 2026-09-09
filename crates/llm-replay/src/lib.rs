//! `llm-replay` — deterministic replay provider (v15 line B).
//!
//! # Why this exists
//!
//! v14 archived 31 PASS sessions as event-stream fixtures, but had no replay
//! engine. The naive design ("POST the goal again and see if it still passes")
//! is **not** a regression gate: the real LLM re-reasons every time, so the
//! result measures model luck, not harness health.
//!
//! `ReplayProvider` pins the model instead. It implements [`LlmProvider`] and
//! serves the *recorded* assistant turns in order, so:
//!
//! * the LLM side is fully deterministic (zero network, zero tokens),
//! * the tool side runs for real (fs, bash, sandbox, approval, budget),
//!
//! which means any harness regression — a broken tool, a state-machine change,
//! a budget/approval rule drift — shows up immediately as a replay mismatch.
//!
//! # Fixture format (v14 `archive_replay_v14.py`, `fixture_schema: 1`)
//!
//! ```json
//! { "meta": { "task": "...", "session_id": "...", "provider": "zhipu" },
//!   "history": { "goal": "...", "messages": [
//!       {"seq":2,"event_type":"token","payload":{"delta":"..."}},
//!       {"seq":3,"event_type":"tool_call","payload":{"call_id":"...","name":"glob","args":{}}},
//!       {"seq":5,"event_type":"tool_result","payload":{"call_id":"...","output":"..."}}
//!   ]}}
//! ```
//!
//! Turn boundary rule: tokens and tool_calls accumulate; the first `tool_result`
//! after any accumulated content closes the turn (parallel tool_calls emitted
//! before their results therefore land in the same turn, which matches how the
//! agent loop issued them).

use std::path::Path;

use agent_types::{Message, MessageContent, Role, ToolCall};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::stream::BoxStream;
use llm_gateway::{Capabilities, ChatRequest, ChatResponse, Embedding, LlmProvider, StreamEvent};

/// One recorded assistant turn: free text plus the tool calls it requested.
#[derive(Debug, Clone, Default)]
pub struct Turn {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
}

/// A recorded session keyed by its goal text.
#[derive(Debug, Clone)]
pub struct RecordedSession {
    /// Fixture file stem, e.g. `T14-add-serde__run1__ab12cd34`. Used as the
    /// explicit selector, because two runs of the same task share a goal.
    pub key: String,
    pub task: String,
    pub session_id: String,
    pub goal: String,
    pub turns: Vec<Turn>,
}

/// Marker the replay driver appends to the goal to pin an exact fixture:
/// `[replay-fixture: <file stem>]`.
pub const FIXTURE_MARKER: &str = "[replay-fixture:";

/// Deterministic replay provider. Registered under the name `replay`.
pub struct ReplayProvider {
    sessions: Vec<RecordedSession>,
    model: String,
}

impl ReplayProvider {
    /// Load every `*.json` fixture in `dir`.
    pub fn load_dir(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        let mut sessions = Vec::new();
        let entries = std::fs::read_dir(dir)
            .map_err(|e| anyhow!("replay: cannot read fixture dir {}: {e}", dir.display()))?;
        for entry in entries {
            let path = entry?.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let raw = std::fs::read_to_string(&path)?;
            let v: serde_json::Value = serde_json::from_str(&raw)
                .map_err(|e| anyhow!("replay: bad json in {}: {e}", path.display()))?;
            let key = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("?")
                .to_string();
            if let Some(mut s) = Self::parse_fixture(&v) {
                s.key = key;
                sessions.push(s);
            } else {
                tracing::warn!("replay: skipped unparsable fixture {}", path.display());
            }
        }
        if sessions.is_empty() {
            return Err(anyhow!(
                "replay: no usable fixtures found in {}",
                dir.display()
            ));
        }
        tracing::info!("replay provider loaded {} sessions", sessions.len());
        Ok(Self {
            sessions,
            model: "replay/recorded".to_string(),
        })
    }

    /// How many sessions were loaded (used by tests and by the service log line).
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// True for a `8-4-4-4-12` hex uuid, which is what a session directory is
    /// named after.
    fn is_uuid(s: &str) -> bool {
        let g: Vec<&str> = s.split('-').collect();
        g.len() == 5
            && [8usize, 4, 4, 4, 12] == [g[0].len(), g[1].len(), g[2].len(), g[3].len(), g[4].len()]
            && g.iter().all(|p| p.chars().all(|c| c.is_ascii_hexdigit()))
    }

    /// Rewrite session-pinned absolute paths into workspace-relative ones.
    ///
    /// v15 found this the hard way: the recorded tool args carry absolute paths
    /// of the *original* session, e.g.
    /// `/home/wutao/codex_work/sessions/f12cc9ef-.../src/lib.rs`. Replaying into
    /// a fresh session would then read/write the OLD directory, the edit never
    /// lands in the new workspace and every verify.sh fails (22/30 in the first
    /// replay run). Tools already execute with cwd = session workspace, so
    /// stripping everything up to and including `sessions/<uuid>/` makes the
    /// recording portable across sessions.
    fn portable_paths(args: &serde_json::Value) -> serde_json::Value {
        let Ok(mut s) = serde_json::to_string(args) else {
            return args.clone();
        };
        while let Some(idx) = s.find("sessions/") {
            let after = idx + "sessions/".len();
            let rest = &s[after..];
            let Some(slash) = rest.find('/') else { break };
            if !Self::is_uuid(&rest[..slash]) {
                break;
            }
            // walk back to the opening quote of this JSON string value
            let start = s[..idx].rfind('"').map(|q| q + 1).unwrap_or(idx);
            let end = after + slash + 1;
            s.replace_range(start..end, "");
        }
        serde_json::from_str(&s).unwrap_or_else(|_| args.clone())
    }

    fn parse_fixture(v: &serde_json::Value) -> Option<RecordedSession> {
        let meta = v.get("meta")?;
        let history = v.get("history")?;
        let goal = history.get("goal")?.as_str()?.to_string();
        let messages = history.get("messages")?.as_array()?;

        let mut turns: Vec<Turn> = Vec::new();
        let mut buf = String::new();
        let mut calls: Vec<ToolCall> = Vec::new();
        let mut pending = false;

        for m in messages {
            let ev = m.get("event_type").and_then(|s| s.as_str()).unwrap_or("");
            let payload = m.get("payload").cloned().unwrap_or(serde_json::Value::Null);
            match ev {
                "token" => {
                    if let Some(d) = payload.get("delta").and_then(|s| s.as_str()) {
                        buf.push_str(d);
                        pending = true;
                    }
                }
                "tool_call" => {
                    let call_id = payload
                        .get("call_id")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string();
                    let name = payload
                        .get("name")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string();
                    let args = payload
                        .get("args")
                        .cloned()
                        .unwrap_or(serde_json::Value::Object(Default::default()));
                    if !name.is_empty() {
                        calls.push(ToolCall {
                            call_id,
                            name,
                            args: Self::portable_paths(&args),
                        });
                        pending = true;
                    }
                }
                "tool_result" if pending => {
                    turns.push(Turn {
                        content: if buf.is_empty() {
                            None
                        } else {
                            Some(std::mem::take(&mut buf))
                        },
                        tool_calls: std::mem::take(&mut calls),
                    });
                    pending = false;
                }
                _ => {}
            }
        }
        if pending {
            turns.push(Turn {
                content: if buf.is_empty() { None } else { Some(buf) },
                tool_calls: calls,
            });
        }
        if turns.is_empty() {
            return None;
        }

        Some(RecordedSession {
            key: String::new(), // filled by load_dir from the file stem
            task: meta
                .get("task")
                .and_then(|s| s.as_str())
                .unwrap_or("?")
                .to_string(),
            session_id: meta
                .get("session_id")
                .and_then(|s| s.as_str())
                .unwrap_or("?")
                .to_string(),
            goal,
            turns,
        })
    }

    /// First user message text in the request — used as the fixture key.
    fn request_goal(req: &ChatRequest) -> Option<String> {
        req.messages.iter().find_map(|m: &Message| {
            if matches!(m.role, Role::User) {
                match &m.content {
                    MessageContent::Text(t) => Some(t.clone()),
                    _ => None,
                }
            } else {
                None
            }
        })
    }

    /// How many recorded turns have already been served in this conversation.
    ///
    /// Counting *all* assistant messages is wrong: the PDCA loop also appends an
    /// assistant message for the Reflect verdict, so the index advanced by 2 per
    /// real exchange and the replay skipped every other recorded turn (v15: T16
    /// wrote `math.rs`, silently skipped `string_utils.rs`, then asked for turn
    /// #4 of 4). Only assistant messages that actually carried tool calls
    /// correspond to a consumed turn.
    fn turn_index(req: &ChatRequest) -> usize {
        req.messages
            .iter()
            .filter(|m| {
                matches!(m.role, Role::Assistant)
                    && matches!(&m.content, MessageContent::ToolCalls(c) if !c.is_empty())
            })
            .count()
    }

    /// Extract `<stem>` out of a `[replay-fixture: <stem>]` marker, if present.
    fn explicit_key(goal: &str) -> Option<&str> {
        let start = goal.find(FIXTURE_MARKER)? + FIXTURE_MARKER.len();
        let rest = &goal[start..];
        let end = rest.find(']')?;
        Some(rest[..end].trim())
    }

    fn find_session(&self, goal: &str) -> Option<&RecordedSession> {
        // 1. Explicit marker wins — two runs of one task share a goal, so goal
        //    matching alone cannot tell run0 from run1.
        if let Some(k) = Self::explicit_key(goal) {
            return self.sessions.iter().find(|s| s.key == k);
        }
        // 2. Fall back to goal text: exact, then containment either way.
        self.sessions
            .iter()
            .find(|s| s.goal == goal)
            .or_else(|| self.sessions.iter().find(|s| goal.contains(&s.goal)))
            .or_else(|| self.sessions.iter().find(|s| s.goal.contains(goal)))
    }

    fn lookup(&self, req: &ChatRequest) -> ChatResponse {
        let Some(goal) = Self::request_goal(req) else {
            return Self::exhausted("no user message in request");
        };
        let Some(session) = self.find_session(goal.trim()) else {
            return Self::exhausted("no fixture matches this goal");
        };
        let idx = Self::turn_index(req);
        match session.turns.get(idx) {
            Some(t) => ChatResponse {
                content: t.content.clone(),
                tool_calls: t.tool_calls.clone(),
                finish_reason: Some(if t.tool_calls.is_empty() {
                    "stop".into()
                } else {
                    "tool_calls".into()
                }),
                usage: None,
                reasoning_content: None,
            },
            None => Self::exhausted(&format!(
                "recording for {} has {} turns, asked for #{}",
                session.task,
                session.turns.len(),
                idx
            )),
        }
    }

    fn exhausted(why: &str) -> ChatResponse {
        ChatResponse {
            content: Some(format!("[replay] {why}")),
            tool_calls: Vec::new(),
            finish_reason: Some("stop".into()),
            usage: None,
            reasoning_content: None,
        }
    }
}

#[async_trait]
impl LlmProvider for ReplayProvider {
    fn name(&self) -> &str {
        "replay"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            chat: true,
            stream: true,
            function_calling: true,
            embeddings: false,
            max_context_tokens: None,
        }
    }

    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        Ok(self.lookup(&req))
    }

    fn stream(&self, req: ChatRequest) -> BoxStream<'static, Result<StreamEvent>> {
        let resp = self.lookup(&req);
        let mut evs: Vec<Result<StreamEvent>> = Vec::new();
        if let Some(c) = resp.content {
            if !c.is_empty() {
                evs.push(Ok(StreamEvent::Token(c)));
            }
        }
        for tc in resp.tool_calls {
            evs.push(Ok(StreamEvent::ToolCallDelta {
                call_id: tc.call_id,
                name: Some(tc.name),
                args_delta: tc.args.to_string(),
            }));
        }
        evs.push(Ok(StreamEvent::Finish {
            finish_reason: resp.finish_reason,
            usage: None,
        }));
        Box::pin(futures::stream::iter(evs))
    }

    async fn embed(&self, _inputs: &[String]) -> Result<Vec<Embedding>> {
        Err(anyhow!("replay provider does not support embeddings"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_types::MessageMeta;
    use chrono::Utc;

    fn fixture_json() -> serde_json::Value {
        serde_json::json!({
            "meta": {"task": "T-demo", "session_id": "sid-1", "provider": "zhipu"},
            "history": {
                "goal": "add multiply",
                "messages": [
                    {"seq": 0, "event_type": "phase", "payload": {"phase": "Init"}},
                    {"seq": 1, "event_type": "token", "payload": {"delta": "let me look"}},
                    {"seq": 2, "event_type": "tool_call",
                     "payload": {"call_id": "c1", "name": "glob", "args": {"pattern": "**/*.rs"}}},
                    {"seq": 3, "event_type": "tool_result",
                     "payload": {"call_id": "c1", "output": "src/lib.rs"}},
                    {"seq": 4, "event_type": "token", "payload": {"delta": "done"}}
                ]
            }
        })
    }

    fn msg(role: Role, text: &str) -> Message {
        Message {
            id: "m".into(),
            role,
            content: MessageContent::Text(text.into()),
            created_at: Utc::now(),
            meta: MessageMeta {
                token_count: None,
                source: None,
            },
            reasoning_content: None,
        }
    }

    /// An assistant message that actually carried tool calls — the only kind
    /// that consumes a recorded turn.
    fn act(name: &str) -> Message {
        Message {
            id: "m".into(),
            role: Role::Assistant,
            content: MessageContent::ToolCalls(vec![ToolCall {
                call_id: "c1".into(),
                name: name.into(),
                args: serde_json::json!({}),
            }]),
            created_at: Utc::now(),
            meta: MessageMeta {
                token_count: None,
                source: None,
            },
            reasoning_content: None,
        }
    }

    fn req(messages: Vec<Message>) -> ChatRequest {
        ChatRequest {
            messages,
            tools: vec![],
            temperature: None,
            max_tokens: None,
            stream: false,
        }
    }

    #[test]
    fn parses_turns_from_event_stream() {
        let s = ReplayProvider::parse_fixture(&fixture_json()).expect("parses");
        assert_eq!(s.task, "T-demo");
        // turn 0 = "let me look" + glob call ; turn 1 = trailing "done"
        assert_eq!(s.turns.len(), 2);
        assert_eq!(s.turns[0].tool_calls.len(), 1);
        assert_eq!(s.turns[0].tool_calls[0].name, "glob");
        assert_eq!(s.turns[1].content.as_deref(), Some("done"));
        assert!(s.turns[1].tool_calls.is_empty());
    }

    #[tokio::test]
    async fn serves_recorded_turns_in_order() {
        let p = ReplayProvider {
            sessions: vec![ReplayProvider::parse_fixture(&fixture_json()).unwrap()],
            model: "replay/test".into(),
        };

        // No assistant turns yet -> turn 0 (the glob call).
        let r0 = p
            .chat(req(vec![msg(Role::User, "add multiply")]))
            .await
            .unwrap();
        assert_eq!(r0.tool_calls.len(), 1);
        assert_eq!(r0.tool_calls[0].name, "glob");
        assert_eq!(r0.finish_reason.as_deref(), Some("tool_calls"));

        // One *acted* turn already in history -> turn 1 (final answer).
        let r1 = p
            .chat(req(vec![
                msg(Role::User, "add multiply"),
                act("glob"),
                msg(Role::Tool, "src/lib.rs"),
            ]))
            .await
            .unwrap();
        assert!(r1.tool_calls.is_empty());
        assert_eq!(r1.content.as_deref(), Some("done"));

        // Past the end -> graceful stop, never a panic.
        let r2 = p
            .chat(req(vec![
                msg(Role::User, "add multiply"),
                act("glob"),
                act("write_file"),
                act("bash"),
            ]))
            .await
            .unwrap();
        assert!(r2.tool_calls.is_empty());
        assert!(r2.content.unwrap().starts_with("[replay]"));
    }

    /// Regression for the v15 "every other turn was skipped" bug: the PDCA loop
    /// appends an assistant *text* message for the Reflect verdict after each
    /// exchange. Counting those advanced the cursor by 2 per real turn, so the
    /// replay silently dropped half the recorded actions (T16 wrote `math.rs`
    /// and never `string_utils.rs`). Text-only assistant messages must be inert.
    #[tokio::test]
    async fn reflect_text_messages_do_not_advance_the_cursor() {
        let p = ReplayProvider {
            sessions: vec![ReplayProvider::parse_fixture(&fixture_json()).unwrap()],
            model: "replay/test".into(),
        };

        // Plan/Reflect chatter only — still turn 0.
        let r = p
            .chat(req(vec![
                msg(Role::User, "add multiply"),
                msg(Role::Assistant, "plan: inspect the crate first"),
                msg(Role::Assistant, "reflect: verdict=continue"),
            ]))
            .await
            .unwrap();
        assert_eq!(
            r.tool_calls.len(),
            1,
            "text-only turns must not be consumed"
        );
        assert_eq!(r.tool_calls[0].name, "glob");

        // One real action + surrounding chatter -> exactly turn 1.
        let r = p
            .chat(req(vec![
                msg(Role::User, "add multiply"),
                msg(Role::Assistant, "plan: inspect the crate first"),
                act("glob"),
                msg(Role::Tool, "src/lib.rs"),
                msg(Role::Assistant, "reflect: verdict=continue"),
            ]))
            .await
            .unwrap();
        assert!(r.tool_calls.is_empty());
        assert_eq!(r.content.as_deref(), Some("done"));
    }

    #[tokio::test]
    async fn explicit_marker_disambiguates_same_goal_runs() {
        // Two runs of one task share a goal; only the marker can tell them apart.
        let mut run0 = ReplayProvider::parse_fixture(&fixture_json()).unwrap();
        run0.key = "T-demo__run0__aaaa".into();
        let mut run1 = ReplayProvider::parse_fixture(&fixture_json()).unwrap();
        run1.key = "T-demo__run1__bbbb".into();
        run1.turns[0].tool_calls[0].name = "grep".into(); // make run1 distinguishable

        let p = ReplayProvider {
            sessions: vec![run0, run1],
            model: "replay/test".into(),
        };

        let r = p
            .chat(req(vec![msg(
                Role::User,
                "add multiply\n\n[replay-fixture: T-demo__run1__bbbb]",
            )]))
            .await
            .unwrap();
        assert_eq!(r.tool_calls[0].name, "grep", "marker must select run1");

        // Without the marker we fall back to goal matching -> first session.
        let r0 = p
            .chat(req(vec![msg(Role::User, "add multiply")]))
            .await
            .unwrap();
        assert_eq!(r0.tool_calls[0].name, "glob");
    }

    #[tokio::test]
    async fn unknown_goal_does_not_panic() {
        let p = ReplayProvider {
            sessions: vec![ReplayProvider::parse_fixture(&fixture_json()).unwrap()],
            model: "replay/test".into(),
        };
        let r = p
            .chat(req(vec![msg(Role::User, "something never recorded")]))
            .await
            .unwrap();
        assert!(r.content.unwrap().contains("no fixture matches"));
    }

    #[test]
    fn session_pinned_paths_become_relative() {
        // recorded against session f12cc9ef-...; replay runs in a *different*
        // session, so the absolute path must not survive the load.
        let args = serde_json::json!({
            "path": "/home/wutao/codex_work/sessions/f12cc9ef-4666-4657-aa26-684ac2c62ac0/src/lib.rs"
        });
        let out = ReplayProvider::portable_paths(&args);
        assert_eq!(out["path"], "src/lib.rs");

        // multiple paths in one arg object are all rewritten
        let two = serde_json::json!({
            "from": "/a/b/sessions/f12cc9ef-4666-4657-aa26-684ac2c62ac0/x.rs",
            "to": "/a/b/sessions/f12cc9ef-4666-4657-aa26-684ac2c62ac0/y.rs",
        });
        let out = ReplayProvider::portable_paths(&two);
        assert_eq!(out["from"], "x.rs");
        assert_eq!(out["to"], "y.rs");

        // a non-uuid `sessions/` segment is left alone (not a session dir)
        let keep = serde_json::json!({ "path": "docs/sessions/readme.md" });
        assert_eq!(
            ReplayProvider::portable_paths(&keep)["path"],
            "docs/sessions/readme.md"
        );
    }
}
