# Fact / Verification Survival — CORE FREEZE REVIEW-01 Node 07

> 三条边界（Compact / History Slice / Resume）× 9 类事实五档判级（preserved/archived/recoverable/lost/unknown）。
> 证据：INV-M01 fixtures（compact）+ slice marker 测试（slice）+ chainlib 真机链（resume）。

| 事实/验证类 | Compact 后 | History Slice 后 | Resume 后 |
|---|---|---|---|
| original_goal（immutable 锚） | **preserved**（state 层，fixture 锁定） | **preserved**（state 层） | **preserved**（持久层恢复） |
| constraints（R2-D） | **preserved**（state） | **preserved** | **preserved** |
| acceptance_criteria | **preserved**（state） | **preserved** | **preserved** |
| tool success（五态 ToolResult） | 旧轮 → **archived**（原文在 archive JSONL）；近 2 轮 preserved | 40 条外 → **lost-to-LLM**（无 archive） | **preserved**（历史随会话持久化） |
| artifact（written_files/磁盘文件） | **preserved**（磁盘事实，非上下文） | **preserved** | **preserved** |
| failure（error_kind/失败分类） | 旧轮 → **archived**；scratch 的 last_failure_class **preserved** | 同 tool success | **preserved** |
| verification（acceptance_result） | **preserved**（scratch，不在压缩路径） | **preserved**（scratch） | **preserved** |
| 验证文本细节（tool_result 原文） | 旧轮 → **archived**（**≠recoverable**：grep 通道模型 0 使用） | 40 条外 → **lost-to-LLM** | **preserved** |
| acceptance_result 明细（failures 列表） | **preserved**（scratch JSON） | **preserved** | **preserved** |

## 关键区分（总包 §13 纪律）

- **archived ≠ recoverable**：archive 有原文（A）+ 提示在场（B）≠ 模型取回（C 未证明，Node 06）。
- **preserved（state）≠ LLM 可见（slice）**：40 条外的 state 事实对 LLM 不可见（lost-to-LLM）——但 Task Continuity 投影（goal/completed/next_action/acceptance_result）每轮注入，**关键事实经此通道持续可见**（不受切片影响——切片只切 history，不切 state 投影）。
- 唯一 **lost** 级：slice 路径的 history 内嵌事实细节（Node 05 F1 登记）。

## Resume 专项（Node 08 chainlib 链 + Node 09 pending）

resume 后 original_goal/constraints/acceptance_criteria/scratch 全部随持久 state 恢复（INV-M01 边界在 resume 侧成立），re-teach=0（P2-LR n09b + 本轮 Node 08 双证）。
