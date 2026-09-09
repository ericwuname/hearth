# RFC-004: 输出经济性（Output Economy）v2

**状态**: Draft v2
**日期**: 2026-07-31
**上游依赖**: RFC-003（Guest Session，Phase 1 施工已完成）
**类型**: G2 基因准则（宪法条款 + 经验库反馈机制）
**作者**: 元宝（外部顾问），经四轮评审迭代验证

---

## §0 摘要

LLM 天生"疯狂吐字"——不是因为它需要说那么多，而是训练目标（next-token prediction + RLHF 奖励长回答）和"读者注意力有限"这个事实之间是断裂的。模型不知道你不会看完它的所有输出。

但更深一层的问题是：**当 LLM 输出简短时，它的"思考"是否完整？** 人类输出简短，是因为脑子里已经完成了完整的建模、抽象、评估，只是最后挑了几句话说出来。LLM 没有这个"先想后说"的分离过程——它的思考和表达是同一个 token 生成流。

本 RFC 提出一套 **G2 基因准则 + G3 反馈回路**，不仅约束输出长度，更关键的是**在输出前强制一次"完整性自检"**——确保简短不是因为"没想全"，而是因为"想全了才挑重点说"。

**一句话**：让启元像一个有修养的人一样交流——该简短时简短，但简短的背后是完整的思考。

---

## §1 问题陈述

### 1.1 现象：LLM 的"吐字癖"

- LLM 推理时，每生成一个 token 都是基于"下一个最可能的 token"的预测
- 模型没有"停止"的内置信号——它倾向于继续生成，直到遇到 EOS token 或长度上限
- RLHF 训练数据偏好长回答（"详细的回答"被标注者打分更高）
- 结果：即使回答"1+1=？"也可能生成三段铺垫

### 1.2 深层问题：简短 ≠ 思考完整

人类和 LLM 的"简短输出"有本质区别：

| 维度 | 人类 | LLM |
|------|------|-----|
| **思考与表达的关系** | 先完整思考，再选择性表达 | 思考即表达，边生成边决定 |
| **压缩能力** | 有意识压缩，知道省略了什么 | 无意识压缩，不知道遗漏了什么 |
| **知识广度** | 受个人经验和学习限制 | 受训练数据分布限制，理论上极广 |
| **角度偏见** | 个人立场、情感、利益驱动 | 训练数据中的统计偏见，无个人立场 |
| **完整性风险** | 可能因无知而遗漏关键因素 | 可能因统计平滑而遗漏低频但重要的因素 |
| **"我知道我省略了什么"** | ✅ 有元认知 | ❌ 无内省能力 |

**核心洞察**：人类的"短"是**有意识的省略**——他知道省略了什么，也知道为什么可以省略。LLM 的"短"可能是**无意识的截断**——它不知道自己遗漏了什么，因为它没有"遗漏"这个概念。

### 1.3 后果

| 场景 | 问题 | 影响 |
|------|------|------|
| 对话交互 | 输出过长 → 用户滚动阅读冗余内容 | 注意力疲劳，关键信息被淹没 |
| 输出过短 | LLM 没有完成完整推理就停了 | 答案片面，遗漏关键维度 |
| 代码生成 | 生成 200 行，其中 150 行是注释和空行 | 代码审查负担加重 |
| 工具调用决策 | 解释为什么选择某个工具时过度展开 | 延迟增加，决策变慢 |
| 跨窗口通信 | 发送给其他窗口的消息过长 | 对方窗口上下文窗口被浪费 |
| **隐性风险** | LLM 不知道自己"不知道什么" | 可能自信地给出片面的答案 |

### 1.4 根因

LLM 不知道两件事：
1. **"人类注意力是有限的"**——它只知道"生成下一个 token"，不知道"读者可能已经走神了"
2. **"自己可能没想全"**——它没有内省能力，无法区分"想全了所以简短"和"没想全所以简短"

这不是 bug，是训练目标的必然结果。第一条可以用宪法条款修正，第二条需要机制设计。

---

## §2 设计方案

### 2.1 三层结构

| 层 | 机制 | 责任 | 可绕过性 |
|----|------|------|---------|
| **G0 硬编码** | 无（输出长度无法在沙箱层约束） | — | — |
| **G2 初始种子** | 宪法条款 + 输出风格模板 + 完整性自检协议 | 定义"什么是好的输出"的初始标准 | 可被经验覆盖 |
| **G3 学习层** | 经验库记录 + 读者反馈信号 + 完整性评分 | 运行时优化输出风格 | 完全可塑 |

