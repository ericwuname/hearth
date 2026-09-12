# hearth TUI 四阶段验收报告 v1.0（评审窗 → 顶层）

| 项 | 值 |
|---|---|
| 验收日期 | 2026-09-12 14:38–14:39 |
| 执行机 | `192.168.220.133`（评审/跑测机，全程零本机执行） |
| 目标目录 | `/home/wutao/hearth-tui-new` |
| 验收脚本 | `/tmp/tui-accept-v2.sh`（**修正版**；原 `/tmp/tui-accept.sh` 存在 3 处缺陷，见 §4） |
| 原始日志 | `/tmp/tui-accept-v2.log`（追加不删） |
| 抓屏留档 | `/tmp/tui-screen.txt`（纯文本）、`/tmp/tui-screen-ansi.txt`（含 ANSI 颜色） |
| 判分人 | 砺·评审（判读分离；终判归顶层/用户） |

## 0. 结论摘要

**TUI 骨架成立，但停在「能编译、能启动、不能用」。** 四阶段中 ①④ 通过，②③ 不通过，
核心是一条**渲染布局缺陷：输入框内容区高度为 0，用户敲的字一个也看不见**。

| 阶段 | 判定 | 一句话 |
|---|---|---|
| ① 功能有没有 | ✅ **通过** | 四区骨架（顶栏/对话/输入/状态）+ 按键处理 + 六色主题俱在，164 行 |
| ② 能不能用 | ⚠️ **不通过** | 编译启动正常、抓屏有 32 行版面，但**输入行整行空白**（无提示语、无光标文字） |
| ③ 好不好用 | ❌ **不通过** | 真按键注入 `hello hearth` → 屏上**命中 0 行**；输入不可见即等于不可用 |
| ④ 好不好看 | ✅ **基本通过** | 六色调色板齐全（69 个 SGR 序列）、边框字符正常；扣分项：底部两区块边框被裁切 |

**距「用户愿意每天打开」差一次 1 行修复**（见 §3）。

---

## 1. 四阶段逐项证据

### ① 功能有没有 —— ✅ 通过

```
-rw-rw-r-- 1 wutao wutao 5492  9月 12 14:32 main.rs
164 /home/wutao/hearth-tui-new/src/main.rs
```

| 组件关键词（源码命中） | 次数 |
|---|---|
| `top_block` / `convo_block` / `input_block` / `status_block` | 2 / 2 / 2 / 2 |
| `Block::bordered` | 4 |
| `Borders::ALL` | 3 |
| `Layout::vertical` | 1 |
| `Constraint::` | 4 |

主入口：`main.rs:18 fn main() -> io::Result<()>`。按键分支齐（`q` / `Ctrl-C` / `Esc` / `Backspace` / `Char`）。

### ② 能不能用 —— ⚠️ 不通过

| 检查项 | 结果 |
|---|---|
| `cargo build --release` | `Finished release profile [optimized] target(s) in 0.09s`（增量） |
| 二进制 | `908304` 字节 `target/release/hearth-tui` |
| tmux 会话存活 | `SESSION_ALIVE=yes` |
| 抓屏字节数 / 非空行 | `5244` / `32` |

**抓屏实录（32 行，节选），注意第 31 行"input"区块：**

```
┌ hearth TUI ──────────────────────────────────────────────────────────────────────────────────────┐
│© 2026 Sapiens AI · hearth is a research demo TUI — not a product                                 │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
┌ conversation (demo) ─────────────────────────────────────────────────────────────────────────────┐
│> you: what can hearth do?                                                                        │
│  hearth:  hearth is a small agent harness that talks to LLMs via a JSON stream.                  │
│> you: show me a hello world                                                                      │
│  hearth:  fn main() { println!("hello world"); } — run with `cargo run`.                         │
│  — end of demo conversation (next steps will stream real turns) —                                │
│                            （此处 22 行空白对话区）                                               │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
┌ input ───────────────────────────────────────────────────────────────────────────────────────────┐   ← 第 31 行
┌ model: agnes-3.0-flash | session: demo | steps: 0 ───────────────────────────────────────────────┐   ← 第 32 行
```

**输入区块只有一条上边框，提示语 `> type a message…  (q = quit, Esc = clear)` 整行不存在。**
全屏 grep `type a message` → **0 命中**。

### ③ 好不好用 —— ❌ 不通过（真按键交互，非源码 grep）

