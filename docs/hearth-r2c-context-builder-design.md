# Hearth R2-C 设计单：Context Builder / Intent Understanding + Cache Evidence + Resource Safety

> **状态**：设计产出 + 两项 Observe 层施工（采集器 v2 / 资源记账），**ContextBuilder 本体不施工**——待顶层评审。
> **执行**：执行窗口（GLM 5.3）· 2026-08-28 · 基线 `db925d1`/v0.2.7（见 `docs/hearth-r2c-baseline.md`）
> **依据**：R2-C 设计任务更新 + 守门员批注 1-7

---

## 1. 当前有效基线

见 `docs/hearth-r2c-baseline.md`（P0 重同步产物）：v0.2.7 / HEAD db925d1 / VM 门禁 370 passed 全绿 / **VM /usr/local/bin/hearth 已升级 0.2.3→0.2.7**（用户手工测试通道）。

## 2. Context Assembly 源码事实（§四 实测分层标注）

`build_messages()`（loop.rs:1286 起，~400 行）真实组装序（全部实测锚点，非概念分类）：

| 区块 | 位置 | 分类 | 字节级稳定性 |
|---|---|---|---|
| system_text：工作流/IDENTIFIER CONTRACT/TOOLS 清单（按 goal_requires_product 分流） | loop.rs:1300-1380 | **session-stable**（同 goal 恒定；跨轮换 goal 变） | R2-A 实测：**不稳**（20 请求 7 唯一 system_hash） |
| constitution（6 条） | constitution_prompt() | **stable**（文件级恒定） | 稳 |
| talent inject_text | 按 goal 关键词触发 | session-stable | 条件触发即变 |
| Hearth.md | loop.rs:124-129 加载 | **session-stable**（会话内恒定） | 稳（文件不变时） |
| experience 注入 | injected_experience | **task-dynamic**（按轮有效） | 不稳 |
| **TaskGraph 状态注入**（`## Task Plan`） | do_plan 前 nodes 遍历 | **task-dynamic（每步变）** | **不稳——R2-A 实锤头号根因** |
| LSP diagnostics / semantic code context | observe 相位产出 | turn-dynamic | 不稳 |
| history（compaction/摘要/工具结果） | ctx_mgr | turn-dynamic | 每请求增长 |
| **Task Continuity 块**（R2-D 新增） | history 尾部 Role::System | **turn-dynamic（尾部，不破前缀）** | 尾部设计——R2-A 根因的对症位置 |
| tool schemas（ChatRequest.tools） | dispatcher.list_tools | **stable**（工具集注册后恒定） | 稳（但与 system 分离传送） |

**结论**：Hearth 的"stable prefix"实际由 4 类不稳定源污染（TaskGraph 状态/experience/LSP/retriever），其中 TaskGraph 是每步必变项。

## 3. Cache Evidence（v2 采集器已施工 + 数据）

### 采集器 v2（补充 1 前置，已施工）

- **迁移**：cache_telemetry agent-core → **llm-gateway**（gateway 已依赖 agent-types，无缝）
- **新增 `TelemetryProvider` 装饰器**：composition root 出口统一包装（CLI `build_provider` 外层 + service main.rs），**loop plan-chat 与 planner 直连 chat（decompose/reflect）共用同一 provider 实例——一处装饰覆盖全部出口**（v1 漏采 planner 出口的缺口闭合）
- session 归属：`HEARTH_TELEMETRY_SID` env 侧通道；jsonl 加 `exit` 字段区分来源
- env `HEARTH_CACHE_TELEMETRY` 开关不变（默认关零开销）

### v2 数据（待真机采集后补全本节）

见 §15 真机计划——采集执行中/完成后回填。**v1 数据的 57.6% 命中率已降级为"plan 相位有偏下界"引用**（补充 6）。

### miss 分类框架（按 v1 数据 + v2 待验证）

| 假设 | 验证方法 | v1 证据 |
|---|---|---|
| H1 system_text 每步变（TaskGraph 注入） | system_hash 唯一值数 / session | **20 请求 7 值——成立** |
| H2 history 逐条增长（正常 miss） | full_hash 变化率 | 正常增长 |
| H3 experience/LSP 偶发注入 | 前后请求 hash 跳变点 | 待 v2 |
| H4 tool schema 变化 | tools 数组 hash（v2 加） | 待 v2 |
| H5 provider cache 粒度规则 | 分段二分（v2 逐消息 hash 链） | 未知 |

## 4. build_messages() 目标结构（ContextBuilder 设计）

**分层裁决**（依据 §2 实测 + R2-A 数据，非概念）：

```text
L1 Stable（字节级恒定，会话内）
   constitution + Hearth.md + 固定工作流骨架 + 工具使用规则
L2 Project（session-stable）
   Hearth.md 项目段 / goal 分流文案（同 goal 恒定）
L3 Task（task-dynamic → 挪到尾部 dynamic 区）
   TaskGraph 状态 / experience / Task Continuity（已裁决尾部）
L4 Dynamic（turn-dynamic）
   history（工具结果/对话）+ 最新用户输入 + Task Continuity 块
L5 Recovery（recovery-only）
   resume 注入 / compaction 摘要（历史开头，独立标记）
```

