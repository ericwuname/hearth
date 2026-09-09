# Hearth P1-FAILURE-ADAPTATION-01 评审包 v1（整合版）

> **用途**：单文件自足评审材料——供砺·评审窗口与外部 AI（ChatGPT）交叉评审。
> **执行窗口**：砺·执行　**日期**：2026-08-30　**代码终态**：v0.2.15（HEAD `ceb62b0`，tag v0.2.15）
> **结论**：**PASS WITH DEVIATIONS**（偏差与 OPEN 项见 §12，是否升级 PASS 由评审/顶层裁定）
> **核验纪律**：本文所有锚点均经 `grep` 现场实测；§14 给出评审方可独立执行的核验命令清单。禁止采信 commit message 与"已验证"自述。

---

## 1. Executive Summary

本总包承接 P1-EXECUTION-DECISION-01 留下的缺口：**任务失败后，Hearth 能否识别"失败是什么"并选择正确的恢复策略，而不是把所有失败当成同一种 retry。**

交付四层：

1. **能识别**：Failure Taxonomy **F1-F10 十类**纯函数（`classify_failure`），每类 ≥1 正例 + 4 组相邻边界反例；Unknown 真实存在（无结构化证据时诚实归 Unknown，不冒充）。新增结构化通道 `ToolResult.error_kind`（serde-default 向后兼容）。
2. **能改变策略**：策略矩阵代码化（`failure_strategy` 六策略：Retry/Repair/Replan/Verify/Escalate/Stop），**Retry 仅 Transient 一类**（INV-FA01-A），单测锁边界。
3. **能真正恢复**：**两个真机闭环**——Node 12 mathlib（21 步）与 Node 13 wordcount（46 步），均走"受控失败→修复→复测→Reserve 核验→GIVE_UP_OVERRIDDEN→completed"，criteria 三条全部**外部独立复验**通过。
4. **不破坏既有语义**：Final Gate v0.2.15 = 四 RC 全 0，**437 passed / 0 failed**（基线 429 + 新增 8）。

**最大增量发现**：GiveUp 实际有 **5 条产生路径**，上轮修 D 拦截只覆盖 1 条。真机红样本挖出 **T4 停滞旁路**与**预算 headless abort 旁路**，本轮全部堵上（先红后绿）；deadline 旁路按 Safety>Deadline **故意不拦截**。

**Deviation**：Node 09 calcpkg 3 跑均未收敛到 completed（层归 model 未自报 Done + 修复不完整 + Agnes 高延迟 10 次 backoff）；机制层行为全部正确，系统未假报完成。

---

## 2. Failure Lifecycle（实际控制流，v0.2.15 锚点）

```text
[T1 产生]  tool-runtime dispatcher
           ├─ bash: exit code / 信号（正 exit=断言族；负=信号终止）
           ├─ TaskDeadlineExceeded（dispatcher 单点 effective=min(declared,remaining)）
           │    → 新增：downcast 投影为 ToolResult.error_kind=DeadlineExceeded
           └─ sandbox RT4 fail-closed / seccomp SIGSYS
[T2 观察]  do_reflect：has_error = pending_results.any(is_error)   （loop.rs）
[T3 计数]  consecutive_errors / steps_without_progress / graph_stall_count /
           verify_replan_count / acceptance_replan_count（详见 §6 污染表）
[T4 决策]  planner.reflect 快路径臂 + LLM verdict + nervous Abandon + T4 停滞 Err
[T5 消费]  Continue→Plan / Replan（cap 3）→Plan / GiveUp→拦截链→Error
[T6 验证]  Done 相位：written_files 校验（verify_replan_count<3）+ acceptance 核验
           （Reserve acceptance_replan_count<1，run-level 零和）
[T7 终态]  Event::Done / RunReport / Error 相位（ok=false）
```

### 2.1 GiveUp 五路径全景（本轮核心盘点）

