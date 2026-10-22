//! hearth-tui：终端 IDE v1（M2）。
//!
//! **职责边界**：本 crate 只做「事件 → 界面状态 → 终端渲染 + 键盘」，
//! 不构造 provider / dispatcher / 会话落盘——那些由 `codex-cli` 用**既有**
//! 构造路径（`build_provider` / `build_dispatcher` / `session_store`）组装后
//! 传入（避免 D-224「同能力多处各写一份」与反向依赖环）。
//!
//! 架构：单进程、无 service、无 VM。`AgentLoop::run_take` 在后台 task 里跑，
//! 事件经 `set_event_sender` 的 unbounded channel 进入 UI；`Esc`/`Ctrl-C` 走
//! S11 优雅打断（置位 + notify），agent 收尾后把本体交还，历史不丢。

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use agent_core::{AgentLoop, Event};
use crossterm::event::{self, Event as TermEvent, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};

pub mod ui;

/// 一轮结束后由 CLI 侧持久化（历史 / taskgoal）。返回错误不阻断 TUI。
pub type PersistFn = Arc<dyn Fn(&AgentLoop) + Send + Sync>;

pub struct TuiOptions {
    pub session_id: String,
    pub budget: u64,
    /// 启动后自动提交的目标（`hearth tui --goal "..."`）。
    pub initial_goal: Option<String>,
    pub persist: Option<PersistFn>,
    /// 仅 headless 冒烟：N 毫秒后自动触发优雅中断（验证 Esc 路径 + 落盘 + resume）。
    pub interrupt_after_ms: Option<u64>,
}

/// 界面可见的一行。`kind` 只影响配色，不参与逻辑。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineKind {
    User,
    Assistant,
    Tool,
    ToolOk,
    ToolErr,
    System,
    Approval,
}

#[derive(Debug, Clone)]
pub struct TranscriptLine {
    pub kind: LineKind,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingApproval {
    pub id: String,
    pub action: String,
    /// 内核 payload 的 `tools`（M2-4：要改什么）；旧内核无此字段 ⇒ 空。
    pub tools: Vec<String>,
}

#[derive(Debug)]
pub struct TuiState {
    pub session_short: String,
    pub lines: Vec<TranscriptLine>,
    pub input: String,
    pub phase: String,
    pub steps: u64,
    pub running: bool,
    pub pending_approval: Option<PendingApproval>,
    pub done_line: Option<String>,
    pub quit: bool,
    pub scroll_from_bottom: u16,
    /// 只用于测试/诊断：收到的 Done 载荷。
    pub last_done: Option<serde_json::Value>,
}

impl TuiState {
    pub fn new(session_id: &str) -> Self {
        let short = session_id.chars().take(8).collect::<String>();
        Self {
            session_short: short,
            lines: Vec::new(),
            input: String::new(),
            phase: "idle".into(),
            steps: 0,
            running: false,
            pending_approval: None,
            done_line: None,
            quit: false,
            scroll_from_bottom: 0,
            last_done: None,
        }
    }

    fn push(&mut self, kind: LineKind, text: String) {
        self.lines.push(TranscriptLine { kind, text });
        // 有界：TUI 长跑不得让内存随输出无界增长（只保留最近 2000 行）。
        const MAX_LINES: usize = 2000;
        if self.lines.len() > MAX_LINES {
            let drop_n = self.lines.len() - MAX_LINES;
            self.lines.drain(0..drop_n);
        }
    }