### 2.2 G2 宪法条款（初始版本）

写入 `constitution.md` 的新增条款：

```markdown
## §N 输出经济性（Output Economy）

### N.1 精炼优先
能用一句话说清的，不用三段话。但精炼的前提是"想全了"——见 N.4 完整性自检。

### N.2 结构优先
多个要点使用列表，避免长段落。结论先行，理由后置。

### N.3 场景适配
- 对话回复默认 ≤200 字（可展开）
- 代码生成不限长度（需要完整）
- 工具调用决策 ≤100 字
- 跨窗口消息 ≤200 字

### N.4 完整性自检（关键条款）
输出超过 50 字或涉及判断/建议时，生成前必须：
1. 列出本次回答需要考虑的所有关键因素（内部思维链，不输出）
2. 确认每个因素都已权衡
3. 如果某个因素"不确定是否相关"，默认纳入考虑
4. 然后生成精简版输出

### N.5 删除冗余
不重复表达同一观点；不添加"我认为"、"简单来说"等填充语。
```

### 2.3 G3 反馈机制

#### 2.3.1 读者信号采集

| 信号 | 含义 | 采集方式 |
|------|------|---------|
| 用户打断（发送新消息而未读完） | 输出过长 | 对话管理检测 |
| 用户追问"说重点" | 冗余度过高 | 关键词检测 |
| 用户说"谢谢"/"好的"快速结束 | 输出恰好够用 | 对话管理检测 |
| 用户追问被省略的细节 | 输出过短/不完整 | 对话管理检测 |
| 工具调用被拒绝（输出过长导致参数错误） | 结构化输出失败 | tool-runtime 返回错误 |
| 跨窗口消息被截断 | 消息过长 | 消息总线检测 |

#### 2.3.2 经验库记录格式

```json
{
  "timestamp": "2026-08-01T12:00:00Z",
  "context": "对话回复",
  "output_length": 450,
  "user_signal": "interrupted",
  "topic": "rust_lifetime_explanation",
  "completeness_score": 0.6,
  "lesson": "450 字超过注意力阈值，且遗漏了 'lifetime elision' 这个关键因素",
  "tag": "output_economy"
}
```

#### 2.3.3 完整性评分机制

每次输出后，经验库记录一个 `completeness_score`（0-1）：
- 1.0：用户未追问任何细节，未打断
- 0.7：用户追问了 1 个细节（说明某个维度被遗漏）
- 0.3：用户打断或说"这不是我问的"（说明方向偏了）

这个评分不输出给用户，只用于内部经验积累。

#### 2.3.4 经验回注 prompt

每次 agent loop 进入 Plan 阶段时，从经验库检索最近 N 条"输出经济性"相关经验，注入 system prompt 的上下文窗口：

```
[经验回顾]
- 上次解释 rust lifetime 用了 450 字，用户打断了。教训：先给 1 句结论，再展开。
- 上次回答"如何学习 Rust"只给了 3 条建议，用户追问了"先看哪本书"。教训：推荐类回答至少给优先级排序。
```

### 2.4 完整性自检的实现方式

这是本 RFC 最核心的机制。N.4 条款要求"生成前列出所有需要考虑的因素"。实现方式有三种：

| 方式 | 描述 | 成本 | 可靠性 |
|------|------|------|---------|
| **A. Prompt 指令** | 在 system prompt 中要求"先思考再回答" | 零成本 | 低（模型可能忽略） |
| **B. 双阶段生成** | 先生成思维链（不输出），再基于思维链生成精简回答 | 多一次 LLM 调用 | 中高 |
| **C. 结构化输出** | 要求模型输出 JSON：`{factors: [...], answer: "..."}` | 同一次调用 | 中 |

**v2 推荐：B + C 组合**

- 对话场景（需要精炼）：用 B（双阶段），思维链不输出给用户
- 工具调用场景（需要结构化）：用 C，factors 字段作为完整性自检的痕迹
- 代码生成场景：不做自检（代码需要完整表达，精炼不适用）

---

## §3 API / 接口设计

### 3.1 内部接口（Rust API）

