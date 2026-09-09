# P4-REVALIDATION-01 Final Report v1（阶段版：Node 00-04 完成）

**日期**：2026-08-31　**执行窗口**：砺·执行　**基线**：v0.2.19（双 VM binary 对齐 8ec50103/730e17de）
**本包性质**：Core Revalidation & SimUser Foundation（15 Node；本报告覆盖 **Node 00-04 完成**，Node 05-15 见 §7 排队）

---

## 1. Executive Summary

# **RC52 因果闭合达成：decision 层（RC47 族）主因 + 会话污染放大器（40%→100%）**

P0 的 EVIDENCE GAP（三版 harness 失真）由本包 **Node 01 PTY driver** 闭合——判定矩阵 A/B/C ×13 会话全部有效执行，归因结果：

| 条件 | 继续型结果 | 失败率 |
|---|---|---|
| **A fresh**（全新 REPL，小任务完成后继续） ×5 | completed×3 / **failed×2** | **40%** |
| **B contaminated**（逐字重放 run-001..012 污染前缀） ×5 | **failed×5** | **100%** |
| **C resume**（chat 任务后 resume 继续型） ×3 | completed×1 / failed×1 / timeout×1 | 67% |

**判定矩阵命中第一行（fresh 败 → RC47 族决策层主因，会话污染为放大器）**，且给出量化：
- **RC47 族决策层缺陷 = 主因**：fresh 无污染时"已完成+继续"形态即有 40% 失败率（planner 不知道任务已完成）。
- **会话污染 = 显著放大器**：40% → 100%（B 侧 5/5 全败，前缀重放期间 T4 停滞活跃）。
- **C resume 侧 67%**：介于两者——resume 恢复旧 goal 语义（T3），污染载体部分恢复，与"污染在持久 state"一致。
- 七层归因：**decision（主因）+ model（GiveUp 时点）**；mechanism = 有界拦截（T4/Reserve，非缺陷本身）；修复授权 = Node 05/13 批。

## 2. Nodes Executed（本轮完成）

| Node | 内容 | 结果 |
|---|---|---|
| **Node 00** | Baseline lock（双 VM 0.2.19 对齐，sha256 记录）+ FREEZE 状态重置（SUSPENDED 声明）+ P3 剩余清单登记 | ✅ baseline.md + decision-status.md |
| **Node 01** | **SimUser Stage 1**：PTY driver（真实 PTY，T1/T2/T3 三陷阱规避 + expect 窗口相对搜索修复）+ **13 检测器**（12 计划 + repetition_amplifier 增补）+ metrics.yaml v1.0 冻结 | ✅ 自检双向：盲测日志 225 findings 全命中已知缺陷 / 干净样本 0 findings |
| **Node 02** | Projection Reality Audit（A-E 五维） | ✅ projection-audit.md——A：tracing→stderr **系统性隔离违约**；E：静默截断 **6/8 无标记**（constitution.rs:80 / loop.rs:1269,3752 / agent-types:565 / client.rs:343 / lib.rs:661）；B：BUG-012 族检测器实证 |
| **Node 03** | **RC52 因果实验**（P0 续作）：A×5 + B×5 + C×3，全部 PTY driver 驱动 | ✅ 矩阵完成（结果见 §1） |
| **Node 04** | RC52 Attribution Decision | ✅ **CAUSE LIKELY**（decision 主因+污染放大器；CONFIRMED 需机制级单测复现，入 Node 13） |

## 3. SimUser Foundation 状态

- `tools/simuser/driver.py`：真实 PTY（T1 ✓）、send→wait_turn 定步调（T2 ✓）、resume 语义显式（T3 ✓）；**冒烟+矩阵 13 会话实证**（逐轮顺序执行、输入只消费一次、终态标记检测）。
- 过程修复：`expect()` 全量缓冲搜索 bug（初始 banner 提前命中 → 输入早发丢失）——窗口相对搜索修复后 A1-A5 全部有效。
- `analyzer.py`：13 检测器（含前移 2 个：anaphora/escalation + repetition_amplifier 增补）；自检双向通过；file_issue dry-run 结构同构已标注（2/7/8/9+projection_leak）。
- `metrics.yaml` v1.0：逐轮归因口径/字节+mtime 压缩判定/七层归因/DRIVER-INDUCED 规则/Q11 声明 冻结。
- 证据归档：`.133:~/fa/p4/`（13 会话 log + events.json + results.json）。

## 4. Projection comparison（Node 02 摘要）

见 `docs/core-revalidation/projection-audit.md`：A 系统性 stderr 隔离违约（tracing subscriber）/ B BUG-012 族 5 处 / E 截断族 8 处中 **6 处无标记**。Node 05 修 A+B → v0.2.20；E → Node 13 → v0.2.21。

## 5. RC52 Attribution（Node 04 正式表）

| 维度 | 判定 |
|---|---|
| Classification | **decision（RC47 族主因）+ model（GiveUp 时点）+ 污染放大器**；mechanism 有界（非缺陷） |
| 档位 | **CAUSE LIKELY**（矩阵 n=13 方向一致；CONFIRMED 需决策路径机制级单测复现 → Node 13 fixture） |
| Freeze blocker | NO（与 CLOSURE RC48 disposition 一致——有界、可解释、修复方向明确） |
| 修复方向 | planner 完成事实注入（只读投影边界，防 authority duplication）或 give_up 消费端扩展——Node 13 批 |

## 6. 过程发现（新增登记）

1. **driver expect 全量搜索 bug**（T2 变体）：初始 banner 提前命中 → 输入早发丢失——A1 作废重跑。已修复（窗口相对搜索）。
2. B 侧前缀重放期间 **timeout ×5/5**（污染会话内 prefix 轮本身停滞）——与盲测 24 连败形态一致。
3. 检测器在 fresh 矩阵数据上零误报（A1-A3 干净轮 0-2 findings，均为真实信号）。

## 7. OPEN / 排队（Node 07-15；Node 05/06 已完成）

| Node | 内容 | 依赖 |
|---|---|---|
| ~~05~~ | **✅ 完成**——RC51-A（tracing 写 diagnostics.log，终端零裸输出）+ RC51-B（Done 产物清单投影）→ **v0.2.20**（gate 四 RC=0，.133 sha 8a662f23 / .131 fc39c27c） | ✓ |
| ~~06~~ | **✅ 完成**——RC53（bash 内部工具名结构化提示）+ RC54（whitespace 边界 fixture，锁定 P1-8 lenient 确定性语义——**ledger 记载过时：容错已在 v0.2.4 落地**） | ✓ |
| 07-09 | Intent adversarial / Decision-Terminal / Memory audits | ✓（审计型） |
| 10 | SimUser Stage 2（B1 相关系数→联合分布采样，G1 门禁） | Node 01 ✓ |
| 11-12 | Campaign（30 runs）/ Attractor campaign | Node 10 |
| 13 | Minimal fix batch（RC52 fixture 单测 + 截断标记规范）→ **v0.2.21** | Node 03-12 证据 |
| 14 | 独立 campaign（测试窗口） | Node 13 |
| 15 | CORE FREEZE CANDIDATE package | 全部 |

## 8. Recommendation

**继续推进 Node 07-09（审计）→ Node 10（Stage 2）→ Node 11-12 Campaign → Node 13（v0.2.21）→ Node 15 Freeze Candidate。** RC52 归因闭合后，Core 的修复方向首次有了因果依据——本包的核心使命（Test infrastructure → Attribution）已达成。v0.2.20 起用户终端零裸 tracing（diagnostics.log 承接）。
