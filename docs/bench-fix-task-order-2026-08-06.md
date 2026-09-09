# v24-post 基准缺陷施工任务书 — P1→P4 四件套

> 来源：`docs/bench-issues-tasklist-2026-08-06.md`（560-run 马拉松证据）
> 给执行窗口：四个缺陷全部可并行施工，均不改架构，均为既有机制的正确化

---

## P4（先做——最高杠杆）：基准落盘 session transcript

**为什么放第一**：P1 和 P2 的根因分析需要 agent 的真实对话（"它想了什么、为什么写了这个名字而不是那个"），当前 560 runs 无一条 transcript 留存。

| 项 | 内容 |
|---|---|
| 改什么 | `bench/runner.py` 的 `run_task()`——session 完成后 GET `/events` 导出 JSONL，写入 `bench/results/transcripts/{session_id}.jsonl` |
| 验证 | 跑 T00 一次 → `bench/results/transcripts/` 出现对应 JSONL 文件，含完整事件流（goal / tool_calls / results / verify） |
| 估时 | 30min |

---

## P3（简单——一行修复）：sandbox 内 cargo 不可达

**现象**：agent 反复 `bash cargo test` 失败（`cargo: 未找到命令`），消耗步数修环境而非写代码。10-run 实录中 9/10 终态为 `budget exhausted`。

| 项 | 内容 |
|---|---|
| 改什么 | `bench/runner.py` 的 `run_task()`——在 session 创建时 prepend `export PATH="$HOME/.cargo/bin:$PATH"` 到 goal 或作为 env setup；或在 sandbox 配置中 (`crates/sandbox/`) 固化 Rust 工具链路径 |
| 验证 | 跑 T00 一次 → agent 的 bash cargo test 不再报 "cargo: 未找到命令"，agent 能自测自己写的代码 |
| 估时 | 15min |

---

## P1（核心——指令遵循硬约束）：T19 命名不遵从

**现象**：560 runs 中 T19 全部失败（deepseek 0/8 + zhipu 0/20），全是 `NO_GENERIC_FN`——agent 跑完了（`phase=done`）但函数名不对。18/20 其他任务在 deepseek 上 ≥7/8。

| 项 | 内容 |
|---|---|
| 改什么 | ① `bench/tasks/T19-merge-duplicate/goal.txt`——把要求的函数签名写死（`fn parse_positive<T>(...)`），明确标注"必须严格使用此标识符名称，禁止自创/改名"<br>② （备选）`crates/agent-core/` system prompt——加一条硬约束："题目要求的符号名称（函数名/类型名/变量名）必须完全匹配，不允许创建替代名称" |
| 验证 | 跑 T19 3 次 deepseek → verify.sh grep 函数名通过 ≥ 1/3 |
| 估时 | 30min（goal.txt 改）+ 可选系统 prompt（1h） |

---

## P2（分析——需要 P4 前置）：T13 不稳定

**现象**：T13 通过率 37.5%（3/8），而其余 18/20 任务 ≥7/8。有 run 间抖动（PASS/FAIL 交替）。

| 项 | 内容 |
|---|---|
| 先做什么 | 等 P4 的 transcript 功能就绪 → 跑 T13 3 次 → 读失败 run 的 transcript → 归类失败模式（是读错了文件？改了但改错了？测试没过？） |
| 修什么 | 取决于根因分析结果——可能是 goal 描述歧义、特定代码模式 agent 不擅长、或需要增加 verify 校验步骤 |
| 估时 | 分析 1h + 修复取决于根因 |

---

## 执行顺序

```
先做（0.5h，并行）：
  P4 transcript 落盘  +  P3 sandbox cargo PATH

再做（1h，P4 产出后）：
  P1 T19 naming goal.txt 强化（不需要 transcript，直接改）
  P2 T13 root cause analysis（需要 P4 的 transcript 来读 agent 真实行为）

验证（0.5h）：
  所有修复后 → 跑 deepseek 20×1 快检
  → T19 不再全挂
  → T13 不再 < 50%
  → agent 自测不卡 cargo 环境
  → transcript 目录有对应文件
```

---

## 验收红线

| 级别 | 判据 |
|---|---|
| 🔴 | P3 修复后 agent 仍因 cargo 找不到浪费步数（regression） |
| 🔴 | P4 修复后 20×1 跑完 transcript 目录仍然空 |
| 🟡 | P1 修复后 T19 仍 0/3 deepseek |
| 🟡 | P2 修复后 T13 < 50% |
| 🔵 | P1/P2 通过率提升幅度 |