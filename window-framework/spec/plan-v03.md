# 窗口群框架 v0.3 施工计划 v0.3.1 — 工作流引擎 + 需求自动建窗

> 基线：v0.2 42/42 全绿（agent 真跑、沙箱、快照、导出）
> v0.2 给了窗口一个脑子。v0.3 让它**不是一个人在干活**。
>
> **v0.3.1 审查补充（2026-08-01）**：审查发现 4 个缺口——需求契约缺 budget/outputs/gate 字段
> （v1.1 补丁6 强制）、gate 仍自然语言（v2.1 §8.3 脚本化）、S4 测试无覆盖清单、
> workflow 引擎与 LLM 耦合（测试成本）。已并入 §v0.3.1 补充。

---

## v0.3 核心命题：从"手动管窗"到"自动流转"

v0.2 的窗口需要人一个一个 `codex window start`。你需要的是一个需求窗口分析完、所有后续窗口自动创建、自动按顺序启动——人只在 gate 点审核。

v0.3 把这个手动环节去掉。

---

## 三件事

### 1. 工作流引擎（设计 §5 落地）

v0.2 有了 `project.toml` 里的 `[workflow]` 占位段。v0.3 让它真正执行：

```yaml
# project.toml 里的流转规则（需求窗口产出或人手写）
[workflow]
[[workflow.stages]]
id = "design"
trigger = "project_start"
windows = ["win-arch-01"]
gate = "shared/gates/arch-gate.sh"

[[workflow.stages]]
id = "implementation"
trigger = "design:done"
windows = ["win-backend-01", "win-frontend-01"]
parallel = true
gate = "shared/gates/impl-gate.sh"

[[workflow.stages]]
id = "review"
trigger = "implementation:done"
windows = ["win-review-01"]
gate = "shared/gates/review-gate.sh"
```

**引擎逻辑**（新增 `framework.py` 的 `WorkflowEngine` 类）：

```
codex workflow start
  → 读 project.toml 的 [workflow] 段
  → 找到 trigger 为 project_start 的 stage
  → 启动该 stage 的所有窗口（并行或串行）
  → 轮询窗口状态
  → 全部 done → 执行 gate 脚本
  → gate 通过 → 标记 stage:xxx:done → 触发下一 stage
  → gate 失败 → 标记 stage:xxx:blocked → 等人类
```

**验收**：手动建 3 个窗口（arch / backend / review），配流转规则，`codex workflow start` → arch 自动启动 → arch done → backend 自动启动 → gate → review。

### 2. 需求窗口分析 → 自动建窗（补丁2 落地）

这是"先建一个窗口聊需求、聊完自动分析需要什么窗口"的完整闭环：

```bash
codex window create --name "win-requirement" --role "需求分析" --prompt "..."
codex window start win-requirement
# 人在这个窗口里聊项目需求、约束、目标
# 聊完后人对需求窗口说"可以了，输出窗口配置"

# 需求窗口输出 YAML →
# framework.py 新增 `workflow deploy` 解析 YAML → 自动建窗

codex workflow deploy win-requirement
  # 1. 读 win-requirement 最后一条 assistant 消息
  # 2. 解析其中的 YAML（§操作契约见下）
  # 3. 为 YAML 中的每个 window 调用 window create
  # 4. 把 workflow.stages 写入 project.toml
  # 5. 为每个 window 生成对应的 gate.sh 占位脚本
```

**需求窗口的输出契约**（框架只认这个格式；v0.3.1 补 budget/outputs/gate，对齐 v1.1 补丁6）：

```yaml
project_type: software
goal: "搭建博客后端 API"
windows:
  - id: "win-arch-01"
    role: "架构设计"
    prompt: "你是博客项目的架构师..."
    depends_on: []
    budget:                      # v0.3.1: 必填（缺则建窗被拒，§8.1）
      max_steps: 20
      max_cost_cny: 0.2
      provider: "deepseek"
    outputs: ["shared/outputs/arch.md"]      # v0.3.1: 必填
    gate: "shared/gates/arch-gate.sh"        # v0.3.1: 必填 .sh 脚本（§8.3）
  - id: "win-backend-01"
    role: "后端开发"
    prompt: "你是后端开发者..."
    depends_on: ["win-arch-01"]
    budget:
      max_steps: 40
      max_cost_cny: 0.5
      provider: "deepseek"
    outputs: ["src/"]
    gate: "shared/gates/backend-gate.sh"
workflow_stages:
  - id: "design"
    trigger: "project_start"
    windows: ["win-arch-01"]
    gate: "human:人类审核方案"     # v0.3.1: human: 前缀 = 人工审核点
  - id: "implementation"
    trigger: "design:done"
    windows: ["win-backend-01"]
    gate: "auto:shared/gates/impl-gate.sh"   # v0.3.1: auto: 前缀 = 脚本 gate
```

**验收**：人在需求窗口聊完项目 → `codex workflow deploy` → 自动创建 arch+backend 两个窗口 + 写入流转规则 → `codex workflow start` → 自动按次序启动。

### 3. v0.2 收尾：自动快照触发

v0.2 的快照命令有了，但自动触发（stop 时/50 轮）没做。v0.3 补上：

- `window stop` 时自动调用 `window snapshot`
- agent 每 50 轮自动快照（在 agent_run 循环中计数）

**验收**：`window start` 跑完 → `window stop` → `.snapshots/` 里自动出现一条快照。

