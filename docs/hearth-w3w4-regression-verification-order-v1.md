# Hearth W3/W4 回归验证单 v1（四证明 · 旧日志基线复跑）

> **性质**：验证轮指令——**默认零代码改动**（发现回归 → 偏差报告停在该点，W3/W4 范围内修复须先报偏差再动）。
> **依据**：ChatGPT 四证明要求 + 《hearth-w1w3w4-batch-report.md》+ 守门员复核（391 passed 实证）。
> **基线**：v0.2.9 / HEAD `26ad3be` 链（W3 `7c2c836` + W4 `c476e77` + 收敛 `ce3a840`）；隔离门禁 **391 passed / 0 FAILED**（`~/t_gate_r2c.log` 守门员已复核）。
> **旧基线（对照物）**：`docs/incidents/2026-08-28-tier3/`（B01-B10，真实 0/10 干净完成：3 挂起 + 7 失败）+ `release/手工测试v0.2.3.txt`（审批门/绿勾误标证据）。

---

## 0. 运行口径（全轮统一）

- VM：`.131`（`~/codex_t`，binary 0.2.9）；**`~/codex` 零触碰**。
- 环境：`HEARTH_ALLOW_NO_CGROUP=1`（开发降级姿势）。
- 通道：Agnes（`agnes-2.5-flash`，已授权预算）；撞 429 按 H4 语义停。
- 采集：每跑开 `HEARTH_CACHE_TELEMETRY=1`（顺手补长任务/compact 前后 cache open 项）；每跑记录 **rc / terminal_state / steps / duration / 人工干预次数（Delegation Friction）/ 是否触发 compact**。
- 任务选取：**逐字复用 incidents 归档的原任务 prompt**（不得改写——改写就失去基线可比性）。

---

## V-1 · 旧故障基线回归跑（证明 ①②）

用 Tier3 归档的**原任务 prompt** 重跑，对照旧基线逐项填表：

| 旧基线（2026-08-28） | 本轮复跑 |
|---|---|
| 10 轮 0/10 干净完成（3 挂起 rc=124 + 7 失败） | 目标：**≥8/10 干净完成，0 挂起** |
| 13-17 步聚集 give_up | terminal_state 分布如实记录 |
| 纯读任务 stalled | 纯读任务 completed（TC-1 类） |

- 至少覆盖：B03（多节点 Reflect→Plan 场景）、B04/B06（曾解码失败挂起——现已 T2/T3 修复）、T-A（三步 create→run→verify→record）、T-B（纯读问询）。
- 每轮跑完即记 `terminal_state`（结构化字段，不猜 rc）。
- **证明 ①** = 工具成功的任务不再因 node.status 未置位而 give_up（对照 T-A 首轮"reflect give_up 但产物在"）。
- **证明 ②** = 纯读任务不被判"不工作"（对照 T-B 首轮 stalled）。
- 若出现新失败：**偏差报告停在该点**，按 T3 模式建立证据链（trace 复跑）后归类，禁止以"Agnes 波动"结案。

## V-2 · 失败任务 resume 连续性（证明 ④ —— 本轮核心新增）

ChatGPT 原话：*"一个失败任务再次继续时，不需要用户重新教它'上次做到哪里'"*。这是从"错误提示"跨进**任务连续性**的验证。

**场景构造（3 轮）**：选一个多步任务，中途制造失败（如第 2 步给一个必然失败的命令/引用不存在文件），确认 run 以 failed 终态收场 → `hearth resume <sid>` → 用户**只说"继续"**。

**验收四条（全部满足才算过）**：
1. resume 后 agent 首轮输出**准确复述上次进度**（原始目标 + 已完成 + 剩余——即 Task Continuity 块内容），0 次重新教；
2. 续做产生**真实增量产物**（对照 R2-D 的 RESUME_OK 模式——文件在断点后继续而非从头重来）；
3. `completion_fact_check()` 在 resume 场景不误拒（Done 判定的 original_goal 对齐在断点续做下依然成立）；
4. 最终终态正确（completed 或真实失败原因，非 stalled/误 give_up）。

**记录**：每轮 `重教次数`（用户重复目标的次数，目标=0）、resume 恢复的 taskgoal/graph 字段快照、首轮输出原文。

**对照**：v0.2.3 手工测试中 resume 后"继续什么？"的历史症状（RC5/RC30 家族）——本轮必须为 0 例。

## V-3 · 六终态投影矩阵（证明 ③）

六终态各构造一次真实场景，断言 CLI 投影（✓/✗/⏸ + reason）与 run report schema 一致：

| 终态 | 构造方式 |
|---|---|
| completed | 已有（T-A 复跑） |
| failed | V-2 的中途失败场景 |
| give_up | T4 stall 场景（连续同图 replan） |
| verify_failed | 产物落盘后篡改（如跑后改文件内容再 verify） |
| timeout | `HEARTH_TASK_TIMEOUT_SECS=60` + 长任务 |
| cancelled | 交互中 Ctrl-C |

**特别断言**：中文错误输出**不得**被画成绿 ✓（RC20 回归锚——`render.rs` 已删 contains 启发式，本轮真机确认）。

## V-4 · 红线与偏差

- D 类禁改清单继续全生效（bash 截断/fs accounting/Goal Revision 接入/resource 控制流/审批委托——**本轮发现的相关现象一律记录不修**，如 RC24 再次阻塞则记录"blocked-by-RC24"跳过该轮）。
- 发现 W3/W4 行为回归 → 偏差报告停在该点，等顶层裁决后修复（W3/W4 范围内）。
- 禁止改 prompt 任务定义来"让数字好看"。

## V-5 · 交付格式（12 项沿用）

1. commit（若有偏差修复，独立 commit）
2. V-1 对照表（10 行 × 旧/新基线）
3. V-2 三轮 resume 证据（首轮输出原文 + 重教次数 = 0）
4. V-3 六终态矩阵 + 中文错误投影证据
5. telemetry jsonl（长任务/compact 样本）
6. Delegation Friction 汇总（本轮均值 vs EC-03 基线）
7. OPEN/UNKNOWN 更新
8. 偏差清单（如有）

完成后停，等顶层验收。**本验证轮通过 = ChatGPT 四证明全闭合 = W3/W4 正式收口，下一批（W6 审批语义 → W8）另行派工。**
