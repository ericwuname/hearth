//! hearth — Hearth CLI（正名，docs/hearth-naming.md）。
//!
//! 派 A 单二进制：进程内直跑内核（默认 auto 模式），也可 --url 连远程 service。
//! 配置三层优先级：参数 > env > ~/.config/hearth/config.toml > 内置默认。

pub mod client;
pub mod config;
pub mod export;
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

/// D-196（2026-10-08, traecode）：解析持久化 taskgoal 的 `constraints` / `acceptance_criteria`。
///
/// 病灶：恢复（resume）时用 `tg.get("constraints").and_then(|v| serde_json::from_value(v.clone())
/// .ok()).unwrap_or_default()` ⇒ 形状不符（旧版本 / 部分写 / 损坏）时**静默置空**，而用户仍被
/// 告知"任务目标已恢复" ⇒ 任务在**失去原约束/验收标准**下继续（D-164/D-177/D-184 同族）。
/// 现改为**显式**解析并留痕：字段**缺失**（旧会话）→ 空列表（合法，无恢复语义）；字段**存在但
/// 形状不符** → 空列表 **+ 一条 note**（由调用方并入 stale 留痕，与 revision mismatch 同通道）。
/// 返回 `(constraints, acceptance_criteria, note)`。
fn parse_taskgoal_lists(tg: &serde_json::Value) -> (Vec<String>, Vec<String>, Option<String>) {
    let mut notes: Vec<String> = Vec::new();
    let one = |key: &str, notes: &mut Vec<String>| -> Vec<String> {
        match tg.get(key) {
            None => Vec::new(),
            Some(v) => match serde_json::from_value(v.clone()) {
                Ok(list) => list,
                Err(e) => {
                    notes.push(format!("{key} 解析失败已置空: {e}"));
                    Vec::new()
                }
            },
        }
    };
    let constraints = one("constraints", &mut notes);
    let criteria = one("acceptance_criteria", &mut notes);
    let note = if notes.is_empty() {
        None
    } else {
        Some(notes.join("; "))
    };
    (constraints, criteria, note)
}

/// D-244：平台规范安装位置（Windows = `%USERPROFILE%\.cargo\bin\hearth.exe`；
/// Unix = `/usr/local/bin/hearth`）。空串 = 无法确定（此时只信开发构建目录）。
fn expected_hearth_path() -> String {
    if cfg!(windows) {
        std::env::var_os("USERPROFILE")
            .map(|h| {
                std::path::PathBuf::from(h)
                    .join(".cargo")
                    .join("bin")
                    .join("hearth.exe")
            })
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    } else {
        "/usr/local/bin/hearth".to_string()
    }
}

/// 路径是否可信：规范安装位置（大小写不敏感）或开发构建目录（`target` / `.workbuddy`）。
/// 纯函数（可单测；反斜杠归一化后比较）。
fn path_is_trusted(exe: &str, canonical: &str) -> bool {
    if !canonical.is_empty() && exe.eq_ignore_ascii_case(canonical) {
        return true;
    }
    let norm = exe.replace('\\', "/").to_ascii_lowercase();
    norm.contains("/target/") || norm.contains("/.workbuddy/")
}

#[cfg(test)]
mod path_guard_tests {
    use super::{expected_hearth_path, path_is_trusted};

    #[test]
    fn canonical_install_is_trusted_case_insensitively() {
        let canon = r"C:\Users\x\.cargo\bin\hearth.exe";
        assert!(path_is_trusted(r"c:\users\X\.CARGO\bin\hearth.exe", canon));
    }

    #[test]
    fn dev_build_paths_are_trusted() {
        let canon = r"C:\Users\x\.cargo\bin\hearth.exe";
        assert!(path_is_trusted(r"C:\repo\target\debug\hearth.exe", canon));
        assert!(path_is_trusted(
            r"C:\Users\x\.workbuddy\bin\hearth.exe",
            canon
        ));
    }

    #[test]
    fn unknown_location_is_flagged() {
        let canon = r"C:\Users\x\.cargo\bin\hearth.exe";
        assert!(!path_is_trusted(r"D:\old\hearth.exe", canon));
    }

    #[test]
    fn expected_path_is_platform_shaped() {
        let p = expected_hearth_path();
        if cfg!(windows) {
            assert!(p.ends_with(r".cargo\bin\hearth.exe"), "{p}");
        } else {
            assert_eq!(p, "/usr/local/bin/hearth");
        }
    }
}

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
    command: Option<Commands>,

    /// S18（手术包二）：headless 单轮——`hearth -p "<提问>"`。
    /// stdout 仅最终正文（quiet 档：进度/思考/工具行走 stderr），退出码语义：
    /// 0=成功、非 0=失败——可直接用于脚本管道 `$(hearth -p "...")`。
    #[arg(short = 'p', long = "print", value_name = "PROMPT")]
    print: Option<String>,

    /// Service URL（提供则连远程 service；缺省 auto 直跑）。
    #[arg(long, global = true)]
    url: Option<String>,

    /// API key（优先级高于 env/config）。
    #[arg(long, global = true)]
    api_key: Option<String>,

    /// provider：deepseek | gemini | openai | agnes | ollama | vllm（直跑模式）。
    #[arg(long, global = true)]
    provider: Option<String>,

    /// 模型名（覆盖 config/env）。
    #[arg(long, global = true)]
    model: Option<String>,

    /// mode：auto（按 --url 自动判定，默认）| remote（强制远程，需 --url）。
    #[arg(long, global = true)]
    mode: Option<String>,
}

#[derive(Subcommand)]
enum SessionAction {
    /// 导出会话证据（改动 diff + 命令日志 + 每轮报告）为单个 JSON。
    Export {
        /// Session UUID.
        id: String,
        /// 输出路径（缺省 <cwd>/hearth-session-<id前8位>.json）。
        #[arg(long)]
        out: Option<String>,
    },
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

        // D-122（2026-10-02, traecode）：`--provider/--model/--mode/--api-key` 曾在本变体里
        // **再声明一份**，于是它们只在**后置**写法下被识别，而顶层同名参数（未标 global）
        // 只在**前置**写法下被识别 —— CLI 自己在十几处错误提示里推荐的却是后置写法
        // （`hearth chat "目标" --url …`），用户照抄必得 `unexpected argument`。
        // 现统一为**顶层 global 单一定义**（见 `Cli`），前后置均可、不再重复声明。
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
    /// [--observer-verdict <n|其他>] [--mood angry]。
    Note {
        /// 素材内容。
        content: String,

        /// 关联 session id。
        #[arg(long)]
        session: Option<String>,

        /// 人类侧自我标注（--self "理解偏差"）。
        #[arg(long = "self", alias = "self-label")]
        self_label: Option<String>,

        /// 反审 Observer（`--observer-verdict n`）。**注意**：只接受**一个**值；
        /// 且仅当值**恰为** `n` 时才触发"异常信号"（`note.rs` 按精确等值判定，
        /// 写成 `--observer-verdict n "理由"` 会被 clap 判为多余参数 → 报错）。
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

        /// PC-2 修复（P0/P1 修复任务书 v1.0）：预算**追加**步数（总预算 =
        /// 断点已用 + N；缺省 = 默认预算档）。resume 是同一任务的继续——
        /// steps 接着数，追加的部分是净余量。
        #[arg(long)]
        budget: Option<u64>,

        /// PC-5（执行窗，2026-09-11）：审批委托——resume 续跑时的破坏性操作
        /// 需要与 chat 同款的委托能力。缺失时非交互 stdin 一律 DenyAll →
        /// 续跑在首个写操作处被拒（EMBER M1 实测：resume 恢复 7 轮历史后
        /// 因 approval_denied_noninteractive 直接 Done，续跑形同虚设）。
        #[arg(long)]
        approve_within: Option<String>,
    },

