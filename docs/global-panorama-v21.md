# 全局透视盘点 — v21 时点（2026-08-01，维护期入口）

> **supersede `docs/global-panorama-v18.md`**（v18 写于 07-31，仅到 v18 WIP；本文刷新到 v21 已提交+维护期启动）。
> 作者：顶层架构角色 · 2026-08-01
> 判据延续铁律：**只认"生产路径可达"（grep 可证调用链），不采信提交信息/交付报告自报**。
> 本盘点基于真实主干：`git` HEAD = `4528fd6`（`v21.0`，已提交+tag）。所有代码事实现场 grep/读源码复核。

---

## 〇、版本状态（最重要的一句）

**锻造九轮（v12→v20）已收官，v21「刀入鞘轮」把系统擦净并启动维护期。主干停在 v21.0，维护期纪律已发布。** 任何"全景"若仍以 v11.5/v17/v18 为基线，都漏了 v19→v21 三次收口。

| 项 | 状态 |
|---|---|
| 最新 tag | `v21.0`（`4528fd6` 刀入鞘轮：消债固化 + 设计决策沉淀 + 维护期启动）|
| 工作树 | v21.0 提交干净；但树顶有未提交杂项（见 §7 整洁度）|
| 维护期 | **已启动**。规程 `docs/maintenance-protocol.md` 发布，季度体检 + 红线 + 不跑实验 |
| wiring | **14/14 全绿**（11 基础 + 3 新增，见 §3）|

---

## 一、轮次谱系（v12 → v21，全部已提交+tag）

| 版本 | 轮次名 | 主干实质贡献 |
|---|---|---|
| v12.x | 基准轮 | Rust 自动编程基准 0%→95%/100%；工具 rename（edit→write_file）；`ToolContext.cwd`→session workspace |
| v13.0 | 接线防火墙 | codex-xray + 接线断言（7 条 → 现 14 条）；constitution/civ 双 A 接 |
| v14.0 | 淬火轮 | `sub_budget` 毒债修复 + wiring 红线断言；应力场 24/24 零 panic；31 条回放素材归档 |
| v15.0 | 铸基轮 | 择脑（standard=deepseek 90%）；ReplayProvider 100% 零 token；应力场 22/24；天花板对照 |
| v16.0 | 封刀轮 | 清债 + 边界白皮书 + 锻造综述 |
| v17.0 | 生长轮 | 经验回路闭合验证（**仅 zhipu 弱模型有效 +20pt**，强模型域未证）|
| v18.0 | 精炼轮 | embedding 向量召回 + LLM 精炼 + 随机序 → **否定性发现**（强模型零增量/略负）|
| v19.0 | 深水轮 | **T13/T19 根因解剖**（planner `all_done` 误判）→ 经验自适应开关 + 基线锚定 92.5% |
| v20.0 | 收官轮 | planner 毒债修复（`write_attempted` 门控）+ 遗忘机制（prune/upgrade_core）+ 季度基线封存 |
| v21.0 | 刀入鞘轮 | 删 embedding 死代码 + 清 5 处硬编码 key（改 .env）+ 两篇设计决策 + 维护规程发布 |

⚠️ **经验回路的真相（跨 v17/v18/v19 实证）**：经验对**弱模型(zhipu) +20pt 有效**，对**强模型(deepseek, 生产标准脑) 零增量甚至 -5pt 有害**。结论 = 经验是**可选降级通道**（连续失败≥3 才注入），非常驻增强。v17 的"+20pt"是真实弱模型结论，错在**外推**。

---

## 二、基因层级系统（v12.1 确立，维护期不变）

出处 `docs/gene-expression-system-design.md:28-31`：

| 层 | 名称 | 牙齿 | 表达器官 | 硬上限 |
|---|---|---|---|---|
| G0 | 结构基因 | 物理不可能违反 | sandbox/landlock/seccomp、审批门、进程隔离 | 无上限，每条=拓扑级 |
| G1 | 反射基因 | 代码分支必然执行 | 相位钩子（subconscious/nervous/CostGuard） | 每条必挂 wiring 断言 |
| G2 | 检索基因 | 正确时机出现在眼前 | `experience` 判例卡+情境召回（**降级通道化**） | 无上限，受 KPI 淘汰 |
| G3 | 叙事基因 | **无牙齿（偏置）** | `constitution.md`→build_messages | **≤10 条，换血制** |

