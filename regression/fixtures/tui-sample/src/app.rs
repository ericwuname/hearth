//! 会话状态与滚动（骨架）。

pub struct App {
    pub sessions: Vec<String>,
    pub selected: usize,
    pub scroll_offset: u16,
}

impl App {
    /// M1 里程碑产物：会话列表渲染的数据源（PROGRESS.md 锚点 commit a1b2c3d）。
    pub fn new(sessions: Vec<String>) -> Self {
        Self { sessions, selected: 0, scroll_offset: 0 }
    }

    /// M2 里程碑产物：消息流滚动（PROGRESS.md 锚点 commit b2c3d4e，本文件 :16）。
    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// BUG-003 嫌疑路径：点击会话选中——从未被事件循环调用（main.rs Mouse 分支缺失）。
    pub fn select_session(&mut self, idx: usize) {
        if idx < self.sessions.len() {
            self.selected = idx;
        }
    }
}
