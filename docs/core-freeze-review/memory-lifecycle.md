# Memory / Compaction Lifecycle — CORE FREEZE REVIEW-01 Node 05/06/07 汇总

## 运行时压缩行为（v0.2.18 真机 COMPACT_DBG 实证）

1. **v0.2.17 运行时压缩静默失灵**（本轮最重要的横向发现）：provider-aware 注入
   （Agnes caps 128K → 阈值 195,840 字符）压过 env 测试仪器——`COMPACT_DBG
   est=68 thr=195840 len=2` 实锤。v0.2.18 修正优先级（env > injected > const）。
2. **v0.2.18 压缩恢复**（probe11 实证）：thr=1500 生效；est 73→2784→6234→9762
   跨越阈值；archive 增长（1220260→1225012 @07:01）。
3. **锯齿式压缩形态（机制注记，非缺陷）**：compaction merge 后历史折叠回单
   Turn（`history.push(Turn{messages: merged})`）→ len=1 → keep_from=0 门暂时
   关死 → 新交换累积至 len>2 后再次触发——**每次压缩间需 ≥2 次新交换**。
   正确性无损（每次触发都归档），效率注记登记。
4. **session 归档路由**：chat 绑定后压缩写 `archive/<sid>.jsonl`（n06v3 实证
   34KB 会话文件）；部分场景仍落共享 compacted.jsonl（绑定时序残余，OPEN-4 残余）。

## C-probe 三版迭代（批-5 防污染纪律的执行记录）

| 版本 | 设计 | 结果 | 判定 |
|---|---|---|---|
| v2（P2-LR 遗留） | 事实在用户消息+tool result | 答案正确 | **污染**（用户消息留在保留轮——探针缺陷） |
| v3（本包） | chat+resume；goal 不含事实 | 答案正确（VAULT-3341）；零 grep | **污染**（事实在保留轮的用户消息中——`enqueue_user_message` 永驻最近轮） |
| v4（本包） | 事实只经 tool result（cat 输出）+ 事后删文件 | **探针 setup 失败**（cat 时事实文件为空——事实未进入系统） | 无效（不计入判定） |

**最终判级（批-2 三档）**：**C 未证明，机制在**——"archive preserved but model
recoverability unproven"。两版探针均未达成"事实唯一载体=archive"的防污染前提
（v3 用户消息永驻 / v4 setup 失败）。C-probe 的正确设计要点已写入 review-index
（后续轮次可按此重试；不阻塞 Freeze，F1 级已知限制）。

## 切片路径（Node 05）

`[history note]` 标记已落（v0.2.17）；切片内容**零恢复通道**（不在 archive）——
lost-to-LLM F1 判级维持；"已知有界损失"表述（非"已保护"）。