    /// Interactive REPL: multi-turn conversation with approvals.
    Repl,

    /// M2: Terminal IDE — full-screen TUI (streaming / approvals / interrupt).
    Tui {
        /// 启动即提交的目标（缺省 = 空白输入框等待输入）。
        #[arg(long)]
        goal: Option<String>,

        /// Max steps budget.
        #[arg(long, default_value = "40")]
        budget: u64,

        /// 无终端冒烟：跑一轮后打印 TUI-SMOKE 摘要（CI / 无 TTY 环境验证接线）。
        #[arg(long)]
        headless: bool,

        /// 仅 headless：N 毫秒后自动优雅中断（验证 Esc 路径 / 落盘 / resume）。
        #[arg(long, hide = true)]
        interrupt_after_ms: Option<u64>,

        /// 仅 headless：审批请求自动批准（验证审批事件 → dispatcher 放行全链）。
        #[arg(long, hide = true)]
        auto_approve: bool,
    },

    /// M2-G: 原生桌面 GUI（egui，单进程；复用同一 agent 内核）。
    Gui {
        /// 启动即提交的目标（缺省 = 空白输入框等待输入）。
        #[arg(long)]
        goal: Option<String>,

        /// Max steps budget.
        #[arg(long, default_value = "40")]
        budget: u64,
    },

    /// M2-5: 会话证据——`hearth session export <id>` 导出 diff + 报告 + 命令日志。
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },

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

/// K-2（契约手术，2026-09-11）：headless 答案提取——读会话 jsonl，取**最后一条
/// assistant 文本**（Message.content 的 `{"Text": "..."}` 或纯 String 形态）。
/// 只读、失败返回 None（headless 下无答案则不输出——不伪造）。
fn headless_answer(session_id: &str) -> Option<String> {
    let path = crate::session_store::sessions_dir().join(format!("{session_id}.jsonl"));
    // D-125（2026-10-02）：有界读入。旧实现 `std::fs::read_to_string` 无上限——
    // 本函数要的是**最后一条** assistant 文本，故不能只读头部：cap 取与会话档同值
    // （64 MiB，见 `session_store::SESSION_FILE_CAP`），截断时 warn 留痕。
    // （宁可明确告知"档被截断、答案可能来自被截断的窗口"，也不静默给出错的"最后一条"。）
    let (content, truncated) =
        bounded_io::read_file_text_capped_std(&path, crate::session_store::SESSION_FILE_CAP)
            .ok()?;
    if truncated {
        tracing::warn!(
            path = %path.display(),
            "会话档超过上限，headless 取答案时已截断读取（本次答案可能不完整）"
        );
    }
    let mut last: Option<String> = None;
    for line in content.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(msgs) = v.get("messages").and_then(|m| m.as_array()) else {
            continue;
        };
        for m in msgs {
            let is_asst = matches!(
                m.get("role").and_then(|r| r.as_str()),
                Some("Assistant") | Some("assistant")
            );
            if !is_asst {
                continue;
            }
            let t = match m.get("content") {
                Some(serde_json::Value::String(x)) => Some(x.clone()),
                Some(serde_json::Value::Object(o)) => o
                    .get("Text")
                    .and_then(|x| x.as_str())
                    .map(|x| x.to_string()),
                _ => None,
            };
            if let Some(t) = t {
                if !t.trim().is_empty() {
                    last = Some(t);
                }
            }
        }
    }
    last
}

/// D-102（2026-10-02, traecode）：`--mode` 的**唯一真实语义**。
///
/// 病灶：该 flag（与 `config set mode`、`HEARTH_MODE`）此前只被塞进
/// `ResolvedConfig.mode` 后**全仓无人读取**——即静默忽略：`hearth chat x --mode remote`
/// 不给 `--url` 时仍会本地直跑，用户以为自己连的是远程服务。
///
/// 现定义（与历史行为兼容）：
/// - `auto`（默认）：按 `--url` 自动判定（有 url = 远程，无 url = 本地直跑）——不变；
/// - `remote`：**强制远程**，必须给 `--url`，否则明确报错（而不是悄悄本地直跑）；
/// - 其它值：拒绝（绝不静默忽略用户输入）。
fn validate_mode(mode: Option<&str>, has_url: bool) -> Option<String> {
    match mode.unwrap_or("auto") {
        "auto" => None,
        "remote" if has_url => None,
        "remote" => Some(
            "--mode remote 需要 --url <service>——例如: hearth chat \"目标\" --url http://localhost:3000"
                .into(),
        ),
        other => Some(format!(
            "--mode 只支持 auto|remote（收到 {other}）——auto=按 --url 自动判定；remote=强制远程（需 --url）"
        )),
    }
}

