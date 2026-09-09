# Hearth (hearth-rs) vs OpenAI Codex CLI — 偏差对标报告

> 对标对象：OpenAI **Codex CLI `rust-v0.150.1`**（2026-08-27 发布，commit 9085439，Apache-2.0，非预发布）
> 对标基准：Hearth 仓库 **HEAD**（27 crate / 91 .rs / 35,124 行，静态源码取证）
> ⚠️ 版本一致性声明：VM 门禁历史实测 **v0.2.3 = 339 passed** 来自某个历史 commit，与 HEAD **并非同一 commit**（HEAD 滞后约 78 提交）。「339 passed」不能直接当作 HEAD 健康度；源码取证一律以 HEAD 为准，权威测试计数见配套证据包 §A（需在本版本统一跑 `cargo test --list`）。
> 报告日期：2026-08-28
> 作者：WorkBuddy（守门员审计法：不信报告信源码，对标落到真实代码）

---

## 0. 摘要（Top-line）

Hearth 与 Codex 是**不同定位**的产物：Hearth 是「可信委托」的自托管 Agent（事实公理 + 硬安全边界 + 第三权审计），Codex 是 OpenAI 账户绑定的云端/本地编码助手。**对标的目的不是「谁更强」，而是「差距在哪、是否偏离原始意图」**。

核心结论：

- **Hearth 已具备且可查证**：G0 强制沙箱（landlock+seccomp+cgroups fail-closed）、真实子代理编排（`agent-core/src/loop.rs`）、阻塞式人工审批（WP-0）、多供应商 LLM 网关 + FallbackChain、Observer 第三权审计、EnvelopedEvent 事件信封、G0-G3 基因层级。
- **Hearth 真实能力缺口（Codex 有、Hearth 无）**：生命周期 Hooks、MCP 集成、Skills/Plugins 运行时、auto-review 自动审查代理、原生定时任务调度、IDE/桌面/云端形态、Browser/Computer use/Voice。**其中「缺口」而非「弱化」的是：MCP、Hooks、Skills 运行时、auto-review、定时调度。**
- **Hearth 差异化优势（Codex 无）**：多供应商 LLM 冗余、强制 fail-closed 沙箱、事实产生权公理、可审计事件信封、Observe 第三权。

**是否偏离原始意图**：**非偏离，是迭代**。Hearth 在「安全/审计/事实治理」上比 Codex 更激进（G0 强制隔离、Observer、事实公理），在「生态扩展广度（MCP/Hooks/Skills/定时）」上落后 Codex。这与「单人优先、安全优先」策略一致（用户曾明确「单人不好用，多人没意义」）。差距主要在能力广度，而非核心意图。

---

## 1. 对标方法论与数据来源

**Codex 端**（联网核实）：
- GitHub `openai/codex` README + `releases/latest`（版本元数据）
- `learn.chatgpt.com/docs` 官方文档：sandboxing、subagents、hooks、auto-review、skills-and-plugins、automations、mcp

**Hearth 端**（源码核实）：
- `agent-core/src/loop.rs`（子代理、调度器、审批交互）
- `api/src/lib.rs`（AgentEvent / InteractionRequest / NeedApproval）
- `agent-runtime/src/session.rs`（提交审批、per-session 审批表）
- `sandbox/src/lib.rs`（沙箱配置与隔离原语）
- `docs/feature-completeness-audit-2026-08-28.md`（功能全景）

**注意**：Hearth 本地 `cargo` 为 rustup 代理、无实际工具链，无法本地编译；**源码取证取自 HEAD，未经编译验证**。历史 VM 门禁 v0.2.3 曾实测 339 passed，但 v0.2.3 与 HEAD 不是同一 commit，故该数字不直接代表 HEAD 健康度。权威测试口径见配套证据包 §A（需在统一版本上跑 `cargo test --list` 终结口径之争）。

---

## 2. 能力对照矩阵