| 子项 | 动作 | 观测 | 判定 |
|---|---|---|---|
| 3.1 输入回显 | `tmux send-keys -l "hello hearth"` | 屏上含 `hello hearth` 行数 = **0** | ❌ |
| 3.2 退格 | `Backspace` ×6 | 无可观测变化（因 3.1 内容本就未渲染） | — |
| 3.3 Esc 清空 | `Escape` | 占位提示仍 **0 命中** | ❌ |
| 3.4 退出 | `send-keys -l "q"` | `QUIT=yes(已退出)` | ✅ |

**关键推论（避免误判）**：`q` 能退出证明**事件循环与按键读取是通的**（`event::read()` 正常）。
问题不在输入处理，而在**渲染**——`input.push(c)` 确实改了状态，只是那块区域高度为 0，画不出来。

### ④ 好不好看 —— ✅ 基本通过

**调色板实测（ANSI 抓屏去重计数）：**

| 转义序列 | 颜色 | 出现次数 |
|---|---|---|
| `ESC[38;5;5m` | Magenta（对话框线） | 26 |
| `ESC[38;5;15m` | Bright White（正文） | 25 |
| `ESC[38;5;6m` | Cyan（顶栏） | 3 |
| `ESC[38;5;2m` | Green（输入框线 / hearth 回复） | 3 |
| `ESC[38;5;8m` | Dark Gray（版权行） | 2 |
| `ESC[38;5;3m` | Yellow（`> you:` 前缀） | 2 |
| `ESC[38;5;4m` | Blue（状态栏） | 1 |
| `ESC[1m` / `ESC[0m` | Bold / Reset | 3 / 3 |

颜色转义序列共 **69** 个；边框字符 `│`×52、`─`×617、`┌`×4、`┐`×4、`└`×2、`┘`×2。
**扣分项**：`└┘` 只有 2 对（对话区），输入区与状态栏的**下边框被裁到屏幕外**——版面底部看着"没封口"。

---

## 2. 根因分析（带源码锚点）

`/home/wutao/hearth-tui-new/src/main.rs:64-70`

```rust
let chunks = Layout::vertical([
    Constraint::Length(3),   // top bar
    Constraint::Min(8),      // conversation
    Constraint::Length(1),   // input line      ← 高度 1
    Constraint::Length(1),   // status bar      ← 高度 1
])
.split(size);
```

`main.rs:138-141`
```rust
let input_block = Block::bordered()
    .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)   // 吃掉 1 行
```

**算术**：`Block` 的 `inner()` 会先扣掉边框占用。输入区拿到 **1 行**，被上边框吃掉后
**内容区高度 = 1 − 1 = 0** → `Paragraph` 无像素可画 → 文字与左右竖线全部消失。
状态栏同理：`Borders::ALL` 扣 2 行，1 − 2 < 0 → 只剩标题（标题画在边框上），底边框越界。

**这解释了为什么"能跑但看不见字"**——不是输入法问题、不是事件问题，
是**把带边框的 widget 塞进了只剩 1 行的格子**。

---

## 3. 修复建议（交施工窗，评审窗不动刀）

`src/main.rs:64-70` 一行改动方向：

```rust
let chunks = Layout::vertical([
    Constraint::Length(3),   // top bar   (bordered → ≥2)
    Constraint::Min(6),      // conversation
    Constraint::Length(3),   // input     (bordered → ≥2，给 3 留视觉余量)
    Constraint::Length(3),   // status    (bordered → ≥2)
])
.split(size);
```

**理由**：任何带边框的 `Block` 至少需要 **2 行**（1 行边框 + 1 行内容）。
当前 `3 + Min(8) + 1 + 1` 恰好占满 32 行，底部两区块必然被裁；
改为 `3 + Min(6) + 3 + 3` 后，底部有富余，且输入区可见。

**建议一并处理（不进本件，登记为后续）**：
1. `main()` 完全未解析 `argv` —— `--demo` 是 **no-op**，demo 内容是硬编码的。
   验收脚本按 `--demo` 调用属"调用与实现契约不符"（不影响本次判定，但会误导后续脚本作者）。
2. 输入区可考虑固定 3 行 + 长文本内滚动，避免超长输入再次顶穿版面。

---

## 4. 工具自身审计（两个脚本都有缺陷，必须记账）

### 4.1 `/tmp/tui-accept.sh` —— 三处缺陷，**原样执行会给出假阴性**

