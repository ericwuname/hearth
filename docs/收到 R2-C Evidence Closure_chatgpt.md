收到 R2-C Evidence Closure。

我审阅了 EC-01~08 以及当前 v0.2.8 / `7066ab3` 基线。总体判断：**R2-C 证据闭环已基本成立，但现在不要施工 ContextBuilder。先完成唯一未闭合的 EC-03 Intent Benchmark，然后基于结果做一次施工前裁决。**

请严格按以下顺序执行，不要自行扩大范围：

### 1. P0：先跑 EC-03 Intent Benchmark

按 R2-C §9 已定义的最小集执行：

* 5 类 × 5 任务

  * 歧义指令
  * 多步目标
  * 隐含约束
  * 中途换向
  * 纯问询
* 每个任务执行 A/B/C/E：

  * A：原始用户输入，无 Hearth context
  * B：当前 Hearth context
  * C：精简 Hearth context，按当前设计近似手工删减
  * E：同模型 Codex 外部参照
* Agnes 通道执行。
* 记录：

  * intent preservation
  * plan alignment
  * tool selection
  * unnecessary assumptions
  * goal drift
* 明确区分：

  * 实测事实
  * 人工评判
  * inference
* 不要把小样本结果包装成统计学结论。

特别关注两个反例：

1. R2-D Task Continuity 是否确实帮助模型保持任务语义；
2. C 的“精简 context”是否在减少动态污染后损失了必要任务信息。

### 2. EC-03 完成后，不要立即施工 ContextBuilder

先输出一份 **R2-C Pre-Construction Decision**，回答三个问题：

**Q1：方案 B 是否继续成立？**

即：

> TaskGraph 拓扑 = task-stable
> TaskGraph 状态 = dynamic / Continuity

如果 benchmark 暴露出模型需要完整 TaskGraph 才能正确理解，则记录为反例，不要为了 cache 强行拆。

**Q2：Experience 是否继续放 L2 Project stable？**

当前源码证据是 confirmed：任务期间只首轮检索、不重新检索。

除非 benchmark 产生实际行为反例，否则保持该裁决，不重新设计。

**Q3：L4 Dynamic 最终应该放什么？**

只允许放真正 turn-dynamic 的内容。

当前候选：

* history
* 最新用户输入
* Task Continuity
* LSP / observe 动态结果
* 其他经过证明确实会随 turn 变化的内容

不要因为“理论上属于动态”就把 task-stable 内容继续塞进去。

### 3. Cache 结论保持当前证据等级

不要把：

> TaskGraph 每步变化 + system_hash 每步变化

升级成：

> TaskGraph 已确认是 cache miss 主因。

当前必须保持：

> TaskGraph 注入是稳定前缀污染的 confirmed structural fact；它是 cache miss 的 likely cause，而不是 confirmed root cause。

除非取得 provider cache segment-level evidence，否则不要升级证据等级。

同样：

* MISS-C history growth = confirmed 正常行为
* MISS-B tool schema = unlikely
* MISS-D serialization = unlikely
* MISS-E provider cache 粒度 = unknown
* MISS-F = unknown

这些分类不要为了形成漂亮结论而强行闭合。

### 4. 57GB：停止继续考古，进入工程处置准备

当前结论已经足够：

* Tool Payload Explosion：confirmed structural defect
* Filesystem Artifact Explosion：仍有盲区
* ResourceLedger：只覆盖 args + result payload
* filesystem write bytes：尚未观测

下一步只做设计，不做控制流施工：

**A. bash stdout 截断**

* 明确默认上限
* 明确截断后的 ToolResult 语义
* 保留“发生过截断”的事实
* 不实现 kill/pause/deny

**B. filesystem write accounting**

* 明确观测边界
* 不要把它和 Tool Payload Bytes 混成一个指标
* 设计独立的 Filesystem Artifact Bytes
* 不实现 pause/deny/terminate

这两项以后分别进入 D 类施工单。

### 5. Goal Revision 暂时不要施工

保留：

`classify_user_input(&str) -> InputClass`

以及：

> “查看状态 / 看 diff / 发生什么了 / 继续 / 怎么样了”

必须归类 Task Control，revision 不增加。

但本轮仍然：

**不要修改 `apply_turn_goal` 控制流。**

它是 D 类事项，等待独立施工单。

### 6. ContextBuilder 本体暂缓

在 EC-03 完成前：

**禁止修改 `build_messages()`。**

不要提前：

* 移 TaskGraph
* 移 experience
* 改 system_text
* 改 history
* 改 tool schema
* 新增第二套 continuity 注入
* 改事实源

R2-D 已有 `task_continuity_message()` 是唯一注入路径，这个不变量继续保持。

