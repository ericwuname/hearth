# Hearth 上手指南（M2 · 终端 IDE）

> 目标：5 分钟内跑通第一例，并且知道怎么中断、怎么恢复、怎么导出可复核证据。

## 0. 前置（一次性）

- **Rust**：仓库根 `rust-toolchain.toml` 已钉 1.98.1；`cargo build -p codex-cli` 能过即可。
- **Windows 的 bash**：产品会**自动**用 `C:\Program Files\Git\bin\bash.exe`（45 位/32 位标准路径）；
  装在别处时设 `HEARTH_BASH_BIN=<bash.exe 绝对路径>`。**不要**指向 `cmd.exe`（工具会 fail-loud 拒绝）。
- **模型 key**：在仓库根 `.env` 写 `AGNES_API_KEY=…`（可加 `AGNES_BASE_URL` / `AGNES_MODEL`），
  或执行 `hearth init` 走引导。key 不进被跟踪文件。

## 1. 第一例：无终端也能验接线（推荐先跑）

```bash
hearth tui --headless --provider agnes --budget 6 \
  --goal "运行 bash 命令 echo HELLO，并告诉我输出"
```

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
- 审批一直不动：交互模式下必须按 `y`/`n`；headless/非交互模式请加 `--approve-within session`
  （仅 `chat` 支持）。
