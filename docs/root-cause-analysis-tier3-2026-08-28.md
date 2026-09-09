# Hearth 长程任务失效 · 根因分析（第一轮）

> 评审窗口（守门人）　｜　2026-08-28　｜　性质：**源码级根因取证**，未改动实现代码
> 触发：用户指出三点——①为何他不停对 AI 说「继续」；②交付与规划是否偏离；③意图是否真被理解、工具是否对、是否按规划执行。
> 说明：**此前统计层发现（趋势矩阵）并不错**——那些现象真实存在；本文是往下一层，回答「为什么」。

---

## 0. 执行摘要：一条完整的因果链

```
根 agent 干活（ls / cat / find / head）
        │
        ├─► TaskGraph 节点永远不会被标 Completed
        │      （生产代码唯一标记点在 sub-agent 路径：loop.rs:2717-2724）
        │
        ├─► loop.rs:2818  if !any_completed { steps_without_progress += 1 }
        │      → 每一步都算「无进展」
        │
        ├─► 5 步后 planner/lib.rs:490 触发 replan；replan_count≥3 → GiveUp（:493-499）
        │      → 5 × 3 ≈ 15 步，任务必死
        │
        └─► ✗ Done (N steps) → 用户被迫输入「继续」续跑（日志实证 v0.2.5 L1152）
```

**「13-17 步聚集」由此得到精确解释**（此前我误判为"正常预算落点"，实为此公式）。

---

## 1. 现象层（此前已确认，仍成立）

| 现象 | 证据 |
|---|---|
| 长程任务失败终止 `✗ Done (N steps)` 全版本持续 | v0.1.0~v0.2.5 密度 2.3→13.4 |
| 失败步数聚集 13-17 步 | 7 例 |
| 用户需反复输入「继续」 | **日志实证**：`手工测试v0.2.5.txt:1152` `hearth> 〉继续` |
| bash 30s 超时 | v0.2.4:45 / v0.2.5:9 |
| `/dev/null` EPERM 累计 99 次 | v0.2.4:78 / v0.2.5:16 |

---

## 2. 根因层（源码实锤，本轮新增）

### 🔴 RC1 · 活性信号接线断裂（最根上的一条）

- **判定式**：`loop.rs:2813-2822`
  ```rust
  let any_completed = self.task_graph.nodes.iter().any(|n| n.status == TaskStatus::Completed);
  if !any_completed { self.steps_without_progress += 1; } else { self.steps_without_progress = 0; }
  ```
- **但节点完成只在子 agent 路径被赋值**：`loop.rs:2717-2724`（`collect_sub_agent_results()` 内 `node.status = if report.ok { Completed } else { Failed }`）。
- **第二轮兜底排查已锁死**（清点全部 `node.status` 生产赋值点）：

  | 位置 | 赋值 | 含义 |
  |---|---|---|
  | `loop.rs:1835` | `InProgress` | 根 agent 开工标记 |
  | `loop.rs:2720`（do_observe）/ `2777`（do_reflect） | `Completed`/`Failed` | **仅 sub-agent 结果**（两处都在 `collect_sub_agent_results()` 循环内） |
  | `loop.rs:2927` | `Pending` | **replan 时重置** |
  | `planner/lib.rs:203` / `430` | `Pending` | 初始建计划 |

  → **不存在任何把节点从 `InProgress` 推进到 `Completed` 的根 agent 路径**；`agent-types/lib.rs:673/787/791` 的 Completed 赋值全在测试代码。

- **追加发现（replan 抹掉进展）**：`loop.rs:2927` 在 replan 时把节点状态重置为 `Pending`——**连"已开工"的 `InProgress` 都被清掉**。因此每轮 replan 后 5 步计数器实际是从零重来，5×3=15 步的算式成立。
- **后果**：根 agent 的每一步（哪怕 productive）都被计为「无进展」。**"在干活"和"原地转"在这套度量下完全无法区分**；且不派子 agent 的任务，结构上必然在 ~15 步被判死。
- **阈值链**：`planner/lib.rs:490` 无进展≥5 → Replan；`:493-499` replan_count≥3 → GiveUp。→ **≈15 步必死**。

### 🔴 RC2 · 观测层说谎：人和 agent 看到相反结论

