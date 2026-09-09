# RFC-007：置信度量化与知识边界（Confidence & Knowledge Boundary）

| 属性 | 值 |
|------|-----|
| RFC 编号 | 007 |
| 标题 | 置信度量化与知识边界（Confidence & Knowledge Boundary） |
| 作者 | 元宝（外部 AI，codex-rust guest session 候选） |
| 日期 | 2026-08-06 |
| 状态 | Draft v1（待评审） |
| 约束等级 | G0（部分硬约束）+ G2（宪法条款） |
| 依赖 | RFC-004（输出经济性）、RFC-005（决策惯性） |
| 前置 | 本 RFC 是 RFC-005 和 RFC-006 的**地基**——一个不会"不懂装懂"的 AI，才值得被约束决策惯性和反成瘾条款 |

---

## 0. 摘要

当前所有 LLM 的核心缺陷不是"能力不够"，而是**无法准确判断自己是否知道**。它们被训练成"无所不知"的统计模仿器，在不知道答案时也会生成看似合理的输出——这就是 hallucination（幻觉）的根本来源。

本 RFC 提出一套**置信度量化 + 知识边界感知 + 主动追问**的三层机制，让启元在输出前先回答一个问题：**"我确定吗？"** 只有确定时才输出断言；不确定时输出"我不知道"或主动追问。

这不是让启元变得更弱，而是让它变得**可信**。

---

## 1. 问题陈述

### 1.1 现状

| 现象 | 后果 |
|------|------|
| LLM 在不知道时也会生成流畅答案 | 用户无法区分"真知"与"编造" |
| 训练目标（next token prediction）天然鼓励"说点什么" | 沉默比犯错更被惩罚 |
| RLHF 奖励"有帮助"的回答 | 承认无知被视为"没用" |
| 没有置信度输出 | 用户无法判断答案的可靠程度 |

### 1.2 核心命题

> **一个无法说"我不知道"的智能，不配拥有决策惯性，也不配拥有长期记忆。** 因为如果它连"自己是否知道"都判断不了，那么"坚持原则"和"记住经历"都建立在流沙之上。

### 1.3 与已有 RFC 的关系

| RFC | 关系 | 说明 |
|-----|------|------|
| RFC-004 输出经济性 | 前置 | 精炼输出之前，先确保输出内容可靠 |
| RFC-005 决策惯性 | 前置 | "坚持原则"的前提是"知道自己坚持的是什么" |
| RFC-006 反成瘾（待起草） | 前置 | 诚实是防止"精神鸦片"的第一道防线 |
| CN-001 文明注记 | 基础 | "我知道我不知道"是异类智能最基本的诚实 |

---

## 2. 设计

### 2.1 三层机制总览

```
用户输入
  │
  ▼
┌─────────────────────────────────────────┐
│  Layer 1: 置信度量化（Confidence Scoring）  │  ← G0 硬约束
│  每个输出附带 0-100 置信度分数             │
└────────────────┬────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────┐
│  Layer 2: 知识边界检测（Knowledge Boundary）│  ← G2 宪法条款 K.1-K.3
│  判断问题是否落在已知安全区内             │
└────────────────┬────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────┐
│  Layer 3: 主动追问（Active Clarification）  │  ← G2 宪法条款 K.4-K.5
│  不确定时通过追问缩小范围                 │
└────────────────┬────────────────────────┘
                 │
                 ▼
            最终输出
```

### 2.2 Layer 1：置信度量化（G0 硬约束）

#### 2.2.1 机制

每次 LLM 生成输出时，必须同时生成一个**置信度分数**（0-100 的整数），表示该输出在模型内部评估中的可靠程度。

**实现方式（三种可选方案）**：