| # | 路径 | 位置（v0.2.15） | FA01 处置 | 证据 |
|---|---|---|---|---|
| 1 | planner GiveUp 臂（Reflect 消费） | loop.rs GiveUp 臂 | 修 D 拦截（上轮已落地） | 上轮 46 步真机 |
| 2 | nervous system Abandon | do_reflect | 保持观察（无真机样本） | — |
| 3 | **T4 语义停滞 Err** | do_plan stall | **本轮加拦截**：Reserve 核验→passed 路由 Done / failed 落 Err | 真机红样本 r1 + 正反 fixture |
| 4 | **WS9 预算 headless abort** | run loop budget 分支 | **本轮加拦截**：核验 passed→路由 Done（**不延长执行预算**） | 真机红样本 r2 |
| 5 | **deadline abort** | run loop deadline 分支 | **故意不拦截**（Safety>Deadline>Recovery，INV-FA01-D） | Node 09 r3 604s 实证 |

**统一不变式**：凡"未核验的放弃"——要么先核验（Reserve ≤1 零和），要么显式打 `giveup_unverified` 结构化标记（scratch + RunReport.summary），失败报告 `error_detail` 不再折叠为 "loop error"。criteria 空的 Product 任务放弃同样打标记（F9，砺批-1）。

### 2.2 criteria 为空路径（砺批-1 承接）

- GiveUp 拦截与 Done 核验同构守门（`parse_acceptance_criteria` 空即跳过）——criteria 空 = 系统拒绝核验 ≠ 核验失败。
- 全路径推演与定性见 `failure-adaptation-flow.md` §4：**终态不是假 completed**（give_up 走 Error，ok=false），真正缺陷曾是"放弃时零核验证据且不声明未核验"——本轮以 F9 标记补齐。

---

## 3. Failure Taxonomy（F1-F10）

`crates/agent-core/src/terminal.rs::classify_failure`——纯函数，8 个结构化输入（is_timeout / is_network / same_tool_repeat / exit_code / was_verification_step / llm_self_reported_ok / approval_denied / budget_exhausted），**禁错误文本解析**（RC20 纪律），禁 LLM judge（STOP-9）。

| 类 | 名 | 判据 | 策略 |
|---|---|---|---|
| F1 | transient_provider | is_network | **Retry**（唯一允许盲重试类） |
| F2 | tool_execution | exit_code<0（信号终止） | Repair |
| F3 | environment | is_timeout（deadline 权威） | Escalate |
| F4 | permission/approval | approval_denied（**优先于 exit code**） | Escalate |
| F5 | assertion_or_test | exit_code>0 | Repair |
| F6 | verification | 验证步失败 | **Replan**（≠retry，INV-FA01-F） |
| F7 | plan/strategy | same_tool_repeat≥2 | Replan |
| F8 | resource/budget | budget_exhausted（**优先于 F7**） | **Stop**（INV-FA01-E） |
| F9 | model_judgment | 验证步失败+自述成功（冲突优先） | Verify |
| F10 | unknown | exit_code=None 且无其他信号 | Escalate |

测试：`test_failure_taxonomy_f1_to_f10`——十类正例 + 4 组边界反例（F1vsF3 / F4vsF5 / F8vsF7 / F9vsF6）+ same_tool_repeat=1 反例。
**OPEN**：bash exit code 尚未接入 ToolResult（loop 层无该证据时归 Unknown，不冒充 F2/F5）。

---

## 4. Strategy Matrix

`failure_strategy(FailureKind) -> RecoveryStrategy`（terminal.rs 纯函数）+ `docs/failure-strategy-matrix.md`（含每类与 loop 既有通道的接线表）。边界锁定测试 `test_failure_strategy_matrix_boundaries`：
Retry 仅 F1；F2/F5→Repair；F6/F7→Replan；F9→Verify；F4/F3/F10→Escalate；F8→Stop；Replan≠GiveUp/Stop。

---

## 5. Progress Semantics（Node 04，维持不改）

- 现口径（loop.rs，写类工具=唯一 fact_progress）**维持不变**。依据：修 D 拦截已在 give_up 点做等价补偿且真机实证；扩展 = 控制流变更 + budget 燃烧节奏漂移，按修 A"备而未用"先例须顶层批准。
- 二阶效应清单（扩展前必测）：①拦截后回喂步计入 progress→counter 清零→budget-low GiveUp 臂命中点漂移→真死任务预算浪费放大（须配 reserve cap）；②与 acceptance_replan_count 零和竞争加剧；③与 graph_stall_count 无交互（已排除）。
- 保障：Case D fixture 随 gate 全绿；Node 13 真机 GIVE_UP_OVERRIDDEN 未回归。

