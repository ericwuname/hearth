//! D1/D2 (hearth-cli): 进程内直跑——CLI 直接驱动内核（派 A 单二进制）。
//!
//! 不经过 service HTTP——组装 provider/dispatcher → AgentLoop → 事件流渲染 +
//! 内联审批（dispatcher.resolve_interaction 进程内直喂）。RT3 fail-closed
//! sandbox 随 BashTool 等默认注入。

use std::sync::Arc;

use agent_core::{Agent, AgentLoop, Goal};
use agent_types::Budget;
use anyhow::{Context, Result};
use api::AgentEvent;
use colored::Colorize;
use llm_gateway::ProviderRegistry;
use tool_runtime::{ToolContext, ToolDispatcher};

use crate::config::ResolvedConfig;
use crate::render;

/// D4: 沙箱隔离徽章——Linux 真隔离 / 非 Linux noop。
pub fn isolation_badge() -> String {
    #[cfg(target_os = "linux")]
    {
        "🔒 landlock+seccomp (fail-closed)".green().to_string()
    }
    #[cfg(not(target_os = "linux"))]
    {
        "⚠️ noop · 仅开发模式（无真实沙箱）".yellow().to_string()
    }
}

/// v0.1.1: cgroup 可用性探测——尝试在 base（HEARTH_CGROUP_BASE 或默认）创建目录。
/// 不可用返回提示（启动即警告），可写返回 None。
#[cfg(target_os = "linux")]
pub(crate) fn cgroup_warning() -> Option<String> {
    let base = std::env::var("HEARTH_CGROUP_BASE").unwrap_or_else(|_| "/sys/fs/cgroup".to_string());
    let base = std::path::Path::new(&base);
    let base_disp = base.display().to_string();
    if !base.join("cgroup.controllers").exists() {
        return Some(format!(
            "{base_disp} 无 cgroup v2——资源限制不会生效。设 HEARTH_CGROUP_BASE=<delegation 子树> 或 HEARTH_ALLOW_NO_CGROUP=1 显式降级"
        ));
    }
    let probe = base.join(format!(".hearth-probe-{}", std::process::id()));
    match std::fs::create_dir(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_dir(&probe);
            None
        }
        Err(e) => Some(format!(
            "{base_disp} 不可写（{e}）——资源限制不会生效。设 HEARTH_CGROUP_BASE=<delegation 子树> 或 HEARTH_ALLOW_NO_CGROUP=1 显式降级"
        )),
    }
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn cgroup_warning() -> Option<String> {
    None
}

/// 按 provider 名组装 LlmProvider（deepseek/openai/ollama/vllm + openai-compatible）。
pub(crate) fn build_provider(cfg: &ResolvedConfig) -> Result<Arc<dyn llm_gateway::LlmProvider>> {
    // R2-C 采集器 v2: 出口统一装饰（全出口单点记录——补充 1）
    Ok(wrap_telemetry(build_provider_inner(cfg)?))
}

fn build_provider_inner(cfg: &ResolvedConfig) -> Result<Arc<dyn llm_gateway::LlmProvider>> {
    // S9（手术包二）：配置了多通道降级链（providers 有序数组，len>1）→
    // FallbackChain 包装：通道级不可恢复错误（401/403/400/畸形流/策略）自动切
    // 下一通道（投影 `[fallback] provider: a → b`——降级数据必须标通道，换
    // provider=换模型层，基准对照不可比）；transient 交回 S7 长退避。
    if cfg.providers.len() > 1 {
        let mut items: Vec<(String, Arc<dyn llm_gateway::LlmProvider>)> = Vec::new();
        for (i, name) in cfg.providers.iter().enumerate() {
            let key = cfg
                .provider_keys
                .get(name)
                .cloned()
                .or_else(|| cfg.api_key.clone());
            // 首通道沿用显式 url（--url/config）；其余通道走各自默认端点。
            let url = if i == 0 { cfg.url.clone() } else { None };
            let p = build_single_provider(name, None, url, key)?;
            items.push((name.clone(), p));
        }
        let chain = llm_gateway::FallbackChain::new(items).with_switch_callback(Arc::new(
            |from: &str, to: &str| {
                eprintln!(
                    "  🔁 [fallback] provider: {from} → {to}（降级链——数据按通道分账，基准对照不可比）"
                );
            },
        ));
        return Ok(Arc::new(chain));
    }
    build_single_provider(
        &cfg.provider,
        cfg.model.clone(),
        cfg.url.clone(),
        cfg.api_key.clone(),
    )
}

/// S9：单通道构造（原 match 抽出——链模式复用；key 缺失给可行动错误）。
fn build_single_provider(
    name: &str,
    model: Option<String>,
    url: Option<String>,
    api_key: Option<String>,
) -> Result<Arc<dyn llm_gateway::LlmProvider>> {
    let model = model.unwrap_or_else(|| match name {
        "deepseek" => "deepseek-v4-flash".into(),
        "openai" => "gpt-4o".into(),
        "gemini" => "gemini-3.6-flash".into(),
        "agnes" => "agnes-2.5-flash".into(),
        "ollama" => "qiyuan-8b".into(),
        _ => "default".into(),
    });
    match name {
        "deepseek" => {
            let key = provider_key_or_bail(name, &api_key)?;
            let url = url.unwrap_or_else(|| "https://api.deepseek.com/v1".into());
            Ok(Arc::new(llm_openai::OpenAiProvider::new(
                "deepseek",
                model,
                Some(url),
                key,
            )))
        }
        // R7 (v0.1.4): gemini 通道——OpenAI 兼容端点（generativelanguage /v1beta/openai）。
        "gemini" => {
            let key = provider_key_or_bail(name, &api_key)?;
            let url = url.unwrap_or_else(|| {
                "https://generativelanguage.googleapis.com/v1beta/openai".into()
            });
            Ok(Arc::new(llm_openai::OpenAiProvider::new(
                "gemini",
                model,
                Some(url),
                key,
            )))
        }
        "openai" => {
            let key = provider_key_or_bail(name, &api_key)?;
            Ok(Arc::new(llm_openai::OpenAiProvider::new(
                "openai", model, url, key,
            )))
        }
        "ollama" => {
            let url = url.unwrap_or_else(|| "http://localhost:11434".into());
            Ok(Arc::new(llm_local::OllamaProvider::new(
                "ollama",
                model,
                Some(url),
            )))
        }
        "vllm" => {
            let url = url.context("vllm 需要 url（--url 或 config set url）——下一步: hearth config set url http://localhost:8000/v1")?;
            Ok(Arc::new(llm_local::VllmProvider::new(
                "vllm",
                model,
                Some(url),
                api_key,
            )))
        }
        // T1 (v0.2.3): agnes——OpenAI 兼容，默认端点/模型（用户会员通道）
        "agnes" => {
            let key = provider_key_or_bail(name, &api_key)?;
            let url = url.unwrap_or_else(|| "https://api.agnes-ai.cn/v1".into());
            Ok(Arc::new(llm_openai::OpenAiProvider::new(
                "agnes",
                model,
                Some(url),
                key,
            )))
        }
        other => anyhow::bail!(
            "未知 provider: {other}——可用: deepseek / openai / gemini / agnes / ollama / vllm。\n\
             下一步: hearth config set provider deepseek"
        ),
    }
}

/// S9：通道 key 缺失 → 可行动错误（含 provider_keys 写法；key 永不入 git）。
fn provider_key_or_bail(name: &str, api_key: &Option<String>) -> Result<String> {
    match api_key.as_deref() {
        Some(k) if !k.is_empty() => Ok(k.to_string()),
        _ => anyhow::bail!(
            "未配置 API key（通道 {name}）。\n\
             下一步（任选其一）：\n\
             1. hearth config set api-key <你的key>\n\
             2. export HEARTH_API_KEY=<你的key>\n\
             3. config.toml 增加 [provider_keys] {name} = \"<key>\"（权限 600，永不入 git）"
        ),
    }
}