    /// 事件摄入——**唯一**状态入口（纯逻辑，可单测；不碰终端）。
    pub fn ingest(&mut self, evt: &Event) {
        match evt {
            Event::Token(t) => {
                if let Some(last) = self.lines.last_mut() {
                    if last.kind == LineKind::Assistant {
                        last.text.push_str(t);
                        return;
                    }
                }
                self.push(LineKind::Assistant, t.clone());
            }
            Event::ToolCall(tc) => {
                let args = summarize_args(&tc.args);
                self.push(LineKind::Tool, format!("→ {} {}", tc.name, args));
            }
            Event::ToolResult(tr) => {
                let first = first_line(&tr.output, 220);
                if tr.is_error {
                    self.push(
                        LineKind::ToolErr,
                        format!("✗ {}: {}", short_id(&tr.call_id), first),
                    );
                } else {
                    self.push(
                        LineKind::ToolOk,
                        format!("✓ {}: {}", short_id(&tr.call_id), first),
                    );
                }
            }
            Event::ThinkSummary { phase, text } => {
                self.push(LineKind::System, format!("[{phase}] {text}"));
            }
            Event::Phase(p) => self.phase = p.clone(),
            Event::Artifact {
                path, size_bytes, ..
            } => {
                self.push(LineKind::System, format!("📄 {path} ({size_bytes} B)"));
            }
            Event::InteractionRequested {
                id, kind, payload, ..
            } => {
                if kind == "approval" {
                    let action = payload
                        .get("action")
                        .and_then(|v| v.as_str())
                        .unwrap_or("approval requested")
                        .to_string();
                    let mut tools = payload
                        .get("tools")
                        .and_then(|v| v.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|x| x.as_str().map(str::to_string))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    // 内核旧载荷只有 action（"execute bash tool"）——把**紧邻的**
                    // 工具调用行（含参数）补进面板，满足 M2-4"要改什么"。
                    if tools.is_empty() {
                        if let Some(last) =
                            self.lines.iter().rev().find(|l| l.kind == LineKind::Tool)
                        {
                            tools.push(last.text.clone());
                        }
                    }
                    self.pending_approval = Some(PendingApproval {
                        id: id.clone(),
                        action: action.clone(),
                        tools: tools.clone(),
                    });
                    let detail = if tools.is_empty() {
                        action.clone()
                    } else {
                        format!("{action} — {}", tools.join(" · "))
                    };
                    self.push(LineKind::Approval, format!("⛔ 需要审批：{detail}"));
                } else {
                    self.push(LineKind::System, format!("[interaction:{kind}] {id}"));
                }
            }
            Event::Done(v) => {
                let status = v.get("status").and_then(|s| s.as_str()).unwrap_or("done");
                let steps = v
                    .get("steps")
                    .and_then(|s| s.as_u64())
                    .unwrap_or(self.steps);
                self.steps = steps;
                self.phase = status.to_string();
                self.running = false;
                let line = format!("⏹ {status} · {steps} 步");
                self.done_line = Some(line.clone());
                self.push(LineKind::System, line);
                self.last_done = Some(v.clone());
            }
            Event::Error(e) => {
                self.push(LineKind::System, format!("⚠ {e}"));
            }
            Event::GoalChanged { new_goal, .. } => {
                self.push(LineKind::System, format!("↻ 目标更新：{new_goal}"));
            }
            Event::PlanDraft { .. } => {}
            Event::SpanOpen { name, .. } => self.phase = name.clone(),
            Event::SpanClose { .. } => {}
        }
    }

