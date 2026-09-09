# 设计决策：经验系统是降级通道，不是常驻增强

> 决策日期：2026-08-01（v21 刀入鞘轮）
> 状态：**已定版**——wiring 断言 `experience-adaptive-switch` 强制执行本决策
> 证据链：v17 生长轮 / v18 精炼轮 / v19 深水轮 三轮正交实验

---

## 决策一句话

**经验注入只在"连续失败≥3 次"后启用（降级通道）；默认不注入。**

## 证据链

### 1. 弱模型（zhipu glm-4.5-air）：经验有效（+20pt）

v17 生长轮（`docs/self-evolution-proof-v17.md`）：

| 轮次 | 经验状态 | 通过率 |
|---|---|---|
| R1 | 空 store | 65.0% |
| R2 | 运行中自动积累 | 80.0%（+15pt） |
| R3 | 预注入 14 条失败经验 | 85.0%（+20pt） |

improved 9 条 / regressed 仅 1 条——经验精准命中"曾经挂掉的题"（T09/T16/T17/T18）。

### 2. 强模型（deepseek v4-flash）：经验有害（-5pt）

v18 精炼轮（`docs/self-evolution-v18-report.md`），20×2 随机序：

| 实验 | 配置 | 通过率 |
|---|---|---|
| E0 | baseline（空 store） | 87.5% |
| E1 | keyword + 规则经验 | 87.5%（+0） |
| E2 | keyword + LLM 精炼 | 85.0%（-2.5） |
| E3 | embedding + LLM 精炼 | 82.5%（-5.0） |

### 3. 机制解释

```
强模型本来就会做 → 注入泛泛经验 → 污染 prompt 干扰推理 → 负收益
弱模型不会做 → 经验给具体修复步骤 → 指明方向 → 正收益

经验系统的价值 = max(0, 任务难度 - 模型能力)
```

### 4. 结论落地

v19 深水轮实现了**自适应开关**（`loop.rs`）：

```rust
if self.consecutive_errors >= 3 {   // 连续失败才注入
    let matches = store.search(&goal_text, 2).await;
    // ...inject
}
```

wiring 断言 `experience-adaptive-switch` 锚定 `["consecutive_errors >= 3", "store.search"]`——**这段代码被改回无条件注入 → 断言红 → 阻断**。

## 为什么不删除整个经验系统？

1. 经验对弱模型（zhipu 65%→85%）仍有 +20pt 价值——降级通道保留
2. 记忆系统的存在本身是长期能力（自我进化闭环），只是不常驻
3. 用户可通过 provider 选择（zhipu）让经验自然生效

## 维护须知

- **不要**把经验注入改成无条件（每次 do_plan 都 search）——这会重蹈 v18 E1-E3 的覆辙
- **不要**再投入 embedding 语义检索（v18 E3 实测 82.5% < 85.0% keyword，零增量）
- **不要**再投入 LLM 经验精炼（v18 E2 实测 85.0% < 87.5% 规则构造，零增量）
- 若某天标准脑换成弱模型，本决策需要重评（开关阈值、注入策略）
