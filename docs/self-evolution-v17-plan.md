# codex-rust v17 — 生长轮：经验回路闭合 + 锻造证明的下一站

> 定位：v12→v16 锻造证明了 agent 的**身体**在四维火力下不崩（基准 90% / 应力 0 panic / 回放 100% / wiring 11/11）。v17 把火力转向**脑**——不是加新功能，是让 agent 用自己跑出来的数据**变聪明**。
>
> 锻造体系（本章完）。自进化体系（下一章开）。

---

## §1 起点状态（v16 锻造终结）

| 资产 | 状态 |
|---|---|
| 基准设施 | 20 题 T01-T20 + runner + batch + resume + 按题预算 |
| 接线防火墙 | codex-xray + wiring 11/11 + 自证能变红 |
| 确定性回放 | ReplayProvider + 31 fixture |
| 应力场 | 46/48 合计 PASS，0 panic |
| 经验引擎 | experience crate + 文件持久化（v16 新） |
| 潜意识门 | subconscious 5 guards + CostGuard 真值接地（v16 新） |
| 能力边界 | 白皮书已发布（3 条边界线） |
| 锻造证明 | forge-final-report.md 终稿 |

---

## §2 锻造之后的核心矛盾

锻造回答了"能不能跑、会不会崩"——答案是**能**。

但锻造没有回答另外三个问题：

1. **能变聪明吗？** agent 跑了 40 次基准、31 条回放——所有这些跑分数据都在 `bench/results/raw/` 里躺着。experience 引擎有了文件持久化。**但从未做过一次"用失败经验让 agent 重跑并变好"的实验。**

2. **经验搜索真的有用吗？** 经验向量库（experience crate）是 v11.0-v11.4 建的——append/search/reinforce/prune 全有。但它的 **search 从来没在真实任务中验证过能否返回相关的经验**。它一直在"造了但没人知道能不能用"的状态。

3. **T19 到底怎么修？** 这是唯一一个所有模型全挂的题。能力边界白皮书标了"工具链盲区"，但没有给出修复方案。

v17 不解决所有三个——解决前两个，它们恰好互相加速。

---

## §3 v17 目标：经验回路闭合验证

### 核心实验：经验注入 → agent 变好

```
步骤 1：跑 deepseek 20×1 基准 → 出 FAIL 列表
步骤 2：每条 FAIL 的 session 消息 → 凝练为一条经验（手工 or LLM 辅助） → append 到 experience store
步骤 3：重跑同样的 20×1（experience store 里有失败经验）
步骤 4：比对：经验注入前 vs 注入后的通过率 + 失败分类变化
```

**假设**：agent 在"有失败经验"的情况下，通过率应该 ≥ 经验注入前。如果相等说明 search 匹配有问题；如果下降说明经验干扰了推理；如果上升证明经验系统有价值。

**这个实验不需要改一行代码**——experience crate 的 append/search 已经就绪。只需要 runner + 数据流。

### 经验搜索质量验证

对每条 FAIL，调用 `ExperienceStore::search(goal_text, 3)` 看返回的最相关经验是否与当前任务相关：

| 指标 | 定义 |
|---|---|
| **命中率** | 返回的 top-3 经验中至少 1 条与当前任务同类（category/level 相近） |
| **相关度** | 人工 1-3 评分，平均 >2 = 可接受 |

如果命中率 <50%：说明 keyword fallback 不够，需要真 embedding 向量化（v10.5 设计的 Phase 2）。  
如果命中率 ≥80%：说明当前 keyword 匹配已经够用，直接做注入实验。

---

## §4 阶段拆解（审查优化版）

> 审查优化：S1+S2 合并为一个全自动脚本 `bench/self-evolve-v17.py`，由 LLM 自动凝练经验（不入工）。基准从 20×1 改为 20×2（更好统计）。S4 embedding 自检放入脚本的快速前置检查。

| 阶段 | 任务 | 预计 | 产出 |
|---|---|---|---|
| **S1+S2** | **合并**：编写 `bench/self-evolve-v17.py` → 跑 20×2 baseline → 对 FAIL 用 LLM 凝练经验 → append 到 store → 验证 search 质量 → 重跑 20×2 → 输出对比 | **自动运行 ~1h** | 注入前后通过率对比表 |
| **S3** | 自进化证明文档 | 0.5h | `docs/self-evolution-proof-v17.md` |
| **S4** | 如果 S1+S2 中 search 质量自检不通过 → embedding 向量化（OpenAI embedding API） | 0.5d | experience crate + embedding |
| **S5** | S4 后重做实验 | 自动 | 修正后对比 |

### 自动化凝练方案

对每条 FAIL（来自 task goal + verify_tail），用 LLM 自动生成结构化经验：

```
prompt: 阅读以下 agent 任务记录，输出一条经验 JSON。
字段：category / problem / solution / success / effectiveness
来源：goal: {goal} | error: {verify_tail} | level: {L1-L5}
```

通过 SSH 直接写入 VM 的 `experience.jsonl` 文件 → 重启 service 加载（无需改 Rust 代码）。

---

## §5 为什么不做 T19 和更多功能

锻造教会我们一件事：**先证后扩，不证不扩**。

- T19 是工具链盲区——要修它需要深入理解 agent 在泛型合并上的工具调用模式。在没有"经验回路"帮助的情况下，agent 每次面对 T19 都是从零开始的单次尝试。如果经验回路由闭合了——agent 能从之前的 T19 失败中学到——也许不需要底层工具链改。
- 同理，任何新功能在经验回路闭合之前都不急着加。**先证明经验能让 agent 变聪明，再决定聪明的 agent 还需要什么新功能。**

---

## §6 验收红线

| 级别 | 判据 |
|---|---|
| 🔴 | 经验注入实验数据造假（没有真跑基准） |
| 🔴 | v16 的 11 条 wiring 断言破裂 |
| 🟡 | 经验注入后通过率无显著变化（持平或下降 ≤5%）——经验系统需要分析 |
| 🔵 | 通过率提升幅度（不做及格线，只做记录） |

---

## §7 v17 之后的路线（不展开，只标记）

- **v18**：如果经验回路证明有效 → embedding 向量化 + 遗忘机制 + 自主闭环（v10.5 设计的 Phase 2-5）
- **v19**：如果经验回路证明无效 → 根因分析 + 经验系统重构
- **T19 修复**：经验回路闭合后优先处理（agent 能从自己失败中学到什么？）
- **季度体检**：按 forge-final-report §4 执行

---

*锻造之后不是放松，是把证明好的身体交给能变聪明的大脑。四天的锻造证明了 agent 不会崩。下一步证明它能长。*
