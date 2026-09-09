# Hearth P1-TASK-TRUTH-01 施工单 v1：Task Fact → Verification → Reflect → Completion 事实与权力边界收口

> **性质**：完整长程施工单——**批准后施工，本文档不含代码变更**（顶层明令：施工单形成后停在顶层批准）。
> **签发依据**：顶层《收到〈P1-CONSOLIDATION-01 Final Report v1〉》（O-4 立项 + P1-TASK-TRUTH-01 命名 + 建议 Node 00-10）+ 守门员前置约束 5 条（O-4 证据分层 / Node 05 吃失败样本 / Node 07 口径预写 / 延续纪律 / Reflect 边界）。
> **上游**：P1-CONSOLIDATION-01 = PASS WITH DEVIATIONS（偏差 = Node 07 长程完成度，29/42 步两样本）。
> **设计单三问预答**（守门员裁决 3 模式）：见 Node 02/03/05 各节。

---

## 0. 使命

把完成判定从：

```text
artifact_exists == true  →  completed
```

升级为证据分层：

```text
Tool Execution Evidence（工具层结构化输出：exit code / test runner 解析）
+ Artifact Evidence（产物存在且非空）
+ Verification Evidence（verify_written_files / acceptance 核验）
+ Acceptance Criteria（用户声明的验收标准——结构化核验）
→ Completion
```

**边界**：不升格为"LLM 语义理解验证"（那是 DEV-2）；不建第二套 criteria 系统（对齐 R2-B 既有 `acceptance_criteria` 承载结构）；Reflect 冲突仍 observe-only（任何控制流变更 = STOP 交顶层）。

## 1. 全局红线（延续 + 本单强化）

延续总包红线全表（Terminal 九态/ApprovalPolicy/HardRedline/sandbox/seccomp/Landlock fail-closed/事件契约/TaskGoal/TaskGraph 定义/单一事实源/单一 Task Continuity）。

**本单强化**：
- acceptance_criteria 的**字符串内容格式约定**（Node 03）属于兼容扩展，不改 schema；
- 完成判定升级只作用于 **criteria 非空**的 run；criteria 空时行为与 v0.2.12 逐字节一致（Node 09 回归锚）；
- Node 05 若需要调整 verify-replan 与 budget 的交互，只允许**收紧性/保障性**方向（保底预算），不得放宽 budget 数值语义（预算数值语义仍在红线内——方案以"标记位豁免"实现而非改 cap）。

## 2. Node 00 — Baseline / Provenance（分窗登记口径首次执行）

- git/HEAD/tag/双 VM binary 核对（系统 PATH 口径：`.133`/`.131` `/usr/local/bin/hearth` = 0.2.12）。
- **分窗登记核实**（vm-version-sync.md 2026-08-30 修订版）：`.131` 施工树 = `~/codex`（自有）；`.133` 验证树 = `~/codex_t`；`.131 ~/codex_t`(0.2.9) DEPRECATED 状态确认（不删）。
- 基线 gate：`~/run_gate_r2c.sh`（.133）四 RC 全 0 落盘 → `.133:/home/wutao/t_gate_tasktruth_node00.log`。
- `df -h /home` 基线（<10G 中止保险丝，全包每 Node 末复查）。
- 上一轮证据迁移核查：`docs/data/n1sbx-o1-20260830/`、`p1ltr-20260830/`、`n1-landlock/` 在库。

## 3. Node 01 — TaskGraph Fact Lifecycle（源码核实，无代码）

审计 TaskGraph 节点状态的事实生命周期，输出事实表：

```text
谁产生节点（planner.decompose / orchestrator）
谁置位 Completed（mark_nodes_completed_on_accept——verify 通过后统一置位）
谁置位 Failed/Skipped
Pending 残留的路径（replan 后旧图 vs 新图；QA 跳过 decompose 后图恒空）
节点状态与 written_files / completion_fact_check 的关系
```

回答（设计输入）：**"修复实际成功但复测时序混乱"在节点状态上如何体现**（Node 07 42 步样本的 TaskGraph 视角解剖）。产出 = 事实表 + 疑点清单（不修代码）。

## 4. Node 02 — Evidence vs Verification 分层设计（设计确认，小代码）

四层证据模型定义 + 与现有机制映射：

