# Hearth CORE FREEZE CLOSURE-01 Final Report v1

**日期**：2026-08-31　**执行窗口**：砺·执行　**基线**：v0.2.18（tag，HEAD `5b0c7b9`）　**生产代码改动：0**（no-code gate ✓）

## 1. Executive Summary

# A. Verdict：**CORE FREEZE**

- **F0 = 0**（六项四层复核）；**F1 ×4 全部 ACCEPTED DEVIATION**（零 BLOCKER）；**RC49 规范链重跑闭环**（Long-run 柱 compact 腿恢复）；**Authority 单源复核通过**（零双 authority）。
- 本轮生产代码改动 = 0（no-code gate ✓）；v0.2.18 保持，无人为 bump。
- 零 STOP 触发；§0 禁区零触碰；四项禁施工项零施工。

## 2. Baseline / Provenance

见 `docs/data/core-freeze-closure-20260831/node00-provenance.md`：三方一致（source=binary=gate=0.2.18）；.131 sha256 `9e92dfc9…`；.131 诊断矛盾消解（当前 binary 干净，strings COMPACT_DBG=0；CFR 时代证据=显式接受含诊断构建）；磁盘 36G free。

## 3. F0 Recheck（Node 03）

六项 × 四层（源码/fixture/真机/独立复验）全表见 `closure-audit.md` Node 03——**F0 = 0**。

## 4. Authority Matrix

见 CFR `authority-matrix.md`（15 权项）+ `closure-audit.md` Node 02 Boundary Matrix（14 domain 全 FROZEN）。零双 authority/零无 authority/零越级；give_up 决策权/否决权分离=有意设计。

## 5. RC48 disposition

**mechanism(bounded 设计) + model → ACCEPTED DEVIATION + RC48-FOLLOWUP RFC（Freeze blocker = NO）。**
证据：n12r1（Reserve 耗尽→false stop）对照 n12r3（Reserve 在→拦截→核验通过→completed 31 步，独立复验 passed）；假设 B（mechanism 污染）排除。两案（Reserve 分账/acceptance-passed 消费扩展）须顶层批准。

## 6. QA completion-awareness disposition

**ACCEPTED DEVIATION（model/decision limitation）。** 注入 completion facts 会产生 authority duplication（批-5 判据成立）；若立项 RFC，边界=只读投影且不得用于 GiveUp 判定，或直接不做。

## 7. Archive C disposition

**ACCEPTED DEVIATION，NO BLOCKER。C = UNKNOWN**（v2/v3 污染、v4 setup 失败——三版定性）；A=PASS（落盘实证）、B=PASS（提示 fixture）。禁升 PASS/FAIL ✓。

## 8. 40-slice disposition

**ACCEPTED DEVIATION。** 判级依据：bounded declared loss-to-LLM（标记存在+state 存活+Continuity 投影）**≠ silent loss**——F0-2 不命中关键。follow-up=切片入 archive 或标记修正（冻结后独立单）。

## 9. Memory invariant（Node 08）

`inv_m01` = **2 passed**（v0.2.18）。A=PASS / B=PASS / **C=UNKNOWN** ✓。

## 10. Resume invariant（Node 09）

reteach=0 + duplicate execution=0 证据对应 v0.2.18（chainfree 链=干净二进制重跑，RC49 闭环同时复证 resume 腿）。

## 11. Reliability Matrix（Node 11）

13 能力全 PASS（4 项带 F1 边界*），每项 evidence reference 见 Final Report v1 CFR 版 §11 与 `closure-audit.md`；WHAT/WHERE/HOW/CONDITIONS 全部落表，无裸断言。

## 12. F1/F2 disposition（Node 12）

| F1 | Classification | Bounded | Core violation | Blocker | Disposition |
|---|---|---|---|---|---|
| RC48 | mechanism(bounded)+model | YES | NO | NO | **ACCEPTED DEVIATION** + RC48-FOLLOWUP RFC |
| QA completion-awareness | model/decision | YES | NO | NO | **ACCEPTED DEVIATION** |
| Archive C | evidence gap | YES | NO | NO | **ACCEPTED DEVIATION**（C=UNKNOWN） |
| 40-slice | mechanism declared | YES | NO | NO | **ACCEPTED DEVIATION** |

F2：6 项 DEFER（清单见 known-deviations.md）。

## 13. Known limitations（委托方需知）

**What Hearth Core guarantees**：五态结构化执行、双证据完成门、分类纯函数、archive 先行压缩、state 层事实跨边界存活、resume 零重教育、沙箱不可绕过。
**What Hearth Core does NOT guarantee**：archive 模型恢复（C=UNKNOWN）、40 切片细节 LLM 可见性（lost-to-LLM）、无 false stop（RC48 约 2/14 复发率）、Agnes 无方差。
**Future RFC**：RC48-FOLLOWUP / QA completion-awareness（只读投影边界）/ 切片入 archive / C-probe 干净重试。

## 14. Freeze boundary

14 domain 全 FROZEN（Boundary Matrix）；冻结后修改须经独立 RFC；Evolution backlog（Subagent/TUI/MCP/Persona/Semantic Memory 等）不属 Core 缺口。

## 15. Final Decision

# **CORE FREEZE**

（Node 13 九条 BLOCKER 规则逐条核销全通过；本轮 RC49 重跑闭环消除了唯一翻盘项）

## 16. Reproducibility commands

见 `docs/core-freeze-review/review-index.md`（硬格式 grep/命令+预期输出）+ `docs/data/core-freeze-closure-20260831/node00-provenance.md`（RC49 evidence：ARCHIVE_BEFORE=120/MID=120/AFTER=120+1 session file）。
