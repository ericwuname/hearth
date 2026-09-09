# 双端口前端 UI —— 前后端职责切分与 AI OS 契约

> 阶段说明：当前处于**设计阶段**（demo v11）。真实后端（codex-rust 内核）实现在后续执行窗口落地。
> 本文目标：把已讨论的 UX 功能按 **前端(FE) / 后端(BE) / 共享契约** 三类切分，并定义 **AI OS 事件流 schema**——
> 它是 BE 产出、FE 消费的**单一事实源**，是前后端解耦的关键契约。

---

## 1. 分层模型

```
┌──────────────────────────────────────────────────────────────┐
│  Human OS（前端 / 给人看）                                       │
│  折叠对话 · 压缩摘要 · 旋转图标 · 弹窗 · span 树时间轴            │
├──────────────────────────────────────────────────────────────┤
│  AI OS Contract（共享契约，唯一事实源）                          │
│  事件流 schema(SSE/JSONL) · span tree schema · clarify/approval │
├──────────────────────────────────────────────────────────────┤
│  AI OS Kernel（后端 / codex-rust）                              │
│  循环+基因 G0-G3+工具+沙箱+xray → 产出事件流与 span 树           │
└──────────────────────────────────────────────────────────────┘
```

**铁律**：Human OS 只是 AI OS 事件流的「人话投影」。FE 不生成任何业务真相，只渲染 BE 产出的事件/span。

---

## 2. 功能职责矩阵

| UX 功能 | 归属 | 说明 |
|---|---|---|
| 一问一答压缩对话布局 | **FE** | 纯渲染，消费内核产出的消息流 |
| 两层折叠（整条 / 过程） | **FE** | 纯 UI 状态，无内核依赖 |
| 最终结果常显、摘要可折 | **FE** | 渲染策略 |
| 旋转图标（执行态） | **FE** | 由事件 `phase`/`status` 推导，FE 渲染转/停 |
| 主动澄清弹窗（输入框 + chip） | **FE 渲染 + BE 触发/续跑** | BE 在 plan 期判断歧义并 emit `need_clarification`（含 options）；FE 渲染弹窗、收集文本/chip、回传 `clarification`；**内核 resume 后续链路** |
| 审批门弹窗 + 常显（不折叠丢失） | **FE 渲染 + BE 触发/裁决** | BE 在执行危险操作前 emit `need_approval`（含 op 描述 + 风险等级）；FE 渲染常显弹窗；**BE 应用 approve/reject 决策** |
| 嵌套子循环 span 树 | **FE 渲染 + BE 建模** | BE 用 `parent_id` 建模 span 树（t0/t1/status/summary/kind）；FE 渲染缩进树 / 时间轴 |
| 任务/子任务时长聚合 | **FE 派生 + BE 度量** | BE 记录每个 span 真实 `t0/t1`；FE 聚合（总时长 = Σ子树、关键路径） |
| AI 思考摘要 | **BE 生成 + FE 渲染** | BE 在 span 关闭时生成 `summary`；FE 在折叠态显示 |
| AI OS 事件流（彩色标签） | **FE 渲染 + BE 产出** | BE emit 各类型事件；FE 格式化着色 |
| 时间轴 scrubber / 回滚点 | **FE（可纯前端）** | 消费 span 树；若需真实回滚，BE 需额外提供快照接口 |
| diff 视图 / 关键路径高亮 | **FE 派生** | 消费 span 树 + 事件流 |
| 协作模式策略（长程独立 / 人机协作 / 关键请示） | **BE 决策 + FE 渲染模式态** | BE 按模式策略决定**何时 emit** `need_clarification`/`need_approval`：长程独立=不 emit（AI 自主最优）；关键请示=仅 high 风险 emit 审批、必要才 emit 澄清；人机协作=规划前批量 emit 澄清、执行期全量 emit 审批。FE 仅渲染模式标签与对应交互密度 |
| **规划驱动的信息缺口（不是为问而问）** | **BE 全责推导 + FE 只渲染** | BE 先产 `plan.draft`（步骤树），再对每步做**缺口推导**，产出 gap 三元组 `{from, why, blocking}`；FE **不得自造问题**，只渲染 BE 给的 gap 及其来源步骤与代价说明 |
| **AI 自行推断项（assumed）** | **BE 生成 + FE 展示** | BE 把「有唯一答案、无需打扰人类」的判断显式列出（读 Cargo.toml 得出框架版本等），随 `plan.draft` 一起下发；FE 在收集窗展示「以下 N 项 AI 已自行确定」——这是「只问真需要」的**可验证证据** |
| **层级折叠三态**（顶层收 / 逐层钻 / 全展开） | **纯 FE** | 卡片三段式 head(常显) / detail(可折) / children(可折)；完成自动折叠、终态全局收束、任意层可逆展开到最底层工具行 |
| **产物卡片 + 一键打开** | **BE emit + FE 渲染 + BE 执行打开** | BE 在写文件/起服务时 emit `artifact` 事件（path/delta/kind）；FE 聚合成常显卡片区；点击回传 `artifact.open` → **BE 执行 `open_artifact`**（file → `shell.openPath`，url → 默认浏览器）。FE 不直接触碰文件系统 |

