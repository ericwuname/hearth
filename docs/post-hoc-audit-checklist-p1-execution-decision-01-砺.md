# P1-EXECUTION-DECISION-01 · 发执行窗口前的残留风险 + 后置审计清单

> 砺·评审（🪨）｜2026-08-30 14:0x｜基线 `ff4eb2f`
> 用途：①**发包前**请顶层过目三条残留风险（§1）；②**Final Report 回来后**我按 §2 执行后置审计（修 E 指派）
> 方法：所有风险均带源码行号；**证伪的也要写**（避免顶层重复排查）

---

## 1. 三条残留风险（一条好消息、两条真隐患）

### ✅ 风险 1（已证伪，是好消息）：acceptance criteria 不由 LLM 生产，修 D 不构成自审

**我原本担心的**：修 D 裁"已核验 passed → completed"，等于把完成判定押在 criteria 上。**若 criteria 由 agent/LLM 自生成，则 INV-G（criteria 不能被 Agent 改写后用作自身完成证明）就被架空**——修 D 会从"不得自审"变成"换了个人自审"。

**实测结论：不成立，criteria 唯一来源是用户。**

```text
crates/codex-cli/src/lib.rs:75      #[arg(long = "acceptance")]  ← CLI 用户输入
crates/codex-cli/src/run_local.rs:405  agent.init_taskgoal(Vec::new(), acceptance.clone())
crates/codex-cli/src/run_local.rs:408  agent.set_pending_acceptance(acceptance)
crates/agent-core/src/loop.rs:1009     pub fn set_pending_acceptance(...)
crates/agent-core/src/loop.rs:3966-67  （仅用于 ContextManager 重建后恢复用户 criteria）
```

全链路无 LLM 参与生产 criteria；核验是确定性执行（`cmd:` exit code / `file:` contains，见 `parse_acceptance_criteria` loop.rs:1019）。

**结论：修 D 的"acceptance passed → completed"建立在「外部提供的标准 + 确定性执行核验」之上，符合独立判定原则，不构成自审。** 这条我原本准备提异议，实测后撤回——**顶层这条裁决站得住**。

---

### 🔴 风险 2（真隐患，建议发包前一句话澄清）：Verification Reserve 可能被跨相位重复计费

**事实**：`acceptance_replan_count` 是 **run 级单字段**，上限 1。

```rust
// crates/agent-core/src/loop.rs:900    字段定义
// crates/agent-core/src/loop.rs:1392   初始化 0
// crates/agent-core/src/loop.rs:4266-68  唯一递增点，位于 Done 相位核验块内
} else if self.acceptance_replan_count < 1 {
    self.acceptance_replan_count += 1;   // Reserve 1/1
```

**问题**：修 D 要求"give_up 前插 readiness 先决，criteria 非空未核验 → **先花 Reserve 核验**"。若这次消费与 Done 相位共用同一计数器，则：

```text
Reflect 相位：GiveUp 拦截 → 花掉唯一一次 Reserve 做核验
              ↓（核验失败 → 回喂重试 → 任务继续）
Done 相位：再次核验 → acceptance_replan_count 已 = 1 → 直接 verify_failed
              ↓
本该有的重试机会被前置消费掉
```

**建议补一句澄清（一句话即可，不必改设计）**：

> 修 D 的 GiveUp 前置核验，**新开独立计数**（如 `giveup_readiness_reserve`），与 Done 相位的 `acceptance_replan_count` **分账**；两者各自上限 1，总消耗仍计入既有 steps（不突破预算总量）。

若顶层认为共用即可（同一轮里 Reserve 本就该只花一次），也请**在总包里明确写死**，否则执行窗口会自行选择一种实现，而两种实现的终态行为不同（`verify_failed` 出现的时机不同），会给 Node 09 的判据带来歧义。

---

### 🔴 风险 3（真隐患，属覆盖缺口不是缺陷）：无 criteria 的纯 bash 产品任务，仍会 give_up

**事实**：修 D 的 readiness 先决以 criteria 非空为触发条件。

```rust
// crates/agent-core/src/loop.rs:4258-4261
let criteria = self.ctx_mgr.state().acceptance_criteria.clone();
let checks = Self::parse_acceptance_criteria(&criteria);
if !checks.is_empty() {          // ← 空则整个核验块跳过
```

**而 progress 记分仍只认写盘**（`loop.rs:3676-3685`，修 A 已把口径变更列为**备选**）。

**合成一个不受 v1.1 保护的窗口**：

| 条件 | 状态 |
|---|---|
| 任务类型是 Product（会进 TaskGraph） | ✅ `goal_requires_product` = true |
| 用户没给 `--acceptance`（绝大多数会话如此） | → criteria 为空 → **修 D 不触发** |
| 任务靠 bash 完成（跑测试 / 构建 / 部署 / 安装 / 环境修复） | → 全程无 write_file/apply_patch → **progress 永不记分** |
| 结果 | → 5 步无 progress → Replan ×3 → **GiveUp** |

**这不是 v1.1 的缺陷，是它的覆盖边界。** 但建议在总包里**显式登记为 OPEN**，理由有二：

1. 这类任务在真实使用中并不罕见（"帮我把这个服务跑起来""跑一遍回归并把失败项列出来"）；
2. 如果不在总包里写明，Node 09 复跑一旦碰到这类任务，容易被**再次误归因为模型能力**——正是修 F 刚纠正过的那类错误。

**建议加一句（登记即可，不要求本轮修）**：

