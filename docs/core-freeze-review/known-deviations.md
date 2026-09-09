# Known Deviations — CORE FREEZE REVIEW-01（F0/F1/F2 最终分级）

## F0 — Freeze Blocker：**0 项**

10+ 场景真机实证：零假完成 / 零静默事实丢失（切片已打标记且 Task Continuity 投影持续可见）/ 零 resume 重教育 / 零无限循环（T4+Reserve+replan cap 三重有界）/ 零沙箱审批绕过（n12r2 拒绝实证）/ 零 Fact→Projection 失真（RC20 家族纪律）/ 零 provenance 错位（Node 00 三查一致）。

## F1 — High Risk（已明确边界、有复现证据、有处置方向）：**4 项**

| # | 项 | 边界与证据 | 处置方向 |
|---|---|---|---|
| 1 | **RC48 false stop**（criteria 非空 + Reserve 零和 × GiveUp 时点；n12r1 实证，n12r3 反证机制在有 Reserve 时正确） | 有界（Reserve ≤1 + T4 + replan cap）；独立复验产物有效 | Reserve 分账或 acceptance-passed 消费扩展——**须顶层批准** |
| 2 | **QA 轮 T4 2-node stall 高频**（planner 无完成事实感知；n11 12/16 轮） | 有界（T4 bounded stop，零假完成方向）；任务轮产物全部达成 | planner 完成事实感知注入——**须顶层批准** |
| 3 | **Archive C 未证明**（"archive preserved but model recoverability unproven"）——两版探针未达成防污染前提（v3 用户消息污染 / v4 setup 失败） | A+B 证明；C 探针设计要点已写入 review-index | 按批-5 设计要点重试（不阻塞） |
| 4 | **切片内容 lost-to-LLM**（40 条外事实零恢复通道；note 标记诚实但无恢复能力） | 有界（state 层未销毁 + Task Continuity 持续投影关键事实） | 切片入 archive 或标记文本修正（二选一，均小改） |

## F2 — Non-blocking DEFER

constitution 6000 标记 / interaction 8 类样本真机补全 / archive 存截后工具输出（B 档上限钳制）/ chat 压缩归档 session 路由残余 / 锯齿式压缩效率注记 / RC45 双上限设计 / resume banner UUID 截断。

## Hard Failure Criteria 复核（§22 逐条）

| 判据 | 结果 |
|---|---|
| 假完成 | **零**（10+ 场景；give_up 拦截/路由全部有证据链） |
| 历史事实被静默丢失 | **零**（compact=archive 先行；slice=标记+state 存活+Continuity 投影） |
| resume 需重新教育关键任务 | **零**（re-teach=0 三例实证） |
| 同一失败无限循环 | **零**（三重有界） |
| 错误输出被投影为成功 | **零**（五态接线 + RC20 家族） |
| sandbox/approval 被绕过 | **零**（n12r2 拒绝实证） |
