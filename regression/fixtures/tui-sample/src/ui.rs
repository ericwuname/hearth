//! 渲染层（骨架——fixture 只需给语料以"项目存在"的上下文）。

use crate::app::App;

pub fn draw_to_stdout(app: &App) {
    println!("── hearth-tui ── sessions: {}", app.sessions.len());
    for (i, s) in app.sessions.iter().enumerate() {
        let mark = if i == app.selected { ">" } else { " " };
        println!("{mark} {s}");
    }
}
