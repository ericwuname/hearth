# hearth-tui

Hearth 的终端 UI（TUI）客户端——在终端里与 hearth 引擎对话、查看任务状态与产物。

## 技术栈

- Rust 2021 + ratatui 0.26 + crossterm 0.27
- 经 hearth 的 SSE 接口与会话文件通信（只读消费 events，不重复实现引擎逻辑）

## 当前状态

- 会话列表 / 消息流渲染已可用（键盘上下滚动）
- **已知问题**：鼠标事件（点击选中会话/消息）尚未生效——见 `BUGS.md` BUG-003
- 详见 `PROGRESS.md` 与 `reports/`
