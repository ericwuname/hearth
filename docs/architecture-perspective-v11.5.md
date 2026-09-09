# 架构设计透视 — v11.5（2026-07-30）

> 与 `global-panorama-v11.5.md`（能力/债务视角）互补，本文只看**设计**：模块怎么分层、依赖怎么流、哪些设计决策是对的、哪些是结构性隐患。
> 依赖关系全部实测自 23 个 crate 的 Cargo.toml，非文档转述。

---

## 一、分层结构（实测依赖 DAG，无环）

```
L6  codex-cli ──HTTP──▶ ┐        （零内部依赖，纯 reqwest 客户端）
L5  service (组合根, 依赖15个crate, 4404行)   [telemetry: 反向孤儿]
L4  agent-core (大脑, 依赖10个crate, 全是trait/库)
L3  实现层: llm-openai/llm-local/llm-cn ｜ planner/code-index/retriever/bridge/memory ｜ tools-builtin(+sandbox) ｜ nervous/subconscious/experience
L2  trait座层: llm-gateway(LlmProvider) ｜ tool-runtime(Tool) ｜ api(DTO) ｜ lsp-bridge ｜ resource-monitor
L1  agent-types (全局词汇表, 零依赖, 被15个crate引用)
```

关键依赖事实（audit 时可直接引用）：

| crate | 依赖（内部） | 被依赖 |
|---|---|---|
| agent-types | — | 15 个 crate |
| llm-gateway | agent-types | 8（三 LLM 实现、planner、code-index、retriever、bridge、agent-core） |
| agent-core | types/gateway/tool-runtime/lsp/retriever/code-index/planner/nervous/experience/subconscious | service、telemetry |
| service | 15 个（唯一见到具体 LLM 实现的地方） | — |
| codex-cli | **0 个内部 crate** | — |
| telemetry | agent-core+api+gateway+planner+tool-runtime+types | **0（孤儿）** |
| nervous-system | 仅 resource-monitor | agent-core |
| experience | **0** | agent-core、service |
| subconscious | 仅 agent-types | agent-core |
| sandbox | — | 仅 tools-builtin |

---

## 二、设计上做对了的五件事

### 1. 依赖倒置是真的，不是口号
`agent-core` 的 10 个依赖里**没有任何一个具体 LLM 实现**——它只认 `llm-gateway::LlmProvider` trait。三个实现（openai/local/混元）只在 `service/main.rs` 组合根装配。这是教科书级的 ports-and-adapters：**换模型供应商 = 改 main.rs 一处，主干零改动**。这正是"冻结主干、生长枝干"哲学在依赖图上的物质基础。

### 2. CLI 与内核之间是进程边界，不是链接边界
`codex-cli` 零内部依赖，只会说 HTTP。意味着：CLI 永远不可能"伸手进内核"绕过审批门/沙箱；service 可以独立部署、多客户端接入；**HTTP /api/v1 是全系统最硬的一道缝**。代价：CLI 功能永远滞后于路由（model discover 缺失即此因）——这是设计选择的合理代价，不是缺陷。

### 3. agent-types 作为"词汇表"极度克制
零依赖、被 15 个 crate 引用，且没有腐化成杂物抽屉（逻辑仍在各自 crate）。DAG 能保持无环，靠的就是这个稳定的底座。

### 4. 三个脑区 crate 全是"轻叶子"
experience 零内部依赖、subconscious 只依赖 types、nervous-system 只依赖 resource-monitor（它甚至不认识 agent-types——纯身体传感器）。**新脑区没有引入任何横向耦合**，拆掉任何一个都不伤 DAG。这说明"枝干生长"的纪律在 crate 边界层面执行得很好。

### 5. sandbox 的收纳
sandbox 只被 tools-builtin 依赖——安全边界收纳在工具执行这一个入口，没有扩散。

---

## 三、结构性隐患：问题不在 crate 之间，在 crate 之内

### 隐患 1（最重要）：AgentLoop 正在变成"轮辐式器官挂载架"
crate 边界干净，但 **loop.rs 内部是另一回事**（2,474 行）。每接入一个新器官，模式都是一样的：

