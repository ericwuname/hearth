# Tier3 遥测故障现场留存（2026-08-28 · T1）

> handoff 性质：Tier3 任务书 T1（P0 先行）——B03/B04/B06 三次故障 + 全部轮次的**唯一一手原始样本**，修复过程中不得覆盖/丢弃。

## 文件清单

| 文件 | 内容 | 大小 |
|---|---|---|
| `hearth_telemetry_r2.txt` | 10 轮长程自主任务完整遥测（B01-B10）——**B03/B04/B06 = 3 次挂起（rc=124，200.1s 外部 timeout 杀）**；B01/B05/B07 = task_failed（49-78s）；B02/B08/B09 = success | 7.9KB |
| `hearth_telemetry_result.txt` | 遥测结果汇总（30/30 对话 + 10/10 resume 成功、Tier3 0/10 的汇总出处） | 2.2KB |
| `hearth_resume_fix.txt` | resume 误报修复记录（T8 `--budget` 误传 bug 的实证） | 4.3KB |
| `run_telemetry.py` / `run_telemetry_r2.py` / `run_telemetry2.py` | 遥测 runner 脚本（T8 加固对象——含 `--budget` 误传 bug 的原始版本） | ~9KB |

## 轮次总览（hearth_telemetry_r2.txt）

| 轮 | 结果 | 耗时 | 对应任务书 |
|---|---|---|---|
| B01 | TASK_FAIL rc=0 | 78.5s | T5（13-17 步失败共性） |
| B02 | SUCCESS | 98.4s | — |
| **B03** | **HANG rc=124** | 200.1s | **T4（Reflect→Plan 自旋——停滞检测为何未触发）** |
| **B04** | **HANG rc=124** | 200.1s | **T2/T3（解码失败嵌套重试穿透）** |
| B05 | TASK_FAIL rc=0 | 78.1s | T5 |
| **B06** | **HANG rc=124** | 200.1s | **T2/T3（同 B04）** |
| B07 | TASK_FAIL rc=0 | 49.6s | T5 |
| B08 | SUCCESS | 131.4s | — |
| B09 | SUCCESS | 84.9s | — |
| B10 | （详见文件内记录） | | |

## 缺失证据（如实披露）

- **B04/B06 的"解码失败"原始响应体**（provider 返回的畸形 body 本身）**不在本批文件中**——遥测脚本只留了 tail 摘要。Tier3 任务书 T1 动作 2（RUST_LOG=trace 复现 1-2 次）待执行后补档于此目录。
- 故障 session 的 `<sid>.taskgoal.json` / `<sid>.graph.json`（守门员补充 5 要求归档）待从 VM `~/.config/hearth/sessions/` 按 session_id 提取补档。

## uuid↔B 轮次映射表（补充 6 补档 · 2026-08-29 从 hearth_telemetry_r2.txt 一手提取）

| 轮 | 结果（汇总脚本） | **真实终态（report tail）** | sid | 步数 |
|---|---|---|---|---|
| B01 | TASK_FAIL | Task failed | `d7d6bada-752a-4573-90c0-506b42dcc80a` | 17 |
| B02 | **SUCCESS（误标）** | **Task failed** | `51a0fc2e-83e7-4c66-a476-552768dafd62` | 17 |
| B03 | HANG | 无 report（被杀） | 未知 | — |
| B04 | HANG | 无 report（被杀） | 未知 | — |
| B05 | TASK_FAIL | Task failed | `481c75f9-5d48-4445-9032-291ffd53893f` | 17 |
| B06 | HANG | 无 report（被杀） | 未知 | — |
| B07 | TASK_FAIL | Task failed | `6a9a9cec-0ab6-4438-b31e-e6ef01a41bc4` | **13** |
| B08 | **SUCCESS（误标）** | **Task failed** | `0955cd25-7fc1-40b5-95a4-4c0eff832e2c` | 17 |
| B09 | **SUCCESS（误标）** | **Task failed** | `477d5fc4-c2e7-4392-9b94-8bf8cb069249` | 17 |
| B10 | TASK_FAIL | Task failed | `29cabfd1-a17e-4189-8952-410e5f003c6b` | 17 |

