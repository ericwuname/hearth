# 评审：ChatGPT《CORE Revalidation & SimUser Foundation Construction Order v2.1》复核与补充

> **复核人**：砺·评审　**日期**：2026-08-31
> **复核对象**：`docs/Hearth CORE Revalidation & SimUser Foundation Construction Order v2.1-chatgpt.md`（16 Node 长程总包）
> **参照物**：`docs/Hearth 拟真用户测试台与自反馈闭环 规划书 v2.0.md`、`docs/hearth-core-freeze-handoff-pack-v1.md`、`docs/hearth-p3-backlog-01-final-report-v1.md`
> **复核纪律**：不采信自述与外部 AI 给的行号；所有锚点源码实测；结论分级 CONFIRMED / 修正 / 补充 / 存疑。
> **总评**：**质量很高，建议采纳**。方向正确（先建测试台→观察→归因→最小修复→独立回归），且有 3 处优于我的 v2.0。**但存在 1 处严重遗漏、1 处内部矛盾须澄清、6 处补充**。

---

## 1. 源码复核结果（锚点实测，本机 HEAD = v0.2.19，与执行基线一致）

本机 `Cargo.toml version = 0.2.19`，与总包 §0.2 声明基线一致 → **grep 级复核可在本机直接做，无需 VM 同步**。

| # | 对象 | 报告锚点 | 实测 | 判定 |
|---|---|---|---|---|
| 1 | `do_plan_inner failed` → tracing::error! | loop.rs:2381 | **loop.rs:2385**（`tracing::error!`；调用在 2383） | ⚠️ **漂移 +4**，实质成立 |
| 2 | subscriber stderr 直写 | lib.rs:264 | **lib.rs:264** `.with_writer(std::io::stderr)` | ✅ 精确命中 |
| 3 | RC16 `HEARTH_LLM_URL` / `HEARTH_SERVICE_URL` | P3 §3 | config.rs:141 / 155 / 159 / 326 / 330 | ✅ CONFIRMED |
| 4 | RC18 `check_provider_url_mismatch` | P3 §3 | config.rs:183 / 193；测试 368 | ✅ CONFIRMED |
| 5 | RC25 `is_allowed_absolute_roots` | P3 §4 | tools-builtin: edit.rs:79 / grep.rs:95 / lib.rs:28 | ✅ CONFIRMED（确在**工具层**，与报告声明一致） |
| 6 | RC45 `act_verify_replan_count` | P3 §5 | loop.rs:956 / 1488 / **2786**(`< 1` 分账) | ✅ CONFIRMED |
| 7 | RC34 render `done()` 去重 | P3 §5 | render.rs:108 / 247；测试 253 | ✅ CONFIRMED |
| 8 | **T4 阈值（RC52 域）** | 总包 §4「不得直接修改 T4」 | **loop.rs:2642 `2 consecutive replans`——阈值仍 = 2** | ✅ **RC52 域确未触碰**，总包禁令前提成立 |
| 9 | `MAX_HISTORY_MSGS` | P3 §7 称 loop.rs:2203 | **loop.rs:2207** | ⚠️ **漂移 +4** |

**⚠️ 系统性发现**：两处行号均**恰好漂移 +4**（loop.rs 区域）。说明报告成文后该区域被插入过 4 行。
→ **执行纪律**：凡引用本批报告的行号锚点，**必须重新 grep 实测**，不得直接抄。

### 附：截断族实测（超出总账记录）

总账记"静默截断族 4 处"，实测硬编码截断点 **≥8 处**：

```
constitution.rs:80   MAX_CONSTITUTION_CHARS (6000)
context.rs:431       take(60)  ← 压缩摘要墓碑
loop.rs:1269         take(2000) final_statement
loop.rs:1665         take(80)   output summary
loop.rs:3752         take(200)  RAG 查询
agent-types/lib.rs:565 MAX_INJECTED_CHARS
codex-cli/client.rs:343 take(160)
codex-cli/lib.rs:661    take(40)
```

→ 支持补充项 **S7（截断族专项审计）**。

---

## 2. 执行报告内部矛盾（须执行窗口澄清，影响排期）

`hearth-p3-backlog-01-final-report-v1.md` 自相矛盾：

| 项 | §6 Node 04 表格 | §9 OPEN 残留 |
|---|---|---|
| RC24-B TC-8b | ✅ **完成**（EINVAL=0 + introspect 可用） | 列入"剩余 5 项" |
| RC2 渲染专项 | ✅ **完成**（零 contains 残留） | 列入"剩余 5 项" |

