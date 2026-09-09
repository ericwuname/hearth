# False Stop Lifecycle — CORE FREEZE REVIEW-01 Node 03/04（RC48）

## 样本

| 跑 | 终态 | 步数 | VERIFICATION_RESERVE | GIVE_UP_INTERCEPTED | GIVE_UP_OVERRIDDEN | 独立复验 |
|---|---|---:|---:|---:|---:|---|
| n12r1 | failed | 21 | 1 | 1 | 0 | sortlib **passed** |
| n12r2 | failed | 33 | 0 | 0 | 0 | sortlib **passed** |
| **n12r3（可比性复现，同任务同参数）** | **completed** | 31 | 1 | 1（含） | **1** | sortlib **passed** |

（n12r2 终态 = approval_denied_noninteractive——另一拓扑，见下。）

## 三假设检验结果（砺批-1）

- **假设 A（criteria 非空 + Reserve 零和）——✅ 成立（精确化）**：n12r1 日志实证 `VERIFICATION_RESERVE=1` + `GIVE_UP_INTERCEPTED=1`（早期核验失败：GiveUp 时点产物未达标 → Reserve 消耗 → replan）；模型随后真实修复（独立复验 passed）；之后 T4 停滞/低位终止时**无第二次核验机会** → failed。criteria 非空（cmd cargo test + file contains n - 1 - i）→ RC47 路由（criteria 空分支）不适用——**与 RC47 已修形态的分界 = criteria 空与非空**。
- **假设 B（progress 语义）——✗ 非直接原因**：r1 终态错误为 T4 stall（"2 consecutive replans identical"）而非 noprogress 规则臂；steps_without_progress 在 write 成功时已被重置（W3 fixture）。
- **假设 C（planner 无 acceptance 感知）——✅ 贡献因子（叠加）**：planner 决策点看不到 acceptance_result → 修复完成后仍 decompose 同图（触发 T4）或放弃。`grep acceptance planner/ = 0` 老根因（修 A 备而未用条款）。

**n12r3 决定性反向证据**：同任务同参数下 GiveUp 时点落在 Reserve 可用时 → 拦截 → 核验 **通过** → GIVE_UP_OVERRIDDEN → completed 全链路正确。即 **机制在有核验机会时完美工作**——false stop 的机制边界 = 核验机会零和的时点性。

## 层归

- **mechanism（设计裁决）**：Reserve ≤1 = 零和防死循环设计（INV-LR04 有界性代价）——false stop 是其**已知有界边界**，非缺陷逻辑错误；有 Reserve 时全链路正确（r3）。
- **model**：GiveUp 时点不可控（早期在未达标时点放弃 = r1 触发条件）+ 修复完成后仍同图 decompose（假设 C）。
- **decision**：无 authority 冲突（决策权/否决权分离，有意设计）。

## Node 04 修复边界裁定

- 许可修法（Reserve 分账 RC45 预案 / give_up 消费端对 "acceptance passed in scratch" 的消费扩展）**均须顶层批准**（砺批-1）→ **本轮不实现**。
- RC48 收敛为 **F1 "可解释、有界、已声明"**：根因=核验机会零和×GiveUp 时点；边界=Reserve 上限（防无限循环）；证据=n12r1/r2/r3 三跑对照；处置方向=上述二选一待批。
- 按批-8 预判：不构成 NOT READY（非稳定机制缺陷且修法越界——修法在消费端局部可做，只是须批准）。
- **注**：r3 证明该形态的 model variance 显著（同参数 1/3 completed）——若顶层批准消费扩展，预期消除此类 false stop。