---

## 阶段拆解

| 阶段 | 内容 | 预计 | 验收 |
|---|---|---|---|
| **S1** | 工作流引擎（WorkflowEngine 类） | 1 天 | 3 窗口流转全自动 |
| **S2** | `workflow deploy` 命令（YAML 解析 → 自动建窗） | 0.5 天 | 一条命令创建全窗口群 |
| **S3** | 自动快照触发 | 0.5 天 | stop 自动快照 / 50 轮自动快照 |
| **S4** | 测试：15 个测试覆盖新功能 | 0.5 天 | 全绿 + v0.1/v0.2 回归 |
| **S5** | 验收报告 v0.3 | 0.5 天 | 对照设计 v2.1 核对 |

---

## 验收红线

| 级别 | 判据 |
|---|---|
| 🔴 | `workflow start` 后窗口未自动启动（依赖解析失败） |
| 🔴 | gate 脚本 exit 0 但 workflow 未推进到下一 stage |
| 🔴 | `workflow deploy` 解析 YAML 后窗口数 ≠ YAML 声明数 |
| 🔴 | 框架 4 断言 + v0.1/v0.2 回归 |
| 🟡 | 串行 window 在前一个 done 之前被启动（并行/串行错乱） |

---

## §v0.3.1 审查补充（4 个缺口修正）

> 审查 `plan-v03.md` v0.3 与 `execution-bridge-v22.md` v1.1 补丁6/7、设计 v2.1 的一致性。
> 以下 4 项随 v0.3 一起施工。

### 补充 1：workflow deploy 契约强制校验（对齐 v1.1 补丁6）

`workflow deploy` 解析需求 YAML 时，每个 window 必须有：

```
必填：id / role / prompt / budget(max_steps,max_cost_cny,provider) / outputs / gate(.sh)
缺任一 → 报错 "window X 缺字段: ..." + 退出码 1（不建半成品窗口）
gate 必须以 .sh 结尾（§8.3 脚本化）；人类 gate 用 "human:" 前缀标记，不建脚本
```

框架 `framework check` 的 `budget-declared` / `gate-script-exists` 断言自动覆盖
deploy 产出的窗口（deploy 后必须 framework check 全绿才算成功）。

### 补充 2：双 gate 语义（对齐 v1.1 补丁6）

```
workflow_stages[].gate 分两类（前缀区分）：
  "human:..."   → 人类审核点（workflow 停在 stage，等 codex workflow gate <id> --approve）
  "auto:path"   → 脚本 gate（exit 0 = 通过，非 0 = blocked 等人类）
无前缀 → 默认按 auto 处理，但要求路径存在（否则报错）
```

### 补充 3：AGENT_MODE 测试模式（防测试烧 LLM token）

workflow 引擎自动启动窗口 → 窗口跑真 agent（调 DeepSeek，每次 ~¥0.01）。**测试环境不能烧**：

```
环境变量 AGENT_MODE=replay（默认 real）：
  replay 模式：window start 不调 LLM——立即标 done + 写一条假 conversation 记录
  real 模式：正常调 LLM（生产行为）
S4 测试全部用 replay 模式（workflow 流转测试 0 成本）
验收红线 S1（workflow start 自动流转）用 replay 验证逻辑正确性；
真 agent 端到端已在 v0.2 验收过，v0.3 不重复烧
```

### 补充 4：S4 测试 15 项清单

| # | 测试 | 覆盖 |
|---|---|---|
| 1 | `test_workflow_reads_stages` | 引擎解析 project.toml [workflow] |
| 2 | `test_workflow_starts_first_stage` | trigger=project_start 窗口自动启动 |
| 3 | `test_workflow_serial_dependency` | 串行：backend 在 arch done 前不启动（红线 🟡） |
| 4 | `test_workflow_gate_auto_pass` | gate 脚本 exit 0 → 推进下一 stage（红线 🔴） |
| 5 | `test_workflow_gate_auto_fail` | gate 非 0 → stage blocked 等人类 |
| 6 | `test_workflow_human_gate` | human: gate → 停在 stage 等 approve |
| 7 | `test_workflow_parallel_stage` | parallel=true 两窗口同时 working |
| 8 | `test_deploy_creates_windows` | deploy YAML → 窗口数 = YAML 声明（红线 🔴） |
| 9 | `test_deploy_writes_workflow` | stages 写入 project.toml |
| 10 | `test_deploy_generates_gate_scripts` | 自动 gate 的 .sh 占位生成 |
| 11 | `test_deploy_rejects_missing_budget` | 缺 budget → 拒绝（补充1） |
| 12 | `test_deploy_rejects_text_gate` | gate 非 .sh → 拒绝（补充1） |
| 13 | `test_auto_snapshot_on_stop` | window stop → .snapshots 自动快照 |
| 14 | `test_auto_snapshot_every_50` | 对话 50 轮自动快照 |
| 15 | `test_framework_check_after_deploy` | deploy 后 4 断言全绿（补充1 闭环） |

---

## v0.4 预留（本轮不做）

- 上下文压缩（§3）
- 共享层冲突仲裁（§8.4）
- 导入外部对话
- 模板系统
- 需求窗口的 LLM 自主分析（v0.3 的 YAML 契约已定好——但需求窗口自己能不能产出规范的 YAML？这需要长对话 + 压缩先就绪——留 v0.4）