use std::cell::Cell;
use std::io::{self, BufRead, BufReader};
use std::panic;
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, EnableMouseCapture, DisableMouseCapture, Event, KeyCode, KeyEventKind,
             KeyModifiers, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
    Frame, Terminal,
};

const HEARTH_BIN: &str = "/home/wutao/hearth-slim/target/release/hearth";
/// Step 5: hearth runs in this cwd so `.hearth/reports/<uuid>` lands here.
const WORKDIR: &str = "/home/wutao/hearth-tui-new";

// ---- Fix 3: only 2 color hues across the whole UI ----
// Structure (borders/titles): DarkGray
// Accent (status key values, running indicator): Cyan
const C_STRUCT: Color = Color::DarkGray;
const C_ACCENT: Color = Color::Cyan;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    You,
    Answer,
    Proc,
    Warn,
}

enum UiMsg {
    Out(String),
    Err(String),
    Done(u64),
}

/// Extract the last process-line snippet to show as "current action" in the
/// status bar while running.  Returns a short label (≤ 24 chars) or None.
fn last_tool_action(convo: &[(Kind, String)]) -> Option<String> {
    let last_proc = convo
        .iter()
        .rev()
        .find(|(k, _)| *k == Kind::Proc)
        .map(|(_, t)| t.clone());

    let text = last_proc?;
    // process lines are stored raw (no "· " prefix in data, that's render-only)
    let clean = text.trim();
    let truncated: String = clean.chars().take(24).collect();
    if truncated.is_empty() {
        None
    } else {
        Some(truncated)
    }
}

/// Step 5: newest dir under `.hearth/reports/` == current session UUID.
fn latest_report_uuid() -> Option<String> {
    let dir = format!("{WORKDIR}/.hearth/reports");
    let mut best: Option<(String, Option<std::time::SystemTime>)> = None;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                let mt = e.metadata().ok().and_then(|m| m.modified().ok());
                match &mut best {
                    None => best = Some((p.file_name().unwrap_or_default().to_string_lossy().into_owned(), mt)),
                    Some((_, bmt)) => {
                        if mt > *bmt {
                            best = Some((p.file_name().unwrap_or_default().to_string_lossy().into_owned(), mt));
                        }
                    }
                }
            }
        }
    }
    best.map(|(n, _)| n)
}

/// T1 (polish3): find the rendered-line index of the first answer-marker line
/// (containing ◂ span or ✓ Done) and the total rendered line count.
/// Mirrors the render logic in draw() so the main loop can compute a scroll target
/// without duplicating the full draw path.
fn find_answer_rendered_index(convo: &[(Kind, String)], fold: bool) -> (Option<usize>, usize) {
    let mut total: usize = 0;
    let mut answer_idx: Option<usize> = None;
    for (kind, text) in convo {
        if *kind == Kind::Proc && fold {
            continue;
        }
        let is_marker = text.contains("◂ span") || text.contains("✓ Done");
        let n_lines = text.lines().count().max(1);
        if is_marker && answer_idx.is_none() {
            answer_idx = Some(total);
        }
        total += n_lines;
    }
    // Hint line (added in draw() when convo is short and not running)
    if !convo.is_empty() && convo.len() <= 4 {
        total += 1;
    }
    (answer_idx, total)
}