| # | 缺陷 | 实证 | 危害 |
|---|---|---|---|
| 1 | **目标目录错**：`D=/home/wutao/hearth-tui` | 该目录是 9/1–9/7 的**旧 Web 项目**（`index.html`/`package.json`/`js/`，**无 `src/`、无 `Cargo.toml`**），不是 TUI 产物 | 四阶段全部落空 → 误判"什么都没做" |
| 2 | 同上引发的**误编译** | `cd /home/wutao/hearth-tui && cargo locate-project` → `{"root":"/home/wutao/Cargo.toml"}`（cargo 向上查找），而 `/home/wutao/Cargo.toml` 是无关包 `docs-rs-source` | `cargo build --release` 会去构建**别的包** |
| 3 | 阶段③ 只做源码 grep、阶段④ `\| head -0` | `head -0` 输出 **0 行**，颜色证据被整条管道丢弃 | ③④ 等于没测 |

**据此本报告改用修正版 `/tmp/tui-accept-v2.sh`**（原脚本保持原样未改动）：
目标目录更正为 `hearth-tui-new`；③ 升级为 `send-keys` 真按键 + `capture-pane` 回读；④ 保留 ANSI 转义做颜色证据。

### 4.2 环境缺陷：**`tmux` 根本未安装**（这是 TUI 施工卡死的直接原因之一）

```
command -v tmux        → 无
dpkg -l | grep tmux    → 无
/usr/bin/tmux          → 不存在
apt-cache policy tmux  → 候选 3.4-1ubuntu0.1（未安装）
```

**因果链（决定性）**：TUI 施工 agent（ember）第三轮的**最后一次工具调用**正是
`tmux new-session -d -s t ... && tmux capture-pane -t t -p`（见 `trace3.err` 末尾），
返回值 `98 字符`——即 shell 的"未找到命令"报错。**它无法自验，只能空转烧轮次。**

**已处置**：2026-09-12 14:38 于 `.133` 安装 `tmux 3.4-1ubuntu0.1`（含 `libevent-core-2.1-7t64`、`libutempter0`），
`tmux -V` → `tmux 3.4` 验证通过。**此项同时为 TUI 后续复验与 EMBER 侧抓屏能力解堵。**

### 4.3 TUI 施工三轮运行史（为何没有 `TUI_DONE`）

| 轮次 | 日志 | 时间 | 终态 |
|---|---|---|---|
| 1 | `run.log`（**0 字节**）+ `trace.err` | 13:32→13:51 | 驱动 shell 中途被杀，`TUI_EXIT`/`TUI_DONE` 两行**从未写入**；期间 Agnes 出现 `read operation timed out` 重试 3 次后失败 |
| 2 | `run2.log` + `trace2.err` | 14:19 | `（达到 20 轮工具循环上限，任务未收敛——请缩小问题范围重试）` |
| 3 | `run3.log` + `trace3.err` | 14:32 | 同上，**20 轮上限**；末次工具调用即 tmux 抓屏（此时 tmux 不存在） |

**结论**：`run.log` 无 `TUI_DONE`、进程已消失 → 按 A 分支触发验收；
但**"进程消失"≠"任务完成"**，真实终态是"**环境缺 tmux 导致自验失败 → 烧完轮次**"。
另注：`tui-run.sh` 用默认 20 轮预算，而 `m5-1-launch.sh` 用 `--budget 30` —— 预算口径不一致，
建议后续统一（TUI 这类"写码+自验"任务 20 轮偏紧）。

**通道健康度旁证**：`curl https://api.agnes-ai.cn/v1/models` → `HTTP 401, t=0.17s`（未带 key 的预期响应，**通道可用**）。

---

## 5. 待顶层裁决 / 下一步

| # | 事项 | 建议 |
|---|---|---|
| 1 | TUI 一行修复（`Layout` 高度） | **建议开一个「TUI 修一轮」小件**，施工窗一次按键自验后复交本报告 §1 口径复验 |
| 2 | 复验脚本 | 复用 `/tmp/tui-accept-v2.sh`（原脚本建议废弃或修正后另存，勿再生效） |
| 3 | ember 轮次预算 | `tui-run.sh` 的 20 轮 → 建议提到 30，与 M5 系列口径一致 |
| 4 | `--demo` 契约 | 要么真解析 argv，要么验收脚本去掉 `--demo`，二选一，别留模糊 |

**口径登记**：本报告数字均出自 `2026-09-12 14:38–14:39` 单次实测；
`tmux 3.4` 为本次新装环境变更，后续复验必须在同一环境（含 tmux）下进行，否则**不可比**。
