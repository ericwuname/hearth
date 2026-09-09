# Failure Adaptation Flow — Node 01 生命周期源码审计（P1-FAILURE-ADAPTATION-01）

日期：2026-08-30　基线：v0.2.14（`2fe0688`）　证据等级：全部锚点本窗实测（confirmed）

> 本文是审计交付物，不改生产控制流。行号以 v0.2.14 源码为准（`grep` 现场复核过，
> 非旧报告复制）。

---

## 1. Failure 全生命周期控制流（真实锚点）

```text
[T1 产生]  tool-runtime dispatcher
           ├─ bash: exit code / 信号（exit>0 → 断言失败族；负值=信号终止）
           ├─ TaskDeadlineExceeded（P1-LTR：dispatcher 单点 effective=min(declared,remaining)）
           └─ sandbox RT4 fail-closed / seccomp SIGSYS(exit 159)
   → ToolResult.is_error=true 装载进 pending_results（agent-core/loop.rs）

[T2 观察]  do_reflect（loop.rs:3709 起）
   → has_error = pending_results.any(is_error)        （loop.rs:3739）

[T3 计数]  loop.rs:3740-3779
   ├─ 有错: consecutive_errors += 1; steps_without_progress += 1
   │        （param_missing 且 ≥2 → force_param_gap，loop.rs:3749-3751）
   └─ 无错: consecutive_errors = 0
            fact_progress = 本轮 write_file/apply_patch 成功（loop.rs:3759-3768，唯一 progress 口径）
            any_completed = 图内有 Completed 节点
            (!any_completed && !fact_progress) → steps_without_progress += 1
            否则 → steps_without_progress = 0

[T4 决策]  Observation 投影（loop.rs:3793-3809）→ planner.reflect
   快路径（planner/lib.rs，规则臂不触 LLM）：
   ├─ consecutive_errors >= MAX_CONSECUTIVE_ERRORS → GiveUp   （lib.rs:479-485）
   ├─ budget_remaining == 0                        → GiveUp   （lib.rs:489-491）
   ├─ budget_fraction <= 0.15 && noprogress >= 2   → GiveUp   （lib.rs:495-502）
   ├─ noprogress >= MAX_STEPS_WITHOUT_PROGRESS(5)  → Replan（cap replan_count<3，超限 GiveUp，lib.rs:506-521）
   └─ 其余 → LLM reflect（verdict 文本 give_up/replan/continue，replan 同样 cap<3，lib.rs:628-643）
   另有两条旁路 GiveUp：
   ├─ nervous system Abandon/DeliverAndQuit（loop.rs:3832-3844）
   └─ graph 停滞：相邻两次 decompose 同图（多节点）×2 → do_plan 返回 Err → Error 相位
      （loop.rs:2478-2509，graph_stall_count 计数）

[T5 消费]  do_reflect verdict 三臂（loop.rs:3848-4045）
   ├─ Continue → Plan
   ├─ Replan   → needs_decompose=true; plan_state.replan_count+=1;
   │             steps_without_progress=0（consecutive_errors **不清**——GiveUp 需跨轮累积，loop.rs:3879-3881）
   │             Failed 节点 → Pending
   └─ GiveUp   → 修 D 拦截（loop.rs:3937-4023，见 §3）→ 未拦截则 Error("give_up")

[T6 验证]  Done 相位（loop.rs:4395-4490）
   ├─ written_files 非空 → verify_written_files（存在+非空）缺失→ verify_replan_count<3 回喂 replan；超限 verify_failed
   └─ criteria 非空 → parse+verify_acceptance_criteria（cmd/file 确定性核验）
       passed → acceptance_result="passed"
       failed 且 acceptance_replan_count<1 → Reserve 消耗，回喂 replan
       failed 且 Reserve 尽 → verify_failed 终态

[T7 终态]  Event::Done(ok) / RunReport{ok, summary} / Error 相位（ok=false, error="give_up..."）
```

## 2. 八问直答（谁产生/分类/计数/决定/清零/证明）

