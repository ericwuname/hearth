# Execution Decision Flow — P1-EXECUTION-DECISION-01 Node 01 审计（2026-08-30）

> 证据等级：本文全部结论 **confirmed**（源码锚点 + 确定性 fixture 双证）。
> fixture：`crates/planner/src/lib.rs::test_execdec_node01_budgetlow_giveup_ignores_acceptance`（绿）。

## 1. Decision Points（谁决定什么）

| # | 决策点 | 落点（file:line） | 数据源 | 权威 |
|---|---|---|---|---|
| D1 | 输入分类（Task/QA/Control/Conversation/GoalMutation） | `loop.rs:324 classify_user_input` | 纯规则（CHITCHAT ≤12 字 :370-378；**无疑问句类别**） | loop 规则层 |
| D2 | 是否进 TaskGraph（product intent） | `loop.rs:156 goal_requires_product`（Node 02 分离已修） | residual 判定 | loop 规则层 |
| D3 | decompose（拆图） | `loop.rs:~1979`（QA 跳过已修） | goal 分类 | loop |
| D4 | plan/拆图内容 | planner LLM | goal+context | planner LLM |
| D5 | **replan 决策** | `planner/lib.rs:506` steps_without_progress≥5 → Replan（cap 3） | Observation | planner 规则 |
| D6 | **give_up 决策（3 臂）** | `planner/lib.rs:483`(errors≥3) / `:489`(remaining=0) / `:495-502`(fraction≤0.15 && noprogress≥2) | Observation（**不含 acceptance/verification**——planner 全文 grep "acceptance"=0 confirmed） | planner 规则 |
| D7 | progress 记分 | `loop.rs:3676-3685`：**只认 write_file/apply_patch**；bash/read（验证类）永不算 | pending_results | loop 活性判据 |
| D8 | budget-low 信号 | `loop.rs` obs 组装（budget_remaining 由 steps_used 推） | RunState.budget | loop→planner |
| D9 | acceptance 核验 | `loop.rs:~4261 verify_acceptance_criteria`（TASK-TRUTH 落地） | cmd exit/file 内容（L1） | loop 确定性核验 |
| D10 | acceptance passed 生产者 | `loop.rs:~4270 set_scratch("acceptance_result","passed")` | D9 结果 | loop |
| D11 | Reflect fact conflict 标记 | `loop.rs` GiveUp 分支（CONSOLIDATION 落地，observe-only） | written_files+errors | loop 观察 |
| D12 | completion 终判 | loop 主循环 GiveUp 分支 → RunReport；**acceptance 核验只在 Done 相位发生**——give_up 路径**完全不触发核验** | — | loop |
| D13 | terminal 归类 | `terminal.rs normalize_terminal_state` 九态 | reason 字符串 | 纯函数 |

## 2. 44 步病灶的准确控制流（confirmed）

```text
修复写盘完成（write_file ✓ → progress 记 1 分）
  ↓ 后续步骤全是 bash 验证/复测（D7：不计 progress）
steps_without_progress 累积 ≥2（连续验证步）
  ↓ budget 同步消耗（replan+验证吃步数）
budget_fraction ≤ 0.15
  ↓ Reflect 调用 → planner :495-502 GiveUp 臂命中
  ↓ 【关键缺口】GiveUp 臂只看 Observation 数字，不看：
     - acceptance criteria 是否存在/是否已核验
     - file_changes / successful_write_count（fixture 证明齐备照样 GiveUp）
     - 任务实质完成状态
  ↓ loop 收 GiveUp → RunReport ok=false "failed"
（TASK-TRUTH 轮：模型自述 PASSED 且事后外部复跑确证 test ok——但决策链从未消费这些事实）
```

**根因定性**：确定性陷阱（非模型能力）——修 F 归因更正成立。三要素：
① GiveUp 臂无 acceptance 感知；② progress 口径不认验证步（复测=无进展）；③ completion 核验只挂在 Done 相位，give_up 路径绕过 D9-D10。

## 3. Current Inconsistencies

| 编号 | 不一致 | 影响 | 处置归属 |
|---|---|---|---|
| INC-1 | **RC44**：生产端写 `{"status":"failed","failures":[...]}` object（loop.rs:4275-4288），投影端 `acceptance_verification_status()` `as_str()` 取不到 → 折叠 pending（loop.rs:1615-1632） | failed 被洗白成 pending——Node 02 四态断路 | Node 02 修 C（前置） |
| INC-2 | GiveUp 臂与 completion 事实零交互（§2） | 真完成被预算误杀 | Node 05 修 D |
| INC-3 | classify_user_input 无疑问句类别（:370-378，CHITCHAT 白名单 ≤12 字兜底） | 纯疑问输入 → GoalMutation → revision 爆炸（峰值 71） | Node 07 修 B |
| INC-4 | 失败无分类（Node 06）：所有 error 统一 consecutive_errors 计数 | transient 与 plan failure 同权重 retry | Node 06 |

## 4. Evidence Strength（现状）

- **L1（工具层）**：confirmed 强（exit code / is_error 结构化）
- **L2（产物层）**：confirmed（written_files/快照）
- **L3（验证层）**：confirmed（TASK-TRUTH O-4）——但**仅在 Done 相位可达**
- **L4（验收层）**：passed 有生产者；failed 投影断路（RC44）
- **决策层**：planner GiveUp 臂 = L0（无证据消费）——本总包修 D 补链

## 5. Node 01 结论

修 D 的落点在 **loop 侧 give_up 决策点前插 readiness 先决**（顶层预裁决已定）：
planner 三常量与 GiveUp 臂保持原样（修 A 标定权限备而不用，Node 09 数据驱动再决定）。
fixture 已把"likely"升级为"confirmed"，Node 05 开工条件满足。
