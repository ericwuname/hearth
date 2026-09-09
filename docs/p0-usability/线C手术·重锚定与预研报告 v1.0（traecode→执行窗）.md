# 线C手术·重锚定与预研报告 v1.0（traecode→执行窗）

> **执行**：traecode 执行窗　**日期**：2026-09-08　**依据**：《R9收官手术·执行委托书》§二（P0 预研，纯只读零改动）
> **性质**：手术前静态重锚定。**未动 loop.rs 一行**（红线 1 合规）。

---

## 一、基线自证

| 项 | 要求 | 实测 | 判定 |
|---|---|---|---|
| HEAD | ≥ 61dea7c | `9eb250d`（R9 委托书 commit，链上第 3 笔） | ✅ |
| loop.rs 行数 | = 13,055 | **13,055**（`[System.IO.File]::ReadAllLines().Count` 精确口径） | ✅ |

**口径警示（仪器登记）**：PowerShell `Measure-Object -Line` 与 `git show | Measure-Object -Line` 报 **12,533**（不计空行）——与 13,055 差 522 恰为空行数。手术行数验收统一用 `wc -l`（VM）或 ReadAllLines（本机），**禁用 Measure-Object -Line**，防口径漂移误判。

## 二、十项重锚六元组表

> 坐标全部为 13,055 基线实测。臂判定缩写：**A现**=A 臂现役（保留/冲突申报）；**B专**=B 臂专属（可删）；**共享**=两臂编译期/入口共用（改写）。

### D-10 · emit_think_summary 相位套话

- ①锚点：本体 :2390-2415（26 行）+ phase_label :2378-2388；套话调用 3 处 :5323/:5337/:5351（step() 内 plan/act/observe 模板投影）
- ②引用：仅 step() 3 处 + 保留项直投清单（见⑥）
- ③实测：本体+label ≈ 36 行，3 调用点 3 行 → **≈ 40 行**
- ④修正减量：**40**（原估 60 偏高——本体已很薄）
- ⑤A 臂触达：3 处套话中 observe 仅 B 臂可达；plan/act 套话 A 臂可达但为纯模板噪声（detail=None）→ 删套话调用不影响 A 臂信息面
- ⑥**保留项红线清单（勿误删）**：R8 预算双闸投影 :5558-5564 / :5576-5582（[budget-warn]/[budget-critical]）；[budget-ok] :5732-5738；[observe 判定] :4509-4512（G-H 审计）；reflect 决策投影 :5378（phase_label 唯一现役消费）；[goal_drift] :6023-6028；[approval_denied_noninteractive] :3934-3937；[delegated] :3885-3888（测试 6630/6701 依赖文本）；reasoning 投影 :3641-3644；[写前目标校验] :1590-1605；[egress] 三连 :2089-2105；[verify] 行 :4351-4364（校验事实非套话）

### D-8 · B 臂机关块

- ①锚点：do_plan_inner 无 tool_calls 文本分支 :3663-3822（v22 注释自述 :3663-3688 明写"下方 3541-3648 的 B 臂机关全部旁路"——注释所指块已漂移至此区间）
- ②引用：write_attempted 消费 :3713；MUST tool_calls 指令 :3807；forced replan :3794；all_done v22 门 :3220-3246（前段，与 D-8 同族）
- ③实测：:3663-3822 ≈ **160 行**（含 A 臂 end_turn 出口 :3692-3701 与 QA 快速路 :3702-3711 ——**这两段是 A 臂现役，必须剥离保留**）
- ④修正减量：**≈120**（160 − A 臂现役两段 ~40 行）
- ⑤A 臂触达：end_turn 出口 :3692-3701 / QA 路由 :3702-3711 / give_up_injections 出口 :3673-3682 —— 三段现役，剥离保留；其余（v22 write 门/forced replan/MUST 指令/all_done 门 :3220-3246）B 专
- ⑥风险：:3713 v22 门删除后 :3729 last_completion_decision 语义随迁——completion_decision 投影保留（诚实性）

### D-1 · goal_requires_product() + 98 词表