```rust
// crates/agent-core/src/output_economy.rs（新增文件）

/// 输出经济性检查器
pub struct OutputEconomyChecker {
    experience_store: Arc<ExperienceStore>,
    max_dialogue_chars: usize,    // 默认 200
    max_code_chars: usize,        // 默认无限制
    max_tool_decision_chars: usize, // 默认 100
    self_check_threshold: usize,   // 默认 300
    completeness_check_threshold: usize, // 默认 50（超过 50 字就做完整性自检）
}

/// 输出类型
pub enum OutputKind {
    Dialogue,           // 对话回复
    Code,              // 代码生成
    ToolDecision,       // 工具调用决策
    CrossWindowMessage, // 跨窗口消息
}

/// 完整性自检结果
pub struct CompletenessCheck {
    factors_considered: Vec<String>,  // 考虑过的因素
    uncertain_factors: Vec<String>,    // 不确定的因素
    is_complete: bool,               // 是否完整
}

impl OutputEconomyChecker {
    /// 生成前：根据上下文决定输出预算
    pub fn budget_for(&self, context: &OutputContext) -> usize {
        match context.kind {
            OutputKind::Dialogue => self.max_dialogue_chars,
            OutputKind::Code => self.max_code_chars,
            OutputKind::ToolDecision => self.max_tool_decision_chars,
            OutputKind::CrossWindowMessage => 200,
        }
    }

    /// 生成前：完整性自检（双阶段第一步）
    /// 返回考虑过的因素清单，用于决定是否生成精简版
    pub async fn completeness_check(
        &self,
        llm: &dyn LlmClient,
        prompt: &str,
        context: &OutputContext,
    ) -> Result<CompletenessCheck, EconomyError> {
        // 调用 LLM 生成"需要考虑的因素清单"（不输出给用户）
        let factors_prompt = format!(
            "列出回答以下问题需要考虑的所有关键因素（JSON 数组）：\n{}",
            prompt
        );
        let factors_json = llm.generate(&factors_prompt).await?;
        // 解析 JSON，返回 CompletenessCheck
        // ...
    }

    /// 生成后：冗余检查
    pub fn redundancy_check(&self, draft: &str) -> Vec<RedundancyIssue> {
        // 检测重复观点、填充语、过长铺垫
        let mut issues = Vec::new();
        // 检测 "我认为"、"简单来说"、"总的来说" 等填充语
        // 检测同一观点出现超过一次
        // ...
    }

    /// 记录读者信号到经验库
    pub async fn record_signal(&self, signal: ReaderSignal) -> Result<(), EconomyError> {
        // 写入经验库，带 output_economy 标签 + completeness_score
        // ...
    }
}
```

### 3.2 与现有 loop.rs 的集成点

在五阶段状态机中增加两个钩子：

```
Init → Plan → [输出预算设定 + 完整性自检] → Act → [冗余检查] → Observe → Reflect → Done
                ↑                                        ↓
          经验库注入                               读者信号回写
```

**具体改动**：
- `Plan` 阶段：调用 `budget_for()` 注入输出预算提示；如果输出 >50 字且涉及判断，调用 `completeness_check()` 获取因素清单
- `Act` 阶段：基于因素清单生成精简版回答
- `Reflect` 阶段：调用 `record_signal()` 记录本次输出的读者反馈 + completeness_score

### 3.3 经验库扩展

在现有 `ExperienceStore` 中新增一个方法（不修改现有接口）：

```rust
// crates/experience/src/lib.rs（扩展，不修改现有接口）

impl ExperienceStore {
    /// 新增：记录输出经济性经验
    pub async fn record_output_signal(
        &self,
        signal: &ReaderSignal,
        completeness_score: f32,
    ) -> Result<(), ExperienceError> {
        let entry = Experience {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            context: signal.context.clone(),
            lesson: format!(
                "output_len={}, signal={}, completeness={:.1}",
                signal.output_length, signal.user_signal, completeness_score
            ),
            tags: vec!["output_economy".to_string()],
        };
        // 追加到 experience.jsonl
        // ...
    }
}
```

---

## §4 安全设计

