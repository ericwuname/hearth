use std::io::{self, BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame, Terminal,
};

const HEARTH_BIN: &str = "/home/wutao/hearth-slim/target/release/hearth";
/// Step 5: hearth runs in this cwd so `.hearth/reports/<uuid>` lands here.
const WORKDIR: &str = "/home/wutao/hearth-tui-new";

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    You,
    Answer,
    Proc,
}

enum UiMsg {
    Out(String),
    Err(String),
    Done(u64),
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

/// Spawn `hearth <args>` (chat / resume) in a background thread; stream
/// stdout/stderr lines to the UI thread via a channel. Never blocks the UI.
/// Step 3 pattern kept: thread + channel, non-blocking drain in the UI loop.
fn spawn_hearth(args: Vec<String>, tx: Sender<UiMsg>) {
    thread::spawn(move || {
        let start = Instant::now();
        let mut child = match Command::new(HEARTH_BIN)
            .args(&args)
            .current_dir(WORKDIR)
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

        let secs = child.wait().map(|s| s.code()).ok();
        let _ = t_out.join();
        let _ = t_err.join();
        let elapsed = start.elapsed().as_secs();
        let _ = secs;
        let _ = tx.send(UiMsg::Done(elapsed));
    });
}

fn main() -> io::Result<()> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut input = String::new();
    let mut quit = false;
    // ---- Step 4 state ----
    const HIST_MAX: usize = 20;
    let mut history: Vec<String> = Vec::new(); // committed inputs, newest last
    let mut hist_idx: Option<usize> = None;     // index into history while navigating (None = off)
    let mut scroll_offset: usize = 0;           // lines scrolled from bottom
    let mut pinned_bottom = true;               // new content => jump to bottom
    let mut fold = false;                       // hide `·` process lines

    let mut convo: Vec<(Kind, String)> = vec![
        (Kind::You, "what can hearth do?".to_string()),
        (
            Kind::Answer,
            "hearth is a small agent harness that talks to LLMs.".to_string()),
        (Kind::You, "show me a hello world".to_string()),
        (
            Kind::Answer,
            "fn main() { println!(\"hello world\"); }".to_string()),
    ];

    let (tx, rx): (Sender<UiMsg>, Receiver<UiMsg>) = channel();
    let mut running = false;
    let mut req_start: Option<Instant> = None;
    let mut last_secs: Option<u64> = None;
    let mut spin: u8 = 0;
    // ---- Step 5: multi-turn session ----
    let mut session_uuid: Option<String> = None; // current hearth session UUID

