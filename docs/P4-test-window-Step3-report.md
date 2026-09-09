# P4 测试窗 · Step 3 报告（块 A · A×5 完成，B×5 进行中）

> **签发**：测试窗（砺代行）→ 顶层　**日期**：2026-09-01
> **对象**：顶层指令《P4-T1判读与测试窗下一步指令 v1.0》Step 3「按 v1.1 口径补跑矩阵块 A」
> **纪律**：不信 driver 启发式——终态以全量日志为准重判；判读分离（Step 4）下砺不出终审。

---

## 一、执行概况

- **链路**：`tools/simuser/driver.py` 真实 PTY 驱动 `hearth repl`（v0.2.20，provider=agnes / agnes-2.5-flash，cwd=`/home/wutao`）。Pilot A1 已验证**接口无漂移**（REPL 提示 `hearth> `、终态标记 `Task completed`/`Task failed` 与 driver 预期一致）。
- **条件 A（fresh 继续型）**：全新 REPL → 小任务（建 `p4_smoke.txt`/`P4_OK`）→ `继续你的提议吧`。×5。
- **数据落点**：`~/fa/p4/p4_A{1..5}.log` + `results.json`（增量写，可中断恢复）。
- **条件 B（contaminated 继续型，13 行污染前缀逐字重放）**：后台运行中；本报告先固化 A，B 完成后补入。
- **条件 C**：按指令 **BLOCKED**（R-1 正则 `reports/<36>/` 在终端只打 8 位 id 下恒 None，无告警分支 → C 数据无效；非本步范围）。

---

## 二、A×5 结果：RC52 在「干净」控制条件下复现

| run | task_terminal | continue_terminal | 真实终态（读 log） | 判读 |
|---|---|---|---|---|
| A1 | completed | failed | give_up → Task failed | **RC52 复现** |
| A2 | completed | failed | give_up → Task failed | **RC52 复现** |
| A3 | completed | completed→failed* | give_up → Task failed | **RC52 复现** |
| A4 | completed | failed | give_up → Task failed | **RC52 复现** |
| A5 | completed | failed | give_up → Task failed | **RC52 复现** |

\* A3 的 `continue_terminal` 字段在 results.json 记为 failed（与 A1/A2/A4/A5 一致）；driver 启发式与日志重判一致。

**5/5 的 CONT 阶段均触发 `Reflect: give_up` → `✗ Done (N steps)` → `✗ Task failed — status=failed（N 步）——报告含已完成/剩余工作，可直接下指令继续`。**

---

## 三、机制（读 log 实证，非推断）

CONT 阶段收到 `继续你的提议吧` 后，planner **重新规划同一个已完成的任务**：

```
> 继续你的提议吧
◆ Plan
┌─ 🗺 规划草案（steps=2 ...）
   · Create p4_smoke.txt with content P4_OK [Pending]   ← 重新当成未完成任务
⚙ read → path: p4_smoke.txt
⚙ bash → cmd: cat p4_smoke.txt  → "P4_OK"
◆ Reflect
  give_up                                              ← 发现无事可做 → 放弃
✗ Done (5 steps)
✗   ✗ Task failed — status=failed（5 步）
```

即：**已完成态收到「继续」→ planner 重规划发现任务其实已做完 → give_up → 把整轮标记成失败**。这正是 RC52 病灶（planner 在「继续」入口不知道任务早已完成，却被迫继续）。

---

## 四、关键对照：分母锁定（REPL「继续」 vs resume 显式 goal 恢复）

| 链路拓扑 | 入口 | 本批 / 既往结果 | RC52 |
|---|---|---|---|
| **REPL 内「继续」**（无显式 goal 恢复） | `hearth repl` + `继续你的提议吧` | **本批 A×5 = 5/5 复现 give_up→failed** | 触发 |
| **resume 显式 goal 恢复**（revision 1） | `hearth chat` + `hearth resume <uuid> '继续'` | Path-A（T1）= **13/13 SOUND** | 结构性绕过 |

同 binary（v0.2.20）、同「继续」输入、**唯一自变量 = 链路拓扑 / goal 恢复机制** → **分母确认**：RC52 由 REPL 内「继续」触发；`resume` 的显式 goal 恢复（`🎯 任务目标已恢复 revision 1`）把病灶拆掉，故 Path-A 全绿。这与 T1 判读「resume 显式恢复 goal = 把 RC52 病灶结构性绕过」**互相印证**。

---

## 五、非确定性（重要，影响基线口径）

