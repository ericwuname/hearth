//! hearth-tui 入口：终端初始化 + 事件循环骨架（fixture 供回归语料上下文用）。
//! NOTE(BUG-003): 当前事件循环只处理 Event::Key——鼠标事件未接（复测失败一次，
//! 见 BUGS.md BUG-003 与 app.rs:52）。

use crossterm::event::{Event, KeyCode, KeyEventKind};

mod app;
mod ui;

fn main() {
    let mut app = app::App::new(vec!["session-1".into(), "session-2".into()]);
    loop {
        // 真实实现：terminal.draw(|f| ui::draw(f, &app))
        ui::draw_to_stdout(&app);
        match crossterm::event::read().expect("event read") {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Down => app.scroll_down(),
                KeyCode::Up => app.scroll_up(),
                // TODO(BUG-003): Event::Mouse 分支缺失——点击无反应的嫌疑点 ①
                _ => {}
            },
            // TODO(BUG-003): 嫌疑点 ① 本体——Mouse 事件直接落入此通配分支被丢弃
            _ => {}
        }
    }
    println!("bye");
}