    /// 测试/审计用：把可见行拼成文本。
    pub fn visible_text(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

fn first_line(s: &str, cap: usize) -> String {
    let line = s.lines().next().unwrap_or("");
    let mut out: String = line.chars().take(cap).collect();
    if line.chars().count() > cap {
        out.push('…');
    }
    out
}

fn summarize_args(v: &serde_json::Value) -> String {
    let s = v.to_string();
    let mut out: String = s.chars().take(120).collect();
    if s.chars().count() > 120 {
        out.push('…');
    }
    out
}

/// 终端恢复守卫——任何退出路径（含 `?` 提前返回）都恢复 raw mode / 主屏。
struct TermGuard;

impl Drop for TermGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
    }
}

/// 运行 TUI（阻塞到用户退出）。
pub async fn run(mut agent: AgentLoop, opts: TuiOptions) -> anyhow::Result<()> {
    use tokio::sync::mpsc::unbounded_channel;

    let dispatcher = agent.scheduler_arc();
    let interrupt_flag = agent.interrupt_handle();
    let interrupt_notify = agent.interrupt_notify();
    let (tx, mut rx) = unbounded_channel::<Event>();
    agent.set_event_sender(tx);

    let session_id = opts.session_id.clone();
    // 所有权槽：run_take 消费 agent，收尾后把本体交还；中断/异常时槽为空
    // （历史退化到上一轮快照——与 CLI 的 S11 语义一致，不假装"没丢"）。
    let mut agent_slot: Option<AgentLoop> = Some(agent);
    let mut state = TuiState::new(&session_id);
    state.push(
        LineKind::System,
        format!(
            "Hearth TUI · session {} · /quit 退出 · Esc 中断 · 输入任务后 Enter 发送",
            state.session_short
        ),
    );

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let _guard = TermGuard;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;
    terminal.clear()?;

    let mut handle: Option<
        tokio::task::JoinHandle<anyhow::Result<(agent_core::RunReport, AgentLoop)>>,
    > = None;

    // 初始目标（`hearth tui --goal`）在进入循环前提交。
    if let Some(g) = opts.initial_goal.clone() {
        state.push(LineKind::User, format!("› {g}"));
        state.running = true;
        submit(&mut agent_slot, &mut state, g, opts.budget, &mut handle);
    }

    loop {
        terminal.draw(|f| ui::draw(f, &state))?;

        // 事件优先：把 agent 已产出的事件全部摄入（不阻塞界面）。
        while let Ok(evt) = rx.try_recv() {
            state.ingest(&evt);
        }

        // Done 已到 ⇒ 等后台 task 交还 agent（通常已就绪），落盘后回到可输入态。
        if !state.running && handle.is_some() {
            if let Some(h) = handle.take() {
                match h.await {
                    Ok(Ok((report, back))) => {
                        state.steps = report.steps;
                        if let Some(p) = &opts.persist {
                            p(&back);
                        }
                        agent_slot = Some(back);
                    }
                    Ok(Err(e)) => state.push(LineKind::System, format!("⚠ run 失败：{e:#}")),
                    Err(e) => state.push(LineKind::System, format!("⚠ run 任务异常：{e}")),
                }
            }
        }

        if state.quit {
            break;
        }

        // 键盘：非阻塞轮询（30ms 内无事件就回去重绘/收事件）。
        if !event::poll(Duration::from_millis(30))? {
            continue;
        }
        let TermEvent::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        // 全局退出：Ctrl-Q（Esc/Ctrl-C 是"中断"，不是退出——M2-3 语义）。
        if ctrl && matches!(key.code, KeyCode::Char('q')) {
            state.quit = true;
            continue;
        }

        // 审批面板优先吃键。
        if let Some(pa) = state.pending_approval.clone() {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    resolve_approval(&dispatcher, &session_id, &pa.id, true).await;
                    state.pending_approval = None;
                    state.push(LineKind::System, "✔ 已批准".into());
                    continue;
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    resolve_approval(&dispatcher, &session_id, &pa.id, false).await;
                    state.pending_approval = None;
                    state.push(LineKind::System, "✘ 已拒绝".into());
                    continue;
                }
                KeyCode::Esc => {
                    resolve_approval(&dispatcher, &session_id, &pa.id, false).await;
                    state.pending_approval = None;
                    state.push(LineKind::System, "✘ 已拒绝（Esc）".into());
                    continue;
                }
                _ => continue,
            }
        }

        // 运行中：Esc / Ctrl-C = 优雅中断（不丢历史）。
        if state.running {
            if matches!(key.code, KeyCode::Esc) || (ctrl && matches!(key.code, KeyCode::Char('c')))
            {
                interrupt_flag.store(true, Ordering::SeqCst);
                interrupt_notify.notify_waiters();
                state.push(LineKind::System, "⏸ 已请求中断（等待本步收尾…）".into());
            }
            continue;
        }