| 问 | 答（锚点） |
|---|---|
| 谁产生 failure | tool-runtime dispatcher（exit code/deadline/sandbox）；provider 层 transient（网络/429）不在 ToolResult 内，走 provider 自身重试 |
| 谁给 failure 分类 | **现状：无人消费。** `terminal.rs:243 classify_failure` 七分类纯函数 + `completion_readiness` 已存在且单测锁定，但 **grep 全 crates 零生产调用点**（仅定义+测试，terminal.rs:233-288）——上轮"只做观察接入、retry 差异化属后续单"，即本总包 Node 02/06 要接的口 |
| 谁计数 | loop.rs do_reflect（consecutive_errors/steps_without_progress）、do_plan（graph_stall_count）、Act done-gate 与 Done 相位（verify_replan_count）、Done 相位+GiveUp 拦截（acceptance_replan_count） |
| 谁决定 retry | **不存在显式 retry 决策点。** 最接近的是 Act done-gate 产物缺失回喂（loop.rs:2640-2681，verify_replan_count<1）与 provider 内部 transient 重试 |
| 谁决定 replan | planner.reflect 规则臂+LLM（cap 3），loop.rs:3856-3895 消费 |
| 谁决定 give_up | planner 三条规则臂+LLM verdict+nervous Abandon+graph 停滞 Err；loop 侧只有修 D 拦截能**否决** give_up |
| 谁能清零 counter | 见 §3 逐项表；关键：fresh run 全清（loop.rs:4184-4199）；acceptance_replan_count **无 run 内清零点**（run-level Reserve） |
| 谁能证明 recovery 成功 | 仅三层事实：L1 exit code（cmd 核验）/ L2 产物（verify_written_files）/ L3 验收（verify_acceptance_criteria）。LLM self-report 无任何通道进入完成事实（INV-ED01-G 结构保证） |

## 3. 计数器互相污染审计（Node 01 核心交付）

| 计数器 | 锚点 | 增 | 清零 | 谁消费 | 污染风险 |
|---|---|---|---|---|---|
| consecutive_errors | :909 | reflect 有错步 :3741 | 无错步 :3753；**replan 不清**（:3879 注释明示故意） | planner GiveUp 臂（≥MAX） | 无跨计数器污染；但**无错步立刻清零**——错误与无错交替出现时 GiveUp 臂永不触发（设计使然，防误杀） |
| steps_without_progress | :914 | 有错步 :3742；无错但无写/无完成 :3775 | fact_progress/any_completed :3777；replan :3881 | planner GiveUp 臂（≥2 + budget low）、Replan 臂（≥5） | **最大污染源**：verification 步（cargo test 通过、read、grep）不计 progress——修复成功后复测反而累积"无进展"，直喂 budget-low GiveUp 臂（44 步病灶要素②）。Node 04 专项 |
| verify_replan_count | :953 | Act done-gate :2670（<1）；Done 相位 :4403（<3） | fresh run :4199 | Act/Done 两处 | **同一计数器、两个不同上限**：Act 消耗后 Done 相位只剩 2 次名额——跨相位零和实例（砺批-2 同构问题，此前未登记） |
| acceptance_replan_count | :935 | GiveUp 拦截 :3976；Done 相位 :4458 | **无 run 内清零点** | 修 D 拦截 + Done Reserve（共用） | 砺批-2 已裁决：run-level 共用、有界、跨相位零和——拦截耗掉后 Done 核验失败直接 verify_failed |
| graph_stall_count | :963 | do_plan 同图 :2494 | 图变化 :2507；v20 gate 重入 :3706 | do_plan（≥2 → Err GiveUp） | 独立性最好；误杀已由单节点图豁免（:2493）修复 |
| budget_low 判据 | planner:495 | — | — | fraction = remaining/(used+remaining) | **不读 deadline**——deadline 权威性由 dispatcher 单点保证（P1-LTR），两预算线互不感知（记录事实，Node 11 复核优先级） |

**结论（污染拓扑）**：真正互相污染的是 **steps_without_progress ↔ planner GiveUp/Replan 臂**（progress 口径过窄把验证工作算作停滞）与 **verify_replan_count 跨相位零和**（Act/Done 双上限共用）。acceptance_replan_count 零和已裁决维持。graph_stall_count 与 budget 线干净。

## 4.（v1.1 必补）criteria 为空时 give_up 与 completion 的完整行为路径

**缺口实证（砺批-1 confirmed，本窗复核一致）**：

