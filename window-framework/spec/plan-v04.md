# 窗口群框架 v0.4 施工计划 v0.4.1 — 上下文压缩（堵住之前，放开之后）

> 基线：v0.3 61/61 全绿（工作流引擎 + deploy + 自动快照）
> 核心判断：v0.3 之前积累的四项遗留（压缩/冲突/LLM 需求分析/导入模板）中，**上下文压缩是其余三项的前置条件**。
> 不压缩 = 窗口对话上限写死 → 长需求聊不完 → deploy 契约不规范 → 后续全部卡住。
>
> v0.4 只做一件事：让窗口的对话可以一直聊下去。
>
> **v0.4.1 审查补充（2026-08-01）**：审查发现 5 个缺口——压缩调 LLM 但无 replay 测试模式、
> 压缩后 conversation.jsonl 存储格式未定义、export 渲染摘要未处理、v0.3 遗留的 50 轮自动快照
> 未纳入、S5 测试 15 项无清单。已并入 §v0.4.1 补充。

---

## 为什么压缩是唯一优先

```
不压缩 → 32K token 硬上限 → 窗口聊 50 轮就满
                              → 需求窗口聊不完需求 → 产出不了 YAML
                              → 开发窗口改不完代码 → gate 过不了
                              → 所有 workflow 卡死

压缩就绪 → 窗口对话无限长 → 需求窗口能充分分析 → 产出高质量 YAML
                           → 开发窗口能 debug + 重试 → gate 通过率上升
                           → 设计 v0.5 的自主建窗才会真正可用
```

---

## v0.4 一件大事：三层压缩引擎

### 不是简单的截断

截断 = 删掉老对话。压缩 = **让窗口自己总结老对话，保留关键信息，扔掉废话**。

```
窗口的完整对话历史（可能 300 轮 / 50K tokens）
        │
        ▼
   ┌─────────────────────────────────────┐
   │ 热层（最近 20 轮）                   │ ← 完整保留，不进压缩
   │ 每轮 tool_calls + 结果 完整保留      │
   ├─────────────────────────────────────┤
   │ 温层（21-100 轮）                    │ ← 每 5 轮 → 让 LLM 压缩为一条结构摘要
   │ 保留：做了什么 / 关键决策 / 产出文件   │
   ├─────────────────────────────────────┤
   │ 冷层（101+ 轮）                      │ ← 全部温层的摘要再次压缩为"历史要点"
   │ 仅保留：几阶段 / 产出什么 / 未解决     │
   └─────────────────────────────────────┘
```

### 压缩触发

```python
def should_compress(window):
    """current_tokens 超过 max_tokens × 0.7 → 触发压缩"""
    if window.current_tokens > window.max_tokens * 0.7:
        return True
    # 或每个 workflow stage 完成后
    if window.stage_just_done:
        return True
    return False
```

### 压缩执行

```python
def compress(window):
    # 1. 自动快照（压缩前必做——防坏）
    snapshot(window)
    
    # 2. 冷层 → 温层 → 热层，逐层处理
    cold = window.layers.get("cold", [])  # 101+ 轮
    warm = window.layers.get("warm", [])  # 21-100 轮
    
    # 3. 对温层：每 5 轮 → 调用 LLM 产出结构化摘要
    for chunk in chunks(warm, 5):
        summary = llm_compress(chunk, max_chars=2000)
        compressed_warm.append(summary)
    
    # 4. 如果温层摘要超过 10 条 → 全部 push 到冷层，冷层再压一次
    if len(compressed_warm) > 10:
        cold_summary = llm_compress(all_warm_summaries, max_chars=2000)
        cold.append(cold_summary)
        compressed_warm = compressed_warm[-10:]  # 保留最近 10 条
    
    # 5. 替换窗口的对话历史 = 冷层摘要 + 温层摘要 + 热层完整
    window.rebuild_context(cold, compressed_warm, hot)
    window.compression_count += 1
    window.current_tokens = recalculate()
```

### 压缩内容契约（LLM 必须产出的格式）

```
窗口: win-backend-01
压缩范围: 第 21-25 轮
---
做了什么: 实现了用户登录 API 的 /auth/login 端点，包括 JWT 签发和密码 bcrypt 校验
关键决策: 选用 jsonwebtoken crate 而非 oauth2（原因：需求只有简单 JWT 登录）
产出文件: src/auth.rs (新增, 85 行), shared/outputs/src/auth.rs
未解决: 登录失败的错误码格式需要 PM 确认（已记录在 shared/progress.md）
```

**压缩质量自检**：压缩后对窗口发一条消息"你上一阶段做了什么"——窗口必须能从摘要中回答出正确的答案。回答不出来 = 压缩过度 = 自动回滚快照 + 调整策略重压。

---

## 阶段拆解

| 阶段 | 内容 | 预计 | 验收 |
|---|---|---|---|
| **S1** | 三层分层模型（热/温/冷 + rebuild_context） | 0.5 天 | 分层数据正确 |
| **S2** | LLM 压缩调用（每 5 轮调 LLM 出结构化摘要） | 1 天 | 摘要包含"做了什么/关键决策/产出/未解决" |
| **S3** | 自动触发 + 压缩前快照 + 回滚保护 | 0.5 天 | 触发了压、压坏了能回滚 |
| **S4** | 压缩质量验证：压缩后窗口必须能回答"上阶段做了什么" | 0.5 天 | 自检测试通过 |
| **S5** | 测试：15 项覆盖（分层/触发/摘要格式/回滚/边界） | 0.5 天 | 全绿 + 回归 61/61 |
| **S6** | 验收报告 v0.4 | 0.5 天 | 对照设计 §3 核对 |

