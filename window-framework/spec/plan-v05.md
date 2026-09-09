# 窗口群框架 v0.5 施工计划 v0.5.1 — 需求分析自主化 + 导入/模板收尾

> 基线：v0.4 78/78 全绿（压缩引擎就绪，窗口能聊无限长）
> 核心判断：v0.4 压缩引擎是关键基础设施——它建成后，需求窗口终于可以充分聊天了。
> v0.5 做两件事：让需求窗口**自主分析并产出窗口配置**（核心价值），
> 然后把设计 v2.1 里拖了三版的导入/模板收掉（工具完备）。
>
> v0.5 是窗口群框架的 **"全自动"版**——之后只需 `project create` + 聊天 + `workflow deploy`，流程自己跑。
>
> **v0.5.1 审查补充（2026-08-01）**：审查发现 5 个缺口——analyze 调 LLM 但无 replay 模式、
> YAML 校验漏契约字段（budget/outputs/gate）、S5 测试 20 项无清单、导入格式映射未定义、
> 模板存储位置/重名处理未定义。已并入 §v0.5.1 补充。

---

## v0.5 两件事

### 1. 需求窗口自主分析（核心：从"人手写 YAML"到"AI 自己产出 YAML"）

v0.3 的 `workflow deploy` 能从对话里解析 YAML，但那个 YAML 是**人写的**。v0.5 让需求窗口自己产出。

**输入**：需求窗口和人类聊了几十轮项目背景、目标、约束  
**输出**：规范的窗口配置 YAML（含 windows + workflow_stages + budget）

**实现**：

```bash
codex window analyze win-requirement
  # 1. 读 win-requirement 的 conversation.jsonl（完整对话，压缩层不管）
  # 2. 提取关键信息：项目目标、类型、约束、产出物
  # 3. 发给 LLM（专用 prompt：需求分析师角色）
  # 4. LLM 产出 YAML → 框架校验 YAML 格式 + 契约字段完整性
  # 5. 展示给人审核：列出的窗口清单 + 流转阶段
  # 6. 人确认 → 自动调用 workflow deploy
```

**专用 prompt 模板**（嵌入框架）：

```
你是一个项目需求分析师。现在有一个需求窗口和人类聊了几十轮，对话历史如下。
你的任务是分析这段对话，产出一份结构化的窗口配置 YAML。

要求：
1. 项目类型从对话中推断（software/writing/data/research/ops）
2. 项目目标用一句话描述
3. 根据对话中的需求复杂度，确定需要的窗口角色 + 数量
4. 每个窗口必须有：id、role、prompt（窗口启动时的 system prompt）、depends_on
5. 窗口之间不要创建冗余角色——不需要 PM/DevOps 就别加
6. workflow_stages 定义流转次序：哪些窗口等哪些窗口、哪些并行
7. 每个 stage 的 gate 声明为 human（人审）还是 auto（自动校验）

输出格式（只输出 YAML，不要废话）：
---
project_type: software
goal: "搭建一个..."
windows:
  ...（同上契约格式）
workflow_stages:
  ...
```

**验收**：人和需求窗口聊 20 轮 → `codex window analyze` → 产出有效 YAML → 人确认 → 自动建全窗口群 + 配流转规则 → `workflow start` 跑通。

### 2. 导入 / 导出 / 模板收尾

设计 v2.1 §5e（导入导出）和 §5g（模板）从 v0.1 规划到现在，v0.5 全部收掉。

#### 导入外部对话

```bash
codex window import --name "win-imported" --role "researcher" \
  --source ~/other-ai-chat.json
```

输入可以是：
- `conversation.jsonl`（同框架产出的格式）
- OpenAI chat completion 的 messages 数组 JSON

导入后自动：
1. 解析 → 写入 `conversation.jsonl`
2. 跑一次压缩（外部对话可能很长）
3. 提取对话中引用的文件路径 → 如果没有对应文件则警告

#### 模板系统

```bash
codex template save "后端开发模板" --from-window win-backend-01
  # 把 win-backend-01 的 prompt + role + budget 保存为模板

codex template list
  # 列出所有已保存模板

codex window create --template "后端开发模板"
  # 一键创建同配置窗口
```

#### 导出增强

v0.2 已有 markdown + json 导出。v0.5 加：
- `--compress` 选项：导出的内容已压缩（只输出冷/温层摘要 + 热层完整对话）——适合给人类快速阅读
- `--full` 选项：导出完整对话（不压缩）

---

## 阶段拆解

| 阶段 | 内容 | 预计 | 验收 |
|---|---|---|---|
| **S1** | `window analyze` 命令（LLM 分析对话 → 产出 YAML） | 1 天 | 20 轮对话 → 有效 YAML → deploy 建窗 |
| **S2** | YAML 质量校验（窗口数合理、role 不重复、depends 无环） | 0.5 天 | 坏 YAML 被拒绝 + 原因 |
| **S3** | `window import`（JSONL + OpenAI 格式兼容） | 0.5 天 | 外部对话导入后 export 一致 |
| **S4** | template save/list/create | 0.5 天 | 三部曲全通 |
| **S5** | 测试：20 项覆盖新功能 | 0.5 天 | 全绿 + 回归 78/78 |
| **S6** | 验收报告 v0.5 + 框架完成度总结 | 0.5 天 | 对照设计 v2.1 全部条款 |

---

## 验收红线