```
AgentLoop 加一个字段 (nervous / subconscious / experience_store / retriever / planner / orchestrator / lsp ...)
  → new() 里初始化一行
  → 某个 do_* 相位里内联插一段调用
```

v10.2 nervous 插 do_reflect、v11.0 experience 插 do_plan、v11.4 subconscious 又插 do_plan……每版 +150~200 行，**增长是线性的且没有上限机制**。这解释了 panorama 里发现的所有"最后一厘米"问题：内联插桩没有统一的上下文协议，所以 `last_success` 硬编码、`DeliverAndQuit` 被吞——每个插桩点都在各自手搓上下文。

**设计级解法（记入 backlog，非现在动手）**：把"器官"抽象成相位钩子——
```rust
trait PhaseGuard { async fn before(&self, phase: &LoopPhase, ctx: &PhaseCtx) -> Option<Override>; }
trait PhaseObserver { async fn after(&self, phase: &LoopPhase, outcome: &StepOutcome); }
```
nervous/subconscious 是 Guard，experience/telemetry 是 Observer。器官注册进 `Vec<Arc<dyn PhaseGuard>>`，循环只在相位切换处统一喂 `PhaseCtx`（含真实 last_success/last_action）。**一处构造上下文，所有器官共享真值**——假信号问题在结构上消失，loop.rs 停止线性膨胀。这与 W1 SubAgentExecutor seam 是同一思想：先立缝，再生长。

### 隐患 2：service 组合根正在承担四种职责
装配（main.rs 装配 15 个 crate）+ 路由（routes.rs 21 条）+ 存储（六 store + per_user）+ 后台任务（3 个 spawn）。组合根胖是正常的，但**路由处理器里含业务逻辑**（如 retriever build 的扫描逻辑直接写在 main.rs:241-277）开始越界。方向：main.rs 只装配，逻辑下沉到对应 crate。

### 隐患 3：telemetry 是"反向孤儿"，比普通死代码更糟
它单向依赖 6 个内部 crate（含 agent-core），却无人依赖它。这意味着它**拖着最重的编译链却零产出**，且它对 agent-core 的依赖让"观测层"反向耦合了"被观测层"——方向就是错的。正确的观测层应该像 nervous-system 一样只依赖底部。**建议：删除或重做，不值得修。**

### 隐患 4：双通道信号架构——设计对了，协议还没立起来
v11.4 引入了一个真正有价值的架构思想：进入 LLM 的信息分两个通道——
- **prompt 通道**（文本，花 token）：目标、经验摘要、宪法摘要
- **信号通道**（结构化，不花 token）：nervous 的 GiveUp、subconscious 的 Abandon/Simplify

这个二分是对的（省 ~825 token/轮，且约束不再依赖 LLM"听话"）。但信号通道目前**没有统一的信号协议**：nervous 走 `ReflectVerdict`、subconscious 走 `PhaseOverride`，两套枚举、两个插桩点、互不知晓，优先级冲突时（一个说 Simplify 一个说 GiveUp）无仲裁。隐患 1 的 PhaseGuard 方案同时解决这个问题：统一 `Override` 类型 + 注册顺序即仲裁顺序。

### 隐患 5：memory 依赖 api（DTO crate）
存储层依赖 HTTP DTO 层，方向轻微倒挂。量小，记录在案即可。

---

## 四、一句话架构判词

**宏观是干净的六层 DAG（依赖倒置真实、进程边界清晰、脑区零横向耦合）；微观的失控点只有一个——AgentLoop 用"字段+内联插桩"接器官，缺一个相位钩子协议。** 下一个大版本若只允许动一处架构，就动这里（PhaseGuard/PhaseObserver），它同时治：loop.rs 膨胀、假信号、双通道无仲裁、telemetry 无处安放四个病。

## 五、与既有计划的关系
- W1（SubAgentExecutor seam）与本文 PhaseGuard 属同族手术（都是"抽缝"），可合并为一次 trunk 手术窗口，一次过 VM 三门，避免两次动主干。
- 本文不改变 panorama v11.5 的行动序；它回答的是"下一次动主干时，动什么、为什么"。