- GiveUp 拦截以 `if !edd_checks.is_empty()` 开门（**loop.rs:3945**）——criteria 空 →
  拦截块整体跳过，直接落 :4024 原始 give_up（`Error("give_up: cannot make progress")`）。
- Done 相位核验同构守门（**loop.rs:4450** `if !checks.is_empty()`）——criteria 空 →
  不做任何 acceptance 核验。
- 唯一仍生效的保护：`written_files` 非空时的产物存在性校验（:4399）。

**criteria 空 + bash-only Product 任务的全路径推演**：

```text
任务只跑 bash 命令（无 write_file/apply_patch）
→ written_files 恒空 → :4399 产物校验也不触发
→ reflect 无错步：fact_progress=false（progress 口径只认写类）
→ steps_without_progress 一路累积 → 5 步 → Replan×3（每次清零再累积）
→ budget-low 或 replan-cap 臂命中 → GiveUp
→ :3945 守门空转 → 原始 give_up → terminal = Error(ok=false)
```

**定性**：
1. 终态**不是假 completed**（give_up 走 Error，report ok=false）——"伪装成正确终态"
   的风险不存在于主路径；真正的问题是**放弃时零核验证据**：任务可能已经成功
   （bash 验证通过）也可能没成功，系统既不验证也不声明"未验证"。
2. 失败报告（summary.error = "give_up: cannot make progress"）**没有区分**
   "criteria 空所以无法核验"与"criteria 核验失败"两种放弃——F9 要求的
   "明示无 acceptance criteria、放弃未经核验"目前在证据层完全缺失。
3. 上轮 Node 14"STATUS.txt 达标但 shout 未修"假阳性与之同根：criteria 覆盖度
   = 用户责任；系统侧唯一能做的是**把"未核验"这个事实显式暴露**，不替用户补写标准。

**F9 绿线设计方向（供 Node 08 落地，先红后绿）**：criteria 为空 + Product 任务
+ give_up 发生时，(a) 不改变终态语义（仍非 completed），(b) 在 scratch/summary
显式写入 `no_acceptance_criteria: true, giveup_unverified: true` 类结构化标记，
使失败报告可审计。**不引入自动核验替身、不引入 LLM judge（STOP-9 红线）**。

### 4.1 Node 09 真机补遗（2026-08-30 施工中实证）：GiveUp 旁路全景图

Node 09 真机三跑实证：除 §1 主链外，GiveUp 实际有 **5 条产生路径**，修 D 拦截
原本只覆盖其中一条。旁路登记与处置：

| # | GiveUp 路径 | 位置 | FA01 处置 |
|---|---|---|---|
| 1 | planner GiveUp 臂（Reflect 消费） | loop.rs GiveUp 臂 | 修 D 拦截（上轮已落地，本轮复用） |
| 2 | nervous system Abandon | :3832-3844 | 未动（观察级，无真机样本） |
| 3 | **T4 语义停滞 Err** | do_plan stall → Err | **本轮加拦截**（先红后绿：真机 43 步样本 + StallPlanner fixture 正反例） |
| 4 | **WS9 预算耗尽 abort（headless）** | run loop budget 分支 | **本轮加拦截**（真机 50 步样本；passed → 路由 Done，**不延长执行预算**） |
| 5 | **deadline_exceeded abort** | run loop deadline 分支 | **故意不拦截**——§11 Safety > Deadline > Recovery，deadline 权威（INV-FA01-D） |

新增不变式（代码层）：**凡"未核验的放弃"，要么先核验（Reserve ≤1，零和），
要么显式标记 giveup_unverified**——三条路径（1/3/4）全部套用；路径 5 豁免有据；
路径 2 保持观察。真机证据：Node 12 跑中 GiveUp 拦截实际触发
（`VERIFICATION_RESERVE(give_up interception)` → acceptance failed → 诚实 failed，
拒绝假 completed——模型写完 RESULT.txt 标记但修复未落盘，与上轮 Node 14 假阳性同形态，
本次被核验层正确拦下）。

## 5. Node 02 分类落点预判（承接 §2）

