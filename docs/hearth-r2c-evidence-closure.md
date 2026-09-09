# Hearth R2-C Evidence Closure（EC-01~08 · 2026-08-28）

> **依据**：《R2-C 顶层评审结论》（Evidence Closure 先行，ContextBuilder 暂不施工）+ 守门员批注 1-5 + Claude Tier3 任务书（两线协调：Tier3 修复先行，采集机制稳定后采数——已按此排期执行）。
> **基线**：v0.2.8 / HEAD `7066ab3`（Tier3 T2/T3/T4 修复已合入）。
> **结论速览**：Cache 数据 41 请求已闭合（token 级 64.3% / request 级 58.5%）、miss 分类 A-F 已落、TaskGraph 拆分裁决 = 方案 B（拓扑 stable + 状态 dynamic）、Experience 三分类已分析、57GB 已二分（payload 逃逸 + filesystem artifact 并存嫌疑，ResourceLedger 边界已明确）、bash 截断设计已出（D 类待单）、Goal Revision 规则级解法已含"查看状态"回归断言。

---

## A. Cache（EC-01/02）

### A.1 采集（v2 采集器，Tier3 修复合入后）

- 原始数据：`docs/data/cache-ec01-20260828.jsonl`（41 行）——4 任务（2×写文件 + 1×读改 + 1×resume 续做）全 rc=0
- 覆盖：plan / act / tool call / resume（**compact 前后未覆盖**——本轮任务均未触发 compaction 阈值，如实标注为 open item）
- **指标分离**（补充 5 schema 约束）：**token-level 为主指标**（Agnes 上报 `prompt_cache_hit_tokens/miss_tokens`），request-level 为派生参考（hit>0 的请求占比）——两栏分列不混算

### A.2 结果

| 指标 | 值 |
|---|---|
| **token-level 命中率** | **64.3%**（46464 / 72308） |
| request-level 命中占比 | 58.5%（24/41 请求 hit>0） |
| system_hash 唯一值 | 3 个 session 分别 3/6/17 个（应=1）——**system 不稳定在 Tier3 修复后仍存在** |
| 高命中请求特征 | msgs=2、prompt≈483、hit=384（小请求） |
| 零命中请求特征 | msgs≥3、prompt≥2400（**正式组装请求全 miss**） |

### A.3 miss 分类（EC-02，证据等级标注）

| 类 | 假设 | 证据 | 等级 |
|---|---|---|---|
| MISS-A system_text 变化 | system 每步被 TaskGraph 状态/experience/LSP 污染 | system_hash 17 唯一值/27 请求 + 零命中集中在大请求 | **likely**（provider 未上报 cache 段位置，无法 confirm 到字段级） |
| MISS-B tool schema 变化 | 工具集恒定 | schemas 注册后不变（源码审查） | unlikely |

> **【勘误 · 2026-08-29 Final Closure】**：MISS-B 拆分修订——tool schema **内容**变化 = unlikely（维持）；tool 数组**顺序**漂移 = **confirmed 污染源**（HashMap 迭代无序，已修 `list_tools` 排序）。pre/post 干净对照（`data/cache-closure-deepseek-20260829.jsonl` vs `data/cache-post-toolssort-20260829.jsonl`）：主链 token 级 64.9%→**74.5%**、request 级 74.1%→**88.9%**——顺序漂移是真实污染源之一，MISS-E（provider 粒度）权重相应下调但未消除（命中率未达理论值，replan 换图重建仍在）。
| MISS-C history 增长 | 正常逐条增长 | 每请求 msgs+1 | confirmed（正常行为非缺陷） |
| MISS-D serialization 差异 | serde 字节确定性 | 同内容重算 hash 一致（单测锁定） | unlikely |
| MISS-E provider 最小前缀/粒度 | Agnes token 级语义未知 | 无 provider metadata | **unknown** |
| MISS-F 其他 | — | — | unknown |

**诚实声明**（批示 §四）：TaskGraph 注入 = 稳定前缀污染的**强嫌疑源**（confirmed：它每步变化且在 system 内），但"它就是 cache miss 头号根因"**仍是 likely 而非 confirmed**——需要 provider 级 cache 段位数据（MISS-E unknown 拖累）。

## B. Intent（EC-03）

5 任务 benchmark 的用例定义已在 R2-C 设计单 §9 落定。**本轮执行状态：未跑**——原因如实披露：Tier3 T2/T3/T4 修复占用 VM 串行窗口（批注 1 顺序裁决），41 请求采集优先完成（EC-01 是 benchmark 的前置数据基线）。**下轮第一项**：5 任务 × A/B/C/E 变体（Agnes 配额充足），变体 C 用手工删减 prompt 近似（批注 2 允许）。

## C. Context（EC-04/05）

### C.1 TaskGraph 拆分比较（EC-04）——**裁决：方案 B 优先**

| 维度 | 方案 A（整图动态） | 方案 B（拓扑 stable + 状态 dynamic） |
|---|---|---|
| token cost | 每请求重复全图（含已 Completed 节点） | 拓扑一次 + 尾部仅增量状态（更省） |
| cache stability | 每步全变 | **拓扑部分跨 replan 稳定**（deps/描述不变时） |
| 模型理解 | 全图可见 | 拓扑+状态分离仍可拼合（Continuity 块已含 completed/remaining/next） |
| 实现复杂度 | 低（现状即 A） | 中（拆分注入两处） |

**建议**：方案 B——node id/description/deps 是结构事实（task-stable），status/result 才是高频变化；B 在 cache stability 与 token cost 双优，且 Task Continuity 块已天然承载状态面（R2-D 基建复用）。**证据等级：likely**（cache 收益待 ContextBuilder 施工后实测 confirm）。

### C.2 Experience 稳定性分类（EC-05）