**结论**：所有「弹窗触发、缺口推导、决策应用、span 建模、时长度量、摘要生成、协作模式策略、产物登记与打开」的**真相都在 BE**；FE 只负责渲染与把人类输入回传。
demo 之所以能用 mock 跑通，是因为它内置了一份**符合契约的事件脚本**——这恰好证明了契约稳定后 FE 可独立开发。

### 2.1 「问题必须由规划推导」的硬约束

这是本轮确立的**产品级红线**：AI 不允许问「模板化的通用问题」，每个问题必须能回答三件事——

| 必答项 | 契约字段 | 反例（不合格） |
|---|---|---|
| 这问题来自哪个规划步骤 | `gap.from = "P2"` | 「你有什么额外要求吗？」（无来源） |
| 不问会付出什么代价 | `gap.why`（具体到返工范围/风险面） | 「为了更好地帮助你」（无代价） |
| 是否阻塞（不答能不能继续） | `gap.blocking` + `gap.assume` | 全部标 blocking（等于没分级） |

推导时机：**规划完成后、执行开始前**，一次性批量问清（三种协作模式都执行此步——长程独立也要背景，只是只问 blocking 项）。
执行期只有满足「规划期不可知（依赖运行时真实数据）」且「继续会产生不可逆后果或超出已批准范围」两个条件，才允许中途打断。

---

## 3. AI OS 事件流 Schema（契约草案）

统一信封：

```json
{ "ts": 1690000000000, "seq": 1, "type": "span_open", "span_id": "s12", "parent_id": "s3", "data": {} }
```

事件类型与 `data` 字段：

| type | 触发时机 | data 字段 |
|---|---|---|
| `session.start` | 会话开始 | `{task}` |
| `session.end` | 会话结束 | `{}` |
| `phase_transition` | 进入某相位 | `{phase: plan\|act\|observe\|reflect}` |
| `span_open` | 任务/子任务/子循环开始 | `{id, kind: task\|subtask\|tool\|loop, parent_id?}` |
| `span_close` | 任务结束 | `{id, dur, status: done\|blocked}` |
| `tool_call` | 工具调用前 | `{name, args?}` |
| `tool_result` | 工具返回 | `{name, out, ok}` |
| `plan.draft` | 规划完成、执行开始前 | `{steps[], gaps_found, gaps_to_ask, auto_assumed, mode}` |
| `need_clarification` | 缺口需人类确认 | `{gap_id, from, blocking, question, why, options?}` |
| `clarification` | 人类回答澄清 | `{gap_id, answer, skipped}` |
| `gap.auto_resolved` | 非阻塞缺口 / 模式不打断 | `{gap_id, from, assumed, reason}` |
| `need_approval` | **执行期**危险操作前 | `{op, risk: low\|med\|high, impact, why}` |
| `approval` | 裁决（人类或策略自动） | `{decision: approve\|reject, by: human\|policy}` |
| `artifact` | 产出文件/服务/URL 时 | `{span_id, kind: file\|url, path, delta, new}` |
| `artifact.open` | 人类点击产物卡片 | `{path, via: system_default\|browser}` |
| `think` | span 内产生思考摘要 | `{span_id, summary}` |