/// Spawn `hearth <args>` (chat / resume) in a background thread; stream
/// stdout/stderr lines to the UI thread via a channel. Never blocks the UI.
fn spawn_hearth(args: Vec<String>, tx: Sender<UiMsg>) {
    thread::spawn(move || {
        let start = Instant::now();
        let mut child = match Command::new(HEARTH_BIN)
            .args(&args)
            .current_dir(WORKDIR)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(UiMsg::Err(format!("spawn failed: {e}")));
                let _ = tx.send(UiMsg::Done(0));
                return;
            }
        };

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let tx_out = tx.clone();
        let t_out = thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(l) = line {
                    let _ = tx_out.send(UiMsg::Out(l));
                }
            }
        });
        let tx_err = tx.clone();
        let t_err = thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(l) = line {
                    let _ = tx_err.send(UiMsg::Err(l));
                }
            }
        });

        // D11 fix: wait for both reader threads to hit EOF (all output consumed),
        // then immediately signal Done so the status bar clears ≤1s after last line.
        // child.wait() is deferred — process may linger briefly but UI already moved on.
        let _ = t_out.join();
        let _ = t_err.join();
        let elapsed = start.elapsed().as_secs();
        let _ = tx.send(UiMsg::Done(elapsed));
        // Let the child finish in the background (avoids blocking UI on slow cleanup).
        let _ = child.wait();
    });
}

