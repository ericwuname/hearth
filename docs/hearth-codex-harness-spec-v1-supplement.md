# 《Hearth × Codex Harness 对标与后续优化迭代规格 v1》需求资料补充

> **性质**：守门员对规格 v1 的源码级核实与需求资料补齐——供规格作者修订 v1 → v2。
> **方法**：源码直读实测（每条结论带 file:line 锚点），不采信二手报告。基线：commit `12c94ad`（v0.2.4）。
> **核心结论**：规格的决策框架（ADOPT/ADAPT/DEFER/QUARANTINE + S/A/B/C 分级 + 横向测试）全部成立，但**基线过时**（写成 v0.2.3）且**缺 Q1 答案**——多项 S 级能力已存在或已完成，排序需要重排。

---

## 1. 基线勘误（最高优先级修正）

规格头注：「当前 Hearth 基线：v0.2.3，审查基线 `f0b4a54`；H1–H10 已形成下一轮地基修复路径」——**已过时**。

| 事实 | 实测 |
|---|---|
| 当前版本 | **v0.2.4**（`Cargo.toml:34`），commit `12c94ad`（2026-08-27，26 files / +2047−41） |
| H1–H10 状态 | **全部已合入**，不是"下一轮路径"。VM 门禁 fmt/clippy/test RC=0，**351 passed / 0 failed**（`~/t_gate.log`） |
| 规格正文随之失效的表述 | "H9 正在补救"（已合入，`patch.rs:104/173`）、"H2 已经修"（正确，已合入）、"H8 我建议继续做"（已合入 `report.rs`） |

**对 S 级排序的直接冲击**：S6（Deadline/Budget）的 deadline 部分已完成（H2：`context.rs:74-90` + `loop.rs:2770-2795`，CLI 默认 900s，`HEARTH_TASK_TIMEOUT_SECS` 可覆盖）；S8（Reliable resume）已有真接线（rebuild_agent + restore_history + restore_task_graph，WS11）；S9（Verification→Done）已有（WS13 写盘后三重校验：存在性→字节数→可运行）。**这三项应从"必须做"移到"已具备/持续加固"。**

## 2. Q1 逐项回答（规格 §34 要求先答"当前有没有"）