---

## 验收红线

| 级别 | 判据 |
|---|---|
| 🔴 | 压缩后 current_tokens 不降反升 |
| 🔴 | 压缩后窗口回答错"上一阶段做了什么"（质量自检失败） |
| 🔴 | 压缩后丢失关键文件引用（产出文件路径消失） |
| 🔴 | 框架 4 断言 + v0.1-v0.3 回归 |
| 🟡 | 压缩耗时 > 5 秒（影响体验但不阻塞） |

---

## §v0.4.1 审查补充（5 个缺口修正）

> 审查 `plan-v04.md` v0.4 与现有代码（conversation.jsonl 格式 / AGENT_MODE 机制）及
> v0.3 遗留清单的一致性。以下 5 项随 v0.4 一起施工。

### 补充 1：压缩的 replay 摘要模式（防测试烧 token）

压缩调用 LLM（S2）每次消耗 token——**测试不能烧**。对齐 v0.3 的 AGENT_MODE：

```
AGENT_MODE=replay 时压缩行为：
  不调 LLM，直接生成"假摘要"（格式与真摘要一致，内容 = 该 chunk 前 3 行消息拼接）
  → 测试验证压缩【机制】正确（分层/触发/回滚/存储），不验证 LLM 摘要质量
AGENT_MODE=real 时压缩行为：
  调 LLM 产出真摘要（生产行为）

LLM key 来源：同 Agent.run（DEEPSEEK_API_KEY env，无 key + real 模式 → 报错不压）
```

### 补充 2：压缩后 conversation.jsonl 存储格式

压缩把老对话换成摘要——**写回 conversation.jsonl 是唯一持久化方式**（现有格式）：

```json
// 压缩后，被压缩的轮次替换为一条 summary 消息：
{"t": "2026-08-01T08:00:00Z", "role": "summary",
 "content": "做了什么: ...\n关键决策: ...\n产出文件: ...\n未解决: ...",
 "meta": {"compressed_rounds": "21-100", "compression_id": 3}}
```

规则：
- 压缩前的对话不物理删除——先快照（防坏），conversation.jsonl 里**替换**为 summary 行
- `summary` 是 role 新值（现有 system/user/assistant/tool 之外的第五种）
- `_read_conv` 需兼容（summary 行参与 export 渲染）

### 补充 3：export 对 summary 行的渲染

`window export --format markdown` 遇到 `role=summary` 时：

```
**📦 摘要 (第 21-100 轮, compression #3)**: 
做了什么: ... / 关键决策: ... / 产出文件: ... / 未解决: ...
```

JSON 导出原样保留（summary 行就是一条消息，格式合法）。

### 补充 4：50 轮自动快照（v0.3 遗留，一并修复）

v0.3 遗留清单里标了"50 轮自动快照未接线"（accepted-v03 §八）。v0.4 补上：

```
Agent.run 循环中：每 50 轮（turn % 50 == 0）→ _auto_snapshot(窗口)
验收：跑 51 轮 replay → .snapshots/ 出现自动快照
```

### 补充 5：S5 测试 15 项清单

| # | 测试 | 覆盖 |
|---|---|---|
| 1 | `test_layers_split` | 热/温/冷三层划分正确（红线 S1） |
| 2 | `test_should_compress_threshold` | tokens > 70% 触发（红线 🔴 判据） |
| 3 | `test_compress_replaces_conv` | 压缩后 conversation.jsonl 含 summary 行 |
| 4 | `test_compress_reduces_tokens` | 压缩后 current_tokens 下降（红线 🔴） |
| 5 | `test_summary_content_contract` | 摘要含"做了什么/关键决策/产出/未解决" |
| 6 | `test_compress_snapshot_before` | 压缩前自动快照（S3） |
| 7 | `test_compress_rollback_on_bad` | 摘要缺字段 → 回滚快照（§5g） |
| 8 | `test_export_summary_rendered` | export markdown 渲染 summary 行（补充3） |
| 9 | `test_export_json_summary_valid` | JSON 导出 summary 合法 |
| 10 | `test_replay_summary_no_llm` | replay 模式不调 LLM（补充1） |
| 11 | `test_no_key_real_refuses` | real 模式无 key → 不压（补充1） |
| 12 | `test_auto_snapshot_50` | 50 轮自动快照（补充4，修复 v0.3 遗留） |
| 13 | `test_compression_count_increments` | window.toml compression_count +1 |
| 14 | `test_compress_boundary_20` | 恰好 20 轮 → 不压（热层全保留） |
| 15 | `test_framework_check_after_compress` | 压缩后 4 断言全绿 |

---

## v0.5 预留（压缩就绪后才能做）

- 需求窗口 LLM 自主分析 → 产出规范 YAML（需长对话先不截断）
- 共享层冲突仲裁（§8.4）
- 导入外部对话 / 模板系统
- **v0.5 之后**：窗口群框架达到"全自动"——人只需建项目+聊需求，其余自动流转