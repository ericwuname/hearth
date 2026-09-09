# 窗口群框架 v0.7 施工计划 v0.7.1 — 结构化输出 + 流转稳定 + 最终集成

> 基线：v0.6 120/120 全绿，冲突仲裁落地，集成测试暴露真实 LLM 6 个 bug 已修。
> 集成测试的诚实结论：analyze→deploy 链路能通，但 LLM 输出的 YAML 需要 9 次迭代才成功一次。
> 核心问题不是"解析器不够宽容"——是"让 LLM 输出自由文本本身就是不可靠的"。
> codex-rust 锻造九轮里学到的东西：**结构化输出（function calling）替代自由文本解析**。
>
> **v0.7.1 审查补充（2026-08-01）**：审查发现 5 个缺口——function calling schema 缺 budget/outputs
> 字段（v1.1 契约强制）、replay 模式无 function calling 假响应、S5 测试无清单、
> workflow notify 机制未定义（轮询 vs 事件驱动）、降级路径无测试。已并入 §v0.7.1 补充。

---

## §1 起点：集成测试暴露的两个真实缺口

| 缺口 | 现象 | 根因 |
|---|---|---|
| **LLM 产出不可靠** | 9 次迭代才 deploy 成功。YAML 缺引号、无缩进、gate 非 .sh、重复 role | YAML 自由文本解析 vs LLM 输出多样性——本质对抗 |
| **工作流真 agent 卡 gap** | 窗口创建成功，但流转启动和 agent 完成判定有缺口 | replay 测试不触发真实 LLM → 掩盖了状态和时序问题 |

---

## §2 v0.7：两个核心修复

### 1. 结构化输出：function calling 替代 YAML 解析

**当前**（v0.5/v0.6）：
```
LLM → 自由文本 YAML → 极简解析器 → 9 次迭代 → 成功 1 次
```

**v0.7**：
```
LLM → function calling（JSON schema 约束）→ 100% 可解析 → 直接建窗
```

**怎么实现**：不改变 "window analyze" 命令的用户接口，改变的是发给 LLM 的方式。

```python
# 旧：自由文本 prompt → 等 LLM 回文本 → try 解析 YAML
response = llm.chat(prompt="输出 YAML ...", messages=conv)
yaml_text = extract_yaml_from_text(response)  # 脆弱
data = parse_yaml(yaml_text)  # 9/10 失败

# 新：function calling → LLM 填结构 → 直接拿数据
tools = [{
    "type": "function",
    "function": {
        "name": "output_project_config",
        "description": "产出窗口配置",
        "parameters": {
            "type": "object",
            "required": ["project_type", "goal", "windows", "workflow_stages"],
            "properties": {
                "project_type": {"type": "string", "enum": ["software", "writing", "data", "research", "ops"]},
                "goal": {"type": "string"},
                "windows": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["id", "role", "prompt"],
                        "properties": {
                            "id": {"type": "string", "pattern": "^win-[a-z]+-\\d+$"},
                            "role": {"type": "string"},
                            "prompt": {"type": "string"},
                            "depends_on": {"type": "array", "items": {"type": "string"}}
                        }
                    }
                },
                "workflow_stages": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["id", "trigger", "windows", "gate"],
                        "properties": {
                            "id": {"type": "string"},
                            "trigger": {"type": "string"},
                            "windows": {"type": "array", "items": {"type": "string"}},
                            "parallel": {"type": "boolean"},
                            "gate": {"type": "string"}
                        }
                    }
                }
            }
        }
    }
}]
response = llm.chat(messages=conv, tools=tools, tool_choice="output_project_config")
data = json.loads(response.tool_calls[0].arguments)  # 100% 可解析
```

**为什么这能解决问题**：不是做更宽容的解析器——是**从根上消除解析问题**。LLM 在 function calling 模式下由模型本身保证 JSON schema 合规，不需要 regex 抓取、不需要 YAML 缩进容错。

### 2. 工作流真 agent 流转稳定

集成测试发现窗口创建了但 workflow 流转卡了。根因是 replay 模式跳过了真实 LLM 调用的异步和超时处理。

**修复**：
- `workflow start` 在启动窗口前检查窗口是否 idle（非 blocked/non-pending）
- 真 agent 完成后 `window.toml` state 的更新必须触发 workflow 引擎的 notify
- 加 `workflow status --watch` 子命令：持续轮询窗口状态并自动推进 stage（v0.3 需要人反复跑 `workflow start`）

---

## 阶段拆解

| 阶段 | 内容 | 预计 | 验收 |
|---|---|---|---|
| **S1** | function calling 替换 YAML 解析（analyze 命令重构） | 1 天 | function calling 产出 JSON → 直接 deploy |
| **S2** | workflow 真 agent 流转稳定（窗口 → state → workflow notify） | 0.5 天 | 三窗口真 agent 流转全自动 |
| **S3** | `workflow status --watch` 自动推进 | 0.5 天 | 启动后自动跑完 |
| **S4** | 最终集成测试：用 function calling 重跑 mini-blog 全链路 | 0.5 天 | analyze 首胜率 > 80% |
| **S5** | 验收报告 v0.7 + 框架 v1.0 定版 | 0.5 天 | 对照设计 v2.1 全部条款 |

---

## 验收红线（比之前的版本更严）

