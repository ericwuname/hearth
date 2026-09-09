# Closure Audit — CORE FREEZE CLOSURE-01 Node 02-12 汇总

## Node 02 — Core Boundary Matrix（冻结边界）

| Domain | 当前权威 | 允许本轮修改 | Freeze 状态 |
|---|---|---|---|
| Intent | classify_user_input 规则 | NO | FROZEN |
| TaskGraph | DefaultPlanner + T4 规则 | NO | FROZEN |
| TaskGoal | state.goal/original_goal immutable | NO | FROZEN |
| Execution | Scheduler 五态 + Sandbox | NO | FROZEN |
| Failure | classify_failure 纯函数 | NO | FROZEN |
| Verification | verify_acceptance_criteria（O-4） | NO | FROZEN |
| Completion | 双证据门（盲区C+acceptance） | NO | FROZEN |
| Fact | state + archive | NO | FROZEN |
| Memory | maybe_compact + archive | NO | FROZEN |
| Resume | persisted state + rebuild | NO | FROZEN |
| Decision | planner 决策权 + loop 否决权（分离设计） | NO | FROZEN |
| Terminal | normalize_terminal_state 九态 | NO | FROZEN |
| Projection | 事实投影（RC20 家族） | NO | FROZEN |
| Security | RT4 + Approval | NO | FROZEN |

**零第二 authority**（CFR Authority Matrix 15 权项复核延续）；本轮生产代码改动 = 0。

## Node 03 — F0 六项四层复核

| F0 项 | 源码机制 | fixture | 真机证据 | 独立复验 | 判定 |
|---|---|---|---|---|---|
| F0-1 假完成 | 双证据门（盲区C+acceptance）+ RC47 路由仅经产物校验 | test_rc47 + w3 fixtures | n12r3/r11 零假完成 | 7 库 cargo test 全 passed | **不命中** |
| F0-2 静默丢失 | compact=archive 先行；slice=标记+state 存活 | INV-M01 loss-mode（fixture 能失败） | n06v3 session 归档实证 | archive 行数增长 | **不命中**（切片=bounded declared） |
| F0-3 重教育 | resume=persisted state 恢复 | — | chainlib/chainfree 链 | re-teach=0 | **不命中** |
| F0-4 无限循环 | T4+Reserve ≤1+replan cap 3 | T4 fixtures | 30+ 场景零死循环 | — | **不命中** |
| F0-5 沙箱绕过 | RT4 fail-closed+DenyAllNonInteractive | rc24 fixtures | n12r2 拒绝实证 | — | **不命中** |
| F0-6 投影假成功 | 事实投影（后端产事实） | w3/f9 fixtures | 五态接线后零误画 | render 锚点 | **不命中** |

**F0 = 0**。

## Node 04 — RC48 disposition（判据预写核对）

- 假设 A（Reserve 零和）✅ 成立——n12r1 `VERIFICATION_RESERVE=1 + GIVE_UP_INTERCEPTED=1`，独立复验 passed 但 failed；n12r3 反证（Reserve 在时全链路 completed 31 步）。
- 假设 B（mechanism 随机/状态污染）✗ 排除——r1/r3 唯一差异 = GiveUp 时点（同代码同参数）。
- 假设 C ✅ 贡献因子——planner 无完成事实感知（`grep acceptance planner/` = 0）。
- **Classification = mechanism(bounded 设计) + model**；**Freeze blocker = NO**；**disposition = ACCEPTED DEVIATION + RC48-FOLLOWUP RFC**（两案待顶层批准，本轮零施工）。
- 不得因 r3 一次成功降级"已解决"——稳定可复现边界（2/14 同族样本）。

## Node 05 — QA completion-awareness

1. planner 拥有 Completion Authority？**否**（O-4 唯一生产者）。
2. 应该拥有？**否**——注入 completion facts = 决策权+完成事实双持 = **authority duplication**（批-5 判据成立）。
3. GiveUp 不知道最终事实？是——这正是 loop 侧拦截存在的理由。
4. RC47 够吗？criteria 空形态够；criteria 非空 + Reserve 零和 = RC48（另一边界）。
5. QA 16 轮 T4 高频？model/decision 行为（对已完成轮再 decompose），机制有界拦截正确。
6. 注入会 duplication？**会**。
**disposition = ACCEPTED DEVIATION（model/decision limitation）**；RFC 边界=只读投影且不得用于 GiveUp 判定，或直接不做。