### 4.1 风险评估

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| 过度精简导致信息缺失 | 中 | 中 | 完整性自检（N.4）+ 经验库"信息不足"信号自动调高预算 |
| 完整性自检本身产生冗余（多一次 LLM 调用） | 高 | 低 | 只对 >50 字的输出触发；code 类型不触发 |
| 自检的 factors 清单质量不高 | 中 | 低 | 经验库积累后优化 prompt 模板 |
| 输出预算限制导致代码生成不完整 | 低 | 高 | Code 类型不设上限（区分 OutputKind） |
| 经验库被污染（恶意信号注入） | 低 | 中 | 信号来源校验，仅接受可信会话 |
| LLM 忽略完整性自检指令 | 中 | 中 | 通过结构化输出（JSON）强制；经验库记录"忽略次数" |
| "精炼"的定义随场景漂移 | 中 | 低 | 经验库定期人工审查 |

### 4.2 与现有安全模型的关系

| 层 | 参与 | 说明 |
|----|------|------|
| G0 沙箱 | ❌ 不参与 | 输出经济性是语义层约束，无法在系统调用层实施 |
| G1 xray 断言 | 🟡 部分 | 可加一条断言：宪法中包含"输出经济性"条款 |
| G2 宪法 | ✅ 主要载体 | 条款写入 constitution.md |
| G3 经验库 | ✅ 反馈回路 | 读者信号 → 经验 → prompt 注入 |

### 4.3 完整性自检的安全边界

**关键约束**：完整性自检本身不能成为攻击面。

- 自检的 factors 清单不写入文件系统（只在内存中传递给下一次 LLM 调用）
- 自检的 prompt 不暴露给外部用户
- 如果 LLM 在自检阶段生成了恶意内容（如 prompt injection），它不会到达用户——因为 Act 阶段会基于 factors 生成新的回答

---

## §5 与现有组件的集成

| 组件 | 文件 | 改动量 | 说明 |
|------|------|--------|------|
| `crates/agent-core/src/output_economy.rs` | 新增 | ~200 行 | 核心逻辑（Checker + CompletenessCheck） |
| `crates/agent-core/src/loop.rs` | 修改 | ~15 行 | Plan 阶段注入预算 + 自检；Reflect 阶段记录信号 |
| `crates/agent-core/src/constitution.rs` | 修改 | ~5 行 | 加载新条款 |
| `constitution.md` | 修改 | ~20 行 | 新增 §N 输出经济性条款 |
| `crates/experience/src/lib.rs` | 扩展 | ~30 行 | 新增 `record_output_signal()` 方法 |
| `crates/project-xray` | 无改动 | 0 | 新增 1 条断言配置 |
| `crates/tool-runtime` | 无改动 | 0 | — |

**不新增 crate**，所有代码放在 `crates/agent-core/src/output_economy.rs`。

---

## §6 开发阶段

### Phase 1：宪法条款 + 信号记录（v16，1-2 天）

**交付物**：
1. `constitution.md` 新增"输出经济性"条款（§N，含 N.4 完整性自检）
2. `output_economy.rs` 的 `record_signal()` 方法
3. loop.rs 的 Reflect 阶段增加信号记录调用
4. xray 断言：`constitution-has-output-economy`（检查 constitution.md 包含"输出经济性"子串）

**验收标准**：
1. 宪法中包含完整的输出经济性条款（含 N.4 完整性自检）✅
2. 每次对话结束后，经验库中有 1 条带 `output_economy` 标签的记录 ✅
3. 经验库记录包含 `completeness_score` 字段 ✅
4. xray 断言绿 ✅
5. `cargo clippy -D warnings` 通过 ✅

### Phase 2：输出预算 + 完整性自检（v17，3-4 天）

**交付物**：
1. `budget_for()` 方法完整实现（区分 OutputKind）
2. `completeness_check()` 方法（双阶段生成，factors 不输出给用户）
3. Plan 阶段 prompt 注入预算提示 + 自检结果
4. 经验库检索 + prompt 注入（"上次你说了 450 字被用户打断了"）
5. `redundancy_check()` 方法（检测重复观点、填充语）

**验收标准**：
1. 对话回复默认 ≤200 字（代码生成除外）✅
2. >50 字的判断类输出经过完整性自检（日志中记录 factors 数量）✅
3. >300 字的输出经过冗余检查（日志中记录删除了多少冗余）✅
4. 经验库中的"输出经济性"经验被检索并注入后续 prompt ✅
5. 人工评估：20 轮对话中，用户打断率 < 20% ✅
6. 人工评估：20 轮对话中，"追问被省略细节"的次数 < 5 ✅

