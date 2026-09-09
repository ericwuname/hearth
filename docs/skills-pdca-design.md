# Skills-PDCA 设计：把"弱模型也能高质交付"做成可执行引擎

> 配套文档：`top-level-design.md`（系统现状与目标）、`handoff-top-gaps-close-v2.md`（缺口执行计划）。
> 本文回答一个问题：**"执行器即护城河"不是口号，怎么在工程上变成现实？**
> 设计原则：**源码级可落地**——所有接缝都锚定 `file:line`，不靠想象。

---

## 0. 一句话目标

把现在"一次 LLM 调用直接分解任务"的 `planner.decompose`，升级成一条**多角色、可验证、自纠错的链式 PDCA 流水线**：

```
需求分析 → 头脑风暴 → 决策树 → 多视角圆桌 → 参谋长定稿 → 拆解执行
                                                      ↓
验收检查(Observe) → 局外人复盘 + 参谋长元决策(Reflect) → 迭代
```

核心价值：**单个弱模型调用只负责"一个窄而结构化"的子任务**，靠"多角色并行批判 + 验收门 + 局外人复盘"把错误在环内消化掉——这就是"弱模型变强"的真实机制。

---

## 1. 核心机制：弱模型为什么会变强（不是玄学）

一个弱模型单独干"整件事"会崩，但弱模型在**窄提示 + 结构化输出 + 有校验**的场景下是可靠的。护城河来自把"智能"从"模型单次推理"转移到"流程的多道保险"：

1. **窄提示（Narrow prompts）**：每个 skill 只问一件事（如"列出这个方案的安全风险"），弱模型答得准。这比"你是个高级架构师，把整个系统设计了"可靠一个量级。
2. **多视角并行批判（Roundtable）**：同时跑 K 个便宜角色（安全官/性能控/可维护性控/用户视角）各自挑刺，再合成异议。K 次便宜批评的总和 > 1 次强模型单遍——这是"委员会效应"，且成本远低于一次强模型。
3. **验收门（Acceptance）**：Act 之后用 skill 在 sandbox 里跑测试/静态检查，对照节点自带的 `acceptance_criteria`。错误在交付前被挡下。
4. **局外人复盘（Outsider）**：用"没参与过程的角色"重新审视整条轨迹，专门抓"当局者迷"的系统性偏差，喂给 Reflect 裁决。
5. **模型路由（Model routing）**：绝大多数 skill 用便宜模型；只在最难的单步（如参谋长定稿）保留强模型档位。`CostMeter` 已存在（`llm-gateway`），只需加一层路由。

> 本质：**用流程的结构化冗余，替代对单次模型推理力的依赖。** 这正是 sandbox/CostMeter/重放这些原语存在的理由——它们让"敢让弱模型多自主行动"成为可能。

---

## 2. 现状接缝（已读源码确认）

| 接缝 | 位置 | 在本设计中的角色 |
|------|------|------------------|
| `Planner::decompose` | `planner/src/lib.rs:43` | **Plan 咽喉**——改为由 `Facilitator::plan` 驱动，内部仍调用 `decompose` 产出最终 DAG |
| `Planner::reflect` | `planner/src/lib.rs:155` | **Check/Act 咽喉**——改为由 `Facilitator::reflect` 驱动，内部仍用启发式 + LLM 裁决，并注入"局外人复盘"结论 |
| `spawn_sub_agent` | `agent-core/src/loop.rs:308`（内部 `tokio::spawn`@339） | **并发跑角色的底**——圆桌/验收/复盘都以 sub-agent 形式并行跑 |
| `sub_agent_handles` / `collect_sub_agent_results` | `loop.rs:222` / `:376` | 收集并行角色结果（**当前仅写 status + 一行文本，见 §5 缺口 G-A4**） |
| `dispatch_parallel` / `join_all` | `tool-runtime/src/dispatcher.rs:108` | 工具级并行（已落地）；角色级并行复用 `spawn_sub_agent` 模式 |
| `PlanContext` | `planner/src/lib.rs:59` 使用 `goal/retrieval_context/lsp_diagnostics/available_tools` | 扩展以携带"规划档案"（各 skill 输出）跨相位传递 |
| `TaskNode` | 字段 `id/description/deps/status/delegable/result`（`planner/src/lib.rs:112`） | 扩展以携带 `acceptance_criteria` 与 `skill_plan` |
| `Observation` | `planner/src/lib.rs:228` 使用 `recent_results/consecutive_errors/steps_without_progress/...` | 扩展以携带 skill 轨迹，供局外人复盘消费 |
| `Role` 枚举 | `agent-types/src/lib.rs:110` | **仅为消息角色**（System/User/Assistant/Tool），不是 Persona——本设计新增独立的 `Skill`/`Persona` 抽象 |

