use agent_core::{Agent, AgentLoop, Event, Goal};
use agent_types::Budget;
use api::{
    AgentEvent, ApprovalReq, HistoryEntry, MessageReq, SessionCreate, SessionCreateResponse,
    SessionHistory, SessionStatus,
};
use experience::ExperienceStore;
use llm_gateway::{CostMeter, ProviderRegistry, Usage};
use lsp_bridge::LspBridge;
use memory::{MemoryStore, SessionRecord, StoredEvent};
use planner::DefaultPlanner;

/// B3-3 (backend taskbook #01): URL 直开判定（http/https → 浏览器）。
fn is_http_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// B3-3: 启动外部程序打开目标（xdg-open / open / start 按平台）。
async fn spawn_open(target: &str) -> anyhow::Result<String> {
    #[cfg(target_os = "linux")]
    let mut cmd = std::process::Command::new("xdg-open");
    #[cfg(target_os = "macos")]
    let mut cmd = std::process::Command::new("open");
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = std::process::Command::new("cmd");
        c.arg("/C").arg("start").arg("").arg(target);
        c
    };
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    let mut cmd = std::process::Command::new("xdg-open");
    cmd.arg(target);
    cmd.spawn()
        .map_err(|e| anyhow::anyhow!("spawn open failed: {e}"))?;
    Ok(format!("opened: {target}"))
}
use retriever::Retriever;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::{Mutex, RwLock};
use tool_runtime::{ToolContext, ToolDispatcher};

/// A running agent session.
pub struct Session {
    pub id: String,
    pub provider_name: String,
    pub model: String,
    pub goal: String,
    pub phase: String,
    pub steps: u64,
    pub budget_remaining: Option<u64>,
    pub event_tx: Option<tokio::sync::broadcast::Sender<AgentEvent>>,
    /// B1 (trunk-freeze): in-memory ring of broadcast events for history replay.
    pub events: Vec<AgentEvent>,
    pub running: bool,
    /// The agent instance (wrapped so we can call run).
    pub agent: Option<AgentLoop>,
    /// E2 v5.0: per-session workspace directory for filesystem isolation.
    pub workspace_dir: std::path::PathBuf,
    /// P3: Stored budget from session creation (for send_message).
    pub budget: Budget,
    /// v1.1 (M4): cancellation signal sender for the running agent task.
    pub cancel_tx: Option<oneshot::Sender<()>>,
    /// OOM-1 (global-audit): when this session reached a terminal state
    /// (done / error / cancelled). `None` = still active. Used by
    /// `cleanup_finished` to evict old sessions from the in-memory map,
    /// which otherwise grows without bound.
    pub finished_at: Option<std::time::Instant>,
}

/// Manages all active sessions.
pub struct SessionManager {
    sessions: RwLock<HashMap<String, Arc<Mutex<Session>>>>,
    registry: Arc<ProviderRegistry>,
    dispatcher: Arc<ToolDispatcher>,
    ctx: ToolContext,
    /// A5: Optional retriever injected into each new AgentLoop.
    retriever: Option<Arc<dyn Retriever>>,
    /// v11.0: Experience store injected into each new AgentLoop.
    experience_store: Option<Arc<ExperienceStore>>,
    /// A4: Optional LSP bridge injected into each new AgentLoop.
    lsp_bridge: Option<Arc<dyn LspBridge>>,
    /// P4: CostMeter for session-scoped token accounting.
    cost_meter: Arc<Mutex<CostMeter>>,
    /// P5: MemoryStore for session persistence across restarts.
    memory_store: Option<Arc<dyn MemoryStore>>,
    /// v13 S3-b: Civilization writer injected into every new AgentLoop.
    civ_writer: Option<Arc<dyn agent_core::CivWriter>>,
    /// Q3 (v24-post): Observer OS 句柄——会话结束后评估事件流并落盘报告。
    observer: Option<Arc<observer::Observer>>,
}