- ①锚点：doc :212-229 + 函数体 **:230-369（140 行，词表 98 词：NEGATIVE 6/PRODUCT_ACTIONS 34/EN 12/DISCOURSE 40/EN 6）**
- ②引用（全仓 .rs 仅 loop.rs）：生产 7 处 = :2437（build_messages 分流，共享）/ :2977（QA 跳 decompose，共享 no-op）/ :3233（all_done，B 主）/ :3702（QA 直答，B 专——A 臂 :3692 先 return）/ :4972 / :5181（do_reflect 两处，B 专）/ :5664（预算 giveup_unverified，**共享现役**）；测试 5 个（:8133/:8158/:8180/:8202/:10279）；docs 40+ 文件仅文档提及
- ③实测：定义 158 行（doc+体）+ 测试 ~120 行
- ④修正减量：**≈270**（定义 158 + 测试 ~120；调用点改写随各 D 项就地消化不计行）
- ⑤A 臂触达：共享 3 处中 :2437（prompt 分流）与 :5664（giveup_unverified 诚实标记）**A 现役**——拆除方案：:2437 分流改为"单循环恒 product 工作流 prompt"常量化；:5664 的 QA 判别语义收窄为恒 false（产品型任务占位）——**改写不删语义**，避免诚实性回退
- ⑥风险：C 语料 QA 类（C-self-assess 等）依赖 QA 直答路径——A 臂 end_turn 判定权在模型，删词表后 prompt 分流常量化对 A 臂行为中性（单循环 prompt 本就一套）；B 臂测试同步删

### D-2 · UserInputKind + classify_user_input

- ①锚点：enum :371-384 + classify :387-476（**106 行，词表 42 词**：TASK_CONTROL 14/CHITCHAT 8/QUESTION 12/PRODUCT 8）
- ②引用：生产**仅 1 处** :5412（run() Goal Revision 三分类，共享）；测试 3 个（:10104/:10315/:10340）；corpus-c.jsonl:2 judge 提及（文档性）
- ③实测：定义 106 + 测试 ~80 + 调用块 :5410-5425 ~16 行
- ④修正减量：**≈200**（原估 70 过低——含测试与调用块）
- ⑤A 臂触达：:5412 在 A 臂执行（run() 共享入口）——Goal Revision 三分类的 TaskControl/Conversation "不动 goal" 语义在 A 臂有效（防护性）→ **冲突申报候选**：拆除需把 :5410-5425 收窄为"全部输入按 GoalMutation 处理"（A 臂行为变化 = TaskControl 输入不再保护 current_goal）。**建议：D-2 收窄为删词表+枚举、保留 run() 分类调用点改直通**——或按增 3 元原则全保留。**预判：改写删除，行为等价性由 test_r75/test_r69 回归保障**
- ⑥风险：C2 误判 29% 病灶源头即此——删除正是治本，但 3 个测试死、corpus-c judge 文档提及不回改

### D-7 · ReflectVerdict 三值决策

- ①锚点：agent-types/src/lib.rs:110-117（+as_str :188-194）
- ②引用：loop.rs :12 import / :94 Event 变体 / :4975-5054（do_reflect 内 emit+match）；planner lib.rs:230/:495（trait 签名）；agent-runtime/session.rs:1064-1066（as_str 转换）；api/codex-cli/service 仅 String 投影（不引类型）；测试 ~60 处（MockPlanner）
- ③实测：loop.rs 生产侧 ≈ 20 行（事件变体+do_reflect 内消费）；agent-types ≈ 12 行；planner 反射签名随 D-3
- ④修正减量：**≈35（loop.rs 侧）**；跨 crate 行数不计 loop.rs 硬线
- ⑤A 臂触达：do_reflect A 臂正常路径不可达；**唯一活入口 = do_act 审批拒绝 :4001（next=Reflect，无 single_loop 门控）**——A 臂审批拒绝时 planner.reflect 真调 LLM。处置：:4001 改指 Done（审批拒绝 = 终止语义，A 臂更符合"护栏触发≠继续规划"）或加 single_loop 短路——**手术时单列小改**
- ⑥风险：Event::Reflection 下游（session/api/codex-cli render/report）String 消费链在 A 臂本就静默；删枚举需同步 planner trait 签名——与 D-3 联动

### D-5 · LoopPhase 枚举 + step() match —— **外部"最后删"判定维持，且不可全删**

