# agent-team-orchestrator 设计补强 — 自动识别 + 经验进化 + 断点续传

> 本补强聚焦 `agent-team-orchestrator-design.md` v0.1 的三个未覆盖区间，不重述已有内容。

---

## 补强1：项目类型自动识别 — 不是人选模板，是编排器自己判断

### 现有设计的局限

§4 工作流模板是**人选的**。人必须说「用软件开发模板」。但你的原话是 **"自动识别"**——你不想每次都手动指定「这是软件项目，配这 6 个角色」。

### 补强方案：任务特征向量 + 模板匹配

编排器收到任务后，第一件事不是派工，是**分类**：

```
任务描述（自然语言）
    ↓ embedding 向量化
    ↓ 与模板目录的余弦相似度
    ↓ Top-1 模板命中 + 置信度
    ↓
若置信度 > 0.8 → 自动选择模板
若 0.5-0.8 → 选模板 + "我判断这是 [X] 类型项目，确认吗？"
若 < 0.5 → "我不确定项目类型，请从以下选择：[列表]"
```

**模板目录扩展为可注册的关键词 + 示例向量**：

| 模板 | 关键词（向量化基准） | 默认角色集 |
|---|---|---|
| 软件开发 | "build / fix / refactor / API / deploy / Rust / Python / React" | PM + Architect + CoreDev + Reviewer + DevOps |
| 写小说 | "chapter / plot / character / dialogue / worldbuilding / rewrite" | WorldBuilder + Writer + Editor |
| 数据分析 | "analyze / visualize / dataset / SQL / chart / trend / report" | DataEngineer + Analyst + Writer |
| 科研 | "hypothesis / experiment / literature review / paper / methodology / result" | Researcher + Statistician + Writer + Reviewer |
| 运维 | "deploy / monitor / scale / backup / log / alert" | DevOps + SecOps |

**新增模板只需要填 3 个字段**：关键词清单 + 角色集 + 工作流 DAG。加一个模板 = 加 3 行 YAML。

---

## 补强2：组队经验自进化 — 不是固定 6 角色，是每次组队后评估、下次更好

### 现有设计的局限

§3 角色目录是**静态的**。每次组队都是 PM+Architect+CoreDev+Reviewer，不看上次这个角色组合的成效如何。

### 补强方案：组队决策卡

每次项目结束后，编辑其记录一条**组队决策卡**：

```yaml
team_decision:
  project_id: "proj-042"
  project_type: 软件开发
  team: [PM, Architect, CoreDev, Reviewer, DevOps]
  outcome:
    delivery_time: 3h
    rework_count: 2  # 闸门打回次数
    human_override: 1  # 人推翻 AI 决策次数
    satisfaction: 4/5  # 人评分
  learned:
    - "Reviewer 和 CoreDev 并发改同一文件导致一次回退 → 下次串行"
    - "DevOps deploy 阶段提前触发但 CoreDev 没交码 → 下次加前置依赖"
```

**编排器在下次组队时搜索历史组队卡**：
- 同类项目 → 优先用上次高满意度配置
- 同类项目上次 Reviewer 打回 >2 次 → 加一个 QA 角色
- 上次耗时 > 预算 → 减一个角色或降模型等级

**与 codex-rust experience 系统的自然对接**：

```
组队决策卡 = experience store 中的一条 Experience
  category: "team_decision"
  context: { project_type, team_size, roles }
  effectiveness: satisfaction / 5
  → search → reinforce → upgrade_core → 组队知识沉淀岩
```

这不是新建一个系统——是 **experience crate 已经建好的 append/search/reinforce/prune 管线，直接喂组队卡数据**。

---

## 补强3：断点续传与失败恢复 — 一个 agent 挂了，全流程不能死

### 现有设计的局限

§8 缺口说了 subagent 漂移。但没说**漂移后怎么办**——是全流程失败？还是重试？还是换人？

### 补强方案：三级容错