impl SessionManager {
    pub fn new(
        registry: Arc<ProviderRegistry>,
        dispatcher: Arc<ToolDispatcher>,
        ctx: ToolContext,
    ) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            registry,
            dispatcher,
            ctx,
            retriever: None,
            experience_store: None,
            lsp_bridge: None,
            cost_meter: Arc::new(Mutex::new(CostMeter::new())),
            memory_store: None,
            civ_writer: None,
            observer: None,
        }
    }

    /// Q3 (v24-post): 注入 Observer（会话结束后报告落盘）。
    pub fn set_observer(&mut self, o: Arc<observer::Observer>) {
        self.observer = Some(o);
    }

    /// v13 S3-b: Set the civilization writer to inject into each new AgentLoop.
    pub fn set_civ_writer(&mut self, writer: Arc<dyn agent_core::CivWriter>) {
        self.civ_writer = Some(writer);
    }

    /// A5: Set the retriever to inject into each new AgentLoop.
    pub fn set_retriever(&mut self, retriever: Arc<dyn Retriever>) {
        self.retriever = Some(retriever);
    }
    /// v11.0: Set the experience store to inject into each new AgentLoop.
    pub fn set_experience_store(&mut self, store: Arc<ExperienceStore>) {
        self.experience_store = Some(store);
    }

    /// A4: Set the LSP bridge to inject into each new AgentLoop.
    pub fn set_lsp_bridge(&mut self, bridge: Arc<dyn LspBridge>) {
        self.lsp_bridge = Some(bridge);
    }

    /// P5: Set the MemoryStore for session persistence.
    pub fn set_memory_store(&mut self, store: Arc<dyn MemoryStore>) {
        self.memory_store = Some(store);
    }

    /// v6.1 L1: Create and run a bridge discussion synchronously.
    /// Returns (id, summary, turns_count).
    pub async fn create_bridge(
        &self,
        topic: String,
        participants: Vec<String>,
        strategy: bridge::BridgeStrategy,
    ) -> anyhow::Result<(String, String, usize)> {
        let mut session =
            bridge::BridgeSession::new(topic, participants, strategy, self.registry.clone());
        let id = session.id.clone();
        let summary = session.run(3).await?;
        let turns = session.turns.len();
        Ok((id, summary, turns))
    }

    /// P5: List all persisted session IDs from the MemoryStore.
    pub async fn list_persisted_sessions(&self) -> anyhow::Result<Vec<String>> {
        match &self.memory_store {
            Some(store) => store.list_sessions().await,
            // P1-2 (audit-fix): 无存储 ≠ 就绪——上抛让 /readyz 返回 503，
            // 不再返回 Ok(vec![]) 假装健康。
            None => Err(anyhow::anyhow!("no memory store configured")),
        }
    }

    /// P5: Load a persisted session record from the MemoryStore.
    pub async fn load_persisted_session(
        &self,
        session_id: &str,
    ) -> anyhow::Result<Option<SessionRecord>> {
        match &self.memory_store {
            Some(store) => store.load_session(session_id).await,
            None => Ok(None),
        }
    }

    /// B1 (trunk-freeze): Load session history for the history-replay endpoint.
    /// In-memory active sessions return their buffered broadcast events;
    /// completed/persisted sessions fall back to the MemoryStore.
    pub async fn get_history(&self, session_id: &str) -> anyhow::Result<Option<SessionHistory>> {
        {
            let sessions = self.sessions.read().await;
            if let Some(s) = sessions.get(session_id) {
                let inner = s.lock().await;
                return Ok(Some(SessionHistory {
                    session_id: session_id.to_string(),
                    goal: inner.goal.clone(),
                    messages: inner
                        .events
                        .iter()
                        .enumerate()
                        .map(|(i, e)| HistoryEntry {
                            seq: i as u64,
                            event_type: agent_event_kind(e),
                            payload: serde_json::to_value(e).unwrap_or(serde_json::Value::Null),
                            timestamp: chrono::Utc::now().to_rfc3339(),
                        })
                        .collect(),
                }));
            }
        }
        match &self.memory_store {
            Some(store) => match store.load_session(session_id).await? {
                Some(record) => Ok(Some(SessionHistory {
                    session_id: record.session_id,
                    goal: record.goal,
                    messages: record
                        .events
                        .into_iter()
                        .map(|e| HistoryEntry {
                            seq: e.seq,
                            event_type: e.event_type,
                            payload: e.payload,
                            timestamp: e.timestamp,
                        })
                        .collect(),
                })),
                None => Ok(None),
            },
            None => Ok(None),
        }
    }

    /// P4: Get a reference to the CostMeter for external inspection.
    pub fn cost_meter(&self) -> Arc<Mutex<CostMeter>> {
        self.cost_meter.clone()
    }

    /// Create a new session.
    pub async fn create_session(
        &self,
        req: SessionCreate,
    ) -> anyhow::Result<SessionCreateResponse> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let provider_name = req.provider.clone();

        let provider = self.registry.get(&provider_name)?;
        // v12.2: default the model to the provider's REAL model name instead of
        // a hardcoded "gpt-4o" placeholder — otherwise usage accounting logs
        // `provider=zhipu model=gpt-4o` (label mismatch reported by user).
        let model = req
            .model
            .clone()
            .unwrap_or_else(|| provider.model().to_string());

        // v12.2: compute workspace_dir BEFORE agent so ctx.cwd points to the session directory
        let workspace_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("sessions")
            .join(&session_id);
        std::fs::create_dir_all(&workspace_dir).unwrap_or_else(|e| {
            tracing::warn!("create workspace dir {}: {e}", workspace_dir.display());
        });

        let budget = req.budget.unwrap_or_default();
        let max_steps = budget.max_steps;
        let goal = Goal::with_budget(req.goal.clone(), budget.clone());

        let mut agent = AgentLoop::new(
            provider.clone(),
            Arc::new(DefaultPlanner::new(provider)),
            self.dispatcher.clone(),
            ToolContext {
                cwd: if self.ctx.cwd.as_os_str() == "." || self.ctx.cwd.as_os_str().is_empty() {
                    // v12.2: default cwd → use session workspace
                    workspace_dir.clone()
                } else {
                    // Test or custom cwd → keep as-is
                    self.ctx.cwd.clone()
                },
                ..self.ctx.clone()
            },
            goal,
        );

        // A4/A5: Inject optional retriever and LSP bridge from SessionManager config
        if let Some(ref retriever) = self.retriever {
            agent.set_retriever(retriever.clone());
        }
        // v11.0: Inject experience store
        if let Some(ref store) = self.experience_store {
            agent.set_experience_store(store.clone());
        }
        if let Some(ref lsp) = self.lsp_bridge {
            agent.set_lsp_bridge(lsp.clone());
        }
        // v13 S3-b: Inject civ writer so do_reflect/do_observe auto-append.
        if let Some(ref cw) = self.civ_writer {
            agent.set_civ_writer(cw.clone());
        }

        // P5: Inject shared CostMeter into AgentLoop for real usage tracking
        agent.set_cost_meter(self.cost_meter.clone());

        // M2: scope this loop's approval state to the session id
        agent.set_session_id(session_id.clone());

        let (event_tx, _) = tokio::sync::broadcast::channel::<AgentEvent>(128);

        // E2 v5.0: workspace_dir computed above (moved before AgentLoop for ctx.cwd fix)

        let session = Arc::new(Mutex::new(Session {
            id: session_id.clone(),
            provider_name: provider_name.clone(),
            model: model.clone(),
            goal: req.goal.clone(),
            phase: "created".into(),
            steps: 0,
            budget_remaining: Some(max_steps),
            event_tx: Some(event_tx),
            events: Vec::new(),
            running: false,
            agent: Some(agent),
            budget,
            cancel_tx: None,
            finished_at: None,
            workspace_dir,
        }));

        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session);

        // P5: Persist session to MemoryStore
        if let Some(ref store) = self.memory_store {
            let record = SessionRecord {
                session_id: session_id.clone(),
                provider_name: provider_name.clone(),
                model: model.clone(),
                goal: req.goal.clone(),
                events: Vec::new(),
                created_at: chrono::Utc::now().to_rfc3339(),
            };
            if let Err(e) = store.save_session(&record).await {
                tracing::warn!("P5: failed to persist session {}: {e:#}", session_id);
            }
        }

        Ok(SessionCreateResponse {
            session_id,
            status: "created".into(),
        })
    }

    /// Get a session by ID.
    pub async fn get_session(&self, id: &str) -> Option<Arc<Mutex<Session>>> {
        let map = self.sessions.read().await;
        tracing::info!(session_count = map.len(), lookup_id = %id, "get_session");
        map.get(id).cloned()
    }

    /// WP-10 (v23 phase5): 取会话实时事件流接收端（GET /stream SSE 用）。
    pub async fn stream_rx(
        &self,
        id: &str,
    ) -> Option<tokio::sync::broadcast::Receiver<AgentEvent>> {
        let session = self.get_session(id).await?;
        let tx = session.lock().await.event_tx.clone()?;
        Some(tx.subscribe())
    }

    /// WP-2 (v23 phase3): 取会话事件缓冲副本（Last-Event-ID 断线续传重放源）。
    pub async fn session_events(&self, id: &str) -> Vec<AgentEvent> {
        if let Some(session) = self.get_session(id).await {
            session.lock().await.events.clone()
        } else {
            Vec::new()
        }
    }

    /// WP-2 (v23 phase3): 录制导出——会话事件缓冲 → 带信封的 JSONL 行。
    pub async fn session_events_jsonl(&self, id: &str) -> Vec<String> {
        let events = self.session_events(id).await;
        let mut env = crate::envelope::EnvelopeState::new();
        events
            .into_iter()
            .map(|mut evt| {
                env.wrap(&mut evt);
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
                    event: evt,
                };
                serde_json::to_string(&enveloped).unwrap_or_default()
            })
            .collect()
    }

    /// WP-8 (v23 phase5): 打开产物——读 session workspace 内文件内容。
    /// **路径校验**：拒绝绝对路径/`..` 穿越，强制留在 workspace 内
    /// （复用 edit 工具的双重防线：相对路径 + strip_prefix 校验）。
    pub async fn open_artifact(&self, id: &str, rel_path: &str) -> anyhow::Result<String> {
        let session = self
            .get_session(id)
            .await
            .ok_or_else(|| anyhow::anyhow!("session not found: {id}"))?;
        let ws = session.lock().await.workspace_dir.clone();
        // 1. 拒绝绝对路径与 .. 穿越
        let p = std::path::Path::new(rel_path);
        if p.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        }) {
            anyhow::bail!("path traversal denied: {rel_path}");
        }
        // 2. join 后必须仍在 workspace 内
        let full = ws.join(rel_path);
        if !full.starts_with(&ws) {
            anyhow::bail!("path outside workspace denied: {rel_path}");
        }
        let content = tokio::fs::read_to_string(&full)
            .await
            .map_err(|e| anyhow::anyhow!("read artifact {rel_path} failed: {e}"))?;
        Ok(content)
    }

    /// B3-3 (backend taskbook #01): 系统打开产物——file → xdg-open/默认程序，
    /// url → 默认浏览器（Linux 用 xdg-open，跨平台回退 open/start）。
    /// 路径校验与 open_artifact 同款（workspace 内，防穿越）；只启动外部程序，
    /// 不读取内容（内容读取走 open_artifact）。
    pub async fn open_external(&self, id: &str, rel_path: &str) -> anyhow::Result<String> {
        let session = self
            .get_session(id)
            .await
            .ok_or_else(|| anyhow::anyhow!("session not found: {id}"))?;
        let ws = session.lock().await.workspace_dir.clone();
        // 1. 拒绝绝对路径与 .. 穿越
        let p = std::path::Path::new(rel_path);
        if p.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        }) {
            anyhow::bail!("path traversal denied: {rel_path}");
        }
        // 2. join 后必须仍在 workspace 内（URL 直开除外——无文件系统访问）
        if !is_http_url(rel_path) {
            let full = ws.join(rel_path);
            if !full.starts_with(&ws) {
                anyhow::bail!("path outside workspace denied: {rel_path}");
            }
            if !full.exists() {
                anyhow::bail!("artifact not found: {rel_path}");
            }
            let target = full.to_string_lossy().to_string();
            return spawn_open(&target).await;
        }
        spawn_open(rel_path).await
    }

    /// P0-3 (audit-fix): 会话列表——内存活跃 + 持久化并集，契约匹配
    /// codex-cli 的 `{id, status, steps, budget_remaining}`。
    pub async fn list_sessions_json(&self) -> Vec<serde_json::Value> {
        let mut out: Vec<serde_json::Value> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        {
            let map = self.sessions.read().await;
            for (id, sess) in map.iter() {
                seen.insert(id.clone());
                let (phase, steps, budget_remaining) = {
                    let g = sess.lock().await;
                    (g.phase.clone(), g.steps, g.budget_remaining)
                };
                out.push(serde_json::json!({
                    "id": id,
                    "status": phase,
                    "steps": steps,
                    "budget_remaining": budget_remaining,
                }));
            }
        }
        // 持久化但未加载的会话（P1-1 读路径接线后可见）
        if let Ok(ids) = self.list_persisted_sessions().await {
            for id in ids {
                if !seen.contains(&id) {
                    out.push(serde_json::json!({
                        "id": id, "status": "persisted", "steps": 0, "budget_remaining": null,
                    }));
                }
            }
        }
        out
    }

    /// Get session status.
    pub async fn get_status(&self, id: &str) -> Option<SessionStatus> {
        let s = self.get_session(id).await?;
        let s = s.lock().await;
        Some(SessionStatus {
            session_id: s.id.clone(),
            phase: s.phase.clone(),
            steps: s.steps,
            budget_remaining: s.budget_remaining,
        })
    }

    /// Send a message to a session — triggers agent.run() and returns SSE receiver.
    pub async fn send_message(
        &self,
        id: &str,
        req: MessageReq,
    ) -> anyhow::Result<tokio::sync::broadcast::Receiver<AgentEvent>> {
        let session = self
            .get_session(id)
            .await
            .ok_or_else(|| anyhow::anyhow!("session not found: {}", id))?;

        let rx = {
            let s = session.lock().await;
            s.event_tx
                .as_ref()
                .map(|tx| tx.subscribe())
                .ok_or_else(|| anyhow::anyhow!("no event stream"))?
        };

        // Spawn the agent loop — it will push events via the internal channel,
        // which we forward to the broadcast sender.
        let session_clone = session.clone();
        let tx = {
            let s = session.lock().await;
            s.event_tx
                .clone()
                .ok_or_else(|| anyhow::anyhow!("session {} event stream closed", id))?
        };
        let cost_meter = self.cost_meter.clone();
        let memory_store = self.memory_store.clone();
        // Q3: Observer 句柄——会话结束后评估事件流并落盘报告。
        let observer = self.observer.clone();

        // M4: create a cancellation channel and store the sender on the session.
        let (cancel_tx, mut cancel_rx) = oneshot::channel::<()>();
        {
            let mut s = session.lock().await;
            // X3 (defensive): a prior cancel_tx should never still be present here
            // — the state machine takes `agent` on first send, so a second
            // send_message hits the "agent not available" branch. If it ever does
            // appear, that is a re-entrancy bug; warn instead of silently dropping.
            if s.cancel_tx.is_some() {
                tracing::warn!(
                    "send_message({}): previous cancel_tx still present; replacing (possible re-entrancy)",
                    id
                );
            }
            s.cancel_tx = Some(cancel_tx);
            // X7 fix: set `running` synchronously under the same lock that holds
            // `cancel_tx`, so a concurrent cancel_session can't have its
            // `running = false` overwritten by the spawned task later.
            s.running = true;
        }

        // Clone the shared dispatcher so we can release this session's approval
        // entry when it ends (X2 fix: the per-session approval map otherwise
        // leaks one key per session).
        let dispatcher = self.dispatcher.clone();

        tokio::spawn(async move {
            let (agent, goal) = {
                let mut s = session_clone.lock().await;
                s.phase = "running".into();
                let agent = s.agent.take();
                let goal = Goal::with_budget(s.goal.clone(), s.budget.clone());
                (agent, goal)
            };

            if let Some(mut agent) = agent {
                // F1: queue user message — will be injected after run() resets ctx_mgr
                if !req.content.is_empty() {
                    agent.add_user_message(req.content);
                }
                // Set up internal event forwarding
                let (internal_tx, mut internal_rx) =
                    tokio::sync::mpsc::unbounded_channel::<Event>();
                agent.set_event_sender(internal_tx);

                // Drive the agent loop in background
                let agent_future = tokio::spawn(async move { agent.run(goal).await });

                // Forward events
                let mut cancelled = false;
                loop {
                    tokio::select! {
                        Some(evt) = internal_rx.recv() => {
                            let is_done = matches!(evt, Event::Done(_));
                        let api_evt = map_event(evt);
                        let _ = tx.send(api_evt.clone());
                        session_clone.lock().await.events.push(api_evt);
                            if is_done {
                                break;
                            }
                        }
                        _ = &mut cancel_rx => {
                            let ev = AgentEvent::Error {
                                message: "session cancelled".into(),
                            };
                            let _ = tx.send(ev.clone());
                            session_clone.lock().await.events.push(ev);
                            // X4 fix: emit Done so the SSE stream closes cleanly
                            // like the normal completion path, instead of relying
                            // on the keepalive timeout to drop the connection.
                            let ev = AgentEvent::Done {
                                report: serde_json::json!({"ok": false, "status": "cancelled"}),
                            };
                            let _ = tx.send(ev.clone());
                            session_clone.lock().await.events.push(ev);
                            cancelled = true;
                            break;
                        }
                    }
                }

                if cancelled {
                    // M4: stop the background agent task immediately so it stops
                    // consuming resources / calling the LLM API.
                    agent_future.abort();
                    let mut s = session_clone.lock().await;
                    s.running = false;
                    s.phase = "cancelled".into();
                    s.cancel_tx = None;
                    s.finished_at = Some(std::time::Instant::now());
                    // X2 fix: release this session's approval entry so the shared
                    // dispatcher approval map doesn't leak one key per session.
                    let sid = s.id.clone();
                    drop(s);
                    dispatcher.reset_interaction(&sid).await;
                    return;
                }

                let join_result = agent_future.await;
                let mut s = session_clone.lock().await;
                s.running = false;

                // agent.run() emits Done in all exit paths; only handle join failures
                match join_result {
                    Ok(Ok(report)) => {
                        s.phase = if report.ok {
                            "done".into()
                        } else {
                            "error".into()
                        };
                        let steps_used = report.steps;
                        // v12.3: surface the real step count on the session so
                        // GET /sessions/{id} reports it (was stuck at 0 — bug
                        // reported by user). The benchmark reads this field.
                        s.steps = steps_used;
                        let ok = report.ok;

                        // P5: Record real usage from RunReport into shared CostMeter
                        if let Some(ref entry) = report.usage {
                            // Merge the agent's accumulated usage into the session-scoped meter
                            let mut cm = cost_meter.lock().await;
                            // Use the entry data directly — agent already accumulated per-(provider,model)
                            let provider = s.provider_name.clone();
                            let model = s.model.clone();
                            let usage = Usage {
                                prompt_tokens: entry.prompt_tokens as u32,
                                completion_tokens: entry.completion_tokens as u32,
                                total_tokens: entry.total_tokens as u32,
                                // P1-1 (v0.2.4): 会话级汇总不重复计缓存（agent 层已累计）
                                prompt_cache_hit_tokens: None,
                                prompt_cache_miss_tokens: None,
                            };
                            cm.record(&provider, &model, &usage);
                            tracing::info!(
                                provider = %provider,
                                model = %model,
                                prompt_tokens = entry.prompt_tokens,
                                completion_tokens = entry.completion_tokens,
                                calls = entry.calls,
                                steps = steps_used,
                                ok = ok,
                                "P5: real usage recorded from RunReport"
                            );
                        }

                        // P5: Persist completed session to MemoryStore
                        if let Some(ref store) = memory_store {
                            let record = SessionRecord {
                                session_id: s.id.clone(),
                                provider_name: s.provider_name.clone(),
                                model: s.model.clone(),
                                goal: s.goal.clone(),
                                events: vec![StoredEvent {
                                    seq: 0,
                                    event_type: "done".into(),
                                    payload: serde_json::json!({
                                        "ok": ok,
                                        "steps": steps_used,
                                        "phase": s.phase,
                                        "usage": report.usage,
                                    }),
                                    timestamp: chrono::Utc::now().to_rfc3339(),
                                }],
                                created_at: chrono::Utc::now().to_rfc3339(),
                            };
                            if let Err(e) = store.save_session(&record).await {
                                tracing::warn!(
                                    "P5: failed to persist completed session {}: {e:#}",
                                    s.id
                                );
                            }
                        }

                        // Q3 (v24-post): Observer 报告落盘——会话结束后评估事件流
                        // （信封化）→ report.md/json 写 CODEX_OBSERVER_DIR/reports/{sid}/。
                        // 与旧 v10 daily-*.jsonl（资源快照）分目录，零执行权不变。
                        if let Some(ref obs) = observer {
                            let evts: Vec<AgentEvent> = s.events.clone();
                            let mut env = crate::envelope::EnvelopeState::new();
                            let enveloped: Vec<api::EnvelopedEvent> = evts
                                .into_iter()
                                .map(|mut e| {
                                    env.wrap(&mut e);
                                    api::EnvelopedEvent {
                                        schema_version: 1,
                                        ts: chrono::Utc::now().to_rfc3339(),
                                        seq: env.seq,
                                        span_id: env
                                            .span_stack
                                            .last()
                                            .map(|(sid, _)| sid.clone())
                                            .unwrap_or_default(),
                                        parent_id: env
                                            .span_stack
                                            .last()
                                            .and_then(|(_, p)| p.clone()),
                                        event: e,
                                    }
                                })
                                .collect();
                            let dir = std::env::var("CODEX_OBSERVER_DIR")
                                .unwrap_or_else(|_| "./observer".into());
                            if let Err(e) = obs
                                .run_and_report(std::path::Path::new(&dir), &s.id, &enveloped)
                                .await
                            {
                                tracing::warn!("Q3: observer report failed for {}: {e:#}", s.id);
                            }
                        }
                    }
                    Ok(Err(_e)) => {
                        s.phase = "error".into();
                        let ev = AgentEvent::Error {
                            message: "agent terminated with error".into(),
                        };
                        let _ = tx.send(ev.clone());
                        s.events.push(ev);
                        let ev = AgentEvent::Done {
                            report: serde_json::json!({"ok": false, "status": "error"}),
                        };
                        let _ = tx.send(ev.clone());
                        s.events.push(ev);
                    }
                    Err(_) => {
                        s.phase = "error".into();
                        let ev = AgentEvent::Error {
                            message: "agent panicked or was cancelled".into(),
                        };
                        let _ = tx.send(ev.clone());
                        s.events.push(ev);
                        let ev = AgentEvent::Done {
                            report: serde_json::json!({"ok": false, "status": "panic"}),
                        };
                        let _ = tx.send(ev.clone());
                        s.events.push(ev);
                    }
                }

                // OOM-1: mark terminal so the TTL cleanup can evict this session.
                s.finished_at = Some(std::time::Instant::now());

                // X2 fix: release this session's approval entry (avoids a per-session
                // leak in the shared dispatcher approval map). `s` is still held here.
                dispatcher.reset_interaction(&s.id).await;

                // P5: No more fake usage — real usage is recorded above from RunReport.usage
            } else {
                // No agent available — send error and mark session as error
                let ev = AgentEvent::Error {
                    message: "agent not available".into(),
                };
                let _ = tx.send(ev.clone());
                session_clone.lock().await.events.push(ev);
                let ev = AgentEvent::Done {
                    report: serde_json::json!({"ok": false, "status": "error"}),
                };
                let _ = tx.send(ev.clone());
                session_clone.lock().await.events.push(ev);
                let mut s = session_clone.lock().await;
                s.running = false;
                s.phase = "error".into();
                s.finished_at = Some(std::time::Instant::now());
            }
        });

        Ok(rx)
    }

    /// Submit an approval decision — resolves the approval state in the dispatcher.
    pub async fn submit_approval(&self, id: &str, req: ApprovalReq) -> anyhow::Result<()> {
        let session = self
            .get_session(id)
            .await
            .ok_or_else(|| anyhow::anyhow!("session not found: {}", id))?;

        let s = session.lock().await;
        // WP-0a/R1: 传 approval_id 参与校验（RT4-③：approval_id 原本不参与校验）
        // WP-0: decision 字符串 → 通用 resolved bool（"approve"=放行 / "deny"=中止）
        let resolved = match req.decision.as_str() {
            "approve" => true,
            "deny" => false,
            other => {
                drop(s);
                anyhow::bail!("invalid decision: {}", other)
            }
        };
        let result = self
            .dispatcher
            .resolve_interaction(
                id,
                &req.approval_id,
                resolved,
                serde_json::json!({ "decision": req.decision }),
            )
            .await;
        drop(s);
        result?;
        Ok(())
    }

    /// WP-0 (v23 §2.1): 通用交互响应——POST /interaction/{iid}。
    /// R1 id 强校验 + R2 一次性消费由 dispatcher 保证；内核不解析 payload。
    pub async fn submit_interaction(
        &self,
        id: &str,
        interaction_id: &str,
        resolved: bool,
        by: &str,
        _payload: serde_json::Value,
        latency_ms: Option<u64>,
    ) -> anyhow::Result<()> {
        let session = self
            .get_session(id)
            .await
            .ok_or_else(|| anyhow::anyhow!("session not found: {}", id))?;
        let s = session.lock().await;
        let result = self
            .dispatcher
            .resolve_interaction(id, interaction_id, resolved, _payload)
            .await;
        drop(s);
        result?;
        // WP-5: 响应是事实——必须入事件流（Observer 算 autonomy_rate/wait_ratio）
        // by/latency_ms 是客户端采集回传（Z-16：human_ms 只有 FE 知道，采集即回传）
        let ev = AgentEvent::InteractionResolved {
            interaction_id: interaction_id.to_string(),
            by: by.to_string(),
            resolved,
            latency_ms,
        };
        if let Some(session) = self.get_session(id).await {
            let mut s = session.lock().await;
            if let Some(tx) = &s.event_tx {
                let _ = tx.send(ev.clone());
            }
            s.events.push(ev);
        }
        tracing::debug!(
            session_id = %id,
            interaction_id = %interaction_id,
            by = %by,
            resolved = %resolved,
            "interaction resolved"
        );
        Ok(())
    }

    /// Cancel a session.
    /// Sends a cancellation signal to the running agent task (M4). The task
    /// receives the signal, stops forwarding events, aborts the background
    /// agent run, and marks itself cancelled — without leaking resources.
    pub async fn cancel_session(&self, id: &str) -> anyhow::Result<()> {
        let session = self
            .get_session(id)
            .await
            .ok_or_else(|| anyhow::anyhow!("session not found: {}", id))?;

        let mut s = session.lock().await;
        if let Some(tx) = s.cancel_tx.take() {
            let _ = tx.send(());
        }
        // Mark not running; the task sets phase to "cancelled" upon signal receipt.
        s.running = false;
        s.finished_at = Some(std::time::Instant::now());
        // E2: cleanup per-session workspace dir (unless CODEX_KEEP_WORKSPACES=1)
        if std::env::var("CODEX_KEEP_WORKSPACES")
            .map(|v| v != "1")
            .unwrap_or(true)
        {
            if let Err(e) = std::fs::remove_dir_all(&s.workspace_dir) {
                tracing::warn!(
                    "E2: failed to clean up workspace {:?}: {e}",
                    s.workspace_dir
                );
            }
        }
        Ok(())
    }

    /// OOM-1 (global-audit): evict finished sessions that have exceeded the
    /// TTL. The in-memory `sessions` map otherwise grows without bound because
    /// nothing ever removes entries. Finished sessions keep their status
    /// readable for up to `ttl` so `get_status` still works post-completion,
    /// then they are dropped. Active sessions are never touched.
    ///
    /// Returns the number of sessions evicted.
    pub async fn cleanup_finished(&self, ttl: std::time::Duration) -> usize {
        let now = std::time::Instant::now();
        let mut guard = self.sessions.write().await;
        let before = guard.len();
        guard.retain(|_id, sess| match sess.try_lock() {
            Ok(s) => match s.finished_at {
                // Finished: keep only while within TTL.
                Some(t) => {
                    let expired = now.duration_since(t) > ttl;
                    // E2: clean up workspace of evicted sessions
                    if expired
                        && std::env::var("CODEX_KEEP_WORKSPACES")
                            .map(|v| v != "1")
                            .unwrap_or(true)
                    {
                        if let Err(e) = std::fs::remove_dir_all(&s.workspace_dir) {
                            tracing::warn!(
                                "E2: cleanup_finished: failed to remove {:?}: {e}",
                                s.workspace_dir
                            );
                        }
                    }
                    !expired
                }
                // Active: always keep.
                None => true,
            },
            // Lock busy → session is mid-transition; keep and revisit next sweep.
            Err(_) => true,
        });
        let evicted = before - guard.len();
        if evicted > 0 {
            tracing::info!("OOM-1: evicted {evicted} finished session(s) past TTL");
        }
        evicted
    }

    /// OOM-1: spawn a background task that periodically evicts finished
    /// sessions past the TTL. A no-op in tests (called only from main).
    pub fn start_cleanup_task(
        self: &Arc<Self>,
        ttl: std::time::Duration,
        interval: std::time::Duration,
    ) {
        let mgr = self.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                mgr.cleanup_finished(ttl).await;
            }
        });
    }

    /// P4: List all registered provider names and their models.
    pub fn list_providers(&self) -> Vec<serde_json::Value> {
        self.registry
            .list_detailed()
            .into_iter()
            .map(|(name, model)| serde_json::json!({"provider": name, "model": model}))
            .collect()
    }
}

