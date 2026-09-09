# Hearth CLI 修复 + 渲染美化任务书（v0.1.1）

> 日期：2026-08-22 | 派发：施工窗口 | 基线：`a6559d9` (v0.1 发布)
> 来源：用户真机测试 `release/手工测试.txt`（VM wutao@192.168.220.131，真实跑通贪吃蛇任务）
> 决策（用户 12:36）：**先修 bug 再美化** + **只做 chat 渲染**（render.rs 层，不碰 repl TUI）
> 守门员核过源码，根因已定位（见各 R 的「根因定位」）。

---

## 0. 背景：用户真机测试暴露的三类问题

用户用 `hearth chat "写一个贪吃蛇的html小游戏"` 真跑，发现：

1. **🔴 write_file 超长参数 JSON 解析失败（P0 本体 bug）**
   - 现象：`WARN tool call 'write_file' has non-JSON arguments (EOF while parsing a string at line 1 column 2350); passing as raw string` → `ERROR ... missing 'path' argument` → 反复撞 4 次 → `budget exhausted`。
   - 结果：贪吃蛇只写出 `index.html`（1383 字节），缺 `style.css`/`game.js`，任务没跑完。
2. **🟡 hearth init 交互缺陷（P0 UX）**：provider/API key 两输入紧挨易混淆（用户填错 2 次）+ API key 明文回显（`sk-88ede...`）。
3. **🔵 CLI 渲染简陋（P1 美化）**：纯文本行式 dump，无用户/AI 气泡分离、无工具调用折叠、无规划草案卡片、WARN/ERROR 不醒目。

---

## 阶段一：P0 修复（必须先做，过闸后再做 P1）

### R1 修复 write_file 超长参数解析失败（🔴 核心）

**根因定位（守门员已核源码）**：
- `crates/llm-openai/src/lib.rs:367-373`：`serde_json::from_str(&tc.function.arguments)` 失败时 `unwrap_or_else` 把整段 arguments 当 `Value::String` 塞进 `ToolCall.args`——**静默降级**。
- `crates/tools-builtin/src/edit.rs:47-50`：`args.get("path").and_then(|v| v.as_str())`，当 `args` 是 `Value::String`（非对象）时 `.get("path")` 返回 None → `missing 'path' argument`。
- 用户日志显示 `column 2350` EOF = LLM 流式吐超长 HTML 参数被截断，JSON 不完整。

**修复要求（双层容错，治标+治本）**：
- **A（LLM 层，治标）** `lib.rs:367`：JSON 解析失败时不立即降级 raw string，先尝试**截断修复**：若 `arguments` 以 `{` 开头但末尾缺 `}`，补 `}` 后重试 `from_str`；仍失败才保留 raw string 并**打更醒目的 WARN**（含截断长度）。
- **B（调度层，治本）** `edit.rs:47` 及所有 builtin 工具取参处：当 `args` 是 `Value::String` 时，先 `serde_json::from_str::<Value>(s)` 再取字段；取不到才报错。这样即使 LLM 层降级了，调度层也能救回一次。
- **C（loop 层，防死循环）**：`agent-core/src/loop.rs` 的 replan 逻辑——当同一工具因 `missing 'path'` 连续失败 ≥2 次，应强制把"工具参数为空/截断"作为 gap 回喂 LLM 澄清（而非继续撞），避免 `budget exhausted` 前白烧 4 步。
- **验收证据**：用用户同款任务（写超长 HTML 的 write_file）构造单测 + 真机复测，确认 write_file 能拿到 path 并落盘完整文件。

### R2 修复 hearth init 交互（🟡 UX）

**根因定位**：`crates/codex-cli/src/lib.rs` / `config.rs` 的 init 用 `read_line` 读 API key 且**明文回显**；provider 与 key 提示紧挨无分隔。

