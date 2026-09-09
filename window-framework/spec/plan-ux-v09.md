# 窗口群框架 UX 层 v0.9 施工计划 v0.9.1 — 呈现面 + 容错呈现 + polish

> 基线：v0.8 前三面覆盖（上手/反馈/边界感），还剩两面 + 整体 polish
> UX 边界 §5 优先级：呈现面 > 容错呈现面
>
> **v0.9.1 审查补充（2026-08-01）**：审查发现 5 个缺口——S4 测试无清单、StepLog 存储位置未定义、
> window run 与 start/watch 关系未理清、last_analyze 写入时机未定义、resume 语义边界未定义。
> 已并入 §v0.9.1 补充。

---

## §1 两个面——现在窗口是个黑箱

UX 六个面里最难的是呈现面——因为之前的版本都是在后台跑 agent，人不知道 agent 在里面干什么。

| 缺失 | 现象 |
|---|---|
| **agent 思考过程不可见** | window start 后等 30-120 秒，中间没有任何输出。agent 在 grep？在 read？在 write？不知道 |
| **进度没有粒度** | 只知道 `[working]` 或 `[done]`，不知道 agent 在第几轮、用了多少 token、还剩多少预算 |
| **工具调用不可见** | agent 调了 write_file、然后 cargo test 挂了、然后重试——这一切人在终端上看不到 |
| **失败后没有"从哪里重试"** | analyze 失败需要重新 run，丢失窗口的半截对话 |

---

## v0.9 三件事

### 1. 呈现面：`window status` 实时输出

`window start` 目前是阻塞或后台运行。v0.9 加一个实时状态流——不是 WebSocket 或复杂事件系统，是**每轮结束时 framework 抓 snapshot 并渲染一行**：

```bash
codex window run win-backend-01 --verbose
# --verbose 模式：每轮 LLM 调用后输出一行

# 输出示例：
━━ win-backend-01 后端开发 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
[1/20]  💭 Plan    → 拆解任务为 3 步       (~2s,  ~200 tokens, ¥0.002)
[2/20]  🔍 Read    → shared/specs/api.md     (~4s)
[3/20]  ✏️  Write   → src/auth.rs (+85行)    (~3s)
[4/20]  🔧 Bash    → cargo test              (~8s)
[5/20]  ❌ FAIL    → tests/auth_test.rs:12    (assertion error)
[6/20]  💭 Plan    → 定位测试错误             (~2s)
[7/20]  ✏️  Edit    → src/auth.rs (改 1 行)  (~3s)
[8/20]  🔧 Bash    → cargo test              (~6s)
[9/20]  ✅ DONE    → gate 通过(3 tests)      (total 28s, ¥0.015)
```

**实现**：
- `Agent.run` 循环每轮结束产生一个 `StepLog{turn, phase, tool, summary, tokens, wall_ms}`
- `window run --verbose` 渲染 StepLog 为上述彩色输出
- 非 tty / `--quiet` 模式输出纯文本一行
- `codex window run` 合并 `start` + `watch` 的语义——不需要两条命令

**验收**：replay 模式下跑一个 10 轮的 demo 窗口 → 输出包含 Plan/Read/Write/Bash/DONE 步骤。

### 2. 容错呈现面：失败后的"从哪里重试"

| 场景 | v0.8 当前 | v0.9 容错 |
|---|---|---|
| analyze 失败 | error + 修复建议 → 需要重新 run | 保留 analyze 输出到 `.snapshots/last_analyze.json` → `codex workflow deploy --last` 直接用上次输出 |
| window agent 中断 | stop 后需要重新 start | `codex window resume`：从 last conversation.jsonl 继续，不重跑已完成步骤 |
| workflow stage 挂 | 整个 workflow redo | `codex workflow retry <stage>`：只重跑该 stage，之前 stage 产物不动 |

**实现**：
- `agent run` 每轮结束后写入 `conversation.jsonl`（已有）→ `resume` 读最后一条继续
- `analyze` 结果写入 `.snapshots/last_analyze.json` → `deploy --last` 直接复用
- `workflow retry <stage>`：mark 该 stage 的所有窗口为 pending，起 workflow

**验收**：`window run` 中途 Ctrl+C → `window resume` → 从断点继续，不从头开始。analyze 成功 → Ctrl+C 中断 deploy → `deploy --last` 不重新 analyze。

### 3. Six-face polish：`codex status` 全局命令

框架目前没有一条命令能回答"我的项目现在怎么样了"。v0.9 加一个：