- ①锚点：枚举 :26-36（7 变体）；step() :5304-5390（旧 :5071-5160 漂移 +233）
- ②引用：Event::Phase emit 6 处（:2942/:3825/:4404/:4751/:5308/:5383）；消费端新坐标 = **agent-runtime/session.rs:1000-1004**（map_event 唯一类型转换点；旧 :1002 漂移 +2）+ **api/src/lib.rs:14-16**（契约定义，api 不 import LoopPhase；旧 :265 已漂进测试区 :263-271）+ 借道投递 session.rs:1070-1076；下游渲染 run_local.rs:970/note.rs:184/service sse.rs/observer lib.rs 219-278
- ③实测：枚举 11 行 + step() 87 行 + Observe/Reflect 臂内 do_* 分派 ~10 行
- ④修正减量：**≈50**（**收窄版**——原估 120 假设全删不成立）
- ⑤A 臂触达：**A 臂现役变体 = Init/Plan/Act/Done/Error 五个**（R8-A1 空轮终止 :3594、审批拒绝 :3943、FA01 :5294 均产 Error；step 骨架 + run() :5885/:6122/:6155-6157 终态消费全现役）→ **仅 Observe/Reflect 两变体 B 专可删**（连带 2 个 emit 点 + step 两臂 + 事件消费端保留——A 臂 Phase 事件仍发）
- ⑥风险：agent-core/src/lib.rs:22 pub use 再导出同步收窄；observer/lib.rs:219-278 的 Phase 计数若断言 Observe/Reflect 存在需查（observer 保留清单，只查不改）

### D-3 · planner.decompose() 调用链

- ①锚点：**:2983-3106 块**（`!self.single_loop && (...)` 守卫的 decompose+derive_gaps 块，124 行）；planner crate（1,288 行，独立文件）
- ②引用：loop.rs :17/:1024/:1612（Planner trait 编译期）/ :2984 decompose / :3015 derive_gaps（全仓唯一生产调用）/ :4978 reflect（B 主 + **A 臂审批拒绝旁路 ：4001 可达**）；构造点 4 处（agent-runtime/session.rs:265、codex-cli/repl.rs:307、run_local.rs:186,341）；service Cargo.toml 声明但零使用（死依赖）；测试 :6287+ 约 60 处 MockPlanner
- ③实测：loop.rs 块 124 行 + reflect 残链随 D-6/D-7
- ④修正减量：**≈130（loop.rs 侧）**；planner crate 1,288 行（测试 612）不计 loop.rs 硬线——**整 crate 删除与否留顶层裁**（A 臂审批旁路 :4001 处置后 loop.rs 运行时依赖归零，但 Planner trait 字段/构造器编译期依赖仍在——要么保留 crate 退化存在，要么连字段重构（4 构造点波及 agent-runtime/codex-cli））
- ⑤A 臂触达：decompose/derive_gaps 零触达（:2983 守卫 + run() :5497 强制 needs_decompose=false）；reflect 经 :4001 旁路可达（同 D-7 处置）
- ⑥风险：**外部影响面低估实证**——planner crate 47.5% 是测试；整删 crate 波及 agent-runtime/codex-cli 构造链；**建议手术范围限定 loop.rs 侧**（删 ：2983-3106），crate 处置另立小包申报

### D-4 · task_graph / TaskGraph / plan_state

- ①锚点：字段 :1063/:1065；checkpoint 挂钩（**R6-8 保留项，先解绑再删**）：TurnCheckpoint 结构 :957-965（:964 task_graph 字段）、task_graph_value :1505-1508、restore_task_graph :1510-1524（**:1519 写 plan_state.task_graph = 解绑点**；:1521 needs_decompose=false 连锁）、发射点 :2896-2907（record_tool_exchange 内）、run_local.rs:200-237（session_store 三落盘）+ :382 挂接
- ②引用：task_graph 读写 ~15 处（:1666/:1894/:2988-3015/:3058/:3109 等，主体在 ：2983-3106 B 专块内随 D-3 消失）；plan_state 26 处（生产 21 + 测试 5）；**plan_state 跨 crate：planner reflect 签名 ：230/:495（随 D-3/D-7）**
- ③实测：loop.rs 侧 ≈ 180 行（字段+读写+拓扑块 ：3127 空图短路+all_done 机制 ：3220-3246 前段）
- ④修正减量：**≈180**（原估 400 过高——大量引用集中在 D-3 已删块内，不重复计）
- ⑤A 臂触达：checkpoint 链全现役保留；task_graph 在 A 臂恒空图（:3127 拓扑 None 短路）；plan_state.replan_count :5488 重置 A 臂执行但恒 0
- ⑥风险：**checkpoint 载荷结构变化**（TurnCheckpoint.task_graph 字段删否）——R6-8 保留语义 = "checkpoint 载荷含 taskgoal+task_graph"；若 D-4 删图，checkpoint 载荷同步缩为 turns+taskgoal（**需顶层确认载荷缩编不算违反 R6-8 保留语义**——预研判定：保留项本意是 checkpoint 机制不死，载荷随数据源消失属自然收窄，申报待批）

