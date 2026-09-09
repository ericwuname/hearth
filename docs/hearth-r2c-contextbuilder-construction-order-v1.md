# Hearth R2-C ContextBuilder Construction Order v1.1

> **性质**：独立施工单 v1.1（顶层六点修订 + 守门员四裁决后定稿）——**本文件只定义施工内容，当前零代码改动**。
> **v1.1 修订记录**：①Experience 逻辑重写（点 1/裁决 2——含 EC-05 判断修正）②cache stability 三层拆分（点 2/裁决 4）③topology_sig 与 last_graph_sig 分立（点 3/裁决 1——sig 实测混入 status 不得复用）④"只移位置"表述修正 + [FAILED] 事实保全（点 4/裁决 3）⑤L4 顺序架构理由（点 5）⑥rollback 分级（点 6）。
> **签发依据**：《审阅了〈R2-C Pre-Construction Decision〉》（六条裁决基线）+ 守门员批注 1-6。
> **执行窗口**：待顶层批准后指定。

---

## ⛔ 零、本施工单不允许修改的 D 类事项（红线，放最前）

以下四件各自走**独立施工单**，本施工单的 diff 中**禁止出现**任何相关改动：

1. **bash stdout 截断**（tools-builtin/bash.rs `format_output`）——设计稿：`hearth-r2c-preconstruction-decision.md` §3.1
2. **filesystem write accounting**（sandbox/dispatcher 维度）——设计稿：同上 §3.2
3. **Goal Revision 三分类接入 `apply_turn_goal`**（`classify_user_input` 纯函数已设计）——T5 行为证据在 EC-03
4. **resource 超限 pause/deny/terminate 控制流** + resource-monitor 接线

附带禁改：tool schema 相位裁剪（D 类候选，§九）；bridge；subagent；TUI。

---

## 一、基线声明（补充 4/补充 5.1：实测值，非预估）

- **代码基线**：v0.2.8 / HEAD `08a827a` 链（Tier3 T1-T4 + T2/T3 测试已合入）
- **门禁基线**：隔离门禁脚本 `~/run_gate_r2c.sh`（**严禁使用 ~/run_gate.sh——它操作 ~/codex，是其他窗口资源，曾致门禁串台假绿灯**），log 落 `~/t_gate_r2c.log`
- **门禁计数**：以施工**开工前实测** `~/t_gate_r2c.log` 的新 RC 行为准（373（Tier3 修复后）；T2/T3/T4 测试并入后实测 **375 passed / 0 FAILED / 0 ignored**（`t_gate_r2c.log` 02:11 轮——补充 4 实测口径达成））
- **运行姿势**：本 VM 无 cgroup delegation——hearth 运行须显式 `HEARTH_ALLOW_NO_CGROUP=1`（RT4 fail-closed 语义正确；开发环境降级是正确姿势，见 EC-03 同源根因发现）

## 二、施工目标

把 `build_messages()`（loop.rs:1305-1530，单函数串行拼接）重构为 **L1-L5 分层组装**，达成：

1. **stable prefix 字节级稳定**（同 goal 会话内 system_text 恒定）——消除 MISS-A（likely 根因）
2. **动态内容全部归位 L4 尾部**（TaskGraph 状态/experience 位置纠偏）
3. **行为零回归**（任务完成率不低于 B 变体基线；现有 375± 测试全绿）
4. **ContextBuilder 只组装事实，不建第二套事实源**（数据源仍为 RunState/TaskGraph/session_store）

## 三、当前 build_messages() 真实代码路径与迁移点（实测锚点）

