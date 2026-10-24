# Hearth 上手指南（M2 · 终端 IDE）

> 目标：5 分钟内跑通第一例，并且知道怎么中断、怎么恢复、怎么导出可复核证据。

## 0. 前置（一次性）

- **Rust**：仓库根 `rust-toolchain.toml` 已钉 1.98.1；`cargo build -p codex-cli` 能过即可。
- **安装 `hearth` 命令**（一次性，否则会报"术语 'hearth' 不被识别"）：
  ```powershell
  Copy-Item .\target\debug\hearth.exe "$env:USERPROFILE\.cargo\bin\hearth.exe" -Force
  ```
  或正式安装：`cargo install --path crates/codex-cli`（release 编译，较慢）。
  没安装时，把下文的 `hearth` 换成 `.\target\debug\hearth.exe` 也能跑。
- **Windows 的 bash**：产品会**自动**用 `C:\Program Files\Git\bin\bash.exe`（45 位/32 位标准路径）；
  装在别处时设 `HEARTH_BASH_BIN=<bash.exe 绝对路径>`。**不要**指向 `cmd.exe`（工具会 fail-loud 拒绝）。
- **模型 key**：在仓库根 `.env` 写 `AGNES_API_KEY=…`（可加 `AGNES_BASE_URL` / `AGNES_MODEL`），
  或执行 `hearth init` 走引导。key 不进被跟踪文件。

## 1. 第一例：无终端也能验接线（推荐先跑）

```powershell
hearth tui --headless --provider agnes --budget 6 --goal "运行 bash 命令 echo HELLO，并告诉我输出"
```

> PowerShell **不认** bash 的 `\` 续行；要换行请用反引号 `` ` ``，或干脆写成一行。

看到 `TUI-SMOKE session=… status=completed … tool_calls=1 done=true` 即说明
**事件管线 + agent + 工具**都通了。`--headless` 只跳过全屏渲染，走的是同一套事件摄入。

## 2. 交互界面

```bash
hearth tui
```

| 键 | 作用 |
|---|---|
| `Enter` | 发送输入 |
| `Esc` / `Ctrl-C` | **中断本轮**（优雅收尾，历史保留） |
| `↑` / `↓` | 回看输出 |
| `y` / `n` / `Esc` | 审批面板：批准 / 拒绝 / 拒绝并继续 |
| `Ctrl-Q` 或输入 `/quit` | 退出 |

审批面板显示「要改什么（含参数）+ 为什么（破坏性命令 / workspace 外写入）+ 批不批」。
**全屏渲染需真 TTY**；CI / 重定向环境请用 `--headless`。

### 2.1 桌面 GUI（`hearth gui`）

```powershell
hearth gui                                  # 打开窗口，输入任务后 Enter
hearth gui --goal "修复 src 里失败的测试"   # 启动即提交
```

- 同一内核与同一会话落盘：单进程、无 WebView/Node 依赖（eframe/egui）。
- 顶栏：phase / 步数 / 运行状态 / **中断** / **导出证据**（等价 `hearth session export`）。
- 审批：破坏性命令会在**输入框上方**出现审批条（参数 + 批准/拒绝）；
  60 秒未处理按**拒绝**处理，本轮结束时会如实标注。
- 窗口打开后输入框**自动获得焦点**，可直接打字。
- 快捷键：`Esc` = 有待审批先拒绝 / 运行中请求中断；`Ctrl+E` = 导出证据；`Ctrl+Q` = 退出。
- 顶栏第二行显示 provider/model 与工作区；右侧 `预算` 可拖动调整（只影响后续提交）。
- 工具调用会折叠成卡片（`✓/✗/…` + 工具名），最新一张自动展开；diff 结果逐行着色。


## 3. 可复核证据（一条命令）

```bash
hearth chat "创建文件 hello.txt，内容写 HELLO" --provider agnes --approve-within session
hearth sessions                      # 找到刚产生的 session id
hearth session export <id>           # → hearth-session-<id前8位>.json
```

导出是**单个自包含 JSON**：

- `changes[].diff`：改前快照 → 当前文件的逐行差异（新建文件 = 全 `+` 行）；
- `reports[]`：每轮 run 的 Markdown 报告（步数/工具成败/token/审批）；
- `transcript[]`：本次会话的命令级日志；
- **未知 ≠ 0**：`taskgoal` / `run_state` / `transcript` 为 `null` 表示**本机没有该档**，
  不是"空历史/零成本"。

## 4. 中断与恢复（含**已知边界**）

交互中按 `Esc`：agent 在步边界停下（≤5s 收尾），把已完成的轮次写入
`%APPDATA%\hearth\sessions\<id>.jsonl`，然后：

```bash
hearth resume <id> --budget 4 "继续"
```

**负结果如实登记**：直接杀进程（SIGKILL / 关窗口）时，**当前未完成轮**不落盘，
只能恢复到上一轮；不声称"任意时刻中断零损失"。已完成的轮次与已改文件不丢
（写前快照在 `snapshots/<id>/`，可用 `hearth rollback <id>` 撤销）。

## 5. 常见问题

- `Bash/Service/0x8007072c`：那是 system32 的 WSL bash。产品已默认回退 Git Bash；
  若仍出现，设 `HEARTH_BASH_BIN` 指到 Git Bash。
- `cargo: command not found`：确认 `%USERPROFILE%\.cargo\bin` 在 PATH；bash 子进程会继承
  OS 必需环境变量（PATH/TEMP/SystemRoot/HOME）。
- TUI 里看不到日志：**故意的**——日志写 `%APPDATA%\hearth\diagnostics.log`
  （或 `~/.config/hearth/diagnostics.log`），避免画花全屏。排障时看该文件。
- 审批一直不动：交互模式下必须按 `y`/`n`；`chat` 非交互请加 `--approve-within session`。
  `tui --headless` **默认 fail-closed**（破坏性操作直接拒绝，不会挂住）；只做审批链冒烟时
  可加隐藏旗标 `--auto-approve`。`gui` 的审批条 60 秒未处理按拒绝处理（内核等待上限）。
- `gui` 报 `HTTP 401 无效的令牌`：是 key 失效（不是界面问题）。更新仓库根 `.env` 的
  `AGNES_API_KEY` 或 `hearth config set api-key <新 key>` 后重启 `hearth gui`。
  **Windows 上 `hearth config set` 写的是 `%APPDATA%\hearth\config.toml`**；手工去改
  `~/.config/hearth/config.toml` 不会生效。`.env` 从**启动目录**加载——在仓库根启动最稳。