| 层 | 证据 | 现有承载 | 缺口 |
|---|---|---|---|
| L1 Tool Execution | exit code / 结构化输出 | ToolResult.is_error + dispatch Result | test runner 输出**未结构化解析**（FAILED/ok 混在文本） |
| L2 Artifact | 存在且非空 | verify_written_files | 无语义检查（by design） |
| L3 Verification | replan 闭环 | verify_replan_count ≤3 | 与 budget 无保障交互（Node 05） |
| L4 Acceptance | 用户验收标准 | acceptance_criteria（R2-B）+ acceptance_verification 三态 | **criteria 从未被机器核验**——三态中 "passed" 无生产者（诚实缺口） |

**关键设计确认**：L4 缺口 = O-4 根因（RESULT.txt 自述 TEST_PASSED 被当完成证据，而 acceptance_verification 恒 "none"）。Node 03 补生产者。

## 5. Node 03 — O-4 Semantic Completion（核心实施）

### 5.1 criteria 格式约定（兼容扩展，不改 schema）

`acceptance_criteria: Vec<String>` 内的**结构化前缀**约定：

```text
"cmd: <command>"        → 命令型：真实执行，exit code 0 = 通过（L1 结构化证据）
"file: <path> contains <text>" → 内容型：文件存在且含 text（L2+确定性检查）
"file: <path> nonempty"        → 存在型（= 现 verify 语义）
（无前缀）               → 自由文本 → 仅注入 Task Continuity（现状），不参与机器核验
```

### 5.2 完成判定升级

Done 相位（verify_written_files 通过后）：

```text
criteria 中存在结构化条目（cmd:/file:）
→ 逐条机器核验（cmd 走既有 bash 通道——审批语义不变；file 走确定性读取）
→ 全过 → acceptance_verification = "passed"（生产者补齐）
→ 任一失败 → acceptance_verification = "failed" + 失败明细
   → 沿用 verify_replan 通道回喂（≤3 次同额度）→ 超限 → verify_failed 终态（复用 Node 04 既有语义）
criteria 全为自由文本/空 → acceptance_verification = "none"（v0.2.12 行为逐字节一致）
```

### 5.3 边界

- **不引入 LLM 语义理解**（核验全部确定性——cmd exit code / 文件内容比对）；
- **artifact 自述（RESULT.txt 内容声明）不作通过证据**——由用户在 criteria 中显式声明 `file: RESULT.txt contains LONGRUN_TEST_PASSED` 才核验（O-4 的"证据分层"精神：模型自述 ≠ 证据）；
- `cmd:` 执行走既有 bash 通道 → **审批语义/沙箱/landlock 全部继承**（不绕过任何边界）。

### 5.4 测试

- 单测：criteria 解析（四类前缀）+ 核验调度 + passed/failed/none 三态生产者（先红后绿——"passed" 无生产者现状红）；
- fixture：Node 04 的 verify_failed fixture 扩展——criteria 带 `cmd:` 条目 → 核验失败 → verify_failed（带 acceptance 失败明细）；
- 回归：criteria 空 → 417 全量原样。

## 6. Node 04 — Reflect-Fact Conflict 样本扩展（observe-only 延续）

- REFLECT_FACT_CONFLICT 标记已在 Node 03（上总包）落地——本 Node 收集 **≥3 个冲突样本**（真机 give_up + artifacts 非空场景），逐样本记录 10.1 清单字段（original_goal/task_graph/successful_tools/artifacts/reflect_output/terminal）。
- 回答 Q2（何时冲突）：按样本分类（预算耗尽型 / 模型误解释型 / 分类错误型——Node 02 修复后的残留分布）。
- 边界：observe-only；任何 Fact↔Reflect 控制流变更 = STOP。

## 7. Node 05 — Budget × Replan × Verification 交互矩阵（核心设计）

### 7.1 输入：Node 07 两失败样本解剖（29/42 步）

事实：受控失败 → 修复成功 → 复测时序混乱 → planner budget-low(7%) give_up。核心问题：**修复后的验证重试有没有预算保障？**

### 7.2 交互矩阵（预写）

| 场景 | 现状行为 | 风险 | 方案候选（Node 05 裁决） |
|---|---|---|---|
| budget 低 + verify replan 待发 | planner budget-low give_up 可抢先 | 修复成功但无预算复测（42 步样本） | A: verify/acceptance replan 记账独立于 progress 计数；B: budget-low give_up 判定豁免"acceptance 核验未完成"场景 |
| budget 低 + 受控失败 replan | 同上 | 受控失败恢复被掐断 | 同 A/B 评估 |
| deadline 低 + 工具长跑 | P1-LTR 已修（工具级截止） | — | 已闭合 |
| acceptance 核验自身耗时 | 计入步数/budget | 核验挤占执行预算 | 核验步不计 progress 计数（观察项） |

