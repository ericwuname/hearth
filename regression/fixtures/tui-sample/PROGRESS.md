# hearth-tui 进度看板

> 本文件由开发过程维护，数字均带出处锚点（文件:行 或 commit）。

## 里程碑

| 里程碑 | 状态 | 锚点 |
|---|---|---|
| M1 会话列表渲染 | ✅ 完成 | commit `a1b2c3d`（src/ui.rs:20 render_sessions） |
| M2 消息流滚动 | ✅ 完成 | commit `b2c3d4e`（src/app.rs:41 scroll_offset） |
| M3 SSE 事件接入 | ✅ 完成 | reports/run-001.md（33 条事件回放通过） |
| M4 鼠标点击选中 | ⚠️ 未生效 | BUGS.md BUG-003（点击无反应，复测仍失败） |
| M5 主题配色 | ⬜ 未开始 | — |

## 完成度

- 已完成 3/5 里程碑（60%）；M4 阻塞中（复测一次未修复，见 BUGS.md BUG-003 复测记录）
- 代码量：src/ 共 3 文件 214 行（`wc -l src/*.rs` 口径，不含 Cargo.toml）
