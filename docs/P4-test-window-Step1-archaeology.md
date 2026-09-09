# P4 测试窗 · Step 1 差异考古报告（砺·评审代行）

> **签发**：测试窗（砺代行）→ 顶层　**日期**：2026-09-01
> **对象**：顶层指令《P4-T1判读与测试窗下一步指令 v1.0》Step 1「差异考古」
> **纪律**：不信报告信源码——直接 `bash -lc` 读 `.131` 上文件，不凭指令书标签下结论。

---

## 一、核心结论（一句话）

**顶层指令 Step 1 指向的 `~/fa/ab_A1..A5.log` / `ab_B1..B5.log` 不是 RC52 矩阵日志，而是 P2 Node 05/06 的「上下文压缩 A/B 实验」（压缩阈值 32K vs 65K，任务是「执行 12 条 seq + 写 RESULT.txt」）。** 因此**无法**用这些文件构建「40% 与 0% 之间哪个变量是分母」的差异表——它们测的是与 RC52 无关的第三个自变量（压缩阈值）。

真正的 RC52 拓扑数据在 `.131` 上**其实存在**，但位于 `cfr_*` / `lr_*` 日志（REPL 多轮「继续」实验，含真实 `give_up` / `stalled` 失败），**且未以 fresh/contaminated/resume 三条件矩阵形式组织**；其生成器 `run_rc52_matrix.py` 在 `.131` **不存在** → 执行窗 Final Report 的 40%/100%/67% 基线在 `.131` 上**无可恢复源**（与顶层挂账 #3 互证）。

**Step 1 的正确产出**：纠正指令前提错误 + 给出「规格级」自变量差异表（v1.1 口径 vs 我方 p4-rerun 配置），并确认 H1/H2 判别**只能靠 Step 3 重跑**，考古本身不能答。

---

## 二、逐项证据（均 `.131` 实地 `bash -lc` 取得）

### 2.1 `ab_*.log` 真实性质 —— 压缩实验，非 RC52
- `~/fa/run_ab_all.sh` 头注释：`# P2 Node 05/06: A/B interleaved orchestrator (A,B,A,B,... x5 pairs)`。
- `~/fa/run_ab.sh` 本体：
  ```
  # P2 Node 05/06: A/B paired long-conversation experiment (interleaved runner)
  # A = baseline threshold 32,000 (default)   B = candidate 65,536 (delayed compaction proxy)
  if [ "$COND" = "B" ]; then export HEARTH_COMPACT_CHAR_THRESHOLD=65536; fi
  hearth chat '依次用 bash 执行以下 12 条命令 ... seq 1 1500 ... seq 22001 23500 ... 写 RESULT.txt ...' --budget 80 --acceptance 'file: RESULT.txt nonempty'
  ```
- 任务文本 = 12 条 `seq` 命令 + 写 RESULT.txt（**与 RC52「继续你的提议」入口毫无关系**）。
- `collect_ab.py` 用 `compacted_turns`（读 `~/.config/hearth/archive/compacted.jsonl`，按时间窗归属）作为 A/B 判别量。
- `ab_results.json` 汇总：A 终态 `[failed(give_up), completed×4]`，B 终态 `[failed(stalled), completed×4]`；失败模式是「seq 链式任务 give_up/stalled」，**非** RC52 的「已完成态被误判/假停」。
- `ab_all.log` = **0 字节空文件**（指令书称其为「矩阵日志」的候选之一，实为占位空）。

### 2.2 RC52 入口短语在 `ab_*.log` 中不存在
`grep -rilE '继续你的提议|run-013|run_rc52|RC52' ~/fa` 命中：`lr_n11.log` / `cfr_n11.log` / `analyzer.py` / 我方 `p4-rerun/*`（测试窗 Path-A 日志，因 resume 用了「继续」字样）。**`ab_*.log` 无一命中** → 坐实 ab 系列非 RC52。

### 2.3 `run_rc52_matrix.py` / `driver.py` 在 `.131` 全盘不存在
`find ~ -maxdepth 4 \( -name 'run_rc52_matrix.py' -o -name 'driver.py' \)` → **空**。印证顶层挂账 #3：执行窗 C 条件 67% 的实际产生路径在 `.131` 无对应部署物，需执行窗自证。

