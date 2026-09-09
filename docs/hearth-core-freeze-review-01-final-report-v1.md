# Hearth CORE FREEZE REVIEW-01 Final Report v1

**日期**：2026-08-31　**执行窗口**：砺·执行　**基线**：v0.2.17　**终态**：**v0.2.18**（tag 待确认于本报告后；gate 447/0 @ `.133:~/t_gate_cfr_final.log`）

## 1. Executive Summary

# **CORE FREEZE READY WITH ACCEPTED DEVIATIONS**

（本结论为执行窗口建议——最终裁决归人工独立评审窗口 + 外部 AI 双窗，§18/§21）

- **F0 = 0**（实证）：零假完成 / 零静默丢失 / 零重教育 / 零无限循环 / 零沙箱绕过 / 零 Projection 失真 / 零 provenance 错位。
- **F1 = 4 项**（全部"可解释、有界、已声明"，§15）：RC48 false stop / QA 轮 T4 stall 高频 / archive C 未证明 / 切片 lost-to-LLM。
- 本轮两项修复：**压缩阈值优先级修正**（v0.2.18，真机 COMPACT_DBG 实证注入压垮 env 仪器）+ provenance 实修（.133 binary 对齐）。
- 零 STOP 触发；§3 禁区零触碰。

## 2. Current Architecture

见 `docs/core-freeze-review/architecture-map.md`（authority-matrix.md 的 12 层链表 + 15 项 authority 权项）。

## 3. Authority Matrix

15 权项逐项实测：**零双 authority / 零无 authority / 零越级**。唯一特殊结构 = give_up 的"决策权（planner）/否决权（loop 消费端三路拦截+RC47 路由）分离"——修 D 预裁决的有意设计，非冲突。

## 4. Decision-Terminal Map

见 `decision-terminal-map.md`。五组语义区分全部成立（Stop≠GiveUp 在 reason 层、Escalate≠Approval≠Stop、GiveUp≠Completion）；Escalate+delegation+无通道 = **结构化降级 Stop**（n12r2 approval_denied 实证，无隐式行为）。

## 5. False Stop Investigation（Node 03/04，最高优先级）

**RC48 根因锁定（三假设检验）**：
- 假设 A ✅（精确化）：criteria 非空 + Reserve 零和（早期核验失败耗尽唯一 Reserve）+ 后续真实修复完成 → 终止时无第二次核验机会（n12r1：VERIFICATION_RESERVE=1 + GIVE_UP_INTERCEPTED=1 + 独立复验 sortlib passed 但 failed）。
- n12r3（可比性复现）**决定性反证**：同任务同参数，GiveUp 时点有 Reserve → 拦截→核验**通过**→GIVE_UP_OVERRIDDEN→**completed 31 步**——机制在有核验机会时全链路正确。
- 假设 B ✗（非直接原因）；假设 C ✅ 贡献因子（planner 无完成事实感知——n11 QA 轮 T4 stall 同根）。
- **与 RC47 已修形态的分界 = criteria 空与非空**。修复（Reserve 分账 / acceptance-passed 消费扩展）**须顶层批准**——本轮登记不实现。层归：mechanism（有界设计代价）+ model（GiveUp 时点）。

## 6. History Slice Investigation（Node 05，批-4）

判级 = **lost-to-LLM（F1）**：context 消失 ✓ + state 存活 ✓ + 无 archive ✓ + 零恢复通道 ✓（批-1 预期证实）。`[history note]` 标记诚实但无恢复能力。最小改善二选一（入 archive / 标记文本修正）登记，冻结窗口不动。**"已知有界损失"而非"已保护"**。

## 7. Archive Recoverability（Node 06，批-5 防污染）

C-probe 三版迭代（v2 污染 / v3 污染（用户消息永驻保留轮）/ v4 setup 失败）——**C 未证明，机制在**（A：archive 落盘实证；B：提示在场 fixture 锁定）。防污染设计要点已写入 review-index 供后续重试。**不阻塞 Freeze**（F1，边界已声明）。

## 8. Fact / Verification Retention（Node 07）

9 类事实 × 三边界五档判级全表见 `fact-lifecycle.md`：state 层 7 类全 preserved（INV-M01 fixture）；旧轮原文 archived（≠recoverable）；slice 细节 lost-to-LLM（F1）；**Task Continuity 投影不受切片影响**（关键事实持续可见）。

## 9. Compact + Resume（Node 08）