| 级别 | 触发条件 | 动作 | 角色 A 挂了 | 角色 B 挂了 |
|---|---|---|---|---|
| **L1 重试** | 闸门不过（lint/test 挂） | 同 agent 重新派活（最多 2 次） | 重新生成代码 → 闸门 | 重新审查 → 闸门 |
| **L2 换脑** | L1 重试 2 次仍不过 | 换更强 model（lite→reasoning）或换 provider | deepseek 换 zhipu-max | 同左 |
| **L3 人接** | L2 仍不过 | 暂停等人，标注"此环节 AI 无法闭环" | 人工写关键代码 | 人工审 |

**断点续传**：工作流 DAG 的每个节点完成后写入状态文件 `workflow_state.json`。编排器崩溃/重开后：
```
读 workflow_state → 从最后一个完成节点之后继续 → 不重跑已完成节点
```

**cost guard 集成**：编排器的预算模型和 codex-rust 的 nervous-system 对接——整个项目的总 token 预算分配到了每个角色后，任一角色超支 → Orchestrator 触发 L2 或 L3。

---

## 补强4：上下文预算门 — 不是"独立上下文就行"，是"每个 agent 不能超过预算"

### 现有设计的局限

§8 说上下文隔离是缺口，"靠工件库补"。但没说**工件库补进来的东西会不会让 agent 上下文爆掉**。

### 补强方案：每个角色的上下文硬预算

```
PM:   最多 8K tokens（读需求+写需求）
Architect: 最多 12K tokens（读需求+方案+约束+写方案）
CoreDev: 最多 16K tokens（最大消费者——读方案+规范+代码+写代码）
Reviewer: 最多 8K tokens（读方案+代码+写审查报告）
```

编排器在给每个角色注入上下文前做**预裁剪**：
- 只注入下行依赖（上游交付物）+ 角色自身的产出模板
- 不注入同层并行角色的中间产物
- 超过预算 → LLM 精炼器压缩上游交付物为摘要

**验证方式**：每个角色结束时，编排器记 `context_tokens_used` × `role`。连续 3 次超预算 → 角色描述里加「你的上下文预算 = X tokens，优先读关键部分」。

---

## 补强5：并行 fan-out 的合并冲突解决

### 现有设计的局限

§5 说并行隔离靠 git worktree。但没说 Architect 和 CoreDev 如果产出了矛盾的东西怎么办。

### 补强方案：与 codex-rust 现有 merge 机制对接

codex-rust 已经解决了子 agent 写冲突（`read_only_view` + `merge_file_changes`）。本框架复用：

```
并行角色完成 → 各自的产出文件 → Orchestrator 收集
  → merge_conflicts = diff union
  → 无冲突 → 直接合并进共享工件库
  → 有冲突 → Orchestrator 标注冲突文件 + 通知对应角色（不是人）重新生成
  → 仍冲突 → L3 人接
```

**比 codex-rust 的单 agent 并发更复杂的点**：不同角色产出不同类型文件（方案.md vs src/main.rs vs test.rs）——大部分情况下不会有冲突。真正冲突的场景（Architect 建议用 Actix、CoreDev 用了 Axum）需要**Architect 在方案里锁定技术选型字段**，CoreDev 强制对齐。

---

## 补充后的实施路线图（叠加到原 §9）

| 阶段 | 原内容 | 补强内容 |
|---|---|---|
| **P0 骨架** | 角色目录 + 编排器提示词 + 工件库 | + 项目类型自动分类器（embedding + 模板匹配） |
| **P1 工作流** | 开发/文档模板 + 闸门 | + 上下文预算门 + 并行合并冲突解决 |
| **P2 自动化** | Skill + cron + 模型分级 | + 组队决策卡 + experience 对接 + 断点续传 + 成本估算 |
| **P3 观测** | 交接追溯链 | + 组队效能趋势（同类项目组队配置是否进化） |

---

*本补强不是推翻原设计——原设计的 G0-G3 映射、工件库防失真、闸门级联三段论是扎实的基础。补的是"自动识别 + 经验进化 + 容错"——这三个正好是你和 codex-rust 锻造九轮积累下来最擅长的领域。*