| 级别 | 判据 |
|---|---|
| 🔴 | `window analyze` 产出包含环依赖的流转规则 |
| 🔴 | `window analyze` 窗口数 = 0 或 > 10（不合理） |
| 🔴 | `window import` 后数据丢失 |
| 🔴 | 框架 4 断言 + v0.1-v0.4 回归 |
| 🟡 | analyze 产出的 YAML 人审退回率 > 50%（用 3 个测试用例测） |

---

## §v0.5.1 审查补充（5 个缺口修正）

> 审查 `plan-v05.md` v0.5 与现有约定（AGENT_MODE 机制 / v1.1 补丁6 契约 / v0.3 deploy 校验）的一致性。
> 以下 5 项随 v0.5 一起施工。

### 补充 1：analyze 的 replay 模式（防测试烧 token）

`window analyze` 调 LLM 产出 YAML——测试必须不烧 token（对齐 v0.3/v0.4 的 AGENT_MODE）：

```
AGENT_MODE=replay 时 analyze 行为：
  不调 LLM，返回内置的"测试 YAML"（含 2 窗口 + 2 stage，契约字段完整）
  → 测试验证 analyze 的【流程】（提取→校验→展示→deploy 接线），不验证 LLM 分析质量
AGENT_MODE=real 时 analyze 行为：
  调 LLM（专用需求分析师 prompt）产出真 YAML
LLM key 来源：DEEPSEEK_API_KEY env（无 key + real → 报错退出码 1）
红线 🟡 退回率测试用 replay（3 个预置对话场景 → 3 次 analyze → 校验产出）
```

### 补充 2：analyze 产出必须过契约校验（对齐 v1.1 补丁6）

S2 的 YAML 质量校验不只查"窗口数/role 重复/depends 无环"——**还要过 v0.3.1 已定的契约**：

```
每个 window 必填：id / role / prompt / budget(max_steps,max_cost_cny,provider) / outputs / gate(.sh)
workflow_stages 每个必填：id / trigger / windows / gate（human: 或 auto:）
缺任一 → analyze 产出判无效 → 要求 LLM 重产或报错
复用 v0.3 deploy 的校验逻辑（同一函数，不重复实现）
```

### 补充 3：S5 测试 20 项清单

| # | 测试 | 覆盖 |
|---|---|---|
| 1 | `test_analyze_replay_yaml` | replay 产出有效 YAML |
| 2 | `test_analyze_contract_fields` | 产出含 budget/outputs/gate（补充2） |
| 3 | `test_analyze_no_cycle` | depends 无环（红线 🔴） |
| 4 | `test_analyze_window_count` | 1 ≤ 窗口数 ≤ 10（红线 🔴） |
| 5 | `test_analyze_role_unique` | role 不重复 |
| 6 | `test_analyze_deploy_chain` | analyze → deploy → 建窗全链路 |
| 7 | `test_analyze_reject_bad_yaml` | LLM 产出坏 YAML → 拒绝 + 原因 |
| 8 | `test_analyze_real_no_key` | real 无 key → 报错（补充1） |
| 9 | `test_analyze_3_scenarios_replay` | 🟡 退回率（3 预置场景 replay） |
| 10 | `test_import_jsonl` | 导入 JSONL → export 一致 |
| 11 | `test_import_openai_format` | OpenAI messages 数组映射（补充4） |
| 12 | `test_import_runs_compression` | 长对话导入后自动压缩 |
| 13 | `test_import_missing_files_warn` | 引用的文件不存在 → 警告 |
| 14 | `test_template_save` | 从窗口保存模板 |
| 15 | `test_template_list` | 列出模板 |
| 16 | `test_template_create_from` | 模板一键建窗 |
| 17 | `test_template_name_conflict` | 重名模板 → 拒绝/覆盖（补充5） |
| 18 | `test_export_compress_flag` | export --compress 只含摘要+热层 |
| 19 | `test_export_full_flag` | export --full 完整对话 |
| 20 | `test_framework_check_after_all` | 全流程后 4 断言全绿 |

### 补充 4：导入格式映射（OpenAI messages → 框架格式）

```
输入 JSON（OpenAI chat completion messages 数组）：
  {"role": "user", "content": "..."}                    → {role:user, content}
  {"role": "assistant", "content": null,
   "tool_calls": [{"function": {"name": "read", "arguments": "{\"path\":\"x\"}"}}]}
    → {role:assistant, tool_calls: [{name:read, args:{...}}]}
  {"role": "tool", "tool_call_id": "...", "content": "..."} → {role:tool, name:"", content}
映射规则：content 缺省→""；tool_calls 的 arguments 是 JSON 字符串→解析为对象；
未知字段忽略；role 白名单 system/user/assistant/tool
```

### 补充 5：模板存储位置 + 重名处理

```
存储：{PROJECTS_ROOT}/templates/{name}.toml（全局，跨项目可用）
模板内容：window.toml 的 role/prompt/budget/outputs/gate（不含 id/state/对话）
重名：template save 遇同名 → 拒绝 + 提示 "用 --force 覆盖"（不静默覆盖）
window create --template：读模板 → 生成新窗口（id 自动 -2/-3 去重）
```

---

## v0.5 之后

窗口群框架达到**设计 v2.1 目标**：
- ✅ skeleton (v0.1) → brain (v0.2) → orchestration (v0.3) → memory (v0.4) → autonomy (v0.5)
- 剩余未落地条款：共享层冲突仲裁 (§8.4) — 留 v0.6，依赖并行工作流大量实测后才需要

**框架完成度**：v0.5 = 人只需创建项目 + 在一个窗口里聊天。其余自动。