| S/A 项 | 当前状态（源码实测） | 真正缺口 |
|---|---|---|
| S1 Inline Approval | **已实现**。本地直跑模式：`NeedApproval` → stdin y/N 内联裁决 → `resolve_interaction` 继续当前 Turn（`run_local.rs:619-720`）；remote REPL：SSE 监听 + 等待输入 + 30s 超时自动 deny（`repl.rs:166-187`） | 不是"实现 inline"，而是：①审批策略矩阵（S5）②remote 模式 30s 超时偏短且 auto-deny 语义粗暴 ③与 clarify 统一进 WP-0 `InteractionRequest`（原语已落地：`api/lib.rs:155`，范例 `loop.rs:1865`） |
| S2 Clarify | **已有雏形**。planner derive_gaps → `InteractionRequested{kind:"clarification"}`（`loop.rs:1286-1301`）；本地回答路径 `run_local.rs:688`。T8 选项丢空格 bug 已修（v0.2.3 `f0b4a54`） | 规划期批量收齐 + 收齐后 resume 的完整链路（= 路线图 B2 深化，第一梯队） |
| S3 Tool lifecycle | **部分**。dispatcher 有超时窗口（默认 30s + v0.2.4 `declared_timeout`，`dispatcher.rs:30/104`）、ApprovalState 四态 session 键控 | 无统一 `ToolInvocation{id,status,start_at,end_at,result}` 结构。注意：`EnvelopedEvent` 已有 seq/span_id/parent_id，**优先复用信封承载，不另起炉灶** |
| S4 Context Builder | **未统一**。system_text 在 `build_messages` 手工拼接（`loop.rs:956-1042`：coding-agent 块 + constitution + 天赋 + Hearth.md + 8 工具 schema） | 正是 T4（提示词瘦身）缺乏结构性支撑的根因。**建议 S4 与 T4 合并为同一施工项**，避免瘦一次身再重建一次 |
| S5 Sandbox/Approval 正交 | 方向已对（G0 沙箱与 ApprovalState 物理分离） | 无显式策略矩阵/策略配置——审批行为硬编码在 loop/dispatcher，"什么时候要审批"不可配置 |
| S6 Budget 分层 | deadline ✅（H2）、steps ✅、transient 重试上限 ✅（≤4/120s，`loop.rs:1636-1649`） | token budget、same_effect_repeat_guard **不存在**——规格此建议有价值，采纳 |
| S7 Structured diff | **无 /diff**。H8 报告含产物文件列表（`.hearth/reports/`） | 高收益低成本；但按事实产生权公理，diff 数据必须后端算（复用 H8 的 written_files 登记 + git 查询在沙箱内做） |
| S8 Reliable resume | 已有（WS11 自动落盘+恢复） | P2-6"工具完成但结果未落盘→crash→副作用重复"未做——规格建议提级到 P1 边缘，**守门员同意提级** |
| S9 Verification | 已有（WS13） | 持续加固即可 |
| S10 Network permission | **未做 = T11**，任务书已备（`docs/hearth-manualtest-v023-taskbook.md` T11：deny 时发 `InteractionRequested{kind:"egress_allowlist_request"}` → approve 落盘 config） | 可直接施工；前置 T10 已完成（`egress_allowlist` 注入链已修） |
| A1-A3 /status /compact /diff | REPL 斜杠命令现状：`/quit /sessions /history <sid> /status <sid> /cancel <sid> /file <path> /help`（`repl.rs:39-87,316-344`）——**全部带 session 参数，属"管理查询"而非"当前线程控制"** | A 级的真实工作 = 把它们改成无参"当前线程"语义 + introspect 数据复用（T3 后油箱表已暴露 history/system/total 三栏） |
| A7 Fork | 无 | session 模型无 fork；DEFER 合理（单人体验优先） |
| A9 Project instructions | **HEARTH.md 已存在**（v0.1.5 起，`loop.rs:124-129`：`{cwd}/Hearth.md` 优先 + 家目录兜底，注入 system prompt） | 规格说"新建 HEARTH.md"是**事实错误**——真缺口是 Codex 式"全局→项目→子目录"逐级作用域与覆盖语义，应改为"扩展 scoping" |
| A10 Skills | 有天赋基因 + template + experience，无 Skill Descriptor/Loader | 骨架级三件套建议合理 |
| A12 doctor | 无 doctor 命令；有 `hearth config`（get/set，v0.2.4 H10 补 egress-allowlist 输出） | 合理新增 |

## 3. 规格中需要修正的其他事实