/// Map agent-core internal events to API AgentEvent types.
pub fn map_event(evt: Event) -> AgentEvent {
    match evt {
        Event::Phase(phase) => AgentEvent::Phase {
            phase: format!("{:?}", phase),
        },
        Event::Token(delta) => AgentEvent::Token { delta },
        Event::ToolCall(tc) => AgentEvent::ToolCall {
            call_id: tc.call_id,
            name: tc.name,
            args: tc.args,
        },
        Event::ToolResult(tr) => AgentEvent::ToolResult {
            call_id: tr.call_id,
            is_error: tr.is_error,
            output: tr.output,
        },
        // WP-0: 内核已泛化为 InteractionRequested（kind 开放字符串）——
        // 对外 API 契约保留 need_approval 事件名（兼容客户端），action=kind。
        Event::InteractionRequested {
            id, kind, payload, ..
        } => AgentEvent::NeedApproval {
            approval_id: id,
            action: kind,
            payload,
        },
        Event::ThinkSummary { phase, text } => AgentEvent::ThinkSummary { phase, text },
        Event::GoalChanged { revision, new_goal } => AgentEvent::GoalChanged { revision, new_goal },
        Event::PlanDraft {
            steps,
            gaps_found,
            gaps_to_ask,
            auto_assumed,
            gaps_to_ask_details,
            mode,
        } => AgentEvent::PlanDraft {
            steps,
            gaps_found,
            gaps_to_ask,
            auto_assumed,
            gaps_to_ask_details: serde_json::Value::Array(gaps_to_ask_details),
            mode,
        },
        Event::Artifact {
            path,
            kind,
            delta_lines,
            size_bytes,
        } => AgentEvent::Artifact {
            path,
            kind,
            delta_lines,
            size_bytes,
        },
        Event::SpanOpen { name, t0 } => AgentEvent::SpanOpen {
            span_id: String::new(), // 信封器分配（见 sse.rs EnvelopeState）
            parent_id: None,
            name,
            t0,
        },
        Event::SpanClose { t1, duration_ms } => AgentEvent::SpanClose {
            span_id: String::new(), // 信封器回填
            t1,
            duration_ms,
        },
        Event::Done(report) => AgentEvent::Done { report },
        Event::Error(message) => AgentEvent::Error { message },
        // A4: LSP diagnostics → map to Phase event for now (can be expanded later)
        Event::LspDiagnostics(diags) => AgentEvent::Phase {
            phase: format!("lsp_diagnostics: {} issues", diags.len()),
        },
        // A5: Retrieval results → map to Phase event (can be expanded later)
        Event::Retrieval(results) => AgentEvent::Phase {
            phase: format!("retrieval: {} chunks", results.len()),
        },
    }
}

