# Observer OS 设计（三端口第三权）

> 状态：**设计定版 v1**（2026-08-02）
> 前置：`docs/frontend-backend-split.md`（双端口前后端契约）、`docs/codex-desktop-demo.html`（v11 原型）
> 本文同时记录 **Human OS / AI OS 的 v1 冻结基线**，作为后续打磨的对照物。

---

## §0 定版声明：Human OS / AI OS v1 冻结基线

以下能力在 demo v11 已验证形态，作为 v1 基线冻结。后续打磨以此为 diff 起点，不再重新讨论形态。

### Human OS v1（北向 · 给人看）

| 能力 | 冻结形态 |
|---|---|
| 对话压缩 | 一问一答；单次回复=一条助理卡片，过程折叠、摘要可再折叠 |
| 折叠三态 | 全部折叠到顶层 / 展开到底层 / 只展开执行中分支；**全部可逆** |
| 终态收束 | 任务完成后自动 collapseAll，只剩关键信息 + 最终结果 + 产物卡片 |
| 执行态指示 | 旋转图标：转=执行中；完成/暂停=**消失**（不留静态圆圈） |
| 规划期澄清 | 弹窗上半部显示规划草案 P1..Pn；每问必须标 `from`（来源步骤）+ `why`（不问的代价）+ `blocking` |
| 自行决议区 | 「AI 已自行确定的 N 项」明示，证明只问真需要 |
| 执行期澄清 | 仅当「规划期不可知」且「不可逆或超出已批准范围」双条件成立才允许打断 |
| 审批门 | 移出折叠区常显；模态强提醒；risk = low / med / high |
| 协作模式 | 人机协作 / 长程独立 / 关键请示（三模式均在开始前收集背景） |
| 产物卡片 | 结果区常显卡片网格，一键打开（触发 `artifact.open`，由 BE 执行） |

### AI OS v1（南向 · 给机器看）

| 能力 | 冻结形态 |
|---|---|
| 事实源地位 | **事件流是唯一事实源**，Human OS 是其人话投影，不产生业务真相 |
| span 树 | 任务 / 子任务 / 子循环 = 带 `parent_id` 的 span，含 `t0/t1/status/summary` |
| 事件类型 | `session.*` / `phase_transition` / `plan.draft` / `span_open` / `span_close` / `tool_call` / `tool_result` / `need_clarification` / `clarification` / `need_approval` / `approval` / `think` / `artifact` / `artifact.open` |
| 传输 | SSE 流式推送，Schema 版本化，确定性幂等 |
| 验收闸门 | BE 产出的真实事件流能被 demo **零改动直接消费** ⇒ 契约稳定 |

---

## §1 观察者定位：不是第三个界面，是第三个权力

Observer OS **不是**「再开一个面板」，而是在 Human OS（人的控制权）、Agent 内核（AI 的执行权）之外，补上第三种权力：**独立审计权**。

```
        Human OS  ──┐
                    ├──只读抽头──▶  Observer OS  ──报告──▶  人 / AI
        Agent 内核 ──┤
        AI OS     ──┘
```

服务对象是**双方**：既向人报告「AI 干得怎么样」，也向 AI 报告「你自己干得怎么样」。中立体现在——**对两边说同样的话，用同一份证据**。

### 与 nervous-system 的严格分家（关键区分）

源码事实（`crates/nervous-system/src/lib.rs`）：`NervousSystem::query()` 返回 `PerceptionReport { action: NerveAction }`，其中 `NerveAction::{Simplify, ReduceSteps, Abandon, DeliverAndQuit}` 会**直接干预执行**，调用点在 `loop.rs:1589`。

| | nervous-system | Observer OS |
|---|---|---|
| 本质 | **交感神经**：感知→决策→执行→验证闭环 | **审计**：只观察、只报告 |
| 权力 | 连续干预（简化计划 / 削减步数 / 中止） | 无执行权（唯一例外见 §3） |
| 数据流 | 参与决策回路 | 旁路，不参与 |

**两者必须是两个 crate、两条数据通路。** 混在一起，中立性立即消失。

---

## §2 中立性四锁：中立不能靠承诺，只能靠结构

| 锁 | 机制 | 违反后果 | 基因层级 |
|---|---|---|---|
| **L1 只读抽头** | 观察者拿到的是事件流**只读句柄**（`Receiver<Arc<Event>>`），编译期不存在任何回写 channel | Rust 类型系统拒绝编译 | **G0** |
| **L2 不可静默** | 内核**没有**关闭观察者的 API；观察者进程/任务挂掉 ⇒ 内核 **fail-closed** 停机 | 无观察 = 不许执行 | **G0** |
| **L3 证据强制** | 每条结论必须带 `evidence: Vec<EventId>`（非空）；空证据结论**不允许序列化**进报告 | 报告生成器直接 reject | **G1**（挂 wiring 断言） |
| **L4 元观察** | 观察者自身耗时 / 漏报 / 队列积压 / 成本，写进同一份日志 | 观察者不能是黑箱 | **G1** |