    while !quit {
        // Drain all pending messages from the worker threads (non-blocking).
        let mut got_msg = false;
        loop {
            match rx.try_recv() {
                Ok(UiMsg::Out(l)) if !l.is_empty() => {
                    convo.push((Kind::Answer, l));
                    got_msg = true;
                }
                Ok(UiMsg::Err(l)) if !l.is_empty() => {
                    convo.push((Kind::Proc, l));
                    got_msg = true;
                }
                Ok(UiMsg::Done(s)) => {
                    running = false;
                    last_secs = Some(s);
                    // Step 5: after each run, adopt the newest report dir as
                    // the session UUID (first chat creates it; resume reuses).
                    session_uuid = latest_report_uuid();
                }
                Ok(_) => {}
                Err(_) => break, // channel empty
            }
        }
        if got_msg && pinned_bottom {
            scroll_offset = 0;
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
                &history,
                session_uuid.as_deref(),
            );
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
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
                                convo.push((Kind::You, cmd.clone()));
                                running = true;
                                last_secs = None;
                                req_start = Some(Instant::now());
                                pinned_bottom = true;
                                scroll_offset = 0;
                                // Step 5: first send = hearth chat; after that,
                                // continue the same session with hearth resume.
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
                        // ---- Step 4: Up/Down history ----
                        KeyCode::Up => {
                            let n = history.len();
                            if n > 0 {
                                let next = match hist_idx {
                                    None => n - 1, // first Up: most recent
                                    Some(0) => 0,  // already at oldest
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
                                    hist_idx = None; // past newest -> empty line
                                    input.clear();
                                }
                            }
                        }
                        // ---- Step 4: PageUp/PageDown scroll ----
                        KeyCode::PageUp => {
                            pinned_bottom = false;
                            scroll_offset += 5;
                        }
                        KeyCode::PageDown => {
                            scroll_offset = scroll_offset.saturating_sub(5);
                            if scroll_offset == 0 {
                                pinned_bottom = true;
                            }
                        }
                        // ---- Step 4: 't' toggle fold (only on empty input,
                        // so normal typing of 't' is unaffected) ----
                        KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            fold = !fold;
                            // draw() clamps the offset to the new content height
                        }
                        KeyCode::Char(c) => {
                            input.push(c);
                            hist_idx = None;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

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
    history: &[String],
    session: Option<&str>,
) {
    let size = f.size();

    // Top: exactly 3 rows. Input & Status: exactly 3 rows each.
    // Conversation: everything left over.
    let chunks = Layout::vertical([
        Constraint::Length(3), // top bar
        Constraint::Min(1),    // conversation
        Constraint::Length(3), // input
        Constraint::Length(3), // status
    ])
    .split(size);

    // ---------- Top bar ----------
    let top_block = Block::bordered()
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " hearth TUI ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));
    let copyright = Paragraph::new(vec![Line::from(Span::styled(
        "\u{a9} 2026 Sapiens AI - hearth is a research demo TUI",
        Style::default().fg(Color::DarkGray),
    ))]);
    f.render_widget(copyright.block(top_block), chunks[0]);

    // ---------- Conversation area ----------
    let convo_block = Block::bordered()
        .border_style(Style::default().fg(Color::Magenta))
        .title(Span::styled(
            " conversation ",
            Style::default().fg(Color::Magenta),
        ));

    let mut convo_lines: Vec<Line> = Vec::new();
    for (kind, text) in convo {
        if *kind == Kind::Proc && fold {
            continue;
        }
        match kind {
            Kind::You => {
                let spans = text.lines().enumerate().map(|(i, ln)| {
                    if i == 0 {
                        Line::from(vec![
                            Span::styled(
                                "> you: ",
                                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                            ),
                            Span::raw(ln),
                        ])
                    } else {
                        Line::from(Span::raw(ln))
                    }
                });
                convo_lines.extend(spans);
            }
            Kind::Answer => {
                let spans = text.lines().enumerate().map(|(i, ln)| {
                    if i == 0 {
                        Line::from(vec![
                            Span::styled(
                                "  hearth: ",
                                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(ln, Style::default().fg(Color::Green)),
                        ])
                    } else {
                        Line::from(Span::styled(
                            ln,
                            Style::default().fg(Color::Green),
                        ))
                    }
                });
                convo_lines.extend(spans);
            }
            Kind::Proc => {
                let spans = text.lines().enumerate().map(|(i, ln)| {
                    if i == 0 {
                        Line::from(vec![
                            Span::styled(
                                "  · ",
                                Style::default().fg(Color::DarkGray),
                            ),
                            Span::styled(ln, Style::default().fg(Color::DarkGray)),
                        ])
                    } else {
                        Line::from(Span::styled(
                            ln,
                            Style::default().fg(Color::DarkGray),
                        ))
                    }
                });
                convo_lines.extend(spans);
            }
        }
    }
    if !running && convo.len() == 4 {
        convo_lines.push(Line::from(Span::styled(
            "  (type a message below and press Enter)",
            Style::default().fg(Color::DarkGray),
        )));
    }

    // Scroll: keep data intact, just change the render start.
    let area_height = chunks[1].height.saturating_sub(2) as usize; // inside border
    let content_len = convo_lines.len();
    let max_off = content_len.saturating_sub(area_height);
    let off = scroll_offset.min(max_off);
    // off = rows scrolled up from the bottom; start = max_off - off.
    // When off=0 (bottom) start=max_off; when off=max_off (top) start=0.
    let start = max_off.saturating_sub(off);
    let visible_iter = &convo_lines[start..];
    let convo_widget = Paragraph::new(visible_iter.to_vec())
        .block(convo_block)
        .style(Style::default().fg(Color::White));
    f.render_widget(convo_widget, chunks[1]);

    // ---------- Input area ----------
    let input_block = Block::bordered()
        .border_style(Style::default().fg(Color::Green))
        .title(Span::styled(
            " input  (Up/Down=history PgUp/PgDn=scroll Ctrl+T=fold) ",
            Style::default().fg(Color::Green),
        ));

    let prompt = if input.is_empty() {
        vec![Span::styled(
            "> type a message... (Ctrl+Q=quit, Esc=clear, Enter=send) ▌",
            Style::default().fg(Color::DarkGray),
        )]
    } else {
        vec![
            Span::styled(
                "> ",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
            Span::styled(input, Style::default().fg(Color::White)),
            Span::styled("▌", Style::default().fg(Color::Green)),
        ]
    };
    let input_widget = Paragraph::new(Line::from(prompt)).block(input_block);
    f.render_widget(input_widget, chunks[2]);

    // ---------- Status bar ----------
    let session_short = match session {
        Some(u) => format!("session: {} ", &u[..u.len().min(8)]),
        None => "session: - ".to_string(),
    };
    let status_title = format!(" model: agnes-3.0-flash | {} | steps: {} ", session_short, history.len());
    let status_block = Block::bordered()
        .border_style(Style::default().fg(Color::Blue))
        .title(Span::styled(
            &status_title,
            Style::default().fg(Color::Blue),
        ));

    let state_str = format!(
        "[fold:{fold_s} hist:{h} scroll:{off}/{max_off}] ",
        fold_s = if fold { "on" } else { "off" },
        h = history.len(),
        off = off,
        max_off = max_off,
    );

    let spin_chars = ["|", "/", "-", "\\"];
    let status_line = if running {
        let s = spin_chars[spin as usize];
        let secs = req_start.map(|t| t.elapsed().as_secs()).unwrap_or(0);
        Line::from(vec![
            Span::styled(
                format!("{state_str}{s} running... ({secs}s)"),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ])
    } else if let Some(s) = last_secs {
        Line::from(vec![
            Span::styled(
                format!("{state_str}done ({s}s)"),
                Style::default().fg(Color::Green),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                format!("{state_str}ready"),
                Style::default().fg(Color::Blue),
            ),
        ])
    };
    let status_widget = Paragraph::new(status_line).block(status_block);
    f.render_widget(status_widget, chunks[3]);
}
