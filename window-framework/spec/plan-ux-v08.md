# 全局 UX 优化计划 v0.8 — 窗口群框架层落地（codex-rust vA 单人形态）

> **基线**：`docs/ux-boundary-definition.md`（全局 UX 边界，六面，零成本验证，不碰 G0/G1）
> **定位**：本计划是**全局 UX 战略**的一部分。UX 优化是端到端的，覆盖「底层 codex-rust（Rust agent）+ 窗口群框架（Python 编排）」两层。v0.8 先落地**窗口群框架层**的高优项（上手 / 反馈 / 边界感）；底层 codex-rust 的 UX 作为全局并行轨单列（见 §6，不在 v0.8 实现但纳入范围）。
> **约束**：不碰 G0/G1 本质；零成本验证（replay + 单测）；平台兼容（Windows 优先）。
> **版本线**：窗口群框架已 v1.0（139/139 全绿，维护期）。UX 版本线与框架版本线**独立**（ux-v08 → 未来 v1.0-UX），不混。

---

## §0 全局 UX 范围与命令命名

- **两层统一覆盖**：
  1. 底层 codex-rust：`codex-cli`（Rust 壳）→ `service`（HTTP 后端）→ agent 循环。其 UX（CLI 呈现 / 反馈 / 边界感 / 上手）属全局范围。
  2. 窗口群框架：命令名 **`wf`**（独立，避免与 `codex-cli` 冲突）。提供 project / window / workflow / analyze / import / export / template。
- **命令命名铁律**：禁止两个独立 `codex` 二进制抢 PATH。窗口群框架用 `wf`；未来若统一入口，由 `codex-cli` 分发 `codex windows ...` 子命令，但 v0.8 先用 `wf`。
- **双版本线**：框架 v1.0（工程层）与 UX 独立版本线（ux-v08 起）不混。

---

## §1 当前 UX 状态诊断（窗口群框架层）

框架有 20+ 条命令，但没有"人怎么用"的设计：

| 问题 | 现象 |
|---|---|
| **没有统一的 CLI 入口** | 每次 `python3 src/framework.py project create`——脚本路径、Python3 版本依赖 |
| **错误信息是 raw traceback** | YAML 解析失败 → Python 报错堆栈，不是"你的契约缺了 budget 字段" |
| **workflow 挂了看不出来** | `wf workflow watch` 轮询输出太技术化，不是"窗口 backend 在第 3 步卡住了——请检查 gate 脚本" |
| **没有 onboarding** | README 快速开始 5 步走完——但没人知道第一步要配什么 |
| **gate 暂停没有高亮** | workflow 停在 human gate → 输出一条 plain text，不是"⚠️ 需要你审核方案" |

> 注：底层 codex-rust（Rust agent）UX 诊断单列 §6，本计划 v0.8 不实现但纳入全局范围。

---

## §2 v0.8 三件事（窗口群框架层）

### 1. 上手面：4 步跑通第一个任务

目标：从第一次打开终端到 agent 完成第一个任务，**4 步内**。

```
Step 1: 安装
  # v0.8 临时：git clone + export PATH（提供 ./scripts/wf 包装避免长路径依赖）

Step 2: 配 key
  wf setup
  → 交互式：选 provider → 输入 API key → 写入 ~/.codex-projects/.env（权限 600，已 gitignore）
  → 默认【不联网验证】；`wf setup --verify` 才发一条测试请求（省钱，避免每次 setup 烧 API）

Step 3: 建项目 + 聊需求（默认 replay 模式，零成本）
  wf quickstart
  → 自动建 demo 项目 → 建需求窗口 → 注入预置对话 → analyze(replay) → deploy → watch
  → 默认【replay 模式，不调 LLM，零成本】；`wf quickstart --live` 才显式调真实 LLM
  → 出错自动回退 replay，并【显式打印回退原因】（禁假绿，边界硬约束）

Step 4: 看结果
  wf project status  # 可以看到所有窗口的状态
```

**可验证**：replay 模式 4 步跑通，不需要 LLM API key。

### 2. 反馈面：错误信息重写

当前的错误形态：

```
Traceback (most recent call last):
  File "framework.py", line 342, in _parse_yaml
    raise ValueError(...)
ValueError: invalid YAML
```

改写为人类可读的三段式：

```
❌ window analyze 失败：LLM 产出的配置格式不正确

原因：缺少 budget 字段（窗口 win-backend-01）
位置：需求窗口 win-req 第 18 轮对话 analyze

修复：
  1. 重新和需求窗口聊天，明确说明需要 budget 信息
  2. 或手动编辑 project.toml 补上 budget
  3. 重试：wf window analyze mini-blog win-req
```

**实现**：`framework.py` 新增 `UXError` 异常类，所有 `ValueError` / `RuntimeError` 用三段式包装：**发生了什么 / 原因 / 怎么修**。不碰框架逻辑，只改错误输出的格式。