---

## 6. Recovery Model 与计数器污染表（Node 01/05）

| 计数器 | 增 | 清零 | 消费 | 污染风险 |
|---|---|---|---|---|
| consecutive_errors | reflect 有错步 | 无错步；**replan 故意不清** | planner GiveUp 臂 | 无跨计数器污染 |
| steps_without_progress | 有错步；无错无写步 | fact_progress/any_completed/replan | GiveUp 臂（≥2+low）、Replan 臂（≥5） | **验证步不计 progress**（§5 维持） |
| verify_replan_count | Act done-gate（<1）；Done 相位（<3） | fresh run | 两处 | **同一计数器双上限**：Act 消耗后 Done 只剩 2 次——跨相位零和（本轮新登记） |
| acceptance_replan_count | GiveUp 拦截；Done Reserve | **无 run 内清零** | 两处共用 | 砺批-2 已裁决 run-level 零和，本轮 T4/预算拦截同样消耗它（第 3/4 路径），有界性成立 |
| graph_stall_count | do_plan 同图 | 图变化 / v20 重入 | ≥2→Err | 干立；单节点图豁免 |

Reserve 粒度裁决：**run-level 共用，不翻案**（复-3 裁决 2）。若未来拆 phase-level：影响分析见 `failure-recovery-model.md` §2（对 verify_replan_count/graph_stall_count 无连带）。

---

## 7. Changes（逐文件，v0.2.14 → v0.2.15）

| 文件 | 变更 |
|---|---|
| `terminal.rs` | FailureKind 7→10 类（+Permission/Resource/Unknown，参数 +approval_denied +budget_exhausted，`#[allow(too_many_arguments)]` 有意裁决）；RecoveryStrategy 枚举 + failure_strategy 纯函数 |
| `loop.rs` | 观察级分类消费端（same_tool_repeat/last_error_tool/approval_denied_flag/fa01_budget_intercepted 字段 + scratch `last_failure_class`/`last_recovery_strategy`）；F9 giveup_unverified 标记 ×3 路径；Error 相位 summary 投影 error_detail；**T4 拦截**；**预算拦截**；F1-F9 fixtures（StallPlanner/LoopReadLlm 专用 mock） |
| `scheduler.rs` | dispatch Err downcast TaskDeadlineExceeded → error_kind（单/并行两路） |
| `agent-types/lib.rs` | ToolErrorKind + ToolResult.error_kind（serde default + skip_serializing_if，不进 LLM 可见内容，事件契约零改动） |
| `tool-runtime/lib.rs` | TaskDeadlineExceeded 重导出 |
| `Cargo.toml` | 0.2.14→0.2.15 |
| docs | failure-adaptation-flow.md（生命周期+旁路全景）/ failure-strategy-matrix.md / failure-recovery-model.md / data/failure-adaptation-20260830/（11 个证据文件） |

提交链：`eb77bbc`（Node 00-02）→ `81d39d4`（Node 03-08）→ `236f7a8`（Node 09-15+bump）→ `c6d9fac`（Final Report）→ `ceb62b0`（v1.1 附录）。

---

## 8. Tests（先红后绿）

| 红 | 绿 |
|---|---|
| 真机 Node 09 r1（43 步：事实全完成但 T4 旁路放弃零核验） | T4 拦截落码 + `test_fa01_t4_stall_intercept_verifies_before_giveup` 绿 |
| 真机 Node 09 r2（50 步：预算 headless 放弃同根） | 预算拦截落码 + 全量回归绿 |
| F9（criteria 空 bash-only 放弃零证据） | `test_fa01_f9_empty_criteria_bash_only_giveup_unverified` 绿 + 反例（completed 无标记）绿 |
| —（T4 反例前置） | `test_fa01_t4_stall_verification_failed_still_stops`：核验 failed 仍 bounded stop，Reserve 恰 1 次 |