- 分类器缺口 = `classify_failure`（7 类）零消费端 + 总包要求 F1-F10 十类。
- 映射：F1 transient_provider↔Transient；F2 tool_execution↔ToolFailure；
  F3 environment↔EnvironmentFailure；F5 assertion↔AssertionFailure；
  F6 verification↔VerificationFailure；F7 plan↔PlanFailure；
  F9 model_judgment↔ModelJudgmentFailure。
- **现有输入无法区分的**：F4 permission/approval（需审批拒绝结构化事实——RC24 有
  审批门记录可挖）、F8 resource/budget（budget_fraction 可派生）。二者均可由
  结构化输入派生，无需 LLM judge（不触 STOP-9）。F10 unknown = 兜底臂。
- 消费端落点候选（Node 06 详设计）：reflect 计数处（:3740）与 give_up 拦截前
  （:3937）——均不改事件契约。

---

## 6. Node 04 — Progress 语义审计（含与修 D 拦截的交互分析）

### 6.1 各动作类型的语义评估（五问制）

| 动作 | 新事实? | 改任务状态? | 减不确定性? | 直接证明完成? | 应影响无进展计数? |
|---|---|---|---|---|---|
| write_file / apply_patch | ✅（产物落盘） | ✅ | ✅ | 部分 | **否（算 progress，现状✅）** |
| bash build | ✅（编译事实） | 否 | ✅ | 否 | 可argue，弱 |
| bash test | ✅（通过=修复证明） | 否 | ✅✅ | **接近** | **争议核心（见 6.2）** |
| bash verification（与 criteria cmd 对应） | ✅ | 否 | ✅✅ | ✅（等价核验） | 争议核心 |
| read / grep / glob / LSP | 可能（定位知识） | 否 | ✅ | 否 | 否（现状合理） |
| acceptance verifier | ✅ | ✅（acceptance_result） | ✅✅ | ✅ | 不经此路径（独立核验） |

### 6.2 现状口径（锚点 loop.rs:3752-3778，砺批-3 修正后新锚点）

`fact_progress` = 本轮 write_file/apply_patch 成功（:3759-3768）。bash test/验证
成功**不计** progress → `steps_without_progress` 累积 → 直喂 planner 两臂：
Replan（≥5）与 budget-low GiveUp（≥2 且 fraction≤0.15）。

### 6.3 与修 D 拦截的交互分析（v1.1 必补，砺批-3）

修 D 拦截（:3937-4023）与 progress 扩展**都改"give_up 前的行为"**，二阶效应清单：

1. **拦截内核验步算 progress → counter 清零 → budget 燃烧节奏改变 → GiveUp 臂
   命中条件漂移**：若扩展语义，拦截后回喂修复+复测的步不再累积 noprogress →
   planner budget-low GiveUp 臂（fraction≤0.15 && noprogress≥2）命中点后移 →
   任务燃烧更多步数才放弃。对成功任务是利好（44 步病灶进一步缓解），对真死任务
   是预算浪费放大器——**无界化风险**（INV-FA01-E），必须配 reserve cap 才可接受。
2. **与 Reserve 零和叠加**：progress 扩展减少 GiveUp 触发 → 更多任务走到 Done
   相位核验 → `acceptance_replan_count`（run-level 单 Reserve）消耗更频繁 →
   拦截与 Done 相位的零和竞争加剧（砺批-2 事实）。
3. **与 graph_stall_count 无交互**（T4 看图签名，不看步计数）——已排除。

### 6.4 裁决（本轮）

**progress 生产语义维持现状不改**，依据：

- 修 D 拦截已在 give_up 决策点做了等价补偿（"无验证不放弃+已验证不否决"），
  且**真机已实证**（上轮 mathlib 46 步 GIVE_UP_OVERRIDDEN → completed）——
  同一病灶不需要两套未标定的补偿叠加；
- 扩展 progress = 控制流变更 + budget 燃烧节奏漂移（6.3-1），按修 A 备而未用
  先例与复-3 裁决 2，**须顶层批准后启用**，执行窗口不擅自改；
- 本总包边界（§2）允许审计 progress semantics，但"最小改动+已有真机证据"优先。

**回归保障**：上轮 Case D fixture（GIVE_UP_OVERRIDDEN 路径）随 gate 全量跑，
Node 15 再做真机样本复核（Node 12 承担）。若未来顶层批准扩展，6.3 清单即为
必测二阶效应 checklist。