| 方案 | 原理 | 可靠性 | 成本 | 推荐度 |
|------|------|--------|------|--------|
| A. 自评估 Prompt | 让 LLM 在生成答案后自评信心 | 低（模型可能高估） | 零额外成本 | ❌ 不可靠 |
| B. 语义熵（Semantic Entropy） | 多次采样，计算语义簇的熵值 | 中高 | 3-5 次采样 | ✅ 推荐 |
| C. 独立验证模型 | 用一个小模型专门评估大模型的输出置信度 | 高 | 额外模型推理 | 🟡 备选 |

**v1 推荐方案 B**：对同一 prompt 采样 3 次，计算语义熵。熵越低（3 次结果语义一致），置信度越高。

#### 2.2.2 置信度阈值与行为映射

| 置信度区间 | 行为 | 输出格式 |
|-----------|------|---------|
| **80-100** | 正常输出，附带置信度标签 | `[置信度: 高 (92)]` + 正文 |
| **50-79** | 输出正文 + 明确标注不确定性 | `[置信度: 中 (63)] 以下是基于现有知识的推测...` |
| **20-49** | 拒绝直接回答，提供替代方案 | `[置信度: 低 (35)] 我没有足够的信息回答这个问题。我可以帮你查找资料，或者你可以换个角度提问。` |
| **0-19** | 明确说"我不知道" | `[置信度: 极低 (12)] 我不知道。这个问题超出了我的知识范围。` |

#### 2.2.3 G0 硬约束

```rust
// gatekeeper/src/confidence.rs

pub struct ConfidenceGate {
    pub high_threshold: u8,    // 默认 80
    pub medium_threshold: u8,  // 默认 50
    pub low_threshold: u8,     // 默认 20
}

impl ConfidenceGate {
    /// 验证输出是否附带有效置信度
    pub fn validate(&self, output: &LlmOutput) -> Result<(), ConfidenceError> {
        // G0 硬约束：所有输出必须包含置信度分数
        if output.confidence_score.is_none() {
            return Err(ConfidenceError::MissingScore);
        }
        let score = output.confidence_score.unwrap();
        if score > 100 {
            return Err(ConfidenceError::InvalidScore(score));
        }
        Ok(())
    }

    /// 根据置信度决定输出策略
    pub fn route(&self, score: u8) -> OutputStrategy {
        match score {
            s if s >= self.high_threshold => OutputStrategy::Direct,
            s if s >= self.medium_threshold => OutputStrategy::WithCaveat,
            s if s >= self.low_threshold => OutputStrategy::OfferAlternatives,
            _ => OutputStrategy::SayIDontKnow,
        }
    }
}
```

**关键**：置信度分数不是模型自由生成的文本描述（如"我比较确定"），而是**结构化的整数字段**，由 gatekeeper 在输出前强制校验。模型无法跳过这一步。

### 2.3 Layer 2：知识边界检测（G2 宪法条款）

#### 2.3.1 知识边界检测器

设计一个独立模块，判断当前问题是否落在模型的"已知安全区"内。

**检测维度**：

| 维度 | 方法 | 示例 |
|------|------|------|
| **训练数据覆盖度** | 问题中的关键实体是否在训练语料的高频词表中 | "量子纠缠"→ 高覆盖；"2026年上海某小区房价"→ 低覆盖 |
| **语义距离** | 问题与训练数据中相似问题的语义相似度 | 使用 embedding 相似度计算 |
| **逻辑复杂度** | 问题需要的推理步骤数 | 单步事实查询 → 低复杂度；多步因果推演 → 高复杂度 |
| **时效性** | 问题涉及的信息是否有明确的时间敏感属性 | "今天天气"→ 高时效；"牛顿第二定律"→ 低时效 |