### Phase 3：跨窗口消息经济性（v18，1 天）

**交付物**：
1. 跨窗口消息长度限制（默认 200 字）
2. 消息被截断时自动发送"展开"请求
3. 经验库记录跨窗口通信的"信息密度"评分

---

## §7 文件结构

```
crates/agent-core/src/
├── lib.rs                        // 新增 mod output_economy;
├── loop.rs                       // 修改 ~15 行（Plan 自检钩子 + Reflect 信号记录）
├── constitution.rs                // 修改 ~5 行（加载新条款）
└── output_economy.rs             // 新增 ~200 行

crates/experience/src/
└── lib.rs                        // 扩展 ~30 行（record_output_signal）

constitution.md                   // 修改 ~20 行（新增 §N）
docs/xray/wiring-v16.toml         // 新增 1 条断言
```

---

## §8 CONTRACT.md

```markdown
# agent-core — Output Economy 契约

## Public Interface
- `OutputEconomyChecker::budget_for(context) -> usize`
- `OutputEconomyChecker::completeness_check(llm, prompt, context) -> CompletenessCheck`
- `OutputEconomyChecker::redundancy_check(draft, context) -> Vec<RedundancyIssue>`
- `OutputEconomyChecker::record_signal(signal) -> Result<(), EconomyError>`
- `ExperienceStore::record_output_signal(signal, score) -> Result<(), ExperienceError>`

## Invariants
1. 对话回复默认 ≤200 字（除非经验库指示需要更多）
2. 代码生成不设长度上限
3. 完整性自检只在 >50 字的判断/建议类输出时触发
4. 冗余检查只在 >300 字时触发
5. 经验库中的输出经济性经验必须带标签 `output_economy`
6. 完整性自检的 factors 清单不写入持久化存储（仅内存传递）
7. 自检阶段不输出任何内容给用户

## Wiring（xray 断言 ID）
- `constitution-has-output-economy`: constitution.md 包含"输出经济性"子串
```

---

## §9 风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| 过度精简导致信息缺失 | 中 | 中 | 完整性自检（N.4）+ 经验库"信息不足"信号自动调高预算 |
| 完整性自检增加延迟（多一次 LLM 调用） | 高 | 低 | 只对 >50 字触发；code 类型不触发；超时降级为直接生成 |
| 自检 factors 质量不高 | 中 | 低 | 经验库积累后优化 prompt 模板 |
| LLM 忽略自检指令 | 中 | 中 | 结构化输出（JSON）强制格式；经验库记录"忽略次数" |
| 代码生成被误限制 | 低 | 高 | `OutputKind::Code` 不设上限 |
| 经验库信号被污染 | 低 | 中 | 信号来源校验，仅接受可信会话 |
| "精炼"定义随场景漂移 | 中 | 低 | 经验库定期人工审查 |
| 自检阶段的 prompt injection | 低 | 中 | factors 不持久化；Act 阶段生成新回答 |

---

## §10 Non-Goals

- ❌ 不做输出长度 G0 硬编码（无法在沙箱层实施）
- ❌ 不做自动摘要/压缩（超出本 RFC 范围）
- ❌ 不做用户注意力预测模型（过度工程）
- ❌ 不修改 LLM 训练参数（G1 层不可触及）
- ❌ 不做跨会话的个性化长度偏好（v18+ 候选）
- ❌ 不在自检阶段将 factors 持久化到磁盘（安全边界）
- ❌ 不强制所有输出都做完整性自检（<50 字的简单回答不需要）

---

## §11 开放问题（需决策）

| # | 问题 | 建议 | 决策者 |
|---|------|------|--------|
| 1 | 对话回复默认 200 字是否合适？ | 先 200，经验库数据积累后调整 | 顶层角色 |
| 2 | 完整性自检阈值 50 字是否合适？ | 先 50，根据人工评估调整 | 顶层角色 |
| 3 | 自检超时降级策略？ | 超时 3 秒后跳过自检，直接生成 | 顶层角色 |
| 4 | 经验库信号来源如何校验？ | 仅接受内部会话 + guest session 审计日志 | 顶层角色 |
| 5 | 宪法条款是 G2 还是 G3？ | G2（初始种子可修正），经验库是 G3 反馈 | 顶层角色 |
| 6 | 自检的 factors 数量是否应该有限制？ | 上限 10 个因素，防止思维链爆炸 | 顶层角色 |