> **【勘误 · 2026-08-29】**：本节 "task-stable confirmed" 结论**已被推翻**——源码实测（loop.rs:516/:787/:1983/:1997/:3089）证实 `injected_experience` 是**错误降级通道**（连续错误≥3 才检索、每轮清空重填）= 条件性 turn-dynamic，非 task-stable。ContextBuilder v1.1/批准书采用**方案 X**（保持降级语义、注入位置移 L4 尾部）。见《顶层批准：R2-C ContextBuilder Construction Order v1.1》。


实测 `injected_experience` 机制（loop.rs）：按 goal 检索注入、**任务期间不重新检索**（首轮定生死）→ 天然 **task-stable**（任务中途不变化）——**不应放 turn-dynamic 尾部**，应放 stable 区（L2 Project 层）。若未来引入任务中途刷新，则降级 task-dynamic。**证据等级：confirmed**（源码行为）。

### C.3 Task Continuity budget

Continuity 块当前含原始目标全文+列表（~300-800 tokens 视 graph 大小），尾部注入不破前缀——budget 可控（<1000 tokens），无需拆分。confirmed。

## D. Resource（EC-06/07/08）

### D.1 57GB 二分（EC-06）

| 路径 | 分析 | 结论 |
|---|---|---|
| Tool Payload Explosion | bash `format_output` 无截断：stdout 全量 → String → ToolResult → history → 同时进内存+LLM 请求 | **confirmed 结构性缺陷**（无上限）；57G 规模更符合此路径反复触发 |
| Filesystem Artifact Explosion | agent 写盘文件（write_file/脚本产物） | 可能并存（node 脚本产物）——现场已清无法精确归因（R2-D 已披露） |

**最终口径**：57GB = **两者叠加的高置信事故**（payload 无上限 confirmed + artifact 现场丢失 unknown），"哪个为主"标 likely（bash 反复跑 node 的日志特征指向 payload）。

### D.2 ResourceLedger 观测边界（EC-07）——**必须承认盲区**

当前 Ledger 记 **args+result bytes = Tool Payload Bytes**；**Filesystem Artifact Bytes（bash 脚本实际生成的 50G 文件）完全不可见**。设计裁决：**两个独立观测维度**——payload 由 dispatcher 记（已落），filesystem 需第二维度（候选：sandbox 层 landlock 写路径挂账目、或 bash/write 工具返回后 du 工作区增量）——**本轮只落设计，filesystem 维度施工待单**。

### D.3 bash 输出截断设计（EC-08，只设计不实现控制流）

```text
max_stdout_bytes = 64KB（Codex 同量级）
max_stderr_bytes = 16KB
strategy = head 32KB + tail 32KB（保头尾——错误常在尾部，开头是命令回显）
truncation metadata = "[truncated: total X bytes, showing head/tail Y]"（LLM 可见）
artifact reference = 超限时全量落 /tmp/hearth-out/<call_id>.log + 路径回传
（LLM 需要时可 read 该文件——观测完备不丢信息）
```
**控制流（超限 kill/pause/deny）= WP-0 D 类，另出单。**

## E. Architecture 建议

1. **ContextBuilder**：方案 B（C.1）+ TaskGraph 状态挪尾部 + experience 上移 stable——施工单待本文件评审。
2. **Goal Revision**（补充 4 回归断言）：三分类规则集**必须含**："查看状态/看 diff/发生什么了/继续/怎么樣了"→ Task Control（不动 revision）。对 t_revision.log 案例的断言：`apply_turn_goal` 前置分类后，"查看状态"输入 revision 保持不变。**本轮未改 apply_turn_goal（D 类红线）**——分类函数设计为纯函数 `classify_user_input(&str) -> InputClass`，施工单落地时接入。
3. **Resource Safety**：Ledger（已落）+ filesystem 维度（设计已出）+ bash 截断（设计已出）——三件套齐，施工顺序建议 bash 截断优先（结构性 confirmed 缺陷）。

## F. Decision 汇总（全项）

| 项 | 决策 | Evidence |
|---|---|---|
| TelemetryProvider v2 | ADOPT（已施工） | confirmed（41 请求全出口覆盖） |
| ResourceLedger（payload 维） | ADOPT（已施工） | confirmed |
| Filesystem write 维度 | ADAPT（设计出，施工待单） | likely |
| TaskGraph 方案 B 拆分 | ADAPT | likely |
| Experience 上移 stable | ~~ADAPT~~ **勘误：方案 X 移 L4（降级通道语义）** | ~~confirmed~~ **已推翻——见 §C.2 勘误** |
| bash 输出截断 | ADAPT（设计出，D 类施工待单） | confirmed（结构性） |
| Goal Revision 三分类 | ADAPT-DEFER（D 类） | likely |
| Bridge | DEFER（INTENDED） | — |
| Intent benchmark 执行 | DEFER→下轮第一项 | — |
| compact 前后 cache 覆盖 | open（未触发阈值） | unknown |

## G. 通过标准核对（批示 §十五 十条）

1✅ Cache v2 41 请求 2✅ miss 分类 A-F 3⚠️ Intent 最小集**未跑**（下轮第一项，已排期）4✅ TaskGraph 方案 B 有证据选择 5✅ Experience 判断完成 6✅ 57GB payload/filesystem 二分完成 7✅ bash 输出爆炸明确诊断 8✅ Ledger 边界明确（payload≠filesystem）9✅ Goal Revision 分类无语义冲突（含反例断言）10✅ D 类全列出（bash 控制流/Revision 门/filesystem 维度/429 外控制流）

**结论**：9.5/10——Intent benchmark 执行是唯一未闭合项，不阻塞 ContextBuilder 施工单的**评审**，但施工前必须补跑（其结果可能微调 L4 层设计）。