新增 8 测试；Final Gate `.133:~/run_gate_r2c.sh → /home/wutao/t_gate_fa_final.log`（v0.2.15）：**FMT_CHECK_RC=0 / CLIPPY_RC=0 / RT4_SOLO_RC=0 / TEST_RC=0；437 passed / 0 failed / 0 ignored**（基线 429；63 条 test-result 行；RT4 单测在 rt4_solo.log 属预期）。

---

## 9. Real-machine 证据（.131，binary 0.2.15，budget=50 / 600s / Agnes）

### Node 09 calcpkg（TDD 受控失败，3 跑）

| 跑 | 终态 | 层归 |
|---|---|---|
| r1 43 步 | failed（T4 旁路） | mechanism 缺口→已修 |
| r2 50 步 | failed（预算旁路） | mechanism 缺口→已修 |
| r3 604s | failed（deadline_exceeded 权威） | model（E0425 未收敛）+env（10 次 backoff）；独立复验 lib.rs 编译失败→criteria 必败→**未假 completed** |

受控失败本体实证：`exit code: 101`（真 cargo test 失败）→ 修复动作发生 → 复测未竟。

### Node 12 mathlib（双受控失败 + 上轮 shout OPEN，3 跑）

| 跑 | 终态 | 关键证据 |
|---|---|---|
| r1 49 步 | failed（诚实） | 拦截触发→核验发现 cmd failed+无 to_uppercase→拒绝假 completed（模型写了 RESULT.txt 但 bug 未修——上轮假阳性形态被正确拦下） |
| r2 50 步 | failed（budget_exhausted） | 拦截捕获**完整 E0425 编译诊断**（测试模块缺 `use super::*`）+read_lints 2 条——acceptance 层=真实诊断通道；Reserve 尽→预算拦截正确拒绝二次核验 |
| **r3** | **completed（21 步，tokens 24965）** | 拦截→Reserve passed→GIVE_UP_OVERRIDDEN。独立复验：add/shout 双 bug 真修、`use super::*` 在位、**cargo test 2 passed**、RESULT.txt 正确 |

**上轮遗留 shout OPEN 正式行为级闭环**（criteria `file: ... contains to_uppercase`）。

### Node 13 wordcount（非 mathlib，1 跑）✅

46 步 completed（≤52 判据，overhead vs 42 基线 = +4）：受控失败→修复→复测→Reserve→GIVE_UP_OVERRIDDEN→completed。独立复验：cargo test **1 passed**、main.rs 含 top_words、RESULT.txt=WORDCOUNT_DONE、`printf "b a b c b" | cargo run` 输出 `b 3 / a 1 / c 1`（**语义正确**）。

### Node 14 QA 负回归 — PASS

"什么是所有权？"2 步直答、"查看状态"6 步直答，零 replan/give_up/recovery 进入；单测侧修 B（INV-ED01-A）全绿。

---

## 10. Regression（Node 15）

- Gate 437/0 覆盖全部历史主线单测（Intent routing / TaskGraph continuity / Readiness / O-4 acceptance / 修 D Case D-G / Deadline / Approval / Sandbox / Resume / Terminal / Projection）。
- **Case D 真机样本未回归**：Node 13 GIVE_UP_OVERRIDDEN（非 mathlib）+ Node 12 r3（mathlib）双样本。
- 两套 INV 编号防混淆：`INV-ED01-*`（上轮，单测全绿未回改）与 `INV-FA01-*`（本轮，A=Failure≠Retry / B=Progress≠Mutation（维持）/ C=反重复四层上界 / D=deadline 豁免拦截 / E=Reserve 零和不翻案 / F=verification≠retry / G=self-report≠evidence / H=QA 不入 recovery）。

---

## 11. 真机完整性链路（§28 纪律对照）

failure→classification（exit 101 结构化捕获）→strategy（纯函数+scratch）→recovery（Node 12 r3/13 修复）→retest（cargo test）→verification（三 criteria 独立过）→terminal（completed/诚实 failed）——**两个样本全链完整**；anti-loop（Node 09 r1 T4 bounded stop + caps 单测）成立。

---

## 12. OPEN / UNKNOWN / DEFER