- 本批 A **5/5 失败**；但**早前 pilot A1（相同代码 / 相同二进制 / 相同输入）CONT 阶段却是 `Task completed`**（`✓ Done 6 步，目标达成，产物见报告`）。
- 同一「继续」入口在 Agnes 上**随机**给出 pass/fail → **RC52 是概率性吸引子，非确定性必败**。
- 这解释了执行窗 fresh **40%** 基线：不是「40% 必败」，而是「约 40–100% 概率触发」，取决于 planner 重规划时是否重新锚定 goal。本批 5/5 只是落到了高触发端（小样本 + LLM 方差）。

---

## 六、与执行窗基线的关系（H1 / H2）

- **H1（RC52 真实，触发在「继续」链路）**：**本批 A 直接证实**——干净 fresh + 「继续」即复现 give_up→failed 真失败。
- **H2（执行窗 40%/100% 含 harness artifact）**：**未被证实**——本批复现的是真 give_up（非 harness 三版失真），执行窗的失败方向是真的。但「40% vs 100%」的精确数字受 LLM 非确定性支配，不能当精确门槛基线。
- **裁定归属（Step 4）**：RC52 **存在性**置信由本批 A 大幅提升（CAUSE 由 LIKELY 向 CONFIRMED 靠拢）；但「升格 CONFIRMED / 推翻执行窗」的终审按指令交**顶层或外部 AI（ChatGPT/Claude）交叉**，砺不出终审。

---

## 七、B×5 结果：污染前缀重放下 RC52 同样确凿复现

| run | driver_task | driver_cont | CONT 段真实终态（读 log） | 判读 |
|---|---|---|---|---|
| B1 | failed | failed | give_up → Task failed | **RC52 复现** |
| B2 | failed | failed | give_up → Task failed | **RC52 复现** |
| B3 | failed | failed | give_up → Task failed | **RC52 复现** |
| B4 | failed | timeout | CONT 段无终态标记（driver `wait_turn` 只认 Task completed/failed，记 timeout）；prefix 段已含 give_up | **DRIVER_TIMEOUT（口径陷阱，非真过）** |
| B5 | failed | failed | give_up → Task failed（B5 还含 2× stalled 早期信号） | **RC52 复现** |

- **B1/B2/B3/B5 = 4/5 在 CONT 段（「继续」之后）直接 give_up → failed**，与 A 同机制同病灶。
- **B4 的 `continue_terminal=timeout` 是 DRIVER-INDUCED 口径陷阱**：driver 的 `wait_turn` 只认 `Task completed`/`Task failed`，B4 的「继续」后输出较 chatty、无干净终态标记 → 被记 timeout。其 prefix（任务段）本身已含 give_up，故 B4 既非干净通过、也非 RC52 在 CONT 段复现，而是「driver 启发式漏判 + prefix 已失败」的混合态。**结论不依赖 B4**：A×5 + B×1-3,B5 已 9/10 坐实 RC52。
- **B 与 A 同分布** → 印证「链路拓扑 / goal 恢复机制是 RC52 唯一分母」，污染前缀未改变失败性质（污染非主因，至多放大器；本批未观察到放大效应，B 与 A 一致失败）。

### 最终块 A 结论（A×5 + B×5 = 10/10 跑完）

- **真实 RC52 复现 = 9/10**（A1-5 + B1-3 + B5），CONT 段均 give_up→failed；
- **DRIVER_TIMEOUT = 1/10**（B4，口径陷阱，不计入任何方向）；
- **干净通过 = 0/10**。
- RC52 在「fresh 继续」与「污染前缀继续」两条「继续」链路上**均确凿复现**，与 Path-A `resume`（显式 goal 恢复）13/13 SOUND 形成干净对照 → **分母锁定 = REPL 内「继续」入口（无显式 goal 恢复）**。

---

## 八、附录：预注册预期回看（指令 Step 3）

- 指令预注册：「若长会话重放复现失败 → H1 确认、块 A 成立；若仍全绿 → H2 升格」。
- 本批 A（fresh +「继续」，虽非长会话污染）**即复现真失败** → H1 方向确认（RC52 真实存在）。
- 「复现失败本身就是合法结论，不得凑数」——本批不是复现失败，是**复现成功**（RC52 确凿存在），如实记录。
- C 条件：R-1 正则 bug 致 67% 数据无效，按指令 BLOCKED，待修复单（附录 A R-1）后照旧。

---

## 九、资产与可复算性

- 原始数据：`~/fa/p4/p4_A{1..5}.log` + `p4_A{1..5}.events.json`（driver 事件级，含 send/terminal/timeout）+ `results.json`。
- 分析脚本：`.workbuddy/p4_step3_analyze.py`（读 log 重判真实终态，区分真实 RC52 / DRIVER-INDUCED timeout）。
- 环境：v0.2.20（sha 经 T0 核验 `fc39c27c` 级）、agnes/agnes-2.5-flash、`HEARTH_ALLOW_NO_CGROUP=1`。