/// R2-C 采集器 v2（守门员补充 1）: provider 构造出口统一包 TelemetryProvider——
/// loop 的 plan-chat 与 planner 直连 chat（decompose/reflect）共用同一实例，
/// 一处装饰覆盖全部出口（v1 只接 plan 出口，57.6% 是有偏下界）。
/// env HEARTH_CACHE_TELEMETRY 开关（默认关零开销）。
fn wrap_telemetry(p: Arc<dyn llm_gateway::LlmProvider>) -> Arc<dyn llm_gateway::LlmProvider> {
    Arc::new(llm_gateway::TelemetryProvider::wrap(p, "cli"))
}

/// X1-3/X1-4 (v0.1.6): 重建 AgentLoop（REPL 崩溃恢复 + resume 本地恢复共用）。
/// 同一 session_id——历史经 restore_history 灌回后可继续对话。
pub(crate) fn rebuild_agent(
    cfg: &ResolvedConfig,
    session_id: &str,
    budget: u64,
) -> anyhow::Result<agent_core::AgentLoop> {
    let provider = build_provider(cfg)?;
    let workspace = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let dispatcher = build_dispatcher(workspace.clone());
    let goal0 = agent_core::Goal::with_budget(
        "conversation".to_string(),
        agent_types::Budget {
            max_steps: budget,
            ..Default::default()
        },
    );
    let snap_cwd = workspace.clone();
    let mut fresh = agent_core::AgentLoop::new(
        provider.clone(),
        Arc::new(planner::DefaultPlanner::new(provider)),
        dispatcher,
        ToolContext {
            cwd: workspace,
            env: tool_env(cfg),
            ..Default::default()
        },
        goal0,
    );
    fresh.set_session_id(session_id.to_string());
    attach_session_safety(&mut fresh, session_id, &snap_cwd);
    Ok(fresh)
}

/// PC-2 修复（P0/P1 修复任务书 v1.0）：resume **预算追加**——总预算 = 断点已用
/// + 追加额度。语义：steps 接着数（agent 侧 resume_keep_steps 恢复旧计数），
/// 追加的部分是净余量（`hearth resume <id> --budget N` 的 N 默认 = REPL_BUDGET）。
/// 纯函数（CLI resume 分支调用；无副作用可单测）。
pub(crate) fn resume_budget(steps_used: u64, extra: u64) -> u64 {
    steps_used.saturating_add(extra)
}

/// S3（P5-FOUNDATION-01 N13）：turn 级 checkpoint——每个工具交换原子落盘。
/// 此前快照只在 run() Ok 收尾（下方 Ok 分支），Ctrl-C/panic/kill 丢整轮进度；
/// 回调内部 save_snapshot 为 tmp+rename 原子写，失败不阻断主路径。
pub(crate) fn attach_session_safety(
    agent: &mut agent_core::AgentLoop,
    sid: &str,
    cwd: &std::path::Path,
) {
    let ck_sid = sid.to_string();
    let snap_sid = sid.to_string();
    let ck_cwd = cwd.to_path_buf();
    agent.set_on_turn_checkpoint(Box::new(move |cp| {
        if let Err(e) = crate::session_store::save_snapshot(&ck_sid, cp.turns) {
            tracing::warn!(error = %e, "turn checkpoint 落盘失败（不阻断）");
        }
        // R6-8（判定权归还长程任务书 v1.0）：one-shot checkpoint——taskgoal 随
        // 同一次交换增量落盘。R7-5/D-4（线C手术）：task_graph 落盘已删（图本体
        // 消失，载荷缩编 turns+taskgoal——申报待批）。
        let rev = crate::session_store::load_taskgoal(&ck_sid)
            .map(|(r, _)| r + 1)
            .unwrap_or(1);
        if let Err(e) = crate::session_store::save_taskgoal(&ck_sid, &cp.taskgoal, rev) {
            tracing::warn!(error = %e, "taskgoal checkpoint 落盘失败（不阻断）");
        }
        // S8（手术包二）：run 级断点状态同点落盘（steps/预算位/产物/scratch）——
        // kill -9 后 `hearth resume <sid>` 恢复执行位（不止历史）。
        if let Err(e) = crate::session_store::save_run_state(&ck_sid, &cp.run_state) {
            tracing::warn!(error = %e, "run 断点状态落盘失败（不阻断）");
        }
        // PC-2 修复：workspace 镜像（`.hearth/runs/<sid>.json`）——断点在项目内
        // 可发现（实测 PC-2 附带缺陷：断点只落 config 区，执行窗找不到）。
        crate::session_store::save_run_state_mirror(&ck_cwd, &ck_sid, &cp.run_state);
    }));
    // S2：写前快照（改前内容 → ~/.config/hearth/snapshots/<sid>/）
    let snap_cwd = cwd.to_path_buf();
    agent.set_on_pre_write_snapshot(Box::new(move |rel| {
        if let Err(e) = crate::snapshot_store::snapshot_before_write(&snap_sid, &snap_cwd, rel) {
            tracing::warn!(error = %e, "改前快照失败（不阻断）");
        }
    }));
}

/// WS9 (v0.2): 目标方向启发式打档——设计/美化/性能/游戏/应用类给 premium（大预算），
/// 超短目标给 economy，其余 standard。返回 (tier, min_steps 基线)。
fn goal_tier(goal: &str) -> (String, u64) {
    let g = goal.to_lowercase();
    if [
        "美化", "设计", "性能", "优化", "游戏", "应用", "完整", "好看", "ui", "界面", "web", "网站",
    ]
    .iter()
    .any(|k| g.contains(k))
    {
        ("premium".into(), 100)
    } else if g.chars().count() <= 8 {
        ("economy".into(), 20)
    } else {
        ("standard".into(), 50)
    }
}

/// D1: 组装工具调度器——RT3 fail-closed sandbox 随 BashTool 默认注入。
/// T10 (v0.2.3): 工具环境变量——出网白名单注入 ctx.env（web_fetch 的唯一读取来源）。
/// 修复"配了白名单仍全拒"结构性 bug：此前 ToolContext.env 全空，白名单永不生效。
pub(crate) fn tool_env(
    cfg: &crate::config::ResolvedConfig,
) -> std::collections::HashMap<String, String> {
    let mut env = std::collections::HashMap::new();
    if !cfg.egress_allowlist.is_empty() {
        env.insert(
            "HEARTH_EGRESS_ALLOWLIST".to_string(),
            cfg.egress_allowlist.join(","),
        );
    }
    // RC25 (P3-BACKLOG): 读范围白名单注入——config read_roots 显式设置时替换默认根
    if let Some(roots) = &cfg.read_roots {
        if !roots.is_empty() {
            env.insert("HEARTH_READ_ROOTS".to_string(), roots.join(","));
        }
    }
    env
}

pub(crate) fn build_dispatcher(cwd: std::path::PathBuf) -> Arc<ToolDispatcher> {
    let mut dispatcher = ToolDispatcher::new();
    // H1 (v0.2.4): bash 用声明超时注册（610s 窗口）——dispatcher 默认 30s 总闸
    // 曾把一切长跑命令（cargo build/test）杀掉（手工实测 30s 超时×N）。
    dispatcher.register_with_declared_timeout(Arc::new(tools_builtin::BashTool::new()));
    dispatcher.register(Arc::new(tools_builtin::ReadTool::new()));
    dispatcher.register(Arc::new(tools_builtin::EditTool::new()));
    // WS2 (v0.2): SEARCH/REPLACE diff 编辑——小步精确、无截断
    dispatcher.register(Arc::new(tools_builtin::PatchTool::new()));
    dispatcher.register(Arc::new(tools_builtin::GlobTool::new()));
    dispatcher.register(Arc::new(tools_builtin::GrepTool::new()));
    // R6-3: TodoWrite——模型自持任务清单（判定权归还：框架不再用分解/停滞
    // 计数替模型管任务，清单活在对话历史里，框架零状态）。
    dispatcher.register(Arc::new(tools_builtin::TodoWriteTool::new()));
    // WS8 (v0.2): introspect 体感工具——LLM 主动查询身体状态（steps/预算/上下文填充）
    dispatcher.register(Arc::new(tools_builtin::IntrospectTool::new()));
    // WS10 (v0.2): web_fetch 受控联网——deny-by-default 出网白名单（HEARTH_EGRESS_ALLOWLIST）
    dispatcher.register(Arc::new(tools_builtin::WebTool::new()));
    // S13（手术包二）：web_search 内置联网搜索——查资料的一等公民（零 key：
    // DuckDuckGo HTML 主源 + 必应回落；出网走 S2 默认放开，显式白名单时收紧）。
    dispatcher.register(Arc::new(tools_builtin::WebSearchTool::new()));
    let _ = cwd;
    Arc::new(dispatcher)
}

