# Hearth CLI 开箱即用任务书 — 像 `codex` / `claude` 一样：装好即全局可用、零配置上手

> ⚠️ **本文件已被 `docs/hearth-cli-design.md` 取代**（2026-08-22，用户决策：派 A 一步到位、一次性做对）。
> 本文件保留作"待选框架"参考（含派 A/B 取舍讨论），**不再是执行依据**。执行窗口以 `hearth-cli-design.md` 为准。

> **签发**：顶层（守门员）
> **日期**：2026-08-22
> **执行窗口**：后端 dev-execution 窗口（照本任务书施工，跑门禁，交证据）
> **来源**：用户原话（2026-08-22）——"希望 Hearth CLI 用法像 codex CLI / Claude CLI 那样简单：下载安装，打开终端输入命令就能调用"
> **目标用户场景**：用户本机 **Linux**（主），Windows PowerShell（次，可选 cross-compile）
> **性质**：**D 类（改控制流拓扑：去掉 client→service 两进程门槛，改单二进制/自托管）+ 产品级 UX**

---

## 0. 源码现状（守门员已核验，避免凭印象）

`crates/codex-cli/src/client.rs` 明确是 **HTTP+SSE client for the codex HTTP service**——CLI 纯客户端，**必须身后有 `service` 在跑**（`localhost:3000`）。这与 Claude Code / Codex CLI 的"单二进制自托管"体验有本质差距：

- 现状：用户必须①先 `cargo run -p service` 起内核 ②再开终端 `cargo run -p codex-cli -- chat`——**两步、两个进程、还要会 cargo**。这远不是"装好即调"。
- **零安装机制**：无 install 脚本、无 `[profile.release]`、无 deb/msi/安装器。当前只有 `cargo run`，无法"下载装好就能用"。
- 二进制名错位：`codex-cli` 包编译出可执行文件叫 `codex`（main.rs），但用户期望叫 `hearth`。

### 现状硬事实（写进任务书边界）
- service 真名 crate = `service`，二进制 = `service`。
- CLI crate = `codex-cli`，二进制 = `codex`，主力 provider = deepseek（写死 `main.rs:156`）。
- 契约文件 `docs/ai-os-event-contract-v1.md` 已钉死（前后端解耦唯一契约）——**本任务不得破坏契约**，只是把"谁起内核"从"用户手动"改成"CLI 自托管/自动拉起"。

---

## 1. 终态目标（用户原话转译）

> 下载安装 → 打开任意终端（PowerShell / bash / zsh）→ 输入 `hearth` → 直接能用。

具体验收画面（以 Linux 为主）：
```
$ hearth "用 Rust 写个读 CSV 的小工具"     # 装好后在任意目录直接跑，无需先起 service、无需 .env
```
- **全局可用**：装完后 `hearth` 在 PATH 任意目录可调用（类似 `codex` / `claude` / `cargo`）。
- **零配置首次运行**：首次运行若无 key，自动引导（交互式填 key 或报错指明显式 `--api-key` / 环境变量），不强制手写 `.env`。
- **单二进制优先**：用户不需要知道"service 存在"。内核在 CLI 进程内或自动拉起，对用户透明。
- **跨平台（次）**：提供 Linux x86_64 主二进制；可选 Windows x86_64（PowerShell 可用）与 macOS。

---

## 2. 架构策略（二选一，执行窗口照选中的施工；守门员推荐 A）

### 派 A — 单二进制自托管（⭐ 推荐，最像 Claude/Codex）
- CLI 进程**直接内嵌 agent 内核**（把 `service` 的会话/loop/LLM/沙箱逻辑作为 library 引入，CLI 调用之），不再依赖独立 `service` 进程。
- 删除"必须先起 service"门槛。`hearth chat/repl` 本进程内跑，`render_events` 直接消费内核事件流（已有 `codex-cli/src/render.rs`，复用）。
- 代价：需把 `service` crate 的会话管理/路由逻辑抽成 `agent-runtime` library（crate 内 refactor，不破坏契约）。D 类但范围可控（不新增业务语义，仅拓扑重组）。

### 派 B — 保留 client+service 分离，但加 `hearth service` 自拉起
- 保留现有 HTTP 架构；新增 `hearth service` 子命令（后台拉起内核），且 `hearth chat/repl` **首次用时若探测 localhost:3000 未起，则自动 `spawn` 一个 service 子进程并自动连接**（child 管理 + 退出时清理）。
- 代价：双进程生命周期管理（比 A 易出泄漏/端口占用问题），但**改动面更小**、风险更低、保住现有契约与 service 独立部署能力。

> 守门员倾向 **A**（真正"单二进制"是用户期望的本质）。若执行窗口评估 A 的 refactor 风险过高，可降级 B 并标注原因回顶层。

---

## 3. 必做项（按依赖排序）