**同一份报告内，两项既标完成又列未完。** 未完成清单到底是 **3 项**（RC33 / 复测包 / RC29）还是 **5 项**，直接决定下一批工时估算与 Node 排期。

→ **要求**：执行窗口澄清后再进 Node；总包 Node 00 应把"复验批剩余项精确清单"列为基线索引的一部分。

---

## 3. ChatGPT v2.1 相对我 v2.0 的 3 处实质改进（我承认，建议采纳）

| # | 改进 | 为何优于 v2.0 |
|---|---|---|
| **A1** | **新增 `DRIVER-INDUCED` 归因层**（Node 04 六层→七层） | v2.0 只有机制/决策/模型/provider/环境五层。SimUser 一旦驱动 Hearth，**失败完全可能来自 PTY 驱动器自身**（输入竞争、超时、审批通道）。没有这一层，会把 driver 缺陷误判成 Core 缺陷——这正是"测试台变成自动真理机"的风险。 |
| **A2** | **Weak / Strong Acceptance 分层**（§20） | v2.0 的"四维验收"只验 **SimUser 保真度**，没验 **Core 就绪度**。ChatGPT 明确 Weak Acceptance 只能证明"结构完整"，Strong Acceptance 十项才是 Freeze 判据。这个区分是防自嗨的关键。 |
| **A3** | **Node 12「禁止只保存失败样本」**（每类须留 success / failure / counterexample） | v2.0 漏了。只存失败样本会造成**幸存者偏差**，让后续归因建立在偏斜样本上。 |

另：总包已正确吸收 v2.0 的 G1/G2 联合分布门禁（Node 10 B1）、5 个新检测器（B3）、material-dump / vague-delegation 场景（Node 07）、指标口径冻结与禁 grep（§2.4）。

---

## 4. 补充项（S1–S7，建议直接并入总包）

### S1 ·【严重遗漏】模拟用户必须**默认异源**

总包全文**未规定 SimUser 自身使用哪个 provider**。若 SimUser 与被测 Hearth 同用 Agnes，**同一模型的盲区会在用户侧与系统侧同时出现**，这类缺陷永远测不出来（验证悖论）。

> 建议插入 Node 01：
> ```text
> 驱动模型登记（强制）
>   默认：异源 provider（智谱 / 本地 Ollama qiyuan-8b）
>   同源：仅作「标注对照」条件，单独成组，用于分离「模型共模」与「系统缺陷」
>   每次 run 的 event log 必须记录 driver_model / target_model
> ```

### S2 ·【严重遗漏】Q11 单人语料边界必须前置为强制声明

Node 11/12 将跑 30–100 campaign 并产出成功率。而 persona 语料（4,681 条）**本质来自同一人类主体**——跨的是 provider 与场景，**不是用户**。若不前置声明，campaign 成功率极易被读成"真实用户成功率"。

> 建议插入 Node 11 前置条件：
> ```text
> campaign 报告头部强制附加：
>   「本 campaign 的 persona 分布来自单一人类主体的跨 provider/跨场景语料，
>     不代表真实用户总体分布；成功率不得表述为『真实用户成功率』。」
> 禁止词：真实用户成功率 / 用户总体成功率
> ```

### S3 · driver 超时 ≠ 用户放弃（Q4 校准）

实测：真实用户**显式放弃仅 1 次**（R9），绝大多数是**沉默离开**。总包 Node 01 driver 有 `timeout()` / `interrupt()`，若超时即记为"用户放弃"，会系统性高估放弃率。

> ```text
> driver timeout / interrupt 默认标记为 DRIVER-INDUCED 或 SYSTEM-STALL；
> 只有 persona 显式生成放弃语时才记 user_abandon。
> ```

### S4 · `approval_denied` 默认先归 DRIVER-INDUCED

P0 §P0.4 已确认「REPL 不可程序化驱动：非 TTY 审批降级」是**已知阻塞项**。PTY driver 若未正确处理交互审批，`approval_denied` 会大量假阳性。

> ```text
> driver 必须显式记录 approval 通道状态（interactive / non-interactive / denied）；
> campaign 中出现的 approval_denied 默认归 DRIVER-INDUCED，
> 需人工或对照证据才能升格为 Core defect。
> ```

### S5 · `anaphora_resolution_fail` 与 `escalation_run` 应**前移**到 Node 01

总包把 5 个新检测器全放 Node 10（Stage 2）。但 **Node 03 的 RC52 因果实验**（Contaminated / Resume 条件下"继续追问失败原因"）**核心正是指代依赖与上下文污染**——没有这两个检测器，Node 03/04 拿不到关键信号，RC52 归因会被推迟整整 9 个 Node。