/// D2: `hearth chat <goal>` 直跑主入口。
///
/// 返回 (session_id, report) 供调用方（如 D5 落盘）。
/// RC24-B/C: `approve_within`（`--approve-within session`）→ 会话级审批委托；
/// 非 tty stdin（headless）→ 非交互审批策略（结构化秒级拒绝，RC24-B）。
pub async fn run_local(
    cfg: &ResolvedConfig,
    goal_text: &str,
    budget: u64,
    approve_within: bool,
    acceptance: Vec<String>,
) -> Result<(String, serde_json::Value)> {
    use agent_core::ApprovalPolicy;
    use std::io::IsTerminal as _;

    // R3/零配置：远程 provider 缺 key → 可行动错误（build_provider 内 require）。
    let provider = build_provider(cfg)?;

    let session_id = uuid::Uuid::new_v4().to_string();
    let workspace = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let dispatcher = build_dispatcher(workspace.clone());

    // D4: 徽章——真实隔离或明确 noop，绝无"假装隔离"。
    println!("{}  (session {})", isolation_badge(), &session_id[..8]);
    // v0.1.1 用户真机（五子棋/贪吃蛇失败根因）：cgroup 不可用（VM 非特权 delegation
    // 未配 HEARTH_CGROUP_BASE）时工具全被 fail-closed 拦——**启动即警告**，
    // 用户不用等任务跑完才发现。非静默降级（与 RT4 语义一致）。
    if let Some(w) = cgroup_warning() {
        render::info(&format!("⚠ cgroup: {w}"));
    }

    let goal = Goal::with_budget(
        goal_text.to_string(),
        Budget {
            max_steps: budget,
            ..Budget::default()
        },
    );
    let mut agent = AgentLoop::new(
        provider.clone(),
        Arc::new(planner::DefaultPlanner::new(provider)),
        dispatcher.clone(),
        ToolContext {
            cwd: workspace.clone(),
            env: tool_env(cfg),
            ..ToolContext::default()
        },
        goal,
    );
    // RC24-B/C: 审批策略注入——显式委托 > 非交互 deny-all > 默认逐条审批。
    if approve_within {
        agent.set_approval_policy(ApprovalPolicy::DelegateSession);
        render::info("  🔓 会话级审批委托已开启（--approve-within session）——命令表级破坏性操作自动放行（逐条审计）；fork bomb/设备写/内核接口仍需审批");
    } else if !std::io::stdin().is_terminal() {
        agent.set_approval_policy(ApprovalPolicy::DenyAllNonInteractive);
        render::info("  🤖 非交互模式（stdin 非 tty）——破坏性操作触发审批时将立即结构化拒绝（approval_denied_noninteractive）");
    }
    let (_, report) = run_local_continue(agent, &session_id, goal_text, budget, acceptance).await?;
    Ok((session_id, report))
}

