# 窗口群框架 v0.2 施工计划 — 窗口心智 + 快照 + 命令面补全

> 基线：v0.1 骨架 22/22 测试全绿（9 条命令 + 4 断言）
> 下一目标：不下猛药——v0.2 做三件事，每件都可独立验收。
> 独立项目，仍然不碰 codex-rust 主仓库。
>
> **v0.2.1 审查补充（2026-08-01）**：审查发现 5 个缺口，已并入下述施工内容——
> ① LLM key 来源、② 写路径沙箱限制、③ window.toml 缺 prompt 字段、
> ④ budget 双上限运行时追踪、⑤ S4 测试具体覆盖点。见 §v0.2.1 补充。

---

## v0.1 现状评估

| 有的 | 缺的 |
|---|---|
| 项目容器 + 目录骨架 ✅ | 窗口 **真的能跑 agent** 吗？❌ |
| 窗口生命周期（创建/启停/删/恢复） ✅ | 对话管理（对话历史存哪？怎么读？）❌ |
| 框架 4 断言自检 ✅ | 快照/回滚 ❌ |
| budget 声明 + gate 声明 ✅ | budget 运行时追踪（tokens 用了多少）❌ |
| 22 测试全绿 ✅ | 导入/导出/模板/workflow/压缩 ❌ |

**关键发现**：v0.1 是一个**文件管理器**——它管项目目录、窗口目录、窗口元数据。但它没有给任何窗口装上"脑子"。一个 `window.toml` 里有角色 prompt，但没有地方执行这个 prompt。

v0.2 不急着造压缩引擎和流转引擎——先让窗口 **能跑起来**。

---

## v0.2 三件事

### 1. 窗口 agent 启动（核心：让窗口有脑）

窗口当前只是一个目录 + 一个 `window.toml`。v0.2 让 `codex window start <id>` 真的启动一个 agent 对话：

```bash
codex window start win-backend-01 --provider deepseek
  # 1. 读 window.toml → 获取 role / prompt / budget / context
  # 2. 创建对话历史文件 windows/win-backend-01/conversation.jsonl
  # 3. 注入 system prompt（角色描述 + 产出路径 + 共享层只读路径）
  # 4. 启动 AgentLoop（或简化为单轮工具调用循环）
  # 5. window.toml state → working / current_tokens 实时更新
```

**最小可行方案**（不依赖 codex-rust agent-core）：
- `framework.py` 新增 `agent_run(window)` 函数
- 直接调 LLM API（DeepSeek OpenAI 兼容）→ system prompt + tool loop（bash/read/write）
- 每月/每次 stop 自动写 `conversation.jsonl` 追加行
- 对话历史格式：`{turn, role, content, tool_calls, tool_results}`

**验收**：`codex window start win-test-01` → 能完成一个简单任务（"在 outputs/ 里写一个 hello.txt"）→ `window.toml` 的 `current_tokens` 从 0 变成正数 → `state = done`。

### 2. 对话存储 + 简单导出

窗口的对话历史需要一个确定的存储格式：

```
windows/win-backend-01/
├── window.toml
├── conversation.jsonl         ← 增量追加，每行一条消息
└── outputs/                   ← 窗口产出文件
```

**文件格式**（JSONL，每行一条消息）：

```json
{"t": "2026-08-01T08:00:00Z", "role": "system", "content": "你是一个后端开发者..."}
{"t": "2026-08-01T08:00:05Z", "role": "user", "content": "请实现用户登录 API"}
{"t": "2026-08-01T08:00:10Z", "role": "assistant", "content": null, "tool_calls": [{"name": "read", "args": {"path": "shared/specs/api.md"}}]}
{"t": "2026-08-01T08:00:15Z", "role": "tool", "name": "read", "content": "...文件内容..."}
```

**导出命令**（execution-bridge 补丁1 里承诺了但 v0.1 没实现）：

```bash
codex window export win-backend-01 --format markdown > backend-chat.md
# 读 conversation.jsonl → 按轮次渲染为 markdown（tool_calls 折叠为摘要行）
```

**验收**：`codex window export` 产出可读的 markdown 文件。

### 3. 快照（轻量版——先做窗口级）

v0.1 有 `.snapshots/` 目录但没写功能。v0.2 做最简版：

```bash
codex window snapshot win-backend-01
  # 复制 window.toml + conversation.jsonl 到 .snapshots/win-backend-01--<timestamp>/

codex window rollback win-backend-01 --to <timestamp>
  # 从快照恢复 window.toml + conversation.jsonl
  # 当前对话历史移入 .snapshots/rolled-back/
```

**触发规则**（自动快照）：
- `window stop` 时自动快照
- 对话每 +50 轮自动快照

**验收**：快照 → 删几轮对话 → 回滚 → 对话恢复。

---

## v0.2 命令面新增

