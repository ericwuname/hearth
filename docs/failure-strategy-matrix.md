# Failure → Strategy Matrix — Node 03（P1-FAILURE-ADAPTATION-01）

日期：2026-08-30　依据：Node 01 源码审计（docs/failure-adaptation-flow.md）+ Node 02 taxonomy（terminal.rs F1-F10）+ 历史真机证据。

## 1. 最终矩阵（v1 定稿）

| 类 | 名称 | 判据（结构化输入，全部可审计） | 默认策略 | Retry? | Replan? | 等待/切通道? | 人工? | 循环内既有通道 |
|---|---|---|---|---|---|---|---|---|
| F1 | transient_provider | provider 层网络/429/5xx（is_network） | backoff + **bounded retry** | ✅ | 通常否 | ✅ | 可选 | provider 内部重试；loop 层无显式 retry |
| F2 | tool_execution | 负 exit code（信号终止/seccomp SIGSYS） | diagnose tool | 条件（仅环境性信号） | 条件 | 否 | 条件 | consecutive_errors 计数 + force_param_gap |
| F3 | environment | timeout/deadline（is_timeout，含 TaskDeadlineExceeded） | diagnose env；**deadline 权威，不绕过** | 通常否 | 条件 | 否 | ✅ | dispatcher 单点 effective=min(declared,remaining) |
| F4 | permission/approval | 审批门拒绝（approval_denied） | escalate（request/deny/delegate） | **否** | 否 | 否 | ✅ | RC24 审批通道（delegated_approvals） |
| F5 | assertion_or_test | 正 exit code（cargo test fail 等） | inspect + **repair** | 条件（修复后复测≠盲试） | 条件 | 否 | 否 | Act 回喂 / acceptance failed 回喂（:4008/:4468） |
| F6 | verification | 验收核验 failed（acceptance_result=failed） | inspect acceptance → 修复 | **否**（INV-FA01-F） | ✅ | 否 | 条件 | Done Reserve（≤1）+ GiveUp 拦截 Reserve（≤1，共用） |
| F7 | plan/strategy | same_tool_repeat≥2（同工具反复失败） | **replan** | 否 | ✅ | 否 | 否 | planner Replan 臂（cap 3）+ graph_stall_count（cap 2） |
| F8 | resource/budget | budget_remaining==0 / Reserve 耗尽（budget_exhausted） | stop / bounded reserve | 否 | 条件（仅当有 Reserve） | 否 | 条件 | budget-low GiveUp 臂 + WS9 预算 ask |
| F9 | model_judgment | 验证步失败但 LLM 自述成功 | record conflict + verify（不信自报） | 否 | 条件 | 否 | 条件 | REFLECT_FACT_CONFLICT 观察 + classify_reflect_fact_conflict 六分类 |
| F10 | unknown | exit_code=None 且无其他结构化信号 | bounded fallback（最多有限次） | 最多有限次数 | 条件 | 否 | ✅ | 无专用通道 → fallback 有限步后落 Replan/GiveUp 计数器 |

## 2. 矩阵三条裁决依据（源码事实 + 历史证据）

1. **F1 允许 retry 的唯一性**：全表只有 transient 允许盲重试——历史上 429/网络重置
   由 provider 层消化（Node 01 审计：loop 层无显式 retry 决策点）。**禁止统一 retry**
   红线由"其余九类默认非 Retry"保证。
2. **F5/F6 的 Repair 与 Replan 分界**：F5 有可执行修复信息（编译错误/断言输出）→
   repair（回喂失败明细，既有通道 :4008/:4468）；F6 是"计划本身不适用"（验收标准
   不满足且无修复信息）→ replan。两者都**消耗 Reserve/步数**，无免费步。
3. **F9 不引入 LLM judge**（STOP-9）：model_judgment 冲突由既有确定性六分类
   `classify_reflect_fact_conflict`（loop.rs）承载，只观察不决策。

## 3. 与 loop 消费端的接线（Node 06 落地）

矩阵以纯函数 `failure_strategy(FailureKind, RecoveryContext) -> RecoveryStrategy`
落码（terminal.rs），单测锁表。循环消费端为**观察级接线**（scratch telemetry +
give_up 前分类留痕），不改变既有 replan/give_up 控制流（STOP-6 边界内）。