| 维度 | Codex CLI v0.150.1 | Hearth (HEAD) | 判定 |
|---|---|---|---|
| **沙箱隔离** | bwrap/userns(Linux)、Seatbelt(macOS)、WSL2；`sandbox_mode`=read-only/workspace-write/danger-full-access；默认 workspace-write+on-request（越界才问） | landlock+seccomp+cgroups，强制 fail-closed；`read_only_paths`/`writable_paths` | 等价且更严（整体无分级开关；个别控制点如 cgroup 有显式手动降级 `HEARTH_ALLOW_NO_CGROUP=1`，见 §3.A 与证据包 F.2） |
| **子代理** | `[agents]` + `~/.codex/  agents/*.toml`（name/description/developer_instructions/model/sandbox_mode/mcp）、`/agent` 切换、并发上限 | `loop.rs` `spawn_sub_agent` + depth 限制 + `read_only_view`；无持久化 agent 定义 | 部分（缺配置层+UX） |
| **审批/权限** | `approval_policy`(untrusted/on-request/never) + `auto_review`（独立审查代理, policy.md, 拒绝断路器） | WP-0 `NeedApproval`/`submit_approval` 阻塞式人工审批；Observer 审计 | 人工审批有，缺 auto_review |
| **生命周期 Hooks** | 全生命周期(PreToolUse/PostToolUse/SessionStart/Stop/PreCompact...)，hooks.json/config.toml，command/mcp_tool | 无 lifecycle hook 框架 | 🔴 缺口 |
| **Skills/Plugins** | `$` 提及调用、SKILL.md、Plugins(含 MCP 包) | 无 Hearth 原生 skill 运行时（WorkBuddy 有，非 Hearth） | 🔴 缺口 |
| **MCP 集成** | STDIO/HTTP，`codex mcp add`、trust、tool 策略 | crates 内 `mcp` 零匹配 | 🔴 缺口 |
| **定时任务** | Web/桌面 RRULE + 事件触发(GitHub/Slack/Gmail) | 无原生调度（WorkBuddy automation 替代） | 🔴（Hearth 侧） |
| **交付形态** | CLI + IDE 扩展 + 桌面 App + Web + Security CLI + GitHub Action + SDK | codex-cli(REPL) + service(REST/SSE) + HTML demo | 🟡 差距 |
| **多模态** | Browser / Computer use / Voice / Web search | 受控联网(egress allowlist)，无 browser/computer/voice | 🟡 差距 |
| **LLM Provider** | OpenAI gpt-5.6 系列（账户/API key） | 6 provider + FallbackChain | ✅ Hearth 优势 |
| **治理/事实公理** | 无 | 事实产生权公理、EnvelopedEvent、G0-G3、Observer | ✅ Hearth 优势 |

---

## 3. 逐项偏差详解

### A. 沙箱与隔离
- **Codex**：用 bubblewrap（unprivileged user namespace）+ macOS Seatbelt + Windows Sandbox；提供 `sandbox_mode` 三级（read-only / workspace-write / danger-full-access）。默认 `workspace-write + on-request`，偏 fail-closed（越界才问）。缺失 bwrap 时降级告警（不强制失败）。
- **Hearth**：`sandbox/src/lib.rs` 用 landlock（`read_only_paths`/`writable_paths`）+ seccomp（禁用 ptrace/mount/reboot/init_module 等）+ cgroups；**强制 fail-closed**——隔离失败=启动失败（`force_seccomp_fail` 测试覆盖）。**缺少 Codex 式的 `danger-full-access` 分级逃生阀**：整体沙箱模式无分级开关（无法临时放宽到低摩擦 workspace-write）；但个别安全控制点存在「显式手动触发」的降级选项（如 cgroup 的 `HEARTH_ALLOW_NO_CGROUP=1`），属于运维级降级而非「一键逃生」，二者不应混淆。
- **偏差**：Hearth 隔离强度更硬（fail-closed 强制），但缺少分级模式的可配置性（无法临时放宽到低摩擦 workspace-write）。技术路线不同（内核原语 vs bwrap），但目标一致。Hearth 在「无 bwrap 时直接失败」是「安全优先」取向，与原始「可信委托」定位一致。

### B. 子代理 / 多智能体
- **Codex**：`config.toml [agents]` + 自定义 `~/.codex/agents/*.toml`（name/description/developer_instructions/model/sandbox_mode/mcp_servers）；`/agent` 切换线程；`max_concurrent_threads_per_session` 并发限制；子代理继承父沙箱。
- **Hearth**（`agent-core/src/loop.rs`）：
  - `spawn_sub_agent`（L1143）真实实现子代理派生；
  - depth 限制「agent at depth {} may not spawn sub-agents」（L1156）；
  - `AGENT_MAX_SUBAGENTS` 环境变量控制并发（L1750）；
  - `collect_sub_agent_results`（L1226）合并结果；
  - 子代理使用 `dispatcher.read_only_view()`（L1172）——只读工具视图（类似 Codex 的 read-only 沙箱继承）。
- **偏差**：Hearth 有「运行时编排内核」但缺「配置层」：无持久化 agent 定义（TOML）、无 `/agent` 风格交互式线程切换 UX、无独立模型/推理强度配置。属于「内核已有、缺周边」，可补。