**关键约束**：`spawn_sub_agent` 有递归守卫（`loop.rs:318`：`self.depth>=1` 拒绝再派生；子代理 `depth=parent+1` 且 `loop.rs:348` 子代理不向父流发事件）。⇒ 圆桌是**单层并行批判**（depth=1，不再嵌套），正好符合"一层多视角"的语义，无需改守卫。

---

## 3. 新抽象：`Skill` 与 `SkillRegistry`

新增 crate `crates/skills`（或并入 `planner`；独立 crate 更符合现有分层）。`Skill` 是"结构化推理角色"的一等类型：

```rust
// crates/skills/src/lib.rs
pub struct Skill {
    pub id: &'static str,
    pub persona: &'static str,        // system prompt：角色 + 方法 + 输出格式要求
    pub output_schema: &'static str,  // 结构化输出 JSON schema（弱模型靠 schema 才稳）
    pub run_mode: RunMode,            // InProcess(单次 chat) | SubAgent(需工具/多步)
    pub model_tier: ModelTier,        // Cheap | Strong | Any  —— 接模型路由
    pub needs_tools: bool,            // 是否需要在 sandbox 里跑工具（如验收）
}

pub enum RunMode { InProcess, SubAgent }
pub enum ModelTier { Cheap, Strong, Any }

pub struct SkillRegistry { /* 内置 skill 表 + get(id) */ }
```

**in-process 调用器**（复刻 `decompose` 已有的 JSON 抽取逻辑，`planner/src/lib.rs:96-105`）：

```rust
pub async fn run_skill(
    skill: &Skill, input: &str, provider: Arc<dyn LlmProvider>,
) -> Result<serde_json::Value> {
    let prompt = format!(
        "{}\n\nInput:\n{}\n\nOutput ONLY JSON matching this schema:\n{}",
        skill.persona, input, skill.output_schema
    );
    let resp = provider.chat(ChatRequest {
        messages: vec![Message::new("sys".into(), Role::System, Text(prompt))],
        temperature: Some(0.2), stream: false, ..Default::default()
    }).await?;
    extract_json_array_or_object(resp.content)   // 复用 decompose 的容错抽取
}
```

---

## 4. 编排层：`Facilitator`（PDCA 链导演）

`Facilitator` 是新增的"规划/复盘导演"，由 `loop.rs` 的 `do_plan`/`do_reflect` 调用。它把 skill 串成链，并复用 `spawn_sub_agent` 做并行角色。

### 4.1 Plan 相位（`do_plan` → `Facilitator::plan`）

```rust
pub async fn plan(goal: &str, ctx: &PlanContext, reg: &SkillRegistry) -> Result<TaskGraph> {
    // ① 需求分析（Cheap, InProcess）
    let req = run_skill(reg.get("requirements_analysis"), goal, cheap).await?;

    // ② 头脑风暴：生成 N 个候选路线（Cheap, InProcess 或并行 sub-agent）
    let ideas = run_skill(reg.get("brainstorm"), &req, cheap).await?;

    // ③ 决策树：把候选折成 选项+利弊+权衡（Cheap, InProcess）
    let tree = run_skill(reg.get("decision_tree"), &ideas, cheap).await?;

    // ④ 多视角圆桌：K 个 Persona 并发挑刺（Cheap, SubAgent 并行）
    let critiques = dispatch_roundtable(&tree, &ROUNDTABLE_PERSONAS).await?;

    // ⑤ 参谋长定稿：综合 需求+决策树+异议 → 策略（可上 Strong 档）
    let strategy = run_skill(reg.get("chief_of_staff"),
                             &synthesize(&req, &tree, &critiques),
                             strong_or_cheap).await?;

    // ⑥ 最终拆解：仍走原 decompose 产 DAG，并把 acceptance_criteria 挂到每个节点
    let mut tg = planner.decompose(&strategy_to_goal(&strategy), ctx).await?;
    enrich_nodes_with_acceptance(&mut tg, &strategy);
    Ok(tg)
}
```

