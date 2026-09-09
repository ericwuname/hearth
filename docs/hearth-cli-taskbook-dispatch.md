# Hearth CLI 开箱即用 — 执行窗口派遣任务书（派 A 单二进制）

> **签发**：顶层（守门员）| 日期：2026-08-22
> **设计依据**：`docs/hearth-cli-design.md`（已决策，非待选框架）
> **命名依据**：`docs/hearth-naming.md`（正名 Hearth，二进制 `hearth`）
> **前置已交付**：RT3 seccomp 硬化（`bcb2080`，验收 `3beee2e`）— 本任务 CLI 启动路径复用其 fail-closed 语义
> **性质**：架构 refactor（派 A 单二进制自托管）+ 安装 + 配置 + 安全透明 + Observer 主动反馈
> **用户最终期望**：项目安全平稳落地，使用过程不出现各种意想不到的报错/风险；VM Linux 跑（隔离风险 + 快）

---

## 0. 用户已拍板的决策（不得再问）

1. **派 A 单二进制，一步到位**：CLI 进程内直接跑内核，用户无感 service 存在。不做得不彻底的半成品。
2. **配置三层优先级**（高→低）：命令行参数 > 环境变量 > `~/.config/hearth/config.toml` > 内置默认。文件是真相源，`hearth config set` 子命令是改文件的界面。可改字段：**api-key / mode / url / provider**。
3. **主要 Linux x86_64**（用户真用）；Windows/macOS 仅当成本 <0.5 天顺带，且非 Linux 强制打印"⚠️ 无真实沙箱"徽章。
4. **安装以 `cargo install --path` 为主**；预编译 + `install.sh` 仅当成本 <0.5 天顺带。
5. **反馈通道主动非被动**：`hearth note`（随时零打扰）+ `hearth chat` 开头轻探（仅异常时出现且 `feedback-prompt=false` 可关）。不做被动末问三选一。
6. **Observer 观察人类侧**：消费 Human 行为流 + 用户 `--self`/`--observer-verdict` 双标注，三权互进（§6.4.5/§6.4.6）。

---

## 1. 本轮交付清单（按依赖排序）

### D1 抽 `agent-runtime` library（架构地基）
- 把 `crates/service` 的会话管理 / agent loop / LLM / 沙箱调用抽成 `crates/agent-runtime`（library crate）。
- `service` 改为 `agent-runtime` 的 **HTTP 包装层**（保留独立部署能力，不删）。
- **红线**（设计 §2.1）：不动 `docs/ai-os-event-contract-v1.md` 事件结构；不动 `crates/sandbox` 内部；不删 service 独立部署。

### D2 改名 + 安装 + 零配置
- 二进制名 `hearth`（crates/codex-cli 改名或新增 bin `hearth`，旧 `codex` 作为别名保留或删，与 `hearth-naming.md` 一致）。
- `Cargo.toml` 加 `[profile.release]`（strip + lto）。
- `cargo install --path crates/hearth-cli`（或等价）后，`~/.cargo/bin/hearth` 全局可用。
- 零配置：无 key 时 `hearth chat` 给**清晰 key 缺失错误**（非占位 key 静默失败——现状 service 用 `sk-placeholder` 是反模式，必须改）。
- 提供 `docs/hearth-cli-guide.md`（用法 + 排错 + 审批流程，给用户/外部顾问看）。

### D3 配置子命令 + `hearth init`
- `hearth config set provider <deepseek|openai|ollama|vllm>` / `set url <...>` / `set mode <...>` / `set api-key <k>` / `get <field>`。
- 配置文件 `~/.config/hearth/config.toml`（不在仓库、不进 git）。
- `hearth init` 交互式首次引导填 key（可选，非强制）。
- 环境变量覆盖：`HEARTH_API_KEY` / `HEARTH_URL` / `HEARTH_PROVIDER` / `HEARTH_MODE`（最高优先级为参数，其次 env，其次 toml）。

### D4 隔离徽章 + 错误可行动化（与 RT3 衔接）
- 启动打印 🔒 `landlock+seccomp (fail-closed)`（Linux 真隔离）或 ⚠️ `noop · 仅开发模式`（非 Linux）。
- 复用 RT3 的 fail-closed：seccomp/landlock 加载失败 → CLI **启动失败并报原因**，绝不让"假装隔离"出现。
- 所有已知失败路径（无 key / 模型 4xx / 网络不可达 / 沙箱加载失败 / 端口冲突）给"具体原因 + 下一步动作"，不抛裸 panic / 不吐 backtrace。