**关键设计决策**：
1. **TaskGraph 状态从 system_text 摘除**，并入尾部 Task Continuity 块（R2-D 已建块）——消除头号不稳定源；TaskGraph 的"计划视图"进 Continuity（模型每步仍可见，位置换成尾部）
2. experience/LSP/retriever 注入 → 尾部 dynamic 区（同原则）
3. **system_text 收敛为纯 stable**：constitution + Hearth.md + 工作流骨架——同 goal 会话内字节恒定
4. 补充 5 红线：**tool schema 不做相位裁剪**（schema 移除 = 工具不可调用——行为回归风险 > cache 收益），本轮 tool schema 保持全量稳定
5. ContextBuilder 只做组装（§十二 原则 7），事实源仍是 RunState/TaskGraph/session_store

**施工边界**：本设计单只定结构与迁移清单；实现 = 独立施工单（依赖顶层评审 + v2 数据回填）。

## 5. TaskGoal × ContextBuilder 接口（§五 复用裁决）

ContextBuilder 消费既有事实（不重建）：
- `RunState.original_goal / constraints / acceptance_criteria`（R2-D 已持久化 taskgoal.json）
- `TaskGraph::next_action_deterministic / completed_titles / remaining_titles`（R2-D 派生函数）
- **注入路径唯一**：`task_continuity_message()`（R2-D 已实现，尾部块）——ContextBuilder 只是把 build_messages 里的零散注入**收编到该块**，不再新增第二套注入（批示 §九 红线）

## 6. Goal Revision 三分类（§六 + 补充 4）

### 真实误报案例分析（t_revision.log 实测反例）

`"查看状态"` 被判 revision 3——根因：`apply_turn_goal` 的判定条件是 `goal.text != 旧值`（**任何文本变化都算修订**），无三分类门。"查看状态"是 Task Control（问询），非 Goal Mutation——revision 被污染。

### 三分类 + 零成本规则门设计（禁止每轮 LLM 调用——补充 4 红线）

```text
User Input
├── Task Control（不动 revision）：前缀/全文匹配规则集
│   继续 / 查看状态 / 看看 diff / 发生什么了 / 为什么 / 怎么样了 / status / 继续+疑问
│   （规则集可配置，第一版 ~15 条高置信模式）
├── Conversation（不动 revision）：显式问句标记（以？/? 结尾且无动作动词）
└── Goal Mutation（revision++）：规则判不了的其余输入
    → 第一版保守策略：默认视为 Mutation（宁可多计不可漏计——
      revision 是观测锚点不是控制信号，多计无害、漏计丢锚）
    → 升级机制：规则置信不足时复用 clarify 通道（不新建）
```

**控制流影响声明**：该门改变 apply_turn_goal 的判定输入（当前无条件算 Mutation）——**WP-0 D 类，本轮只交付设计，施工须顶层单**（任务书 §六 明文）。

## 7. Resource Safety（§七/§八 + 补充 3）

### 57GB 复盘

| 问 | 答 |
|---|---|
| 哪个工具产生？ | **无法精确归因**——现场已清（telemetry 数据保全，但 57G 目录无 per-file 清单）。间接证据：21 步中 bash 反复跑 node 验证 fizzbuzz；bash `format_output` **无输出截断**（stdout 全量入 String→ToolResult→history）是头号结构性嫌疑 |
| resource-monitor 为什么没信号？ | **实锤（105 行全读）**：相位体检器——仅 `memory_percent>80` / `disk_free_gb<1.0` 系统级快照；与工具写盘路径**零接线**；且其告警只进 tracing/宪法触发，无工具层拦截语义。57G 写满后它至多能"事后知道" |
| 为什么没有 actionable signal？ | 观测粒度错位：磁盘级体检 vs 需要的是 **per-call/per-tool 字节级**；且观测点（相位边界）与行为点（工具执行）分离 |

### Observe 层施工（已落地）

**dispatcher 单漏斗字节记账**（`ResourceLedger`，tool-runtime/dispatcher.rs）：
- `per_tool_bytes / total_bytes / calls` 累计（args+result 字节）
- 阈值 warn：单调用 >10MB / 任务累计 >1GB → `tracing::warn`（actionable signal——不拦截）
- snapshot() 供 introspect/报告消费
- **红线**：warn/pause/deny/terminate 控制流 = WP-0 D 类，本轮零实现

**后续候选**（D 类，须顶层单）：bash `format_output` 输出截断上限（头号修复优先级）、write_file 单文件上限、任务累计写盘上限（G0 级拓扑约束候选）。

## 8. Bridge 状态决策（§十 + 补充 6）