`dispatch_roundtable` 直接复用 `loop.rs:308 spawn_sub_agent` + `collect_sub_agent_results`：每个 persona 起一个 depth=1 的 sub-agent，跑完收集（**依赖 §5 G-A4 修复，否则拿不到批判正文**）。

> **视角与元认知注入点**：`user_advocate`（用户思维）在 ①需求分析 阶段即并入"用户会被坑吗"视角，并作为圆桌的一个固定 Persona；`self_reflection`（自我反思）在 ⑤参谋长定稿**之前**先做一道"认知边界检查"——若 `unknown_unknowns` 或 `blind_spot_warnings` 非空，强制回到 ①补充信息而非盲目定稿。两 skill 是压住弱模型幻觉、避免片面全知的关键保险（schema 见 §6）。

### 4.2 Observe 相位（验收门）

`do_observe` 完成后，对每个刚完成的 `TaskNode` 跑 **acceptance skill**（`needs_tools=true`，在 sandbox 子进程里跑测试/静态检查），对照 `node.acceptance_criteria`。不达标 → 标记节点 `Failed` 并带诊断，触发 Reflect 的 replan。

### 4.3 Reflect 相位（Check/Act）

```rust
pub async fn reflect(obs: &Observation, state: &PlanState) -> Result<ReflectVerdict> {
    // 局外人复盘： fresh-eyes 角色审视整条轨迹（Cheap, 消费 obs + skill_trace）
    let outsider = run_skill(reg.get("outsider"), &obs_to_text(obs), cheap).await?;
    // 参谋长元决策：结合启发式 + 局外人结论 → continue/replan/give_up
    // 内部仍调用原 planner.reflect 快/慢路径（planner/src/lib.rs:155）
    let verdict = planner.reflect_with_context(obs, state, &outsider).await?;
    verdict
}
```

`replan` 时，`outsider` 的诊断会作为"为何失败"的上下文重新喂给 `Facilitator::plan` 的第①步，使下一轮规划比上一轮更聪明——**这就是"复盘迭代"的闭环**。

---

## 5. 必须的代码缺口（落地前先修）

| 编号 | 位置 | 问题 | 修复 |
|------|------|------|------|
| **G-A4（核心）** | `loop.rs:350-368` `RunReport` | 子代理只回传 `ok/steps/summary(goal)`，其**自由文本产出被丢弃**（summary 只含 goal/steps/ok，不含实际工作产物） | `RunReport` 增加 `output_text: String` / `artifacts: Vec<Artifact>`，由 `sub_agent.run` 的末轮消息抽取填充 |
| **G-A4b** | `loop.rs:806-821` / `:838-848` | `collect_sub_agent_results` 调用方仅写 `node.status` + 一行 `TaskResult.output` | 改为内容级 merge：把子代理 `output_text` 写入 `node.result.output` 并触发父上下文更新 |
| **G-CTX** | `PlanContext`（`planner/src/lib.rs:344` 测试构造器可见字段） | 无跨相位"规划档案"载体 | 增 `planning_dossier: Option<PlanningDossier>`（装各 skill 输出，Plan→Reflect 复用） |
| **G-NODE** | `TaskNode`（`planner/src/lib.rs:112`） | 节点无验收标准/技能计划 | 增 `acceptance_criteria: Option<String>`、`skill_plan: Option<Value>`（参谋长定稿时写入，验收门消费） |
| **G-OBS** | `Observation`（`planner/src/lib.rs:228` 使用字段） | 无 skill 轨迹 | 增 `skill_trace: Vec<SkillTrace>` 供局外人复盘 |
| **G-ROUTE** | `llm-gateway` `CostMeter` | 有成本核算但无"按 tier 选模型" | 新增 `ModelRouter`：`Cheap/Strong` 两档池 + 按 skill.model_tier 路由；`CostMeter` 继续记账 |
| **G-MODE** | `agent-core/src/loop.rs:192`(`AgentLoop`)/`:233 new`/`:958 run` | 无运行模式概念，loop 默认全自主执行 | `AgentLoop` 增 `mode: Mode` + `prefilled_task_graph: Option<TaskGraph>`；`run()` 按模式选初始相位；Plan 冻结 TaskGraph 并持久化；Execution `do_reflect` 改"升级到计划节点"而非自 replan（见 §9） |