1. **H-A08 的警告恰好命中当前实现**：H3 归档（v0.2.4）当前正是"所有 session 堆进一个无 scope 共享文件"——`~/.config/hearth/archive/compacted.jsonl` 单文件追加（`context.rs:217-233`）。警告成立，应列为 H3 后续任务：归档文件按 session 分片或每行加 session_id 字段。
2. **H-A05 现状补充**：apply_patch 是 SEARCH/REPLACE 单文件编辑（事务性整片拒绝，`patch.rs:1-55`）+ H9 宽松匹配档；**无 Add/Delete/Move hunks**——Add 靠 write_file 兜底，**Delete/Rename 没有任何工具**（规格"最低标准"里的真缺口）。
3. **§29 格式破损**：`按：```text价值 × 成熟度 × 维护成本`——代码围栏断裂，修订时修复。
4. **CLI 命令清单勘误**（对照 §28）：实际子命令 = chat / repl / config / init / note / setup / resume / sessions / history / approve / deny / cancel / status / tools / replay 等（`lib.rs:59-154`）；规格清单里的 `civ` 等需逐个核对，`whoami/template` 存在与否以 lib.rs 为准。

## 4. 施工必须遵守的项目铁律（规格未提，缺了会被打回）

| 铁律 | 对规格的影响 |
|---|---|
| **WP-0 四类法**（A 呈现 / B 旧事实新视图 / C 新事实 / **D 新人类介入点=改控制流**） | 审批策略矩阵（S5）、阻塞式写前确认、same_effect_repeat_guard 触发 replan，全是 **D 类**——须先顶层任务书，不能直接施工 |
| **事实产生权公理**（后端产事实，前端投影；数据会被引用→后端独占） | S7 /diff、A1 /status、§39 指标统计必须后端算；禁"前端先本地算以后挪后端" |
| **门禁**：fmt/clippy/test 全 RC=0 + VM 真机证据 + 实现率≥0.9 + 🔴=0；**commit message 门禁自述不可信**（f0b4a54 实测 TEST_RC=101 教训，以 `~/t_gate.log` 为准） | 每个验收项要求"落盘 gate log"而非口头/commit 自述 |
| **冻结区**：demo-v21j-frozen（UI 39 断言）、genes-next.md（PAUSED）不可动 | A 级 CLI/UX 改造不触 UI 冻结区；REPL 斜杠命令属 CLI 层，安全 |
| **VM 真机验证**：本地 Windows 无效，Linux VM（Ubuntu 24.04）实测；沙箱 git 分支名禁带斜杠 | §39 横向测试必须在 VM 跑 |
| **外部 AI 行号锚点必须实测**（此前外部 AI 曾引用超文件长度的幻觉行号） | 规格 v1 无行号引用（安全）；v2 建议全部结论带实测锚点 |

## 5. 规格未提、但可直接复用的资产

1. **bench/（295 文件）**：已有任务集（tasks/）、结果、红队脚本——§39 的 10 个横向任务可直接映射进 bench/tasks，不必新建测试基建。
2. **Delegation Friction（§40）的计量不靠人工**：H8 执行报告已记录 approvals 字段 + `InteractionRequested/Response` 事件流 + 审批超时 auto-deny 计数——"用户干预次数"可从落盘数据自动统计。建议定义：`Delegation Friction = 人工干预次数 / 任务数`，先跑 5 个手工任务拿 baseline。
3. **project-sync crate（45 测试）**：EVENT_LOG 哈希链 + Task Ledger——Phase E 横向对比结果可入此账本，防篡改、可审计。
4. **需求总账**：`docs/requirements-ledger.md` 已存在——§35 的 Benchmark Matrix 应并入总账，不要另开平行文档（防止双头账）。
5. **隔离区落地建议**（对 §5 的工程化补充）：`quarantine/` 代码**不加入 workspace members**（`Cargo.toml` 27 crates 是显式列表，不加即零编译成本），配 `docs/quarantine-register.md` 总账 + 每项五元组（why_quarantined/original_use_case/known_failure/reentry_condition/related_commit）。比 `_quarantine` 目录前缀更可靠，因为是编译器强制而非约定强制。

## 6. Phase 排序修订建议

规格 §38 的 Phase A（地基 P0 收口）**实际已完成**（v0.2.4）——直接进 Phase B。Phase C 的 S 级重排：

```text
移出 S 级（已完成）：S6(deadline 部分)、S8、S9
新 S 级施工序：
  S1' 审批策略矩阵（D 类→先出任务书）
  S2' Clarify 批量收齐+resume（=B2 深化）
  S3' ToolInvocation 统一模型（复用 EnvelopedEvent）
  S4' Context Builder（与 T4 提示词瘦身合并施工）
  S10 Network permission request（任务书已备，可直接派）
```

## 7. 数字基线（v2 引用时更新）

27 crates / 91 rs 文件 35,124 行 / 351 测试函数（VM passed 351）/ 225+ commits / 版本 v0.2.4（`12c94ad`）/ 手工测试已验证：provider 选择、错误分类 Fatal 化、上下文三栏度量、沙箱 /dev/null+node/python3、出网白名单注入链。

---

*本补充由守门员于 2026-08-27 依据源码直读实测编写；每条状态判断均有 file:line 锚点，可复核。*
