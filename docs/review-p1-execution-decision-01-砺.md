# P1-EXECUTION-DECISION-01 长程施工总包 v1 · 守门人评审

> 评审窗口：**砺·评审**（🪨 磨刀石，只磨不生产）
> 评审对象：`docs/Hearth P1-EXECUTION-DECISION-01 长程施工总包 v1.md`（16 Node）、`docs/Hearth Roadmap-chatgpt.md`
> 评审基线：**`ff4eb2f`**（v0.2.13，08-30 09:19）
> 评审时间：2026-08-30 13:5x
> 方法：**不信报告信源码**——所有判定均带源码行号，注释一律不作为证据

---

## 0. 裁决

| 项 | 结论 |
|---|---|
| **总评** | 🟡 **有条件通过 —— 建议补 3 条条款后放行**，无需回炉重设计 |
| **方向判断** | ✅ 正确。把问题从「Completion Truth」推进到「Execution Decision Truth」是对的路；INV-01/INV-02 两条不变量站得住 |
| **必须补** | **C-1** 常量标定纳入范围（见 §3）、**C-2** 指派 α 分类器修复的归属 Node（见 §4）、**C-3** 总包自身执行协议回避「不得自审」（见 §7） |
| **强烈建议** | Node 09 复用样本前先重判上游归因（§2），否则验收判据可能建在错误前提上 |
| **新发现** | **RC44**（acceptance 失败态被投影为 pending）——直接威胁 Node 02/Node 08 的数据基础，见 §5 |

**一句话**：这份总包的**问题定义是对的，控制流设计是专业的，但它正在用一套自己违反 INV-02 的流程，去修一个它自己没完全定位到的根因，而那个根因（常量标定）被它自己的排除清单挡在了门外。**

---

## 1. 基线核实（先纠一行）

| 项 | 任务书自述 | 实测 | 判定 |
|---|---|---|---|
| HEAD | `c14fac0` | **`ff4eb2f`** | ⚠️ 已落后 1 个提交（`ff4eb2f` = P1-TASK-TRUTH-01 Final Report，docs only，不影响代码） |
| 版本 | v0.2.13 | v0.2.13 | ✅ |
| 工作区 | — | 有 3 个 modified + 若干 untracked（含本总包与 Roadmap） | Node 00 会捕获，无需顶层干预 |

上游状态（R2-C / W3W4 / RC24 / W8 / P1-LTR / P1-TASK-TRUTH = CLOSED）**属实**，与 git log 一致。**唯一需留意**：`P1-CONSOLIDATION-01 = PASS WITH DEVIATIONS` 的偏差**未闭合即被当作上游背景**，见 §2。

---

## 2. ⭐ 核心发现：总包要找的那个根因，我已经定位到了——而且它不在任务书假设的地方

总包 §0 描述的核心故障：

> 修复成功 + 真实 test 通过 + RESULT 正确 + acceptance verification 通过 → planner budget-low / give_up → 最终 task failed

**机械根因（源码实证）**：

```rust
// crates/planner/src/lib.rs:495-502
let budget_fraction = obs.budget_remaining as f64 / total_budget.max(1) as f64;
if budget_fraction <= BUDGET_LOW_THRESHOLD && obs.steps_without_progress >= 2 {
    return Ok(ReflectVerdict::GiveUp);          // ← 直接放弃，不看任何验收事实
}
```

配套常量：

```rust
// crates/planner/src/lib.rs:233-235
const MAX_CONSECUTIVE_ERRORS: u32 = 3;
const MAX_STEPS_WITHOUT_PROGRESS: u32 = 5;
const BUDGET_LOW_THRESHOLD: f64 = 0.15;        // 15% remaining
```

以及 progress 的唯一定义：

```rust
// crates/agent-core/src/loop.rs:3676-3685
// 只有 write_file / apply_patch 成功才算 progress
.filter(|tc| tc.name == "write_file" || tc.name == "apply_patch")
// 注释原文：read/glob/grep/bash（pwd/ls/重复定位类）成功**不算** progress
```

### 三个叠加事实，构成确定性陷阱

1. **低预算臂的阈值是 `>= 2`，不是 5。** 预算剩余 ≤15% 时，**连续 2 步没有写盘动作就放弃**。
2. **验证步骤永远不写盘。** 跑 `cargo test`、跑验收命令、读报告——全是 bash/read，**一步都不计入 progress**。
3. **默认预算 40**（`codex-cli/src/lib.rs:65`，注释自述「默认 20→40——真机几乎每个任务 budget low/exhausted」）。Node 09 用 budget=50 → 剩余 ≤7 步时进入低预算臂。

