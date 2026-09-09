# History Slice Lifecycle — CORE FREEZE REVIEW-01 Node 05（批-4 判级）

## 实验构造（源码实测 + P2-LR 既有证据 + marker 测试）

构造路径：build_messages（loop.rs:2203-2205）`MAX_HISTORY_MSGS=40` → 超出即 split_off 保留最近 40 条 → 去孤儿 Tool 消息 → **注入 `[history note]` System 标记（P2-LR 已落）**。

判级（对"早期插入的 fact/约束/验证结果"）：

| 检查项 | 结果 | 依据 |
|---|---|---|
| 1. 是否从 context 消失 | **是**（LLM 不可见） | 切片后 prompt 仅含最近 40 条 |
| 2. 是否持久化 | **是（state 层）** | 消息仍在 state.history（未销毁） |
| 3. 是否 archive | **否** | 切片不落 archive（archive 只收压缩轮次） |
| 4. 是否可自动恢复 | **否** | 无标记→模型可被告知被裁（note），但**无工具/通道取回内容** |
| 5. 模型是否需要主动知道去 grep | **即使知道也无效** | **archive 里没有切片内容——B 档救不回（批-1 预期证实）** |

## 判级：**lost-to-LLM（F1）**

- 早期事实：context 消失 + state 存活 + 无 archive + **零恢复通道**。
- `[history note]` 标记文本诚实（"原始记录仍完整保留于会话状态"）**但无恢复能力**——模型既无工具也无检索通道；且"完整保留"不等于"可检索"。
- 对比 compaction 路径：archive 先行 + 检索提示 + B 档通道（模型 0 使用，另一 OPEN）——**保护不对称在切片侧更深一层**。

## 最小改善方向登记（二选一，冻结窗口内均不动）

1. **切片内容入 archive**（复用既有通道，零新架构）——涉及 build_messages 切片时同步落盘；
2. **标记文本诚实性修正**（零代码）：note 改为"该内容已不可检索"——避免误导模型以为可找回。

本轮登记为 **F1 "已知有界损失"**（非"已保护"）——review-index 列为委托方需知限制。