| 注入项 | 当前位置（loop.rs） | 当前分类 | 迁移动作 |
|---|---|---|---|
| constitution | :1397 `constitution_prompt()` | stable | **留 L1** |
| Hearth.md | :1412 `self.hearth_md` | session-stable | **留 L2** |
| experience | :1422 `injected_experience` → system | task-stable（源码证实任务期不重检索） | **迁移到 L2**（Hearth.md 后、TaskGraph 前的稳定区——位置在 system 内但不再每步变） |
| **TaskGraph 全量** | :1429-1441 `## Task Plan:` → system | **每步变（MISS-A 主污染源）** | **拆分**（§四）：拓扑→L2 稳定区；状态→L4 Continuity 块 |
| retrieval_context | :1446 scratch | turn-dynamic | **迁 L4 尾部** |
| LSP diagnostics | :1456 scratch | turn-dynamic | **迁 L4 尾部** |
| history/compaction | :1476-1514 | turn-dynamic | **留 L4** |
| Task Continuity | :1519 `task_continuity_message()` | turn-dynamic（尾部） | **留 L4 尾部**（唯一注入路径，吸收 TaskGraph 状态面——§五） |
| plan chat 重试环 | :1967+ | 无关 prompt | 不动 |
| goal 分流文案（goal_requires_product） | :1300-1380 system 骨架 | session-stable（同 goal 恒定） | **留 L1/L2** |

## 四、TaskGraph topology/status 具体拆分方式（方案 B 落地）

```text
L2 稳定区新增「Task Topology」块（task-stable——仅 decompose 产出新图时重建）:
  ## Task Topology:
    t1: 写 HTML 骨架 (deps: )
    t2: 写 JS 逻辑 (deps: t1)
    ...
  ——只含 id/description/deps；decompose 签名变化时整块重建（正常 replan 带来
  语义变化即重建——与 T4 停滞检测的 sig 复用同一 hash）

L4 Continuity 块扩展（吸收状态面，复用 task_continuity_message() 唯一路径）:
  已有: 原始目标/已完成/剩余/下一步/约束/验收标准（completed_titles/remaining_titles/
  next_action_deterministic 派生——零新事实源）
  不得再注入第二套 TaskGraph/TaskGoal 机制（R2-D 不变量）
```

**落点（点 3/裁决 1 修订——双 sig 分立）**：`last_graph_sig`（T4 引入，loop.rs:1762-1768）实测 hash 输入**含 `status`**——**不得复用**为 topology 触发器（status 每步变 → topology 块每步重建 = MISS-A 复辟）。改为：
```text
topology_sig（新增）：只 hash id/description/deps（去掉 status 行）
  → 专用 L2 Topology 块重建触发器（结构不变就不重建）
last_graph_sig（保留现状含 status）：继续作 T4 停滞检测
  → 语义签名就该含 status（同图同状态才算原地踏步）
```
两 sig 各司其职互不冲突；施工时 topology_sig 计算紧邻 last_graph_sig（同处缓存，零额外成本）。

## 五、Task Continuity 唯一路径收编

- `task_continuity_message()`（loop.rs:951）是 TaskGoal/TaskGraph 状态的**唯一注入函数**——ContextBuilder 不得新建注入函数
- 收编内容：原 `## Task Plan:` 的 status 标记面（[DONE]/[FAILED] 等每步变化部分）并入 Continuity 块的"已完成/剩余/下一步"段（语义已覆盖，无需逐节点重复）
- build_messages 中删除 :1429-1441 的 TaskPlan system 注入 + :1422 experience 注入 → 改由分层组装器输出

## 六、Experience 处置（点 1/裁决 2 修订——**修正 EC-05 前判断**）

**源码实测修正（裁决 2 锚点核实）**：`injected_experience` 生命周期 = 字段 loop.rs:516 / init None :787 / 填充 :1997（**仅当 consecutive_errors>=3 的降级通道**）/ 重置 :1983（检索前清空）与 :3089（"仅当轮有效"每轮清空）。**EC-05 曾判 "task-stable confirmed" 是错误前提**——真实行为 = **error-degradation channel（条件性 turn-dynamic）**：先空后填 + 每轮重置，恰是点 1 禁止的 "empty→populated" 模式。

**修订裁决（两方案并列，X 推荐——待顶层选定）**：

