use agent_core::{Agent, AgentLoop, Event, Goal};
use agent_types::Budget;
use api::{
    AgentEvent, ApprovalReq, HistoryEntry, MessageReq, SessionCreate, SessionCreateResponse,
    SessionHistory, SessionStatus,
};
/// D-33 收敛：有界读入 / 有界排空 / 树杀的共用实现。
use bounded_io::{read_file_text_capped, MAX_CAPTURED_BYTES};
use experience::ExperienceStore;
use llm_gateway::{CostMeter, ProviderRegistry, Usage};
use memory::{MemoryStore, SessionRecord, StoredEvent};

/// B3-3 (backend taskbook #01): URL 直开判定（http/https → 浏览器）。
fn is_http_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

// P0-08 / D-33：产物预览的**字节**上限与有界读取。
//
// 病灶（D-31）：`open_artifact` 原用 `tokio::fs::read_to_string` —— 把文件**整份**
// 读进内存，再作为 HTTP 响应体原样返回，**没有任何上限**。而该文件就在会话
// workspace 内、由 agent 自己产出（`cargo build 2>&1 | tee build.log`、一次大数据
// 导出、一次网页抓取落盘……）——一个多 GB 的产物即可把**服务端** OOM。
// 触发成本：一次 `GET /api/v1/sessions/:id/artifact/open?path=…`。
//
// D-33 收敛（2026-10-01，顶层裁决「收敛」）：实现与上限都已改用共享 crate
// `bounded_io`（语义：读到上限、退合法 UTF-8 边界、未截断时严格拒绝非 UTF-8）。
// P1-06：原为 `///` 且下方有空行 —— clippy `empty_line_after_outer_attr` 会报错。

/// 2026-10-01 安全修复（traecode）：`open_external` 的 target 来自请求参数
/// （`GET /api/v1/sessions/:id/artifact/open-external?path=…`），**原先直接交给 shell**
/// → 构成**命令注入**。
///
/// 病灶：Windows 分支原为 `cmd /C start "" <target>`，而 `&`/`|`/`^`/`<`/`>` 等 cmd
/// 元字符会被当作命令分隔符。触发路径：`?path=http://a%26calc.exe` —— Query 解码后得到
/// 含 `&` 的字符串 → cmd 视为两条命令，第二条被执行（认证用户 → 宿主机任意命令执行）。
/// 附带次生缺陷：Windows 分支已在闭包内 `arg(target)`，函数尾部又 `arg(target)` 一次
/// → target 被传两遍。
///
/// 修复：**任何平台都不经 shell** —— Windows 改用 `explorer.exe`（`std::process::Command`
/// 走 CreateProcess 直接传参，无元字符解析），并去掉重复传参；另加前置校验。
fn validate_open_target(target: &str) -> anyhow::Result<()> {
    if target.is_empty() {
        anyhow::bail!("open target is empty");
    }
    if target.chars().any(|c| c.is_control() || c == '"') {
        anyhow::bail!("open target contains control chars or double quote: rejected");
    }
    Ok(())
}

/// B3-3: 启动外部程序打开目标（xdg-open / open / explorer 按平台）。
/// 安全：**任何平台都不经 shell**（见上 `validate_open_target` 的修复说明）。
async fn spawn_open(target: &str) -> anyhow::Result<String> {
    validate_open_target(target)?;
    #[cfg(target_os = "windows")]
    let mut cmd = std::process::Command::new("explorer.exe");
    #[cfg(target_os = "macos")]
    let mut cmd = std::process::Command::new("open");
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let mut cmd = std::process::Command::new("xdg-open");
    cmd.arg(target);
    cmd.spawn()
        .map_err(|e| anyhow::anyhow!("spawn open failed: {e}"))?;
    // P0-04（2026-10-01, traecode）：如实措辞——`spawn` 成功 ≠ **打开**成功
    // （headless Linux 上 xdg-open 常立即非零退出，旧文案 "opened:" 会误报成功）。
    Ok(format!(
        "opened: {target} (仅表示已请求外部程序处理，不代表打开成功)"
    ))
}
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::{Mutex, RwLock};
use tool_runtime::{ToolContext, ToolDispatcher};

/// D-101（2026-10-02, traecode）：单会话事件缓冲上限。达到上限时**批量**丢弃最旧
/// 事件（一次丢 MAX/10，摊还 O(1)），并累加绝对基址 `events_base_seq`，使信封 seq
/// 保持**绝对编号**（截断后客户端会看到 id 跳变并收到保留窗口内的全部事件，而不是
/// 因重新编号被静默过滤掉）。上限取 50_000 条：按事件平均几 KB 估算，单会话峰值
/// 内存约数十 MB 量级，配合既有 cleanup_finished 的会话回收足以避免无界增长。
const MAX_SESSION_EVENTS: usize = 50_000;