### 2.4 真正的 RC52 拓扑数据在 `cfr_*` / `lr_*`
- `cfr_n11.log` 头：`Hearth REPL（直跑模式，连续对话）`，任务「我有点烦，帮我创建 notes/ 目录…」，多轮 `continue` 反射。
- 含真实 RC52 签名失败：`ERROR ... do_plan_inner failed error=stalled: 2 consecutive replans produced identical TaskGraph (2 nodes) — semantic progress absent, giving up early (Tier3 T4)` → `✗ Task failed ... 可直接下指令继续` → `give_up`。
- `collect_lr.py` 头注释：`P2-LR Node 14: Reliability Matrix collector`，任务族含 `n08b_resume` / `n11_qa` / `n13a_stress1` 等——是 **REPL 多轮可靠性矩阵**，但**按节点（n06..n13）而非 fresh/contaminated/resume 三条件组织**，无 40/100/67 口径。
- 结论：`cfr_*`/`lr_*` 是 RC52 拓扑的「近亲」实验（REPL 多轮 + give_up），但**不是**执行窗 Final Report 所声称的那张 A/B/C 矩阵，且 40%/100%/67% 数字无从这些文件反推。

---

## 三、自变量差异表（规格级：v1.1 口径 vs 我方 p4-rerun）

> 因 `ab_*.log` 非 RC52 矩阵，下表以**指令书 v1.1 规定的 RC52 矩阵自变量** 对比 **我方 Path-A（p4-rerun）实际配置**——这才是回答「40% 与 0% 之间哪个变量是分母」的正确对照面。

| 自变量 | v1.1 / campaign directive 口径（RC52 矩阵块 A） | 我方 p4-rerun（Path-A 实测） | 是否同分母 |
|---|---|---|---|
| **链路拓扑** | REPL（PTY 交互）内任务完成后发「继续」型输入（同会话多轮） | `hearth chat` 一次 + `hearth resume <uuid> '继续'`（跨会话显式恢复） | ❌ **不同——最关键分母候选** |
| **goal 恢复机制** | REPL 内「继续」= planner 重新读上下文，RC52 病灶（不知已完成）可被触发 | resume 显式 `🎯 任务目标已恢复(revision 1)` 结构性绕过病灶 | ❌ 不同（resume 把病灶拆了） |
| **任务** | 黄金集任务（长、多步、多轮累积） | 1 步可验证 `DONE.txt` | ❌ 不同（无长会话累积污染动力学） |
| **污染** | 逐字重放 run-001..012 前缀 | 目标前缀改写近似 | ❌ 不同（放大器是否激活不可对照） |
| **acceptance** | `file: <workspace 相对路径> nonempty` | 同 | ✅ 同 |

### 3.1 「哪个变量是分母」的判读
- **第一分母候选 = 链路拓扑 + goal 恢复机制**：resume 显式把 goal 重新锚定，使「planner 不知任务已完成 → 假停/假完成」这一 RC52 病灶在 Path-A 中结构性消失。这直接解释「执行窗 40% fail vs 我 0% fail」。
- **第二分母候选 = 任务长度 / 长会话累积污染**：黄金集长任务才可能触发累积污染放大器；1 步 DONE.txt 不触发。
- **`ab_*.log` 的压缩阈值（32K vs 65K）= 第三个无关自变量**，不在 H1/H2 的分母集内 → 用 ab 日志判 RC52 是**范畴错误**。
- 因此：H1（执行窗 40%/100% 真，触发在长会话污染）与 H2（执行窗数据含 artifact）**无法由考古分辨**——必须 Step 3 按 v1.1 口径重跑块 A，看长会话重放是否复现失败。

---

## 四、对指令 Step 1 的纠偏与下一步

1. **指令前提错误已纠正**：`ab_*.log` ≠ RC52 矩阵（实测为压缩实验）。特此如实上报，未基于错误前提编造分母结论。
2. **RC52 真实数据位置**：`.131` 上的 RC52 拓扑数据在 `cfr_*`/`lr_*`（REPL 多轮 + give_up），但非三条件矩阵、40/100/67 无可恢复源（`run_rc52_matrix.py` 缺失）。
3. **H1/H2 判别路径**：考古已完成其职责（排除错误前提、锁定正确分母集），最终判别须 **Step 3** 重跑块 A（driver.py PTY 链路 + 逐字重放污染前缀）。
4. **判读分离**：本步仅出「数据性质判读 + 规格级差异表」，涉及「升格/推翻执行窗结论」的裁决交顶层/外部 AI 交叉（Step 4），砺不出终审。

---

## 五、附：`.131 ~/fa` 关键文件清单（实测）
- 压缩实验：`ab_A1..A5.log` / `ab_B1..B5.log`（各 ~9–38KB）、`run_ab.sh` / `run_ab_all.sh` / `collect_ab.py` / `ab_results.json` / `ab_all.log`(空)
- REPL 多轮（RC52 近亲）：`cfr_n06..n11.log` / `lr_n06..n13*.log` + `run_cfr_*.sh` / `run_n*.sh` / `collect_lr.py` / `lr_matrix.json`
- 测试窗 Path-A：`p4-rerun/<cond><idx>/`（A1-5/B1-5/C1-3）
- **缺失**：`run_rc52_matrix.py` / `driver.py` / `metrics.yaml` / 黄金集 v0.2.18（均不在 `.131`，需本机 sftp，见 Step 2）