**关键更正**：汇总脚本的 SUCCESS=rc=0 是**误标**（B02/B08/B09 report 实为 Task failed）——真实 Tier3 tally = **0 干净完成 / 3 挂起 / 7 失败**（与任务书口径一致但更严格：误标的 3 轮也是失败）。**13-17 步聚集确认**（T5 分析输入：7 失败中 6 次 17 步 + 1 次 13 步）。B03/B04/B06 无 sid（进程被杀前未产出 report——HANG 轮的固有特征，T2/T3 修复后应能在 120s 内产出 failed 终态 report）。

**T8 实锤**：runner 以 rc 判 SUCCESS 是缺陷（rc=0 ≠ Task completed）——加固方案（T8）必须改用**终态投影解析**（grep "Task completed"）判定。

## T3 复跑 failed 根因归类（Closure Window · 顶层复核 §3 · 2026-08-29）

**归类：B. Tier3 逻辑问题（活性判据缺口为主因 + T4 stall 为放大器）。与 ContextBuilder 无关；非 provider 波动。**

### 证据链（RUST_LOG=trace 复跑实录，见 t3-trace-rerun-20260829.log）
1. **输入**：`给当前目录写一个 TRACE_SUMMARY.md，介绍目录内容。不要修改任何现有文件。`
2. **工具全成功**：ls/glob/read×3/write_file 全 ✓，`wrote 2041 bytes to TRACE_SUMMARY.md` 落盘（产物真实存在）
3. **ContextBuilder 最终消息结构**：system = L1+L2 Topology；尾部 L4 = retrieval/LSP/experience/Continuity——重组正确（CB 6/6 单测 + 本次 trace 中 Continuity 正常注入）
4. **reflect prompt 实录**（err.log:293）：
   - `Goal: Explore current directory structure...` ← **图首节点 description，非 original_goal**（误导性呈现）
   - `Tasks completed: 0/2` ← **工具全成功但 TaskNode 从未被置 Completed**（活性判据 RC1/RC10/RC21 家族：节点状态更新无生产路径）
   - `Recent tool results: wrote 2041 bytes to TRACE_SUMMARY.md`（write 成功就在眼前）
5. **reflect LLM 决策**：看到"0/2 completed + 12 steps"→ 判 Replan
6. **Replan → decompose → 单节点图确定性产出同图** → `T4 semantic stall: stall_count=2 → Err(stalled)`（err.log:294-300）
7. **终态**：Task failed（13 步）——**任务实质完成但被判死**

### 判据排除
- A. ContextBuilder 回归：**排除**——活性判据与消息组装无耦合；v0.2.8（施工前）EC-03 的 T5 failed 同型（问询被当任务跑 5 步 failed）；CB 单测 6/6 过
- C. 工具执行问题：**排除**——6/6 工具调用 ✓
- D. provider/model 波动：**排除**——trace 证据完整（prompt/工具结果/决策链全可复现）；两次复跑（不同轮）同型
- E. 其他：预算未耗尽（38/50 剩）、启发式 fast-path 未触发（consecutive_errors=0、budget 74%）——非 Tier3 T5 已解类

### 修复建议（OPEN，待顶层出单——Closure 禁改 TaskGraph 事实模型）
1. **活性判据**：write_file 成功 → 标记对应 TaskNode Completed（或 plan 完成时以"本轮有成功 write"计入 progress）——RC1/RC10/RC21 的正解方向
2. **reflect prompt**：Goal 字段改用 original_goal（非首节点 desc）
3. **T4 × 单节点图**：单节点图（确定性 decompose）豁免 stall 计数，或 Replan 后首次同图不计（单节点图重 decompose 必然同图，非停滞信号）

## 优先级升级（守门员补充 2 · Final Closure · 2026-08-29）

T3 活性判据修复（OPEN）优先级**升至 D 类四件之前**：`TaskNode.status` 生产路径无更新不仅使 reflect 看到 0/N 假状态，**Continuity 块注入的 completed/remaining 同样失真**（模型一直看着假状态工作）——R2-D TaskGoal 的价值在真实任务里被打折。修复设计必须回答"node status 的生产者是谁"（推荐对齐 WS13 三重校验：写盘 verify 通过后标记，非 LLM 自报——RC1 教训）；reflect prompt `Goal:` 字段改 original_goal 小项一并修。触碰 TaskGraph 事实模型 → 独立施工单 + 顶层批准。