### D-6 · do_observe() / do_reflect()

- ①锚点：do_observe **:4403-4749**（347 行）；do_reflect **:4750-5302**（553 行）；合计 ≈ **900 行**
- ②引用：step() 两臂分派（随 D-5 收窄删除）；:4001 审批旁路入口（改指 Done）；R5-3 B 臂计数（:4782/:4790/:4904——**A 臂侧已由 a_arm_act_tally 接管**）；Event::Observe 判定投影 :4509（保留清单"事件流"——随相位消亡，observer 侧对应投影同步失效，申报）
- ③实测：900 行
- ④修正减量：**≈900**（原估 500 过低——实测两函数加起来 900）
- ⑤A 臂触达：零（唯一入边 = :4255 Act 尾 B 臂分支 + :4001 旁路 + :4697 observe 尾自环——前两处置后全断）
- ⑥风险：error_kind 归类缺口随删（R8-A2 遗留已登记，接受）；B4 同名异构随之消解（任务书本意）

### D-9 · 计数器残留（签 3 收窄版）

- ①锚点与逐计数器判定（**A 臂触达实证**）：

| 计数器 | 维护点 | 消费点 | A 臂判定 | 处置 |
|---|---|---|---|---|
| steps_without_progress :1072 | **a_arm_act_tally :4300-4338（R8 新增，A 现役）**+ B 臂 do_reflect :4826 | build_messages/观察 | **A 现役** | **保留**（签 3） |
| same_tool_repeat :1164 | **a_arm_act_tally（A 现役）**+ B 臂 :4779 | strategy-forced 注入 | **A 现役** | **保留**（签 3） |
| stuck_loop/search_streak :1076-1080 | do_act :4194-4225（**A 臂可达**，硬停旁路但置位+注入仍生效） | build_messages :2787 stuck-breaker 注入 | **A 现役** | **保留 + 冲突申报**（不在签 3 点名清单，按增 3 元原则实测优先——属探索收敛机制族） |
| verification_evidence :1147 | do_act :2885（bash 命令，A 可达）| verification_state 终态投影 :1774 | **A 现役** | **保留**（诚实性） |
| verify_replan_count :1149 | run :5400 重置；Done 相位 ：5898-5909 | Done 验证 replan | **A 现役**（A 臂 Done 可达） | **保留** |
| consecutive_errors :1067 | **仅 do_reflect :4782/:4904 维护（B 专）** | :3349 经验注入门（**A 臂走但恒 false 死分支**）/:2303/:2312/:2356/:4790 | A 臂=死分支 | **删 B 侧**（含 :3349 门分支——A 臂行为零变化；experience_store 机制本身保留） |
| needs_decompose :1104 | :1685/:5492 初始（B 语义）+ B 侧 8 处置位 | :2983 B 门（A 臂恒 false） | A 臂恒 false | **删**（随 D-3/D-8） |
| write_attempted :1121 | :3841 置位（A 可达但**只写不读**）+ :5502 重置 | :3240/:3713 B 侧两门 | A 臂无行为 | **删**（随 D-8；置位行同步删） |
| act_verify_replan_count :1152 | :3246-3270（B 专） | all_done 门 | B 专 | **删** |
| plan_state.replan_count | :5488 重置（A 执行恒 0）+ B 侧写 | reflect 入参/B 臂 replan 边界 | B 专（除死重置） | **随 D-4 删** |
| empty_turn_active/streak、progress_nudged、budget_warned_50/80、fa01_budget_intercepted | R8 新增 | — | **A 现役** | **保留**（签 3） |

- ②引用：见上表（全 :line 实测）
- ③实测：B 侧残留合计 ≈ **110 行**（consecutive_errors 全链 ~25 + needs_decompose ~30 + write_attempted ~25 + act_verify_replan_count ~15 + plan_state.replan_count ~15）
- ④修正减量：**≈110**（收窄版，原估 500 依签 3 大幅收窄——含 a_arm_act_tally 60 行豁免 + stuck 族 40 行冲突豁免）
- ⑤A 臂触达：逐个已判（上表）；**冲突申报两笔：stuck 族现役保留、consecutive_errors 死分支删除不影响 A 臂**
- ⑥风险：consecutive_errors 经验注入门 ：3349 删除后 experience_store 在 A 臂完全失活——若顶层认为 A 臂应有经验注入，需另行设计（B 臂维护者已死，注入条件永假，现状=A 臂本就无经验注入）