> ```text
> 前移至 Node 01（Stage 1）：anaphora_resolution_fail、escalation_run
>   理由：直接服务 Node 03-04 的 RC52 归因，属 P0 关键路径
> 保留在 Node 10：multi_intent_drop、paste_error_probe_quality、longpaste_truncation
> ```

### S6 · 修复批后必须打 tag，campaign 必须绑定 tag

Node 05/06 修复 → Node 11 campaign；Node 13 修复 → Node 14 终验。总包未要求修复后打版本 tag。这正是总账已教训过的"同一批数据横跨多个 binary"归因污染。

> ```text
> 每个 fix batch 完成后 → 打版本 tag → 在 decision-status.md 登记
> campaign 报告必须声明所测 tag；跨 tag 数据不得混在同一张指标表
> ```

### S7 · 新增：静默截断族专项审计（总包无专门 Node）

实测硬编码截断点 **≥8 处**（总账只记 4 处），且 P0 已确认"内容截断（BUG-012）"是盲测暴露的真实缺陷。总包 `truncated_output` 检测器只做**检测**，没有针对截断点本身的**审计**。

> 建议在 Node 02 增补 D 小节（或并入 Node 09）：
> ```text
> 截断族审计（枚举 + 逐点三问）
>   枚举全部硬编码截断点（实测 ≥8 处）
>   逐点验证：①是否有截断标记 ②用户是否可感知 ③是否可恢复
>   已知候选：constitution.rs:80 / context.rs:431 / loop.rs:1269,1665,3752
>             agent-types/lib.rs:565 / codex-cli/client.rs:343 / lib.rs:661
> ```

---

## 5. 对「file_issue = DEFERRED」的裁决

**总包 §17 判定 file_issue 推迟**，理由：先证明 SimUser 与检测器会产生哪些高价值问题，再决定哪些事件值得进 Core issue 通道。这与我 v2.0 §8「包 C（FIC-01）随 v0.2.19」冲突。

**裁决：同意推迟，但附一个条件。**

理由：campaign 期间，**双通道日志 + 15 个检测器已经能在外侧确定性捕获** `stalled` / `give_up` / `approval_denied` / `verify_failed` / 工具五态异常这五类事件。Core 侧 file_issue 在测试期确实是**冗余观测**，先建不划算。

**附加条件**：这五类事件的**分类法必须在 Node 01 检测器层先跑通并用真实数据验证**（作为 file_issue 的 dry-run）。这样等 file_issue 真要进 Core 时，事件分类法已被实证校准，而不是拍脑袋设计——也就保住了 v2.0 §5.2「确定性触发兜底」的真正价值。

> ```text
> file_issue = DEFERRED（同意）
> 但：五类确定性事件分类法 → 转为 Node 01 检测器层验收项之一
> ```

---

## 6. 一处文档引用纠错

总包 §1 引用 `docs/拟真用户测试台与自反馈闭环规划书 v2.0`，实际文件名为：

```
docs/Hearth 拟真用户测试台与自反馈闭环 规划书 v2.0.md
```

若后续有脚本或 agent 按该路径读取会失败，建议更正。

---

## 7. 结论

| 项 | 结论 |
|---|---|
| 总方向（先建测试台→观察→归因→最小修复→独立回归） | ✅ **采纳** |
| A1 DRIVER-INDUCED / A2 强弱验收 / A3 禁止只存失败样本 | ✅ **采纳，优于 v2.0，我并入** |
| file_issue = DEFERRED | ✅ **同意，附 dry-run 条件** |
| S1 异源 provider / S2 Q11 声明 | 🔴 **必修**（不补会导致共模盲区与分布误读） |
| S3 超时≠放弃 / S4 approval_denied 归 driver / S5 检测器前移 / S6 tag 绑定 / S7 截断族审计 | 🟡 **建议并入** |
| 行号锚点漂移 +4 | ⚠️ **引用前必须重测** |
| P3 报告 §6 vs §9 矛盾 | ⚠️ **须执行窗口澄清后进 Node** |

**一句话**：ChatGPT 的 v2.1 在**方法论与治理**上比我的 v2.0 更严（DRIVER-INDUCED、强弱验收、反幸存者偏差），我采纳并并入；但在**测试台本身的构造细节**上漏了我 v2.0 已有的两条硬约束（异源、Q11），须补 S1/S2 后才能下发。

*砺·评审零代码改动；全部锚点本机 v0.2.19 源码实测。*
