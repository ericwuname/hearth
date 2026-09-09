顶层批准：**R2-C ContextBuilder Construction Order v1.1 正式进入施工。**

批准口径固定如下：

1. **Experience 选择方案 X**

   * `injected_experience` 按现有真实语义视为条件性 turn-dynamic。
   * 从 system 动态区迁移至 L4。
   * **不得改成 Y**：禁止提前检索、禁止改变“连续错误≥3后检索/每轮清空重填”的既有行为。
   * 禁止用空槽位制造“假稳定”；不得出现首轮 None、后续再填导致 prefix 漂移而未被检测的情况。

2. **TaskGraph 使用独立 `topology_sig`**

   * 只 hash `id/description/deps`。
   * `last_graph_sig` 保持 T4 停滞检测用途，**不得复用**。
   * L2 Task Topology 仅在 `topology_sig` 变化时重建。
   * status/result/remaining/next 等动态状态全部由既有 `task_continuity_message()` 路径承载。

3. **ContextBuilder 目标**

   * 将 `build_messages()` 从当前串行拼接重构为 L1-L5 分层组装。
   * 不建立第二套 TaskGraph/TaskGoal/事实源。
   * 只改变事实的分层与位置，**不得丢失任何执行所需事实**。
   * `[FAILED]` 等失败事实必须进入 Continuity，并增加对应回归测试。

4. **L4 固定顺序**

   * history
   * retrieval_context
   * LSP diagnostics
   * Experience（方案 X，turn-dynamic）
   * Task Continuity（最终语义锚点，必须最后）

5. **tool schema**

   * 本施工单禁止 phase pruning。
   * `ChatRequest.tools` 保持全量稳定。
   * 按 v1.1 要求保留/补齐 `tool_schema_hash` telemetry。

6. **三层 stability 验收必须同时实现**

   * A：`system_hash` 同 goal session 内应稳定为 1。
   * B：逐消息 hash chain 验证 message prefix stability。
   * C：provider cache outcome 单独观察。
   * A/B 达成而 C 不改善时，**不得因为 provider 黑盒结果回退 ContextBuilder**。

7. **D 类全部冻结**
   本次 diff 禁止涉及：

   * bash stdout 截断及其控制流
   * filesystem write accounting
   * Goal Revision 三分类接入 `apply_turn_goal`
   * resource 超限 pause/deny/terminate
   * resource-monitor 接线
   * tool schema 相位裁剪
   * bridge
   * subagent
   * TUI

8. **环境与门禁**

   * 基线：v0.2.8 / 当前批准 commit 链。
   * VM 运行必须使用 `HEARTH_ALLOW_NO_CGROUP=1`。
   * **只允许使用 `~/run_gate_r2c.sh`，禁止 `~/run_gate.sh`。**
   * 开工先实测门禁，记录新的 baseline；不要把历史 375 passed 当作当前实测结果。

9. **施工顺序**

   * 先实现分层组装器及最小单测。
   * 再接入 `topology_sig`。
   * 再迁移 Experience → L4。
   * 再完成 Continuity 状态收编。
   * 最后做 telemetry + benchmark 对照。
   * 每一步保持可编译、可测试、可回退。
   * 不得一次性大改后再找问题。

10. **必须验证的核心回归**

* 同 goal 两次 `build_messages()`：system_text 字节完全一致。
* TaskGraph status 改变：system_text 不变、Continuity 更新。
* topology 改变：L2 topology 正确重建。
* status 改变但 topology 不变：L2 不重建。
* Experience 在现有触发语义下变化：只影响 L4，不污染 stable prefix。
* compact 后 Continuity/Topology 仍存在。
* resume 后分层结构保持一致。
* 无 Experience 时不得出现“先空后填”的隐性 prefix 漂移。
* `[FAILED]` 节点事实保全。
* 全量门禁保持 0 failure。

11. **cache 对照**
    施工完成后严格复跑 EC-01 同口径：

* 同 4 个任务
* Agnes
* 同 TelemetryProvider v2
* `HEARTH_ALLOW_NO_CGROUP=1`
* 对比 token-level hit rate、request-level hit ratio、system_hash 唯一值、message prefix hash chain。
* 不得跨口径比较百分比。
* provider segment 粒度仍标记 UNKNOWN。

12. **Intent 回归**
    完成 EC-03 B/C 的 5×2=10 跑复测。
    重点确认 T2/T3/T4 的执行能力没有因 ContextBuilder 重构下降，T5 当前问题继续留给独立 Goal Revision D 类施工单。

**执行纪律：**
这次不是重新设计 ContextBuilder，而是执行已经批准的 v1.1。
如果施工过程中发现需要改变上述架构裁决，立即停在该点并提交“偏差报告”，不要自行扩 scope。

施工完成后只交付：

* diff/commit
* 实测门禁结果
* 单测结果
* EC-01 cache 对照
* EC-03 B/C 复测
* stability A/B/C 结果
* deviations / UNKNOWN / OPEN

**现在开始施工。**

---

# 守门员批注（2026-08-29 · 批准书复核 + 源码实证，与正文同效力）

> 批准前关键声明已独立复核：**Experience"错误降级通道"源码实锤**——`loop.rs:1976-1983` 的 v19.0 注释自证："only after `consecutive_errors >= 3` … **a degradation channel, not a constant enhancement**"，且 `:1983` 每轮清空重评。EC-05 的 "task-stable confirmed" 确认推翻，方案 X（保语义、移 L4）选择正确。v0.2.3 治理双 VM 实证收口（交互 shell 两台均 `hearth 0.2.8`）。以下 4 条随施工执行。

## 补充 1：EC-05 勘误必须回填两处历史文档

"task-stable confirmed" 的旧结论还留在：①`hearth-r2c-evidence-closure.md` §C.2 与 F 表（"Experience 上移 stable / confirmed"）；②`hearth-r2c-preconstruction-decision.md` Q2（"维持 confirmed 未动摇"）。两处各加一行勘误注记指向本批准书（"条件性 turn-dynamic，v1.1/批准书更正"）——防后续引用旧结论。施工单 v1.1 本体已改，不重复。

## 补充 2：方案 X 的 prefix 链细节——"experience 出现/消失"是 expected-dynamic

方案 X 下 `injected_experience` 为 None→Some 时，其消息插入在 history 之后、Continuity 之前——**N-1 序列不再是 N 的前缀**（Continuity 位置后移一格）。这不违规（L4 本就是 dynamic 区），但 **B 层 prefix-hash 链测量必须把"experience 出现/消失"标为 expected-dynamic 事件**，不计入回归告警——否则施工后第一次触发降级检索就会被误报 prefix 回归、误触回退。写进 §六 B 层验收口径。

## 补充 3：v19 设计语义备忘（本轮不动，留档）

experience 条件注入是有 benchmark 背书的刻意设计：v17 弱模型 +20pt / v18 强模型 −5pt（noise 污染）。当前 agnes-2.5-flash 属强属弱未知——若未来 benchmark 发现强模型下注入有害，调优阈值属于**行为变更 = D 类**，本轮禁止顺手改。此备忘防止施工窗口把"降级通道"当成待修缺陷。

## 补充 4：v0.2.3 PATH 治理收口确认（环境前提）

双 VM 交互 shell 实测 `bash -lc 'hearth --version'` → **均 0.2.8**（`~/.cargo/bin/hearth` 旧副本已删）。`docs/vm-env-path-fork-2026-08-29.md` 记载的三层二进制分叉正式闭环——施工期间的版本核验只需 `hearth --version` 一条命令。