## 三、修正减量合计 + 达标三态预判

| 项 | 修正减量（loop.rs） | 对照顶层估算 |
|---|---|---|
| D-10 | 40 | 60 ↓ |
| D-8 | 120 | 107 ↑ |
| D-1 | 270 | 140 ↑（含测试 120） |
| D-2 | 200 | 70 ↑（含测试） |
| D-7 | 35 | 80 ↓（跨 crate 不计硬线） |
| D-5 | 50 | 120 ↓（R8-A1 依赖收窄） |
| D-3 | 130 | 300 ↓（crate 不计硬线） |
| D-4 | 180 | 400 ↓（与 D-3 去重） |
| D-6 | 900 | 500 ↑（实测 900） |
| D-9 | 110 | 收窄版 ≈150 ↓ |
| **生产合计** | **≈2,035** | 顶层估算 1,850-1,900 |

**关键修正（顶层算术遗漏项）**：B 臂测试同步删除（施工闸 3 明文，loop.rs 测试区 ：6287-12533 = **6,247 行**在内）：

| 测试类别 | 预估函数数 | 预估行数 |
|---|---|---|
| D-1 路由测试（goal_requires_product/node02） | 5 | ~120 |
| D-2 分类测试 | 3 | ~80 |
| D-6/D-7 observe/reflect/verdict 类 | ~15 | ~600 |
| D-3/D-8 decompose/v22/all_done/replan 流程类（MockPlanner 驱动） | ~20 | ~900 |
| GiveUp 链（R6-1/R6-2/RC47/RC52/FA01——**逐个判定**：GIVE_UP_CONFIRMED :5294 在 do_act 尾 A 臂可达，相关测试不能盲删） | ~8 | ~350 |
| MockPlanner fixture/构造点改写（A 臂测试 60+ 处仍需 fixture——若 D-3 删字段则改写非删除） | — | 改写 ~300（不计减量） |
| **测试合计（中位）** | ~51 | **≈2,050** |

**总预估 = 2,035（生产）+ 2,050（测试）≈ 4,085 行 → 13,055 − 4,085 ≈ 8,970**

### 达标三态预判：**预判达标（<10,500）**——前提"含 B 臂测试同步删除"（施工闸 3 本就要求）

对照增 1 三裁定：预判 `wc -l` ≈ 8,970 < 10,500（硬线内，不触重基线评审带）；若测试删除不及预估（GiveUp 链大量保留 + fixture 改写），下界 13,055−2,035−800 ≈ 10,220 仍 <10,500；仅当"测试删除 <800 行且生产不及预估"才可能落评审带——概率低。**修正合计仍 <2,558（生产侧 2,035）——顶层增 1 的算术诚实判断在生产侧成立，达标依赖测试同步删除，与施工闸 3 语义一致。**

## 四、B 臂测试删除清单预编（禁改断言，预期内删除）

| D 项 | 死亡测试（新行号） | 判死理由 |
|---|---|---|
| D-1 | test_goal_requires_product_classification :8133 / test_w8_a1_routing_discourse_to_qa :8158 / test_w8_a1_routing_product_action_verbs :8180 / test_w8_a1_routing_defaults_and_mixed :8202 / test_node02_intent_constraint_separation :10279 | 被测函数删除 |
| D-2 | test_inv_a_taskcontrol_and_question_never_goal_mutation :10104 / test_w8_a2_classify_user_input_t5_regression :10315 / test_w8_a2_task_control_input_preserves_goal :10340 | 同上 |
| D-6/D-7 | test_p2_a4_observe_consumes_lsp / test_r13_observe_writes_verdict_scratch / test_r13_observe_flags_dead_artifact / test_r511_qa_zero_write_skips_reflect_llm / test_w8_a4_drift_verdict_parsing 及 do_reflect 三臂 match 直测 | 相位删除 |
| D-3/D-8 | test_p3_stuck_replans_or_gives_up / test_chat_qa_no_forced_replan / v22 写文件闸测试（:8097 注释锚）/ all_done 门测试 / MockPlanner replan 流程类 | 机关删除 |
| GiveUp 链 | test_node05_case_d / test_fa01_f9 / test_r61_* / test_rc47_* / test_rc52_* | **逐个判定**：经 MockPlanner GiveUp 驱动 B 臂流程者死；:5294 GIVE_UP_CONFIRMED A 臂可达路径相关断言保留改写 |