**可验证**：replay 模式触发 **4 类**常见错误（缺 budget、环依赖、gate 脚本缺失、窗口数不合理）→ 每条错误输出含"修复"段。

### 3. 边界感面：gate 暂停 + 进度可视化

**workflow watch 输出改造**：

当前：
```
[17:30:01] design: win-arch-01 [pending]
[17:30:05] design: win-arch-01 [working]
[17:31:20] design: win-arch-01 [done]
[17:31:20] ⚠️  design gate: 人类审核方案 → 等待批准
```

改后（高亮 + 可操作提示 + Windows/tty 降级）：
```
━━━ workflow: mini-blog ━━━

  ✅ design       [done]     win-arch-01 (3min, 12轮)
  ⏸️  implementation [waiting] 等待你审核设计方案

     gate: 人类审核方案
     操作: wf workflow gate implementation --approve
           wf workflow gate implementation --reject "原因..."
  ⬜ review       [pending]

━━━━━━━━━━━━━━━━━━━━━━━
```

**实现**：
- `workflow watch` 输出用 Unicode box-drawing + 颜色（detect tty + 平台）
- **Windows 兼容**：检测平台，Win 默认 cmd 降级纯文本 / ASCII，不输出乱码（用户本机为 Win11，必须先保证不乱码）
- gate 暂停时输出 `--approve` / `--reject` 命令提示
- 非 tty / Windows 降级为纯文本（不破坏脚本调用）

---

## §3 阶段拆解

| 阶段 | 面 | 内容 | 预计 | 验收 |
|---|---|---|---|---|
| **S1** | 上手 | `wf setup`（配 key + 默认不验证，`--verify` 可选） | 0.5 天 | setup → 成功配 key（.env 600）→ demo 项目跑通 |
| **S2** | 上手 | `wf quickstart`（默认 replay，`--live` 显式；回退显式告知原因） | 0.5 天 | replay 模式跑通 + 出错打印回退原因 |
| **S3** | 反馈 | UXError 三段式错误重写（**4 类**常见错误） | 0.5 天 | 4 类错误含修复段 |
| **S4** | 边界感 | `wf workflow watch` 输出改造（box-drawing + 颜色 + Win/tty 降级 + 操作提示） | 0.5 天 | gate 暂停含 `--approve` 提示；Windows 不乱码 |
| **S5** | 全 | 测试：10 项覆盖（setup/quickstart/uxerror/watch 输出） | 0.5 天 | 全绿 + v0.1-v0.7 回归 |
| **S6** | 全 | 验收报告 v0.8 | 0.5 天 | 对照边界定义 §4 核对 |

---

## §4 验收红线

| 级别 | 判据 |
|---|---|
| 🔴 | `wf setup` 后 key 写入明文（非 .env），或 .env 权限非 600 / 被 git 跟踪 |
| 🔴 | replay 模式下 `quickstart` 不能跑通 |
| 🔴 | 错误消息只有 traceback 没有"修复"段 |
| 🔴 | gate 暂停时输出不含 `--approve` 命令提示 |
| 🔴 | 框架 4 断言 + v0.1-v0.7 回归 |
| 🔴 | 命令名仍为 `codex`（与 codex-cli 冲突）——必须 `wf` |
| 🟡 | 非 tty / Windows 环境 box-drawing 变成乱码（须降级纯文本） |
| 🟡 | `quickstart` 出错回退未显式告知原因（假绿） |

---

## §5 v0.8 之后（全局 UX 路线图）

- v0.9：呈现面 + 容错呈现（agent 思考过程可见 + 失败重试路径）——两层统一
- v0.9+：底层 codex-rust UX 轨（见 §6）
- v1.0-UX：六面全局落地，框架 UX 定版，进入维护期

---

## §6 底层 codex-rust UX 轨（全局范围，v0.8 不实现，单列待排期）

> 全局 UX 必须覆盖底层。以下属 codex-rust（Rust agent）UX，待 v0.9+ 或独立 plan，v0.8 仅列出范围：

- **指令面**：`codex-cli` 子命令梳理（与 `wf` 不冲突的命名空间）
- **呈现面**：agent 思考 / 工具 / 进度实时可见（当前 service 日志在 `/tmp`，用户不可见）
- **反馈面**：agent 失败（如审批拒绝、沙箱越界）的清晰报错
- **边界感面**：`codex-cli` 触发审批门时的高亮与放行 / 拒绝引导
- **上手面**：`codex-cli` 首次配置引导（.env / key）
- 注：以上不改 agent 逻辑，只改 CLI 呈现 / 提示 / 引导层，符合边界硬约束。

---

*注：本计划为全局 UX 战略的一部分，v0.8 落地窗口群框架层；底层 codex-rust UX 见 §6，不遗漏。*