| 方案 | 做法 | 语义影响 | 稳定性 |
|---|---|---|---|
| **X（推荐）** | experience 保持"错误降级"语义不变，注入位置**移 L4 尾部**（dynamic 区） | 零语义变化（降级时才出现，天然 dynamic） | L4 在 suffix——先空后填不破 stable prefix ✓ |
| Y（批示字面选项） | 检索提前到 session 首次 build_messages **之前**完成（或显式永不注入），session 内**禁止重置**（:1983/:3089 两点豁免 experience 字段或改写语义） | **行为变化**：从"错误降级才注入"变"开局即注入"——需顶层明确接受 | L2 stable ✓ |

禁止事项（两方案共同）：**不得用空槽位占位掩盖先空后填**；不得在 system 已进 cache 后从 empty→populated。

## 七、L4 组装顺序（尾部，自上而下）

```text
[L4 tail 区]
1. history（compaction/摘要/工具结果——现状保留）
2. retrieval_context（语义搜索命中——turn-dynamic）
3. LSP diagnostics（硬信号——turn-dynamic）
4. Task Continuity 块（R2-D 唯一路径，含状态面）
5. （latest user input 由既有 turn 结构承载，不单独插入）
```
**顺序架构理由（点 5 固化）**：Task Continuity 是当前任务状态的**最终语义锚点**——必须位于本轮所有 observe/dynamic evidence **之后**（模型最后读到的是"我现在在哪、下一步干什么"，此前读到的是证据）。未来施工不得无依据调整此顺序。

## 八、Recovery / compact 边界

- resume 注入（restore_history）与 compact 摘要（WS4）保持既有位置（history 开头区）
- **Task Continuity 不被 compact 删除**：它从 RunState/TaskGraph 派生（T4 测试已锁）——compact 只动 history，不触 Continuity
- compaction 触发时的 cache 影响：摘要替换历史头部 → 前缀失效一次（正常，OPEN 项继续采集）

## 九、tool schema 稳定约束

- `ChatRequest.tools` 全量稳定（dispatcher 注册后恒定）——**本施工单禁止相位裁剪**（补充 5/EC-03 反例：schema 移除=工具不可调用）
- schema hash 采集继续（telemetry jsonl 加 tool_schema_hash 字段——施工时顺手）

## 十、兼容性与行为回归风险

| 风险 | 缓解 |
|---|---|
| LLM 对新 prompt 结构不适应（位置敏感） | 分层组装保持**执行所需事实集合不丢失，改变事实的分层表达与位置**（非单纯 relocation——TaskPlan 全图 → topology + Continuity state 是结构重组）；T2/T3/T4 三任务 B 变体复测对照 |
| Task Plan 从 system 移除后模型丢失计划视图 | Continuity 块承接状态面 + L2 Topology 块承接结构面——**[FAILED] 事实保全（裁决 3）**：Continuity 块新增"失败节点"行（id+result 摘要），单测锁定（构造含 FAILED 节点的 graph，断言迁移前后失败事实对模型可见） |
| experience 位置变化 | L2 固定槽位 + 空位占位（字节偏移稳定） |
| stream/provider 兼容 | build_messages 输出仍是 Vec\<Message\>，接口零变化 |
| 前缀缓存对 message 序列敏感 | 分层后 system 单条 + history 顺序不变——结构上更稳 |

## 十一、最小测试矩阵

1. **单测（零预算）**：build_messages 输出的 system_text 在同 goal 两次调用字节相同（**MISS-A 的直接回归断言**——当前会失败，施工后必须过）
2. 单测：TaskGraph 状态变化 → Continuity 块更新但 system_text 不变
3. 单测：compact 后 Continuity/Topology 存活（扩展现有 test_t4）
4. 单测：resume 后 L2/L4 结构与断线前一致
5. 单测：experience 空位占位（无 experience 时字节偏移不漂移）
6. 全量门禁：`~/run_gate_r2c.sh` 375± 全绿（fmt/clippy/RT4_SOLO/test）

## 十二、cache telemetry 对比验收（点 2 三层拆分 + 裁决 4）