### 7. Bridge 保持 DEFER

当前裁决继续：

> INTENDED → DEFER

re-entry：

> B4-2 桌面真流开工

本轮不要接 bridge，也不要删除依赖。

---

### 本轮最终交付物

请只交付：

1. **EC-03 Intent Benchmark 实测结果**
2. **R2-C Pre-Construction Decision**
3. **bash stdout 截断 D 类施工设计**
4. **filesystem accounting D 类施工设计**
5. 更新后的 Evidence Closure 状态表
6. 明确列出仍然 OPEN / UNKNOWN / DEFER / D 类事项

然后**停在这里，不施工 ContextBuilder。**

施工前必须满足：

> EC-03 完成
>
> * ContextBuilder 方案没有被 benchmark 反证
> * Evidence 等级没有被夸大
> * D 类控制流继续隔离

最后再次强调：

**这轮目标不是“把 R2-C 做完”，而是把 ContextBuilder 的施工前证据链闭合。不要为了完成度提前施工。**

---

# 守门员批注（2026-08-28 深夜 · EC 交付独立复核后补充，与正文同效力）

> EC 交付已独立复核：commit 链（4997f5b/7066ab3/182bdbc）在库、41 行采集数据在库、事故归档 `docs/incidents/2026-08-28-tier3/` 存在且 README 诚实披露缺失证据、9.5/10 通过标准核对属实。以下 6 条随本指令执行。

## 补充 1（先澄清）：v0.2.8 的门禁计数疑点——Tier3 负面测试去哪了

VM `~/t_gate.log` 实测当前 **TOTAL_PASSED = 370，与 v0.2.7 完全相同**。Tier3 T2/T3/T4 修复（v0.2.8 已合入）按任务书各带负面测试（mock provider 120s 收口 / 畸形 body 快速失败 / 停滞熔断）——若已落，计数应 >370。两种可能：①t_gate.log 是 v0.2.7 的旧门禁，v0.2.8 未重跑全量；②负面测试未真正落地。**执行窗口第一件事：补跑全量 `cargo test --workspace` 落新 RC 行，并说明 Tier3 三项修复的负面测试落点与当前计数**（本条不是打回——是门禁证据链例行闭合，`4997f5b` 只见 T4 测试参数修正，T2/T3 的测试落点需交待）。

## 补充 2：EC-03“5 类 × 5 任务”口径歧义——钉死为最小集

正文写"5 类 × 5 任务"字面可读成 **25 任务**。钉死口径：**最小轮 = 5 类各 1 任务 = 5 任务 × A/B/C/E ≈ 15-20 次完整任务跑**；25 任务全矩阵留作后续加厚轮。否则执行窗口可能一头跑成 25 任务（VM 排期爆掉）或自行砍到不足。

## 补充 3：E 变体的环境前提——先确认 Codex CLI 可用，且“同模型”必须真同

E 变体（同模型 Codex 外部参照）执行前先核实：VM/本机是否已装上游 Codex CLI，且其 provider/model/key 配置与 Hearth **逐字一致**（同 provider 同 model 同参数）——配置不一致则"同模型"对比失真，整个 benchmark 的 E 列作废。若环境不可得，E 变体降级为"API 直连裸 prompt"近似并在报告标注口径差异，不许假装等价。

## 补充 4：评分 rubric 统一 + 双评防偏

"区分实测事实/人工评判/inference"落成可执行形态：6 指标（intent/plan/tool selection/assumptions/drift/intervention）用 **0-2 分制统一 rubric 表**逐任务评分；执行窗口初评后，**抽 2 个任务交第二窗口复核**（防单点评分偏差）；5 任务小样本只出方向性结论（正文已禁包装，落成 rubric 纪律）。

## 补充 5：B 变体一鱼三吃 + compact open 项补法

EC-03 的 B 变体（当前 Hearth）每任务跑时：①开 `HEARTH_CACHE_TELEMETRY=1`（顺手补 EC-01 的"compact 前后"open 项——多步目标任务若触发压缩即得样本）；②记录 Delegation Friction（人工干预次数）——EC-03/cache/friction 三个数据面一次采集。

## 补充 6：T1 补档两件未完动作（README 已诚实披露，别烂尾）

①B04/B06 原始响应体不可复得（进程已死、未开 trace）→ 按 Tier3 T1 动作 2 用 RUST_LOG=trace 复现 1-2 次补档；②故障轮次对应的 `<sid>.taskgoal.json`/`graph.json` 从 VM `~/.config/hearth/sessions/` 提取补档时，**核对 uuid↔B 轮次映射**（当前 sessions/ 已有 3 组，是否覆盖 B03/B04/B06 未确认）——T5 的 13-17 步分析依赖这份映射。