**修复要求**：
- 加 `rpassword` 依赖（Cargo.toml），API key 输入**不回显**（密码式）。
- provider 与 API key 两输入之间加**明确分隔标签 + 默认值提示**，避免用户把 key 填进 provider 字段（用户踩坑 2 次的根因）。
- 不要在终端回显明文 key（config.toml 落盘是文件权限问题，不在本任务范围，但回显必须关）。
- **验收证据**：真机 `hearth init` 三次（正确填 / 填错 provider / 留空跳过），确认不回显 + 字段不混淆 + 错误引导清晰。

### R3 通用门禁（P0）

- `cargo fmt && cargo clippy` 0 warning；`cargo test` 全绿（含 R1 新增单测）。
- 注意：service integration `--test-threads=4` 偶发竞态已知（挂账），用 `--test-threads=2` 或单跑验证。

---

## 阶段二：P1 chat 渲染美化（P0 过闸后做）

### R4 chat 模式渲染重做（render.rs 层，仅 `colored` 不引 TUI 库）

**现状**：`crates/codex-cli/src/render.rs`（184 行，已用 `colored`）是扁平 dump，每行一个图标+颜色。

**美化要求（对照 codex/claude 体感，中等粒度）**：
- **用户/AI 气泡分离**：用户指令用 `>` 前缀或右对齐气泡；AI 输出（💭 思维 + 正文）用左对齐区块，视觉区分"谁在说话"。
- **工具调用折叠**：`⚙ glob {"pattern":"**/*"}` 这类——单行显示 `⚙ glob → **/*`（参数截断到 80 字 + `…`）；完整参数不刷屏。多工具调用用轻量缩进列表。
- **规划草案卡片化**：`🗺 规划草案（steps=N gaps=M）` 下方用缩进项目符号列步骤，假设项 `⚡` 用醒目色（黄）单独标。
- **日志分级**：`WARN`/`ERROR` 用醒目色 + 固定前缀（如 `⚠` / `✗`），不淹没在事件流里；`lsp_bridge: NoopLspBridge` 这类开发期 WARN 默认降噪（可 `RUST_LOG` 开）。
- **产物落盘提示**：`📄 产物 file: snake_game/index.html（+46 行）` 保留并加颜色高亮（已是青色，确认醒目）。
- **不破坏事实产生权**：render 层只做颜色/布局/折叠，**绝不**改 BE 传来的语义（warn/tool_call/artifact 等字段含义不变）；BE 仍不返回颜色/HTML/坐标。
- **验收证据**：用用户同款贪吃蛇任务跑 `hearth chat`，截屏/录输出对比美化前后；确认气泡分离、工具折叠、规划卡片、WARN 醒目、且 BE 事件字段未被 FE 篡改（grep 源码确认 render 只读不写语义）。

### R5 安装链路不动

- 安装/配置/安全默认等 v0.1 已验收项**不得回退**。美化与 R1/R2 修复不得影响 `hearth --version` / 零配置报错 / 安全徽章。

---

## 交付清单

| 交付 | 路径 | 说明 |
|---|---|---|
| 修复+美化 PR/commit | crates/llm-openai, crates/agent-core, crates/tools-builtin, crates/codex-cli | R1-R4 源码 |
| 验收报告 | `docs/acceptance-cli-fix-pretty-2026-08-22.md` | 含用户同款任务真机复测输出 |
| 新增单测 | 各 crate tests | R1 截断修复 + 调度层容错 |

## 范围外（明确排除）

- `hearth repl` 交互式 TUI 升级（Reedline）——本次只做 chat 渲染，不做 REPL 重构。
- 桌面版（B4-2 Tauri）——路线图第三梯队，等本次修复+美化收口、用户复测满意后再开。
- Windows/macOS 安装包——v0.2 候选。
- cgroup delegation 环境配置——属用户 VM 环境，README §3.3 已写清（HEARTH_ALLOW_NO_CGROUP=1）。

## 闸门判据

- 🔴=0（无接口/trait 违规）+ 实现率≥0.9 + 用户同款贪吃蛇任务真机跑完（写出 index.html+style.css+game.js 三文件）= 过闸。
- 守门员独立核验：R1 必查 `edit.rs` 取参容错 + `lib.rs:367` 截断修复 + loop replan 防死循环；R4 必查 render 只读语义不改 BE 字段。