### B-0 二进制改名 + 安装通道（产品级，优先）
- 把用户可见命令从 `codex` 改为 **`hearth`**（crate 内 `[[bin]] name = "hearth"`；保留旧 `codex` 别名可选）。
- 加 `Cargo.toml [profile.release]`：`opt-level=3 / lto=true / strip=true / codegen-units=1`（产出小且快的二进制）。
- 提供**安装方式**（任选实现，至少 1 种）：
  - **A. `cargo install --path crates/hearth-cli`**（Rust 用户标准，装完进 `~/.cargo/bin`，自动进 PATH）——最像开发者预期。
  - **B. 预编译二进制 + 安装脚本 `install.sh` / `install.ps1`**：`curl ... | sh` 式下载解压、移到 `/usr/local/bin`（Linux）或提示加入 PATH（Windows）——最像"下载安装"体验。
  - 二选一实现；推荐 A（零额外维护）+ 附带 B 的 `install.sh` 给非 Rust 用户。
- **全局 PATH 验收**：装完后新开终端在**非仓库目录**跑 `hearth --help` 成功（证明全局可用，非 `cargo run`）。

### B-1 零配置首次运行（UX 核心）
- `hearth` 启动时按优先级解析 LLM key：`--api-key` 参数 > `HEARTH_API_KEY` / `DEEPSEEK_API_KEY` 环境变量 > 首次交互引导（仅 `repl`/`chat` 交互模式提示输入，非 CI 模式直接报错指明显式方式）。
- 不再强制 `.env`（保留 `.env` 支持作为兼容，但不要求手写）。
- 若任何 provider 都无 key：`hearth chat` 直接打印**清晰错误**（"未配置 API key：用 `hearth --api-key sk-xxx chat ...` 或设环境变量 HEARTH_API_KEY"），不静默拿占位 key 去调（现状 `service` 用 `sk-placeholder` 是反例，须改）。

### B-2 自动拉起 / 自托管（按 §2 选中派）
- 派 A：内嵌 runtime，`hearth chat` 本进程跑内核。
- 派 B：`hearth chat` 探测 `localhost:3000`，未起则自动 spawn service 子进程（管理 child PID + 退出清理 + 端口冲突回退提示）。
- 两种派都必须保证：**用户无需手动起 service**。验收画面即 §1 的终端示例。

### B-3 首次运行引导（`hearth init` / 自动）
- 新增 `hearth init`：交互式配置（选 provider、填 key、存到 `~/.config/hearth/config.toml`），一键完成"下载安装后第一次上手"。
- `hearth chat` 若无配置，提示 `hearth init` 或 inline 引导（与 B-1 配合）。

### B-4 跨平台二进制（次，可选）
- 至少产出 **Linux x86_64** release 二进制（你主用）。
- 可选：GitHub Actions 或本地 `cross` 编译 **Windows x86_64**（PowerShell 可用）+ **macOS**（若你需要）。
- 注意：Windows 上沙箱 crate 走 `NoopSandbox`（真实隔离仅 Linux），安装脚本须区分平台提示"Windows 仅开发模式、无真实沙箱"。

---

## 4. 门禁（每批交证据）

- **R1（可静态验）**：`Cargo.toml` 含 `[profile.release]`；二进制 `name="hearth"`；install 脚本/ cargo install 路径存在。
- **R2（真实 Linux 验收·硬闸门）**：在**非仓库目录**新开终端，`hearth --help` 成功（全局可用）；`hearth chat "hello world"` 端到端跑通（无需手动起 service、无需手写 .env）。
- **R3（零配置）**：清空所有 env + 无 .env 时，`hearth chat` 给出清晰 key 缺失错误（非占位 key 静默失败）。
- **R4（跨平台·若做 B-4）**：Windows PowerShell 下 `hearth --help` 可运行（沙箱降级提示正确）。
- 通用：`cargo fmt --check` 0 / `clippy -D warnings` 0 / `test --workspace` 全过（baseline 240 + 新增）。

---

## 5. 红线（执行窗口不得违反）

- **禁**破坏 `docs/ai-os-event-contract-v1.md` 契约（前后端解耦唯一契约）——本任务只改"谁起内核"，不改事件结构。
- **禁**把"无 key"静默降级成占位 key 去调 LLM（现状 service 的反模式）——必须清晰报错。
- **禁**删已有 `codex-cli` 测试来过闸——只增不改，且 B-0 改名须同步更新测试里的二进制调用名。
- **禁**动 `codex-rust` 旧名语义（crate 内部名可保留，仅**用户可见命令**改 `hearth`）；与 `docs/hearth-naming.md` 定名一致。
- **禁**引入重依赖（如 reqwest 之外的 HTTP 栈、或安装框架）除非经顶层确认——保持零/轻依赖。
- 事实产生权：配置解析是 FE/CLI 行为，**不涉及后端事实产生**，本任务不触碰内核事实语义。

---

## 6. 验收回顶层

执行窗口交 B-0~B-4 证据后，回守门员独立验收（**必须真在 Linux 非仓库目录跑 `hearth --help` + `hearth chat` 端到端**，不读报告）。🔴=0 且 实现率≥0.9 过闸。