```bash
codex status
# 输出：
━━ my-blog (software) ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  项目窗口: 4（3 done / 1 working）

  ✅  win-req       需求分析   done      (20轮, 3.2K tokens)
  ✅  win-arch-01   架构设计   done      (12轮, 1.8K tokens)
  🔄 win-backend-01 后端开发   working   (8/40步, 1.2K tokens, ¥0.008)
  ⏸️  win-review-01  代码审查   waiting   (等待 gate: 人类审核)

  当前阶段: implementation → review（等待你审核方案）
  gate 操作: codex workflow gate review --approve
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

不比 `codex project status` + `codex workflow watch` 分开看——一条命令全部聚合。

---

## 阶段拆解

| 阶段 | 面 | 内容 | 预计 |
|---|---|---|---|
| **S1** | 呈现 | `window run --verbose`（StepLog + 彩色渲染） | 0.5 天 |
| **S2** | 容错 | `window resume` / `deploy --last` / `workflow retry` | 0.5 天 |
| **S3** | polish | `codex status` 全局聚合命令 | 0.5 天 |
| **S4** | 全 | 测试：12 项覆盖（verbose 输出/ resume 断点/ status 聚合/ 降级） | 0.5 天 |
| **S5** | 全 | 验收报告 v0.9 + UX 六面总结 | 0.5 天 |

---

## 验收红线

| 级别 | 判据 |
|---|---|
| 🔴 | `window resume` 后重跑已完成的步骤（非断点续传） |
| 🔴 | `--verbose` 输出不含工具调用名称（仍黑箱） |
| 🔴 | `codex status` 显示的窗口状态与实际 `window.toml` 不一致 |
| 🔴 | 框架 4 断言 + v0.1-v0.8 回归 |
| 🟡 | 非 tty 环境 `--verbose` 输出仍为 raw traceback（需降级纯文本） |

---

## §v0.9.1 审查补充（5 个缺口修正）

> 审查 `plan-ux-v09.md` v0.9 与现有代码（conversation.jsonl 格式 / window start/watch / deploy）的一致性。
> 以下 5 项随 v0.9 一起施工。

### 补充 1：StepLog 持久化到 conversation.jsonl 的 meta 字段

StepLog 不只是渲染——resume 需要它。持久化方案：

```
conversation.jsonl 每条消息追加 meta.step_log：
  {"t":..., "role":"assistant", "content":..., "tool_calls":[...],
   "meta": {"step_log": {"turn": 3, "phase": "Act", "tool": "read",
            "summary": "shared/specs/api.md", "tokens": 350, "wall_ms": 4200}}}
```

- Agent.run 每轮结束更新当前轮消息的 meta（追加写：先写消息再更新 meta 行）
- `window run --verbose` 读 conversation.jsonl 的最后 N 条 meta.step_log 渲染
- `window resume` 读最后一条完整 step_log 的 turn → 从 turn+1 继续

### 补充 2：window run 保留 start/watch 兼容

```
window run = start（跑 agent）+ --verbose（渲染 StepLog）的 UX 增强
  start/watch 保留不废弃（脚本兼容），run 是交互层新命令
window run 不带 --verbose = 等同 window start
window run --verbose = start + 实时 StepLog 渲染
```

### 补充 3：last_analyze.json 写入时机

```
window analyze 无论 --confirm 与否，都写 .snapshots/last_analyze.json：
  {"ts":..., "windows":[...], "workflow_stages":[...], "valid": true/false}
  - analyze 成功 → valid=true（--confirm 额外写对话）
  - analyze 失败 → valid=false + error 字段（保留失败快照供排查）
workflow deploy --last → 读 last_analyze.json（valid=true）→ 直接建窗
  --last 与指定 window 互斥（给了 window 参数 → 用对话；给了 --last → 用快照）
```

### 补充 4：resume 跳过不完整轮

Ctrl+C 中断时最后一条 assistant 消息可能半截（LLM 响应没写完）：

```
window resume：
  1. 读 conversation.jsonl 最后 3 条
  2. 若最后一条 assistant 无 tool_calls 且无 content（空响应）→ 视为中断残留，丢弃
  3. 从最后一条【完整】step_log 的 turn+1 继续
  4. 保留已完成的产出文件（不重做）
  5. state: blocked → working（resume 语义）
```

### 补充 5：S4 测试 12 项清单

| # | 测试 | 覆盖 |
|---|---|---|
| 1 | `test_run_verbose_steps` | --verbose 输出含 Plan/Read/Write/Bash/DONE（红线 🔴） |
| 2 | `test_run_quiet_plain` | 非 tty/--quiet → 纯文本单行（红线 🟡 降级） |
| 3 | `test_step_log_persisted` | conversation.jsonl meta.step_log 存在（补充1） |
| 4 | `test_run_equals_start` | run 不带 verbose = start 行为 |
| 5 | `test_resume_from_turn` | 断点续传：turn+1 开始，不重跑（红线 🔴） |
| 6 | `test_resume_skips_incomplete` | 空 assistant 残留丢弃（补充4） |
| 7 | `test_resume_keeps_outputs` | 已完成产出不重做 |
| 8 | `test_deploy_last` | deploy --last 用快照建窗 |
| 9 | `test_deploy_last_valid` | last_analyze invalid → 拒绝 |
| 10 | `test_workflow_retry_stage` | retry 只重置该 stage 窗口 |
| 11 | `test_status_aggregate` | status 输出与 window.toml 一致（红线 🔴） |
| 12 | `test_framework_check_after_all` | 全流程后 4 断言全绿 |

---

## v0.9 之后：UX 定版

```
v0.8 上手 + 反馈 + 边界感   ████████░░░░
v0.9 呈现 + 容错 + polish  ████████████
                           六面全闭，UX 进入维护期
```

UX 维护期 = 保持"上手面 4 步能跑通 / 反馈面 100% 有修复段 / 呈现面 agent 思考可见"三条红线不退。新 UX 需求走"发现→提案→验收"路径，不主动加。