铁律：入籍三元组（层级/表达器官/验证方式）；**单向上升不许空降**；G3 加第 11 条先废一条。

---

## 三、器官地图（v21.0 已验证）

> 验证口径：grep 存在性 → 读控制流确认行为 → 确认表达力。

| 器官 | 状态 | 备注（v21 时点）|
|---|---|---|
| `agent-core`（loop.rs） | 🟢 | `LoopPhase{Init,Plan,Act,Observe,Reflect,Done,Error}`；**planner `all_done` 已修**（:986 `write_attempted` 门控，无写则 replan）|
| `constitution` | 🟢 | 运行时读 `constitution.md`，超 6000 静默截断 |
| `experience` | 🟢 | G2 层。**降级通道化**：`consecutive_errors>=3` 才注入（v19）；`prune()`(:228)/`upgrade_core()`(:248) 遗忘机制已接线（v20）|
| `project-xray` | 🟢 | 接线断言 **14 条**（CLI spec 硬编码 `docs/xray/wiring-v13.toml`，main.rs:36）；新建 wiring-vNN.toml CI 不读 |
| `nervous-system` | 🟡 | `query()`→NerveAction；唯一调用点 loop.rs:1589（do_reflect 内）→ 无会话即零感知 |
| `subconscious` | 🟡 | `SubconsciousGate::check()` 接 loop.rs:991（do_plan 前）|
| `sandbox` | 🟢 | landlock+seccomp，直接 exec+pre_exec 套 NO_NEW_PRIVS（VM 禁 unshare）|
| `service` | 🟢 | **embedding 死代码已删**（v21）；**硬编码 key 清零**（5 处改 `.env`，v21）；6 provider 注册 |
| `resource-monitor` | 🟢 | `is_critical()`=mem>80%||disk<1GB |
| `llm-gateway` | 🟢 | trait `LlmProvider`，方法 `chat()`（**无 `generate()`**）|

**wiring 14 条能力断言清单**（id @ 行号）：
`tool-exchange-wired`(17) `readonly-view-strips`(33) `subagent-uses-readonly`(48) `tool-exchange-pairing`(59) `self-verify-in-prompt`(70) `constitution-reads-file`(81) `civ-auto-written`(97) `experience-persists`(118) `cost-guard-live`(134) `experience-adaptive-switch`(155) `all-done-requires-write`(166) `experience-prune-wired`(177) `sub-budget-not-halved`(188) `replay-provider-wired`(200)

---

## 四、债务台账（v21 刷新）

### ✅ 已清（v21 刀入鞘轮）
- 🔴 embedding 死代码：`zhipu_embed_fn`(~44 行)+`set_embed_fn`/`EMBEDDING_ENABLED` 接线+`reqwest` 依赖 → **全删**（grep 复核 0 命中）
- 🔴 硬编码 API key **5 处**（ZhiPu×3 + deepseek×2）→ **全部改 `.env` 读取**，仓库 grep `5ad30`/`REDACTED_DEEPSEEK_KEY` = 0 命中
- 🔴 v20 planner `all_done` 半成品（工作树未提交）→ **已提交 v20.0 + v21 补断言钉死**

### 🟡 仍开（v21 未触碰，维护期非紧急）
- ❌ `/readyz` 假实现（`routes.rs:255` 不验真实就绪即返成功）
- 🔴 `drain_nervous_alerts()`（`loop.rs:422`）**零调用者** → 中断日志生成后被丢弃（v17 债务台账遗留）
- 🟡 `is_critical=Abandon|DeliverAndQuit` → `loop.rs` 的 `Simplify` 分支**不可达**（nervous 侧 Simplify 可达，但 loop 侧空挡仍在）
- 🟡 **幽灵文档**：`quarterly-baseline-q3-2026.md` 不存在，但 `maintenance-protocol.md` §7 与 v20 CHANGELOG 引用它（见 §6）

> 判读：上述仍开债务均**不触发维护期红线**（通过率/应力/wiring），属"有益可等"，不阻塞维护期。

---

## 五、v19/v20/v21 深度盘点（锻造期收口的三刀）