- **对比基线**：EC-01 的 41 请求（`docs/data/cache-ec01-20260828.jsonl`）
- **同口径强制**：同任务集（EC-01 的 4 任务原样）/ 同通道（Agnes）/ 同采集脚本（TelemetryProvider v2）/ 同运行姿势（HEARTH_ALLOW_NO_CGROUP=1）
- **三层指标分离测量（点 2——不得把 system 稳定等同于 full prompt stable）**：
  - **A. System Stability**：system_hash unique == 1 / session（ContextBuilder 直接可控）
  - **B. Message Prefix Stability**：后续 request 保持既有 message 序列为前缀，新增内容只出现在 suffix——**逐消息 hash 链**测量（裁决 4：与采集器 v2 的"相邻请求公共前缀长度"改进项合并落地，一个采集器同时服务 A 与 B，不另起炉灶）
  - **C. Provider Cache Outcome**：token-level hit rate + request-level hit rate（64.3% / 58.5% 基线）
- 验收目标：A 达成是施工直接断言；B 是结构正确性证明；C 是最终效果——**A/B 达成而 C 未改善时，结论是"稳定化正确但 provider 侧另有瓶颈"（MISS-E unknown 继续成立），不回退**
- 禁止跨口径比百分比；provider metadata 缺失保持 MISS-E unknown

## 十三、Intent benchmark B/C 对比复测（补充 1：口径声明）

- **cgroup 降级口径声明**：B 变体全部在 `HEARTH_ALLOW_NO_CGROUP=1` 下跑（无资源限制护栏）——本施工单复测同口径；未来 delegation 环境复测若出现差异，引用此 caveat
- 复测 = EC-03 的 5 任务 B/C 两变体重跑（10 跑），rubric 0-2 同表，与 EC-03 基线对照
- 关注：施工后 B 是否保持 T2/T3/T4 优势（行为零回归）；C 近似口径差异继续标注
- 顺带补 compact open 项：T2 多步任务若触发压缩即得样本

## 十四、rollback / fail-safe 分级（点 6）

- **env 开关**：`HEARTH_CONTEXT_BUILDER=v2`（分层）默认 / `=v1`（旧串行拼接）一键回退——v1 路径保留一个 release 周期
- **结构性 rollback（任一即回退）**：gate failure / system_hash 稳定性失败（同 session 唯一值 >1）/ message prefix 结构违反设计 / 明确 Intent·completion 行为回归
- **效果性 rollback candidate（观察，不立即回退）**：同任务/同通道/同运行姿势下 cache 指标**持续或显著**退化——**禁止把单次百分比波动直接作为 rollback 依据**（点 6）
- fail-safe：v2 组装器 panic（理论不可能——纯拼接）时由 env 开关回 v1 重跑

## 十五、证据纪律对照表（批示 §四 逐条落位）

| 表述 | 等级 |
|---|---|
| TaskGraph 是 cache miss 头号根因 | **likely**（不升级） |
| 方案 B 提升 cache | **待施工后实测**（不预写） |
| provider cache segment 粒度 | **unknown** |
| compact 前后 cache | **open** |
| 57GB 主犯 | **likely**（不写 confirmed） |
| E 变体 | **API 直连近似，≠ Codex harness**（后续引用保持此口径；真 Codex 对比需先修 relay+agnes-2.5-flash 配置，OPEN 不阻塞） |
| Experience task-stable / Task Continuity 换向正向 | confirmed（源码+EC-03 实测） |

## 十六、D 类排序备忘（引用，不实现——补充 2）

Goal Revision 三分类接入（第一，T5 用户伤害实证）→ bash stdout 截断（第二，57G 结构缺陷）→ filesystem accounting（第三，降级环境无兜底故优先级上调）→ resource 控制流（最后）。

---

**产出完毕，停在这里。** 本施工单经顶层批准后，由施工窗口按 §三 迁移点表执行；执行窗口在批准前不做任何 build_messages 改动。
