# R2-C ContextBuilder Final Acceptance

**日期：2026-08-29**

## 裁决

**R2-C ContextBuilder v0.2.9 正式验收通过（FINAL ACCEPTED）。**

本次施工完成既定 L1-L4 ContextBuilder 分层重构，核心结构性不变量均已获得单测及运行证据支持；未发现足以触发 rollback 的 ContextBuilder 结构性缺陷。

## 已闭合

* L1/L2 stable system 构造完成。
* TaskGraph topology 与动态 status 分离完成。
* `topology_sig` 与 `last_graph_sig` 正确分立。
* Task Continuity 保持唯一注入路径并位于 L4 尾部。
* Experience 按批准的 X 方案保持动态语义，不再错误宣称 task-stable。
* `[FAILED]` 事实保全完成并有单测锁定。
* compact / resume 边界获得回归测试覆盖。
* message prefix hash-chain 已具备可验证性。
* tool schema 顺序漂移已定位并修复；同 agent / 同 phase 下 tools hash 已证实稳定。
* ContextBuilder 与 D 类禁改事项边界保持不变。
* 全量隔离门禁最终实测保持全绿。
* EC-03 B/C 独立盲评完成。
* DeepSeek 同口径 cache 对照完成。

## Cache 结论

本轮 pre/post 观察到：

* token-level：64.9% → 74.5%
* request-level：74.1% → 88.9%

该结果支持 stability 改造及 deterministic tool ordering 对 cache 行为具有正向影响，但不将全部改善量单独归因于 ContextBuilder。

provider cache segment / granularity 仍为 UNKNOWN。

## 已降级/修正的结论

T4 不再作为 Task Continuity 正向行为证据。

独立盲评确认 B/C 均受到 rm approval gate 阻塞，因此此前“B 实质完成、C 失败”的 T4 结论撤销。

T2 多步任务仍提供有效的 B >> C 行为证据。

## 独立遗留问题

以下事项不属于 R2-C ContextBuilder 回归，不阻塞本次验收：

1. **Tier3 TaskGraph 活性判据缺陷**：TaskNode 状态无法反映工具实际生产结果，导致 Reflect 虚假 0/N → Replan → Stall。该问题为下一施工单的最高优先级。
2. **rm approval gate one-shot 阻塞**：归入 D 类控制流问题。
3. **Q-1 topology diff observability**：OPEN。
4. **Q-2 telemetry depth/role 维度**：OPEN，用于隔离主 agent 与 read-only sub-agent 的观测口径。
5. provider cache segment / granularity：UNKNOWN。
6. rubric 全量 5 任务扩展：可选，不作为 R2-C 验收条件。

## 边界

R2-C 至此停止施工。

不得以遗留 OPEN / UNKNOWN 项为理由重新打开 ContextBuilder 架构；除非后续出现新的直接证据证明 R2-C 核心不变量被破坏，否则 v0.2.9 ContextBuilder 作为当前稳定基线保留。

**Final Acceptance：APPROVED.**

---

# 守门员批注（2026-08-29 · Final Acceptance 复核：无异议，附收官配套 5 条）

> 复核实证：commit `b4433ea` 在库（全库 278 commits）；pre/post 数据 `docs/data/cache-post-toolssort-20260829.jsonl` 在库；VM 隔离门禁 387 全绿此前已实证。**本验收守门员无异议**——证据链（EC → Pre-Construction Decision → Construction Order v1.1 → 施工 → Closure Window → Final Acceptance）完整且每步带实测。以下 5 条为收官配套。

## 补充 1：cache 提升的归因口径——接受"组合归因"，不做拆分实验

74.5% / 88.9% 的提升来自**两个同轮变更**（ContextBuilder 分层 + tools 排序修复），严格单变量归因需"v0.2.9+随机顺序"中间对照——成本大于收益。**裁决：接受组合归因**，文档标注"提升由 stability 改造与 deterministic ordering 共同贡献，未做单变量拆分"即可，正文表述已达标。

## 补充 2：T4 证据恢复路径——挂在 RC24 修复单的验收项上

T4 换向证据被 rm 审批门阻塞（盲评抓出）撤销——但这是 **RC24 已知问题**（one-shot 无交互通道 → 破坏性审批门静默阻塞）。恢复路径明确：**T4 复测改在 repl 模式跑（有交互通道，可批准 rm）**，或待 RC24/审批门 D 类单修复后复测。把"T4 换向 Continuity 证据复测"写为该 D 类单的验收项之一——证据可恢复，不丢。

## 补充 3：下一轮优先级队列定版（供顶层派工）

```text
1. T3 TaskGraph 活性判据（最高——RC1 家族实证，R2-D TaskGoal 价值兑现的前提，
   Continuity 的 completed/remaining 在真实任务中失真）
2. rm approval gate one-shot 处置（RC24；兼 T4 证据恢复前提）
3. Goal Revision 三分类接入（T5 问询路由行为证据，classify_user_input 已备）
4. bash stdout 截断（57G 结构缺陷，设计稿已备）
5. filesystem write accounting（降级环境无兜底）
6. resource 超限控制流（最后）
7. Q-1 拓扑 diff / Q-2 depth 维度（telemetry 增强，随就近施工单顺带）
```

## 补充 4：v0.2.9 资产化配套（稳定基线的四个动作）

①**release tarball 打包**（施工报告标"待打包"，补上）；②**打 `v0.2.9` tag**（方案 B：发版节点打 tag——这就是发版节点）；③**补 CHANGELOG**——实测 v0.2.5–v0.2.9 **五个版本无 CHANGELOG**（根目录止于 v0.2.4），违反历史惯例，建议补一份汇总（R1 终态/Tier3/EC/ContextBuilder 四波），对外资产不能断档；④双 VM `/usr/local/bin/hearth` 0.2.9 已在位 ✅。

## 补充 5：里程碑复盘触发（用户工作流惯例）

R2-C Final Acceptance = 大里程碑（ContextBuilder 分层落地 + cache 证据链闭环）。按惯例触发**全局 re-survey**：08-27 的全面审查报告基线数据已经三次过时（27→27 crates 不变但版本/测试数/能力面全变），建议下一份全面审查更新后再定下一主线（T3 修复 vs 桌面接真流 vs Tier3 遥测扩容）。