        match key.code {
            KeyCode::Esc => state.input.clear(),
            KeyCode::Enter => {
                let goal = state.input.trim().to_string();
                if goal.is_empty() {
                    continue;
                }
                if matches!(goal.as_str(), "/quit" | "/q" | "/exit") {
                    state.quit = true;
                    continue;
                }
                state.input.clear();
                state.push(LineKind::User, format!("› {goal}"));
                state.running = true;
                state.done_line = None;
                submit(&mut agent_slot, &mut state, goal, opts.budget, &mut handle);
            }
            KeyCode::Backspace => {
                state.input.pop();
            }
            KeyCode::Up => {
                state.scroll_from_bottom = state.scroll_from_bottom.saturating_add(3);
            }
            KeyCode::Down => {
                state.scroll_from_bottom = state.scroll_from_bottom.saturating_sub(3);
            }
            KeyCode::Char(c) if !ctrl => state.input.push(c),
            _ => {}
        }
    }

    // 退出前：若仍在跑，先请求中断，再等 ≤5s（避免把后台 task 留在半途）。
    if state.running {
        interrupt_flag.store(true, Ordering::SeqCst);
        interrupt_notify.notify_waiters();
    }
    if let Some(h) = handle {
        let _ = tokio::time::timeout(Duration::from_secs(5), h).await;
    }
    Ok(())
}

fn submit(
    agent_slot: &mut Option<AgentLoop>,
    state: &mut TuiState,
    goal_text: String,
    budget: u64,
    handle: &mut Option<
        tokio::task::JoinHandle<anyhow::Result<(agent_core::RunReport, AgentLoop)>>,
    >,
) {
    let Some(moved) = agent_slot.take() else {
        state.running = false;
        state.push(
            LineKind::System,
            "⚠ agent 不可用（上一轮异常退出）——请 /quit 后用 `hearth resume <id>` 恢复".into(),
        );
        return;
    };
    let goal = agent_core::Goal::with_budget(
        goal_text,
        agent_types::Budget {
            max_steps: budget,
            ..Default::default()
        },
    );
    let task = tokio::spawn(async move { moved.run_take(goal).await });
    *handle = Some(task);
}

/// headless 冒烟结果（无 TTY 环境下的 E2E：事件管线 + agent 接线，不含渲染）。
#[derive(Debug, Clone)]
pub struct TuiSummary {
    pub session: String,
    pub status: String,
    pub steps: u64,
    pub lines: usize,
    pub tokens_seen: usize,
    pub tool_calls: usize,
    pub done: bool,
}

impl std::fmt::Display for TuiSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TUI-SMOKE session={} status={} steps={} lines={} tokens={} tool_calls={} done={}",
            self.session,
            self.status,
            self.steps,
            self.lines,
            self.tokens_seen,
            self.tool_calls,
            self.done
        )
    }
}

/// 无终端冒烟：跑一轮真实 agent（provider 由 CLI 决定），把事件摄入同一 `TuiState`。
/// 这是 `hearth tui --headless --goal ...` 的实现——CI/无 TTY 环境可验证接线。
pub async fn run_headless(mut agent: AgentLoop, opts: TuiOptions) -> anyhow::Result<TuiSummary> {
    use tokio::sync::mpsc::unbounded_channel;

    // 中断句柄必须在 agent move 进 task 之前取（与交互路径同一机制）。
    let interrupt_flag = agent.interrupt_handle();
    let interrupt_notify = agent.interrupt_notify();
    if let Some(ms) = opts.interrupt_after_ms {
        let flag = interrupt_flag.clone();
        let notify = interrupt_notify.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(ms)).await;
            flag.store(true, Ordering::SeqCst);
            notify.notify_waiters();
        });
    }

    let goal_text = opts
        .initial_goal
        .clone()
        .filter(|g| !g.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("--headless 需要 --goal"))?;
    let (tx, mut rx) = unbounded_channel::<Event>();
    agent.set_event_sender(tx);
    let goal = agent_core::Goal::with_budget(
        goal_text,
        agent_types::Budget {
            max_steps: opts.budget,
            ..Default::default()
        },
    );
    let task = tokio::spawn(async move { agent.run_take(goal).await });

    let mut state = TuiState::new(&opts.session_id);
    let mut tokens_seen = 0usize;
    let mut tool_calls = 0usize;
    while let Some(evt) = rx.recv().await {
        match &evt {
            Event::Token(_) => tokens_seen += 1,
            Event::ToolCall(_) => tool_calls += 1,
            _ => {}
        }
        let done = matches!(evt, Event::Done(_));
        state.ingest(&evt);
        if done {
            break;
        }
    }
    let (report, back) = task.await??;
    if let Some(p) = &opts.persist {
        p(&back);
    }
    Ok(TuiSummary {
        session: opts.session_id.clone(),
        status: state.phase.clone(),
        steps: report.steps,
        lines: state.lines.len(),
        tokens_seen,
        tool_calls,
        done: state.last_done.is_some(),
    })
}