/// A running agent session.
pub struct Session {
    pub id: String,
    pub provider_name: String,
    pub model: String,
    pub goal: String,
    /// D-171（2026-10-08, traecode）：会话**归属用户**（多租户隔离判据）。
    ///
    /// 由 [`SessionManager::create_session_with_owner`] 在创建时落定；HTTP 侧所有
    /// 以 id 寻址的会话 handler 都先经 `ensure_session_owner` 比对调用者 uid，
    /// **不匹配一律 404**（不泄露"该 id 存在但归别人"）。无归属上下文（测试/CLI）
    /// 走 `create_session` → `"default"`。持久化前该字段此前**完全缺失** ⇒ 重启后
    /// 任何租户都能看到彼此的持久化会话（跨租户泄露）。
    pub owner: String,
    pub phase: String,
    pub steps: u64,
    pub budget_remaining: Option<u64>,
    pub event_tx: Option<tokio::sync::broadcast::Sender<AgentEvent>>,
    /// B1 (trunk-freeze): 本会话事件的**全量内存缓冲**——供 history 查询
    /// （`get_history`）与 SSE 断线续传重放（`sse_stream_with_replay`）。
    ///
    /// D-74（2026-10-01, traecode）**注释订正**：旧注释称其为 "in-memory **ring**"，
    /// 与实现不符——当时它是**无上限 `Vec`**，只推不减。
    ///
    /// D-101（2026-10-02, traecode）**实施上限 + 绝对基址**：本字段现为**有上限 `Vec`**
    /// （见 [`MAX_SESSION_EVENTS`]），唯一入队口为 [`Session::push_event`]，越界时**批量**
    /// 丢弃最旧事件并推进 [`Session::events_base_seq`]，使信封 seq 保持**绝对编号**。
    /// 不变量：未截断（基址=0）时行为与历史版本逐字节一致；截断后客户端只会看到
    /// Last-Event-ID **跳变**（可感知），而不再被重新编号静默过滤掉（丢事件且无感知）。
    pub events: Vec<AgentEvent>,
    /// D-101：`events[0]` 的**绝对**信封序号偏移（= 已从头部丢弃的事件数）。
    /// 0 = 未截断（此时行为与历史版本逐字节一致）。会话内信封 seq = events_base_seq + 位置。
    pub events_base_seq: u64,
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
    /// P0-04（2026-10-01, traecode）：会话**创建时刻**。
    ///
    /// 为什么需要：`cleanup_finished` 原以 `finished_at: None` 判定"活跃，永不回收"，
    /// 但**从未 `send_message` 的会话 `finished_at` 恒为 `None`** → 永不被回收，
    /// 内存与 `sessions/{uuid}` 工作区目录**双向无界增长**（反复 `POST /api/v1/sessions`
    /// 即为一条 DoS 路径）。有了创建时刻，即可按 idle TTL 回收**非运行中**者。
    pub created_at: std::time::Instant,
}

impl Session {
    /// D-101：唯一的事件入队口——超过 `MAX_SESSION_EVENTS` 时批量丢弃最旧事件并推进
    /// 绝对基址（只 warn 一次以免刷屏：按"首次越界"判断，**不为此新增依赖/字段**）。
    fn push_event(&mut self, ev: AgentEvent) {
        if self.events.len() >= MAX_SESSION_EVENTS {
            // 批量丢弃（一次 MAX/10）以摊还成本：单次 drop 为 O(n) 搬移，但平摊到
            // 每次 push 即 O(1)（丢弃量固定，摊还次数也固定）。
            let drop_n = (MAX_SESSION_EVENTS / 10).max(1);
            self.events.drain(0..drop_n);
            self.events_base_seq += drop_n as u64;
            // 基址恰为本次丢弃量 ⇒ 这是本会话**首次**越界；此后不再重复告警（防刷屏）。
            if self.events_base_seq == drop_n as u64 {
                tracing::warn!(
                    session_id = %self.id,
                    dropped = drop_n,
                    base_seq = self.events_base_seq,
                    "会话事件缓冲达上限，已丢弃最旧事件（重放降级：客户端将看到 Last-Event-ID 跳变）"
                );
            }
        }
        self.events.push(ev);
    }
}

/// D-109（2026-10-02, traecode）：文明线写入器**工厂**的类型别名——按归属用户（owner）
/// 构造该用户的写入器。组合根（service）在闭包内把 owner 绑定到 per-user store。
/// （抽别名同时消除 clippy `type_complexity`。）
pub type CivWriterFactory = Arc<dyn Fn(&str) -> Arc<dyn agent_core::CivWriter> + Send + Sync>;

/// D-175（2026-10-08, traecode）：经验库**工厂**的类型别名——按归属用户（owner）构造该用户的
/// 经验库。组合根（service）在闭包内把 owner 绑定到 `per_user.experience_for(owner)`，使经验
/// 条目（含 `problem = 会话目标文本`）落到**该租户的档**，而不是全局单例（跨租户泄露）。
/// 未注册工厂时回落全局单例（单租户/测试，行为不变）。
pub type ExperienceFactory = Arc<dyn Fn(&str) -> Arc<ExperienceStore> + Send + Sync>;