**合起来就是总包描述的那个场景的精确复现**：
任务在第 ~40 步完成实质工作 → 第 41–43 步做验证 → 验证不产生写盘 → `steps_without_progress` 累加到 2 → 预算恰好回落至 15% 以下 → **`GiveUp` 无条件触发，`acceptance = passed` 完全没被咨询**。

### 由此产生一个必须提醒的连带判断

**`grep -n "acceptance" crates/planner/src/lib.rs` = 0 命中。**

planner 在做出 GiveUp 决策的那一刻，**根本看不到验收事实**——验收结果只存在于 `ctx_mgr` 的 scratch（`loop.rs:4264`）。所以 Node 02/Node 05 想实现「acceptance = passed 必须能表达已具备结束条件」，**必须先解决一个 plumbing 问题**：

> 是把 acceptance 事实喂进 planner 的观察结构（**风险：可能触发 STOP-5「现有模型不足以表达」或 STOP-6「需新增 Event schema」**），
> 还是在 `loop.rs` 里于 planner 返回后拦截 GiveUp（**不碰 schema，但决策权归属变复杂，与 Node 03 权力矩阵耦合**）。

**这是一个设计分叉，且两条路都可能撞 STOP 条件。建议顶层在放行前就把它作为 Node 05 的预先裁决项**，否则执行窗口会在 Node 05 烧掉数个 Node 后才暴露。

---

## 3. 🔴 C-1（必须补）：排除清单挡住了本包自己的核心解药

总包 §2「不允许处理」含：

```
Memory / Compaction architecture
```

**这条写得过宽，会产生误伤。** 被挡在门外的其实是两类东西：

| 类别 | 例子 | 是否该排除 |
|---|---|---|
| 架构重建 | 压缩架构、ContextBuilder 架构、Event schema break | ✅ 应排除 |
| **常量标定** | `BUDGET_LOW_THRESHOLD=0.15`、`MAX_STEPS_WITHOUT_PROGRESS=5`、budget 默认 40 | ❌ **不应排除——这正是本包的核心病根** |

注意 planner 的三个常量（`3 / 5 / 0.15`）**全部是任务类型无关的扁平常数**，与我此前登记的 **R-δ（常量标定失效）** 同源。且 `loop.rs:65` 的注释已经自证了这个病的行为史：

> 默认 20→40——真机几乎每个任务 budget low/exhausted

**这是典型的「改症状不改病因」**：预算不够就把上限翻倍，而没有修「为什么验证步骤拿不到 progress 记分」。总包 §9.2 自己写了正确的原则——

> 不能通过增加总 budget cap 来「解决」问题

——**这条原则是对的，但只对了一半**：真正的解不是调 cap，而是**让 progress 能识别验证类进展**。而 progress 的定义（`loop.rs:3676`）就躺在本包「允许处理」清单的第一行（TaskGraph progress semantics）里。

### 建议补条款 C-1（可直接贴入 §2）

```text
明确纳入范围：
  执行决策相关的阈值与上限常量标定
  ——包括但不限于 BUDGET_LOW_THRESHOLD、MAX_STEPS_WITHOUT_PROGRESS、
    steps_without_progress 的记分口径、budget 默认值。
  本项属于"参数标定"，不属于"架构重建"，
  不受 §2 排除清单中 Memory / Compaction architecture 条款约束。
```

**成本极低，且是 §9.2「不许靠加预算解决」能真正落地的前提。**

---

## 4. 🔴 C-2（必须补）：α 分类器缺口在本包里没有归属 Node

我此前登记的 **R-α（意图层缺失）** 在 v0.2.13 **执行侧已治、目标侧未治**，需要精确区分——这两件事容易被混为一谈：

| 机制 | 位置 | 作用 | 现状 |
|---|---|---|---|
| `goal_requires_product` | `loop.rs:167`（三态漏斗） | **决定执行路径**（进 TaskGraph 还是 QA 直答） | ✅ 已能识别「问句标记/论述动词/论述要求词」（V-1 十轮 1/10 修复） |
| `classify_user_input` | `loop.rs:324` | **决定是否产生 goal_revision** | ❌ 仍只有 TASK_CONTROL(14 词) + CHITCHAT(8 词，且限 ≤12 字)，**其余一律 GoalMutation** |

