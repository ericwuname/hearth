//! 纯渲染层：`&TuiState → Frame`。不读事件、不碰文件（便于目测与后续快照测试）。

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::{LineKind, TuiState};

fn style_for(kind: &LineKind) -> Style {
    match kind {
        LineKind::User => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        LineKind::Assistant => Style::default().fg(Color::White),
        LineKind::Tool => Style::default().fg(Color::Yellow),
        LineKind::ToolOk => Style::default().fg(Color::Green),
        LineKind::ToolErr => Style::default().fg(Color::Red),
        LineKind::System => Style::default().fg(Color::DarkGray),
        LineKind::Approval => Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    }
}

pub fn draw(f: &mut Frame, st: &TuiState) {
    let approval_h = if st.pending_approval.is_some() { 6 } else { 0 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(6),
            Constraint::Length(approval_h),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(f.area());

    // ── header ─────────────────────────────────────────────────────────
    let status = if st.running { "运行中" } else { "就绪" };
    let status_style = if st.running {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    };
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " Hearth ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            "  session {}   phase {}   steps {}   ",
            st.session_short, st.phase, st.steps
        )),
        Span::styled(status, status_style),
    ]))
    .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(header, chunks[0]);

    // ── transcript（默认跟随尾部；↑ 键向上翻看）─────────────────────
    let inner_h = chunks[1].height.saturating_sub(2) as usize;
    let end = st
        .lines
        .len()
        .saturating_sub(usize::from(st.scroll_from_bottom));
    let start = end.saturating_sub(inner_h);
    let lines: Vec<Line> = st.lines[start..end]
        .iter()
        .map(|l| Line::from(Span::styled(l.text.clone(), style_for(&l.kind))))
        .collect();
    let transcript = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" 会话 "))
        .wrap(Wrap { trim: false });
    f.render_widget(transcript, chunks[1]);

    // ── approval（M2-4：要改什么 / 为什么 / 批不批）──────────────────
    if let Some(pa) = &st.pending_approval {
        let what = if pa.tools.is_empty() {
            pa.action.clone()
        } else {
            format!("{} — {}", pa.action, pa.tools.join(" · "))
        };
        let body = vec![
            Line::from(Span::styled(
                format!("⛔ 待审批：{what}"),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "为什么：该操作被审批策略判定为破坏性命令 / workspace 外写入",
                Style::default().fg(Color::Gray),
            )),
            Line::from(Span::styled(
                "[y] 批准    [n] 拒绝    [Esc] 拒绝并继续",
                Style::default().fg(Color::Yellow),
            )),
        ];
        f.render_widget(
            Paragraph::new(body).block(Block::default().borders(Borders::ALL).title(" 审批 ")),
            chunks[2],
        );
    }

    // ── input ─────────────────────────────────────────────────────────
    let input_text = if st.running {
        Span::styled("（运行中：Esc 中断）", Style::default().fg(Color::DarkGray))
    } else {
        Span::styled(st.input.clone(), Style::default().fg(Color::White))
    };
    f.render_widget(
        Paragraph::new(Line::from(input_text))
            .block(Block::default().borders(Borders::ALL).title(" 输入 ")),
        chunks[3],
    );
    if !st.running {
        // 光标跟在输入末尾（CJK 宽度按字符数近似——仅影响视觉观感）。
        let x = chunks[3].x + 1 + st.input.chars().count().min(u16::MAX as usize) as u16;
        let y = chunks[3].y + 1;
        if x < chunks[3].x + chunks[3].width.saturating_sub(1) {
            f.set_cursor_position((x, y));
        }
    }

    // ── footer ────────────────────────────────────────────────────────
    let hint = if st.pending_approval.is_some() {
        " y 批准 · n 拒绝 · Esc 拒绝 · Ctrl-Q 退出"
    } else if st.running {
        " Esc / Ctrl-C 中断本轮 · ↑↓ 滚动 · Ctrl-Q 退出"
    } else {
        " Enter 发送 · Esc 清空输入 · ↑↓ 滚动 · /quit 或 Ctrl-Q 退出"
    };
    f.render_widget(
        Paragraph::new(Span::styled(hint, Style::default().fg(Color::DarkGray)))
            .wrap(Wrap { trim: true }),
        chunks[4],
    );
}