/// Manages all active sessions.
pub struct SessionManager {
    sessions: RwLock<HashMap<String, Arc<Mutex<Session>>>>,
    registry: Arc<ProviderRegistry>,
    dispatcher: Arc<ToolDispatcher>,
    ctx: ToolContext,
    // P1-04（2026-10-01, traecode）：`retriever`(A5) / `lsp_bridge`(A4) 字段已删除
    // —— 顶层裁决「删除」。它们在 agent-core 内**只被赋值、从未被读取**，
    // 注入后永不生效（详见 loop.rs 同处注释）。
    /// v11.0: Experience store injected into each new AgentLoop.
    experience_store: Option<Arc<ExperienceStore>>,
    /// D-175（2026-10-08, traecode）：经验库**工厂**——按 owner 构造 per-user 档。
    /// 设定后优先于 `experience_store`（后者退化为"无工厂时的回落单例"）。
    experience_factory: Option<ExperienceFactory>,
    /// P4: CostMeter for session-scoped token accounting.
    cost_meter: Arc<Mutex<CostMeter>>,
    /// P5: MemoryStore for session persistence across restarts.
    memory_store: Option<Arc<dyn MemoryStore>>,
    /// D-109（2026-10-02, traecode）：文明线写入器的**工厂**——按**归属用户**（owner）
    /// 构造该用户的写入器。旧实现是**单个全局** writer（写全局档
    /// `MEMORY_DIR/civilization.jsonl`），而 API 读侧读的是 per-user 档
    /// （`per_user.civ_for(uid)`）⇒ 自动写入的 milestone/reflection 在 API 上不可见。
    /// 现改为工厂：组合根在闭包内把 owner 绑定到 per-user store，写入与读取落到**同一个
    /// 文件**（写侧对齐读侧，跨租户隔离不破）。
    civ_writer_factory: Option<CivWriterFactory>,
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
            experience_store: None,
            experience_factory: None,
            cost_meter: Arc::new(Mutex::new(CostMeter::new())),
            memory_store: None,
            civ_writer_factory: None,
            observer: None,
        }
    }

    /// Q3 (v24-post): 注入 Observer（会话结束后报告落盘）。
    pub fn set_observer(&mut self, o: Arc<observer::Observer>) {
        self.observer = Some(o);
    }

    /// D-109（2026-10-02, traecode）：注入文明线写入器**工厂**——按归属用户（owner）
    /// 构造该用户的写入器。工厂在 [`SessionManager::create_session_with_owner`] 里
    /// 以该会话的 owner 调用一次；组合根（service）在闭包内把 owner 绑定到 per-user
    /// store，使 agent-loop 的自动写入与 API 读侧（`per_user.civ_for(uid)`）落到
    /// **同一个文件**（写侧对齐读侧；读写不会跨租户串档）。
    pub fn set_civ_writer_factory(&mut self, factory: CivWriterFactory) {
        self.civ_writer_factory = Some(factory);
    }

    /// v11.0: Set the experience store to inject into each new AgentLoop.
    pub fn set_experience_store(&mut self, store: Arc<ExperienceStore>) {
        self.experience_store = Some(store);
    }

    /// D-175（2026-10-08, traecode）：注入经验库**工厂**——按 owner 构造 per-user 档，
    /// 使经验条目（含会话目标文本）**按租户分区**。设工厂后 `experience_store` 单例
    /// 仅作无工厂时的回落。
    pub fn set_experience_factory(&mut self, factory: ExperienceFactory) {
        self.experience_factory = Some(factory);
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

    /// Create a new session（无归属上下文——owner 取 `"default"`）。
    ///
    /// D-109：需要按用户归属落档的调用方（HTTP handler）请用
    /// [`SessionManager::create_session_with_owner`]。
    pub async fn create_session(
        &self,
        req: SessionCreate,
    ) -> anyhow::Result<SessionCreateResponse> {
        self.create_session_with_owner(req, "default").await
    }

    /// D-109（2026-10-02, traecode）：带**归属用户**的会话创建。`owner` 用于给
    /// agent-loop 注入**该用户作用域**的文明线写入器——否则自动写入的 milestone/
    /// reflection 会落进无人读取的全局档，在 `hearth civ feed` / API 上不可见。
    pub async fn create_session_with_owner(
        &self,
        req: SessionCreate,
        owner: &str,
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

        // P1-04：原 A4/A5 的 retriever / lsp_bridge 注入已删除（顶层裁决「删除」）。
        // v11.0: Inject experience store
        // D-175：**优先按 owner 构造 per-user 经验库**（工厂），否则回落全局单例——
        // 修复前恒用全局单例 ⇒ 多租户下所有租户的目标文本落进同一 `experience.jsonl`。
        if let Some(ref factory) = self.experience_factory {
            agent.set_experience_store(factory(owner));
        } else if let Some(ref store) = self.experience_store {
            agent.set_experience_store(store.clone());
        }
        // D-109: 注入**按 owner 构造**的文明线写入器——写入落到该用户的可见档
        //（`per_user.civ_for(owner)`）。run() 收尾由 `note_civ_outcome` 单点调用。
        if let Some(ref factory) = self.civ_writer_factory {
            agent.set_civ_writer(factory(owner));
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
            owner: owner.to_string(),
            phase: "created".into(),
            steps: 0,
            budget_remaining: Some(max_steps),
            event_tx: Some(event_tx),
            events: Vec::new(),
            events_base_seq: 0,
            running: false,
            agent: Some(agent),
            budget,
            cancel_tx: None,
            finished_at: None,
            created_at: std::time::Instant::now(),
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
                owner: owner.to_string(),
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
        self.session_events_with_base(id).await.1
    }

    /// D-101：取事件缓冲副本 + **绝对 seq 基址**（基址 = 已丢弃的事件数）。
    pub async fn session_events_with_base(&self, id: &str) -> (u64, Vec<AgentEvent>) {
        if let Some(session) = self.get_session(id).await {
            let s = session.lock().await;
            (s.events_base_seq, s.events.clone())
        } else {
            (0, Vec::new())
        }
    }

    /// WP-2 (v23 phase3): 录制导出——会话事件缓冲 → 带信封的 JSONL 行。
    pub async fn session_events_jsonl(&self, id: &str) -> Vec<String> {
        let (base, events) = self.session_events_with_base(id).await;
        let mut env = crate::envelope::EnvelopeState::new();
        env.seq = base; // D-101：保持绝对编号
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
    ///
    /// P0-08（2026-10-01, traecode）：读取加了**字节**上限（见 [`MAX_ARTIFACT_BYTES`]）。
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
        let (mut content, truncated) = read_file_text_capped(&full, MAX_CAPTURED_BYTES as u64)
            .await
            .map_err(|e| anyhow::anyhow!("read artifact {rel_path} failed: {e}"))?;
        // P0-08：截断**不静默**——预览者必须知道自己看的不是全貌。
        if truncated {
            content.push_str(&format!(
                "\n\n[hearth] 产物超过 {} MiB，预览仅显示前 {} MiB（要看完整内容请在会话工作区内直接打开该文件）。\n",
                MAX_CAPTURED_BYTES / (1024 * 1024),
                MAX_CAPTURED_BYTES / (1024 * 1024)
            ));
        }
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
    ///
    /// D-171（2026-10-08, traecode）：**按归属用户过滤**（多租户隔离）。修复前无任何
    /// 归属条件 ⇒ 在 D-169 开启的 `HEARTH_USERS` 多租户下，任一租户都能在列表里看到
    /// 其他租户的会话 id（再用 id 直读事件/历史即得对方目标文本）。内存侧按 `Session.owner`，
    /// 持久化侧按记录头 `owner`（经 `session_owner` 轻量读——不加载事件体）。
    pub async fn list_sessions_json_for(&self, owner: &str) -> Vec<serde_json::Value> {
        let mut out: Vec<serde_json::Value> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        {
            let map = self.sessions.read().await;
            for (id, sess) in map.iter() {
                let (phase, steps, budget_remaining, sess_owner) = {
                    let g = sess.lock().await;
                    (
                        g.phase.clone(),
                        g.steps,
                        g.budget_remaining,
                        g.owner.clone(),
                    )
                };
                seen.insert(id.clone());
                if sess_owner != owner {
                    continue;
                }
                out.push(serde_json::json!({
                    "id": id,
                    "status": phase,
                    "steps": steps,
                    "budget_remaining": budget_remaining,
                }));
            }
        }
        // 持久化但未加载的会话（P1-1 读路径接线后可见）——D-171：仅列**属于本租户**者。
        if let Ok(ids) = self.list_persisted_sessions().await {
            for id in ids {
                if seen.contains(&id) {
                    continue;
                }
                if self.persisted_owner(&id).await.as_deref() != Some(owner) {
                    continue;
                }
                out.push(serde_json::json!({
                    "id": id, "status": "persisted", "steps": 0, "budget_remaining": null,
                }));
            }
        }
        out
    }

    /// D-171（2026-10-08, traecode）：会话**归属用户**查询。内存命中取 `Session.owner`；
    /// 未命中回落持久化记录头（`MemoryStore::session_owner` 轻量读）。`None` = 无法判定归属
    /// （会话不存在 / 无 store / 读失败）——调用方一律按"不存在"处理（fail closed）。
    pub async fn owner_of(&self, id: &str) -> Option<String> {
        if let Some(sess) = self.get_session(id).await {
            return Some(sess.lock().await.owner.clone());
        }
        self.persisted_owner(id).await
    }

    /// D-171：从持久化层读归属用户（无 store / 记录缺失 / I/O 错 → `None`）。
    async fn persisted_owner(&self, id: &str) -> Option<String> {
        let store = self.memory_store.as_ref()?;
        store.session_owner(id).await.ok().flatten()
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

        // D-178（2026-10-08, traecode）：**终态/在途会话不再接受消息**。
        //
        // 病灶：本函数是**单发**设计——首次 `send_message` 会把 `s.agent` **取走**（`agent.take()`）。
        // 对**已完成**（`finished_at.is_some()`）或**已在跑**（`running == true`）的会话再发一次，
        // 会：① 走进 `agent None` 分支把 phase **改写为 "error"**、`finished_at` 重置 —— 与 D-159
        // 同族（"终态不可变"被破坏：丢掉真实完成时间、重置 TTL 淘汰时钟）；② 把**成功完成**的会话
        // 对外报成 error（假故障）。CLI 侧早已**自行**兜底（`lib.rs` 对 `status==done/cancelled`
        // 拒绝 resume、提示"会话已结束"）——恰因服务端缺这道守卫。
        // 判据与 D-159/D-152 同源：`finished_at.is_some()` = 已落终态；`running` = 在途。
        {
            let s = session.lock().await;
            if s.running || s.finished_at.is_some() {
                anyhow::bail!("session already finished or running: {}", id);
            }
        }

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
                        session_clone.lock().await.push_event(api_evt);
                            if is_done {
                                break;
                            }
                        }
                        _ = &mut cancel_rx => {
                            let ev = AgentEvent::Error {
                                message: "session cancelled".into(),
                            };
                            let _ = tx.send(ev.clone());
                            session_clone.lock().await.push_event(ev);
                            // X4 fix: emit Done so the SSE stream closes cleanly
                            // like the normal completion path, instead of relying
                            // on the keepalive timeout to drop the connection.
                            let ev = AgentEvent::Done {
                                report: serde_json::json!({"ok": false, "status": "cancelled"}),
                            };
                            let _ = tx.send(ev.clone());
                            session_clone.lock().await.push_event(ev);
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
                                owner: s.owner.clone(),
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
                            env.seq = s.events_base_seq; // D-101：保持绝对编号
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
                        s.push_event(ev);
                        let ev = AgentEvent::Done {
                            report: serde_json::json!({"ok": false, "status": "error"}),
                        };
                        let _ = tx.send(ev.clone());
                        s.push_event(ev);
                    }
                    Err(_) => {
                        s.phase = "error".into();
                        let ev = AgentEvent::Error {
                            message: "agent panicked or was cancelled".into(),
                        };
                        let _ = tx.send(ev.clone());
                        s.push_event(ev);
                        let ev = AgentEvent::Done {
                            report: serde_json::json!({"ok": false, "status": "panic"}),
                        };
                        let _ = tx.send(ev.clone());
                        s.push_event(ev);
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
                session_clone.lock().await.push_event(ev);
                let ev = AgentEvent::Done {
                    report: serde_json::json!({"ok": false, "status": "error"}),
                };
                let _ = tx.send(ev.clone());
                session_clone.lock().await.push_event(ev);
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
            s.push_event(ev);
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
        // D-159（2026-10-05, traecode）：已完成（终态）的会话，cancel 必须是**幂等 no-op**。
        // 修复前会无条件执行下面的 `finished_at = now` 与 workspace 清理 ⇒ ① 丢失真实完成时间、
        // 重置 TTL 淘汰时钟；② 立即删掉 workspace（连同产物，早于 TTL 清理）——破坏"终态不可变"。
        // 判据用 `finished_at.is_some()`：**运行中**与**从未启动**的会话其 `finished_at` 均为 `None`
        // （前者由任务收信号后落终态、后者由本处落 `cancelled`，见 D-152），故不受本守卫影响。
        if s.finished_at.is_some() {
            return Ok(());
        }
        // D-152（2026-10-04, traecode）：先记下**是否有运行中任务可收信号**——它决定终态由谁落定。
        let signaled = if let Some(tx) = s.cancel_tx.take() {
            let _ = tx.send(());
            true
        } else {
            false
        };
        // Mark not running; the task sets phase to "cancelled" upon signal receipt.
        s.running = false;
        if !signaled {
            // 从未启动（`cancel_tx == None`，无任务可收信号）⇒ **由本处落定终态**。
            // 修复前此处不置 `phase`，于是 `phase` 永远停在 `"created"`：活体上
            // `hearth cancel <id>` 打印 `cancelled <id>`（rc=0），而 `hearth status <id>`
            // 仍显示 `"phase": "created"`——CLI 报成功、状态却没变（假成功）。
            // 有任务在跑时**不在此处置**：仍由任务收信号后自行落 `phase`，避免与之竞态。
            s.phase = "cancelled".into();
        }
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
        // E2: 是否随会话一起清理工作区目录（默认清理；`CODEX_KEEP_WORKSPACES=1` 保留）。
        let cleanup_ws = std::env::var("CODEX_KEEP_WORKSPACES")
            .map(|v| v != "1")
            .unwrap_or(true);
        guard.retain(|_id, sess| match sess.try_lock() {
            Ok(s) => match s.finished_at {
                // Finished: keep only while within TTL.
                Some(t) => {
                    let expired = now.duration_since(t) > ttl;
                    if expired && cleanup_ws {
                        if let Err(e) = std::fs::remove_dir_all(&s.workspace_dir) {
                            tracing::warn!(
                                "E2: cleanup_finished: failed to remove {:?}: {e}",
                                s.workspace_dir
                            );
                        }
                    }
                    !expired
                }
                // P0-04（2026-10-01, traecode）：**从未启动**的会话 `finished_at` 恒为 `None`。
                // 原逻辑「Active: always keep」使其**永不回收** → 内存 map 与
                // `sessions/{uuid}` 工作区目录**双向无界增长**（反复 `POST /api/v1/sessions`
                // 且从不 `send_message` 即构成 DoS）。
                // 现按 idle TTL（自 `created_at` 起算）回收**非运行中**者；
                // `running == true`（真的在跑）的会话**仍然不触碰**。
                None => {
                    let idle_expired = now.duration_since(s.created_at) > ttl;
                    let evict = !s.running && idle_expired;
                    if evict && cleanup_ws {
                        if let Err(e) = std::fs::remove_dir_all(&s.workspace_dir) {
                            tracing::warn!(
                                "P0-04: cleanup: failed to remove idle workspace {:?}: {e}",
                                s.workspace_dir
                            );
                        }
                    }
                    !evict
                }
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

    /// D-161（2026-10-05, traecode）：参与者名是否可被 registry 解析。
    ///
    /// 与 `bridge` 内部实际使用的 `registry.get()` **同一解析语义**（含别名 / `name:model`
    /// 形态），供服务端在**入参边界**校验 bridge 参与者名字——避免把"客户端拼错 provider 名"
    /// 一路放进 bridge、再由 `map_err` 冒充成 500 服务端故障（见 `routes::create_bridge`）。
    pub fn has_provider(&self, name: &str) -> bool {
        self.registry.get(name).is_ok()
    }
}

/// Map agent-core internal events to API AgentEvent types.
pub fn map_event(evt: Event) -> AgentEvent {
    match evt {
        // S5：Event::Phase 载荷已是标签字符串（相位机拆除后不再包装 enum）。
        Event::Phase(phase) => AgentEvent::Phase { phase },
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

    // D-33 收敛：原 P0-08 的 3 条 `read_text_capped` 辅助单测已随之搬到
    // `bounded_io::tests`（实现去哪、测试去哪）。

    /// OOM-1 regression: finished sessions past TTL are evicted; active
    /// sessions and recently-finished sessions are kept.
    #[tokio::test]
    async fn test_v12_cleanup_finished_evicts_only_expired() {
        let registry = Arc::new(ProviderRegistry::new());
        let dispatcher = Arc::new(ToolDispatcher::new());
        let ctx = ToolContext::default();
        let mgr = SessionManager::new(registry, dispatcher, ctx);

        // Manually insert three sessions: active / freshly-finished / long-finished.
        // P0-04（2026-10-01）：新增 `running` / `created` 两个维度，以覆盖
        // 「从未启动的会话也须回收」这条修复点。
        let mk = |id: &str,
                  running: bool,
                  finished: Option<std::time::Instant>,
                  created: std::time::Instant| {
            Arc::new(Mutex::new(Session {
                id: id.into(),
                provider_name: "p".into(),
                model: "m".into(),
                goal: "g".into(),
                owner: "default".into(),
                phase: "done".into(),
                steps: 0,
                budget_remaining: None,
                event_tx: None,
                events: Vec::new(),
                events_base_seq: 0,
                running,
                agent: None,
                budget: Budget::default(),
                cancel_tx: None,
                finished_at: finished,
                created_at: created,
                workspace_dir: std::path::PathBuf::from("/tmp/test"),
            }))
        };
        let now = std::time::Instant::now();
        {
            let mut guard = mgr.sessions.write().await;
            // 创建后**从未 send_message**：finished_at 恒 None（旧逻辑下永不回收）
            guard.insert("idle".into(), mk("idle", false, None, now));
            // 真在跑：running=true
            guard.insert("running".into(), mk("running", true, None, now));
            // 刚结束
            guard.insert("finished".into(), mk("finished", false, Some(now), now));
        }

        // 宽 TTL：一个都不应被回收。
        let evicted = mgr
            .cleanup_finished(std::time::Duration::from_secs(3600))
            .await;
        assert_eq!(evicted, 0, "宽 TTL 下不应回收任何会话");
        assert!(mgr.get_session("finished").await.is_some());

        // 零 TTL：finished 与 idle 均应被回收，running 必须保留。
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let evicted = mgr.cleanup_finished(std::time::Duration::ZERO).await;
        assert_eq!(evicted, 2, "finished 与 idle 都应被回收");
        assert!(mgr.get_session("finished").await.is_none());
        assert!(
            mgr.get_session("idle").await.is_none(),
            "P0-04 修复点：从未启动的会话必须被回收（修复前此处恒为 Some → 无界增长）"
        );
        assert!(
            mgr.get_session("running").await.is_some(),
            "running=true 的会话永不回收"
        );
    }

    /// D-152 回归锁（**先红后绿**）：`cancel_session` 对**从未启动**的会话（`cancel_tx == None`，
    /// 无运行中任务可收信号）必须**由本处落定终态** `phase = "cancelled"`。
    ///
    /// 修复前它只置 `running=false` / `finished_at`，`phase` 永远停在 `"created"` ⇒ 活体可复现
    /// 矛盾：`hearth cancel <id>` 打印 `cancelled <id>`（rc=0），而紧接着 `hearth status <id>`
    /// 仍显示 `"phase": "created"`——CLI 报成功、状态却没变（假成功）。
    #[tokio::test]
    async fn test_d152_cancel_never_started_sets_cancelled_phase() {
        let mgr = SessionManager::new(
            Arc::new(ProviderRegistry::new()),
            Arc::new(ToolDispatcher::new()),
            ToolContext::default(),
        );
        let ws = std::env::temp_dir().join("hearth-d152-ws");
        let _ = std::fs::create_dir_all(&ws);
        let session = Arc::new(Mutex::new(Session {
            id: "never-started".into(),
            provider_name: "p".into(),
            model: "m".into(),
            goal: "g".into(),
            owner: "default".into(),
            phase: "created".into(),
            steps: 0,
            budget_remaining: None,
            event_tx: None,
            events: Vec::new(),
            events_base_seq: 0,
            running: false,
            agent: None,
            budget: Budget::default(),
            cancel_tx: None, // 从未启动 ⇒ 无任务可收信号
            finished_at: None,
            created_at: std::time::Instant::now(),
            workspace_dir: ws,
        }));
        mgr.sessions
            .write()
            .await
            .insert("never-started".into(), session.clone());

        mgr.cancel_session("never-started")
            .await
            .expect("cancel 必须成功");

        let phase = session.lock().await.phase.clone();
        assert_eq!(
            phase, "cancelled",
            "从未启动的会话被 cancel 后 phase 必须是 cancelled（修复前恒为 created）——\
             否则 CLI 报 `cancelled` 而 status 显示 `created`，属假成功"
        );
    }

    /// D-159 回归锁（**先红后绿**）：对**已完成**（`finished_at.is_some()`）的会话，
    /// `cancel_session` 必须是**幂等 no-op**——不得改写 `finished_at`（否则丢失真实完成时间、
    /// 重置 TTL 淘汰时钟），也不得删除其 workspace（连同产物，早于 TTL 清理）。
    ///
    /// 现实形态：正常跑完的会话 `cancel_tx` 仍残留一个**已失效**的 sender（完成路径不清它），
    /// 于是 `cancel_session` 里 `signaled == true`（`send` 静默失败）——`phase` 不会被改写，
    /// 但 `finished_at = now` 与 `remove_dir_all(workspace_dir)` **照样执行** ⇒ 破坏终态。
    #[tokio::test]
    async fn test_d159_cancel_finished_session_is_noop() {
        let mgr = SessionManager::new(
            Arc::new(ProviderRegistry::new()),
            Arc::new(ToolDispatcher::new()),
            ToolContext::default(),
        );
        let ws = std::env::temp_dir().join(format!("hearth-d159-ws-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&ws);
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::write(ws.join("artifact.txt"), b"keep me").unwrap();

        let finished_at = std::time::Instant::now();
        // 残留的失效 sender（接收端已随任务结束被 drop）——镜像真实完成态。
        let (stale_tx, stale_rx) = oneshot::channel::<()>();
        drop(stale_rx);
        let session = Arc::new(Mutex::new(Session {
            id: "already-done".into(),
            provider_name: "p".into(),
            model: "m".into(),
            goal: "g".into(),
            owner: "default".into(),
            phase: "done".into(),
            steps: 3,
            budget_remaining: None,
            event_tx: None,
            events: Vec::new(),
            events_base_seq: 0,
            running: false,
            agent: None,
            budget: Budget::default(),
            cancel_tx: Some(stale_tx),
            finished_at: Some(finished_at),
            created_at: finished_at,
            workspace_dir: ws.clone(),
        }));
        mgr.sessions
            .write()
            .await
            .insert("already-done".into(), session.clone());

        mgr.cancel_session("already-done")
            .await
            .expect("已完成会话的 cancel 必须成功（幂等 no-op）");

        let s = session.lock().await;
        assert_eq!(s.phase, "done", "cancel 不得改写已完成会话的终态 phase");
        assert_eq!(
            s.finished_at,
            Some(finished_at),
            "cancel 不得改写已完成会话的 finished_at（否则丢失真实完成时间并重置 TTL 淘汰时钟）"
        );
        drop(s);
        assert!(
            ws.join("artifact.txt").exists(),
            "cancel 不得删除已完成会话的 workspace（连同产物，早于 TTL 清理）"
        );
        let _ = std::fs::remove_dir_all(&ws);
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
            owner: "default".into(),
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
            events_base_seq: 0,
            running: true,
            agent: None,
            budget: Budget::default(),
            cancel_tx: None,
            finished_at: None,
            created_at: std::time::Instant::now(),
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
                owner: "default".into(),
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

    /// D-101 regression: 事件缓冲达上限后**批量**丢弃最旧事件，绝对 seq 基址只增不减；
    /// 未越界时基址恒为 0、长度等于 push 条数（= 未截断时行为与历史版本逐字节一致）。
    #[tokio::test]
    async fn test_d101_event_buffer_capped_with_absolute_base() {
        let session = Arc::new(Mutex::new(Session {
            id: "cap-1".into(),
            provider_name: "p".into(),
            model: "m".into(),
            goal: "cap".into(),
            owner: "default".into(),
            phase: "running".into(),
            steps: 0,
            budget_remaining: None,
            event_tx: None,
            events: Vec::new(),
            events_base_seq: 0,
            running: true,
            agent: None,
            budget: Budget::default(),
            cancel_tx: None,
            finished_at: None,
            created_at: std::time::Instant::now(),
            workspace_dir: std::path::PathBuf::from("/tmp/test"),
        }));

        let mk = |i: u64| AgentEvent::Error {
            message: format!("evt-{i}"),
        };

        // 1) 未越界：基址恒 0，长度 = push 条数（绝对编号即位置，无偏移）。
        {
            let mut s = session.lock().await;
            for i in 0..100u64 {
                s.push_event(mk(i));
            }
            assert_eq!(s.events_base_seq, 0, "未越界时基址必须为 0");
            assert_eq!(s.events.len(), 100, "未越界时长度 = push 条数");
        }

        // 2) 恰越界一次：批量丢弃最旧事件，基址 > 0，且绝对编号不丢不重。
        let total: u64 = MAX_SESSION_EVENTS as u64 + 1;
        {
            let mut s = session.lock().await;
            for i in 100..total {
                s.push_event(mk(i));
            }
            assert!(
                s.events.len() <= MAX_SESSION_EVENTS,
                "截断后长度须维持在上限内"
            );
            assert!(s.events_base_seq > 0, "发生截断后基址须 > 0");
            // 不变量：基址（已丢弃）+ 缓冲长度（保留）== 总 push 条数（不丢不重）。
            assert_eq!(
                s.events_base_seq + s.events.len() as u64,
                total,
                "绝对编号须不丢不重"
            );
        }
    }

    /// D-171 回归锁（**先红后绿**）：会话**归属**必须可判且**列表按租户过滤**。
    ///
    /// 修复前 `owner_of` / 归属过滤**都不存在**（`list_sessions_json()` 无 owner 参数）
    /// ⇒ 多租户下任一租户可见全部会话。本测试锁三件事：① `owner_of` 对**内存**会话返回
    /// `Session.owner`；② 对**仅持久化**（不在内存表）的会话回落记录头判归属（重启场景）；
    /// ③ `list_sessions_json_for` 内存与持久化**两侧**都只列本租户。
    #[tokio::test]
    async fn test_d171_owner_of_and_scoped_list() {
        let mut mgr = SessionManager::new(
            Arc::new(ProviderRegistry::new()),
            Arc::new(ToolDispatcher::new()),
            ToolContext::default(),
        );
        let tmp = tempfile::tempdir().unwrap();
        let store = Arc::new(JsonlMemoryStore::new(tmp.path()));
        // 一条 alice 的**持久化**会话（不在内存表 ⇒ 走 owner_of 的持久化回落）。
        store
            .save_session(&SessionRecord {
                session_id: "p-alice".into(),
                provider_name: "p".into(),
                model: "m".into(),
                goal: "g".into(),
                owner: "alice".into(),
                events: vec![],
                created_at: "t".into(),
            })
            .await
            .unwrap();
        mgr.set_memory_store(store);

        // 内存里挂一条 bob 的会话。
        let sess = Arc::new(Mutex::new(Session {
            id: "m-bob".into(),
            provider_name: "p".into(),
            model: "m".into(),
            goal: "g".into(),
            owner: "bob".into(),
            phase: "created".into(),
            steps: 0,
            budget_remaining: None,
            event_tx: None,
            events: Vec::new(),
            events_base_seq: 0,
            running: false,
            agent: None,
            budget: Budget::default(),
            cancel_tx: None,
            finished_at: None,
            created_at: std::time::Instant::now(),
            workspace_dir: std::path::PathBuf::from("/tmp/test"),
        }));
        mgr.sessions.write().await.insert("m-bob".into(), sess);

        // ① 内存命中；② 持久化回落；③ 未知 → None。
        assert_eq!(mgr.owner_of("m-bob").await.as_deref(), Some("bob"));
        assert_eq!(
            mgr.owner_of("p-alice").await.as_deref(),
            Some("alice"),
            "仅持久化的会话必须能从记录头判归属（重启后仍隔离）"
        );
        assert!(mgr.owner_of("nope").await.is_none());

        // ④ 列表按 owner 过滤：alice 只见她的持久化会话，bob 只见他的内存会话。
        let ids_for = |v: &[serde_json::Value]| -> Vec<String> {
            v.iter()
                .filter_map(|x| x["id"].as_str().map(|s| s.to_string()))
                .collect()
        };
        assert_eq!(
            ids_for(&mgr.list_sessions_json_for("alice").await),
            vec!["p-alice".to_string()],
            "alice 的列表不得含 bob 的会话"
        );
        assert_eq!(
            ids_for(&mgr.list_sessions_json_for("bob").await),
            vec!["m-bob".to_string()],
            "bob 的列表不得含 alice 的会话（修复前全局可见 = 跨租户泄露）"
        );
    }
}
