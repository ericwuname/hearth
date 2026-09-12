# hearth TUI 首版界面抓屏（EMBER 施工，2026-09-12 16:40 实抓）

来源：/home/wutao/hearth-tui-new/（cargo build --release ✓ 908,304 字节二进制）
抓屏命令：tmux new-session -d + capture-pane -p

```
┌ hearth TUI ────────────────────────────────────────────────────────────────────────────────────────────────┐
│© 2026 Sapiens AI · hearth is a research demo TUI — not a product                                           │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌ conversation (demo) ───────────────────────────────────────────────────────────────────────────────────────┐
│> you: what can hearth do?                                                                                  │
│  hearth:  hearth is a small agent harness that talks to LLMs via a JSON stream.                            │
│> you: show me a hello world                                                                                │
│  hearth:  fn main() { println!("hello world"); } — run with `cargo run`.                                   │
│  — end of demo conversation (next steps will stream real turns) —                                          │
│                                                                                                            │
│  （中略：对话区主体，共 20 行）                                                                              │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌ input ───────────────────────────────────────────────────────────────────────────────────────────────────────┐   ← ⚠️ 只渲染出上边框（布局 bug）
┌ model: agnes-3.0-flash | session: demo | steps: 0 ───────────────────────────────────────────────────────────┐   ← ⚠️ 同上
```

## 四阶段初步判定（执行窗实测）

| 阶段 | 证据 | 判定 |
|---|---|---|
| ① 功能有没有 | 164 行 main.rs；Block×5 / Paragraph×4 / Layout×2 / Style×17 / Color×17；顶栏+对话区+input(×16)+状态栏(×4) 均已定义 | ✅ 有 |
| ② 能不能用 | 908KB binary；tmux 启动即出界面；`q`/Ctrl-C 退出路径在 | ✅ 能（**输入不回显** ✗） |
| ③ 好不好用 | KeyCode 处理 ×6（Esc/Enter/Char）；**无 Up/Down/PageUp/折叠** ✗ | 🟡 基础 |
| ④ 好不好看 | 边框/分区/对齐/版权行 OK；**底部两区块只画了上边框（布局塌）** ✗；无 spinner ✗ | 🟡 上半好 |

**下一步（step2 卡）**：修底部布局（input 框与状态栏完整渲染）+ 输入回显 + 接 hearth 进程（tmux 里真交互）+ spinner。