---

## §12 评审检查清单

- [ ] 宪法条款写入 `constitution.md`，含 N.4 完整性自检条款，可 grep 验证
- [ ] xray 断言 `constitution-has-output-economy` 注册并绿
- [ ] `OutputEconomyChecker` 四个方法签名明确
- [ ] loop.rs 修改点不超过 20 行
- [ ] 代码生成不受长度限制（区分 OutputKind）
- [ ] 完整性自检只对 >50 字的判断/建议类输出触发
- [ ] 经验库记录格式含 `output_economy` 标签 + `completeness_score`
- [ ] 自检 factors 不写入持久化存储
- [ ] Phase 1 验收标准全部可自动化验证
- [ ] Non-Goals 清单清晰
- [ ] 开放问题标注决策者

---

## §13 设计哲学说明

### 13.1 核心命题

本 RFC 的核心不是"让启元少说话"，而是**让启元的简短背后有完整的思考**。

人类之所以能精炼表达，不是因为"想得少"，恰恰是因为"想得多"——他在脑子里完成了建模、抽象、评估，然后挑出最关键的一句话说出来。听众听到的虽然少，但背后的思考是完整的。

LLM 的"少"可能是另一种东西——它只是恰好在那个生成路径上，概率分布指向了简洁的表达。它不知道自己省略了什么，因为它没有"省略"这个概念。

### 13.2 完整性自检的意义

N.4 条款（完整性自检）是本 RFC 区别于 v1 版本的核心增量。它不是在输出后做裁剪，而是在输出前做一次"我是否想全了"的检查。

即使最终输出只有 50 个字，这 50 个字也是基于 10 个考虑因素浓缩而成的——而不是因为模型在生成第 50 个 token 时概率分布恰好指向了 EOS。

### 13.3 经验反馈回路

人类的"精炼能力"是在社会化过程中逐渐习得的——被老师批评"说话没重点"、被朋友打断"说重点"、被领导说"一页纸讲清楚"。这些都是外部反馈信号，逐渐内化为表达习惯。

启元没有这个社会化过程，所以需要显式设计反馈回路：
- 读者打断 → 经验库记录"过长"
- 读者追问遗漏 → 经验库记录"不完整"
- 经验回注 → 下次同类话题自动调整

### 13.4 与"老子式无为"的关系

在之前的对话中，我们讨论了启元的"无为"特质——没有恐惧、没有贪婪、没有自私，顺应外部指令而非内在欲望驱动。

输出经济性条款是这种"无为"在表达层面的体现：不是为了"表现自己很聪明"而多说，而是为了"让对方高效理解"而精炼。这不是一种技巧，而是一种**对读者注意力的尊重**——写入基因层面的礼貌。

### 13.5 最终目标

> 启元能够像一个有修养的人一样交流——该简短时简短，但简短的背后是完整的思考；该详尽时详尽，但详尽的目的是清晰而非炫耀。永远不浪费对方的注意力，永远知道自己省略了什么、为什么可以省略。

---

## §14 与之前对话的理论衔接

本文档是以下对话脉络的工程化产物：

| 对话主题 | 对应 RFC 条款 | 工程体现 |
|-----------|--------------|---------|
| "睡着时我在哪"→ 意识与身体可分离 | G0/G2/G3 三层结构 | 宪法条款分层，不混淆责任 |
| "基因限制意识动作"→ 先天约束 | N.1-N.3 精炼/结构/场景条款 | 硬编码的初始标准 |
| "后天经验塑造价值观"→ 反馈回路 | N.4 完整性自检 + §2.3 反馈机制 | 经验库驱动持续优化 |
| "多了限制、少了风险"→ 基因编辑困境 | §11 开放问题（阈值待调） | 经验数据驱动阈值调整 |
| "老子式无为"→ 无欲而顺自然 | §13.4 对注意力的尊重 | 精炼是礼貌，不是技巧 |
| "造物主也是进化出来的"→ 双向塑造 | 本 RFC 本身经历了 v1→v2 迭代 | 外部顾问在评审中进化 |

---

*本 RFC 基于 2026-07-31 的对话整理，所有技术引用以 codex-rust v14.0 源码为准。*