### D5 Observer 主动反馈 + 人类侧观察
- `hearth note "..."`（通用素材）+ `--session <id>` + `--verdict y/n` + `--self "人类侧自我标注"` + `--observer-verdict n "反审 Observer"` + `--mood angry/frustrated`。
- `hearth chat` 开头轻探：仅当上次 session 有异常信号（重试率高 / give_up / 审批高拒批）时出现；`hearth config set feedback-prompt false` 可关。
- Observer 消费 **AI 事件流 + Human 行为流**（§6.4.1 + §6.4.5 的 12 维度表），落盘 `~/hearth/observer/<session>.jsonl`（AI 侧）与 `~/hearth/observer/human-<session>.jsonl`（人类侧）。
- 红线：Observer 零执行权延伸至人类侧——只记、只回放、只建议，绝不自动改用户配置/内核。

### D6（可选，<0.5 天）跨平台 + 预编译安装器
- 跨平台编译 `hearth` for Windows/macOS（若有需求）；非 Linux 强制 ⚠️ noop 徽章。
- `install.sh`（`curl | sh` 下载预编译二进制到 `/usr/local/bin`）。

---

## 2. 并入：RT3 🟡 文档滞后遗留（次轮顺手修）

执行窗口在本任务**开头**先修 RT3 验收遗留（`docs/rt3-acceptance-record.md` 🟡 1/2），避免契约文档与源码长期不一致：
- `docs/seccomp-allowlist-v1.md` line 10 "77 个" → "99 个"；line 42 "包含全部 77 个" → "包含全部 99 个"。
- `crates/sandbox/src/lib.rs:610` 注释 "77 个实测必需 syscall" → "99 个"。
- 改完跑 `test_allowlist_covers_cargo_syscalls` 确认仍绿（契约与单测对齐）。

---

## 3. 门禁（硬闸门，守门员独立验收）

- **R1（静态）**：`Cargo.toml` 含 `[profile.release]`；二进制 `hearth` 存在；`agent-runtime` crate 存在且 `service` 改为其包装层（或保留独立部署）。
- **R2（真实 Linux·硬闸门）**：在 **VM 非仓库目录**新开终端，`hearth --help` 成功；`hearth chat "hello"` 端到端跑通（**无需手动起 service、无需手写 .env**）。
- **R3（零配置）**：清空所有 env + 无 config 时，`hearth chat` 给清晰 key 缺失错误（非占位 key 静默失败）。
- **R4（安全透明）**：Linux 启动打印 🔒 `landlock+seccomp` 徽章；非 Linux 打印 ⚠️ noop 徽章（若做 D6）。
- **R5（错误可行动）**：模拟"模型 4xx / 网络不可达"，断言 CLI 输出含具体原因 + 下一步，无裸 panic。
- **R6（Observer 素材·双侧）**：`hearth note` 各形态（含 `--self`/`--observer-verdict`/`--mood`）落盘 `~/hearth/observer/`；AI 侧 + human- 侧 jsonl 分别生成；开头轻探仅异常时出现且 `feedback-prompt=false` 可关。
- **R7（Observer 反审闭环）**：`hearth note --observer-verdict n "..."` 能纠偏 Observer 判定器；且 Observer 不得因反审自动改用户配置/内核。
- 通用：`fmt --check` 0 / `clippy -D warnings` 0 / `test --workspace` 全过（baseline 244 + 新增）。

---

## 4. 红线（执行窗口不得违反）

- 禁破坏 `docs/ai-os-event-contract-v1.md` 契约。
- 禁占位 key 静默降级（D2/D4）。
- 禁非 Linux 静默假装隔离（D4/D6）。
- 禁裸 panic / 未处理 backtrace 暴露给用户（D4）。
- 禁删 `service` 独立部署能力（仅 refactor 为 runtime 包装层）。
- 禁动 `codex-rust` 语义（仅用户可见命令改 `hearth`）；与 `docs/hearth-naming.md` 一致。
- 禁 Observer 越权：素材只记录/落盘/供查缺补漏，绝不自动改内核/回写控制流/改用户配置（零执行权铁律）。
- 禁夸大自动能力：不得声称"自动发现所有理解偏差/思考缺失/人类情绪逻辑矛盾"——须明示自动信号 + 用户标注两层互补（§6.4.3/§6.4.5）。
- 不引重依赖（保持轻量）。

---

## 5. 交付物（回顶层验收时提交）

- 架构 refactor 相关 commit（D1-D5，D6 可选）。
- `docs/hearth-cli-guide.md`（新增用法文档）。
- RT3 契约文档 77→99 同步 commit（D6 开头）。
- 验收报告 `docs/acceptance-hearth-cli-<日期>.md`（含 R1-R7 实测证据 + 红队/端到端日志）。

**节奏**：按 D1→D5 施工，每批跑门禁，交证据后回顶层（守门员）独立验收。RT3 已 PASS，本任务可立即开工。