> G-A4 是**前置依赖**：圆桌/参谋长/验收/复盘的全部价值，都建立在"父代理能拿到子代理真实产出"之上。它与 `top-level-design.md` 的 A4 缺口同源，本设计把它具体化为上述两处修复。

---

## 6. 八个内置 Skill 的职责与输出 schema（草案）

| Skill | 角色 persona（摘要） | 输入 | 输出 schema（要点） | run_mode / tier |
|-------|----------------------|------|---------------------|-----------------|
| `requirements_analysis` 需求分析 | "你是把模糊目标澄清成可验收规格的分析师" | 原始 goal | `{goal, scope_in, scope_out, constraints, acceptance_criteria, open_questions}` | InProcess / Cheap |
| `brainstorm` 头脑风暴 | "你是发散思维引擎，禁止自我审查" | 需求规格 | `{candidates: [{id, approach, pros, cons, fit}]}` | InProcess / Cheap |
| `decision_tree` 决策树 | "你把候选折成带权衡的决策树" | 候选集 | `{nodes:[{id, option, pros, cons, risk}], edges:[parent→child]}` | InProcess / Cheap |
| `roundtable` 圆桌会议 | 多个 Persona：安全官/性能控/可维护性控/用户视角 | 决策树+策略草案 | `{critiques:[{persona, severity, finding, suggestion}]}` | **SubAgent 并行** / Cheap |
| `chief_of_staff` 参谋长 | "你是整合各方、拍板定稿的参谋长" | 需求+决策树+异议 | `{strategy, key_decisions, node_plans:[{id, acceptance_criteria}], risks}` | InProcess（或 Strong）/ Strong 可选 |
| `acceptance` 验收检查 | "你是只认证据的验收官，跑测试/静态检查" | 节点产物+acceptance_criteria | `{passed:bool, evidence, failures:[...]}` | SubAgent（需 tools）/ Cheap |
| `outsider` 局外人 | "你没参与过程，专挑系统性盲点" | 全轨迹摘要 | `{blind_spots:[...], root_cause, recommendation}` | InProcess / Cheap |
| `retrospective` 复盘（可选） | "你是复盘教练，提炼可复用经验" | 全轨迹 | `{lessons:[{pattern, do/don't}], replay_hints}` | InProcess / Cheap |
| `user_advocate` 用户思维 | "你是最终用户代言人：前端讲易用、后端讲缜密，凡事先问'用户会被坑吗'" | 需求+草案 | `{user_risks:[{area, risk, severity}], usability_notes, rigor_notes}` | InProcess / Cheap |
| `self_reflection` 自我反思 | "你专审认知边界：列明已知已知/已知未知/未知未知，反对全知全能断言" | 任意中间态 | `{known_knowns:[], known_unknowns:[], unknown_unknowns:[], confidence, blind_spot_warnings:[]}` | InProcess / Cheap |

---

## 7. 增量落地路线（避免 big-bang，每阶段可独立验收）

- **阶段 0 — 模式骨架（前置，跨切面）**
  落地 `Mode` 枚举 + `AgentLoop` 字段（`loop.rs:192/233`）+ `run()` 初始相位选择（`loop.rs:958`）+ Plan 冻结 / Execution 升级两个最小行为（G-MODE，§9）。其余阶段在其上叠加，否则 skill 无处挂载权限。