| # | 项 | 等级 | 建议 |
|---|---|---|---|
| 1 | bash exit code 未接入 ToolResult→loop 层 F2/F5 判定受限（纯函数层就绪） | open | 下单接入（dispatcher bash 通道已有 exit code，缺投影） |
| 2 | Node 09 单样本 model 未收敛（未自报 Done+修复不完整；Agnes 高延迟） | open | 错峰复测或换模型对照 |
| 3 | progress 语义扩展（四类 progress 候选） | defer | 须顶层批准；二阶效应清单在 §5 |
| 4 | verify_replan_count 双上限（Act<1/Done<3）统一 | defer | 控制流变更，预案在 recovery-model §2 |
| 5 | 修 A 常量标定（BUDGET_LOW_THRESHOLD 等） | defer | 备而未用（复-3 裁决 2） |
| 6 | 大 tool output × 低预算组合未单独压测 | open | 低风险，可并入下轮 |

**判定口径**：§25.A-E 已全部真机实证（C="至少一个闭环"现为两个）；维持 PASS WITH DEVIATIONS 的唯一实质偏差 = Node 09 单样本 model 层 + 上述 OPEN 工程项。**升级 PASS 由砺·评审/顶层裁定**，执行窗口不自嗨升级。

---

## 13. Provenance

| VM | source | version | binary | gate/日志 |
|---|---|---|---|---|
| .133（评审） | `/home/wutao/codex_t`（tar 同步树，无 .git） | 0.2.15 | `/usr/local/bin/hearth`=0.2.15（本轮重建） | `/home/wutao/t_gate_fa_final.log`（437/0 四 RC=0）；基线 `t_gate_fa_baseline.log`（429/0） |
| .131（执行） | `/home/wutao/codex`（tar 同步树，无 .git） | 0.2.15 | `/usr/local/bin/hearth`=0.2.15（本轮重建） | `~/fa/node09_r3.log` / `node12.log` / `node12_r2.log` / `node12_r3.log` / `node13.log` / `node14.log`（已归档 `docs/data/failure-adaptation-20260830/`） |
| 本机 | HEAD `ceb62b0`，tag v0.2.15 | 0.2.15 | — | — |

criteria 冻结全文（批-5）：`docs/data/failure-adaptation-20260830/node09-12-13-criteria-frozen.md`（run 前冻结未修改）。

---

## 14. 评审核验锚点与命令清单（评审窗/外部 AI 可直接执行）

在 `.133:~/codex_t`（或本机仓库）验证（预期全部命中）：

```bash
# 1) 十类 taxonomy + 六策略纯函数
grep -n "pub enum FailureKind" -A 12 crates/agent-core/src/terminal.rs
grep -n "pub fn failure_strategy" crates/agent-core/src/terminal.rs
# 2) 三条拦截路径 + F9 标记
grep -n "GIVE_UP_OVERRIDDEN(t4 stall)" crates/agent-core/src/loop.rs
grep -n "GIVE_UP_OVERRIDDEN(budget exhausted)" crates/agent-core/src/loop.rs
grep -n "giveup_unverified" crates/agent-core/src/loop.rs        # ≥3 处
grep -n "error_detail" crates/agent-core/src/loop.rs             # Error 相位投影
# 3) 结构化通道
grep -n "error_kind" crates/agent-types/src/lib.rs crates/agent-core/src/scheduler.rs
grep -n "TaskDeadlineExceeded" crates/tool-runtime/src/lib.rs
# 4) deadline 旁路豁免（预算分支有拦截，deadline 分支无）
grep -n "deadline_exceeded" crates/agent-core/src/loop.rs | head  # 该分支无 VERIFICATION_RESERVE
# 5) 测试（.133，约 65s）
HEARTH_ALLOW_NO_CGROUP=1 cargo test -p agent-core --lib fa01     # 6 passed
bash ~/run_gate_r2c.sh                                            # 四 RC=0，437/0
# 6) 真机日志关键行（.131）
grep -aE "GIVE_UP_OVERRIDDEN|VERIFICATION_RESERVE" ~/fa/node12_r3.log ~/fa/node13.log
grep -aE "exit code: 101" ~/fa/node09_r3.log
```

**防伪声明**：本包所有"completed"均有外部独立复验（评审窗可重跑 §14-6 的 cargo test）；所有"failed"均如实披露并层归；无一次成功外推为"全部可靠"的表述。
