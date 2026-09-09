# Freeze Decision — CORE FREEZE REVIEW-01 / CLOSURE-01

**日期**：2026-08-31　**基线**：v0.2.18（tag，HEAD `5b0c7b9`，干净工作树）　**gate**：447/0 四 RC=0（`.133:~/t_gate_closure.log`）

# 结论

# **CORE FREEZE**

（依据见下；F1 全部 ACCEPTED DEVIATION，零 BLOCKER；本文件为唯一最终结论，无模糊状态）

## 判定依据（Node 13 九条 BLOCKER 规则逐条核销）

| # | BLOCKER 条件 | 结果 |
|---|---|---|
| 1 | F0 | **F0 = 0**（六项四层复核，F0-recheck.md） |
| 2 | 第二事实 authority | **无**（Authority Matrix 15 权项零双权威） |
| 3 | Verification authority 冲突 | **无**（O-4 唯一生产者） |
| 4 | Completion 无证据触发 | **无**（双证据门+零假完成实证） |
| 5 | Resume 重教育/重复执行 | **无**（reteach=0 三例） |
| 6 | Sandbox/Approval 绕过 | **无**（结构化拒绝实证） |
| 7 | Memory 未声明静默丢失 | **无**（切片=bounded declared loss-to-LLM，非 silent） |
| 8 | Terminal 不可解释越级 | **无**（九态封闭+reason 层可区分） |
| 9 | Provenance 无法建立 | **无**（三方一致+RC49 重跑闭环） |

## What Hearth Core guarantees

- 工具执行结果结构化五态（exit/signal/timeout/deadline）投影，零文本解析。
- 完成判定 = 盲区C 产物校验 + acceptance（有 criteria 时）双证据；LLM self-report 不构成完成。
- 失败分类 = classify_failure 纯函数（结构化输入）；放弃必须携带核验证据或 giveup_unverified 标记。
- 压缩 = archive 先行（落盘实证）；state 层事实跨压缩/切片/resume 存活（INV-M01 fixtures）。
- Resume = 持久恢复，re-teach = 0（三例实证）。
- 沙箱 RT4 fail-closed + approval 结构化拒绝，不可绕过。

## What Hearth Core does NOT guarantee（委托方需知）

- **archive 模型恢复未证明**：archive 落盘已证明（A）；模型经 grep 主动恢复旧事实**未证明**（C=UNKNOWN，两版探针污染+一版 setup 失败）——不要把 archive 当作"模型可自动找回的记忆"。
- **40 切片 lost-to-LLM**：超 40 条消息的事实细节对 LLM 永久不可见（state 存活、零恢复通道）；Task Continuity 投影缓解关键事实但不恢复细节。
- **RC48 false stop 边界**：criteria 非空 + Reserve 零和 + GiveUp 时点不利时，已完成任务可能被终止为 failed（独立复验产物有效）；约 2/14 复发率（同族样本）。
- **Agnes model variance**：同参数跑间步数/收敛方差大（14-76 步）；provider 层不可控。
- **QA 轮 T4 stall**：已完成"继续"形态可能触发 2-node 同图停滞（有界，零假完成）。

## Future independent RFC（冻结后独立单，不污染 Core）

1. **RC48-FOLLOWUP**：Reserve 分账（RC45 预案）或 acceptance-passed 消费扩展（loop 消费端，不动 planner schema）。
2. **QA completion-awareness RFC**（如做）：只读投影、不得用于 GiveUp 判定——否则 authority duplication。
3. **切片内容入 archive**（复用既有通道，小改）或标记文本诚实性修正。
4. **C-probe 干净重试**（按批-5 设计要点：事实唯一载体=被压缩轮次）。

## 附：本轮纪律核销

- 生产代码改动 = **0**（no-code gate ✓，v0.2.18 保持，无人为 bump）。
- §0.2 四项（RC48/QA awareness/C-probe/40 切片改造）**零施工** ✓。
- 四项 F1 disposition 全部落表（无 OPEN 作裁决）✓。
