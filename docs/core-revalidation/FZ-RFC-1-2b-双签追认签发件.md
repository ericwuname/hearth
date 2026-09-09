# FZ-RFC-1 / FZ-RFC-2b 双签追认签发件

> **目的**：闭合 COND-6（放行令 §2：FZ-RFC-1/2b 双签追认，时点=Step 2 campaign 运行期间，Node 15 冻结包归档前硬门槛——未闭环则 Node 15 顺延）。
> **性质**：两项冻结区改动均已实施并进入 v0.2.23 发版（commit `117a947`），本件为**事后追认**（retrospective ratification）——签认 = 确认"变更申报内容、实施证据、不变量清单"三者一致且授权链完整。
> **制作**：执行窗 2026-09-02（依裁决书 §4.5 准备，供 campaign 期间签署）

---

## RFC-1 · 生产沙箱 env_clear + 最小白名单

| 项 | 内容 |
|---|---|
| 触达冻结区 | `crates/sandbox/src/lib.rs`（spawn 路径） |
| 申报 | `.env_clear()` + 最小白名单（PATH/HOME/LANG/LC_ALL/TMPDIR）+ `tool_env` 注入后置覆盖；禁透传 `*KEY*/*TOKEN*/*SECRET*` |
| 风险定性（实测更正） | "key 直透子进程"🔴 不成立（`tool_env` 只注入两个 HEARTH_* 变量）；真实风险 = 纵深防御缺口 🟡——无 env_clear 兜底。降级但仍实施（成本低） |
| 实施证据 | s4_tests 2 个：白名单 6 项精确性 + 父进程 SECRET 不可见；gate 486/0 |
| 不变量 | dev `NoopSandbox:185` env_clear 语义对齐；非交互审批语义不变 |
| 完整规格 | `docs/p5-foundation/specs.md` §4 |

## RFC-2b · RC52 give_up 三臂统一消费端

| 项 | 内容 |
|---|---|
| 触达冻结区 | `crates/agent-core/src/loop.rs`（Reflect GiveUp 臂 + T4 stall 臂 + budget exhausted 臂） |
| 申报 | 统一 helper `rc52_route_done_if_session_artifacts()`：criteria 空 + 0 errors + session 级产物 → 回填路由 Done；`completion_fact_check()` 前移防无关产物误判；budget 臂锁存 `fa01_budget_intercepted` 防死循环 |
| 实施证据 | RED 集成测试 `test_rc52_hydration_integration_through_run_boundary` 先红后绿；A 组熟悉版盲测三臂路由生产生效 16+ 轮；B 组正式轮 UNVERIFIED 投影验证（裁决书 T-3/T-4 抽验） |
| 不变量 | T4 阈值=2 不动；planner schema 零改动（STOP-1/2）；criteria 非空路径零改动；权威不复制（只读回填）；有界停止保留 |
| 残留 | B 臂（无产物续作）不触发路由 = OD-1 🟡（v0.3 候选），已显式登记 |
| 完整申报 | `docs/core-revalidation/FZ-RFC-2b-RC52三臂统一消费端.md` |

---

## 授权链

1. **FZ-RFC-2b**：顶层 2026-09-01 口头批准（"B 相二次修复批准即实施" + 全权委托），书面追认待——**本件签署即书面追认**。
2. **FZ-RFC-1**：P5-FOUNDATION-01 总包 §4 N00 规格冻结（顶层签发）内授权，实施于 v0.2.23。

## 签署

| 签位 | 意义 | 状态 |
|---|---|---|
| 执行窗 | 确认实施内容与申报一致、证据真实（先红后绿 / gate 486/0 / 盲测可观察） | ✅ 已签（执行窗 2026-09-02） |
| 顶层（人工窗） | 书面追认授权链与变更内容 | ✅ **准签**（2026-09-03，凭证＝`docs/core-freeze-candidate/冻结解冻派工单顶层签发件 v1.0（顶层）.md` **T-3①**）<br>**原阻塞项 S-2 已移除**：spawn 级集成测试 `crates/sandbox/tests/s4_spawn_env.rs` 已落地并通过，四断言全过（白名单可见／父进程 SECRET 不透传／**`HEARTH_EGRESS_ALLOWLIST`+`HEARTH_READ_ROOTS` 不被 `env_clear` 清掉**／`PATH` 在），已随全量门禁跑通（分支树 493/0）。顶层签发前抽验 6/6 全中（签发件 §〇）。<br>⚠️ **RFC-1 仍为双签制——外部签位到位前不产生完整效力** |
| 外部 AI 窗 | 独立追认（可交叉核对本件所列证据与源码） | ⬜ 待签 —— **当前唯一未签位**；材料包 `docs/core-freeze-candidate/external-review-pack-v1.md`（Q1–Q6 预注册），用户转发即触发 |

> 签署纪律：任一签位否决 → 变更回退评审；两项均须三签齐备方可进 Node 15 冻结包。
>
> **执行窗登记记录（T-3① 配套施工，2026-09-03 20:15）**
> - 顶层签位状态由 ⬜ 未签署 改为 ✅ 准签，凭证为本文件所引签发件 T-3①。
> - **门禁数字按分树口径登记**（签发件 T-2）：本件表内"gate 486/0"指 **tag 树**（`117a947`，130 agent-core + 356 rest）；分支树（`l1-b3-rescue` `18ccd85`）为 **493/0/1 ignored**（135 + 358）。**两数并列，不得混标。**
> - 剩余动作：**外部签位**（T-6）到位后本件三签齐备，方可触发 R1 归档仪式。