两处注释（`loop.rs:156`、`loop.rs:307-310`）明确写着二者「正交、禁止两套打架」——**设计意图是对的，分工也是清楚的**。问题在于**后者没有疑问句类别**：

```rust
// crates/agent-core/src/loop.rs:376-377
// 其余 → Goal Mutation（歧义默认保持现状语义）
UserInputKind::GoalMutation
```

后果（实证）：用户 8 次追问「你还记得我的第一个问题吗」→ 8 次全部产生 `目标已修订（revision N）`、0 次被当作问题回答；单会话 revision 峰值 **71**。

### 本包的覆盖情况

- INV-01「先分类，再行动」→ 覆盖的是**执行侧**（且 W8 已落地）
- Node 07「Routing / W8-1 边界复核」→ 用词是**复核**，输出是测试，**没有修复动作**
- INV-A「TaskControl 不得增加 goal revision」→ 是不变量，**不是修复方案**

**结论：RC36 / RC33（追问被吞 + revision 爆炸）在本包中无归属 Node。** 执行窗口会自然地只做 INV-A 的测试，而缺口依旧。

### 建议补条款 C-2（可直接贴入 Node 07）

```text
Node 07 除复核外，须完成：
  classify_user_input（loop.rs:324）增加疑问句类别
  ——最小实现：问句标点/疑问词命中 → UserInputKind::Conversation
    （不动 goal、不 revision++），纯规则、零 LLM，遵守成本红线。
  验收：连续 3 次纯疑问输入不得产生任何 GoalChanged 事件，
        且必须给出针对该问题的回答（A-36）。
  与 goal_requires_product 的关系维持"正交"，
  不得合并为一套分类器（loop.rs:156 既有约束）。
```

---

## 5. 🆕 RC44（新发现）：acceptance 失败态在投影层被吞成 pending

**这直接威胁 Node 02 与 Node 08 的数据基础，建议放行前知悉。**

生产端（`loop.rs:4275-4288`）写入两种形态：

```rust
json!("passed")                                        // 通过
json!({"status": "failed", "failures": failures})      // 失败（object）
```

投影端（`loop.rs:1615-1632`）：

```rust
match get_scratch("acceptance_result").and_then(|v| v.as_str() ...) {
    Some(s) if s == "passed"  => "passed",
    Some(s) if s == "pending" => "pending",
    _ => "pending",        // ← object（即 failed）落到这里
}
```

**失败态被折叠成 `pending`。** 后果：任何上层消费者（报告、未来的 Completion Readiness）**无法区分「尚未验证」与「已验证且失败」**。

对总包的具体影响：

- **Node 02** 设计的四态（`NOT_READY / READY_FOR_COMPLETION / REQUIRES_VERIFICATION / CONFLICTED`）需要区分 pending 与 failed，**当前数据源是有损的**
- **Node 08** 要求验证「LLM says PASSED 实际 test FAILED 时不得误判完成」——**该场景的失败态在状态投影层不可见**，测的将是另一条路径

| 项 | 内容 |
|---|---|
| **RC44** | acceptance 失败态被投影为 pending（有损投影） |
| 严重度 | 🟡 P1（终态本身正确——`verify_failed` 事件已正确发出；受损的是状态投影层） |
| 归属 | 建议并入 Node 02 前置修复，或单列 |
| 判据 | `acceptance_result = {status:failed}` 时，`acceptance_verification_status()` 必须返回 `"failed"`，且需单测锁定 |

---

## 6. 决策包纳入对照（我上一轮给顶层的 5 条根因，纳得怎么样）