fn main() -> io::Result<()> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    // T3: panic hook — restore terminal (mouse capture, raw mode, alt screen) on panic
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = execute!(std::io::stdout(), DisableMouseCapture);
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut input = String::new();
    let mut quit = false;
    // ---- Step 4 state ----
    const HIST_MAX: usize = 20;
    let mut history: Vec<String> = Vec::new();
    let mut sent: usize = 0;
    let max_off_cell = Cell::new(0usize);
    let mut hist_idx: Option<usize> = None;
    let mut scroll_offset: usize = 0;
    let mut pinned_bottom = true;
    let mut fold = false;
    let mut in_summary_block = false; // Fix6: tracks whether we're inside a ═══-delimited 任务总结 block
    let mut pending_auto_scroll = false; // T1 polish3: one-shot auto-scroll to answer on Done

    let mut convo: Vec<(Kind, String)> = Vec::new();

    let (tx, rx): (Sender<UiMsg>, Receiver<UiMsg>) = channel();
    let mut running = false;
    let mut req_start: Option<Instant> = None;
    let mut last_secs: Option<u64> = None;
    let mut spin: u8 = 0;
    let mut session_uuid: Option<String> = None;

    while !quit {
        // Drain all pending messages from the worker threads (non-blocking).
        let mut got_msg = false;
        loop {
            match rx.try_recv() {
                Ok(UiMsg::Out(l)) if !l.is_empty() => {
                    let trimmed = l.trim_start();
                    // Fix6: lines carrying the answer must never be folded
                    let has_answer = trimmed.contains("◂ span") || trimmed.contains("✓ Done");
                    // T3 (polish3): explicit open/close pairing instead of odd/even toggle.
                    //  Opening marker: ═══ line containing "任务总结" → in_summary_block = true
                    //  Closing marker: ═══ line NOT containing "任务总结" → in_summary_block = false
                    //  An orphan ═══ (no "任务总结") safely closes / no-ops the block.
                    let is_summary_delim = trimmed.starts_with('\u{2550}');
                    let is_proc;
                    if has_answer {
                        is_proc = false; // always Answer, never folded
                    } else if is_summary_delim {
                        let is_open = trimmed.contains("任务总结");
                        in_summary_block = is_open;
                        is_proc = true; // the ═══ delimiter line itself is Proc
                    } else if in_summary_block {
                        is_proc = true; // inside summary block → fold
                    } else {
                        is_proc = trimmed.starts_with("──")
                            || trimmed.starts_with("📄 ")
                            || trimmed.starts_with("✓ ")
                            || trimmed.starts_with("─────")
                            || trimmed.starts_with("· ")
                            || l.contains("▸ span");
                    }
                    let is_confirm =
                        l.contains("需要你确认") || l.contains("❓");
                    let kind = if is_proc { Kind::Proc } else { Kind::Answer };
                    convo.push((kind, l.clone()));
                    if is_confirm {
                        convo.push((
                            Kind::Warn,
                            "⚠ 该请求需要人工确认，当前 TUI 不支持交互回复 —— 请在终端直接运行 hearth chat 处理".to_string(),
                        ));
                    }
                    got_msg = true;
                }
                Ok(UiMsg::Err(l)) if !l.is_empty() => {
                    convo.push((Kind::Proc, l));
                    got_msg = true;
                }
                Ok(UiMsg::Done(s)) => {
                    running = false;
                    last_secs = Some(s);
                    session_uuid = latest_report_uuid();
                    pending_auto_scroll = true; // T1: will auto-scroll on next loop iteration
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        if got_msg && pinned_bottom {
            scroll_offset = 0;
        }
        // T1 (polish3): on a real turn's Done, bring the first answer-marker line
        // to the top of the viewport, exactly once per turn. Subsequent PgUp/PgDn/
        // wheel are never snapped back (pinned_bottom is released here).
        if pending_auto_scroll && !running {
            pending_auto_scroll = false;
            let (aidx_opt, total) = find_answer_rendered_index(&convo, fold);
            if let Some(aidx) = aidx_opt {
                let term_h = terminal.size().map(|s| s.height).unwrap_or(24);
                // Mirrors draw(): chunks[1] = height - 9, area = chunks[1] - 2.
                let area_height = term_h.saturating_sub(9).saturating_sub(2) as usize;
                let max_off = total.saturating_sub(area_height);
                let desired_off = max_off.saturating_sub(aidx);
                scroll_offset = desired_off.min(max_off);
                pinned_bottom = false; // release bottom-pin so user scroll is free
            }
        }
        if running {
            spin = (spin + 1) % 4;
        }

        terminal.draw(|f| {
            draw(
                f,
                &input,
                &convo,
                running,
                req_start,
                last_secs,
                spin,
                scroll_offset,
                fold,
                session_uuid.as_deref(),
                sent,
                &max_off_cell,
            );
        })?;

        if event::poll(Duration::from_millis(100))? {
            // ---- Fix 2: Mouse events ----
            match event::read()? {
                Event::Mouse(m) => {
                    match m.kind {
                        MouseEventKind::ScrollUp => {
                            pinned_bottom = false;
                            scroll_offset = (scroll_offset + 3).min(max_off_cell.get());
                        }
                        MouseEventKind::ScrollDown => {
                            scroll_offset = scroll_offset.saturating_sub(3);
                            if scroll_offset == 0 {
                                pinned_bottom = true;
                            }
                        }
                        _ => {}
                    }
                }
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => quit = true,
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                quit = true;
                            }
                            KeyCode::Esc => {
                                input.clear();
                                hist_idx = None;
                            }
                            KeyCode::Backspace => {
                                input.pop();
                                hist_idx = None;
                            }
                            KeyCode::Enter => {
                                if !input.is_empty() && !running {
                                    let cmd = input.clone();
                                    input.clear();
                                    hist_idx = None;
                                    if history.len() >= HIST_MAX {
                                        history.remove(0);
                                    }
                                    history.push(cmd.clone());
                                    sent += 1;
                                    convo.push((Kind::You, cmd.clone()));
                                    running = true;
                                    last_secs = None;
                                    req_start = Some(Instant::now());
                                    pinned_bottom = true;
                                    scroll_offset = 0;
                                    let args: Vec<String> = match &session_uuid {
                                        None => vec![
                                            "chat".into(),
                                            cmd.clone(),
                                            "--budget".into(),
                                            "40".into(),
                                            "--approve-within".into(),
                                            "session".into(),
                                        ],
                                        Some(u) => vec![
                                            "resume".into(),
                                            u.clone(),
                                            cmd.clone(),
                                            "--budget".into(),
                                            "30".into(),
                                            "--approve-within".into(),
                                            "session".into(),
                                        ],
                                    };
                                    spawn_hearth(args, tx.clone());
                                }
                            }
                            KeyCode::Up => {
                                let n = history.len();
                                if n > 0 {
                                    let next = match hist_idx {
                                        None => n - 1,
                                        Some(0) => 0,
                                        Some(i) => i - 1,
                                    };
                                    hist_idx = Some(next);
                                    input = history[next].clone();
                                }
                            }
                            KeyCode::Down => {
                                if let Some(i) = hist_idx {
                                    let n = history.len();
                                    if i + 1 < n {
                                        hist_idx = Some(i + 1);
                                        input = history[i + 1].clone();
                                    } else {
                                        hist_idx = None;
                                        input.clear();
                                    }
                                }
                            }
                            KeyCode::PageUp => {
                                pinned_bottom = false;
                                scroll_offset = (scroll_offset + 5).min(max_off_cell.get());
                            }
                            KeyCode::PageDown => {
                                scroll_offset = scroll_offset.saturating_sub(5);
                                if scroll_offset == 0 {
                                    pinned_bottom = true;
                                }
                            }
                            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                fold = !fold;
                            }
                            KeyCode::Char(c) => {
                                input.push(c);
                                hist_idx = None;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // ---- Fix 2: ensure mouse capture is always disabled on exit ----
    let _ = execute!(io::stdout(), DisableMouseCapture);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn draw(
    f: &mut Frame,
    input: &str,
    convo: &[(Kind, String)],
    running: bool,
    req_start: Option<Instant>,
    last_secs: Option<u64>,
    spin: u8,
    scroll_offset: usize,
    fold: bool,
    session: Option<&str>,
    sent: usize,
    max_off_cell: &Cell<usize>,
) {
    let size = f.size();

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(3),
        Constraint::Length(3),
    ])
    .split(size);

    // ---------- Top bar (Fix 3: DarkGray structure) ----------
    let top_block = Block::bordered()
        .border_style(Style::default().fg(C_STRUCT))
        .title(Span::styled(
            " hearth TUI ",
            Style::default().fg(C_STRUCT).add_modifier(Modifier::BOLD),
        ));
    let copyright = Paragraph::new(vec![Line::from(Span::styled(
        "\u{a9} 2026 Sapiens AI - hearth is a research demo TUI",
        Style::default().fg(C_STRUCT),
    ))]);
    f.render_widget(copyright.block(top_block), chunks[0]);

    // ---------- Conversation area (Fix 1: block-level markers, Fix 3: minimal colors) ----------
    let convo_block = Block::bordered()
        .border_style(Style::default().fg(C_STRUCT))
        .title(Span::styled(
            " conversation ",
            Style::default().fg(C_STRUCT),
        ));

    let mut convo_lines: Vec<Line> = Vec::new();
    for (kind, text) in convo {
        if *kind == Kind::Proc && fold {
            continue;
        }
        match kind {
            Kind::You => {
                // Fix 1: single "> " prefix on the first line only
                let lines_iter = text.lines();
                for (i, ln) in lines_iter.enumerate() {
                    if i == 0 {
                        convo_lines.push(Line::from(vec![
                            Span::styled("> ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                            Span::styled(ln, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                        ]));
                    } else {
                        convo_lines.push(Line::from(Span::styled(ln, Style::default().fg(Color::White).add_modifier(Modifier::BOLD))));
                    }
                }
            }
            Kind::Answer => {
                // Fix 1: no per-line "hearth:" prefix; 2-space indent, default foreground
                let lines_iter = text.lines();
                for ln in lines_iter {
                    convo_lines.push(Line::from(Span::styled(
                        format!("  {ln}"),
                        Style::default(), // default fg — no extra color
                    )));
                }
            }
            Kind::Proc => {
                // Fix 1: keep "· " prefix, dark gray
                let lines_iter = text.lines();
                for (i, ln) in lines_iter.enumerate() {
                    if i == 0 {
                        convo_lines.push(Line::from(vec![
                            Span::styled("· ", Style::default().fg(C_STRUCT)),
                            Span::styled(ln, Style::default().fg(C_STRUCT)),
                        ]));
                    } else {
                        convo_lines.push(Line::from(Span::styled(ln, Style::default().fg(C_STRUCT))));
                    }
                }
            }
            Kind::Warn => {
                convo_lines.push(Line::from(Span::styled(
                    format!("  {text}"),
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                )));
            }
        }
    }
    if !running && convo.len() <= 4 {
        convo_lines.push(Line::from(Span::styled(
            "  (type a message below and press Enter)",
            Style::default().fg(C_STRUCT),
        )));
    }

    let area_height = chunks[1].height.saturating_sub(2) as usize;
    let content_len = convo_lines.len();
    let max_off = content_len.saturating_sub(area_height);
    max_off_cell.set(max_off);
    let off = scroll_offset.min(max_off);
    let start = max_off.saturating_sub(off);
    let visible_iter = &convo_lines[start..];
    let convo_widget = Paragraph::new(visible_iter.to_vec())
        .wrap(Wrap { trim: false })
        .block(convo_block);
    f.render_widget(convo_widget, chunks[1]);

    // ---------- Input area (Fix 3: DarkGray border, White input) ----------
    let input_block = Block::bordered()
        .border_style(Style::default().fg(C_STRUCT))
        .title(Span::styled(
            " input  (Up/Down=history PgUp/PgDn=scroll Ctrl+T=fold wheel=scroll) ",
            Style::default().fg(C_STRUCT),
        ));

    let prompt = if input.is_empty() {
        vec![Span::styled(
            "> type a message... (Ctrl+Q=quit, Esc=clear, Enter=send) ▌",
            Style::default().fg(C_STRUCT),
        )]
    } else {
        vec![
            Span::styled("> ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(input, Style::default().fg(Color::White)),
            Span::styled("▌", Style::default().fg(C_ACCENT)),
        ]
    };
    let input_widget = Paragraph::new(Line::from(prompt)).block(input_block);
    f.render_widget(input_widget, chunks[2]);

    // ---------- Status bar (Fix 3: DarkGray structure, Cyan accent; Fix 4: show last action) ----------
    let session_short = match session {
        Some(u) => format!("session: {} ", &u[..u.len().min(8)]),
        None => "session: - ".to_string(),
    };
    // D10: renamed "steps:" → "turns:" (it was history.len(), i.e. user-sent message count,
    // not LLM inference steps).  Removed "hist:" from state_str to avoid two
    // same-meaning fields in the same status bar row.
    let status_title = format!(
        " model: agnes-3.0-flash | {} | turns: {} ",
        session_short,
        sent,
    );
    let status_block = Block::bordered()
        .border_style(Style::default().fg(C_STRUCT))
        .title(Span::styled(&status_title, Style::default().fg(C_STRUCT)));

    // D10: removed "hist:" from state_str — "turns:" already shows history.len() in the title bar.
    let state_str = format!(
        "[fold:{fold_s} scroll:{off}/{max_off}] ",
        fold_s = if fold { "on" } else { "off" },
        off = off,
        max_off = max_off,
    );

    let spin_chars = ["|", "/", "-", "\\"];
    let status_line = if running {
        let s = spin_chars[spin as usize];
        let secs = req_start.map(|t| t.elapsed().as_secs()).unwrap_or(0);
        // Fix 4: show last tool action
        let action = last_tool_action(convo);
        let action_str = action
            .as_deref()
            .map(|a| format!(" ⚙ {a}"))
            .unwrap_or_default();
        Line::from(vec![
            Span::styled(
                format!("{state_str}{s} running... ({secs}s){action_str}"),
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            ),
        ])
    } else if let Some(s) = last_secs {
        Line::from(vec![
            Span::styled(
                format!("{state_str}done ({s}s)"),
                Style::default().fg(C_ACCENT),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                format!("{state_str}ready"),
                Style::default().fg(C_ACCENT),
            ),
        ])
    };
    let status_widget = Paragraph::new(status_line).block(status_block);
    f.render_widget(status_widget, chunks[3]);
}