> 传输：SSE（`text/event-stream`）或 JSONL 流式。FE 订阅后逐行渲染。

---

## 4. Span Tree Schema

```json
{
  "id": "s12",
  "parent_id": "s3",
  "kind": "subtask",
  "title": "实现 healthz 端点",
  "t0": 1690000010000,
  "t1": 1690000018000,
  "status": "done",
  "think_summary": "新建 healthz.rs，按澄清加 Bearer 校验",
  "cost": { "tokens": 1200, "usd": 0.002 }
}
```

- `parent_id` 为空 = 根任务。
- `t1 - t0` = 该 span 自身时长；**聚合时长 = 子树 t0..t1 并集**（FE 可派生总耗时/关键路径）。

---

## 5. Clarify / Approval 协议

**澄清（规划期，开放问答）**
- 请求：`{ question, options?: string[], context? }`
- 响应：`{ answer: string, skipped: boolean }`

**审批（执行期，二选一）**
- 请求：`{ op: string, risk: low|med|high, impact: string }`
- 响应：`{ decision: approve|reject, note?: string }`

> 二者本质不同：澄清是「缺信息→收集」，审批是「危险操作→确认」。BE 必须在正确的相位注入对应的事件。

---

## 6. 给执行窗口的任务边界

**FE 任务（纯渲染层，可用 mock 事件流独立跑通）**
- 双端口布局、折叠/压缩、旋转图标、澄清/审批弹窗、span 树缩进渲染、时长聚合、事件流着色。
- 验收：当前 demo（v9）即 FE 原型，可直接对接真实事件流。

**BE 任务（内核需新增）**
1. span 树采集器：在 `loop.rs` 每进入/退出相位、任务、子循环时 emit `span_open`/`span_close` 并带 `parent_id`、真实 `t0/t1`。
2. **规划缺口推导器**（本轮新增，优先级最高）：plan 相位产出步骤树后，对每步做缺口扫描，输出 `{from, why, blocking, options, assume}` 三元组；同时输出 `assumed[]`（已自行确定项）。**禁止输出无 from/why 的通用问题**。
3. clarify 决策点：批量 emit `need_clarification`（规划前），收齐 `clarification` 后 resume；执行期仅在「运行时才可知 + 不可逆/超范围」双条件下才 emit。
4. approval 决策点：在写文件/执行 shell/绑定端口等危险操作前 emit `need_approval`（带 `why` 风险面描述），应用裁决；按协作模式决定自动批准还是等人类。
5. **产物登记器**：任何 write_file / bind_tcp / 生成 URL 的操作 emit `artifact`；并提供 `open_artifact` 命令（file → 系统默认程序，url → 浏览器）。
6. 事件流 SSE 出口：把内核事件统一序列化为 §3 信封并流式推送。
7. 思考摘要：span 关闭时由 LLM 生成 `think_summary`。

**验收闸门**：BE 产出的事件流能被当前 demo（v11）**直接消费且零改动** ⇒ 契约稳定、前后端解耦达标。
额外闸门：随机抽 10 个真实任务，其规划期产出的 gap 必须 **100% 带可追溯的 `from` 与具体 `why`**，且 `blocking=false` 的项必须带 `assume` 默认值——不满足即判定为「为问而问」，退回重做。