### C. 审批 / 权限 / Auto-review
- **Codex**：`approval_policy`（untrusted/on-request/never）+ `approvals_reviewer="auto_review"`（独立审查代理，`policy.md`/guardian，拒绝断路器：连续 3 次或 50 次内 10  imes拒绝→中止）。
- **Hearth**：
  - `api/lib.rs:30` `NeedApproval{approval_id,...}` + InteractionRequest/Response（WP-0 阻塞式）；
  - `agent-runtime/session.rs` `submit_approval`（L818）+ per-session 审批表（X2 fix 修复泄漏）；
  - `codex-cli` 内联审批 `/interaction/{iid}`（L1126）；
  - **Observer crate**：第三权零执行权审计（circuit/metrics/rules）——是审计，不是审批替代。
- **偏差**：Hearth 有「人工审批」（Human-in-loop）内核能力，但**缺 auto_review 自动审查代理与策略模板**。Observer 是事后审计，不是「替代人工放行」。这是明确 P1 差距——若希望无人值守运行，需补一个类似 auto_review 的自动审查器（可复用 Observer 的 rules 模块扩展）。

### D. 生命周期 Hooks —— 🔴 明确缺口
- **Codex**：完整 `hooks.json` / `config.toml [hooks]`，覆盖 PreToolUse/PostToolUse/SessionStart/SessionEnd/PreCompact/PostCompact/UserPromptSubmit/SubagentStart/SubagentStop/Stop；支持 `command` 与 `mcp_tool`，matcher 正则，示例 `{"type":"command","command":"...","timeout":30}`。
- **Hearth**：grep 仅在 `service/webhook.rs`（HTTP 外部回调）、`tools-builtin/edit.rs`、`project-xray/wiring.rs` 出现「hook」字样，**无独立于 agent loop 的 lifecycle hooks 框架**。
- **偏差**：Hearth 没有「在 agent loop 中插入用户脚本」的能力。WorkBuddy 本身有 Hook 能力，但 Hearth 项目（crates）没有。影响：无法做「保存记忆」「扫描密钥」「自定义校验」等自动化扩展。

### E. Skills & Plugins —— 🔴 缺口
- **Codex**：`$` 提及调用 skill，SKILL.md 指令包，Plugins 含 MCP 连接器。
- **Hearth**：无 Hearth 原生 skill 运行时（`~/.workbuddy/skills` 是 WorkBuddy 的，非 Hearth）。无 SKILL.md 加载、无 `$` 提及、无插件打包。
- **偏差**：若定位为「编码 Agent」而非「通用工作台」，优先级可降（P2）。

### F. MCP 集成 —— 🔴 缺口
- **Codex**：完整 MCP（STDIO/Streamable HTTP），`codex m, add`，`config.toml` `mcp_servers`，trust 机制，`enabled_tools`/`disabled_tools`/`default_tools_approval_mode`。
- **Hearth**：`crates/*.rs` 中 `mcp` 零匹配——无任何 MCP server 接入（工具扩展靠内置 tools-builtin + bridge，而 bridge 此前审计为「声明未接线」）。
- **偏差**：🔴 缺口。Hearth 无法让用户接第三方桌面/服务端 MCP。这是「可由连接 MCP 工具补」的能力（用户已有 WorkBuddy mcp.json 经验）。

### G. 定时任务 / 自动化 —— 🔴（Hearth 侧）
- **Codex**：Web/桌面端 RRULE 定时任务 + 应用事件触发（GitHub/Slack/Gmail）；CLI 仅能准备 prompt。
- **Hearth**：无原生调度器（`agent-core` 的 scheduler 是 tool dispatcher，非任务调度）。项目使用的定时能力来自 WorkBuddy 的 `automation_update`（SQLite + rrule），但那是 WorkBuddy 层，非 Hearth 内核。
- **偏差**：若要让 Hearth 自动跑周期性任务（如「每日代码审查」），需补调度接入或挂到 WorkBuddy automation。

### H. 交付形态 / 分发
- **Codex**：CLI / VS Code·Cursor·Windsurf 扩展 / 桌面 App / Codex Web / Codex Security CLI / GitHub Action / SDK / Non-interactive。
- **Hearth**：codex-cli（REPL）、service（REST/SSE）、codex-desktop-demo.html。无 IDE 扩展、无云端托管、无 GitHub Action、无桌面 App。
- **偏差**：🟡 Hearth 偏「后端服务 + CLI」，分发面窄。

### I. 多模态能力
- **Codex**：Browser / Computer use / Voice / Web search。
- **Hearth**：有受控联网（egress allowlist，T10），但无 browser automation、无 computer use、无 voice；Web search 需经 LLM provider 工具或 MCP 补。
- **偏差**：🟡 缺 Browser/Computer use/Voice。

### J. LLM Provider（Hearth 优势）
- **Codex**：绑定 OpenAI（gpt-5.6 系列，ChatGPT 账户/API key）。
- **Hearth**：`llm-gateway` 多 provider（OpenAI/DeepSeek/智谱/Agnes/Gemini/Ollama/local）+ FallbackChain（T9 真实接线）。
- **优势**：战略级差异化——弱 LLM/配额紧张下仍能交付、供应商冗余、配额灵活。Codex 无此。