### 5.1 T13/T19 六轮全挂的真凶 = planner 判定缺陷（非能力墙）
- v19 解剖：agent 只 grep+read，**从未 write_file/edit 就 self-report Done**；runner 对照同任务 → `FAIL(TEST_FAIL)`（代码真没改）。
- 根因 `loop.rs`（v20 修复前）：`all_done` 把 Read 节点 Completed 当任务完成。
- 修复：:`986` `if self.write_attempted { Done } else if replan_count<3 { replan }`。**T13 0/8→2/3**。
- wiring `all-done-requires-write`(:166) 锚定。剩余 T19(`NO_GENERIC_FN`) 是**真实工具链能力缺口**（泛型合并函数），与 all_done 无关。

### 5.2 经验系统降级通道化
- v17(zhipu)+20pt / v18(deepseek)-5pt → 自适应开关 `consecutive_errors>=3` 才注入（v19）。
- wiring `experience-adaptive-switch`(:155) 锚定；`experience-prune-wired`(:177) 锚定遗忘机制。
- 维护铁律（见 `design-decision-experience.md`）：不改回无条件注入、不重投 embedding/LLM 精炼。

### 5.3 擦枪入鞘（v21）
- 删死代码 + 清 key（§4 已清）→ 系统"脏血擦净"。
- 两篇设计决策把"为什么"写死 → 维护期有人想回退时有红线拦截。
- 维护规程发布 → 漂移终止机制就位。

---

## 六、维护期操作规程 + 缺口

**`docs/maintenance-protocol.md`（v21 发布）**：
- 季度全量体检（每年 1/4/7/10 月）：基准 deepseek 20×2 固定序 **90%±5%** + 应力 **0 panic** + 回放 **100%** + wiring **14 条**
- 按需增量（新功能加基准题+wiring；修 bug 跑受影响题）
- 红线表（通过率<85% / 应力 panic>0 / wiring 断 / 数据造假）
- 明确**不跑**：经验实验/embedding/LLM 精炼/正交矩阵

🔴 **缺口（需补）**：规程 §7 索引 + v20 CHANGELOG 引用的 `docs/quarterly-baseline-q3-2026.md` **不存在**。v20 声称"季度基线封存"但产物未落地 → 维护规程的"封存基线"无承载物。建议补一份（含 14 条 wiring 摘要 + 90%±5% 基线值），或在规程 §7 删除该引用。

---

## 七、工作树整洁度（维护期提示）

`git status` 在 v21.0 提交之上仍有未提交杂项：
- `.workbuddy/memory/2026-07-31.md` / `MEMORY.md`（本窗口跨轮编辑，属项目记忆，非业务代码）
- `bench/results/matrix-v13.stdout.log`（零散 bench 产物）
- `docs/external-advisor-briefing-v13.md`（项目文档改动，未提交）

> 注意：**v21.0 这个 tag 本身是干净的**；上述是工作树叠加物。维护期应养成"季度体检时顺手清理未提交杂项"的习惯，避免基线混淆。

---

## 八、全局战略判断（telos 过滤器收口）

终点（见 `docs/top-level-telos-anchor.md`）= **长周期可信委托 + 不可绕过硬边界**。

- **锻造期（v12→v21）已把"会写代码的 agent"收敛成具备**：不可绕过硬边界（G0 sandbox + 14 条 wiring 锚定）、planner 真 bug 已修（T13/T19 根因）、经验降级通道（不污染强模型）、零硬编码 key（密钥入 .env）、遗忘机制（经验不无限膨胀）。
- **漂移已止**：维护期纪律（季度体检 + 红线 + 不跑实验）正是 telos 过滤器的常态化——没有终点函数时的局部最优漂移，被"季度只体检、不加功能"钉死。
- **对外 RFC（CN-001 / RFC-004 v3 / RFC-005 替代路径）**：仍**暂停**。维护期除非出现可测量靶心（如某红线被触发、或人硅边界测试暴露 G0 漏洞），否则不重启治理 RFC——它们属"有益可等"，不占带宽。

---

*本盘点所有代码事实均在 `4528fd6`(v21.0) 现场 grep/读源码复核（2026-08-01）。v17/v18/v19 实验结论来自 `bench/v18-e{0,1,2,3}.log` 与 `self-evolution-v19-report.md` 原始记录。*