/// B2 (v0.1.3): 连续对话直跑——复用同一 AgentLoop（保留 ctx_mgr 历史），
/// 每轮 run 后拿回 agent 状态。REPL 每轮调用（对齐 Codex Thread/Turn）。
/// X1-3 (v0.1.6): 返回 Option<AgentLoop>——agent panic 时 None（崩溃隔离，不杀进程）。
/// 返回 (agent, report)——agent 供下一轮复用（None=本轮崩溃，需重建）。
pub async fn run_local_continue(
    mut agent: AgentLoop,
    session_id: &str,
    goal_text: &str,
    budget: u64,
    acceptance: Vec<String>,
) -> Result<(Option<AgentLoop>, serde_json::Value)> {
    let workspace = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    // v0.1.2 (hearth-harness-review-supplement 补充5): run 起始时刻——transcript 记 wall_secs
    let run_started = std::time::Instant::now();

    // P2-LR Node 07（P2-MC OPEN-4 闭环，砺批-2 预判命中）：主路径 session 绑定——
    // 此前只有 rebuild_agent（崩溃恢复/resume）调 set_session_id，chat 首轮的
    // AgentLoop 从未绑定 → 首次压缩归档落共享 compacted.jsonl（会话隔离失效）。
    // 必须在 run() 前绑定（首次 build_messages 即可能触发压缩）。
    agent.set_session_id(session_id.to_string());
    // S3：turn 级 checkpoint（kill/Ctrl-C/panic 只丢最后一个交换）
    attach_session_safety(&mut agent, session_id, &workspace);

    // R4: 用户气泡（视觉区分"谁在说话"）
    render::user_prompt(goal_text);

    // B2: 追加用户消息 → run() 注入历史（连续对话引用前文）
    agent.enqueue_user_message(goal_text.to_string());
    // WS9 (v0.2): CLI 直跑 = 交互模式（预算 ask 需要真人应答）
    agent.set_interactive(true);
    // R2-F (v0.2.6): egress 审批落盘回调——approve 后追加 host 进 config.toml
    // （复用 config set 既有写入路径 set_field+save——批注 5 坑②；并发/转义由
    // 既有实现负责）。文件追加后下个进程生效；当前进程 ctx.env 已由 loop 层更新。
    {
        let persist_host: agent_core::EgressPersistFn = Arc::new(move |host, _joined| {
            match crate::config::Config::load() {
                Ok(mut cfg) => {
                    // env 优先是安全例外（P2-3），但运行时批准的 host 必须落文件
                    // 否则下个进程丢失；env 里已有的 host 重复写入无害。
                    let resolved = cfg.resolve(None, None, None, None, None);
                    let mut list = resolved.egress_allowlist;
                    if !list.iter().any(|h| h == host) {
                        list.push(host.to_string());
                    }
                    let joined = list.join(",");
                    match cfg.set_field("egress-allowlist", &joined) {
                        Ok(()) => render::info(&format!(
                            "  🔓 已放行 {host} 并写入配置（egress-allowlist）"
                        )),
                        Err(e) => {
                            tracing::warn!(error = %e, "egress host 持久化失败（内存白名单仍生效）")
                        }
                    }
                }
                Err(e) => tracing::warn!(error = %e, "config 读取失败——egress host 未持久化"),
            }
        });
        agent.set_egress_persist_callback(persist_host);
    }

    // 事件转发：unbounded channel（与 service 相同机制）
    let (internal_tx, mut internal_rx) =
        tokio::sync::mpsc::unbounded_channel::<agent_core::Event>();
    agent.set_event_sender(internal_tx);

    let mut env = agent_runtime::EnvelopeState::new();
    let mut depth: usize = 0;
    // R6 (D5): AI 侧事件流收集（Observer 素材——落盘 ~/hearth/observer/<sid>.jsonl）
    let mut enveloped: Vec<api::EnvelopedEvent> = Vec::new();

    // WS9 (v0.2): 默认预算（REPL_BUDGET）按目标方向打档——premium/standard/economy
    // 给不同步数基线（上策100/中策50/下策20）；用户显式 --budget N 时尊重显式值。
    let mut run_budget = Budget {
        max_steps: budget,
        ..Budget::default()
    };
    if budget == crate::repl::REPL_BUDGET {
        let (tier, min_steps) = goal_tier(goal_text);
        run_budget.tier = tier;
        run_budget.max_steps = budget.max(min_steps);
        run_budget.allocated_units = Some(min_steps);
    }
    // H2 (v0.2.4): 任务级 wall-clock deadline——CLI 每轮默认 15 分钟。
    // env HEARTH_TASK_TIMEOUT_SECS 可覆盖；0 = 显式关闭（不建议）。
    // 步数预算管"轮次"，deadline 管"时间"——两层独立（P1-4 分层重设计的顶层半边）。
    run_budget.max_time_secs = std::env::var("HEARTH_TASK_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .or(Some(900));
    // R2-C 采集器 v2: session_id 侧通道（TelemetryProvider 从 env 读——
    // gateway 层不可知会话，jsonl 每行仍可归属会话）
    std::env::set_var("HEARTH_TELEMETRY_SID", session_id);
    let run_goal = Goal::with_budget(goal_text.to_string(), run_budget);
    let dispatcher = agent.scheduler_arc(); // 供渲染审批用
                                            // R2-D (批示 3, v0.2.7): 任务初始化——生命周期条件（original absent 即写），
                                            // **在首次副作用（工具执行）之前持久化** taskgoal（spawn 前）。
                                            // provenance（批示 4）：第一版 constraints/criteria 来源=恢复值或空
                                            // （Hearth.md 已由 system 注入覆盖项目级约束；InteractionRequest 通道后续）。
    {
        // Node 03 (O-4): criteria 生产入口接线——用户 --acceptance 声明的验收
        // 标准（结构化前缀）经 init_taskgoal 进 TaskGoal（R2-B 承载结构复用）。
        let init = agent.init_taskgoal(Vec::new(), acceptance.clone());
        // Node 03 (O-4): pending 桥——run() 内 ContextManager::new 重建会清
        // criteria，init 后/run 前挂到 agent 上由 run() 恢复。
        agent.set_pending_acceptance(acceptance);
        let existing = crate::session_store::load_taskgoal(session_id);
        let next_rev = existing.as_ref().map(|(r, _)| r + 1).unwrap_or(1);
        if init || existing.is_none() {
            match crate::session_store::save_taskgoal(session_id, &agent.taskgoal_value(), next_rev)
            {
                Ok(()) if init => render::info("  🎯 任务目标已锚定（original_goal 持久化）"),
                Err(e) => tracing::warn!(error = %e, "taskgoal 初始持久化失败（不阻断）"),
                _ => {}
            }
        }
    }
    // S11（手术包二）：中断句柄——必须在 agent move 进 task **之前**取（Ctrl-C
    // 时置位 + 唤醒 in-flight 模型调用，agent 优雅收尾后交还，上下文保留）。
    let interrupt_flag = agent.interrupt_handle();
    let interrupt_notify = agent.interrupt_notify();
    // B2: run_take——run 结束后把 agent 拿回（保留历史供下一轮）
    let mut agent_handle = tokio::spawn(async move { agent.run_take(run_goal).await });

    // REPL 体验 (v0.2.1): 运行中 Ctrl-C 取消本轮——select 事件流 / agent 完成 / Ctrl-C。
    // S11（手术包二）：Ctrl-C 由"abort 整个 task（本轮进度只能退回到上一轮快照）"
    // 升级为**优雅打断**——置位 + notify → agent 在步边界/in-flight 模型调用处
    // 立即停下，走 interrupted 收尾（paused 语义）并交还 agent 本体；本轮上下文
    // 完整保留，回到提示符后可追问"刚才做到哪"。仅当优雅收尾超时（5s）才兜底
    // abort（历史回落到上一轮快照）。
    // 输入阶段由 reedline 处理 Ctrl-C（转义序列），运行阶段走这里（SIGINT handler）。
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);

    // 事件循环只负责渲染/收集（X1-3 后最终 report 来自 run_take 返回值，事件内 Done 不再收集）
    type RunResult =
        Result<Result<(agent_core::RunReport, AgentLoop), anyhow::Error>, tokio::task::JoinError>;
    enum RunOutcome {
        Finished(Box<RunResult>),
        /// S11：Ctrl-C 优雅打断——等待 agent 收尾（agent 本体不丢）。
        Interrupted,
    }
    let outcome = loop {
        tokio::select! {
            evt = internal_rx.recv() => {
                match evt {
                    Some(evt) => {
                        let mut api_evt = agent_runtime::map_event(evt);
                        env.wrap(&mut api_evt);
                        enveloped.push(api::EnvelopedEvent {
                            schema_version: 1,
                            ts: chrono::Utc::now().to_rfc3339(),
                            seq: env.seq,
                            span_id: env
                                .span_stack
                                .last()
                                .map(|(id, _)| id.clone())
                                .unwrap_or_default(),
                            parent_id: env.span_stack.last().and_then(|(_, p)| p.clone()),
                            event: api_evt.clone(),
                        });
                        let is_done = matches!(api_evt, AgentEvent::Done { .. });
                        render_agent_event(session_id, &dispatcher, &api_evt, &mut depth).await?;
                        if is_done {
                            break RunOutcome::Finished(Box::new((&mut agent_handle).await)); // 拿到最终 RunReport
                        }
                    }
                    None => {
                        break RunOutcome::Finished(Box::new((&mut agent_handle).await)); // 事件通道关闭（agent 已结束）
                    }
                }
            }
            _ = &mut ctrl_c => {
                // S11（手术包二）：优雅打断——置位（步边界收手）+ notify（唤醒
                // in-flight 模型调用）。不在此处 abort：下方的收敛块等待 agent
                // 收尾（≤5s）后走通用路径（快照落盘 + agent 交还，上下文保留）。
                tracing::info!("run interrupted by Ctrl-C (S11 graceful interrupt)");
                interrupt_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                interrupt_notify.notify_waiters();
                break RunOutcome::Interrupted;
            }
        }
    };
    // S11: 打断收敛——等待 agent 优雅收尾（interrupted 收尾已 emit Done，事件
    // 循环可能已提前 break，这里以 JoinHandle 为准）。超时兜底 abort（保留
    // "永不挂死"旧语义——agent 卡在不可打断调用里时仍有出路）。
    let outcome_result: Option<RunResult> = match outcome {
        RunOutcome::Finished(r) => Some(*r),
        RunOutcome::Interrupted => {
            const INTERRUPT_GRACE_SECS: u64 = 5;
            match tokio::time::timeout(
                std::time::Duration::from_secs(INTERRUPT_GRACE_SECS),
                &mut agent_handle,
            )
            .await
            {
                Ok(res) => Some(res),
                Err(_) => {
                    tracing::warn!(
                        grace_s = INTERRUPT_GRACE_SECS,
                        "S11: graceful interrupt timed out — aborting agent task (fallback)"
                    );
                    agent_handle.abort();
                    None
                }
            }
        }
    };
    // 拿回 agent（保留历史）
    // X1-3 (v0.1.6): 崩溃隔离——agent 任务 panic 不杀进程（真机 `agent task join failed`
    // 直接崩掉整个 REPL，前面 20 轮对话全丢）。panic 转成本轮失败报告 + None agent：
    // 进程活着、transcript 已落盘（X1-1），可 resume 续接。
    let (agent_back, report) = match outcome_result {
        // S11: 优雅打断超时兜底（agent 卡死）→ 取消报告（历史回落到上一轮快照）。
        None => {
            let cancel_report = serde_json::json!({
                "ok": false,
                "status": "cancelled",
                "error": "本轮已取消（Ctrl-C）——历史保留，可输入新目标继续",
                "goal": goal_text,
                "steps": 0,
            });
            (None, cancel_report)
        }
        Some(agent_future) => match agent_future {
            Ok(Ok((r, agent))) => {
                // X1-1 (v0.1.6): 会话持久化——每轮结束把完整历史 Turn 快照落盘
                // （~/.config/hearth/sessions/<sid>.jsonl，原子写；失败不阻断主路径）。
                if let Err(e) =
                    crate::session_store::save_snapshot(session_id, &agent.history_turns())
                {
                    tracing::warn!(error = %e, "session 落盘失败（不阻断）");
                }
                // 任务状态持久化 (v0.2): task_graph 一并落盘（resume 恢复，防 replan 回退）
                // R2-D (批示 2): taskgoal 与 graph 用**同一 state_revision**——
                // resume 时 revision 不一致 → recovery path（补充 3：stale 标注+warning，
                // 不阻断不静默）。
                {
                    let next_rev = crate::session_store::load_taskgoal(session_id)
                        .map(|(r, _)| r + 1)
                        .unwrap_or(1);
                    if let Err(e) = crate::session_store::save_taskgoal(
                        session_id,
                        &agent.taskgoal_value(),
                        next_rev,
                    ) {
                        tracing::warn!(error = %e, "taskgoal 落盘失败（不阻断）");
                    }
                }
                // RunReport 无 Serialize——手动转 Value（与 run() 的 Done 事件同构）
                // G1 (v0.2.5): 终态规范化——loop 层 reason（deadline_exceeded/
                // budget_exhausted/verify_failed/...）不再被压成二值 completed/failed，
                // 统一走 terminal::normalize（九态封闭集），CLI/报告/Observer 同源。
                let reason = r
                    .summary
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let terminal = agent_core::normalize_terminal_state(r.ok, reason);
                let report_value = serde_json::json!({
                    "ok": r.ok,
                    "steps": r.steps,
                    "summary": r.summary,
                    "status": terminal,
                    "status_detail": if reason.is_empty() { serde_json::Value::Null } else { serde_json::json!(reason) },
                    // 成本护栏 (v0.2): token 用量透出——CLI 渲染成本可见
                    "usage": r.usage,
                    // C-1/C-12：终态验证标注（VERIFIED | UNVERIFIED）——
                    // UNVERIFIED 的 completed 投影必须带"（未验证）"。
                    "verification": agent.verification_state(),
                    // R1-1（对话可用性根治任务书 v1.0）：give_up 改判标记——
                    // 改判的 completed 恒 UNVERIFIED 且不计入成功率
                    // （A-4(b) 验收门）；投影层据此呈现"改判·未验证"。
                    "rerouted_unverified": agent.rerouted_unverified(),
                    // R3-1 收尾三行数据源：账本未完成栏（"还剩什么"——事实
                    // 生成非模型自报，R2-1 账本）。
                    "ledger_pending": agent.ledger_pending_texts(),
                });
                (Some(agent), report_value)
            }
            Ok(Err(e)) => {
                tracing::error!(error = %e, "agent run failed");
                let fail = serde_json::json!({
                    "ok": false,
                    // G1: provider/网络等 run 级错误 → failed（G1-02 异常必映射终态）
                    "status": agent_core::normalize_terminal_state(false, "error"),
                    "status_detail": "agent_error",
                    "error": format!("{e:#}"),
                    "goal": goal_text,
                    "steps": 0,
                });
                (None, fail)
            }
            Err(e) => {
                tracing::error!(error = %e, "agent task panicked — session survived");
                render::error(
                    "agent 内部 panic（本轮失败）——进程已隔离，可继续或重试；历史已落盘可 resume",
                );
                let fail = serde_json::json!({
                    "ok": false,
                    // G1: panic → aborted（异常中止，区别于任务失败——G1-02 必映射）
                    "status": agent_core::normalize_terminal_state(false, "agent_crashed"),
                    "status_detail": "agent_crashed",
                    "error": format!("agent 内部 panic: {e}"),
                    "goal": goal_text,
                    "steps": 0,
                });
                (None, fail)
            }
        },
    };

    // R6: AI 侧落盘（Observer 消费——零执行权，只记）
    let _ = crate::note::persist_ai_events(session_id, &enveloped);
    // v0.1.2 (hearth-harness-review-supplement 补充5): 最小 transcript——一行 JSONL，
    // 让验证层 fail 有据可查。失败不阻断（warn）。
    let status = report.get("status").and_then(|s| s.as_str()).unwrap_or(
        if report["ok"].as_bool().unwrap_or(false) {
            "completed"
        } else {
            "failed"
        },
    );
    if let Err(e) = crate::transcript::record_run(
        &workspace,
        session_id,
        goal_text,
        budget,
        run_started,
        report["steps"].as_u64().unwrap_or(0),
        report["ok"].as_bool().unwrap_or(false),
        status,
        &crate::transcript::collect_tool_calls(&enveloped),
        &crate::transcript::collect_written_files(&enveloped),
    ) {
        tracing::warn!(error = %e, "transcript 落盘失败（不阻断）");
    }
    // P1-9 (v0.2.4): 结构化执行报告——不翻原始 log 即可复盘本轮
    // （.hearth/reports/<sid>/run-NNN.md；失败不阻断）。
    {
        let (approvals, reflections) = crate::report::collect_approvals_and_reflections(&enveloped);
        let usage_line = report
            .get("usage")
            .and_then(|u| u.as_object())
            .map(|u| {
                let p = u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                let c = u
                    .get("completion_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let calls = u.get("calls").and_then(|v| v.as_u64()).unwrap_or(0);
                format!("tokens: ↑{p} ↓{c} (calls={calls})")
            })
            .unwrap_or_else(|| "tokens: n/a".into());
        let input = crate::report::RunReportInput {
            session_id,
            goal: goal_text,
            status,
            ok: report["ok"].as_bool().unwrap_or(false),
            steps: report["steps"].as_u64().unwrap_or(0),
            wall_secs: run_started.elapsed().as_secs(),
            usage_line,
            tool_calls: crate::report::collect_tool_call_details(&enveloped),
            written_files: crate::transcript::collect_written_files(&enveloped),
            approvals,
            reflections,
            errors: crate::report::collect_errors(&enveloped),
            // R2-D (批示 5): verification scope——criteria 空（本轮无写入通道）恒
            // "none"；agent_back 在场时向 agent 查询真实状态。
            acceptance_verification: agent_back
                .as_ref()
                .map(|a| a.acceptance_verification_status())
                .unwrap_or("none"),
            // Node 05 (N-3): 审批委托投影素材（summary.approval_delegated[_cmds]）
            approval_delegated: {
                let delegated = report
                    .get("summary")
                    .and_then(|s| s.get("approval_delegated"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let cmds = report
                    .get("summary")
                    .and_then(|s| s.get("approval_delegated_cmds"))
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|c| c.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                (delegated, cmds)
            },
            // G1-03: 剩余工作——R7-5/D-4（线C手术）：图节点提取已删（图本体
            // 消失），统一给通用指引（报告里"还剩什么"必可回答）。
            remaining_work: if report["ok"].as_bool().unwrap_or(false) {
                Vec::new()
            } else {
                vec!["本轮未完成——直接输入新指令继续（历史已保留）".into()]
            },
            // S12（手术包二）：产物自检结果入报告（Done 事件顶层字段；未跑=null）
            selfcheck: report.get("selfcheck").cloned().filter(|v| !v.is_null()),
            // S14（手术包二）：任务总结（TL;DR）入报告头部（agent 侧 run 单一
            // exit 生成——含机械降级块；永不编造，非 empty 才写）。
            summary: report
                .get("summary")
                .and_then(|s| s.get("run_summary"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .map(String::from),
        };
        match crate::report::write_run_report(&workspace, &input) {
            Ok(p) => render::info(&format!("  📄 执行报告: {}", p.display())),
            Err(e) => tracing::warn!(error = %e, "report 落盘失败（不阻断）"),
        }
    }
    // G1-03 (v0.2.5): 终态投影——用户必须明确看到完成/失败/取消/超时，
    // 禁止"输出结束=沉默"（任务书 §0.2 B 最高优先级体感问题）。
    // 终态来自 terminal::normalize 统一映射（报告/transcript 同源）。
    {
        let detail = report
            .get("status_detail")
            .and_then(|v| v.as_str())
            .or_else(|| report.get("error").and_then(|v| v.as_str()))
            .unwrap_or("");
        let steps = report["steps"].as_u64().unwrap_or(0);
        match status {
            "completed" => {
                // C-1/C-12：UNVERIFIED 的 completed 禁止裸"目标达成"（三层摸底
                // C-12 裁定：37 步零验证的 run-001 必须可见地标注）。
                let verification = report
                    .get("verification")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNVERIFIED");
                // R1-1（对话可用性根治任务书 v1.0）：give_up 改判场景——
                // A-4(b) 验收门要求改判**不得呈现为无保留完成**，且统计上
                // 不计入成功率（G-C 判据：改判须呈现未完成）。
                let rerouted = report
                    .get("rerouted_unverified")
                    .map(|v| !v.is_null())
                    .unwrap_or(false);
                // ── R1-4 known-failing 报告层拦截（G-B 一票否决门的投影面）──
                // 0.9-0.3 会话"已知 0/16 失败项却标 ✅100%"的投影侧终结：
                // completed 但存在未清已知失败 → **禁止裸 ✓**。
                let known_failing: Vec<String> = report
                    .get("summary")
                    .and_then(|s| s.get("known_failing_open"))
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                if !known_failing.is_empty() {
                    render::error(&format!(
                        "  ⚠ Task claims completed（{steps} 步）——⚠️ G-B 拦截：{} 项已知失败未复测通过，本完成声明不可信、禁止计入验收通过：{:?}",
                        known_failing.len(),
                        known_failing
                    ));
                } else if verification == "VERIFIED" && !rerouted {
                    render::info(&format!(
                        "  ✓ Task completed（{steps} 步）——目标达成，产物见报告"
                    ));
                } else if rerouted {
                    render::info(&format!(
                        "  ✓ Task completed（{steps} 步）——⚠️ 改判达成（未验证·不计入成功率）：本 run 曾放弃，因存在产物而改判；产物合格性未经核验，请人工复核报告"
                    ));
                } else {
                    render::info(&format!(
                        "  ✓ Task completed（{steps} 步）——目标达成（未验证），产物见报告"
                    ));
                }
            }
            "deadline_exceeded" => {
                let cap = report
                    .get("summary")
                    .and_then(|s| s.get("cap_secs"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                render::error(&format!(
                    "  ✗ Task failed — deadline exceeded（{cap}s 上限，用时 {}s）——可提高 HEARTH_TASK_TIMEOUT_SECS 后 resume 继续",
                    run_started.elapsed().as_secs()
                ));
            }
            "cancelled" => {
                // T-B (W8/N-2): one-shot 无 submit()——旧注释前提不成立，headless
                // Ctrl-C 后用户看不到任何 cancelled 投影（真机 sigint 实测）。
                // 统一投影点：one-shot 与 REPL 都从此处渲染（repl.rs submit 的
                // 重复行已同步移除——避免双渲染）。
                render::info("  ⏹ 本轮已取消（Ctrl-C）——历史保留，可 resume 或输入新目标继续");
            }
            "paused" => {
                // R6-6（判定权归还长程任务书 v1.0）：预算耗尽 = 交还控制权 +
                // 当前状态 + 建议，**非 Task failed**（护栏触发 ≠ 判定失败）。
                // .133 真机 A9 实证缺口：normalize 已映射 paused，但本投影层
                // 曾落入 other 臂渲染 "✗ Task failed — budget_exhausted"。
                // S8（手术包二）：统一暂停语义——provider 故障/窗口耗尽同走本臂，
                // 一律显示"断点已存 + resume 指引"（不再有不可恢复的 failed）。
                let reason = report
                    .get("summary")
                    .and_then(|s| s.get("reason"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let label = if reason == "interrupted" {
                    // S11（手术包二）：Ctrl-C 打断——上下文保留，非失败非取消。
                    "  ⏹ 本轮已打断（Ctrl-C）——上下文保留，可直接继续"
                } else if reason.starts_with("provider") {
                    "  ⏸ 暂停（provider 故障）——上下文与断点均已保留"
                } else {
                    "  ⏸ 预算护栏触发——任务未完成但非失败，控制权交还"
                };
                render::info(&format!("{label}（{steps} 步）"));
                if let Some(handover) = report.get("summary").and_then(|s| s.get("handover")) {
                    if let Some(state) = handover.get("state").and_then(|v| v.as_str()) {
                        render::info(&format!("  ── 交还 | 做到哪了: {state}"));
                    }
                    if let Some(pending) = handover.get("pending").and_then(|v| v.as_array()) {
                        let items: Vec<String> = pending
                            .iter()
                            .filter_map(|x| x.as_str().map(String::from))
                            .collect();
                        if !items.is_empty() {
                            render::info(&format!(
                                "  ── 交还 | 还剩什么: {}（{} 项）",
                                items.first().map(String::as_str).unwrap_or(""),
                                items.len()
                            ));
                        }
                    }
                    if let Some(sugg) = handover.get("suggestion").and_then(|v| v.as_str()) {
                        render::info(&format!("  ── 交还 | 建议: {sugg}"));
                    }
                }
                // S8：断点/resume 指引（provider 路径在 summary.resume_hint，
                // 预算路径在 summary.handover.resume_hint——两处都投影）。
                let resume_hint = report
                    .get("summary")
                    .and_then(|s| s.get("resume_hint"))
                    .and_then(|v| v.as_str())
                    .or_else(|| {
                        report
                            .get("summary")
                            .and_then(|s| s.get("handover"))
                            .and_then(|h| h.get("resume_hint"))
                            .and_then(|v| v.as_str())
                    });
                if let Some(hint) = resume_hint {
                    render::info(&format!("  ── 断点 | {hint}"));
                }
            }
            "aborted" => {
                // panic 崩溃隔离——上方 render::error 已渲染，此处不重复
            }
            other => {
                let why = if detail.is_empty() {
                    format!("status={other}")
                } else {
                    detail.to_string()
                };
                render::error(&format!(
                    "  ✗ Task failed — {why}（{steps} 步）——报告含已完成/剩余工作，可直接下指令继续"
                ));
                // RC24-B: 结构化拒绝附可行动提示（summary.hint 由 loop 层给出）
                if let Some(hint) = report
                    .get("summary")
                    .and_then(|s| s.get("hint"))
                    .and_then(|h| h.as_str())
                    .filter(|_| detail.contains("approval_denied"))
                {
                    render::info(&format!("  ↳ {hint}"));
                }
            }
        }
        // RC51-B (P4 Node 05): 终态 ≠ 一行状态——产物清单直接投影
        // （BUG-012/盲测 run-010"我需要猜你的结果"实证；非内部日志倾倒）。
        let written = crate::transcript::collect_written_files(&enveloped);
        if !written.is_empty() {
            render::info(&format!("  📦 产物 {} 个:", written.len()));
            for p in written.iter().take(5) {
                render::info(&format!("     - {p}"));
            }
            if written.len() > 5 {
                render::info(&format!("     - … 其余 {} 个见执行报告", written.len() - 5));
            }
        }
        // ── R3-1 收尾三行（W-E 进度可问的终态面；任务书 R3-1）──
        // 每轮终态必附：改了什么 / 还剩什么 / 依据——**全部由 Observe/账本
        // 采集的事实生成，非模型自报**（0.9-0.3 病理"472 横幅噪声淹没 +
        // 疑问句答文件路径"的投影侧终结）。failed/cancelled 也渲染（"还剩
        // 什么"对失败轮更重要——用户据此下继续指令）。
        {
            let artifacts: Vec<String> = report
                .get("summary")
                .and_then(|s| s.get("artifacts"))
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let pending: Vec<String> = report
                .get("ledger_pending")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let decision = report
                .get("summary")
                .and_then(|s| s.get("completion_decision"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let verification = report
                .get("verification")
                .and_then(|v| v.as_str())
                .unwrap_or("UNVERIFIED");
            render::info(&format!(
                "  ── 收尾 | 改了什么: {}",
                if artifacts.is_empty() {
                    "本轮无产物落盘".to_string()
                } else {
                    format!("{}（{} 个）", artifacts.join("、"), artifacts.len())
                }
            ));
            render::info(&format!(
                "  ── 收尾 | 还剩什么: {}",
                if pending.is_empty() {
                    "账本无未完成项".to_string()
                } else {
                    format!("{}（{} 项未完成）", pending.join("；"), pending.len())
                }
            ));
            render::info(&format!(
                "  ── 收尾 | 依据: 完成决策 = {}；验证状态 = {verification}",
                if decision.is_empty() {
                    "（无记录）"
                } else {
                    decision
                }
            ));
            // ── R3-2 goal_drift 升级投影（G-F：知情必拦截的明确提示形态）──
            // 旧：observe-only 只发 ThinkSummary（淹没在噪声里）。升级：
            // 终态处**明确警示行**（不改终态——LLM 判定可能误报，改 failed
            // 会冤枉；"改终态或明确提示"取后者）。真正改终态留待 R4 以
            // 双会话重放数据裁定误报率后再议。
            let drift = report
                .get("summary")
                .and_then(|s| s.get("goal_drift"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if drift {
                render::error(
                    "  ⚠ G-F goal_drift 疑似：终局产物与原始目标语义相关度存疑（独立判定认为任务被替换/漂移）——请人工核对产物是否确实是你要的",
                );
            }
        }
        // ── S14（手术包二）：任务总结（TL;DR）——收尾最后一块，也是"事后不用
        // 看过程"的答案（用户原话"内容太多，我未必会看过程"）。数据源 =
        // report.summary.run_summary（agent 侧 run 单一 exit 生成：一次
        // ≤800 token 调用；模型不可用/超时/缺段 → 机械降级；打断轮不发起
        // 额外调用）——CLI 只投影，不二次生成、不加工内容。
        if let Some(block) = report
            .get("summary")
            .and_then(|s| s.get("run_summary"))
            .and_then(|v| v.as_str())
            .filter(|b| !b.trim().is_empty())
        {
            render::summary_block(block);
            if report
                .get("summary")
                .and_then(|s| s.get("run_summary_generated"))
                .and_then(|v| v.as_bool())
                == Some(false)
            {
                render::info(
                    "  ↳ 本总结为**机械降级**（模型不可用/超时/被打断）——产物·自检为实数据，叙述段标「生成失败」；过程事实见执行报告",
                );
            }
        }
    }
    Ok((agent_back, report))
}

/// 事件渲染 + 内联审批（与 B4-1 的 HTTP 渲染同构，进程内直喂）。
async fn render_agent_event(
    sid: &str,
    dispatcher: &Arc<ToolDispatcher>,
    evt: &AgentEvent,
    depth: &mut usize,
) -> Result<()> {
    match evt {
        AgentEvent::Phase { phase } => render::phase(phase),
        AgentEvent::Token { delta } => render::token(delta),
        AgentEvent::ToolCall { name, args, .. } => render::tool_call(name, args),
        AgentEvent::ToolResult {
            is_error, output, ..
        } => {
            if *is_error {
                render::error(&format!("工具出错: {output}"));
            } else {
                render::tool_result(output);
            }
        }
        AgentEvent::Reflection { verdict } => render::reflection(verdict),
        AgentEvent::Done { report } => {
            let steps = report.get("steps").and_then(|s| s.as_u64()).unwrap_or(0);
            let ok = report.get("ok").and_then(|o| o.as_bool()).unwrap_or(false);
            render::done(steps, ok);
            // ── R3-1 收尾三行（REPL 本地模式返工：与 one-shot 同源事实，
            // 数据取自 Event::Done payload 同源字段）──
            if ok {
                let artifacts: Vec<String> = report
                    .get("artifacts")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let pending: Vec<String> = report
                    .get("ledger_pending")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let decision = report
                    .get("completion_decision")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let verification = report
                    .get("verification")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNVERIFIED");
                let known_failing: Vec<String> = report
                    .get("known_failing_open")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                if !known_failing.is_empty() {
                    render::error(&format!(
                        "  ⚠ G-B 拦截：{} 项已知失败未复测通过，本完成声明不可信、禁止计入验收通过：{:?}",
                        known_failing.len(),
                        known_failing
                    ));
                }
                render::info(&format!(
                    "  ── 收尾 | 改了什么: {}",
                    if artifacts.is_empty() {
                        "本轮无产物落盘".to_string()
                    } else {
                        format!("{}（{} 个）", artifacts.join("、"), artifacts.len())
                    }
                ));
                render::info(&format!(
                    "  ── 收尾 | 还剩什么: {}",
                    if pending.is_empty() {
                        "账本无未完成项".to_string()
                    } else {
                        format!("{}（{} 项未完成）", pending.join("；"), pending.len())
                    }
                ));
                render::info(&format!(
                    "  ── 收尾 | 依据: 完成决策 = {}；验证状态 = {verification}",
                    if decision.is_empty() {
                        "（无记录）"
                    } else {
                        decision
                    }
                ));
            }
            // 成本护栏 (v0.2): token 用量可见——跑完知道烧了多少（用户"缺钱"关切）
            if let Some(u) = report.get("usage").and_then(|u| u.as_object()) {
                let prompt = u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                let comp = u
                    .get("completion_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let calls = u.get("calls").and_then(|v| v.as_u64()).unwrap_or(0);
                if prompt + comp > 0 {
                    render::info(&format!("  📊 tokens: ↑{prompt} ↓{comp} (calls={calls})"));
                }
            }
            // R7 (v0.1.4): 失败不伪装 Done——失败时若带 error，给出可行动提示
            // （provider 故障请重试/切通道；其余失败提示看日志）。
            if !ok {
                if let Some(err) = report.get("error").and_then(|e| e.as_str()) {
                    let is_provider = err.contains("provider")
                        || err.contains("read body")
                        || err.contains("transient")
                        || err.contains("deadline");
                    // PC-3 修复（P0/P1 修复任务书 v1.0）：provider 层已统一口径——
                    // 连接拒绝/超时/502/5xx 全部 Transient（长退避重试 → 窗口耗尽
                    // 则暂停可 resume）；"端点死→换通道"的旧提示不再适用（那条
                    // 分类已删除）。仅"连续 read-body 畸形流"仍 Fatal（不重试）。
                    let malformed_stream = err.contains("read body failed twice");
                    if is_provider && !malformed_stream {
                        render::info(
                            "provider 通道瞬时故障（连接拒绝/超时/502/5xx）——已长退避重试；窗口耗尽会暂停（可 resume），持续失败请检查网络/API key/限流",
                        );
                    } else if is_provider && malformed_stream {
                        render::info(
                            "provider 响应流连续畸形（read body 两次失败）——重试同一通道无意义；请稍后重试或 hearth config set provider <其他通道>",
                        );
                    } else {
                        render::info("任务失败——详见上方错误信息，调整后重试");
                    }
                }
            }
        }
        AgentEvent::Error { message } => render::error(message),
        AgentEvent::SpanOpen { name, t0, .. } => {
            *depth += 1;
            render::span_open(name, *depth, t0);
        }
        AgentEvent::SpanClose { duration_ms, .. } => {
            render::span_close(*depth, *duration_ms);
            *depth = depth.saturating_sub(1);
        }
        AgentEvent::Artifact {
            path,
            kind,
            delta_lines,
            ..
        } => {
            render::artifact(path, kind, *delta_lines);
        }
        AgentEvent::ThinkSummary { phase, text } => render::think_summary(phase, text),
        // R2-D (批示 1): 目标修订可见——用户换目标时明确提示（original 不变）
        AgentEvent::GoalChanged { revision, new_goal } => {
            render::info(&format!(
                "  🔁 目标已修订（revision {}）：{}——原目标保留为锚点",
                revision,
                new_goal.chars().take(60).collect::<String>()
            ));
        }
        AgentEvent::PlanDraft {
            steps,
            gaps_found,
            gaps_to_ask,
            auto_assumed,
            gaps_to_ask_details,
            ..
        } => {
            render::plan_draft(
                steps,
                *gaps_found,
                *gaps_to_ask,
                auto_assumed,
                gaps_to_ask_details,
            );
        }
        AgentEvent::NeedApproval {
            approval_id,
            action,
            payload,
        } => {
            use std::io::Write as _;
            // B3-A: clarification（澄清）——打印问题（payload 的 from/why），
            // 读用户文本回答作为 payload 回喂（不是简单 y/n）。
            // R10-C4 (v0.1.6): 选项式——style=single_select 且 options 非空时列出
            // 编号选项；输入数字=选中、y/n=确认、其他文本=自由答案（不丢）。
            // WS9 (v0.2): budget_reassess 同选项式（继续/重新评估）。
            if action == "clarification"
                || action == "budget_reassess"
                // R2-F (v0.2.6): egress 审批走选项式（显示 host/why，approve/deny）——
                // 与 loop 层 handle_egress_denials 的应答解析（answer=approve/yes）对齐。
                || action == "egress_allowlist_request"
            {
                let from = payload.get("from").and_then(|f| f.as_str()).unwrap_or("");
                let why = payload.get("why").and_then(|w| w.as_str()).unwrap_or("");
                let style = payload
                    .get("style")
                    .and_then(|s| s.as_str())
                    .unwrap_or("free_text");
                let options: Vec<String> = payload
                    .get("options")
                    .and_then(|o| o.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                print!(
                    "{}",
                    format!("\n❓ 需要你确认 [{from}] {why}\n").bright_yellow()
                );
                // 选项式：先打印编号选项再收输入
                if style == "single_select" && !options.is_empty() {
                    for (i, o) in options.iter().enumerate() {
                        print!("{}", format!("  {}) {}\n", i + 1, o).bright_cyan());
                    }
                }
                print!("{}", "  你的选择: ".to_string().bright_yellow());
                let _ = std::io::stdout().flush();
                // O-1 (P1-small): headless（stdin 非 tty）——无人应答，不阻塞等待
                // stdin（对照实证：V2R2 budget ask 卡 200s 至外部超时）。立即放弃
                // 应答 → loop 侧按"重新评估/超时"走终止路径。照抄 RC24-B approval
                // 分支 is_terminal 先例；Budget 数值语义与 ApprovalPolicy 零改动。
                use std::io::IsTerminal as _;
                if !std::io::stdin().is_terminal() {
                    render::info("  ⚠ 非交互模式（stdin 非 tty）——无人应答，按放弃处理（budget_reassess/clarification headless guard）");
                    let _ = dispatcher
                        .resolve_interaction(sid, approval_id, false, serde_json::Value::Null)
                        .await;
                    return Ok(());
                }
                let mut answer = String::new();
                let answered = std::io::stdin()
                    .read_line(&mut answer)
                    .map(|_| !answer.trim().is_empty())
                    .unwrap_or(false);
                // 解析：数字 → 选中对应选项；y/n → 确认/取消；其他文本 → 自由答案
                let mut resolved_answer: serde_json::Value = serde_json::Value::Null;
                if answered {
                    let trimmed = answer.trim();
                    if let Ok(n) = trimmed.parse::<usize>() {
                        if n >= 1 && n <= options.len() {
                            resolved_answer = serde_json::json!({ "answer": options[n - 1].clone(), "choice": n });
                        } else {
                            resolved_answer = serde_json::json!({ "answer": trimmed });
                        }
                    } else if matches!(trimmed.to_lowercase().as_str(), "y" | "yes") {
                        resolved_answer = serde_json::json!({ "answer": "yes" });
                    } else if matches!(trimmed.to_lowercase().as_str(), "n" | "no") {
                        resolved_answer = serde_json::json!({ "answer": "no" });
                    } else {
                        resolved_answer = serde_json::json!({ "answer": trimmed });
                    }
                }
                // 显示用副本（resolve_interaction 会 move 原值——E0382）
                let display_answer = resolved_answer
                    .get("answer")
                    .and_then(|a| a.as_str())
                    .unwrap_or("已选")
                    .to_string();
                let resolved = dispatcher
                    .resolve_interaction(sid, approval_id, answered, resolved_answer)
                    .await
                    .is_ok();
                render::info(&format!(
                    "  ✓ 澄清已提交（{}）",
                    if answered {
                        display_answer
                    } else {
                        "放弃".to_string()
                    }
                ));
                let _ = resolved;
                return Ok(());
            }
            // 审批（approval）——y/N 内联裁决
            // RC24-B: 非交互（stdin 非 tty）——不等待、不静默、不假装成功：
            // 立即结构化拒绝 + 可行动提示（headless 下旧路径 EOF→静默 deny 后
            // run 继续空转 23s 才 failed——现在秒级、带 reason、带下一步）。
            use std::io::IsTerminal as _;
            if !std::io::stdin().is_terminal() {
                render::error(&format!(
                    "⛔ {action} — 非交互模式审批被拒（approval_denied_noninteractive）"
                ));
                render::info(
                    "  ↳ 下一步: 用 hearth repl（可交互批准）或 --approve-within session（显式委托）后重跑",
                );
                let _ = dispatcher
                    .resolve_interaction(sid, approval_id, false, serde_json::Value::Null)
                    .await;
                return Ok(());
            }
            print!("{}", format!("⛔ {action} — 批准? [y/N] ").bright_red());
            let _ = std::io::stdout().flush();
            let mut buf = String::new();
            let approved = std::io::stdin()
                .read_line(&mut buf)
                .map(|_| {
                    let v = buf.trim().to_lowercase();
                    v == "y" || v == "yes"
                })
                .unwrap_or(false);
            // 进程内直喂（HTTP 版走 /interaction——语义等价）
            dispatcher
                .resolve_interaction(sid, approval_id, approved, serde_json::Value::Null)
                .await
                .context("审批提交失败")?;
            render::info(&format!(
                "  ✓ {} 已提交",
                if approved { "批准" } else { "拒绝" }
            ));
        }
        AgentEvent::InteractionResolved { by, resolved, .. } => {
            render::info(&format!("↪ 交互已响应 by={by} resolved={resolved}"));
        }
    }
    Ok(())
}

/// 供测试/诊断：provider 注册表探测（验证 provider 组装不炸）。
#[allow(dead_code)]
pub fn registry_smoke(cfg: &ResolvedConfig) -> Result<()> {
    let _p = build_provider(cfg)?;
    let _reg = ProviderRegistry::new();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 回归 WS9 (v0.2): 目标方向启发式打档——设计/游戏类 premium(100)，
    /// 超短问答 economy(20)，常规 standard(50)。无打档时此测试红。
    #[test]
    fn test_goal_tier_heuristic() {
        assert_eq!(goal_tier("帮我美化这个网页界面").0, "premium");
        assert_eq!(goal_tier("写一个贪吃蛇游戏").0, "premium");
        assert_eq!(goal_tier("1+1等于几").0, "economy");
        assert_eq!(goal_tier("重构模块 A 并补充测试").0, "standard");
        // 基线步数：premium≥100 / economy=20 / standard=50
        assert!(goal_tier("美化界面").1 >= 100);
        assert_eq!(goal_tier("hi").1, 20);
        assert_eq!(goal_tier("1+1").1, 20);
        assert_eq!(goal_tier("重构模块 A 并补充测试").1, 50);
    }

    /// S9（手术包二）：多通道配置 → FallbackChain 包装（非单通道）；
    /// 单通道/空链 → 行为不变（原 provider 直通）。
    #[test]
    fn test_s9_build_provider_multi_channel_yields_chain() {
        let base = || crate::config::ResolvedConfig {
            provider: "agnes".into(),
            model: None,
            url: None,
            api_key: Some("test-key".into()),
            mode: "auto".into(),
            feedback_prompt: true,
            egress_allowlist: vec![],
            url_warning: None,
            read_roots: None,
            providers: Vec::new(),
            provider_keys: std::collections::HashMap::new(),
        };
        // 多通道 → 链
        let mut multi = base();
        multi.providers = vec!["agnes".into(), "gemini".into()];
        let p = build_provider_inner(&multi).expect("多通道链构造必须成功");
        assert_eq!(p.name(), "fallback", "多通道必须包 FallbackChain");
        assert_eq!(p.model(), "agnes", "链 model 报告主通道（分账归属）");
        // 单通道 → 直通（行为不变）
        let single = base();
        let p2 = build_provider_inner(&single).expect("单通道构造必须成功");
        assert_eq!(p2.name(), "agnes", "单通道保持原 provider（零行为变化）");
        // 链内缺 key → 可行动错误
        let mut nokey = base();
        nokey.api_key = None;
        nokey.providers = vec!["agnes".into(), "gemini".into()];
        let err = match build_provider_inner(&nokey) {
            Ok(_) => panic!("链内缺 key 必须报错（不允许静默空 key）"),
            Err(e) => e.to_string(),
        };
        assert!(
            err.contains("provider_keys") || err.contains("api-key"),
            "缺 key 必须给可行动指引: {err}"
        );
    }

    /// PC-2 修复（P0/P1 修复任务书 v1.0）：resume 预算追加——总预算 = 断点已用
    /// + 追加额度（steps 接着数，追加部分是净余量）。饱和加法防溢出。
    #[test]
    fn test_pc2_resume_budget_adds_extra() {
        assert_eq!(super::resume_budget(7, 40), 47, "总预算 = 已用 + 追加");
        assert_eq!(super::resume_budget(0, 40), 40, "无已用 = 纯追加");
        assert_eq!(super::resume_budget(3, 0), 3, "零追加 = 保持已用水位");
        assert_eq!(
            super::resume_budget(u64::MAX, 5),
            u64::MAX,
            "饱和加法（防溢出回绕成小预算）"
        );
    }
}