| 级别 | 判据 |
|---|---|
| 🔴 | function calling 产出 JSON 解析失败（schema 约束下不应出现） |
| 🔴 | analyze 首胜率 < 50%（三次独立测试） |
| 🔴 | 真 agent 完成后 workflow 未自动推进到下一 stage |
| 🔴 | 框架 4 断言 + v0.1-v0.6 全回归 |
| 🟡 | function calling 降级：若 provider 不支持 function calling → 回退 YAML 模式 + 警告 |

---

## §v0.7.1 审查补充（5 个缺口修正）

> 审查 `plan-v07.md` v0.7 与现有约定（v1.1 补丁6 契约 / AGENT_MODE replay 机制 / workflow 引擎实现）的一致性。
> 以下 5 项随 v0.7 一起施工。

### 补充 1：function calling schema 必须含 budget/outputs（v1.1 补丁6 契约）

plan 的 schema 示例里 windows 只有 `id/role/prompt/depends_on`——**漏了 budget/outputs/gate**！
若 LLM 不产出这些 → deploy 校验拒绝 → 问题没解决（和 v0.6 一样卡在契约）。

```
windows.items.properties 补：
  budget: {type:object, properties:{max_steps:{type:integer}, max_cost_cny:{type:number}, provider:{type:string}}}
  outputs: {type:array, items:{type:string}}        # 产出路径
  gate: {type:string}                                # "human:..." 或 "auto:xxx.sh"
required 补：["id","role","prompt","budget","outputs","gate"]

function calling 产出 → json.loads → validate_deploy_yaml（复用）→ 直接 deploy
```

### 补充 2：replay 模式的 function calling 假响应

AGENT_MODE=replay 时 analyze 不调 LLM——返回**内置假 JSON**（与 function calling 产出同构）：

```
REPLAY_CONFIG_JSON = {
  "project_type": "software",
  "goal": "测试项目",
  "windows": [{"id": "win-dev-01", "role": "开发", "prompt": "你是开发者",
               "depends_on": [], "budget": {"max_steps": 30, "max_cost_cny": 0.3,
               "provider": "deepseek"}, "outputs": ["src/"],
               "gate": "auto:shared/gates/dev-gate.sh"}],
  "workflow_stages": [{"id": "impl", "trigger": "project_start",
                       "windows": ["win-dev-01"], "gate": "human:人类审核"}]
}
测试验证：replay analyze → json.loads → validate → deploy 链路（0 token）
```

### 补充 3：S5 测试 15 项清单

| # | 测试 | 覆盖 |
|---|---|---|
| 1 | `test_analyze_fc_replay_json` | replay 返回假 JSON 且可解析 |
| 2 | `test_analyze_fc_contract_fields` | JSON 含 budget/outputs/gate（补充1） |
| 3 | `test_analyze_fc_validate` | JSON → validate_deploy_yaml 通过 |
| 4 | `test_analyze_fc_deploy_chain` | analyze → deploy 全链路 |
| 5 | `test_analyze_fc_real_no_key` | real 无 key → 报错 |
| 6 | `test_analyze_fc_malformed` | LLM 返回坏 JSON → 拒绝 + 原因 |
| 7 | `test_analyze_fc_fallback_yaml` | 🟡 降级：模拟 provider 不支持 → 回退 YAML 模式 + 警告（补充5） |
| 8 | `test_workflow_auto_progress` | 窗口 done 后 workflow 自动推进 |
| 9 | `test_workflow_blocks_on_blocked` | 窗口 blocked → 不推进 |
| 10 | `test_workflow_status_watch` | --watch 自动推进（S3） |
| 11 | `test_workflow_state_notify` | state 更新触发引擎重读（补充4） |
| 12 | `test_workflow_rerun_idempotent` | 重复 workflow start 不重复启动 done 窗口 |
| 13 | `test_sandbox_audit_after_flow` | 全流程后路径审计（v0.6 补充4 延续） |
| 14 | `test_compress_after_flow` | 长对话窗口压缩仍可用 |
| 15 | `test_framework_check_after_all` | 全流程后 4 断言全绿 |

### 补充 4：workflow notify 用轮询增强，不改架构

S2 说"state 更新必须触发 workflow notify"——**现有架构是轮询**（workflow start 循环读 state）。
不要改成事件驱动（过度设计）：

```
保持轮询，但增强：
  1. workflow start 循环内：每次读窗口 state（已有）→ 加强频率（sleep 2s）
  2. 窗口 agent 完成 → state=done（已有）→ 引擎下一轮读到 → 推进 stage（已有逻辑）
  3. 真正缺口：workflow start 是一次性调用，跑完就退出——窗口是异步完成的！
     → 新增 workflow status --watch：持续轮询（不退出）直到全部 stage done 或 blocked
  验收：三窗口真 agent 时 --watch 自动推进到全部完成
```

### 补充 5：function calling 降级路径 + 测试

🟡 降级：provider 不支持 function calling → 回退 YAML 模式 + 警告。

```
检测：analyze real 模式发起 function calling 请求 → 400/不支持错误 → 捕获 → 回退旧 YAML 流程
测试：模拟（env FC_DISABLED=1 → 强制走 YAML 路径）→ 断言输出警告 + YAML 流程可用
```

---

## v0.7 之后

- 框架定版 **v1.0**：设计 v2.1 全部条款落地 + 集成测试验证
- 进入维护期：季度集成测试，按需修复
- 两个方向（待定）：
  - A) 对接 codex-rust agent-core（用锻造九轮验证的成熟 agent 引擎替换内置 LLM 调用）
  - B) 跑分基准（给窗口群框架建立类似 codex-rust 的基准/应力/回放防线）