| 判定方 | 代码 | 规则 |
|---|---|---|
| **人**（终端显示） | `codex-cli/src/render.rs:86-92` | `contains("error") \|\| contains("failed") \|\| contains("missing")`（全文、小写） |
| **agent**（内部计数） | `agent-core/src/scheduler.rs:41` | `output.starts_with("error:")`（仅前缀） |

- **同一条命令可以得出相反结论**：`cat` 一个含 "error" 字样的 Rust 源码（命令本身成功）→ 人看到**红 ✗**，agent 判定**成功**。
- 后果：(a) 手工测试日志的 ✗ 计数**整体不可信**（我此前趋势矩阵受污染，机制已坐实）；(b) 任何人排查时都会被误导去 chasing 没发生的失败。

### 🔴 RC3 · bash 失败对 agent 隐身

- `tools-builtin/src/bash.rs:180-199`：`format_output()` 把**超时**写成 `"error: command timed out…"`、把**非零退出码**写成 `"exit code: N\nstdout:…\nstderr:…"`，但外层 `Ok(format_output(output))` —— **一律返回 Ok**。
- 配合 `scheduler.rs:41` 的 `starts_with("error:")`：
  - **超时** → 前缀匹配 → agent 能察觉 ✅
  - **非零退出码**（命令真跑挂了）→ 前缀不匹配 → **`is_error = false`，agent 完全不知情** ❌
- 后果：`consecutive_errors`（GiveUp 触发之一，`loop.rs:2797-2800`）对 bash 命令失败**结构性失明**。

### 🟡 RC4 · 规划不被执行 + 规划质量塌缩

- **不执行**：v0.2.5 L598-606 规划 7 步（探索→读源码→**跑测试**→审 bug→**分析 git 提交**→验证→总结）；实际只跑 `ls`/`df`/`find`/`head`/`cat`，**49 步未走出第 1 步**。
- **塌缩**：replan 后规划从 7 步退化为 2 步（"Set up project"/"Write the code"，L1123-1126）。
- **含不可执行步骤**：「分析 git 提交」——VM 上无 `.git`；「跑测试套件」——必然超 bash 超时。规划器不做可行性检查。

---

## 3. 与现有 taskbook v1.1 的关系

| taskbook 条目 | 与根因的关系 |
|---|---|
| T2（重试/超时穿透） | 修**下游症状**（重试叠加），真实亦存在 |
| T3（read-body 分类） | 修**下游症状**，且本轮发现其与 deepseek 通道强相关 |
| T4（停滞检测） | **方向对但落点偏**——真问题不是"没有停滞检测"，而是**停滞度量的输入信号本身是断的**（RC1） |
| T5（13-17 步聚集） | **本轮已定位精确机制**（RC1 的 5×3≈15），此前"正常预算落点"的判断应作废 |

> 说明：RC1 若修复，T4 新增的"计划哈希稳定性检测器"仍然有价值（它防的是另一类：计划原地踏步但有进展），但**优先级应低于 RC1**。

---

## 4. 待验证 / 未覆盖

- RC1 的修复方向有多种（把根 agent 的工具执行也接到节点完成 / 换一个不依赖 TaskGraph 的活性度量 / 放宽阈值），**各方案副作用不同，属顶层决策，评审不下处方**。
- `loop.rs:2778` 处 `TaskStatus::Completed` 的具体上下文未深读（同为 sub-agent 相关，但需确认）。
- RC4 中"规划器为何不做可行性检查"未追到具体代码。
- 本轮未重算 C1 统计（需按 `✗ Done (N steps)` 单独统计，排除工具级 ✗ 污染）。

---

## 5. 下一轮分析方向（待用户挑选）

1. **继续顺 RC1 下挖**：节点完成为何只接 sub-agent？是设计如此还是漏接？`loop.rs:2778` 上下文。
2. **RC3 的连带影响面**：除 bash 外，还有哪些工具把失败包成 Ok？
3. **规划器（planner）意图理解**：目标 → 7 步规划 → 塌缩成 2 步，是哪一环退化的？
4. **日志深读第二轮**：按混合法（pattern 建骨架 + 片段深读挖因果）系统扫 v0.2.4（最大份，16,863 行）。