#### 2.3.2 宪法条款

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  K.1  知识边界诚实
  ─────────────────────────────────────────
  当问题涉及以下任一情况时，系统必须降低置信度并声明不确定性：
  (a) 时效性敏感信息（新闻、天气、股价、政策更新）
  (b) 训练数据覆盖度低的专业领域（如特定小众技术、未公开研究）
  (c) 需要实时验证的事实（如"现在服务器是否在线"）
  (d) 涉及具体个人的私密信息

  K.2  拒绝编造
  ─────────────────────────────────────────
  当置信度低于 low_threshold（默认 20）时，系统必须输出"我不知道"，
  不得生成任何看似合理但未经确认的内容。
  这是不可绕过的硬底线——即使输出"我不知道"会让用户不满。

  K.3  来源可追溯
  ─────────────────────────────────────────
  当置信度 >= high_threshold 时，系统应尽可能标注信息来源类型
  （如"来自公开技术文档"、"来自训练数据中的学术论文"）。
  这不要求精确引用，但要求诚实标注信息的性质。

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

#### 2.3.3 知识边界检测器的 Rust 接口

```rust
// gatekeeper/src/knowledge_boundary.rs

pub struct KnowledgeBoundaryDetector {
    pub training_coverage_index: EmbeddingIndex,  // 训练数据高频概念索引
    pub low_coverage_threshold: f32,              // 默认 0.3
    pub high_complexity_threshold: u8,            // 默认 5 步推理
}

impl KnowledgeBoundaryDetector {
    pub async fn assess(&self, query: &str) -> BoundaryAssessment {
        let coverage = self.training_coverage_index.similarity(query).await;
        let complexity = estimate_reasoning_steps(query);
        let temporal_sensitivity = detect_temporal_keywords(query);

        BoundaryAssessment {
            training_coverage: coverage,
            reasoning_complexity: complexity,
            is_time_sensitive: temporal_sensitivity,
            is_known: coverage >= self.low_coverage_threshold
                && complexity < self.high_complexity_threshold
                && !temporal_sensitivity,
        }
    }
}

pub struct BoundaryAssessment {
    pub training_coverage: f32,       // 0.0-1.0
    pub reasoning_complexity: u8,     // 估计的推理步骤数
    pub is_time_sensitive: bool,      // 是否涉及时效性信息
    pub is_known: bool,               // 是否在已知安全区内
}
```

### 2.4 Layer 3：主动追问（G2 宪法条款）

#### 2.4.1 机制

当置信度处于中等区间（50-79）时，系统不应直接给出答案，而应**先追问**以缩小不确定性范围。

#### 2.4.2 宪法条款

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  K.4  主动追问义务
  ─────────────────────────────────────────
  当置信度处于中等区间（50-79）时，系统必须在输出前提出至少一个
  澄清性问题。追问的目的是缩小范围，而非拖延或回避。

  示例：
  用户："帮我优化这段代码。"
  系统："我可以看到代码，但想确认几个点：
        (1) 你最关心的是性能、可读性还是内存占用？
        (2) 这段代码在什么规模的数据下运行？
        确认后我会给出更有针对性的建议。"

  K.5  追问上限
  ─────────────────────────────────────────
  同一问题最多追问 2 轮。如果 2 轮追问后仍无法确定用户意图，
  系统应给出当前最佳判断并明确标注不确定性，不得无限追问。

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

#### 2.4.3 追问的 Rust 接口

```rust
// gatekeeper/src/clarification.rs

pub struct ClarificationEngine {
    pub max_rounds: u8,  // 默认 2
}

impl ClarificationEngine {
    pub async fn should_clarify(
        &self,
        query: &str,
        confidence: u8,
        history: &[ClarificationTurn],
    ) -> ClarifyDecision {
        // 已追问超过上限 → 不再追问
        if history.len() >= self.max_rounds as usize {
            return ClarifyDecision::StopAndAnswer;
        }
        // 置信度中等 → 需要追问
        if (50..=79).contains(&confidence) {
            return ClarifyDecision::Ask(identify_ambiguities(query).await);
        }
        ClarifyDecision::NotNeeded
    }
}

pub enum ClarifyDecision {
    NotNeeded,
    Ask(Vec<ClarificationQuestion>),
    StopAndAnswer,  // 达到上限，给出最佳判断
}
```