**L3 是「客观」的可验证定义**：不是「我觉得客观」，而是**报告里每一句话都能 grep 回原始事件**。人和 AI 都能当场复核。
缺了 L3，观察报告就是系统里的第二个 LLM 幻觉源。

### 反 LLM 污染约束
- 指标层（计数、时长、成本、失败率）**必须由确定性代码计算**，禁止 LLM 参与。
- LLM 只允许出现在**叙述层**（把已算好的指标写成人话），且不得引入指标层不存在的数字。
- 验收：随机抽 20 条报告句子，逐句核对数字来源；出现任何指标层没有的数字 ⇒ 🔴 阻塞。

---

## §3 观察者悖论与权力边界

> 有牙齿就不中立，没牙齿就没用。

**取舍：权力极度收窄，只留一条。**

| 输出 | 层级 | 效力 |
|---|---|---|
| 常规观察结论 | **G2（偏置）** | 进 `experience` 判例库，只影响 AI **下一轮**决策上下文；**不能改变当前执行** |
| 红线熔断 | **G0（不可绕过）** | 仅限预算爆表 / 沙箱越界 / 重试风暴等硬红线，动作只有一个：**拉闸**。拉闸本身也写入事件流，事后可审 |

禁止项（写死）：
- ❌ 观察者不得下发「建议改用 X 工具」这类实时指令（那是 nervous-system 的职责）。
- ❌ 观察者不得修改计划、不得写文件、不得调用工具。
- ❌ 观察者不得对同一事件产生两种口径（对人一套、对 AI 一套）。

---

## §4 观察四象限与指标定义

### 象限 1 · 系统运行
| 指标 | 定义 | 来源 |
|---|---|---|
| `phase_duration_ms` | 各相位（Plan/Act/Observe/Reflect）耗时 | `phase_transition` 事件 |
| `tokens_in/out`、`cost_usd` | 累计 token 与成本 | gateway 计量 |
| `tool_fail_rate` | `tool_result.status=err` / 总调用 | `tool_result` |
| `provider_latency_p50/p95`、`downgrade_count` | provider 延迟与降级次数 | gateway |
| `rss_mb`、`disk_free` | 资源快照 | 复用 `resource_monitor::snapshot` |

### 象限 2 · 人机交互（最容易被忽略、价值最高）
| 指标 | 定义 | 说明 |
|---|---|---|
| `interrupt_count` | 单会话打断人类的次数 | 分规划期 / 执行期 |
| `approval_reject_rate` | 审批被拒 / 审批总数 | 高 ⇒ AI 判断与人类预期偏离 |
| **`clarify_skip_rate`** | 澄清被「跳过」/ 澄清总数 | **高 ⇒ AI 在「为问而问」**，直接闭环 §Human OS 硬约束 |
| **`wait_ratio`** | 人等 AI 时长 / AI 等人时长 | **量化协作模式选得对不对** |
| `human_correction_count` | 人类推翻或改写 AI 产出的次数 | 质量代理指标 |

### 象限 3 · 异常行为
| 指标 | 定义 | 红线 |
|---|---|---|
| `sandbox_denied` | 沙箱拒绝次数 | >0 即告警 |
| `retry_storm` | 同一操作连续失败重试 ≥ N | **G0 熔断** |
| `plan_thrash` | 同一步反复 replan 次数 | 对应历史 A3 replan 无界 |
| **`false_done`** | 宣称 done 但产物集为空 | **对应 dogfooding 已修的 `_has_outputs` 缺陷** |
| `gene_assert_fail` | G1 wiring 断言失败 | 🔴 阻塞 |
| `budget_overrun` | 成本超预算 | **G0 熔断** |

### 象限 4 · 效能趋势
| 指标 | 定义 |
|---|---|
| `roi` | 改动行数 / 成本（复用 `resource_monitor::compute_roi`） |
| `first_pass_rate` | 首次即通过验收的任务比例 |
| `rework_rate` | 返工任务 / 总任务 |
| `trend_delta` | 与上周同类任务对比的退化/改善 |

---

## §5 报告形态：双投影 × 三节律

### 双投影（同一份事实，两种表达）

| 读者 | 形态 | 消费方式 |
|---|---|---|
| **人** | 体检徽章（绿/黄/红）+ 异常卡片 + 趋势图；每条可点开看证据链 | 平时收在顶栏，异常才冒泡，**永不阻断操作** |
| **AI** | 结构化判例卡 JSON | 写入 `experience` 库，下一轮 `do_plan` 按 G2 时机召回 |

### 三节律

| 节律 | 触发 | 内容 |
|---|---|---|
| **实时告警** | 红线命中即刻 | 单条异常 + 证据 + 是否已熔断 |
| **会话复盘** | 会话结束 | 做了什么 / 代价多少 / 哪里返工 / 四象限快照 |
| **周期报告** | 日 / 周 | 跨会话趋势、退化项、同类任务对比 |

---

## §6 数据契约

### 观察结论（Finding）

