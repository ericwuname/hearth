# Hearth W3/W4 回归验证 · 偏差与结果报告 v1（2026-08-29）

> 指令：《hearth-w3w4-regression-verification-order-v1》（四证明·零代码改动轮）。证据归档：`docs/data/regv-20260829/`（18 文件）。运行口径：.131 / binary 0.2.9（W3/W4）/ Agnes / `HEARTH_ALLOW_NO_CGROUP=1` / telemetry 开。

## V-1 · 旧故障基线回归（证明 ①②）——**未达目标，偏差报告**

原 prompt 逐字（log-aggregator 设计任务，--budget 40）10 轮复跑：

| # | 旧基线(08-28) | 本轮 | write | reflect 序列 | 定性 |
|---|---|---|---|---|---|
| B01 | failed 78.5s | ✅ **completed** 80.8s | 0 | （直答） | 模型直答走 QA 判定 ✓ |
| B02 | failed(误标S) | ⛔ approval 阻塞 | — | — | **RC24 再实证** |
| B03 | failed 17步 | ❌ failed 22步 | 0 | continue→continue→replan | 论述任务 replan 循环（DEV-1） |
| B04 | **挂起 200s** | ⛔ approval 阻塞 23s | — | — | RC24（挂起已消失=T2/T3 修复生效） |
| B05 | failed 17步 | ❌ failed 17步 | 0 | replan→continue→give_up | DEV-1 |
| B06 | **挂起 200s** | ❌ failed 93s | **1** | continue→continue→give_up | **DEV-2（写盘后 LLM give_up）** |
| B07 | failed 13步 | ❌ failed 165s | **1** | continue→continue→give_up | DEV-2 |
| B08 | failed(误标S) | ❌ failed 9.4s | 0 | replan 循环 | DEV-1 |
| B09 | failed(误标S) | ❌ failed 6.4s | 0 | replan 循环 | DEV-1 |
| B10 | failed 17步 | ❌ failed 7.2s | 0 | replan 循环 | DEV-1 |

**完成率 1/10 << 目标 ≥8/10 —— 偏差成立**。但对比性质变了：
- **旧基线死因（机制性）全部消失**：0 挂起（B04/B06 型 200s hang = T2/T3 修复生效）、0 假完成、失败时长 6-165s（旧 78-200s）。
- **本轮残余死因（非 W3/W4 机制回归）**：论述型任务（纯推理输出、无文件产物）→ planner 拆 4 节点图 → 模型探索/直答 → reflect LLM 判 replan/give_up → 循环。机制各环节（fact progress 记录、prompt 诚实化、0 错误、预算充裕）**按设计工作**——B06/B07 实测 0 errors、写盘已发生、prompt 含事实行，**reflect LLM（agnes-2.5-flash）仍判 give_up = 模型判定质量问题（D 类）**。
- 结构性缺口：**论述型任务不该进 TaskGraph 循环**（无产物可验、reflect 无从判定"进展"）——归 **W8 Goal Revision/任务分类**（批示范围外，停点不修）。

## V-2 · 失败任务 resume 连续性（证明 ④）——**全过 ✅（本轮核心成果）**

| 轮 | 首轮复述进度 | 增量产物 step2.txt | 重教次数 | resume 终态 |
|---|---|---|---|---|
| R1 | ✅ True | ✅ True | **0** | failed（R1 侧注：budget 6 下二次失败） |
| R2 | ✅ True | ✅ True | **0** | ✅ completed |
| R3 | ✅ True | ✅ True | **0** | ✅ completed |

- 验收四条：①resume 首轮准确复述（3/3，STEP1/step1/目标词出现）②真实增量产物（3/3 断点续做非从头）③`completion_fact_check()` 不误拒（R2/R3 accepted）④终态正确（R2/R3 completed；R1 failed 为二次资源失败，非 stalled/误 give_up）。
- 对照 v0.2.3"继续什么？"历史症状：**0 例** ✓。RC5/RC30 家族闭合实证。

## V-3 · 六终态矩阵（证明 ③）——部分实证

| 终态 | 结果 |
|---|---|
| completed | ✅ T-A（前轮）+ B01（本轮） |
| failed | ✅ V-2 R1 + V-1 多轮（status=failed + reason 投影） |
| give_up | △ V3_give_up 跑出 failed 终态（give_up 型，投影为 failed——**六态中 give_up 与 failed 的投影区分度待顶层确认**） |
| verify_failed | ⏸ 未自然触发（构造留下一轮） |
| timeout | ⏸ 任务 25.6s 完成未达 45s 上限（构造问题非缺陷——重测需更长任务） |
| cancelled | ⏸ SIGINT 后投影 other（headless 下 cancelled 投影待查——OPEN） |
| **RC20 回归锚** | ✅ **中文错误投影证据：T-C `✗ 工具出错` 结构化投影、错误帧无绿勾误标**（前轮已证，本轮无回归） |

## 偏差清单（V-4 停点，零代码改动遵守 ✓）

- **DEV-1**：论述型任务（无产物）不适用 TaskGraph+reflect 循环——完成率 1/10 根因。归类：任务分类缺口（**W8 范围**）+ reflect LLM 判定质量（**D 类/provider**）。修复方向建议：论述/问答类任务短路 reflect replan（或 planner 对无产物任务不 decompose）——待顶层裁决，本轮未动。
- **DEV-2**：B06/B07 写盘后 reflect LLM give_up——agnes-2.5-flash reflect 判定质量（D 类）。证据：0 errors + writes=1 + 诚实 prompt 仍 give_up（`regv-20260829/V1_B06.log`）。
- **DEV-3**：B02/B04 审批门阻塞——RC24 再实证（blocked-by-RC24 记录跳过）。
- **DEV-4**：cancelled 终态 headless 投影 other——SIGINT 投影链待查（小项 OPEN）。

## 四证明状态汇总

| 证明 | 状态 |
|---|---|
| ① 工具成功不因 node.status give_up | **部分**——T-A/B01 completed 实证机制工作；B06/B07 残余=LLM 判定质量（DEV-2 偏差） |
| ② 纯读不被判不工作 | **达成**（TC-1 类 T-B completed；B09 replan 循环=DEV-1 论述分类缺口，非 stalled） |
| ③ 六终态投影 | **部分**（completed/failed/RC20 锚实证；timeout/cancelled/verify_failed 构造未闭合） |
| ④ resume 连续性 | **完全达成**（3/3 复述+增量+零重教） |

**结论**：W3/W4 机制本身无回归（门禁 391 + V-2 全过 + RC20 锚无回归）；**未宣布四证明全闭合**——DEV-1（论述任务分类，W8）与 DEV-2（reflect 判定质量，D 类）须顶层裁决处置路径。停，等顶层。