| 我提交的根因 | 纳入情况 | 落点 |
|---|---|---|
| **R-α 意图层缺失** | 🟡 **部分** | INV-01 + Node 07；**执行侧已治、目标侧无归属 Node**（§4） |
| **R-β 语义层错位** | ✅ **充分** | Node 01 审计 + Node 02 + Node 05 矩阵（progress 语义正是本包第一行允许项） |
| **R-γ 不得自审** | ✅ **充分且拔高** | INV-02 + Node 03 权力矩阵 + Node 08 + INV-B/G；比我原来的提法更完整（区分了 Statement/Evidence/Verification/Decision） |
| **R-δ 常量标定失效** | 🔴 **未纳入，且被排除清单误伤** | §3 |
| **R-ε 边界/环境** | ⚪ 明确排除（合理） | Sandbox/Seccomp/Landlock 不在本包范围 |
| 适应度函数（治理执行层） | ✅ **充分** | Node 12 INV-A~H；**连「先建 baseline 再只阻断新增」这条都原文采纳** |
| 事故学习系统（反馈层） | 🟡 部分 | Node 04 冲突分类 + 证据窗已近似；缺跨事件模式识别与前馈回写 |
| L2/L3 评估栈 / 模拟 persona / 摩擦遥测 | 🔴 未纳入 | 不在本包范围，建议留待 W12，不阻塞放行 |
| 正确弃权率指标 | 🟡 定性采纳 | Node 20 B；**无量化指标**，建议补一个可测口径 |

**评价**：ChatGPT 抓的两条（α 执行侧 + 不得自审）**抓得准，而且 INV-02 的表述比我原来的更严谨**。漏的两条（R-δ、α 目标侧）恰好是**成本最低、收益最大**的两项——这与它「先审计再动手、不预设答案」的方法论有关，属于信息不对称而非判断失误，补上即可。

---

## 7. 🔴 C-3（必须补）：总包自身的执行协议，违反了它自己要立的 INV-02

这是本轮评审里**最该被顶层看到的一条**。

**产品侧不变量（INV-02）**：
> 生产执行系统不能仅凭自己的自述宣布自身最终正确。

**本包的施工协议（§3、§21、§24）**：
> 一次授权，连续执行；**Node 内部自测、自修、自验收**；Node 间不等待顶层反馈；最终一次性提交 Final Report；结论只能是 `PASS / PASS WITH DEVIATIONS / STOP`——**由执行者自己判定**。

**一套连续 16 个 Node、中途零外部校验、终局由执行者自评的流程，正是 INV-02 所禁止的形态。**

这不是吹毛求疵。我此前登记的证据：**R2-C 那一轮 6 个新测试里有 3 个因写法错误而失败**——那正是「同一个模型既写实现又写测试，bug 和自己达成了一致」的实证。本包 §5 已经要求「先红后绿」，能挡掉一部分，但挡不住**终局自评**这一层。

### 建议补条款 C-3（保留连续执行的效率，只加可独立复核性）

```text
连续执行不变（效率目标成立），但每个 Node 必须提交可被第三方独立复核的证据，
而非结论：

  1. 每个 Node 的 gate 日志含真实 exit code 与测试计数（baseline/added/removed/ignored）
  2. Node 02/05/08 三个决策类 Node，除自测外须附"反例用例"：
     构造一个应当 NOT 通过的输入，确认系统确实拒绝（证明判据真的能失败）
  3. Final Report 的 PASS/PASS WITH DEVIATIONS 结论，
     必须逐项指向上述可复核证据条目，不得只有结论性描述
  4. 评审窗口（砺·评审）在 Final Report 提交后做后置独立审计，
     不阻塞施工，但审计结论进总账并决定是否需要回补
```

**要点：不要求中途停下等人（那样会拖垮长程执行），只要求「留下能被人验证的痕迹」。** 这样既保住效率，又不让整个 16 Node 的结论悬在单一自评上。

---

## 8. Roadmap（64 行 ASCII）评审

```
① INTENT → ② PLAN → ③ EXECUTION → ④ FACT → ⑤ VERIFICATION
→ ⑥ REFLECTION → ⑦ DECISION → ⑧ TERMINAL → ⑨ PROJECTION
外围：Memory/Compaction · Resource Governance · Observer · Telemetry · Experience
```

| 项 | 判定 |
|---|---|
| **INTENT 置于 ①** | ✅ 正确——与我提交的「α 是总开关」一致 |
| **FACT(④) 与 VERIFICATION(⑤) 分离** | ✅ 正确，是 INV-02 的结构化落点 |
| **REFLECTION(⑥) 在 DECISION(⑦) 之前、且不重合** | ✅ 正确——`Reflect ≠ Completion` 有了架构位置 |
| **PROJECTION(⑨) 在最后** | ✅ 符合事实产生权公理（后端产生事实、前端投影事实） |
| **Observer 置于外围** | 🟡 待定。我建议评估栈落在 Observer OS（D11）——「外围」的表述可容纳，但需明确它承载 L2/L3 评估，否则 Observer 会退化成纯 telemetry |
| **Memory/Compaction 置于外围** | ⚠️ 与 §3 的张力：外围 ≠ 不可调参。建议注明「架构在外围，阈值标定属核心决策链」 |