## Node 06 — Archive C

A = PASS（落盘实证：session 文件+compacted.jsonl 增长）；B = PASS（提示 fixture）；**C = UNKNOWN**（v2/v3 污染、v4 setup 失败——三版定性，不升 PASS/FAIL）。**disposition = ACCEPTED DEVIATION，NO BLOCKER**（M8 核心是避免第二事实模型；C 是 evidence gap 非 authority defect）。

## Node 07 — MAX_HISTORY_MSGS=40

判级依据（F0-2 不命中关键）：`[history note]` 标记存在（test_history_slice_marker passed）+ state 层存活 + **Task Continuity 持续投影关键事实** → **bounded declared loss-to-LLM ≠ silent loss**。Mechanism status = FROZEN；Risk = F1；**disposition = ACCEPTED DEVIATION**；follow-up = 切片入 archive 或标记文本修正（冻结后独立单）。

## Node 08 — Memory Invariant

`cargo test -p agent-core --lib inv_m01` = **2 passed**（v0.2.18）。A = PASS / B = PASS / **C = UNKNOWN**（禁升格）✓。

## Node 09 — Resume Final Check

reteach=0（chainlib + chainfree + P2-LR n09b 三例）与 duplicate execution=0 证据对应 v0.2.18（chainfree 链为 v0.2.18 干净二进制重跑——RC49 闭环同时即 resume 复证）。

## Node 10 — 强断言降级检查（"2+12"构成明细，批-3）

| 断言 | 归类 | 明细 |
|---|---|---|
| "零假完成" | **PROVEN**（30+ 场景+fixture 层） | — |
| "零静默丢失" | **BOUNDED**（切片=declared loss；compact=archived） | — |
| "false stop 2+12" | **BOUNDED（明细）** | **2** = n12r1/r2（RC48 形态一：Reserve 零和；mechanism+model）；**12** = n11 首跑 QA/情绪轮 T4 2-node stall（RC48 形态二：planner 无完成感知；model+decision；mechanism 拦截有界）——同族不同触发，共 14 例 |
| "archive C" | **UNPROVEN/UNKNOWN** | — |
| "textkit 收敛" | **MODEL-DEPENDENT**（未收敛 1 例） | — |
| "Agnes provider 行为" | **ENVIRONMENT/MODEL-DEPENDENT**（backoff/延迟/variance） | — |
| "reteach=0" | **PROVEN**（三例） | — |
| "零沙箱绕过" | **PROVEN**（拒绝实证） | — |
| 禁止"成功样本→普遍保证" | 执行 ✓（全部断言带边界） | — |

## Node 11 — Final Reliability Matrix

见 Final Report §11（每 PASS 附 evidence reference；无裸 "tested/verified/works"）。

## Node 12 — F1 Disposition Table（最终裁决）

| F1 | Classification | Bounded? | Core violation? | Freeze blocker? | Disposition | Follow-up |
|---|---|---|---|---|---|---|
| RC48 false stop | mechanism(bounded)+model | YES（Reserve≤1+T4+cap） | NO | **NO** | **ACCEPTED DEVIATION** | RC48-FOLLOWUP RFC（两案待批） |
| QA completion-awareness | model/decision | YES（T4 有界） | NO（authority 分离设计） | **NO** | **ACCEPTED DEVIATION** | RFC（只读投影边界）或不做 |
| Archive C | evidence gap | YES（archive 持久化） | NO（非 authority defect） | **NO** | **ACCEPTED DEVIATION** | C-probe 干净重试 |
| 40-slice recovery | mechanism（declared） | YES（state 存活+Continuity） | NO（非 silent loss） | **NO** | **ACCEPTED DEVIATION** | 入 archive 或标记修正 |

零 BLOCKER；零 OPEN 作裁决。