- **阶段 1 — 抽象与单进程 skill（低风险）**
  新建 `crates/skills`：`Skill`/`SkillRegistry` + `run_skill`（复刻 decompose 的 JSON 抽取）。先接 2 个 InProcess skill（`requirements_analysis`、`decision_tree`）进 `decompose`，零 sub-agent 复杂度。验证：规划质量可观测提升、成本不变。
- **阶段 2 — 圆桌并行 + 参谋长 + A4 修复（核心）**
  实现 `dispatch_roundtable`（复用 `spawn_sub_agent`）、`chief_of_staff` 定稿；**同时修 G-A4**（RunReport 携带产出 + 内容级 merge）。这是"弱模型变强"的关键一跃。
- **阶段 3 — 验收门 + 局外人复盘（闭环）**
  Observe 接 `acceptance` skill（sandbox 内跑验证）；Reflect 接 `outsider` + `chief_of_staff` 元决策，replan 时回灌诊断。PDCA 全环打通。
- **阶段 4 — 模型路由（成本护城河）**
  加 `ModelRouter`，默认 Cheap 跑全部 skill，仅 `chief_of_staff`/最难节点可上 Strong；`CostMeter` 继续记账。此时"弱模型高质 + 低成本"同时成立。

---

## 8. 与现有顶层缺口/路线图的衔接

- 直接闭合/依赖于：`top-level-design.md` 的 **A4**（merge）→ 本设计 G-A4；**T6**（replan 硬封顶，已修）→ 本设计 Reflect 复用其裁决；**T4**（retriever/lsp 接线）→ 可作为 `requirements_analysis` 的上下文源。
- 新增路线图项建议（接在 T1–T11 之后）：**T12** `crates/skills` 抽象；**T13** 圆桌并行 + 参谋长；**T14** 验收门 + 局外人复盘；**T15** 模型路由；**T16** 运行模式系统（Mode + 权限矩阵 + Plan 冻结 / Execution 升级）。
- 风险登记补充：弱模型的**真实天花板**（见 §10）、圆桌带来的延迟/成本、skill prompt 自身的维护成本。

---

## 9. 运行模式（Mode）系统：四种权限边界

模式是跨切面的权限策略，决定"规划 / 执行 / 询问 / 自纠"四件事谁能做。挂在 `AgentLoop`（`loop.rs:192` 结构体 / `:233 new`）上，由 `run()`（`:958`）按模式选初始相位并贯穿 `do_plan` / `do_act` / `do_reflect`。

### 9.1 模式枚举与权限矩阵

```rust
// crates/agent-core/src/loop.rs
pub enum Mode {
    Dialogue,    // 对话沟通：只探讨，不执行
    Plan,        // 计划规划：归纳探讨 + 冻结拆解，不自执行
    Execution,   // 执行落地：按冻结任务执行，不思考/不询问，错误回计划节点
    Exploration, // 目标探索：自主循环，边想边做直到目标完成
}
```

| 权限 | Dialogue | Plan | Execution | Exploration |
|------|---------|------|-----------|-------------|
| 执行写操作工具 | ❌（仅只读/不执行） | ❌（仅产出计划） | ✅ | ✅ |
| 规划后自动执行 | ❌ | ❌（冻结后停，交人审） | n/a（直接 Act） | ✅（自循环） |
| 向用户询问/审批 | n/a | n/a | ❌（抑制审批，沙箱内自批） | ❌（自主，仅破坏性靠沙箱兜底） |
| 自主 replan | n/a | n/a | ❌（升级到计划节点） | ✅ |
| 启用圆桌/验收/复盘 skill | 讨论用（不落地） | ✅ | ❌（纯执行，仅验收门） | ✅ |
| 停在 Plan 返回给用户 | ✅（探讨摘要） | ✅（冻结计划） | ❌ | ❌ |

### 9.2 各模式如何驱动 loop（file:line 接地）