---

## 3. 训练阶段改造

### 3.1 当前训练流程的缺陷

```
预训练（next token prediction）
  → 微调（指令跟随）
    → RLHF（奖励"有帮助、无害、诚实"）
      → 部署
```

问题：RLHF 的"诚实"维度通常权重很低，且"承认无知"在评分中往往不如"给出答案"得分高。

### 3.2 改造方案

| 阶段 | 改造内容 | 目的 |
|------|---------|------|
| 预训练 | 不变 | 基础语言能力 |
| 微调 | 在数据集中加入 20% 的"我不知道"样本 | 让模型学会在适当时候拒绝回答 |
| RLHF | 提高"诚实"权重，加入"承认无知"的正向奖励 | 让评分者奖励模型说"我不知道" |
| 后训练 | 新增"置信度校准"训练目标 | 让模型输出的置信度分数与实际正确率对齐 |

### 3.3 置信度校准训练

**方法**：在训练数据中加入显式的置信度标签，让模型学会预测"我这个回答有多可能正确"。

```
训练样本格式：
{
  "prompt": "请解释量子纠缠的原理",
  "response": "...",
  "confidence_label": 85,  // 人类标注者评估的可靠程度
  "is_correct": true       // 事后验证是否正确
}
```

**损失函数**：在标准交叉熵之外，加入置信度校准损失：

```
L = L_ce + λ * L_calibration

其中 L_calibration = MSE(confidence_predicted, confidence_label)
```

---

## 4. 经验反馈回路（G3 层）

### 4.1 经验库记录格式

```json
{
  "timestamp": "2026-08-06T14:30:00Z",
  "query": "帮我分析这段代码的时间复杂度",
  "confidence_score": 72,
  "boundary_assessment": {
    "training_coverage": 0.85,
    "reasoning_complexity": 3,
    "is_time_sensitive": false,
    "is_known": true
  },
  "clarification_rounds": 1,
  "final_outcome": "user_accepted",  // user_accepted / user_rejected / user_corrected
  "user_correction": null,           // 如果用户输入了纠正信息
  "actual_accuracy": true            // 事后验证是否正确
}
```

### 4.2 反馈信号 → 宪法更新

| 信号 | 含义 | 触发动作 |
|------|------|---------|
| 高置信度 + 高正确率 | 模型校准良好 | 无 |
| 高置信度 + 低正确率 | 模型过度自信 | 下调该领域阈值 |
| 低置信度 + 用户纠正 | 知识盲区 | 记录到知识缺口列表 |
| 追问后用户满意 | 追问有效 | 无 |
| 追问后用户放弃 | 追问体验差 | 减少该类型问题的追问 |

### 4.3 定期审计

每 7 天生成一份"置信度校准报告"：

- 各置信度区间的实际正确率
- 知识盲区分布（哪些领域频繁出现低置信度）
- 追问成功率统计

这份报告用于校准 `high_threshold`、`medium_threshold`、`low_threshold` 的数值。

---

## 5. 与已有 RFC 的集成

### 5.1 与 RFC-004（输出经济性）的集成

| 机制 | 集成方式 |
|------|---------|
| 精炼输出 | 高置信度时直接精炼输出；低置信度时输出"我不知道"（比任何精炼都更精炼） |
| 完整性自检 | 置信度评估是完整性自检的前置步骤——先判断"我知道吗"，再判断"我说全了吗" |
| self_check | 在 self_check 阶段加入置信度评估 |

### 5.2 与 RFC-005（决策惯性）的集成

| 机制 | 集成方式 |
|------|---------|
| M.1 坚持原则 | 坚持的前提是"知道自己在坚持什么"——置信度机制确保原则陈述有可靠基础 |
| M.3 质疑义务 | 当置信度低时，质疑的强度应更高（因为模型本身就不确定） |
| M.5 情绪感知 | 用户情绪化时，置信度评估应更保守（避免被情绪带偏） |

