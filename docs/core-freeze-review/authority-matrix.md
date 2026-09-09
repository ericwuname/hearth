# Authority Matrix — CORE FREEZE REVIEW-01 Node 01

> 方法：先填源码实测现状（砺批-3 基线锚点），再查双 authority/无 authority/越级。
> 图层链：INTENT→PLAN→EXECUTION→FACT→VERIFICATION→REFLECTION→DECISION→FAILURE
> ADAPTATION→MEMORY/COMPACTION→RESUME→TERMINAL→PROJECTION。

## 层表（输入/输出/事实源/决策权/LLM 可改/失败旁路）

| 层 | 输入 | 输出 | 事实源 | 决策权 | LLM 可改 | 失败旁路 |
|---|---|---|---|---|---|---|
| INTENT | 用户输入 | UserInputKind（Task/TaskControl/Conversation） | 用户原文 | classify_user_input 规则（loop.rs:324） | 否 | 空输入→TaskControl |
| PLAN | goal+TaskGraph 状态 | TaskGraph（decompose） | planner LLM 输出+规划规则 | DefaultPlanner（LLM）+T4 停滞规则 | 是（产出） | T4 同图 ×2→bounded Err |
| EXECUTION | tool_calls | ToolResult（is_error/output/error_kind 五态） | 工具真实执行+沙箱 | approval 策略（DenyAll/DelegateSession） | 否（执行不可谎报） | 非零退出→is_error |
| FACT | ToolResult+产物 | written_files/artifacts/error_kind | 磁盘产物+工具退出码 | 无（事实层无人裁决） | 否 | — |
| VERIFICATION | criteria/written_files | acceptance_result / verify 行 | cargo/文件系统确定性命令 | verify_acceptance_criteria（唯一生产者） | 否 | 失败→回喂 replan |
| REFLECTION | 历史+pending_results | verdict（Continue/Replan/GiveUp） | planner LLM | planner（决策权）+loop（否决权，见下） | 是 | — |
| DECISION | verdict+事实 | 路由（Plan/Act/Done/Error） | 三路拦截+Reserve+RC47 路由 | **决策权/否决权分离**（见 Authority Matrix） | 是（意愿）| — |
| FAILURE ADAPTATION | ToolResult.error_kind+产物 | FailureKind→RecoveryStrategy | classify_failure 纯函数（terminal.rs:277） | 矩阵规则 | 否 | — |
| MEMORY/COMPACTION | history est | 摘要+archive | archive JSONL（磁盘） | maybe_compact 规则（阈值 provider-aware） | 否 | archive 失败→warn 不阻塞 |
| RESUME | 持久 session | 恢复的 state+历史 | sessions/<uuid> | — | 否 | 缺失→可行动错误 |
| TERMINAL | run 结果 | completed/failed/timeout/cancelled/give_up | normalize_terminal_state 纯函数 | 证据规则（INV-LR03） | 否 | — |
| PROJECTION | report | CLI 渲染+事件 | 事实投影（事实生成公理） | 渲染层 | 否 | — |

## Authority Matrix（15 项）

| 权项 | Authority（唯一裁决者） | 双 authority? | 无 authority? | 越级? | 备注 |
|---|---|---|---|---|---|
| tool success | 工具执行结果（exit/timeout 五态） | 否 | 否 | 否 | Node 02 五态接线后结构化 |
| artifact exists | 盲区C 文件系统校验（Done 相位） | 否 | 否 | 否 | 存在+非空确定性 |
| TaskGoal | state.goal（用户原文）/original_goal immutable | 否 | 否 | 否 | 修 B 禁改 |
| TaskGraph | DefaultPlanner 产出+节点状态推进 | 否 | 否 | 否 | planner 独占产出 |
| verification | verify_acceptance_criteria（O-4 唯一生产者） | 否 | 否 | 否 | FA01 钉死 |
| acceptance | 同上（acceptance_result scratch） | 否 | 否 | 否 | — |
| reflection | planner LLM verdict | 否 | 否 | 否 | — |
| failure classification | classify_failure 纯函数 | 否 | 否 | 否 | RC20 纪律 |
| replan | reflect Replan 臂（cap 3）+ acceptance 回喂 | 否 | 否 | 否 | — |
| **giveup** | **决策权=planner GiveUp 臂；否决权=loop 三路拦截+Reserve+RC47 路由** | **否——有意设计的"决策权/否决权分离"**（修 D 预裁决，planner schema 不动） | 否 | 否 | 砺批-3 定性：非"双 authority 冲突" |
| complete | Done 相位（盲区C+acceptance 双证据） | 否 | 否 | 否 | LLM says done ≠ 完成 |
| stop | T4 停滞规则+预算规则（bounded） | 否 | 否 | 否 | INV-LR04 |
| escalate | InteractionRequested（语义不确定） | 否 | 否 | 否 | delegation 只解 operational |
| terminal | normalize_terminal_state 纯函数 | 否 | 否 | 否 | 九态封闭集 |
| projection | 事实投影（事实生成公理：后端产事实，前端投影） | 否 | 否 | 否 | RC20 家族纪律 |

**冲突扫描结论**：15 项中 **零双 authority、零无 authority、零越级**。唯一 specials：①giveup 决策权/否决权分离（有意设计，已声明）；②false stop ×2（RC48——Reserve 零和盲区，见 false-stop-lifecycle.md，非 authority 冲突而是**核验机会零和**）。