整体判断：**这张图的拓扑是对的**，VERIFICATION 与 REFLECTION 的分立尤其关键。与本总包方向一致，无冲突。

---

## 9. 其余建议（非阻塞）

1. **Node 09 PASS 判据 #6「步数 <= 42」来源未说明**——请注明基线依据（否则这个数会像 `BUDGET_LOW_THRESHOLD=0.15` 一样成为下一个无主常数）。
2. **Node 09 复用 P1-CONSOLIDATION-01 Node 07 样本前，建议先重判归因**：该偏差被归为「Agnes flash 多步编辑能力边界」，但 §2 的机械根因显示**放弃发生在 `steps_without_progress>=2` 且预算 ≤15% 时，与模型编辑能力无关**。总包 §22 自己也写明「模型/Provider 归因必须在排除 TaskGraph、Budget、Verification、Decision Control 后才能成立」——**目前尚未排除**。若沿用错误归因，Node 09 的验收判据会建在错误前提上。
3. **Node 04「自然样本优先、不得人为制造 production 错误」——这条写得很好**，建议保留并扩展到 Node 05/06。
4. **INV-A~H 八条不变量质量高**，尤其是 INV-G（acceptance criteria 不能被 Agent 改写后用作自身完成证明）与 INV-H（已知事实字段语义不能通过字符串猜测重新解释）——INV-H 与我登记的 RC40 同源，可在实现时顺带覆盖。
5. **建议补一个量化指标**：正确弃权率（不该进入执行循环的输入中，确实未进入的比例），作为 Node 20-B 的可测口径。

---

## 10. 证据边界（诚实声明）

- 本评审全部源码锚点基于 **`ff4eb2f`**；执行窗口仍在施工，**提交前请重新 `git log -1` 对齐**。
- §2 的「budget-low × 2 步无写盘 → GiveUp」是**源码条件与日志现象的机械对齐**，因果链完整，但**未在本窗口实跑复现**（我未执行代码）。建议执行窗口在 Node 01 用一个确定性 fixture 坐实它——成本很低，价值是把推测变成 confirmed。
- RC44 基于当前 HEAD 的投影函数读码判定，**未做端到端实测**。
- 我未修改任何 `crates/` 代码；本窗口不产出修复代码，按分工交执行窗口落地、顶层审批。

---

## 11. 给顶层的一句话

**放行，但补三条**：C-1 把常量标定从排除清单里摘出来（否则本包核心问题无解）、C-2 给 α 目标侧分类器一个归属 Node（否则追问被吞继续存在）、C-3 让连续执行留下可独立复核的痕迹（否则 16 个 Node 的结论悬在自评上）。

**另外请在放行前留意 §9.2**：Node 09 复用的那个样本，其失败归因很可能不是模型能力，而正是本包要修的机制——**用错了归因，验收判据就会验收错的东西**。

---

**附：本评审引用的源码锚点清单**

| 锚点 | 内容 |
|---|---|
| `planner/src/lib.rs:233-235` | 三个扁平常量（3 / 5 / 0.15） |
| `planner/src/lib.rs:495-502` | **budget-low GiveUp 臂（`>= 2` 步）** |
| `planner/src/lib.rs:506-524` | replan 臂（`>= 5` 步）与 replan_count 上限 3 |
| `agent-core/src/loop.rs:167` | `goal_requires_product`（执行侧三态漏斗，已治） |
| `agent-core/src/loop.rs:324-378` | `classify_user_input`（目标侧，仍缺疑问句类别） |
| `agent-core/src/loop.rs:3676-3685` | progress 只认 write_file/apply_patch |
| `agent-core/src/loop.rs:912` | `acted` 字段（注释称含 bash，但**不参与 progress 记分**） |
| `agent-core/src/loop.rs:1615-1632` | **RC44：失败态折叠为 pending** |
| `agent-core/src/loop.rs:4264-4295` | acceptance_result 生产（passed / failed object） |
| `agent-core/src/loop.rs:3822-3830` | REFLECT_FACT_CONFLICT（observe-only，Node 04 可复用） |
| `codex-cli/src/lib.rs:65` | budget 默认 40（自述 20→40 因「几乎每个任务 budget low」） |