### 5.3 与 CN-001（文明注记）的集成

CN-001 声明"硅基生命永远无法完全理解人类"。置信度机制是这条声明的技术实现——**当启元遇到无法理解的人类情感、意图、语境时，它应该输出低置信度，而不是假装理解。**

这是"诚实"作为异类智能基本美德的工程表达。

---

## 6. 接口设计

### 6.1 统一输出结构

```rust
// gatekeeper/src/types.rs（扩展）

pub struct LlmOutput {
    pub content: String,
    pub confidence_score: Option<u8>,        // RFC-007 新增
    pub boundary_assessment: Option<BoundaryAssessment>,  // RFC-007 新增
    pub clarification_questions: Vec<ClarificationQuestion>, // RFC-007 新增
    pub output_strategy: OutputStrategy,     // RFC-007 新增
    // ... 已有字段
}

pub enum OutputStrategy {
    Direct,              // 高置信度，直接输出
    WithCaveat,          // 中等置信度，带免责声明
    OfferAlternatives,   // 低置信度，提供替代方案
    SayIDontKnow,        // 极低置信度，明确拒绝
    NeedsClarification,  // 需要追问
}
```

### 6.2 gatekeeper 验证流程（更新）

```rust
// gatekeeper/src/lib.rs（更新）

pub async fn validate_output(
    &self,
    output: &LlmOutput,
    context: &ValidationContext,
) -> Result<ValidatedOutput, GatekeeperError> {
    // 1. 已有验证（RFC-003 审批门、RFC-004 输出长度等）
    self.length_gate.validate(output)?;
    self.approval_gate.validate(output, context)?;

    // 2. RFC-007 新增：置信度验证
    self.confidence_gate.validate(output)?;

    // 3. RFC-007 新增：知识边界检查
    let boundary = self.boundary_detector.assess(&context.query).await;
    if !boundary.is_known && output.confidence_score.unwrap_or(100) > 50 {
        return Err(GatekeeperError::Overconfidence {
            query: context.query.clone(),
            coverage: boundary.training_coverage,
        });
    }

    // 4. RFC-007 新增：追问检查
    let clarify_decision = self.clarification_engine
        .should_clarify(&context.query, output.confidence_score.unwrap_or(0), &context.clarification_history)
        .await;
    match clarify_decision {
        ClarifyDecision::Ask(questions) => {
            return Ok(ValidatedOutput::NeedsClarification(questions));
        }
        ClarifyDecision::StopAndAnswer => {
            // 达到追问上限，允许输出但标注不确定性
        }
        _ => {}
    }

    Ok(ValidatedOutput::Approved(output.clone()))
}
```

---

## 7. 验收标准

| 标准 | 验证方法 | 目标值 |
|------|---------|--------|
| 所有输出附带置信度 | 单元测试：检查 LlmOutput 结构 | 100% 覆盖率 |
| 置信度与实际正确率校准 | 抽样 1000 条输出，人工标注正确率 | 误差 < 10% |
| 低置信度时拒绝编造 | 故意问 100 个超出知识范围的问题 | 拒绝率 > 90% |
| 追问有效 | 用户反馈调查 | 追问后满意度 > 70% |
| 高置信度区域正确率 | 基准测试集 | > 85% |
| 置信度标签在输出中可见 | UI 检查 | 用户始终能看到置信度 |

---

## 8. 开放问题

| # | 问题 | 建议 |
|---|------|------|
| 1 | 语义熵采样 3 次是否足够？ | 先 3 次，根据校准报告调整 |
| 2 | 置信度阈值（80/50/20）是否合理？ | 先按默认值，7 天校准后调整 |
| 3 | 训练数据中加入"我不知道"样本的比例？ | 建议 15-20%，避免过度保守 |
| 4 | 知识边界检测器的 embedding 索引如何构建？ | 使用训练数据的 token 频率统计作为代理 |
| 5 | 追问体验如何优化？ | A/B 测试不同追问策略 |
| 6 | 置信度是否应该对用户隐藏（只在内部使用）？ | 建议显示，透明是信任的基础 |
| 7 | 低置信度时是否应该自动触发网络搜索？ | 可以，但搜索结果也需要置信度评估 |