/// 主入口（hearth 与 codex 别名共享）。
pub async fn hearth_main() -> Result<()> {
    // ── D-106（2026-10-02, traecode）：加载 `.env` ──
    //
    // 病灶：`hearth setup` 会写 `./.env`（`CODEX_URL`/`CODEX_API_KEY`），仓库里也有
    // 既定的上手模板 `.env.example`（"复制为 .env 并填入真实值"）——**但 CLI 从不读
    // .env**（只有 service 侧 `dotenvy::dotenv()`），于是整条 onboarding 路径对 CLI
    // 是死的：用户按模板配好 key，`hearth chat` 依旧报"未配置"。
    //
    // 处置＝接线（而非停写 .env）：`.env` 已在 `.gitignore` 首行（无泄露面），service
    // 早已加载（两侧解释同一份部署应一致），且 `dotenvy` **不覆盖**已存在的进程 env
    // ⇒ 与既有的"参数 > env > config > 默认"优先级不冲突（进程 env 仍然最优先）。
    //
    // 位置必须在**任何 env 读取之前**（下方 provider/url/api_key 解析全部依赖 env）。
    let _ = dotenvy::dotenv();

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
    // R4 (v0.1.1): 日志分级——默认过滤开发期噪音；需要时 RUST_LOG=...=debug 打开。
    // P1-10（D-40 收口）：原 `lsp_bridge=off` 条目随 `lsp-bridge` crate 删除一并移除
    // （crate 已不存在，过滤目标名失效；EnvFilter 对未知 target 静默忽略，但仍应清理）。
    // WARN/ERROR 由渲染层醒目展示。
    // R5 (v0.1.3 B7): 日志统一走 stderr——REPL 提示符（stdout）不再混入 WARN 日志
    // （真机：`hearth> 〉2026-...WARN...` 行污染）。
    // R5 (v0.1.4): REPL 交互模式默认砍 WARN 只留 ERROR——reedline 读行时 stderr
    // 写同屏仍会打断提示符行重绘（真机 271-282）；WARN 排障价值 < 输入体验。
    // 用户显式设 RUST_LOG 时尊重其配置（可用 RUST_LOG=hearth=warn 找回 WARN）。
    let is_repl = std::env::args().any(|a| a == "repl");
    let default_filter = if is_repl {
        "hearth=info,error"
    } else {
        "hearth=info,warn"
    };
    // RC51-A (P4 Node 05): tracing **不再直灌用户终端**（P0-A 系统性隔离违约修复）。
    // 默认写诊断文件 ~/.config/hearth/diagnostics.log（append）；
    // RUST_LOG 显式设置 = 开发者覆盖（stderr 保留）。用户可见错误仍由 render 层投影。
    let rc51_filter = tracing_subscriber::EnvFilter::new(
        std::env::var("RUST_LOG").unwrap_or_else(|_| default_filter.into()),
    );
    let rc51_builder = tracing_subscriber::fmt().with_env_filter(rc51_filter);
    // M2（视觉验收实测）：TUI 占整屏且处于 raw mode——tracing 只要写 stderr 就会
    // 在界面上画出一片日志（截图实证）。`tui` 子命令一律**不写终端**：强制落诊断
    // 文件；连文件都开不了时退到 `sink`（宁可丢日志，不可画花界面）。
    let tui_mode = std::env::args().nth(1).as_deref() == Some("tui");
    if std::env::var("RUST_LOG").is_ok() && !tui_mode {
        rc51_builder.with_writer(std::io::stderr).init();
    } else {
        let diag = std::env::var("HOME")
            .map(|h| std::path::PathBuf::from(h).join(".config/hearth/diagnostics.log"))
            .or_else(|_| {
                std::env::var("APPDATA").map(|a| {
                    std::path::PathBuf::from(a)
                        .join("hearth")
                        .join("diagnostics.log")
                })
            })
            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp/hearth-diagnostics.log"));
        if let Some(parent) = diag.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&diag)
        {
            Ok(f) => {
                rc51_builder.with_writer(std::sync::Mutex::new(f)).init();
                if !tui_mode {
                    eprintln!("diagnostics → {}", diag.display());
                }
            }
            Err(_) if tui_mode => {
                rc51_builder.with_writer(std::io::sink).init();
            }
            Err(_) => {
                rc51_builder.with_writer(std::io::stderr).init();
            }
        }
    }

    let mut cli = Cli::parse();
    // ── S18（手术包二）headless 单轮：`-p "<提问>"` ≡ `--quiet chat "<提问>"` ──
    // 零复制实现：改写为等价 Chat 子命令；quiet 档（progress 走 stderr 语义由
    // render 的 quiet() 保证）——stdout 仅正文，便于脚本捕获。
    if let Some(prompt) = cli.print.take() {
        crate::render::set_verbosity(0);
        // K-2（契约手术）：headless 输出契约——stdout 只出答案；进度/收尾/总结走 stderr。
        crate::render::set_headless(true);
        cli.command = Some(Commands::Chat {
            goal: prompt,
            budget: 40,
            acceptance: Vec::new(),
            approve_within: Some("session".to_string()),
        });
    }

    // RC27/D-244：PATH 同名护栏——只有**非规范安装位置且非开发构建**才告警
    // （E1 家族防线：陈旧 binary 经 PATH 混入）。仅告警不阻断。
    // 修复前按 Linux 的 `/usr/local/bin/hearth` 判定，Windows 的规范位置
    // `%USERPROFILE%\.cargo\bin\hearth.exe` 每次启动都被误报（用户实测）。
    if let Ok(exe) = std::env::current_exe() {
        let exe_str = exe.to_string_lossy().to_string();
        let canonical = expected_hearth_path();
        if !path_is_trusted(&exe_str, &canonical) {
            let expected = if canonical.is_empty() {
                "平台规范安装位置（未知）".to_string()
            } else {
                canonical
            };
            eprintln!(
                "WARN hearth 解析自 {exe_str}（非 {expected}）——PATH 中可能存在陈旧 binary；请用 `hearth --version` 核对 commit 与 sha256"
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
    // 修复 4（顶层批复）：config 为真相源——max_tokens 覆盖注入 agent 侧取参
    //（复用既有 HEARTH_MAX_TOKENS 读取路径；缺省保持内置默认，零行为变化）。
    if let Some(mt) = file_cfg.max_tokens {
        std::env::set_var("HEARTH_MAX_TOKENS", mt.to_string());
    }
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

    // S18：command 为 Option（-p 模式已在上面转为 Chat）——缺失则给用法。
    let Some(command) = cli.command else {
        render::error(
            "未提供命令——用法: hearth -p \"<提问>\"（headless）| hearth chat <goal> | hearth repl | hearth --help",
        );
        return Ok(());
    };

    // D-102：`--mode` 校验（唯一真实语义见 `validate_mode`）——此前该 flag 被静默忽略。
    // D-129（2026-10-02）：校验对象从 **flag** 改为 `mode` 的**有效值**——D-102 只修了
    // flag 一路，`config set mode` / `HEARTH_MODE` 两个来源仍**既不生效也不报错**：
    // 实测 `HEARTH_MODE=remote` 不给 `--url` 仍**本地直跑**（用户以为连的是远程）、
    // `HEARTH_MODE=bogus` 连报错都没有。现按 arg > `HEARTH_MODE` > config.toml
    // 解析有效值并校验，报错里点名**来源**（可行动）。
    //
    // 作用域**刻意收窄**到"真正依赖本机/远程路由"的子命令：`mode` 只影响
    // chat/repl/resume/replay 这几条本地↔远程的判定；若对所有子命令都拦，
    // 一旦 config.toml 里存了一个坏值，连**修它用的** `hearth config set mode auto`
    // 都会被这条校验挡住 ⇒ 用户被锁死在 CLI 之外（只能手改文件）。
    {
        let routes_locally_or_remotely = matches!(
            command,
            Commands::Chat { .. }
                | Commands::Repl
                | Commands::Resume { .. }
                | Commands::Replay { .. }
        );
        if routes_locally_or_remotely {
            let (eff_mode, src) = file_cfg.effective_mode(cli.mode.as_deref());
            if let Some(err) = validate_mode(Some(&eff_mode), url.is_some()) {
                render::error(&format!("{err}（来源: {src}）"));
                return Ok(());
            }
        }
    }

    match command {
        Commands::Chat {
            goal,
            budget,
            acceptance,
            approve_within,
        } => {
            // D-122：Chat 的 `--mode` 不再有独立定义（改顶层 global 单一定义），
            // 故上面那次 `validate_mode(cli.mode…)` 已同时覆盖前置与后置写法——
            // 此处的重复校验随重复声明一并去掉。
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
                    cli.provider.as_deref(),
                    None,
                    cli.api_key.as_deref(),
                    cli.mode.as_deref(),
                    cli.model.as_deref(),
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
                    Ok((sid, _report)) => {
                        // K-2（契约手术）：headless（-p）——stdout 只出答案。
                        // 答案 = 会话最后一条 assistant 文本（render 进度已全走 stderr）。
                        if crate::render::headless() {
                            if let Some(ans) = headless_answer(&sid) {
                                println!("{ans}");
                            }
                        }
                        // D-217：原此处"成本护栏（token 用量）"块**已删除**——用量行现由
                        // **共享路径** `run_local_continue` 末尾打印一次（fresh 与 resume/REPL
                        // 续跑同口径），在此再打会**重复**一次。
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
            // D-102：此前这里**硬编码 "deepseek"**（且完全不传 model）⇒ 用户在远程模式下
            // 给的 `--provider/--model` 被静默忽略（选 openai 也照样按 deepseek 建会话）。
            // 改为走与直跑模式**同一套三级解析**（arg > env > config），使同一个 flag 在
            // 两种模式下解释一致；model 缺省时不下发（服务端用该 provider 的真实模型名）。
            let resolved = file_cfg.resolve(
                cli.provider.as_deref(),
                None,
                cli.api_key.as_deref(),
                cli.mode.as_deref(),
                cli.model.as_deref(),
            );
            let sid = match client
                .create_session(&goal, budget, &resolved.provider, resolved.model.as_deref())
                .await
            {
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
                // D-105（2026-10-02, traecode）：`config get` 的字段集此前与 `config set`
                // **不对称**——set 支持 model / read-roots，get 却不认（报"未知字段"），
                // 于是"能设的读不回来"。现两者对齐为同一份字段集。
                let read_roots = || match &resolved.read_roots {
                    Some(rs) if !rs.is_empty() => rs.join(","),
                    _ => "(默认：cwd + HOME)".to_string(),
                };
                if field.is_empty() {
                    println!("provider        = {}", resolved.provider);
                    println!(
                        "model           = {}",
                        resolved
                            .model
                            .clone()
                            .unwrap_or_else(|| "(provider 默认)".into())
                    );
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
                    println!("read-roots      = {}", read_roots());
                    // P2-3 (v0.2.4): egress 出网白名单（env 优先合并——安全例外，
                    // 环境变量可强制收紧不被配置文件放宽）。
                    let egress = if resolved.egress_allowlist.is_empty() {
                        "(deny-by-default：全部拒绝)".to_string()
                    } else {
                        resolved.egress_allowlist.join(",")
                    };
                    println!("egress-allowlist = {egress}");
                    // D-201（2026-10-08, traecode）：降级链此前在 `config get` 概览里**缺席**
                    // （`ResolvedConfig.providers` 已生效）⇒ 用户看不到已生效的降级链。
                    println!(
                        "providers       = {}",
                        if resolved.providers.is_empty() {
                            format!("(单通道：{})", resolved.provider)
                        } else {
                            resolved.providers.join(",")
                        }
                    );
                    println!("(文件: {})", path.display());
                } else {
                    let v = match field.as_str() {
                        "provider" => resolved.provider,
                        "model" => resolved
                            .model
                            .clone()
                            .unwrap_or_else(|| "(provider 默认)".into()),
                        "url" => resolved.url.unwrap_or_else(|| "(provider 默认)".into()),
                        "api-key" | "api_key" => mask_key(resolved.api_key.as_deref()),
                        "mode" => resolved.mode,
                        "feedback-prompt" | "feedback_prompt" => {
                            resolved.feedback_prompt.to_string()
                        }
                        "read-roots" | "read_roots" => read_roots(),
                        // D-201（2026-10-08, traecode）：`providers`（降级链）此前 `config get`
                        // 不认（报"未知字段"）——用户看不到已生效的降级链。
                        "providers" => {
                            if resolved.providers.is_empty() {
                                format!("(单通道：{})", resolved.provider)
                            } else {
                                resolved.providers.join(",")
                            }
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
                                "未知字段: {other}——可用: provider/model/url/api-key/mode/feedback-prompt/read-roots/providers/egress-allowlist"
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
            let url = read_line(
                "service URL [http://localhost:3000]: ",
                "http://localhost:3000",
            );
            let key = read_line("API key（回车跳过）: ", "");
            let env_path = std::path::Path::new(".env");
            // D-128（2026-10-02, traecode）：**合并**而非截断重写。
            // 旧实现 `std::fs::write(env_path, out)` 会**清空**用户的 `.env`——
            // 用户照 `.env.example` 配好的 provider key（`AGNES_API_KEY` /
            // `DEEPSEEK_API_KEY` / `HEARTH_PROVIDER`…）被静默删除（实测 3 行 → 1 行），
            // 之后 CLI 只报"未配置 API key"，用户无从知道是自己的 `.env` 被 setup 清了。
            // D-131（2026-10-03）：这里同时是 **无界读入** 与 **静默清空 .env** 两个病灶。
            // 旧写法 `read_to_string(env_path).unwrap_or_default()`：读失败（权限/非 UTF-8）
            // 会退化成空串，随后 `merge_env_assignments("")` 的产出里只剩本次写入项，
            // `fs::write` 一落盘就把用户既有 .env **覆盖成 1~2 行**——这正是 D-128 修掉的
            // 那类数据丢失，只是换成了"读失败"触发面。现改为：读**有界**，且任何失败/超限
            // 都**拒绝改写**（宁可让用户手工处理，也不动他的文件）。
            let existed = env_path.exists();
            let existing = if existed {
                match bounded_io::read_file_text_capped_std(
                    env_path,
                    bounded_io::MAX_CAPTURED_BYTES as u64,
                ) {
                    Ok((text, false)) => text,
                    Ok((_, true)) => {
                        render::error_structured(
                            "拒绝改写 .env（文件过大，读取会被截断）",
                            &format!(
                                "{} 超过 {} 字节上限；继续改写会丢失尾部内容",
                                env_path.display(),
                                bounded_io::MAX_CAPTURED_BYTES
                            ),
                            "请先备份并精简 .env，或手工编辑后再运行 hearth setup",
                        );
                        return Ok(());
                    }
                    Err(e) => {
                        render::error_structured(
                            "拒绝改写 .env（读取失败）",
                            &format!(
                                "{} 读取失败: {e}；继续改写会清空你既有的配置",
                                env_path.display()
                            ),
                            "请检查文件权限/编码；确认无需保留后再手工处理",
                        );
                        return Ok(());
                    }
                }
            } else {
                String::new()
            };
            let mut updates: Vec<(&str, &str)> = vec![("CODEX_URL", url.as_str())];
            if !key.is_empty() {
                updates.push(("CODEX_API_KEY", key.as_str()));
            }
            let (merged, replaced) = config::merge_env_assignments(&existing, &updates);
            std::fs::write(env_path, merged).context("write .env failed")?;
            let preserved = existing
                .lines()
                .filter(|l| {
                    let t = l.trim_start();
                    !t.is_empty()
                        && !t.starts_with('#')
                        && !updates.iter().any(|(k, _)| {
                            t.split_once('=')
                                .map(|(lk, _)| lk.trim() == *k)
                                .unwrap_or(false)
                        })
                })
                .count();
            if existed {
                render::info(&format!(
                    "已更新 {}（就地改写 {replaced} 项，**保留原有其它变量 {preserved} 项**——未触碰你的 provider key）",
                    env_path.display()
                ));
            } else {
                render::info(&format!("已新建 {}", env_path.display()));
            }

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

        Commands::Resume {
            id,
            goal,
            budget,
            approve_within,
        } => {
            // X1-4 (v0.1.6): 本地直跑模式 resume——从落盘 JSONL 重建历史继续
            // （治"窗口废了重开"：前面 N 轮对话不丢，进程重启后无缝续接）。
            if url.is_none() {
                let turns = crate::session_store::load_turns(&id);
                if turns.is_empty() {
                    render::error_structured(
                        "会话不存在或为空",
                        &format!(
                            "未找到本地会话 {id}（{}）",
                            crate::session_store::sessions_dir().display()
                        ),
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
                let mut agent = match run_local::rebuild_agent(&resolved, &id, repl::REPL_BUDGET) {
                    Ok(a) => a,
                    Err(e) => {
                        render::error(&format!("agent 重建失败: {e:#}"));
                        return Ok(());
                    }
                };
                agent.restore_history(turns);
                // PC-5：审批委托注入（与 chat --approve-within session 同语义）。
                // 续跑常在非交互（nohup/ssh）下发起——无委托则首个写操作即被拒。
                match approve_within.as_deref() {
                    Some("session") => {
                        agent.set_approval_policy(agent_core::ApprovalPolicy::DelegateSession);
                        render::info("  🔓 续跑：会话级审批委托已开启（--approve-within session）");
                    }
                    Some(other) => {
                        render::error(&format!(
                            "--approve-within 仅支持 session（收到 {other}）——用法: hearth resume <id> --approve-within session"
                        ));
                        return Ok(());
                    }
                    None => {
                        use std::io::IsTerminal as _;
                        if !std::io::stdin().is_terminal() {
                            agent.set_approval_policy(
                                agent_core::ApprovalPolicy::DenyAllNonInteractive,
                            );
                            render::info(
                                "  🤖 续跑非交互模式——破坏性操作将被结构化拒绝（如为 headless 续跑，请加 --approve-within session）",
                            );
                        }
                    }
                }
                // S8（手术包二）：run 级断点恢复——steps/预算位/产物清单/关键
                // scratch 一并回灌（执行位接续，不止历史）。无断点 → 仅历史
                // 恢复（退化不炸）。
                // PC-2 修复（P0/P1 修复任务书 v1.0）：resume = **重新进入消息
                // 循环继续执行**——①current_goal 随断点恢复（不被 "continue"
                // 覆盖漂移）②steps 接着数（resume_keep_steps）③预算追加
                // （总预算 = 已用 + --budget N，默认默认档）。
                let mut prior_steps: u64 = 0;
                let mut resume_goal: Option<String> = None;
                if let Some(rs) = crate::session_store::load_run_state(&id) {
                    prior_steps = rs.get("steps_used").and_then(|v| v.as_u64()).unwrap_or(0);
                    let artifacts = rs
                        .get("written_files")
                        .and_then(|v| v.as_array())
                        .map(|a| a.len())
                        .unwrap_or(0);
                    // current_goal 优先（PC-2 新增字段）；旧断点退回 original_goal。
                    resume_goal = rs
                        .get("current_goal")
                        .and_then(|v| v.as_str())
                        .filter(|g| !g.is_empty())
                        .map(String::from)
                        .or_else(|| {
                            rs.get("original_goal")
                                .and_then(|v| v.as_str())
                                .map(String::from)
                        });
                    agent.restore_run_state(&rs);
                    render::info(&format!(
                        "  ⏱ 断点执行位已恢复（steps={prior_steps}，产物 {artifacts} 项，预算位/scratch 随附）"
                    ));
                    render::info(&format!(
                        "  💾 断点文件: {}（workspace 镜像 .hearth/runs/{id}.json）",
                        crate::session_store::runs_dir()
                            .join(format!("{id}.json"))
                            .display()
                    ));
                }
                // PC-2：resume 是同一任务的继续——goal 为空（用户显式缺省）时用
                // 断点里的 current_goal（不被 "continue" 覆盖 = 目标不漂移）。
                let msg = if goal.is_empty() {
                    match resume_goal {
                        Some(g) => {
                            render::info(&format!(
                                "  🔁 续跑目标（断点恢复）: {}",
                                g.chars().take(60).collect::<String>()
                            ));
                            g
                        }
                        None => "continue".to_string(),
                    }
                } else {
                    goal
                };
                // PC-2：steps 接着数 + 预算追加（总预算 = 已用 + 追加额度）。
                agent.set_resume_keep_steps(true);
                let extra = budget.unwrap_or(repl::REPL_BUDGET);
                let total_budget = run_local::resume_budget(prior_steps, extra);
                render::info(&format!(
                    "  ▶ 续跑模式：steps 接着数（已用 {prior_steps}），预算追加 {extra} 步（总 {total_budget}）——继续执行中"
                ));
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
                        // D-196：显式解析 constraints/acceptance_criteria——形状不符**留痕**
                        // （不静默置空），并与 revision mismatch 的 stale_note 合并后一并上报。
                        let (constraints, criteria, parse_note) = parse_taskgoal_lists(tg);
                        let merged_note = match (stale_note.take(), parse_note) {
                            (None, None) => None,
                            (a, b) => {
                                Some([a, b].into_iter().flatten().collect::<Vec<_>>().join("; "))
                            }
                        };
                        agent.restore_taskgoal(
                            original.to_string(),
                            constraints,
                            criteria,
                            tg.get("goal_revision")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0),
                            merged_note,
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
                    run_local::run_local_continue(agent, &id, &msg, total_budget, Vec::new())
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
                    // D-208（2026-10-08, traecode）：`blocked`（G0 红线 fail-closed 置态，见
                    // agent-runtime `red_line_terminal_phase`）**也是终态**——CLI 必须与
                    // done/cancelled 同处理，否则会去 resume 一个已被红线拦下的会话。
                    if status == "done" || status == "cancelled" || status == "blocked" {
                        let hint = if status == "blocked" {
                            "该会话触发 G0 红线（越权/安全事实）已被拦下，不可续跑；详情见 observer 报告与事件导出"
                        } else {
                            "重新发起：codex chat \"新目标\""
                        };
                        render::error_structured("会话已结束", &format!("status = {status}"), hint);
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
                // D-102：远程 REPL 此前也把 provider 硬编码成 "deepseek"（见 repl.rs），
                // 同样静默忽略 --provider/--model。这里按同一套三级解析后传入。
                let resolved = file_cfg.resolve(
                    cli.provider.as_deref(),
                    None,
                    cli.api_key.as_deref(),
                    cli.mode.as_deref(),
                    cli.model.as_deref(),
                );
                repl::run(
                    u,
                    api_key,
                    resolved.provider.clone(),
                    resolved.model.clone(),
                )
                .await?;
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

        Commands::Tui {
            goal,
            budget,
            headless,
            interrupt_after_ms,
            auto_approve,
        } => {
            // M2：终端 IDE v1——本地进程内 TUI（不依赖 service / VM）。
            let resolved = file_cfg.resolve(
                cli.provider.as_deref(),
                None,
                cli.api_key.as_deref(),
                cli.mode.as_deref(),
                cli.model.as_deref(),
            );
            if let Some(w) = &resolved.url_warning {
                eprintln!("{w}");
            }
            let session_id = uuid::Uuid::new_v4().to_string();
            let mut agent = run_local::rebuild_agent(&resolved, &session_id, budget)?;
            // headless 无真人：auto-approve 冒烟用 Interactive（由 TUI 自动批准）；
            // 其余 headless 一律 fail-closed，避免审批请求把 smoke 永久挂住。
            agent.set_interactive(!headless || auto_approve);
            if headless && !auto_approve {
                agent.set_approval_policy(agent_core::ApprovalPolicy::DenyAllNonInteractive);
            }
            // 每轮结束落盘（与 REPL/chat 同源：session_store 单一实现）。
            let sid = session_id.clone();
            let persist: hearth_tui::PersistFn =
                std::sync::Arc::new(move |a: &agent_core::AgentLoop| {
                    if let Err(e) = crate::session_store::save_snapshot(&sid, &a.history_turns()) {
                        tracing::warn!(error = %e, "TUI 会话落盘失败（不阻断）");
                    }
                    let next_rev = crate::session_store::load_taskgoal(&sid)
                        .map(|(r, _)| r + 1)
                        .unwrap_or(1);
                    if let Err(e) =
                        crate::session_store::save_taskgoal(&sid, &a.taskgoal_value(), next_rev)
                    {
                        tracing::warn!(error = %e, "TUI taskgoal 落盘失败（不阻断）");
                    }
                });
            let opts = hearth_tui::TuiOptions {
                session_id,
                budget,
                initial_goal: goal,
                persist: Some(persist),
                interrupt_after_ms,
                auto_approve,
            };
            if headless {
                let summary = hearth_tui::run_headless(agent, opts).await?;
                println!("{summary}");
            } else {
                hearth_tui::run(agent, opts).await?;
            }
        }

        Commands::Gui { goal, budget } => {
            // M2-G：原生 GUI——与 TUI 共用 provider/dispatcher/落盘构造，只换渲染层。
            let resolved = file_cfg.resolve(
                cli.provider.as_deref(),
                None,
                cli.api_key.as_deref(),
                cli.mode.as_deref(),
                cli.model.as_deref(),
            );
            if let Some(w) = &resolved.url_warning {
                eprintln!("{w}");
            }
            let session_id = uuid::Uuid::new_v4().to_string();
            let mut agent = run_local::rebuild_agent(&resolved, &session_id, budget)?;
            agent.set_interactive(true);
            let sid = session_id.clone();
            let persist: hearth_tui::PersistFn =
                std::sync::Arc::new(move |a: &agent_core::AgentLoop| {
                    if let Err(e) = crate::session_store::save_snapshot(&sid, &a.history_turns()) {
                        tracing::warn!(error = %e, "GUI 会话落盘失败（不阻断）");
                    }
                    let next_rev = crate::session_store::load_taskgoal(&sid)
                        .map(|(r, _)| r + 1)
                        .unwrap_or(1);
                    if let Err(e) =
                        crate::session_store::save_taskgoal(&sid, &a.taskgoal_value(), next_rev)
                    {
                        tracing::warn!(error = %e, "GUI taskgoal 落盘失败（不阻断）");
                    }
                });
            let export: hearth_gui::ExportFn =
                std::sync::Arc::new(|id| export::export_session(id, None));
            let provider = resolved.provider.clone();
            let model = resolved
                .model
                .clone()
                .unwrap_or_else(|| "(default)".to_string());
            let workspace = std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            hearth_gui::run(
                agent,
                hearth_gui::GuiOptions {
                    session_id,
                    budget,
                    initial_goal: goal,
                    persist: Some(persist),
                    export: Some(export),
                    provider,
                    model,
                    workspace,
                },
            )?;
        }

        Commands::Session { action } => match action {
            SessionAction::Export { id, out } => {
                let path = export::export_session(&id, out.map(std::path::PathBuf::from))?;
                println!("证据已导出 → {}", path.display());
            }
        },

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
                        &format!(
                            "未找到本地会话 {id}（{}）",
                            crate::session_store::sessions_dir().display()
                        ),
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
                        &format!(
                            "未找到本地会话 {id}（{}）",
                            crate::session_store::sessions_dir().display()
                        ),
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
                // D-131（2026-10-03）：有界读入——tarpaulin JSON 含**逐行**覆盖明细，
                // 大仓可达数十 MB；旧实现 `read_to_string` 无上限，一条 `hearth coverage`
                // 就能把整份报告读进内存。超限时明确告知，而不是静默给出一个
                // "没有百分比字段"的假结论。
                let (text, truncated) = bounded_io::read_file_text_capped_std(
                    &path,
                    bounded_io::MAX_CAPTURED_BYTES as u64,
                )?;
                if truncated {
                    render::error_structured(
                        "覆盖报告过大，未解析",
                        &format!(
                            "{} 超过 {} 字节上限（已截断读取）",
                            path.display(),
                            bounded_io::MAX_CAPTURED_BYTES
                        ),
                        "如需查看覆盖率，请用 cargo tarpaulin 的输出或缩小扫描范围",
                    );
                    return Ok(());
                }
                let data: serde_json::Value = serde_json::from_str(&text)?;
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
                Err(e) => return Err(e),
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
                Err(e) => return Err(e),
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
                Err(e) => return Err(e),
            }
        }

        Commands::Replay { id } => {
            // P1-5 (v0.2.4): local 模式——回放本地会话完整消息（JSON）。
            if url.is_none() {
                let turns = crate::session_store::load_turns(&id);
                if turns.is_empty() {
                    render::error_structured(
                        "会话不存在或为空",
                        &format!(
                            "未找到本地会话 {id}（{}）",
                            crate::session_store::sessions_dir().display()
                        ),
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
                .get_json(&format!("/api/v1/sessions/{}/messages", pct_encode(&id)))
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
                    // D-61：全量百分号编码（旧实现只编码空格 → `&`/`#` 可注入参数）。
                    let encoded = pct_encode(&query);
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
                    // D-124（2026-10-02）：此前是 `let _ = post_json(...)` 丢弃 Result
                    // **再无条件**打印 "marked done" —— 请求失败（含 404/500）也照样
                    // 宣告成功；而 4xx/5xx 带 JSON 错误体时，`post_json` 还会把它当
                    // "成功响应"返回（见 `client::json_capped` 的状态码校正）。现改走
                    // 只认状态码、不解析体的写原语 + `?` 上抛：失败 → stderr + 非零退出。
                    // 顺带按 D-61 口径对 id 做百分号编码（id 来自用户，旧实现直拼进路径）。
                    client
                        .post_ok(
                            &format!("/api/v1/workline/nodes/{}", pct_encode(&id)),
                            &serde_json::json!({ "progress": 1.0, "status": "completed" }),
                        )
                        .await?;
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
        // D-60（2026-10-01）：按 **字符** 截断。旧实现 `&k[..4]` / `&k[k.len() - 4..]`
        // 是**字节**切片——key 含非 ASCII（多字节 UTF-8）时切点落在非字符边界 → panic
        // （`hearth config get api-key` 直接崩）。此处改用 `chars()` 计数与截取。
        Some(k) if k.chars().count() > 8 => {
            let head: String = k.chars().take(4).collect();
            let tail: String = k.chars().skip(k.chars().count() - 4).collect();
            format!("{head}…{tail}")
        }
        Some(k) if !k.is_empty() => "****".into(),
        _ => "(未设置)".into(),
    }
}

/// D-61（2026-10-01）：最小百分号编码（RFC 3986 `unreserved` 之外一律编码）。
///
/// 用于把**用户输入**安全地嵌进 URL 的查询值与路径段。旧实现只做
/// `query.replace(' ', "%20")`，`&`/`#`/`=`/`/`/`?` 等会破坏 URL 结构——`&x=y`
/// 可注入额外查询参数、`/` 可跳转路径层级（参数注入 / 路径注入）。
pub(crate) fn pct_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// D-153（2026-10-04, traecode）：SSE `data` 是服务端**扁平信封**——顶层含 `type`，
/// 且变体字段**平铺**在同一层（`token`→`delta`、`phase`→`phase`、`error`→`message`、
/// `reflection`→`verdict`、`done`→`report{…}`；见 `service/src/sse.rs::build_sse`）。
///
/// 取字符串字段时**优先按信封取值**，仅在 `data` 本身就是字符串时回退——**不得**对对象
/// 直接 `as_str()`：修复前 token/phase/error/reflection 都这么取，于是整数为对象 → `None`
/// → **静默不渲染**（实测远程模式不显示模型答案、不显示相位与错误）。
pub(crate) fn sse_str<'a>(data: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    data.get(key)
        .and_then(|v| v.as_str())
        .or_else(|| data.as_str())
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
                    // D-153：按扁平信封取字段（修复前对对象 `as_str()` → 恒 None → 静默不渲染）
                    if let Some(p) = sse_str(&sse.data, "phase") {
                        render::phase(p);
                    }
                }
                "token" => {
                    // D-153：token 载荷在 `delta`（修复前 `as_str()` → 恒 None ⇒ 远程不显示答案）
                    if let Some(d) = sse_str(&sse.data, "delta") {
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
                    // D-153：报告嵌在平铺信封的 `report` 里（修复前在顶层取 steps/ok → 恒 0/false
                    // ⇒ 远程模式把成功任务渲染成 `✗ Done (0 steps)`）。
                    let rep = sse.data.get("report").unwrap_or(&sse.data);
                    let steps = rep.get("steps").and_then(|s| s.as_u64()).unwrap_or(0);
                    let ok = rep.get("ok").and_then(|o| o.as_bool()).unwrap_or(false);
                    render::done(steps, ok);
                }
                "error" => {
                    if let Some(e) = sse_str(&sse.data, "message") {
                        render::error(e);
                    }
                }
                "reflection" => {
                    if let Some(v) = sse_str(&sse.data, "verdict") {
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
                // D-197（2026-10-08, traecode）：补齐远程渲染事件覆盖（承 D-153 同族——
                // "远程渲染点漏改一处"）。本地渲染器 `run_local.rs` 对 `AgentEvent` 是**穷尽
                // match**（编译器保证齐全），而远程按字符串匹配、缺臂即静默落 `_ => {}`。
                // 本处与 `repl.rs` 均补 `goal_changed`/`interaction_resolved`（此前远程丢失：
                // 用户看不到目标修订与交互响应事实）。防复发：`tests/remote_render_event_parity_gate.rs`。
                "goal_changed" => {
                    let revision = sse
                        .data
                        .get("revision")
                        .and_then(|r| r.as_u64())
                        .unwrap_or(0);
                    let new_goal = sse_str(&sse.data, "new_goal").unwrap_or("");
                    render::info(&format!(
                        "  🔁 目标已修订（revision {revision}）：{}——原目标保留为锚点",
                        new_goal.chars().take(60).collect::<String>()
                    ));
                }
                "interaction_resolved" => {
                    let by = sse_str(&sse.data, "by").unwrap_or("");
                    let resolved = sse
                        .data
                        .get("resolved")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    render::info(&format!("↪ 交互已响应 by={by} resolved={resolved}"));
                }
                // D-195：服务端广播被追平 ⇒ 事件被丢。服务端发一条**非终止** `notice`
                // 事件报告缺口；此处**显式提示**（修复前落 `_ => {}` ⇒ 静默——用户看到
                // 被截断/跳字的答案却无任何指示）。可用 Last-Event-ID 重连重放补齐。
                "notice" => {
                    let dropped = sse
                        .data
                        .get("dropped")
                        .and_then(|d| d.as_u64())
                        .unwrap_or(0);
                    render::info(&format!(
                        "⚠ 流式事件丢失 {dropped} 条（服务端广播被追平）；可携 Last-Event-ID 重连重放，或 GET /events 全量取回"
                    ));
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

#[cfg(test)]
mod d153_tests {
    use super::sse_str;
    use serde_json::json;

    /// D-153 回归锁：SSE `data` 是服务端**扁平信封**（顶层 `type` + 变体字段**平铺**）。
    /// 取字段必须按信封形状——修复前对对象直接 `as_str()` 恒 `None`，导致远程模式
    /// **静默不渲染** token/phase/error/reflection（不显示模型答案），且 `done` 在顶层
    /// 取 steps/ok 恒 0/false（把成功任务渲染成 `✗ Done (0 steps)`）。
    #[test]
    fn test_d153_sse_str_reads_flattened_envelope() {
        // 与服务端实际帧一致（见 crates/service/src/sse.rs::build_sse 的扁平信封）
        let token = json!({"schema_version":1,"seq":2,"type":"token","delta":"2"});
        assert_eq!(sse_str(&token, "delta"), Some("2"), "token 载荷在 `delta`");
        assert_eq!(
            token.as_str(),
            None,
            "前置：data 是对象——旧写法 `data.as_str()` 恒 None（= 远程不显示答案的根因）"
        );

        let phase = json!({"seq":1,"type":"phase","phase":"plan"});
        assert_eq!(sse_str(&phase, "phase"), Some("plan"));

        let err = json!({"seq":3,"type":"error","message":"boom"});
        assert_eq!(sse_str(&err, "message"), Some("boom"));

        let refl = json!({"seq":4,"type":"reflection","verdict":"continue"});
        assert_eq!(sse_str(&refl, "verdict"), Some("continue"));

        // 兜底：老式"data 直接是字符串"仍可用（不破坏既有形态）
        assert_eq!(sse_str(&json!("plain"), "delta"), Some("plain"));

        // done：报告嵌在 `report`（旧写法在顶层取 steps/ok 恒 0/false）
        let done = json!({"seq":5,"type":"done","report":{"ok":true,"steps":1}});
        let rep = done.get("report").unwrap();
        assert_eq!(rep.get("ok").and_then(|o| o.as_bool()), Some(true));
        assert_eq!(rep.get("steps").and_then(|s| s.as_u64()), Some(1));
        assert_eq!(
            done.get("steps"),
            None,
            "前置：steps 不在顶层——旧写法恒取 0（= `✗ Done (0 steps)` 的根因）"
        );
    }
}

#[cfg(test)]
mod d6061_tests {
    use super::*;

    /// 先红后绿（D-60）：`mask_key` 必须按**字符**截断。
    /// 旧实现 `&k[..4]` / `&k[k.len() - 4..]` 是字节切片 → 多字节 UTF-8 key
    /// 切在非字符边界即 **panic**（`hearth config get api-key` 直接崩）。
    #[test]
    fn test_d60_mask_key_multibyte_no_panic() {
        // 每个中文 3 字节：`len()=30`（>8）但字符数=10（>8）→ 旧代码 `&k[..4]` 落在字符中
        let k = "密钥密钥密钥密钥密钥"; // 10 个汉字
        assert_eq!(k.chars().count(), 10);
        assert!(k.len() > 8, "前提：字节长度 > 8，旧实现会走切片分支");

        let masked = mask_key(Some(k)); // 旧实现此处 panic: byte index 4 is not a char boundary
        assert!(masked.contains('…'), "应打码：{masked}");
        assert_eq!(masked.chars().count(), 9, "4 头 + 1 省略号 + 4 尾");
        // 首尾各保留 4 个字符
        let first4: String = k.chars().take(4).collect();
        let last4: String = k.chars().skip(6).collect();
        assert_eq!(masked, format!("{first4}…{last4}"));

        // 常规 ASCII 分支不受影响
        assert_eq!(mask_key(Some("sk-abcdefghij")), "sk-a…ghij");
        assert_eq!(mask_key(Some("short")), "****");
        assert_eq!(mask_key(None), "(未设置)");
    }

    /// D-61：URL 编码必须覆盖 `&`/`#`/`=`/`/`/空格` 等会破坏 URL 结构的字符
    /// （旧实现只编码空格 → `?search=a&b=c` 可注入额外参数）。
    #[test]
    fn test_d61_pct_encode_escapes_url_structural_chars() {
        assert_eq!(pct_encode("a b"), "a%20b");
        assert_eq!(pct_encode("a&b"), "a%26b");
        assert_eq!(pct_encode("a=b"), "a%3Db");
        assert_eq!(pct_encode("a#b"), "a%23b");
        assert_eq!(pct_encode("a/b"), "a%2Fb");
        assert_eq!(pct_encode("a?b"), "a%3Fb");
        // unreserved 保持原样
        assert_eq!(pct_encode("Az0-._~"), "Az0-._~");
        // 非 ASCII 逐字节编码（中文 = 3 字节）
        assert_eq!(pct_encode("中"), "%E4%B8%AD");
    }

    /// D-102 回归锁：`--mode` 必须**有真实语义**，不得静默忽略。
    ///
    /// 修复前：`mode` 被解析进 `ResolvedConfig.mode` 后全仓无人读取——`--mode remote`
    /// 不给 `--url` 时仍会本地直跑（用户以为连的是远程）。此锁钉住三件事：
    /// ① `auto`（默认）永远放行（= 既有按 `--url` 判定的行为不变）；
    /// ② `remote` **没有** `--url` 时必须报错；
    /// ③ 未知取值必须报错（而不是静默忽略用户输入）。
    #[test]
    fn test_d102_validate_mode_has_real_semantics() {
        assert!(
            validate_mode(None, false).is_none(),
            "默认 auto + 无 url = 本地直跑，放行"
        );
        assert!(validate_mode(Some("auto"), false).is_none());
        assert!(validate_mode(Some("auto"), true).is_none());
        assert!(
            validate_mode(Some("remote"), true).is_none(),
            "remote + 有 url = 远程"
        );

        let e = validate_mode(Some("remote"), false).expect("remote 无 url 必须报错");
        assert!(e.contains("--url"), "错误信息要给出可操作提示：{e}");

        let e = validate_mode(Some("bogus"), true).expect("未知取值必须报错");
        assert!(e.contains("bogus"), "错误信息要含用户输入：{e}");
    }

    /// D-122 回归锁：顶层参数必须是 **global**（前后置写法都认），且**不得重复声明**。
    ///
    /// 红侧（修复前实测）：`hearth chat "目标" --url <service>` 被 clap 直接拒绝——
    /// `error: unexpected argument '--url' found / Usage: hearth.exe chat <GOAL>`；
    /// 而 CLI 自己在**十几处**错误提示里推荐的就是这条后置写法（例如
    /// `lib.rs` 里反复出现的 "下一步: hearth chat \"目标\" --url http://localhost:3000"），
    /// 用户照抄必错。真因是参数被**声明了两套**：`--provider/--model/--mode/--api-key`
    /// 在 `Chat` 变体里另有一份（只认后置），顶层那套未标 `global`（只认前置）。
    ///
    /// 本锁钉三件事：① 后置写法被接受且值真的落进顶层字段；② 前置写法零回归；
    /// ③ `Cli::command().debug_assert()` —— clap 自带的重复/无效声明检查，
    /// 防"再给某个子命令补一份同名参数"把本缺陷重新种回去。
    #[test]
    fn test_d122_top_level_flags_are_global_and_unduplicated() {
        use clap::{CommandFactory as _, Parser as _};

        let c = Cli::try_parse_from([
            "hearth",
            "chat",
            "hi",
            "--url",
            "http://h:1",
            "--provider",
            "agnes",
            "--model",
            "m",
            "--api-key",
            "k",
            "--mode",
            "remote",
            "--budget",
            "7",
        ])
        .expect("后置写法必须被接受（修复前：unexpected argument '--url'）");
        assert_eq!(c.url.as_deref(), Some("http://h:1"), "url 必须落进顶层字段");
        assert_eq!(c.provider.as_deref(), Some("agnes"));
        assert_eq!(c.model.as_deref(), Some("m"));
        // 关键：远程客户端只用**顶层** api_key（`client` 在 match 之前构造）
        // ⇒ 后置 `--api-key` 必须落在这里，否则"给了 key 仍 401"。
        assert_eq!(c.api_key.as_deref(), Some("k"));
        assert_eq!(c.mode.as_deref(), Some("remote"));

        // ② 前置写法零回归（`hearth --url … <子命令>`）。
        let c2 = Cli::try_parse_from(["hearth", "--url", "http://h:2", "sessions"])
            .expect("前置写法必须保持可用");
        assert_eq!(c2.url.as_deref(), Some("http://h:2"));

        // ③ 无重复/无效声明（同参数声明两套会让 clap 的 debug 断言炸）。
        Cli::command().debug_assert();
    }
}

#[cfg(test)]
mod d196_tests {
    use super::*;

    /// D-196（2026-10-08, traecode）：恢复路径的字段解析——**形状不符必须留痕**，不静默置空。
    #[test]
    fn test_d196_parse_taskgoal_lists_notes_shape_mismatch() {
        // ① 正常形状：原样解析，无 note。
        let ok = serde_json::json!({
            "constraints": ["a", "b"],
            "acceptance_criteria": ["file: out.txt nonempty"]
        });
        let (c, a, note) = parse_taskgoal_lists(&ok);
        assert_eq!(c, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(a, vec!["file: out.txt nonempty".to_string()]);
        assert!(note.is_none(), "正常形状不得有 note：{note:?}");

        // ② 字段**缺失**（旧会话）→ 空列表、**无** note（合法，无恢复语义）。
        let (c, a, note) = parse_taskgoal_lists(&serde_json::json!({}));
        assert!(c.is_empty() && a.is_empty());
        assert!(note.is_none(), "字段缺失是合法情形，不应报 note：{note:?}");

        // ③ 字段**存在但形状不符** → 空列表 **+ note**（修复前：`.ok().unwrap_or_default()`
        //    静默置空、无任何留痕，用户仍被告知"任务目标已恢复"）。
        let bad = serde_json::json!({
            "constraints": "should-be-an-array",
            "acceptance_criteria": {"not": "an array"}
        });
        let (c, a, note) = parse_taskgoal_lists(&bad);
        assert!(c.is_empty() && a.is_empty());
        let note = note.expect("形状不符必须留痕（不得静默置空）");
        assert!(note.contains("constraints"), "note 须点名字段：{note}");
        assert!(
            note.contains("acceptance_criteria"),
            "note 须点名字段：{note}"
        );

        // ④ 一个字段好、一个字段坏 → 好的**保留**、坏的置空并留痕（不因一坏全丢）。
        let half = serde_json::json!({
            "constraints": ["keep"],
            "acceptance_criteria": 42
        });
        let (c, a, note) = parse_taskgoal_lists(&half);
        assert_eq!(c, vec!["keep".to_string()], "可用字段必须保留");
        assert!(a.is_empty());
        assert!(
            note.expect("坏字段必须留痕")
                .contains("acceptance_criteria"),
            "note 须点名坏字段"
        );
    }
}