| 命令 | 功能 | 新增/补全 |
|---|---|---|
| `window start` | **真启动 agent**（v0.1 只改状态） | 核心新增 |
| `window stop` | 暂停 agent + 自动快照 | 增强 |
| `window export` | 导出对话为 markdown | 补全（execution-bridge 承诺项） |
| `window snapshot` | 手动快照 | 新增 |
| `window rollback` | 从快照恢复 | 新增 |
| `window status` | 显示 current_tokens / 对话轮数 | 新增子命令 |

---

## 阶段拆解

| 阶段 | 内容 | 预计 | 验收 |
|---|---|---|---|
| **S1** | agent 启动引擎（LLM API + 工具循环） | 1 天 | `window start` 能完成一个简单任务 |
| **S2** | 对话存储格式 + 导出 markdown | 0.5 天 | `window export` 产出可读文件 |
| **S3** | 快照 + 回滚 | 0.5 天 | 快照→删→回滚→恢复 |
| **S4** | 测试：10 个测试覆盖新功能 | 0.5 天 | 测试全绿 |
| **S5** | 验收报告 v0.2 | 0.5 天 | 对照设计 v2.1 核对 |

---

## 验收红线

| 级别 | 判据 |
|---|---|
| 🔴 | `window start` 后 agent 不能完成"写一个 hello.txt"（端到端断裂） |
| 🔴 | 框架 4 断言退化 |
| 🔴 | 对话数据丢失（停止后 conversation.jsonl 为 0 字节） |
| 🟡 | `window export` 不可读（原始 JSON 直接输出不算） |

---

## §v0.2.1 审查补充（5 个缺口修正）

> 审查 `plan-v02.md` v0.2 与 `execution-bridge-v22.md` v1.1 补丁6/7、设计 v2.1 §8.1/§8.7 的一致性。
> 以下 5 项必须随 v0.2 一起施工，否则 agent 引擎无法落地。

### 补充 1：LLM API key 来源（防硬编码）

```
key 读取优先级（对齐 codex-rust v21 密钥纪律——仓库零硬编码）：
  1. 环境变量 DEEPSEEK_API_KEY（推荐）
  2. 项目根 .env（window-framework/.env，gitignore）
  3. 无 key → 报错 "DEEPSEEK_API_KEY not set" + 退出码 1（不静默降级）

绝不写死 key 到 framework.py / window.toml / conversation.jsonl
```

### 补充 2：写路径沙箱限制（v2.1 §8.7 落地）

agent 工具循环的 write/read 必须限制路径（防窗口间污染）：

```
可写：{window_dir}/outputs/  +  {project}/shared/outputs/（产出落点）
只读：{project}/shared/（specs/decisions/progress）
禁写：其他窗口目录 / .trash / .archive / .snapshots / project.toml / window.toml

实现：agent_run() 里 write_file 工具封装一个 path 校验器：
  resolve(path) 必须在允许前缀内，否则返回 "DENIED: path outside window outputs"
```

### 补充 3：window.toml 增加 prompt 字段（补丁6 契约对齐）

v0.1 的 window.toml 模板**缺 prompt 字段**——agent 启动读不到角色描述。修正：

```toml
[window]
id = "win-backend-01"
name = "后端开发"
role = "backend-developer"
prompt = "你是博客后端开发者。根据架构方案实现 API..."   # v0.2 新增
state = "pending"
```

`window create` 加 `--prompt` 参数；缺 prompt 的窗口 `window start` 报错（不给"没脑子的窗口"发 token）。

### 补充 4：budget 双上限运行时追踪（v2.1 §8.1）

plan 只提 current_tokens，补齐双上限 + 超限停：

```
每次 tool 调用后更新 window.toml [budget]：
  current_tokens += 本次用量（LLM 响应 usage）
  current_cost += 估算成本（tokens × 单价）
超限行为（二选一触发即停）：
  current_tokens > context.max_tokens      → 停，state=done（对话太长）
  current_cost  > budget.max_cost_cny      → 停，state=blocked（成本超限，等人类）
```

### 补充 5：S4 测试具体覆盖点（10 个测试清单）

| # | 测试 | 覆盖 |
|---|---|---|
| 1 | `test_agent_writes_hello` | S1 红线：agent 完成简单任务 |
| 2 | `test_conversation_appended` | 对话追加 conversation.jsonl 非空 |
| 3 | `test_current_tokens_updated` | current_tokens 0→正数 |
| 4 | `test_state_done` | 完成后 state=done |
| 5 | `test_export_markdown_readable` | export 产出含 role/时间/内容 |
| 6 | `test_snapshot_created` | snapshot 复制 window.toml+conversation |
| 7 | `test_rollback_restores` | 删对话→回滚→恢复 |
| 8 | `test_rollback_preserves_current` | 当前历史移入 rolled-back/ |
| 9 | `test_write_path_denied` | 补充2：写窗外路径被拒 |
| 10 | `test_no_key_errors` | 补充1：无 key 报错退出码 1 |

---

## v0.3 预留（本轮不做）

- 上下文压缩（v0.2 有了对话历史后才能做）
- 工作流引擎
- 需求窗口分析 → 自动建窗
- 导入外部对话
- 共享层冲突仲裁
- 回归测试（`framework check` 4 断言 + 应力场预留）