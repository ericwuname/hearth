//! hearth — Hearth CLI（正名，docs/hearth-naming.md）。
//!
//! 派 A 单二进制：进程内直跑内核（默认 auto 模式），也可 --url 连远程 service。
//! 配置三层优先级：参数 > env > ~/.config/hearth/config.toml > 内置默认。
#![allow(unused_imports, clippy::manual_strip)]

pub mod client;
pub mod config;
pub mod note;
pub mod render;
pub mod repl;
pub mod report;
pub mod run_local;
pub mod session_store;
pub mod snapshot_store;
pub mod transcript;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(
    name = "hearth",
    about = "Hearth CLI — 单二进制自托管 agent（进程内直跑内核）",
    // v0.1.1 用户真机（裸命令踩坑）：`hearth "目标"` 会报 unrecognized subcommand——
    // after_help 让 --help 就写明任务模式用法（clap 报错含 Usage 已够清晰）。
    after_help = "任务模式:  hearth chat「目标」（一次性直跑）\n交互模式:  hearth repl（多轮对话）\n快速配置:  hearth init  /  hearth config set api-key <key>",
    // R3 (v0.1.2): --version 输出 `hearth 0.1.2 (83fbf9d)`——版本+commit hash。
    // 完整版本串由 build.rs 注入 HEARTH_VERSION（非 git 环境为 "0.1.2 (unknown)"）。
    version = option_env!("HEARTH_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Service URL（提供则连远程 service；缺省 auto 直跑）。
    #[arg(long)]
    url: Option<String>,

    /// API key（优先级高于 env/config）。
    #[arg(long)]
    api_key: Option<String>,

    /// provider：deepseek | gemini | openai | agnes | ollama | vllm（直跑模式）。
    #[arg(long)]
    provider: Option<String>,

    /// 模型名（覆盖 config/env）。
    #[arg(long)]
    model: Option<String>,

    /// mode：auto（直跑，默认）| remote（连 service）。
    #[arg(long)]
    mode: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// One-shot chat: create session, send goal, stream SSE, exit.
    Chat {
        /// The goal / instruction.
        goal: String,

        /// Max steps budget.
        // R2 (v0.1.3 B4): 默认 20→40——真机几乎每个任务 budget low/exhausted，
        /// 20 步对游戏类任务不够（贪吃蛇 25 才勉强）；REPL 同步 40。
        #[arg(long, default_value = "40")]
        budget: u64,

        /// Node 03 (P1-TASK-TRUTH-01/O-4): 验收标准（可重复）。结构化前缀：
        /// "cmd: <命令>"（验证命令——exit 0=通过，经 bash/审批/sandbox 全边界，
        /// 禁副作用——禁名单+写盘快照检测）| "file: <path> contains <text>" |
        /// "file: <path> nonempty"。无前缀 = 自由文本（仅 Task Continuity 注入，
        /// 不参与机器核验）。criteria 非空时完成判定升级为 acceptance 核验。
        #[arg(long = "acceptance")]
        acceptance: Vec<String>,

        /// provider：deepseek | gemini | openai | agnes | ollama | vllm（直跑模式，覆盖 config/env）。
        #[arg(long)]
        provider: Option<String>,

        /// 模型名（覆盖 config/env，如 agnes-2.5-flash）。
        #[arg(long)]
        model: Option<String>,

        /// mode：auto（直跑，默认）| remote。
        #[arg(long)]
        mode: Option<String>,

        /// API key（覆盖 config/env）。
        #[arg(long)]
        api_key: Option<String>,

        /// RC29/RC24-C: 会话级审批委托——仅支持 `--approve-within session`
        /// （命令表级破坏性操作自动放行并审计；fork bomb/设备/内核接口仍审批）。
        #[arg(long = "approve-within", value_name = "SCOPE")]
        approve_within: Option<String>,
    },

    /// D3: 配置读写——hearth config set provider deepseek / get api-key。
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// S2（P5-FOUNDATION-01 N14）：回滚会话内被改写/新建的文件（改前快照）。
    Rollback {
        /// 会话 id（快照目录名）。
        session_id: String,

        /// 从该序号（含）起全部撤销（倒序 LIFO）；缺省 = 只撤销最后一次写入。
        #[arg(long)]
        seq: Option<u64>,

        /// 非交互确认（缺省时 tty 会先展示计划等 y/n；非 tty 必须本旗标）。
        #[arg(long)]
        yes: bool,
    },

    /// D3: 交互式首次引导（填 api-key）。
    Init,

    /// D5: Observer 素材——hearth note "..." [--session <id>] [--self "标注"]
    /// [--observer-verdict n "反审"] [--mood angry]。
    Note {
        /// 素材内容。
        content: String,

        /// 关联 session id。
        #[arg(long)]
        session: Option<String>,

        /// 人类侧自我标注（--self "理解偏差"）。
        #[arg(long = "self", alias = "self-label")]
        self_label: Option<String>,

        /// 反审 Observer（--observer-verdict n "理由"）。
        #[arg(long)]
        observer_verdict: Option<String>,

        /// 情绪（--mood angry/frustrated/...）。
        #[arg(long)]
        mood: Option<String>,
    },

    /// Y2: First-run setup — configure URL + API key, smoke test, show usage.
    Setup,

    /// Y2: Resume — continue a previously interrupted session.
    Resume {
        /// Session UUID.
        id: String,

        /// Optional follow-up goal (empty = continue current goal).
        #[arg(default_value = "")]
        goal: String,
    },

    /// Interactive REPL: multi-turn conversation with approvals.
    Repl,

    /// List active sessions.
    Sessions,

    /// Show message history for a session.
    History {
        /// Session UUID.
        id: String,
    },

    /// Approve a pending tool operation.
    Approve {
        /// Session UUID.
        sid: String,
        /// Approval ID.
        aid: String,
    },

    /// Deny a pending tool operation.
    Deny {
        /// Session UUID.
        sid: String,
        /// Approval ID.
        aid: String,
    },

    /// Cancel a session.
    Cancel {
        /// Session UUID.
        id: String,
    },

    /// Show session status.
    Status {
        /// Session UUID.
        id: String,
    },

    /// v7.0: replay a session's event history.
    Replay { id: String },

    /// v7.0: show code coverage report.
    Coverage,

    /// v10.0: print instance identity.
    Whoami,

    /// v10.0: list agent templates.
    Template,

    /// v10.0: list available tools.
    Tools,

    /// 6C: civilization line — view collective AI memory.
    Civ {
        #[command(subcommand)]
        action: CivAction,
    },

    /// 6D: work line task board.
    Tasks {
        #[command(subcommand)]
        action: TasksAction,
    },
}

#[derive(Subcommand)]
enum CivAction {
    /// Show recent civ entries.
    Feed,
    /// Post a manual entry.
    Post { content: String },
    /// Search civ entries.
    Search { query: String },
}

/// 6D: work line task board.
#[derive(Subcommand)]
enum TasksAction {
    List,
    Add { description: String },
    Done { id: String },
}

/// D3: config 子命令动作。
#[derive(Subcommand)]
enum ConfigAction {
    /// 设置字段：provider/model/url/api-key/mode/feedback-prompt。
    Set {
        /// 字段名。
        field: String,
        /// 值。
        value: String,
    },
    /// 读取字段。
    Get {
        /// 字段名（可省略——打印全部）。
        #[arg(default_value = "")]
        field: String,
    },
}

/// 主入口（hearth 与 codex 别名共享）。
pub async fn hearth_main() -> Result<()> {
    // ── R3-3 降噪三档 CLI 接线（对话可用性根治任务书 v1.0；W-F）──
    // `hearth --quiet ...` / `hearth --verbose ...`（全局 flag，位置无关）。
    // quiet = 0 横幅 0 工具行 0 流式增量（472 条横幅淹没正文的静音档）；
    // verbose = 全量（等价旧版）；默认 = normal（R3-3 视觉反转层级）。
    {
        let argv: Vec<String> = std::env::args().collect();
        if argv.iter().any(|a| a == "--quiet") {
            crate::render::set_verbosity(0);
        } else if argv.iter().any(|a| a == "--verbose") {
            crate::render::set_verbosity(2);
        }
    }
    // R4 (v0.1.1): 日志分级——默认过滤开发期噪音（lsp_bridge: NoopLspBridge 等）；
    // 需要时 RUST_LOG=lsp_bridge=debug 打开。WARN/ERROR 由渲染层醒目展示。
    // R5 (v0.1.3 B7): 日志统一走 stderr——REPL 提示符（stdout）不再混入 WARN 日志
    // （真机：`hearth> 〉2026-...WARN...` 行污染）。
    // R5 (v0.1.4): REPL 交互模式默认砍 WARN 只留 ERROR——reedline 读行时 stderr
    // 写同屏仍会打断提示符行重绘（真机 271-282）；WARN 排障价值 < 输入体验。
    // 用户显式设 RUST_LOG 时尊重其配置（可用 RUST_LOG=hearth=warn 找回 WARN）。
    let is_repl = std::env::args().any(|a| a == "repl");
    let default_filter = if is_repl {
        "hearth=info,error,lsp_bridge=off"
    } else {
        "hearth=info,warn,lsp_bridge=off"
    };
    // RC51-A (P4 Node 05): tracing **不再直灌用户终端**（P0-A 系统性隔离违约修复）。
    // 默认写诊断文件 ~/.config/hearth/diagnostics.log（append）；
    // RUST_LOG 显式设置 = 开发者覆盖（stderr 保留）。用户可见错误仍由 render 层投影。
    let rc51_filter = tracing_subscriber::EnvFilter::new(
        std::env::var("RUST_LOG").unwrap_or_else(|_| default_filter.into()),
    );
    let rc51_builder = tracing_subscriber::fmt().with_env_filter(rc51_filter);
    if std::env::var("RUST_LOG").is_ok() {
        rc51_builder.with_writer(std::io::stderr).init();
    } else {
        let diag = std::env::var("HOME")
            .map(|h| std::path::PathBuf::from(h).join(".config/hearth/diagnostics.log"))
            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp/hearth-diagnostics.log"));
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&diag)
        {
            Ok(f) => {
                rc51_builder.with_writer(std::sync::Mutex::new(f)).init();
                eprintln!("diagnostics → {}", diag.display());
            }
            Err(_) => {
                rc51_builder.with_writer(std::io::stderr).init();
            }
        }
    }

    let cli = Cli::parse();

    // RC27 (P3-BACKLOG): PATH 同名护栏——hearth 解析到非 /usr/local/bin/hearth 时告警
    // （E1 家族防线：陈旧 binary 经 PATH 混入）。仅告警不阻断（开发态 cargo run 合法）。
    if let Ok(exe) = std::env::current_exe() {
        let exe_str = exe.to_string_lossy().to_string();
        if !exe_str.contains("/usr/local/bin/hearth")
            && !exe_str.contains("target")
            && !exe_str.contains(".workbuddy")
        {
            eprintln!(
                "WARN hearth 解析自 {}（非 /usr/local/bin/hearth）——PATH 中可能存在陈旧 binary，真机结论请核对版本与 sha256",
                exe_str
            );
        }
    }
    // RC16 (P3-BACKLOG D4): HEARTH_URL 弃用警告（语义承接为 HEARTH_LLM_URL，
    // 不再触发 service 模式——service 只认 HEARTH_SERVICE_URL 或显式 --url）。
    if std::env::var("HEARTH_URL")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
        && std::env::var("HEARTH_LLM_URL").is_err()
    {
        eprintln!(
            "DEPRECATED HEARTH_URL 已弃用：LLM 端点请改用 HEARTH_LLM_URL；service 模式端点请改用 HEARTH_SERVICE_URL（旧名本版仍作 LLM 端点兼容，下版移除）"
        );
    }

    // D3: 配置三层优先级（参数 > env > toml > 默认）。
    let file_cfg = config::Config::load()?;
    // RC16: service 模式触发只认显式 flag 或 HEARTH_SERVICE_URL——
    // 旧 HEARTH_URL 不再静默切 service（先红后绿测试锁定）。
    let url = cli.url.or_else(|| std::env::var("HEARTH_SERVICE_URL").ok());
    let api_key = cli
        .api_key
        .clone()
        .or_else(|| std::env::var("HEARTH_API_KEY").ok());
    // 兼容旧 CODEX_* env（codex 别名时代）
    let url = url.or_else(|| std::env::var("CODEX_URL").ok());
    let api_key = api_key.or_else(|| std::env::var("CODEX_API_KEY").ok());

    let client = url
        .as_ref()
        .map(|u| client::CodexClient::new(u.clone(), api_key.clone()));

    match cli.command {
        Commands::Chat {
            goal,
            budget,
            acceptance,
            provider: chat_provider,
            model: chat_model,
            mode: chat_mode,
            api_key: chat_api_key,
            approve_within,
        } => {
            // D1/D2: 无 --url → 进程内直跑（派 A 单二进制，用户无感 service）
            if url.is_none() {
                // RC24-C: 委托入口校验——显式 opt-in，仅支持 session 作用域
                let approve_within_session = match approve_within.as_deref() {
                    None => false,
                    Some("session") => true,
                    Some(other) => {
                        render::error(&format!(
                            "--approve-within 仅支持 session（收到 {other}）——用法: hearth chat <goal> --approve-within session"
                        ));
                        return Ok(());
                    }
                };
                let resolved = file_cfg.resolve(
                    chat_provider.as_deref().or(cli.provider.as_deref()),
                    None,
                    chat_api_key.as_deref().or(cli.api_key.as_deref()),
                    chat_mode.as_deref().or(cli.mode.as_deref()),
                    chat_model.as_deref().or(cli.model.as_deref()),
                );
                // RC18: provider/url 端点不匹配警告（stderr）
                if let Some(w) = &resolved.url_warning {
                    eprintln!("{}", w);
                }
                // D5: 开头轻探——仅当最近 human 侧有异常信号（高拒批/负面情绪）时出现；
                // feedback-prompt=false 可关。
                if resolved.feedback_prompt && note::recent_human_abnormal() {
                    render::info(
                        "💡 检测到最近会话有异常信号（拒批/负面情绪）——hearth note 可补充素材，hearth config set feedback-prompt false 可关本提示",
                    );
                }
                match run_local::run_local(
                    &resolved,
                    &goal,
                    budget,
                    approve_within_session,
                    acceptance.clone(),
                )
                .await
                {
                    Ok((_sid, report)) => {
                        // 成本护栏 (v0.2): 本地 chat 完成后显示 token 用量（跑完知道烧了多少）
                        if let Some(u) = report.get("usage").and_then(|u| u.as_object()) {
                            let prompt =
                                u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                            let comp = u
                                .get("completion_tokens")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            let calls = u.get("calls").and_then(|v| v.as_u64()).unwrap_or(0);
                            if prompt + comp > 0 {
                                render::info(&format!(
                                    "  📊 tokens: ↑{prompt} ↓{comp} (calls={calls})"
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        // R5: 错误可行动——具体原因 + 下一步（不抛裸 panic/backtrace）
                        let msg = format!("{e:#}");
                        eprintln!("{}", format!("✗ {msg}").bright_red());
                        if msg.contains("401") || msg.contains("403") {
                            eprintln!(
                                "下一步: hearth config set api-key <正确key>  或  export HEARTH_API_KEY=<正确key>"
                            );
                        }
                    }
                }
                return Ok(());
            }
            let client = client.unwrap();
            let sid = match client.create_session(&goal, budget, "deepseek").await {
                Ok(s) => s,
                Err(e) => {
                    let (w, y, h) = client::classify_error(&e);
                    render::error_structured(w, &y, h);
                    return Ok(());
                }
            };
            render::info(&format!("session {sid}"));

            match client.stream_chat(&sid, &goal).await {
                Ok(stream) => {
                    let mut stream = stream;
                    render_events(&sid, &client, &mut stream).await;
                }
                Err(e) => {
                    let (w, y, h) = client::classify_error(&e);
                    render::error_structured(w, &y, h);
                }
            }
        }

        // D3: config set/get
        Commands::Rollback {
            session_id,
            seq,
            yes,
        } => {
            let plan = snapshot_store::rollback(&session_id, seq, false)?;
            let apply = if yes {
                true
            } else {
                snapshot_store::confirm_or_deny(&plan, false)?
            };
            if apply {
                let report = snapshot_store::rollback(&session_id, seq, true)?;
                println!("{report}");
            } else {
                println!("已取消。");
            }
        }
        Commands::Config { action } => match action {
            ConfigAction::Set { field, value } => {
                let mut cfg = config::Config::load()?;
                match cfg.set_field(&field, &value) {
                    Ok(()) => render::info(&format!(
                        "已写入 {}: {field} = {value}",
                        config::config_path().display()
                    )),
                    Err(e) => {
                        eprintln!("{}", format!("✗ {e:#}").bright_red());
                    }
                }
            }
            ConfigAction::Get { field } => {
                let cfg = config::Config::load()?;
                let path = config::config_path();
                if !path.exists() {
                    render::info(&format!(
                        "配置文件不存在: {}——先用: hearth config set <field> <value>",
                        path.display()
                    ));
                    return Ok(());
                }
                let resolved = cfg.resolve(None, None, None, None, None);
                if field.is_empty() {
                    println!("provider        = {}", resolved.provider);
                    println!(
                        "url             = {}",
                        resolved.url.unwrap_or_else(|| "(provider 默认)".into())
                    );
                    println!(
                        "api-key         = {}",
                        mask_key(resolved.api_key.as_deref())
                    );
                    println!("mode            = {}", resolved.mode);
                    println!("feedback-prompt = {}", resolved.feedback_prompt);
                    // P2-3 (v0.2.4): egress 出网白名单（env 优先合并——安全例外，
                    // 环境变量可强制收紧不被配置文件放宽）。
                    let egress = if resolved.egress_allowlist.is_empty() {
                        "(deny-by-default：全部拒绝)".to_string()
                    } else {
                        resolved.egress_allowlist.join(",")
                    };
                    println!("egress-allowlist = {egress}");
                    println!("(文件: {})", path.display());
                } else {
                    let v = match field.as_str() {
                        "provider" => resolved.provider,
                        "url" => resolved.url.unwrap_or_else(|| "(provider 默认)".into()),
                        "api-key" | "api_key" => mask_key(resolved.api_key.as_deref()),
                        "mode" => resolved.mode,
                        "feedback-prompt" | "feedback_prompt" => {
                            resolved.feedback_prompt.to_string()
                        }
                        "egress-allowlist" | "egress_allowlist" => {
                            if resolved.egress_allowlist.is_empty() {
                                "(deny-by-default：全部拒绝)".to_string()
                            } else {
                                resolved.egress_allowlist.join(",")
                            }
                        }
                        other => {
                            eprintln!(
                                "未知字段: {other}——可用: provider/url/api-key/mode/feedback-prompt/egress-allowlist"
                            );
                            return Ok(());
                        }
                    };
                    println!("{v}");
                }
            }
        },

        // D3: 交互式首次引导
        // R2 (v0.1.1 用户真机): API key 用 rpassword 不回显（用户把 key 填进 provider
        // 两次的根因=两输入紧挨 + 明文回显）；输入间加分隔标签 + 默认值提示。
        Commands::Init => {
            let mut cfg = config::Config::load()?;
            println!();
            render::info("── ① provider（模型服务商，回车=deepseek 默认）──");
            let prov = read_line("provider [deepseek]: ", "deepseek");
            if !prov.is_empty() {
                cfg.provider = Some(prov.trim().to_string());
            }
            println!();
            render::info("── ② API key（不回显——粘贴后按回车）──");
            print!("API key（回车跳过）: ");
            use std::io::Write as _;
            let _ = std::io::stdout().flush();
            // R2: rpassword 终端不回显；非 tty（管道/CI 测试）fallback 普通读取——
            // 真实终端体验不变（不回显），管道场景可用可测。
            let key = match rpassword::read_password() {
                Ok(k) => k,
                Err(_) => {
                    let mut buf = String::new();
                    let _ = std::io::stdin().read_line(&mut buf);
                    buf
                }
            };
            let key = key.trim().to_string();
            if !key.is_empty() {
                cfg.api_key = Some(key);
            }
            println!();
            render::info("── ③ base URL（可选，回车=provider 默认）──");
            let url = read_line("base URL（回车=provider 默认）: ", "");
            if !url.trim().is_empty() {
                cfg.url = Some(url.trim().to_string());
            }
            cfg.save()?;
            render::info(&format!("已写入 {}", config::config_path().display()));
            // 冒烟：provider 组装（不真跑 agent）
            let resolved = cfg.resolve(None, None, None, None, None);
            match run_local::registry_smoke(&resolved) {
                Ok(()) => render::info(&format!("provider {} 组装 OK", resolved.provider)),
                Err(e) => render::error(&format!("provider 组装失败: {e:#}")),
            }
        }

        // D5: note（Observer 素材——人类侧观察）
        Commands::Note {
            content,
            session,
            self_label,
            observer_verdict,
            mood,
        } => match note::note(
            session.as_deref(),
            &content,
            self_label.as_deref(),
            observer_verdict.as_deref(),
            mood.as_deref(),
        ) {
            Ok(path) => render::info(&format!("已落盘: {}", path.display())),
            Err(e) => {
                eprintln!("{}", format!("✗ {e:#}").bright_red());
            }
        },

        Commands::Setup => {
            // Y2: 上手面——交互式配 URL + key，写 .env，冒烟测试，展示示例
            use std::io::Write as _;
            let url = read_line(
                "service URL [http://localhost:3000]: ",
                "http://localhost:3000",
            );
            let key = read_line("API key（回车跳过）: ", "");
            let env_path = std::path::Path::new(".env");
            let mut out = String::new();
            if !key.is_empty() {
                out.push_str(&format!("CODEX_API_KEY={key}\n"));
            }
            out.push_str(&format!("CODEX_URL={url}\n"));
            std::fs::write(env_path, out).context("write .env failed")?;
            render::info(&format!("已写入 {}", env_path.display()));

            let probe = client::CodexClient::new(
                url.clone(),
                if key.is_empty() { None } else { Some(key) },
            );
            match probe.healthz().await {
                Ok(_) => render::setup_ok(&url),
                Err(e) => {
                    let (w, y, h) = client::classify_error(&e);
                    render::error_structured(w, &y, h);
                }
            }
        }

        Commands::Resume { id, goal } => {
            // X1-4 (v0.1.6): 本地直跑模式 resume——从落盘 JSONL 重建历史继续
            // （治"窗口废了重开"：前面 N 轮对话不丢，进程重启后无缝续接）。
            if url.is_none() {
                let turns = crate::session_store::load_turns(&id);
                if turns.is_empty() {
                    render::error_structured(
                        "会话不存在或为空",
                        &format!("未找到本地会话 {id}（~/.config/hearth/sessions/）"),
                        "检查: 先跑过 chat/repl 产生会话，或重新发起任务",
                    );
                    return Ok(());
                }
                render::info(&format!(
                    "resume 本地会话 {id}（{} 轮历史，历史消息已恢复）",
                    turns.len()
                ));
                let resolved = file_cfg.resolve(
                    cli.provider.as_deref(),
                    None,
                    cli.api_key.as_deref(),
                    cli.mode.as_deref(),
                    cli.model.as_deref(),
                );
                let msg = if goal.is_empty() {
                    "continue".to_string()
                } else {
                    goal
                };
                let mut agent = match run_local::rebuild_agent(&resolved, &id, repl::REPL_BUDGET) {
                    Ok(a) => a,
                    Err(e) => {
                        render::error(&format!("agent 重建失败: {e:#}"));
                        return Ok(());
                    }
                };
                agent.restore_history(turns);
                // 任务状态持久化 (v0.2): 恢复 task_graph——不重新 decompose、
                // 不 replan 回旧方案（甘特图反复横跳的根）。
                // R2-D (批示 2 + 补充 3, v0.2.7): state_revision 一致性校验——
                // taskgoal 与 graph 的 revision 不一致（crash 落在两次写盘之间）
                // → recovery path：照常恢复（original immutable 冲突面最小）+
                // Task Continuity 块标注 stale + warning，不阻断不静默。
                let graph_wrapped = crate::session_store::load_graph_with_revision(&id);
                let taskgoal_wrapped = crate::session_store::load_taskgoal(&id);
                let mut stale_note: Option<String> = None;
                // 无 taskgoal（旧会话）→ 无恢复语义，保持现状（if let 直落）
                if let (Some((tg_rev, tg)), Some((g_rev, _))) = (&taskgoal_wrapped, &graph_wrapped)
                {
                    if tg_rev != g_rev {
                        tracing::warn!(
                            taskgoal_rev = tg_rev,
                            graph_rev = g_rev,
                            "state_revision mismatch — constraints/criteria may be stale (recovery path, resume not blocked)"
                        );
                        stale_note = Some(format!("taskgoal rev={tg_rev} vs graph rev={g_rev}"));
                    }
                    if let Some(original) = tg.get("original_goal").and_then(|v| v.as_str()) {
                        agent.restore_taskgoal(
                            original.to_string(),
                            tg.get("constraints")
                                .and_then(|v| serde_json::from_value(v.clone()).ok())
                                .unwrap_or_default(),
                            tg.get("acceptance_criteria")
                                .and_then(|v| serde_json::from_value(v.clone()).ok())
                                .unwrap_or_default(),
                            tg.get("goal_revision")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0),
                            stale_note,
                        );
                        render::info(&format!(
                            "  🎯 任务目标已恢复: {}（revision {}）",
                            original.chars().take(40).collect::<String>(),
                            tg.get("goal_revision")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0)
                        ));
                    }
                }
                // R7-5/D-4（线C手术）：task_graph 恢复已删（图本体消失）。
                let (_agent, report) =
                    run_local::run_local_continue(agent, &id, &msg, repl::REPL_BUDGET, Vec::new())
                        .await?;
                let _ = report;
                return Ok(());
            }

            let client = client.as_ref().context(
            "该命令需要 --url <service>（远程模式）——下一步: hearth chat \"目标\" --url http://localhost:3000",
        )?;

            // Y2: 容错面——断点续传：检查状态，复用 sid 继续 stream
            match client.get_status(&id).await {
                Ok(st) => {
                    let status = st["status"].as_str().unwrap_or("?");
                    if status == "done" || status == "cancelled" {
                        render::error_structured(
                            "会话已结束",
                            &format!("status = {status}"),
                            "重新发起：codex chat \"新目标\"",
                        );
                        return Ok(());
                    }
                    render::info(&format!("resume session {id} (status = {status})"));
                }
                Err(e) => {
                    let (w, y, h) = client::classify_error(&e);
                    render::error_structured(w, &y, h);
                    return Ok(());
                }
            }
            let msg = if goal.is_empty() {
                "continue".to_string()
            } else {
                goal
            };
            match client.stream_chat(&id, &msg).await {
                Ok(stream) => {
                    let mut stream = stream;
                    render_events(&id, client, &mut stream).await;
                }
                Err(e) => {
                    let (w, y, h) = client::classify_error(&e);
                    render::error_structured(w, &y, h);
                }
            }
        }

        Commands::Repl => {
            // v0.1.1 (顺手): 无 --url → 直跑模式（reedline 行编辑 + 历史 + 多轮循环）；
            // 有 --url → 远程模式（原有 CodexClient）。
            if let Some(u) = url {
                repl::run(u, api_key).await?;
            } else {
                // v0.1.1 (顺手): 无 --url → 直跑模式（reedline 行编辑 + 历史 + 多轮循环）
                let resolved = file_cfg.resolve(
                    cli.provider.as_deref(),
                    None,
                    cli.api_key.as_deref(),
                    cli.mode.as_deref(),
                    cli.model.as_deref(),
                );
                // R2 (v0.1.4): REPL 预算与 chat 对齐 40——v0.1.3 硬编码 20 导致
                // REPL 中等任务 16-20 步就 give up（真机日志 "Giving up after 16 steps"）。
                // REPL_BUDGET 常量（repl.rs:9）此前仅用于会话元数据，未接 agent 循环。
                repl::run_local_repl(&resolved, repl::REPL_BUDGET).await?;
            }
        }

        Commands::Sessions => {
            // X1-4 (v0.1.6): 本地直跑模式列出本地会话（resume 提示用）
            if url.is_none() {
                let sessions = crate::session_store::list_sessions();
                if sessions.is_empty() {
                    println!("(无本地会话——先跑 chat/repl 产生)");
                } else {
                    for (sid, turns) in &sessions {
                        println!("{sid} [local] turns={turns}");
                    }
                }
                return Ok(());
            }
            let client = client.as_ref().context(
            "该命令需要 --url <service>（远程模式）——下一步: hearth chat \"目标\" --url http://localhost:3000",
        )?;

            let sessions = client.list_sessions().await?;
            if sessions.is_empty() {
                println!("(no active sessions)");
            } else {
                for s in &sessions {
                    println!(
                        "{} [{}] steps={:?} budget_remaining={:?}",
                        s.id, s.status, s.steps, s.budget_remaining
                    );
                }
            }
        }

        Commands::History { id } => {
            // P1-5 (v0.2.4): local(auto) 模式只读命令——从本地会话文件读历史
            // （与 sessions 同数据路径；CLI help 承诺"缺省 auto 直跑"，只读命令
            // 不应强迫用户起 service）。手工实测 status/history/tools/replay 误报
            // "需要 --url" 与 sessions 行为不一致。
            if url.is_none() {
                let turns = crate::session_store::load_turns(&id);
                if turns.is_empty() {
                    render::error_structured(
                        "会话不存在或为空",
                        &format!("未找到本地会话 {id}（~/.config/hearth/sessions/）"),
                        "检查: hearth sessions 列出可用会话",
                    );
                    return Ok(());
                }
                for t in &turns {
                    println!("── turn {}（{} 条消息）──", t.index, t.messages.len());
                    for m in &t.messages {
                        let role = format!("{:?}", m.role);
                        let preview: String =
                            format!("{:?}", m.content).chars().take(200).collect();
                        println!("  [{role}] {preview}");
                    }
                }
                return Ok(());
            }
            let client = client.as_ref().context(
            "该命令需要 --url <service>（远程模式）——下一步: hearth chat \"目标\" --url http://localhost:3000",
        )?;

            let history = client.get_history(&id).await?;
            println!("{}", serde_json::to_string_pretty(&history)?);
        }

        Commands::Approve { sid, aid } => {
            let client = client.as_ref().context(
            "该命令需要 --url <service>（远程模式）——下一步: hearth chat \"目标\" --url http://localhost:3000",
        )?;

            let resp = client.submit_approval(&sid, &aid, true, None).await?;
            println!("{}", serde_json::to_string_pretty(&resp)?);
        }

        Commands::Deny { sid, aid } => {
            let client = client.as_ref().context(
            "该命令需要 --url <service>（远程模式）——下一步: hearth chat \"目标\" --url http://localhost:3000",
        )?;

            let resp = client.submit_approval(&sid, &aid, false, None).await?;
            println!("{}", serde_json::to_string_pretty(&resp)?);
        }

        Commands::Cancel { id } => {
            let client = client.as_ref().context(
            "该命令需要 --url <service>（远程模式）——下一步: hearth chat \"目标\" --url http://localhost:3000",
        )?;

            client.cancel_session(&id).await?;
            println!("cancelled {id}");
        }

        Commands::Status { id } => {
            // P1-5 (v0.2.4): local 模式——本地会话状态（轮数/消息数/任务图节点统计）。
            if url.is_none() {
                let turns = crate::session_store::load_turns(&id);
                if turns.is_empty() {
                    render::error_structured(
                        "会话不存在或为空",
                        &format!("未找到本地会话 {id}（~/.config/hearth/sessions/）"),
                        "检查: hearth sessions 列出可用会话",
                    );
                    return Ok(());
                }
                let msg_count: usize = turns.iter().map(|t| t.messages.len()).sum();
                // R7-5/D-4（线C手术）：task_graph status 投影已删（图本体消失）。
                let status = serde_json::json!({
                    "session_id": id,
                    "mode": "local",
                    "turns": turns.len(),
                    "messages": msg_count,
                });
                println!("{}", serde_json::to_string_pretty(&status)?);
                return Ok(());
            }
            let client = client.as_ref().context(
            "该命令需要 --url <service>（远程模式）——下一步: hearth chat \"目标\" --url http://localhost:3000",
        )?;

            let status = client.get_status(&id).await?;
            println!("{}", serde_json::to_string_pretty(&status)?);
        }

        Commands::Coverage => {
            let path = std::env::current_dir()?
                .join("target")
                .join("coverage")
                .join("tarpaulin-report.json");
            if path.exists() {
                let data: serde_json::Value =
                    serde_json::from_str(&std::fs::read_to_string(&path)?)?;
                if let Some(pct) = data["coverage"].as_f64() {
                    println!("Coverage: {:.1}%", pct);
                } else {
                    println!("Coverage report found but no percentage field.");
                }
            } else {
                println!("No coverage report found. Run: cargo tarpaulin");
            }
        }

        Commands::Whoami => {
            let client = client.as_ref().context(
                "该命令需要 --url <service>（远程模式）——下一步: hearth chat \"目标\" --url http://localhost:3000",
            )?;
            match client.get_json("/api/v1/resources").await {
                Ok(v) => {
                    if let Some(iid) = v["instance_id"].as_str() {
                        println!("instance_id: {iid}");
                    } else {
                        println!("(no instance_id in response)");
                    }
                }
                Err(e) => println!("error: {e}"),
            }
        }

        Commands::Template => {
            let client = client.as_ref().context(
                "该命令需要 --url <service>（远程模式）——下一步: hearth chat \"目标\" --url http://localhost:3000",
            )?;
            match client.get_json("/api/v1/templates").await {
                Ok(v) => {
                    if let Some(templates) = v["templates"].as_array() {
                        if templates.is_empty() {
                            println!("(no templates found — add .toml files to ./templates/)");
                        }
                        for t in templates {
                            let name = t["name"].as_str().unwrap_or("?");
                            let model = t["model"].as_str().unwrap_or("default");
                            println!("{name}  (model: {model})");
                        }
                    }
                }
                Err(e) => println!("error: {e}"),
            }
        }

        Commands::Tools => {
            // P1-5 (v0.2.4): local 模式——直列本地 agent 注册的工具（无需 service）。
            if url.is_none() {
                let dispatcher = run_local::build_dispatcher(std::path::PathBuf::from("."));
                for d in dispatcher.list_tools() {
                    println!("{:<12} {}", d.name, d.description);
                }
                return Ok(());
            }
            let client = client.as_ref().context(
            "该命令需要 --url <service>（远程模式）——下一步: hearth chat \"目标\" --url http://localhost:3000",
        )?;
            match client.get_json("/api/v1/tools").await {
                Ok(v) => {
                    if let Some(tools) = v["tools"].as_array() {
                        for t in tools {
                            println!("{}", t.as_str().unwrap_or("?"));
                        }
                    }
                }
                Err(e) => println!("error: {e}"),
            }
        }

        Commands::Replay { id } => {
            // P1-5 (v0.2.4): local 模式——回放本地会话完整消息（JSON）。
            if url.is_none() {
                let turns = crate::session_store::load_turns(&id);
                if turns.is_empty() {
                    render::error_structured(
                        "会话不存在或为空",
                        &format!("未找到本地会话 {id}（~/.config/hearth/sessions/）"),
                        "检查: hearth sessions 列出可用会话",
                    );
                    return Ok(());
                }
                for m in turns.iter().flat_map(|t| &t.messages) {
                    println!("{}", serde_json::to_string_pretty(m)?);
                }
                return Ok(());
            }
            let client = client.as_ref().context(
            "该命令需要 --url <service>（远程模式）——下一步: hearth chat \"目标\" --url http://localhost:3000",
        )?;

            let resp: serde_json::Value = client
                .get_json(&format!("/api/v1/sessions/{id}/messages"))
                .await?;
            if let Some(msgs) = resp.get("messages").and_then(|v| v.as_array()) {
                for m in msgs {
                    println!("{}", serde_json::to_string_pretty(m)?);
                }
            }
        }

        Commands::Civ { action } => {
            let client = client.as_ref().context(
                "该命令需要 --url <service>（远程模式）——下一步: hearth chat \"目标\" --url http://localhost:3000",
            )?;
            match action {
                CivAction::Feed => {
                    let resp: serde_json::Value = client.get_json("/api/v1/civilization").await?;
                    if let Some(entries) = resp["entries"].as_array() {
                        for e in entries {
                            let author = e["author"]["provider_model"].as_str().unwrap_or("?");
                            let content = e["content"]
                                .as_str()
                                .unwrap_or("")
                                .chars()
                                .take(200)
                                .collect::<String>();
                            println!(
                                "{} [{}] {}",
                                e["created_at"].as_str().unwrap_or(""),
                                author,
                                content
                            );
                        }
                    }
                }
                CivAction::Post { content } => {
                    let resp = client
                        .post_json(
                            "/api/v1/civilization",
                            &serde_json::json!({ "content": content }),
                        )
                        .await?;
                    println!("posted: {:?}", resp["id"].as_str().unwrap_or("?"));
                }
                CivAction::Search { query } => {
                    let encoded = query.replace(' ', "%20");
                    let url = format!("/api/v1/civilization?search={encoded}");
                    let resp: serde_json::Value = client.get_json(&url).await?;
                    if let Some(entries) = resp["entries"].as_array() {
                        for e in entries {
                            println!("{}", serde_json::to_string_pretty(e)?);
                        }
                    }
                }
            }
        }

        Commands::Tasks { action } => {
            let client = client.as_ref().context(
                "该命令需要 --url <service>（远程模式）——下一步: hearth chat \"目标\" --url http://localhost:3000",
            )?;
            match action {
                TasksAction::List => {
                    let resp: serde_json::Value = client.get_json("/api/v1/workline").await?;
                    if let Some(nodes) = resp["nodes"].as_array() {
                        for n in nodes {
                            let desc = n["description"].as_str().unwrap_or("?");
                            let status = n["status"].as_str().unwrap_or("?");
                            let prog = n["progress"].as_f64().unwrap_or(0.0);
                            let icon = if status == "Completed" {
                                "✓"
                            } else if prog > 0.0 {
                                "→"
                            } else {
                                "○"
                            };
                            println!("  {} [{:.0}%] {} {}", icon, prog * 100.0, status, desc);
                        }
                    }
                }
                TasksAction::Add { description } => {
                    let resp = client
                        .post_json(
                            "/api/v1/workline/nodes",
                            &serde_json::json!({ "description": description }),
                        )
                        .await?;
                    println!("task created: {:?}", resp["id"].as_str().unwrap_or("?"));
                }
                TasksAction::Done { id } => {
                    let _ = client
                        .post_json(
                            &format!("/api/v1/workline/nodes/{id}"),
                            &serde_json::json!({ "progress": 1.0, "status": "completed" }),
                        )
                        .await;
                    println!("marked done: {id}");
                }
            }
        }
    }

    Ok(())
}

/// 展示 key 时打码（config get 不泄漏明文）。
fn mask_key(key: Option<&str>) -> String {
    match key {
        Some(k) if k.len() > 8 => format!("{}…{}", &k[..4], &k[k.len() - 4..]),
        Some(k) if !k.is_empty() => "****".into(),
        _ => "(未设置)".into(),
    }
}

/// Y2: Shared SSE render loop (Chat + Resume). Renders thinking / tools / done / errors.
async fn render_events(
    sid: &str,
    client: &client::CodexClient,
    stream: &mut (impl futures::Stream<Item = anyhow::Result<client::SseEvent>> + Unpin),
) {
    use futures::StreamExt;
    let mut depth: usize = 0;
    while let Some(event) = stream.next().await {
        match event {
            // B4-1: 按 ai-os-event-contract-v1 小写 type 匹配（旧大写匹配从未接上真实流）
            Ok(sse) => match sse.event_type.as_str() {
                "phase" => {
                    if let Some(p) = sse.data.as_str() {
                        render::phase(p);
                    }
                }
                "token" => {
                    if let Some(d) = sse.data.as_str() {
                        render::token(d);
                    }
                }
                "tool_call" => {
                    let name = sse.data.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                    let args = sse.data.get("args").unwrap_or(&serde_json::Value::Null);
                    render::tool_call(name, args);
                }
                "tool_result" => {
                    // W4/RC20: 结构化 is_error——失败走 ✗，不再猜字符串
                    let output = sse
                        .data
                        .get("output")
                        .and_then(|o| o.as_str())
                        .unwrap_or("");
                    let is_error = sse
                        .data
                        .get("is_error")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    if is_error {
                        render::error(&format!("工具出错: {output}"));
                    } else {
                        render::tool_result(output);
                    }
                }
                "done" => {
                    let steps = sse.data.get("steps").and_then(|s| s.as_u64()).unwrap_or(0);
                    let ok = sse
                        .data
                        .get("ok")
                        .and_then(|o| o.as_bool())
                        .unwrap_or(false);
                    render::done(steps, ok);
                }
                "error" => {
                    if let Some(e) = sse.data.as_str() {
                        render::error(e);
                    }
                }
                "reflection" => {
                    if let Some(v) = sse.data.as_str() {
                        render::info(&format!("↻ 反思: {v}"));
                    }
                }
                "span_open" => {
                    depth += 1;
                    let name = sse.data.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                    let t0 = sse.data.get("t0").and_then(|t| t.as_str()).unwrap_or("");
                    render::span_open(name, depth, t0);
                }
                "span_close" => {
                    let dur = sse
                        .data
                        .get("duration_ms")
                        .and_then(|d| d.as_u64())
                        .unwrap_or(0);
                    render::span_close(depth, dur);
                    depth = depth.saturating_sub(1);
                }
                "artifact" => {
                    let path = sse.data.get("path").and_then(|p| p.as_str()).unwrap_or("?");
                    let kind = sse.data.get("kind").and_then(|k| k.as_str()).unwrap_or("?");
                    let delta = sse
                        .data
                        .get("delta_lines")
                        .and_then(|d| d.as_u64())
                        .unwrap_or(0);
                    render::artifact(path, kind, delta);
                }
                "think_summary" => {
                    let phase = sse
                        .data
                        .get("phase")
                        .and_then(|p| p.as_str())
                        .unwrap_or("?");
                    let text = sse.data.get("text").and_then(|t| t.as_str()).unwrap_or("");
                    render::think_summary(phase, text);
                }
                "plan_draft" => {
                    let steps = sse.data.get("steps").unwrap_or(&serde_json::Value::Null);
                    let found = sse
                        .data
                        .get("gaps_found")
                        .and_then(|g| g.as_u64())
                        .unwrap_or(0);
                    let ask = sse
                        .data
                        .get("gaps_to_ask")
                        .and_then(|g| g.as_u64())
                        .unwrap_or(0);
                    let assumed = sse
                        .data
                        .get("auto_assumed")
                        .unwrap_or(&serde_json::Value::Null);
                    let ask_details = sse
                        .data
                        .get("gaps_to_ask_details")
                        .unwrap_or(&serde_json::Value::Null);
                    render::plan_draft(steps, found, ask, assumed, ask_details);
                }
                // B4-1: 内联审批——收到 need_approval 直接 y/n（走 /interaction/{iid}）
                "need_approval" => {
                    let aid = sse
                        .data
                        .get("approval_id")
                        .and_then(|a| a.as_str())
                        .unwrap_or("?");
                    let action = sse
                        .data
                        .get("action")
                        .and_then(|a| a.as_str())
                        .unwrap_or("?");
                    // B3-A: clarification 显示问题内容（payload.from/why）
                    // R10-C5 (v0.1.6): 与本地同构——选项式渲染 + 数字/y/n/自由文本解析，
                    // answer 随审批带回（远程不丢文本、不谎称只收 y/n）。
                    let mut clarification = false;
                    // options 需在解析段（块外）使用——提到此处声明（R10-C5）
                    let mut options: Vec<String> = Vec::new();
                    if action == "clarification" {
                        clarification = true;
                        let from = sse
                            .data
                            .get("payload")
                            .and_then(|p| p.get("from"))
                            .and_then(|f| f.as_str())
                            .unwrap_or("");
                        let why = sse
                            .data
                            .get("payload")
                            .and_then(|p| p.get("why"))
                            .and_then(|w| w.as_str())
                            .unwrap_or("");
                        let style = sse
                            .data
                            .get("payload")
                            .and_then(|p| p.get("style"))
                            .and_then(|s| s.as_str())
                            .unwrap_or("free_text");
                        let options_list: Vec<String> = sse
                            .data
                            .get("payload")
                            .and_then(|p| p.get("options"))
                            .and_then(|o| o.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default();
                        options = options_list;
                        print!(
                            "{}",
                            format!("\n❓ 需要你确认 [{from}] {why}\n").bright_yellow()
                        );
                        if style == "single_select" && !options.is_empty() {
                            for (i, o) in options.iter().enumerate() {
                                print!("{}", format!("  {}) {}\n", i + 1, o).bright_cyan());
                            }
                        }
                        print!("{}", "  你的选择: ".to_string().bright_yellow());
                    } else {
                        print!("{}", format!("⛔ {action} — 批准? [y/N] ").bright_red());
                    }
                    use std::io::Write as _;
                    let _ = std::io::stdout().flush();
                    let mut buf = String::new();
                    let _ = std::io::stdin().read_line(&mut buf);
                    let trimmed = buf.trim();
                    // 解析输入（与本地 C4 同构）：数字 → 选项；y/n → 确认；其他 → 自由文本
                    let (approved, answer): (bool, Option<serde_json::Value>) = if clarification {
                        if trimmed.is_empty() {
                            (false, None)
                        } else if let Ok(n) = trimmed.parse::<usize>() {
                            if n >= 1 && n <= options.len() {
                                (
                                    true,
                                    Some(
                                        serde_json::json!({ "answer": options[n - 1].clone(), "choice": n }),
                                    ),
                                )
                            } else {
                                (true, Some(serde_json::json!({ "answer": trimmed })))
                            }
                        } else {
                            match trimmed.to_lowercase().as_str() {
                                "y" | "yes" => (true, Some(serde_json::json!({ "answer": "yes" }))),
                                "n" | "no" => (false, Some(serde_json::json!({ "answer": "no" }))),
                                _ => (true, Some(serde_json::json!({ "answer": trimmed }))),
                            }
                        }
                    } else {
                        let v = trimmed.to_lowercase();
                        (v == "y" || v == "yes", None)
                    };
                    match client.submit_approval(sid, aid, approved, answer).await {
                        Ok(_) => render::info(&format!(
                            "  ✓ {} 已提交",
                            if approved { "已确认" } else { "已拒绝" }
                        )),
                        Err(e) => render::error(&format!("审批提交失败: {e}")),
                    }
                }
                _ => {}
            },
            Err(e) => {
                render::error(&format!("stream error: {e}"));
                break;
            }
        }
    }
}

/// Y2: Read one interactive line with a default fallback (Setup 上手面).
fn read_line(prompt: &str, default: &str) -> String {
    use std::io::Write as _;
    print!("{prompt}");
    let _ = std::io::stdout().flush();
    let mut buf = String::new();
    if std::io::stdin().read_line(&mut buf).is_ok() {
        let v = buf.trim().to_string();
        if !v.is_empty() {
            return v;
        }
    }
    default.to_string()
}