async fn resolve_approval(
    dispatcher: &Arc<tool_runtime::ToolDispatcher>,
    session_id: &str,
    interaction_id: &str,
    approved: bool,
) {
    let answer = if approved { "approve" } else { "deny" };
    let _ = dispatcher
        .resolve_interaction(
            session_id,
            interaction_id,
            approved,
            serde_json::json!({ "answer": answer }),
        )
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_types::{ToolCall, ToolResult};

    #[test]
    fn tokens_accumulate_into_one_assistant_line() {
        let mut st = TuiState::new("1234567890abcdef");
        st.ingest(&Event::Token("hel".into()));
        st.ingest(&Event::Token("lo".into()));
        assert_eq!(st.visible_text().matches("hello").count(), 1);
        assert_eq!(
            st.lines
                .iter()
                .filter(|l| l.kind == LineKind::Assistant)
                .count(),
            1
        );
    }

    #[test]
    fn tool_call_result_render_ok_and_err() {
        let mut st = TuiState::new("s");
        st.ingest(&Event::ToolCall(ToolCall {
            call_id: "c1".into(),
            name: "bash".into(),
            args: serde_json::json!({"cmd": "cargo test"}),
        }));
        st.ingest(&Event::ToolResult(ToolResult {
            call_id: "c1".into(),
            is_error: true,
            output: "exit code: 1\nboom".into(),
            artifacts: vec![],
            error_kind: None,
        }));
        let text = st.visible_text();
        assert!(text.contains("→ bash"), "{text}");
        assert!(text.contains("✗ c1: exit code: 1"), "{text}");
    }

    #[test]
    fn approval_event_opens_panel_with_tools() {
        let mut st = TuiState::new("s");
        // 旧内核载荷没有 tools —— 必须回退到紧邻的工具调用行（含参数）。
        st.ingest(&Event::ToolCall(ToolCall {
            call_id: "c9".into(),
            name: "bash".into(),
            args: serde_json::json!({"cmd": "rm -rf ./build"}),
        }));
        st.ingest(&Event::InteractionRequested {
            id: "a0".into(),
            kind: "approval".into(),
            blocking: true,
            timeout: None,
            on_timeout: Some("abort".into()),
            payload: serde_json::json!({"action": "execute bash tool"}),
        });
        let pa = st.pending_approval.clone().expect("必须有待审批");
        assert!(
            pa.tools.iter().any(|t| t.contains("rm -rf ./build")),
            "回退必须带上参数：{:?}",
            pa.tools
        );
        st.pending_approval = None;
        st.ingest(&Event::InteractionRequested {
            id: "a1".into(),
            kind: "approval".into(),
            blocking: true,
            timeout: None,
            on_timeout: Some("abort".into()),
            payload: serde_json::json!({
                "action": "execute bash tool",
                "tools": ["bash: rm -rf ./build"],
            }),
        });
        let pa = st.pending_approval.clone().expect("必须有待审批");
        assert_eq!(pa.id, "a1");
        assert_eq!(pa.tools, vec!["bash: rm -rf ./build".to_string()]);
        assert!(st.visible_text().contains("需要审批"));
    }

    #[test]
    fn done_updates_phase_steps_and_running() {
        let mut st = TuiState::new("s");
        st.running = true;
        st.ingest(&Event::Done(serde_json::json!({
            "status": "completed", "steps": 7,
        })));
        assert_eq!(st.phase, "completed");
        assert_eq!(st.steps, 7);
        assert!(!st.running);
        assert!(st.done_line.is_some());
    }

    #[test]
    fn transcript_is_bounded() {
        let mut st = TuiState::new("s");
        for i in 0..2500 {
            st.ingest(&Event::Error(format!("e{i}")));
        }
        assert_eq!(st.lines.len(), 2000, "长跑不得让内存无界增长");
        assert!(st.visible_text().contains("e2499"));
    }
}