chainfree 单跑成链：plan→write→test fail→repair→retest→**compact（env 7000，S-3）**→stop→resume→complete；re-teach=0、CHAIN_OK ✓、独立复验 1 passed。**P2-MC Deviation① 单跑闭环**（S-4）。

## 10/11. Long-run Product A/B（Node 09）

todoapi（多模块）completed + **1 passed** 独立复验；units（round-trip 拓扑）completed + **3 passed**。

## 12. Controlled Failure（Node 10）

Case A 常量错误拓扑 completed + **3 passed** 独立复验；Case B 环境适配拓扑 completed + ADAPTED_OK ✓。零盲重试（strategy change 实证）。

## 13. QA / Intent Boundary（Node 11）

16 轮：任务轮产物全达成（todo/done.txt 精确内容 ✓）；"继续"两形态区分实证（已完成形态暴露 RC48 第二形态=QA T4 stall 高频 12/16——**新 F1 如实登记**）；"查看状态"零 revision++；情绪+任务无切句复发；用户纠正无异常。

## 14. Reliability Matrix

| 能力 | mechanism | behavior | evidence | status |
|---|---|---|---|---|
| Intent | ✓ 规则分类 | ✓ QA/任务分流正确 | W8 fixtures+真机 | PASS |
| Plan | ✓ decompose+T4 有界 | ✓（QA stall=F1） | 真机 | PASS* |
| Execution | ✓ 五态结构化 | ✓ | 集成测试+真机 | PASS |
| Fact | ✓ state/archive 分层 | ✓ | INV-M01+真机 | PASS |
| Verification | ✓ O-4 唯一生产者 | ✓ | FA01+CFR fixtures | PASS |
| Failure Recovery | ✓ 分类+策略+有界 | ✓（RC48 边界） | n12 三跑对照 | PASS* |
| Completion | ✓ 双证据门 | ✓ 零假完成 | 真机 | PASS |
| Memory | ✓ archive 先行 | ✓（C 未证明=F1） | probe 三版 | PASS* |
| Resume | ✓ 持久恢复 | ✓ reteach=0 | 三例实证 | PASS |
| Decision | ✓ 拦截链+RC47 路由 | ✓（RC48 时点） | n12r1/r3 对照 | PASS* |
| Terminal | ✓ 九态封闭 | ✓ reason 层可区分 | 报告实测 | PASS |
| Projection | ✓ 事实投影 | ✓ | RC20 家族 | PASS |
| Security | ✓ RT4+approval | ✓ 拒绝实证 | n12r2 | PASS |

False 指标：false completion **0** / false giveup **0** / false replan **0** / **false stop 2+12**（RC48 两形态，有界）/ silent loss **0** / reteach **0** / duplicate execution：任务轮零（QA 轮重执行已披露）。

## 15. OPEN / UNKNOWN / DEFER

见 `known-deviations.md`：F0=0；F1×4（RC48 / QA T4 stall / C 未证明 / 切片 lost-to-LLM）；F2 DEFER×6。

## 16. Provenance

见 `provenance.md`：双 VM 0.2.18 三查一致（.133 滞后实修关闭）；gate `~/t_gate_cfr_final.log` 447/0；备份在案；.131 binary 含诊断 eprintln（披露，正式发布前移除）。

## 17. Security

RT4 fail-closed 全程；approval 结构化拒绝实证；egress 白名单无绕过；零 STOP 触发；§4 红线零触碰。

## 18. Independent Review Package

`docs/core-freeze-review/`：architecture-map（authority-matrix）/ decision-terminal-map / false-stop-lifecycle / history-slice-lifecycle / memory-lifecycle / fact-lifecycle / failure-lifecycle / long-run-evidence / known-deviations / provenance / **review-index.md**（grep+预期输出硬格式，批-7）。

## 19. Final Freeze Recommendation

**建议：CORE FREEZE READY WITH ACCEPTED DEVIATIONS。**

依据：F0=0 三包+本轮 10+ 场景实证；F1 四项全部"可解释、有界、已声明"（批-8 预判兑现）；五层验收中四层 ✅，Independent Review 层 = 本 review package 交付后由人工+外部 AI 双窗执行。残余 F1 均为 model/decision 层或有界机制边界，无一触及 Core 事实模型；两项修复方向（RC48 消费扩展 / planner 完成感知）已明确并**待顶层批准**——批准与否不改变 Freeze 边界，只改变 F1 数量。