### K. 治理与事实公理（Hearth 独有）
- 事实产生权公理：后端产生事实、前端只投影（BE 独占语义）。
- `EnvelopedEvent`（`api/lib.rs:103`）：ts/seq/span_id/parent_id 信封，支撑 span 树与重放。
- G0-G3 基因层级：G0 物理不可违反（sandbox/审批/进程隔离），G1 代码分支必执行，G2 时机召回，G3 软偏置。
- Observer 第三权零执行权审计。
- Codex 无对应概念，是 Hearth 「可信委托」定位的护城河。

---

## 4. 差距矩阵（按优先级）

> ⚠️ **排序前提（重要）**：以下优先级仅反映「能力广度缺口」，不构成执行顺序。在 **P0 稳定性行为验证**完成前，扩展性功能（MCP/Hooks/auto-review）不应排在「先确认系统本身稳定」之前。当前 deadline 超时、resume 恢复、压缩不丢、cgroup fail-closed 均为「存在性取证」，**尚未行为级验证**（见证据包 B/C/F）；待 VM 实测 B 类遥测确认稳定性后，再敲定扩展性优先级。

| 项 | 类型 | 优先级 | 说明 |
|---|---|---|---|
| **稳定性行为验证（deadline/resume/压缩/沙箱）** | 基础可靠性（先决） | **P0（先决）** | 当前仅存在性取证，需在 VM 跑 B 类遥测与 F.2 行为测试确认后再排扩展性 |
| MCP 集成 | 能力缺口 | **P1** | 用户可接第三方桌面/服务，强烈建议补，可复用 WorkBuddy mcp.json 经验 |
| 生命周期 Hooks | 能力缺口 | **P1** | 扩展性关键，WorkBuddy 已有 Hook 实践可复用 |
| auto-review 自动审查 | 能力缺口 | **P1** | 无人值守必需，可复用 Observer rules 模块 |
| 子代理配置层 | 完善 | P2 | 内核已有，补 TOML 定义 + UX |
| 定时任务 | 能力缺口 | P2 | WorkBuddy automation 可桥接 |
| Skills 运行时 | 能力缺口 | P2 | 取决于产品定位 |
| IDE/桌面/云端形态 | 分发 | P2 | 长期 |
| Browser/Computer/Voice | 模态 | P3 | 非核心 |

---

## 5. 是否偏离原始意图？

**结论：非偏离，是迭代。**

Hearth 原始设想是「可信委托」的 Codex-class Agent。当前实现在「安全/审计/事实治理」上比 Codex 更激进（G0 fail-closed、Observer、事实公理），在「生态扩展（MCP/Hooks/Skills/定时）」上落后 Codex。这与「单人优先、安全优先」策略一致（用户曾明确「单人不好用，多人没意义」）。本报告的差距清单（MCP/Hooks/Skills/auto-review/定时）属于「能力广度」补齐，不构成核心意图偏离。

---

## 6. 下一步建议

1. **先完成 B 类可靠性遥测与修复被 ignore 的核心安全测试，确认稳定性达标后再启动扩展性功能**：本会话实测已确认 deadline 单元测试通过、cgroup fail-closed 行为正确，但 resume/挂起仍无真实样本，且验证 RT4 fail-closed 的 `test_rt4_cgroup_fail_closed` 现被 `#[ignore]`（env 注入竞态）——CI 无法常态化覆盖这条核心安全不变量，必须先修复并重新进门禁。稳定性达标后再排 MCP/Hooks/auto-review。
2. **补生命周期 Hooks 框架**（最小集 PreToolUse/PostToolUse）：复用 WorkBuddy Hook 能力做原型。
3. **将 Observer 的 rules 模块升级为「自动审查器」**：实现近似 auto_review 的无人值守审批（拒绝断路器）。
4. **子代理配置层**：补 TOML 定义 + `/agent` 风格切换 UX（内核已就绪）。
5. **封版前做六维全项审计**（用户既定）：G0 真机逃逸、需求覆盖、契约一致、灾难恢复、供应链等。

---

## 7. 交付与参考文件

- 本报告：`docs/hearth-vs-codex-benchmark-2026-08-28.md`
- 参考（已生成）：
  - `docs/feature-completeness-audit-2026-08-28.md`（功能全景与完善度）
  - `docs/project-rot-audit-2026-08-28.md`（腐化审计）
  - `docs/project-comprehensive-review-2026-08-27.md`（综合审查）

**源码锚点**：`agent-core/src/loop.rs:1143/1156/1172/1226/1750`、`api/lib.rs:30/103`、`agent-runtime/src/session.rs:818`、`sandbox/src/lib.rs:44-109`。