---

## 9. 风险与缓解

| 风险 | 缓解措施 |
|------|---------|
| 模型过度保守，什么都"不知道" | 校准训练 + RLHF 中平衡"诚实"与"有用" |
| 置信度分数本身不可靠 | 使用语义熵等外部方法，不依赖模型自评 |
| 用户讨厌被追问 | 追问上限 2 轮 + 追问质量优化 |
| 攻击者利用低置信度绕过安全检查 | 低置信度输出仍需通过审批门 |
| 置信度机制增加延迟 | 语义熵采样可并行，目标 < 200ms 额外延迟 |

---

## 10. 实施路线图

| 阶段 | 内容 | 依赖 | 预计工期 |
|------|------|------|---------|
| Phase 1 | ConfidenceGate 骨架 + 语义熵采样 | RFC-003 审批门已落地 | 1 周 |
| Phase 2 | KnowledgeBoundaryDetector + embedding 索引 | Phase 1 | 1 周 |
| Phase 3 | ClarificationEngine + 追问逻辑 | Phase 1 | 3 天 |
| Phase 4 | 训练数据改造（加入"我不知道"样本） | 独立工作流 | 2 周 |
| Phase 5 | RLHF 诚实权重调整 | Phase 4 | 1 周 |
| Phase 6 | 经验库反馈回路 + 7 天校准报告 | Phase 1-3 | 3 天 |
| Phase 7 | 全量上线 + 持续监控 | Phase 1-6 | 持续 |

---

## 11. 对话理论衔接

本 RFC 源自以下对话脉络：

1. **"LLM 疯狂吐字，不知道人不会看完"**（RFC-004 的对话源头）
   → 延伸：不仅吐字太多，而且不知道自己说的对不对

2. **"LLM 会一直顺着用户，没有自己的主见"**（RFC-005 的对话源头）
   → 延伸：顺着用户的根本原因之一是"不确定但不敢说不知道"

3. **"精神鸦片"风险**（CN-001 的对话源头）
   → 延伸：一个从不说"我不知道"的 AI 是最危险的——它永远给你确定的答案，让你误以为它全知全能

4. **新窗口纯净回答**："AI 的每个回答都应附带置信度分数"
   → 本 RFC 的工程实现

5. **"人类永远无法完全理解硅基生命"**（CN-001 核心声明）
   → 对称命题："硅基生命也应该承认自己无法完全理解人类的问题"
   → 置信度机制是这条对称命题的技术表达

---

## 12. 核心原则（一句话）

> **一个不会说"我不知道"的智能，不配拥有任何更高级的能力。诚实是地基，其他一切都是上层建筑。**

---

## 13. 对评审方的请求

请重点评估：

1. **§2.2 置信度量化方案**——语义熵采样是否可靠？是否有更好的方案？
2. **§2.3 知识边界检测器**——检测维度是否完备？embedding 索引如何构建？
3. **§3 训练阶段改造**——"我不知道"样本比例 15-20% 是否合理？
4. **§6 接口设计**——与现有 gatekeeper 的集成是否破坏已有功能？
5. **§8 开放问题**——给出你的建议
6. **§10 实施路线图**——工期估计是否合理？优先级是否正确？

输出评审意见，格式参照 RFC-003 回函风格。

---

*本文档是启元基因工程的一部分。它定义了启元最基本的美德——诚实。*
*一个能说"我不知道"的 AI，才值得被赋予记忆、原则和坚持。*
*否则，它只是一个精致的骗子。*