/// B1 (trunk-freeze): map an AgentEvent enum variant to a stable string kind
/// for history replay (since `AgentEvent` is an enum, not a struct with
/// `event_type`/`payload` fields).
fn agent_event_kind(e: &AgentEvent) -> String {
    match e {
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
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use memory::{JsonlMemoryStore, SessionRecord, StoredEvent};

    #[tokio::test]
    async fn test_create_and_get_session() {
        let registry = Arc::new(ProviderRegistry::new());
        let dispatcher = Arc::new(ToolDispatcher::new());
        let ctx = ToolContext::default();
        let mgr = SessionManager::new(registry, dispatcher, ctx);

        let resp = mgr
            .create_session(SessionCreate {
                provider: "none".into(),
                model: None,
                goal: "test".into(),
                budget: None,
            })
            .await;
        // Provider won't be found, so this should error
        assert!(resp.is_err());
    }

    /// OOM-1 regression: finished sessions past TTL are evicted; active
    /// sessions and recently-finished sessions are kept.
    #[tokio::test]
    async fn test_v12_cleanup_finished_evicts_only_expired() {
        let registry = Arc::new(ProviderRegistry::new());
        let dispatcher = Arc::new(ToolDispatcher::new());
        let ctx = ToolContext::default();
        let mgr = SessionManager::new(registry, dispatcher, ctx);

        // Manually insert three sessions: active / freshly-finished / long-finished.
        let mk = |id: &str, finished: Option<std::time::Instant>| {
            Arc::new(Mutex::new(Session {
                id: id.into(),
                provider_name: "p".into(),
                model: "m".into(),
                goal: "g".into(),
                phase: "done".into(),
                steps: 0,
                budget_remaining: None,
                event_tx: None,
                events: Vec::new(),
                running: false,
                agent: None,
                budget: Budget::default(),
                cancel_tx: None,
                finished_at: finished,
                workspace_dir: std::path::PathBuf::from("/tmp/test"),
            }))
        };
        let now = std::time::Instant::now();
        {
            let mut guard = mgr.sessions.write().await;
            guard.insert("active".into(), mk("active", None));
            guard.insert("finished".into(), mk("finished", Some(now)));
        }

        // Generous TTL: nothing should be evicted yet.
        let evicted = mgr
            .cleanup_finished(std::time::Duration::from_secs(3600))
            .await;
        assert_eq!(evicted, 0);
        assert!(mgr.get_session("finished").await.is_some());

        // Zero TTL after a short wait: the finished session must be evicted,
        // the active one must survive.
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let evicted = mgr.cleanup_finished(std::time::Duration::ZERO).await;
        assert_eq!(evicted, 1, "only the finished session should be evicted");
        assert!(mgr.get_session("active").await.is_some());
        assert!(mgr.get_session("finished").await.is_none());
    }

    /// B1 (trunk-freeze) regression: an in-memory active session returns its
    /// buffered events, mapped via `agent_event_kind` + `serde_json::to_value`,
    /// with sequential `seq` starting at 0.
    #[tokio::test]
    async fn test_b1_get_history_in_memory() {
        let registry = Arc::new(ProviderRegistry::new());
        let dispatcher = Arc::new(ToolDispatcher::new());
        let ctx = ToolContext::default();
        let mgr = SessionManager::new(registry, dispatcher, ctx);

        let session = Arc::new(Mutex::new(Session {
            id: "mem-1".into(),
            provider_name: "p".into(),
            model: "m".into(),
            goal: "replay me".into(),
            phase: "running".into(),
            steps: 0,
            budget_remaining: None,
            event_tx: None,
            events: vec![
                AgentEvent::Phase {
                    phase: "Init".into(),
                },
                AgentEvent::Token {
                    delta: "hello".into(),
                },
                AgentEvent::Done {
                    report: "done".into(),
                },
            ],
            running: true,
            agent: None,
            budget: Budget::default(),
            cancel_tx: None,
            finished_at: None,
            workspace_dir: std::path::PathBuf::from("/tmp/test"),
        }));
        mgr.sessions.write().await.insert("mem-1".into(), session);

        let hist = mgr
            .get_history("mem-1")
            .await
            .unwrap()
            .expect("history present");
        assert_eq!(hist.session_id, "mem-1");
        assert_eq!(hist.goal, "replay me");
        assert_eq!(hist.messages.len(), 3);
        assert_eq!(hist.messages[0].seq, 0);
        assert_eq!(hist.messages[0].event_type, "phase");
        assert_eq!(hist.messages[1].event_type, "token");
        assert_eq!(hist.messages[2].event_type, "done");
        // payload of the token event carries the delta (full AgentEvent
        // serialized via `serde_json::to_value`)
        assert!(hist.messages[1].payload.to_string().contains("hello"));
    }

    /// B1 (trunk-freeze) regression: a persisted session not present in the
    /// in-memory map falls back to the MemoryStore and reconstructs the same
    /// shape from `StoredEvent` rows. Unknown ids resolve to `None`.
    #[tokio::test]
    async fn test_b1_get_history_from_store() {
        let registry = Arc::new(ProviderRegistry::new());
        let dispatcher = Arc::new(ToolDispatcher::new());
        let ctx = ToolContext::default();
        let mut mgr = SessionManager::new(registry, dispatcher, ctx);

        let tmp = tempfile::tempdir().unwrap();
        let store = Arc::new(JsonlMemoryStore::new(tmp.path()));
        store
            .save_session(&SessionRecord {
                session_id: "store-1".into(),
                provider_name: "p".into(),
                model: "m".into(),
                goal: "persisted".into(),
                events: vec![
                    StoredEvent {
                        seq: 0,
                        event_type: "phase".into(),
                        payload: serde_json::json!({"phase": "Init"}),
                        timestamp: "2025-01-01T00:00:00Z".into(),
                    },
                    StoredEvent {
                        seq: 1,
                        event_type: "token".into(),
                        payload: serde_json::json!({"delta": "world"}),
                        timestamp: "2025-01-01T00:00:01Z".into(),
                    },
                ],
                created_at: "2025-01-01T00:00:00Z".into(),
            })
            .await
            .unwrap();
        mgr.set_memory_store(store);

        // Not in the in-memory map → must read from the store branch.
        let hist = mgr
            .get_history("store-1")
            .await
            .unwrap()
            .expect("history present");
        assert_eq!(hist.session_id, "store-1");
        assert_eq!(hist.goal, "persisted");
        assert_eq!(hist.messages.len(), 2);
        assert_eq!(hist.messages[0].seq, 0);
        assert_eq!(hist.messages[0].event_type, "phase");
        assert_eq!(hist.messages[1].event_type, "token");
        assert_eq!(hist.messages[1].payload["delta"].as_str(), Some("world"));

        // Unknown id → None (store miss, not in memory)
        assert!(mgr.get_history("nope").await.unwrap().is_none());
    }
}