```json
{
  "id": "fnd_01H...",
  "session_id": "sess_...",
  "quadrant": "interaction",
  "metric": "clarify_skip_rate",
  "value": 0.67,
  "threshold": 0.30,
  "severity": "warn",
  "statement": "本次 3 个澄清问题中 2 个被跳过，可能存在为问而问",
  "evidence": ["evt_0091", "evt_0104", "evt_0117"],
  "observed_at": "2026-08-02T00:12:03Z",
  "observer_version": "obs-1.0.0"
}
```

**硬约束**：`evidence` 非空；`value` 必须由确定性代码算出；`statement` 中出现的每个数字必须能在 `value`/`threshold` 或 evidence 事件中找到。

### 报告（Report）

```json
{
  "report_id": "rpt_...",
  "kind": "session_postmortem",
  "scope": {"session_id": "sess_..."},
  "health": "yellow",
  "findings": ["fnd_...", "fnd_..."],
  "metrics": {"cost_usd": 0.41, "wait_ratio": 3.2, "roi": 158.0},
  "meta": {"observer_cost_usd": 0.002, "lag_ms": 34, "dropped_events": 0}
}
```

`meta` 即 L4 元观察，**必填**。

### 新增事件类型（写入 AI OS 事件流）

| 事件 | 时机 | payload |
|---|---|---|
| `observation` | 产生 Finding | Finding 全量 |
| `observer.report` | 报告生成 | Report 全量 |
| `observer.circuit_break` | 红线熔断 | `{rule, evidence, action}` |
| `observer.health` | 观察者自检心跳 | `{lag_ms, dropped, alive}` |

---

## §7 前后端职责

| 能力 | 归属 | 说明 |
|---|---|---|
| 事件抽头与订阅 | **BE** | 只读 `Receiver`，独立任务 |
| 指标计算（确定性） | **BE** | 纯代码，禁 LLM |
| 异常检测规则 | **BE** | 阈值配置化，可热更 |
| 叙述生成 | **BE**（LLM，受限） | 只许改写已有数字 |
| 证据链维护 | **BE** | Finding ↔ EventId 索引 |
| 熔断执行 | **BE** | 唯一执行权 |
| 体检徽章 / 异常卡片 / 趋势图 | **FE** | 纯渲染 |
| 证据下钻（点结论跳到事件流对应行） | **FE** | 消费 `evidence` 数组高亮 AI OS 流 |
| 报告导出 | FE 触发 + BE 生成 | 与 `artifact.open` 同机制 |

**FE 不得自行计算任何指标**——否则会出现「前端算的 3 次、后端算的 2 次」这类双事实源。

---

## §8 UI 形态建议（避免第三栏认知负荷）

**不建议**做成常驻第三竖栏——三栏并列会让人的注意力被稀释，违背 Human OS「低认知负荷」原则。

建议形态：
1. **顶栏体检徽章**：一个圆点 + 一句话（绿「运行正常」/ 黄「2 项待关注」/ 红「已熔断」）。
2. **点开 = 抽屉/独立标签页**：四象限卡片 + 趋势 + Finding 列表。
3. **异常主动冒泡**：右下角 toast，不阻断，可忽略；红线熔断除外（必须确认）。
4. **证据下钻**：点 Finding → 右侧 AI OS 事件流自动滚动并高亮 `evidence` 中的事件行。

---

## §9 落地路线与验收闸门

| 阶段 | 内容 | 验收闸门 |
|---|---|---|
| **O1** | 新建 `crates/observer`，只读订阅事件流 + L1/L2 结构锁 | 类型层证明无回写通路；杀死 observer ⇒ 内核 fail-closed（有测试） |
| **O2** | 象限 1+3 确定性指标 + Finding 结构 + L3 证据强制 | 空证据 Finding 构造失败（单测）；指标与手工统计一致 |
| **O3** | 象限 2 人机交互指标 | `clarify_skip_rate` / `wait_ratio` 能从 v11 demo 事件流算出 |
| **O4** | 报告生成（会话复盘）+ 双投影 + L4 元观察 | 20 句抽检零幻觉数字；`meta` 必填校验 |
| **O5** | 红线熔断（G0）+ FE 徽章/抽屉/证据下钻 | 熔断可触发、可审、不可绕过；FE 零指标计算 |

**总闸门**：观察者产出的 Finding，随机抽 10 条，**100% 能凭 `evidence` 复现结论**；否则判定不中立，退回重做。

---

## §10 反模式清单（施工时禁止）

1. ❌ 把观察者塞进 `nervous-system`（混淆审计与干预）。
2. ❌ 让 LLM 直接「看日志写报告」（指标必须先由代码算出）。
3. ❌ 观察结论直接改写当前计划（越权，破坏 G2 边界）。
4. ❌ 给内核提供 `observer.disable()`（破坏 L2）。
5. ❌ 前端自算指标（双事实源）。
6. ❌ 报告只给人不给 AI，或两边口径不一致（破坏中立定义）。
7. ❌ 无证据的定性结论（如「本次表现良好」而无指标支撑）。
