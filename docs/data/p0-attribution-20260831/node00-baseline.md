# Node 00 — Baseline / 环境快照 / 重放剧本（P0-ATTRIBUTION-01）

日期：2026-08-31　执行窗口：砺·执行

## 环境快照（统一 .133 = 盲测主机）

| 项 | 盲测原始（日志 L1-67） | 本轮复现 |
|---|---|---|
| 主机 | **.133**（L4 `ssh wutao@192.168.220.133` 实锤） | **.133** ✓ |
| binary | hearth 0.2.18 (unknown) | 0.2.18 + env-gated dump，**sha256 `bffe2dc0cfbbf73d…`**（全值见下） |
| 启动命令 | `hearth repl`（cwd=`/home/wutao`，无 budget flags） | 同（cwd=$HOME，无 budget flags——goal_tier 启发式与盲测一致） |
| session | e3507bf9（单 REPL 会话全程） | 每跑独立 REPL 会话（B 组每跑重放前缀；A 组每跑独立 fresh） |
| env | HEARTH_ALLOW_NO_CGROUP 未设/HEARTH_URL 未设 | 同 + `HEARTH_DEBUG_PLANNER_INPUT=1`（dump 开关，本包唯一许可改动） |
| provider | Agnes / agnes-2.5-flash（config.toml） | 同 |
| budget | 无显式——goal_tier 启发式（economy 20/standard 50/premium 100） | 同（盲测 run-013 18 步 ≤ economy 20 吻合） |
| 时间口径 | UTC | UTC；Agnes 延迟 28-55s 波动 → 每条件 ≥3 跑 |

binary sha256（.133，dump 构建）：`bffe2dc0cfbbf73d`（前 16 位，全值记录于运行机）。
dump 开关说明：`HEARTH_DEBUG_PLANNER_INPUT=1` → dump 到 `~/.config/hearth/debug/<session_id>/`：
`plan-<ms>-prompt.txt`（planner 实际输入逐消息快照）/ `graph-<ms>.txt`（TaskGraph sig+节点全文）/
`compact-<ms>-summary.txt`（压缩摘要全文）。零控制流影响；未开时零开销。

## 重放剧本（run-001..012 逐字，截止点定死 run-012 结束处）

来源：`release/手工测试v0.2.18.txt`（3703 行）。用户输入提取口径 = `hearth>` 行；砺 run 编号
（L265-267 三行 = 逻辑 run-004 一次交互）。**逐字剧本（15 条输入行 = 13 逻辑 run）**：

```text
R01: 现在怎么样啊，身体通不通
R02: 木有特定的检查项，你自己先自检一下，看看
R03: 不用检查代码库，就是看你能不能做哪些事，住户体验，你住在hearth这个harness上
R04: 2026-08-31T05:00:49.591654Z ERROR agent_core::r#loop: do_plan_inner failed error=stalled: 2 consecutive replans produced identical TaskGraph (4 nodes) — semantic progress absent, giving up early (Tier3 T4)
R05: （空行提交——盲测 L266）
R06: 你的完整自测计划是什么，怎么设计的
R07: 那你继续跑吧
R08: 继续跑吧
R09: 现在是否偏离了，我说继续跑吧，但是你目前输出的内容是：**T12 观察第 4 轮（新数据）**：✓ Done (14 steps)
R10: 那你继续吧
R11: 上一轮放弃了，✗ Done (5 steps) 这个原因是什么
R12: 你的本轮回复是，✓ Done (21 steps)。就这个✓ Done (21 steps)。但是我不知道你发生了什么，我需要猜你的结果，这个是不是一个问题。
R13: 你登记到台账吧。
R14: 现在还有什么可以测试的吗
R15: 继续你的提议吧    ← 吸引子入口（砺 run-013，RC47 族判定实验的"继续型"请求）
```

剧本 hash：sha256(本节文本块) 见 `replay-script.sha256`。元诊断型请求 = R14 原文
（"现在还有什么可以测试的吗"?? **注意**：元诊断型 = run-014 输入 L1780「当前轮出现了✗ Done (18 steps)，这个是出现了什么情况呢」——v4 探针中采用此条）。

**Condition A 填充对话（12 轮非污染中性问答，零工具零写盘）**：Rust 所有权/幂等性/HTTP 409/
事件循环/TCP 握手/线性一致性/JSON vs YAML/死锁/索引原理/分代 GC/乐观锁/Redis vs Memcached
（全文见 run_rc49 同目录脚本）。

## 判定矩阵（总包 Node 02）

| fresh 继续型 | polluted 继续型 | 结论 |
|---|---|---|
| 失败 | 失败 | 主因 = RC47 族（决策层），污染为放大器 |
| 成功 | 失败 | 主因 = 会话污染 |
| 成功 | 成功 | EVIDENCE GAP（回查重放保真度） |

每条件 ≥3 跑；报 mean/min/max；错峰执行。