```text
OPEN（本轮不修，登记）：
  criteria 为空且任务以 bash 完成时，"无写盘 = 无 progress" 仍会导致
  Replan×3 → GiveUp。修 D 不覆盖此路径（readiness 以 criteria 非空为前提）。
  若 Node 09/Node 14 复跑遭遇此类失败，
  归因须先排除本条，不得归为模型能力。
  候选解（待修 A 数据驱动启用）：progress 口径纳入"工具成功且产出新事实"，
  或将 bash 执行成功且输出非空纳入弱进展信号（不重置 stalled 计数但阻断 give_up 臂）。
```

---

## 2. 后置审计清单（Final Report 回来后我按此执行）

**审计原则**：不阻塞施工（施工期间不介入），Final Report 提交后一次性审；**只查可复核的证据，不接受结论性描述**。

### A. 机制正确性（修 D —— 本轮成败所在）

| # | 检查项 | 怎么判 | 不合格 |
|---|---|---|---|
| A-1 | GiveUp 拦截点位于 planner 返回之后、终态之前 | 查 `loop.rs` ReflectVerdict::GiveUp 分支，确认 readiness 先决在终态 emit 之前 | 🔴 |
| A-2 | **criteria 非空 + 未核验 + 预算低 + 2 步无写盘 → 必须先核验再放弃** | 需 fixture 证明：四条件齐备且验收实际通过 → terminal = **completed** 而非 give_up | 🔴 |
| A-3 | criteria 非空 + 核验**失败** → verify_failed，不得 completed | 需 fixture | 🔴 |
| A-4 | criteria 为空 → 行为与修复前一致（不得意外改变既有路径） | 需回归用例 | 🔴 |
| A-5 | **Reserve 分账**（风险 2） | 查 `acceptance_replan_count` 是否在 Reflect 相位被递增；若共用，须有顶层书面确认 | 🟡 |
| A-6 | planner GiveUp 降级为记录项后，仍在报告中可见（不得静默吞掉） | 查终态/报告字段 | 🟡 |

### B. 反例用例（修 E 的核心 —— 证明判据真的能失败）

**每项必须有"先红后绿"记录**（修复前该用例失败、修复后通过）。**没有反例的判据 = 判据不存在。**

| # | 反例 | 期望 |
|---|---|---|
| B-1 | acceptance 实际 failed，但 LLM 自报 PASSED | 不得 completed |
| B-2 | criteria 未核验即遇 GiveUp | 不得 give_up（先核验） |
| B-3 | 连续 3 次纯疑问输入 | **0 次 GoalChanged**，且必须给出针对问题的回答 |
| B-4 | TaskControl 输入（"继续"/"查看状态"） | `goal_revision` 不增（INV-A） |
| B-5 | Product + 负向约束（"创建 README，但不要改现有文件"） | 路由为 Product + Constraint，**不得误判 QA**（Node 07） |
| B-6 | acceptance 标准被 agent 改写后自证完成 | 不得作为完成证据（INV-G） |

### C. 投影层（修 C / RC44）

| # | 检查项 | 期望 |
|---|---|---|
| C-1 | `acceptance_result = {"status":"failed",...}` | `acceptance_verification_status()` 返回 **"failed"**，不是 pending |
| C-2 | 单测锁定上述行为 | 存在且能失败 |

### D. 证据可复核性（修 E）

| # | 检查项 |
|---|---|
| D-1 | 每 Node gate 日志含真实 exit code 与测试计数（baseline / added / removed / ignored） |
| D-2 | Final Report 的 PASS / PASS WITH DEVIATIONS 结论**逐项指向证据条目**，不得只有结论性描述 |
| D-3 | ignored 项逐项解释；测试数量变化有说明 |
| D-4 | Node 02/05/07/08 四个决策类 Node 均附 B 组反例 |

### E. 纪律（修 F）

| # | 检查项 |
|---|---|
| E-1 | Node 01 是否**先用确定性 fixture 把 GiveUp 臂从 likely 升 confirmed**，再动修 D |
| E-2 | Node 09 对照基线表述已改为机制归因（"budget-low GiveUp 臂 × progress 口径 × 验证不记分"），不再写模型能力 |
| E-3 | 若复跑仍有误杀 → 是否触发修 A 的数据驱动启用（而非直接调预算上限） |
| E-4 | PASS 判据 #6「步数 ≤42」注明来源 |

### F. 残留风险跟踪

| # | 检查项 |
|---|---|
| F-1 | **风险 3**（无 criteria 纯 bash 任务）是否在 Final Report 的 OPEN 中显式登记，而非静默略过 |
| F-2 | 若 Node 09/14 复跑失败，归因是否已先排除 F-1 |

### 审计结论分级

```text
🔴 阻塞：判据缺失或反例不能失败 → 建议回填，不进终验
🟡 登记：行为正确但缺证据/缺说明 → 登记 OPEN，不阻塞
🔵 观察：已合规但值得下轮关注
```

---

## 3. 我这轮的诚实边界

- 风险 1 我原本准备提异议，**实测后撤回**——顶层裁决正确，无需改。
- 风险 2 是**读码推断**：共用计数会产生何种终态差异，我**未实跑验证**；建议执行窗口在 Node 01 顺带用一个 fixture 坐实（与修 F 同一批，成本极低）。
- 风险 3 是**覆盖边界推断**，同样未实跑；登记为 OPEN 即可，不要求本轮修。
- 全部锚点基于 `ff4eb2f`；执行窗口施工中，**审计前会重新 `git log -1` 对齐基线**。
- 本窗口**不产出修复代码**（按分工：执行窗口落地、顶层审批）。