**裁决：INTENDED → DEFER**（采纳补充 6 倾向）。依据：bridge（284 行）对应三端口架构 Human OS 接真流方向（路线图 B4-2）；无 production 调用 ≠ abandoned。
- re-entry 条件：B4-2 桌面接真流开工
- 届时不接 → 改 ABANDONED，清理 workspace 依赖
- 本轮零改动

## 9. Intent Understanding Benchmark 设计（§十一）

```text
A 原始用户输入（无 context）      — 基线
B 当前 Hearth context            — 现状
C 精简 Hearth context（L1 收敛后）— 本设计效果
D B + Task Continuity 注入        — R2-D 效果隔离
E 同模型 Codex                    — 外部参照
```
用例集：5 类 × 5 任务（歧义指令/多步目标/隐含约束/中途换向/纯问询），指标：intent preservation / plan alignment / tool selection / unnecessary assumptions / goal drift。执行挂 Agnes 预算（本轮只落设计+用例定义，执行待 v2 数据回填后）。

## 10. ADOPT / ADAPT / DEFER / QUARANTINE 汇总

| 项 | 决策 |
|---|---|
| TelemetryProvider 装饰器 | **ADOPT**（已施工——本轮唯一结构性新增） |
| ResourceLedger 记账 | **ADOPT**（已施工——Observe 层） |
| TaskGraph 状态挪尾部 | **ADAPT**（并入 Task Continuity，施工待单） |
| experience/LSP 注入挪尾部 | **ADAPT**（同上） |
| tool schema 相位裁剪 | **QUARANTINE**（补充 5 回归风险——除非有行为护栏，不碰） |
| Goal Revision 三分类门 | **ADAPT-DEFER**（设计已出，D 类施工须顶层单） |
| bash 输出截断 | **DEFER→D 类**（高优先级候选） |
| Bridge | **DEFER**（INTENDED，re-entry=B4-2） |
| Subagents / TUI / MCP | DEFER（不变） |

## 11. WP-0 D 类事项（本轮零实现）

① Goal Revision 三分类门（改 apply_turn_goal 判定）② bash 输出截断/写盘上限 ③ resource warn→pause/deny ④ resource-monitor 接线改造

## 12. 修改范围（本轮已施工部分）

| 文件 | 变更 |
|---|---|
| llm-gateway/src/cache_telemetry.rs | 自 agent-core 迁入 + record_with_exit/exit 字段 |
| llm-gateway/src/telemetry_provider.rs | 新建（装饰器） |
| llm-gateway/src/lib.rs | 注册模块 |
| llm-gateway/Cargo.toml | +chrono/sha2 |
| codex-cli/run_local.rs | build_provider 拆分+包装 / HEARTH_TELEMETRY_SID |
| service/main.rs | openai 出口包装 |
| tool-runtime/dispatcher.rs | ResourceLedger + dispatch 记账 |
| agent-core | cache_telemetry 迁出（模块删除） |

## 13. 测试计划

- 已落：gateway cache_telemetry 3 测试（随迁）；tool-runtime ResourceLedger 累计/排序测试
- 待补：TelemetryProvider 包装后 chat 记录行为测试（env 开关双态）
- 真机：采集覆盖 plan/act/reflect/resume/compact（§15）

## 14. 架构不变量影响

- 事件契约：只增（v2 零新变体——telemetry 不走事件流）✓
- 三端口/Observer：零影响（未知事件忽略）✓
- 事实源：telemetry/ledger 均为观测，非事实源 ✓
- **风险披露**：dispatcher 双实例（read_only_view 独立 ledger）——只读流量记账分离，snapshot 口径须注明

## 15. VM 计划与真机采集（执行中/回填）

采集矩阵：2 个写文件任务 + 1 个多轮 repl（含"继续/查看状态"）+ 1 个 resume 续做 + 1 个长会话触发 compact——目标 30-50 请求，全部出口可见（v2）。数据回填 §3，miss 分类驱动 ContextBuilder 施工单。

## 16. 五问回答（§十八 验收）

1. **为什么意图保持不如 Codex**：context 组装层——stable prefix 每步被 TaskGraph/experience 污染（R2-A 实锤）+ goal 语义仅 60 字符摘要存续（R2-D 前）+ 工具结果全量回灌无截断。不是模型问题。
2. **每轮稳定知道四要素**：R2-D Task Continuity 块（original/completed/remaining/next_action 每轮尾部注入，compact 后仍存活——T4）。
3. **为什么 compact 不应删 Continuity**：它从 RunState/TaskGraph 派生（非历史数据），压缩历史不触碰事实源——T4 已证。
4. **cache 真实瓶颈**：57.6%（plan 相位有偏下界）；头号瓶颈=system 内动态注入；v2 数据回填后给全量修正值。
5. **57GB 为什么没被发现**：resource-monitor 是系统级相位体检器（disk_free<1G 才响），与工具写盘零接线、无 per-call 观测、无工具层信号——粒度错位。ResourceLedger 已补观测层。

**ContextBuilder 施工前置**：本设计单过顶层评审 + §3 v2 数据回填完成。