手术时每项施工闸第 1 条全仓 grep 会产出精确到行的最终清单（本表为预编基线，±10%）。

## 五、疑似冻结项预置申报（签 4 材料格式）

| 候选 | 引用清单要点 | 冻结理由预判 |
|---|---|---|
| planner crate（D-3 关联） | loop.rs:17/1024/1612 类型依赖 + 4 构造点（agent-runtime/codex-cli） | 整删波及 3 crate 构造链，超 loop.rs 手术边界 |
| Event::Reflection 下游（D-7 关联） | session.rs:1064 → api lib.rs:37/308 → codex-cli render/report/service sse | String 投影链 4 crate，枚举删除需同步契约（api 契约变更=外部接口变更） |
| experience_store 经验注入门（D-9 关联） | :3349 门删后 store 在 A 臂失活但结构保留 | 机制去留属产品决策非手术范围 |

预判：**冻结非必需**——三项均可按"loop.rs 侧删除 + 跨 crate 申报"处理；若顶层倾向最小爆炸半径，上述三项按签 4 逐项冻结（冻结不豁免 wc -l，三项合计在 loop.rs 内行数贡献 <100，冻结代价可控）。

## 六、冲突申报（增 3 元原则）

1. **stuck_loop/search_streak（D-9 候选）**：不在签 3 点名清单，但 do_act :4194-4225 维护 + build_messages :2787 注入均在 A 臂可达且有行为 → **裁定建议：保留**（探索收敛机制族，同 a_arm_act_tally 逻辑）。删除预估已按保留口径修正。
2. **UserInputKind :5412（D-2）**：A 臂执行且有防护语义（TaskControl 不动 goal）→ 删除需行为等价改写（直通 GoalMutation）——**预判改写删除**（C2 误判病灶正是它），但 A 臂防护语义损失需顶层知情。
3. **checkpoint 载荷 task_graph 字段（D-4）**：R6-8 保留语义="机制不死"，载荷缩编（turns+taskgoal）预判不违反本意——**申报待批**。
4. **consecutive_errors 经验注入门 ：3349（D-9）**：A 臂恒 false 死分支删除——A 臂现状本就无经验注入，删除零行为变化，如实申报。

## 七、Event::Phase 消费端 + checkpoint 挂钩点新坐标（委托书 §六-7）

| 项 | 旧坐标（96f4161） | 新坐标（13,055 基线） |
|---|---|---|
| Event::Phase 消费① | session.rs:1002 | **session.rs:1000-1004**（map_event；kind 表 :1083-1085） |
| Event::Phase 消费② | api/lib.rs:265 | **api/lib.rs:14-16**（契约定义；:265 已漂进测试区 :263-271） |
| checkpoint 载荷结构 | — | TurnCheckpoint :957-965 |
| checkpoint 回调挂接 | — | on_turn_checkpoint :1133-1137 / set_on_turn_checkpoint :1761-1764 / 发射点 :2896-2907 / run_local.rs:200-237+382 |
| plan_state 解绑点（D-4 前置） | — | **restore_task_graph 内 :1519**（唯一写 plan_state.task_graph 处） |
| emit_think_summary 保留红线 | — | :5558-5564/:5576-5582（R8 预算投影，勿删） |

## 八、预研结论

1. 十项重锚完成，坐标全部刷新至 13,055 基线（外部 1429b58 坐标平均漂移 +230~540，直接照搬必错）。
2. **修正减量：生产 ≈2,035 + 测试 ≈2,050 ≈ 4,085 → 预判达标**（13,055−4,085 ≈ 8,970 < 10,500）；生产侧单项合计 2,035 < 2,558 与顶层增 1 判断一致——**达标依赖测试同步删除（施工闸 3 本意），非额外放水**。
3. 冲突申报 4 笔（§六），其中 stuck 族保留与 checkpoint 载荷缩编需顶层表态。
4. 冻结非必需（§五），最小爆炸半径方案可选项。
5. 触发未齐（包B/闸 3），loop.rs 零改动；本报告归档，闸门齐即按 D-10→D-8→D-1→D-2→D-7→D-5→D-3→D-4→D-6→D-9 顺序开刀。

---
*traecode 执行窗 · 2026-09-08 · 全部坐标 13,055 基线实测（rg -n + ReadAllLines）；引用面经 3 路并行扫描 + 签 3 关键项人工复核*