- **Dialogue 对话沟通**：`run()` 进入 `Init→Plan`（`loop.rs:939-941`），`do_plan` 仅跑 `user_advocate` + `brainstorm` + `self_reflection` 做**思路探讨摘要**，不调 `Facilitator` 全链、不产出可执行 TaskGraph，落到 `Done`（`loop.rs:542/553`）并 emit `PlanProposed{discussion}`。绝不进 `Act`。
- **Plan 计划规划**：`do_plan` 跑完整 `Facilitator::plan`（§4）→ 产出 TaskGraph → **冻结**（写入 MemoryStore/会话的 `frozen_plan`）→ emit `PlanFrozen{task_graph}`，返回 `Done`，**不进 Act**。人/计划节点审后，以 `Execution` 模式 + `prefilled_task_graph` 起新 loop 执行。
- **Execution 执行落地**：`AgentLoop::new` 收 `prefilled_task_graph` + `mode=Execution`；`run()` 初始相位直接置 `Act`（跳过 `Init→Plan`）。`do_act`（`:615`）**抑制审批门**（`loop.rs:617-658` 在 Execution 下自动放行，依赖 sandbox 兜底）；`do_reflect`（`:831`）遇 `Replan`/`GiveUp` **不自决策**，改为 emit `EscalateToPlan{reason, observation}` 并 `Done`——"错误 / 重复执行反馈到计划节点，等待下一步规划执行"。纯执行、不思考、不中断询问。
- **Exploration 目标探索**：即当前默认 `run()` 自主行为——`do_plan`(轻量)→`Act`→`Observe`→`Reflect` 自决 `Continue/Replan/GiveUp`（`:888/:915` replan→Plan，`:927` give_up→Error）循环至 `Done`/目标达成。启全部 skill，抑制审批（自主）。

### 9.3 模式与 skill 的关系

- Dialogue / Plan 用 `user_advocate` + `self_reflection` 保证"为人着想 + 不自以为全知"；
- Plan / Exploration 用完整 skill 集（含圆桌/验收/复盘）；
- Execution **不用任何规划类 skill**，只保留 `acceptance` 验收门（Observe 阶段对照冻结节点的 `acceptance_criteria`），把"思考"彻底外包给上游计划节点——这正是"执行模式不必思考"的工程落地。

### 9.4 落地说明

模式骨架是跨切面前置件，建议在阶段 1 之前先落地 `Mode` 枚举 + `AgentLoop` 字段 + `run()` 初始相位选择 + Plan 冻结 / Execution 升级两个最小行为（G-MODE）。其余 skill 在其上叠加。

## 10. 边界与诚实声明（避免过度承诺）

1. **弱模型不是万能**：skill 流程能做"分解、批判、验证、重试、隔离"，但**补不了模型根本没有的潜在推理力**（全新架构设计、跨多文件硬核调试、开放研究）。因此 `chief_of_staff`/最难单步保留 Strong 档——本设计是"模型无关 + 优雅降级"，不是"假装弱模型等于强模型"。
2. **延迟代价**：多 skill 串行 + 圆桌并行，单次规划的延迟与 token 量高于单次调用。靠 (a) 圆桌并行摊薄、(b) 便宜模型、(c) 仅在必要时上强模型 来平衡。
3. **skill prompt 是会腐化的**：persona/schema 需随任务域演进维护，应纳入回归测试（这正是 §7 每阶段"可独立验收"的原因）。
4. **A4 是硬前置**：在 G-A4 修复前，圆桌/参谋长/复盘全部是"盲跑"——父代理看不到子代理产出。阶段 2 必须与 G-A4 同批落地。

---

## 11. 验收口径（门禁）

- 每个阶段有独立可观测指标：规划通过率、验收一次通过率、replan 次数、单任务总成本、弱模型(如 gpt-4o-mini 级) 在强模型 70% 质量下的成本比。
- "护城河成立"的判据：**用 Cheap 档跑满整条 PDCA，在基准任务集上达到 Strong 档单遍调用 ≥80% 的验收通过率，且单任务成本 ≤ Strong 档的 40%。**
- 源码级判据：每个 skill 有结构化输出断言测试；圆桌并行有 `join_all` 汇总断言；G-A4 修复后 sub-agent `output_text` 非空的断言测试。