方案实施原则：**标记位豁免**（不改 budget cap 数值——红线）；A/B 候选在 Node 05 内做源码级影响面评估后择一实施（保障性方向）。

### 7.3 测试

矩阵逐格单测 + 42 步样本场景的 Mock 复现（受控失败→修复→budget-low→验证完成——修复后复测**有预算保障**）。

## 8. Node 06 — Failure Adaptation（受控失败语义）

- 受控失败（TDD/复现测试）是合法工作流：确认 verify_replan 与 acceptance 核验路径对"失败→修复→复测"的**不误伤**（受控失败不计 consecutive_errors？现状取证——若计，评估豁免标记）。
- 产出：受控失败场景的端到端 fixture（TDD 小任务：红→修复→绿）。

## 9. Node 07 — Long-run 复跑（**口径预写**，先定后跑）

- **同任务同 prompt**（mathlib 受控失败 8 步骤，逐字复用 CONSOLIDATION Node 07）。
- 对照基线：29 步 failed（budget 30）/ 42 步 failed（budget 50）。
- **improvement 判定（预写，禁事后挑口径）**：
  1. `terminal = completed`；
  2. `cargo test` 真实 ok（L1 结构化证据——日志含 `test result: ok`，非模型自述）；
  3. RESULT.txt 内容与核验一致（L4 acceptance `file:` 条目 passed）；
  4. 步数 ≤ 42；
  5. 全程零 REFLECT_FACT_CONFLICT / 零假声明。
- 判定"improvement 达成"= 1-5 全满足；仅部分满足 = DEV-2 深化证据（不归 provider——除非 trace 排除全部机制因素，顶层 §二要求）。
- budget 50（与最优基线同口径）+ telemetry 开 + deadline 600s。

## 10. Node 08 — Discussion 回归

10+ turn REPL（Rust 主题十问，逐字复用）——零 TaskGraph 循环（W8/QA-skip 回归）。

## 11. Node 09 — 全量回归 + Final Gate

- 跨层回归：Context/Completion/Routing/Approval(TC-8/9/RC29/redline)/Deadline(P1-LTR 四态)/Resume/Sandbox(/dev/null·file·dir·mkdir·seccomp deny)。
- `~/run_gate_r2c.sh`（.133）四 RC 全 0 → 路径登记。
- 版本 bump 0.2.12 → **0.2.13** + CHANGELOG **前置于 final gate**（修 5：gate 覆盖最终提交态）。

## 12. Node 10 — Provenance（分窗登记口径）

- `.133`：系统 PATH hearth 0.2.13 + source ~/codex_t @ HEAD。
- `.131`：系统 PATH hearth 0.2.13 + source **~/codex** @ 同源（分窗登记口径）。
- 双查：binary --version + Cargo version + source marker。

## 13. Node 11 — Ledger / CHANGELOG / Final Report

- 总账回填：O-4 状态、Node 05 矩阵结论、Node 07 复跑对照表。
- Final Report（§22 十节格式）：PASS / PASS WITH DEVIATIONS / STOP 三选一硬结论。

## 14. STOP CONDITIONS（本单）

延续总包 §4.2 七条 + 本单两条：
- STOP-8：criteria 结构化格式约定被发现与 TaskGoal/事件 schema 冲突（无法以兼容扩展落地）；
- STOP-9：acceptance 核验需要 LLM 语义理解才能满足（越界到 DEV-2，无法以确定性手段实现）。

## 15. 验收硬项清单

```text
① criteria 四类解析单测全绿
② acceptance "passed" 生产者落地（三态完整）
③ Node 04 fixture 扩展（acceptance 失败 → verify_failed 带明细）
④ Node 05 交互矩阵 + 42 步样本 Mock 复现（验证预算保障）
⑤ Node 07 复跑：预写口径判定（completed + 结构化 test ok + 无假声明）
⑥ Discussion 10+ turn 回归
⑦ criteria 空 → v0.2.12 逐字节回归
⑧ 全量 gate 四 RC 全 0（路径登记）
⑨ 双 VM provenance 0.2.13（分窗口径）
⑩ Ledger/CHANGELOG 回填（保留 CLOSED/OPEN/UNKNOWN/DEFER 分层）
```

## 16. 执行模式

长程连续自主执行（Node 间不等确认，STOP-1~9 才停）；**本单批准后开工，Final Report 一次提交**。
