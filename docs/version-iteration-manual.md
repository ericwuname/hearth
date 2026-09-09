# 版本迭代说明书（Version Iteration Manual）

> **这份文档的目的**：让"每个版本干了什么、做了哪些、没做哪些"一目了然，随时间推移不遗忘。
> 它是项目的**全局视角锚点**——顶层窗口每次刷新全局认知、规划下一版前，先读它。
> **基线事实（写于 2026-08-02 刷新）**：主干 HEAD = `8e39775`；已打 tag 的最末版本 = **`v22.0`**（重打指向 HEAD，含 Y + Docker 全部改动）；`v22.0` 已发布（✅ 双引擎收口稳态）。
> 所有"做了/没做"均以真实 CHANGELOG + 守门员审计 + 测试报告核验，非凭印象。

---

## 〇、更新纪律（必读）

**触发条件**：每次以下任一发生，顶层窗口**必须**在此文档追加对应版本一节，并更新文末「全局收口表」与「基线事实」：
1. 打出新版本 tag（`git tag vXX.X`）；
2. 发布版本说明 / CHANGELOG；
3. 进入维护期后的"季度体检"或"按需增量"完成并归档。

**更新规则**：
- 每版必须填齐三项：**做了什么（增量）** / **没做哪些（范围外·已知债）** / **关键决策·红线状态**。
- 增量描述引用真实文件:行号或测试项（T1–T7 / ST1–ST8），不写空话。
- 未做项若已在后续版本完成，回原版节标注 ✅ 并链到完成版。
- 本说明书与 `maintenance-protocol.md` 关联：维护期每次版本动作，checklist 含"更新本说明书"。

---

## 一、版本时间线总览

| 版本 | tag | 轮次名 | 一句话增量 | 状态 |
|---|---|---|---|---|
| v12.6/v12.7 | `v12.7` | 基准轮 | 通过率 0%→90%，修 4 根因 + 2 失败项 | ✅ 已发布 |
| v13.0 | `v13.0` | 接线防火墙 | codex-xray 7 条红线 + civ/constitution 接 | ✅ 已发布 |
| v14.0 | `v14.0` | 淬火轮 | sub_budget 毒债修复 + 应力场 24/24 零 panic | ✅ 已发布 |
| v15.0 | `v15.0` | 铸基轮 | 择脑 deepseek 90% + ReplayProvider 零 token | ✅ 已发布 |
| v16.0 | `v16.0` | 封刀轮 | CostGuard 真值接地 + 边界白皮书 | ✅ 已发布 |
| v17.0 | `v17.0` | 生长轮 | 经验回路闭合（仅弱模型有效 +20pt） | ✅ 已发布 |
| v18.0 | `v18.0` | 精炼轮 | embedding + 精炼 → **否定性发现** | ✅ 已发布 |
| v19.0 | `v19.0` | 深水轮 | T13/T19 根因解剖 + 经验自适应开关 | ✅ 已发布 |
| v20.0 | `v20.0` | 收官轮 | planner 毒债修复 + 遗忘机制 | ✅ 已发布 |
| v21.0 | `v21.0` | 刀入鞘轮 | 删死代码 + 清 5 处 key + 维护规程 | ✅ 已发布 |
| v22.0 | `v22.0` | 验证/工程化轮 | 全链路测试达标 + 双引擎收口（EPIC-B/C + Y 方向 + Docker 验证） | ✅ 已发布 |

---

## 二、逐版本详节

### v12.6 / v12.7 — 基准轮（tag `v12.7`）
**一句话**：把"会写代码的 agent"从通过率 0% 拉到 90%，打通主干。

**做了什么（增量）**
- 根因 A：工具结果从不回写会话历史（grep 死循环真凶）→ 新增 `record_tool_exchange()`，在 `do_act` 写入 `pending_results` 后调用。
- 根因 B：每相位重复 decompose（性能根因）→ 加 `needs_decompose` 标志，仅首次 / `ReflectVerdict::Replan` 后 decompose。
- 根因 C：system prompt 工具名写错 → 提示全改 `read` / `write_file(path, content)`。
- 根因 D：bench harness 误判 TEST_FAIL → 20 个 fixture 的 `Cargo.toml` 追加 `[workspace]` 隔离。
- 修复 1：子智能体并发写同一文件（架构级真 bug，T10）→ 写操作归主、子智能体只读（`MUTATING_TOOLS` + `read_only_view()` + 断言 `test_read_only_view_strips_mutating_tools`）。
- 修复 2：T15 测例本身自相矛盾（不是 agent 错）→ 改自洽测例。

**没做哪些（范围外·已知债）**
- 子代理预算仍 `sub_budget=7` 被砍半（留 v14 修）。
- civ 未自动写（留 v13）。
- 沙箱写白名单未独立覆盖只读视图（后续）。

**关键决策 / 红线状态**：写操作归主、子只读 → **G0 边界雏形**确立。

---

### v13.0 — 接线防火墙（tag `v13.0`）
**一句话**：把 v12.7 三项修复 + 两条烂账决策锁死成"接线断言"，防火墙可自证。

**做了什么（增量）**
- 接线断言规格 `docs/xray/wiring-v13.toml`（schema=1，**7 条**，全 red）：锁死 tool 回写 / 只读视图 / 下标配对 + constitution 读文件 / civ 写。
- civ 自动写入（依赖倒置）：`agent-core` 新增 `CivWriter` trait；`do_observe`/`do_reflect` 经 `civ_note()` 真写文明线；`service` 经 `CivWriterAdapter` 注入 `CivilizationStore`（VM 实证 11 条 Milestone）。
- CI 删死的 `cargo test -p telemetry`，新增 xray `wiring` + `scan` 两 step（第四门）。
- VM gate 新增 codex-xray 第四门（xray_ok），四门全绿。

**没做哪些（范围外·已知债）**
- `sub_budget` 真实预算（v14-1）。
- 应力场工装（v14）。
- 经验降级通道（v17+）。

**关键决策 / 红线状态**：**接线自证范式确立**——后续 T3 验证"断线变红、恢复变绿"即源于此。

---

### v14.0 — 淬火轮（tag `v14.0`）
**一句话**：防回退 + 应力场，把"偶然达标"变"必然达标"。

**做了什么（增量）**
- wiring 第 8 条红线 `sub-budget-not-halved`（severity=red）：防该修复被回退即 CI 断裂。
- 应力场工装 `bench/stress_v14.py`：ST1–ST8 × 3（token 饥饿 / 恶意目标 / 空项目 / 300 并发 burst / 磁盘满 tmpfs / 乱码输入 / 会话隔离 / 审批门），全部判据源码核实 → **24/24 零 panic**。
- `sub_budget=7` 修复（v14-1），子代理真实完成预算（解决 T09/T13/T18 偶发）。
- 基准 zhipu 20×2：77.5%（主表）/ 87.5%（预算修正口径，≥85% 红线过）。

**没做哪些（范围外·已知债）**
- zhipu 77.5% 未达 90% 目标（v15 择脑解决）。
- 回放绝对路径问题（v15）。

**关键决策 / 红线状态**：**红线断言防回退机制**确立——后续每条修复都挂 wiring 断言。

---

### v15.0 — 铸基轮（tag `v15.0`）
**一句话**：选对模型 + 回放零成本，奠定后续基线。

**做了什么（增量）**
- 择脑：**standard brain = deepseek**，36/40 = **90.0%**（天花板对照 4/6）。
- ReplayProvider 落地：**31/31 = 100% 零 token**。
- 回放绝对路径修复：fixture 含原会话绝对路径 → 新增 `portable_paths()` 正则剥离。
- `service` 部署：`bench/_vm_restart_service.py` 修复 pkill 自匹配，用 `setsid nohup ... & disown` 可靠后台启动。
- 应力 22/24。

**没做哪些（范围外·已知债）**
- 成本 / 治理护栏（v16 CostGuard）。
- 经验回路（v17）。

**关键决策 / 红线状态**：**模型选择 = 能力相关**——奠定"弱模型才需经验"的后续结论基础。

---

### v16.0 — 封刀轮（tag `v16.0`）
**一句话**：成本护栏接真值 + 写边界白皮书，锻造收尾。

**做了什么（增量）**
- CostGuard 真值接地：`loop.rs:989` 从硬编码 `cost_ratio: 0.0` 改为 `self.nervous.cost_ratio()`（`nervous-system` 新增 `fn cost_ratio()`）。
- 清债 + 边界白皮书 + 锻造综述。
- 无功能性 bug 修复（v15 已达标，本轮只清债 + 写稿）。

**没做哪些（范围外·已知债）**
- 经验回路（v17）。
- embedding（v18）。

**关键决策 / 红线状态**：成本护栏接真实值，不再裸奔。

---

### v17.0 — 生长轮（tag `v17.0`）
**一句话**：首次验证"经验回路"有效，但仅限弱模型。

**做了什么（增量）**
- 经验回路闭合验证：zhipu glm-4.5-air，R1→R3 三轮 **65%→80%→85%**（+20pt）。
- R3 的 85% 超过 v14 历史最佳 77.5%（+7.5pt），同 provider/任务集/工具链仅凭经验回路达成。
- R3 vs R2 的 +5pt 证明经验**内容质量**有增量。

**没做哪些（范围外·已知债）**
- deepseek 验证（欠费，临时切 zhipu；`V17_PROVIDER=deepseek` 可重跑）。
- LLM 精炼（v18）。
- embedding 向量化（v18）。

**⚠️ 关键决策 / 红线状态**：**仅弱模型有效，强模型域未证**。这条后续被 v18 证明不可外推——经验对 deepseek（生产标准脑）零增量甚至有害。

---

### v18.0 — 精炼轮（tag `v18.0`）
**一句话**：把经验回路从"能跑"推到"跑得好"，却得到**否定性发现**。

**做了什么（增量）**
- embedding 向量召回：zhipu `embedding-3` 256 维 + cosine + keyword fallback（legacy 条目回填 + 迁移期安全网）。
- LLM 经验精炼 + 随机序（`--shuffle`）。
- service 新增 `EMBEDDING_ENABLED` 环境开关（默认开）。

**没做哪些（范围外·已知债）** —— 实为**否定结论**
- **embedding 无增量**（82.5% < 85.0%）→ v19 停止该线。
- **LLM 精炼无增量**（85.0% < 87.5%）→ v19 停止该线。
- 无顺序效应（E0 随机 87.5% ≈ v15 固定 90%）—— 推翻"v17 是顺序效应"的早期误判。
- 方法学偏离：计划用 zhipu 可比，实际 E0–E3 全用 deepseek。

**🔴 关键决策 / 红线状态**：**经验对强模型零增量/略负** → 奠定"经验 = 可选降级通道（连续失败≥3 才注入），非常驻增强"的方向。embedding / LLM 精炼 / 自主闭环整条线**停止**。

---

### v19.0 — 深水轮（tag `v19.0`）
**一句话**：用解剖刀找到 T13/T19 六轮全挂的真凶——不是能力墙，是 planner 判定缺陷。

**做了什么（增量）**
- 🔴 **T13/T19 root cause**：agent 只 grep+read，从未 write 就 self-report Done；`loop.rs` `all_done` 把 Read 节点 Completed 当任务完成。runner 对照同任务 → `FAIL(TEST_FAIL)`（代码真没改）。
- 经验自适应开关：`consecutive_errors >= 3` 才注入（wiring `experience-adaptive-switch`）。
- deepseek 固定序基线 **92.5%**（新高）。

**没做哪些（范围外·已知债）**
- `all_done` 修复本身（本版只诊断 + 加开关；修复留 v20）。
- T19 仍是唯一工具链盲区（v20 修复后重验）。

**关键决策 / 红线状态**：**找到悬案真凶 = 去根因而非调经验**，是六轮最高杠杆一刀。

---

### v20.0 — 收官轮（tag `v20.0`）
**一句话**：把 v19 的诊断落成修复 + 遗忘机制，系统闭环。

**做了什么（增量）**
- S1 planner `all_done` 修复：`loop.rs` 加 `write_attempted` 门控——all_done 但从未写操作时强制 replan（replan_count < 3 兜底）。wiring 新断言 `all-done-requires-write`。T13 0/8→2/3。
- 遗忘机制：`experience` 的 `prune()` / `upgrade_core()`（reference_count 门控），wiring `experience-prune-wired`。
- 季度基线封存（声称）。
- workspace 测试全绿；wiring **14/14**（11 基础 + 3 新增）。

**没做哪些（范围外·已知债）**
- 🔴 **季度基线封存产物实际未落地**：`quarterly-baseline-q3-2026.md` 不存在（v21 发现，维护规程 §7 挂空引用）。
- T13 从必挂变波动（修复"不写"，剩余"写错"是模型质量）。
- T19 仍是工具链盲区（NO_GENERIC_FN）。

**关键决策 / 红线状态**：六轮毒债根治 + 经验系统闭环；但"封存"成了幽灵承诺（待补）。

---

### v21.0 — 刀入鞘轮（tag `v21.0`，维护期起点）
**一句话**：擦净死代码 + 清密钥 + 写死红线 + 启动维护期。

**做了什么（增量）**
- **Removed（消债）**：删 embedding 死代码 `zhipu_embed_fn()`（44 行）+ `set_embed_fn` 接线 + `EMBEDDING_ENABLED` 开关（v18 证零增量）。
- **Added（沉淀 + 规程）**：
  - `docs/design-decision-planner-v20.md`——all_done 必须要求真实 Write。
  - `docs/design-decision-experience.md`——经验 = 降级通道，不得回退无条件注入。
  - `docs/maintenance-protocol.md`——季度体检 / 按需增量 / 红线 / 响应流程 / 新功能 checklist / 年度回顾。
  - `docs/maintenance-v21-report.md`——本轮报告。
- **Verified**：wiring 14/14 全绿；5 处硬编码 key 全改 `.env`（grep `5ad30`/`sk-28d737` = 0）。
- 维护期启动声明（v21.0 起）。

**没做哪些（范围外·已知债）**
- ❌ `/readyz` 假实现（`routes.rs`）不验真实就绪即返成功。
- 🔴 `drain_nervous_alerts()` 零调用者 → 中断日志被丢弃。
- 🟡 `Simplify` 分支不可达（nervous 侧可达，loop 侧空挡）。
- 🔴 季度基线文档幽灵引用（见 v20 遗留）。
- 🟡 对外 RFC（CN-001 / RFC-004 v3 / RFC-005 替代路径）**暂停**。
- 🟡 guest 多用户会话（B4）未施工（`guest.rs` 不存在）。

**关键决策 / 红线状态**：**漂移终止机制就位**——"季度只体检、不加功能、不跑实验"把无目标函数漂移钉死。系统已干净可维护。

---

### v22.0 — 验证 / 工程化 + 双引擎收口轮（✅ 已发布，tag `v22.0`，HEAD = `8e39775`）
**一句话**：用独立测试窗口验证 v21 达标 + 修复 v21 实测差异 + 搭出"多窗口角色协作"框架 + EPIC-B/C 旧债还清 + Y 方向收口 + Docker 验证。

**做了什么（增量）**
- **全链路测试 T1–T7 FINAL（测试窗口在 VM 实测，不改码）**：
  - T1 编译门：fmt PASS（修复 v21 fmt 漂移 commit `1a93ea2`）/ clippy 0 warning / **205 test 全过** / wiring **14/14**。（时点快照：v22 验证期；当前已演进为 **214 passed / wiring 15/15**，见 §四 基线事实）
  - T2 服务冒烟：`/healthz=200`，T00 PASS。
  - T3 接线自证：断 `record_tool_exchange` → 13/14 变红（防火墙活）/ 恢复 14/14。
  - T4 经验持久化：重启前 112 → 重启后 112（不丢）。
  - T5 provider：deepseek + zhipu 双通。
  - T6 基准快检：**18/20 = 90.0%**（≥85% 红线；T13/T14 持续 PASS；T15 波动、T19 能力墙）。
  - T7 应力快检：**no_panic 24/24**（应力红线达标）。
- **修复 v21 实测 9 处差异**：runbook 已内置修正（代码根 / pkill / 预编译二进制+软链 / T3 整体替换 / `BENCH_PROVIDER=zhipu` / `batch --runs 1` / 单文件计数 / T7 完整 env），本次执行无命令级差异。
- **修复 v22 问题**（commit `422adb9`）：ST8 harness 预算 + T19 双 Done 出口门控 + planner 委托语义。
- **建立"项目-窗口群协作框架 v2.1"**（`docs/project-windows-framework-design.md`）：多 AI 角色协作机制设计（需求/架构/开发/审查/测试/文档/部署窗口 + 任务流转规则 + 人类在产出物上审核 + 8 缺口补充）。**这正是当前"规划窗口 / 审计窗口"运作的方法论**。
- **诚实结论**：T19 双门控修复后仍 0/3 → 确认是**模型能力墙**（deepseek 对"泛型重构"不产生写动作），非代码缺陷，记入能力边界白皮书；T15 = 纯波动题，无需修。

**没做哪些（范围外·已知债）**
- T19 能力墙（NO_GENERIC_FN，v13 起盲区）**未解**，诚实记能力边界。
- T15 波动**未根治**（观察即可，非回归）。
- 🟡 guest 多用户会话（B4）未施工（**永久排除**，B/C 路线不排期）。
- 🟡 对外 RFC 继续暂停。
- ✅ ~~`/readyz` 假实现~~ **已真探活**（EPIC-B，2026-08-01，commit `05f90cf`：list_persisted_sessions Ok→200 / Err→503 + 测试）。
- ✅ ~~季度基线文档幽灵引用~~ **已决断**（Y1-3，2026-08-01：选 B 删引用，规程 §7 指向真实体检报告）。
- ⚠️ ~~**窗口群框架目前是设计文档 + 协作方法论**~~ **已工程化落地（2026-08-01）**：`window-framework/` 独立 Python 项目，v0.1→v1.0.3（skeleton→brain→orchestration→memory→autonomy→integration→structured output→dogfooding→多窗口），**177/177 测试全绿**，真实 LLM dogfooding 单窗口（analyze 100% / 全链路 done / 人类 2 次）**+ 多窗口**（3 窗口分工：info-architect→html-generator→qa-reviewer，check 4/4 + conflict 0）均验证通过。原"靠 WorkBuddy Agent/团队机制人工驱动"已由可运行编排器（project/window/workflow 引擎 + function calling）替代。

**关键决策 / 红线状态**：**v21 经独立测试窗口验证达标**（可信委托的证据闭环）；多角色协作框架搭好。**v22.0 已打 tag（2026-08-01）**——EPIC-B/C 两笔旧债还清 + Y 方向（P1 安全收口 + P2 codex-cli 六面 UX）+ Docker build 验证全部完成，双引擎进入"只做体检、不再进化"稳态。

---

### 窗口群框架 v1.0.2 → v1.0.3（2026-08-01，独立 Python 项目 `window-framework/`）
**一句话**：把「项目-窗口群协作框架 v2.1」从方法论落地为可运行编排器，并用真实 LLM dogfooding 证明可用（单窗口 + 多窗口）。

**做了什么**
- **v0.1→v0.7 七版演进**：project/window 容器 + agent 引擎（沙箱/budget/StepLog）+ workflow 引擎（stage/gate/trigger）+ 三层上下文压缩 + AI 需求分析（function calling 结构化输出）+ 导入/模板 + 冲突仲裁。
- **v0.9 UX 六面全闭**：上手/反馈/边界感/呈现（run --verbose）/容错（resume/retry/deploy --last）/polish（全局 status）。
- **v1.0.1 dogfooding**：真实 LLM（deepseek-v4-flash）自建文档站成功——analyze 首胜率 100%、全链路 done、人类介入 2 次；修复 12 个单测测不出的集成 bug（reasoning_content 回传 / 虚假 done 防护 / 探索死循环引导 / working 残留死锁 / tokens 爆炸 / stage gate 生成等）。
- **v1.0.2 维护优化**：provider 路由（deepseek/zhipu/agnes/openai）、VERSION 修正、v10 回归套件（dogfooding 修复保护）。
- **v1.0.3 多窗口 dogfooding**：真实 LLM 三窗口分工链（info-architect→html-generator→qa-reviewer）验证——analyze 产出 3 窗口 / 全 done / check 4/4 PASS / conflict 0 / 14 html 产出 / 人类 2 次；修复 3 个真 bug（gate approve 未验窗口状态 / human gate 断言误报 / provider key 缺失报错）。

**没做哪些（边界）**
- ~~多窗口并行流转尚未真实 LLM 全链路验证~~ **v1.0.3 已验证**（3 窗口分工 + 顺序流转；"并行"在引擎 = 多窗口分工 + 依赖链流转，非时间并行）。
- `provider=anthropic` 等未知 provider 回退 deepseek（可扩展）。

**测试状态**：**177/177 全绿**（v10 17 + v09 21 + v07 19 + v06 14 + v05 28 + v04 17 + v03 19 + v02 21 + framework 21）。

---

### v23.0 — 三端口架构轮（2026-08-02~08-04，tag `v23.0`，HEAD = `5ec25e9`，239 tests）
**一句话**：把三端口架构（Human OS 前端 + AI OS 内核 + Observer OS 第三权）从设计落成可运行代码——**11 个 WP 全闭环**，v23 是最后一次大规模施工。

**做了什么**
- **WP-0 通用交互原语**（阶段二）：`InteractionRequest{id,kind,blocking,timeout,on_timeout,payload}` + `InteractionResponse{id,by,resolved,payload,latency_ms}`——kind 开放字符串非 enum，**内核只认 id/blocking/timeout，加一种人类交互 = 0 行内核改动**；R1 id 强校验 + R2 一次性消费；`POST /interaction/{iid}` 通用路由 + `/approvals` 薄适配层（对外契约不变）。
- **WP-1 事件信封与 span 树**（阶段三）：`EnvelopedEvent{schema_version,ts,seq,span_id,parent_id}`（serde flatten 向后兼容）+ `SpanOpen/SpanClose`——span_id 服务端分配（事实产生权），深度 3 嵌套还原测试。
- **WP-2 SSE 出口与录制重放**：`sse_stream_with_replay`（Last-Event-ID 断线续传，G4 停在最后事实 seq）+ `GET /events` JSONL 录制 + G2 重放确定性。
- **WP-3 规划缺口推导器**：`Gap{from,why,blocking,auto_assumed}`（构造器强制契约）+ `derive_gaps` + clarification 走通用原语（内核不加一行分支）。
- **WP-4~7 Observer 四件套**（阶段四，新建 `crates/observer` 独立 crate——禁塞 nervous-system）：L1 只读抽头 + L2 fail-closed（seq 断档 → Err，协调者拒启）+ 四象限确定性指标（G3 两次字节级相同，禁 LLM）+ 规则引擎（`observer-rules.toml` 5 规则 + Finding L3 evidence 强制 + report.md/json 双重报告）+ G0 红线熔断（sandbox/预算/重试风暴 → CircuitBreak 事件，只拉闸可审计）。
- **WP-8 产物登记**（阶段五）：`Artifact` 事件（loop do_act 对 write_file/edit 成功 emit）+ `GET /artifact/open`（workspace 内路径校验，穿越全拒）。
- **WP-9 思考摘要**：`ThinkSummary` 确定性模板（零 LLM 不烧 token）+ `THINK_SUMMARY_STYLE` 参数化 + LLM 增强开关；信封自动带 span_id。
- **WP-10 前端接真流**：新增 **`GET /stream`** SSE 端点（EventSource 直达，复用 replay + Last-Event-ID）+ demo 真实模式面板（span 时间线 / 审批按钮 / artifact 卡片 / 断线重连，mock 保留 fallback）。

**没做哪些（边界）**
- RT3/P2-2 seccomp 白名单（用户挂起，大工程+安全敏感独立排期）。
- Q1~Q11 未决项（top-level-plan §8）。
- 季度体检基准 deepseek 20×2（烧 token，预算纪律待预算——Observer/Human OS 体检零成本已先行）。

**测试状态**：**239 passed**（v22 214 + v23 25 增量）/ fmt 0 / clippy 0 / release build ✓ / VM 端到端 ✓（/stream 200 / artifact 穿越全拒 400 / 正常 200）。

---

## 三、全局收口表（做了 / 没做）

### ✅ 已做（锻造九轮 + v22 收官，到 v22.0 全绿）
- 通过率 0%→90%（v12.7→v22 稳定在 90%±5% 带）。
- 不可绕过硬边界：G0 sandbox（landlock+seccomp）+ 15 条 wiring 红线全绿 + 接线自证防火墙。
- planner 真 bug 根治（T13/T19 "只读即完成"六轮毒债）。
- 经验系统降级通道化（自适应开关 + 遗忘机制），不再污染强模型。
- 零硬编码 key（全改 `.env`；Y1-1 Agnes key 清零 0 命中）；embedding 死代码删除。
- 维护期纪律就位（季度体检 + 红线 + 不跑实验）。
- v21 经独立测试窗口全链路验证达标（T1–T7 FINAL）。
- **EPIC-B `/readyz` 真实探活**（2026-08-01，Ok→200 / Err→503 + 两路径测试）。
- **EPIC-C 神经系统告警链**（2026-08-01：`drain_civ_alerts` 落文明线 + 删死包装 `drain_nervous_alerts`；`Simplify` 死 arm 移除）。
- **季度基线幽灵引用已决断**（2026-08-01：B 删引用，规程 §7 指向真实体检报告）。
- **codex-cli 六面 UX**（2026-08-01：setup / resume / 结构化错误 / need_approval 补强 + 8 测试保护，214 passed）。
- **Docker build 已验证**（2026-08-01：codex-rust:v22 构建成功，容器 healthz/readyz 200，landlock 正常，Dockerfile 三处缺陷修复）。
- **v22.0 tag 已打**（指向 HEAD，含全部 Y + Docker 改动）。

### 🟡 没做 / 仍开（Known Debt，维护期非紧急，不触红线）
- guest 多用户会话（`guest.rs` 不存在，B4，**永久排除**）。
- 对外 RFC（CN-001 / RFC-004 / RFC-005）暂停。
- T19 模型能力墙（NO_GENERIC_FN，诚实记边界，非代码缺陷）。
- 季度体检基准 deepseek 20×2（烧 token，测试预算纪律下待预算；应力 24/24 + 回放 31/31 已有 v22 真值）。
- ✅ **窗口群框架已工程化落地**（2026-08-01）：`window-framework/` v1.0.3 全链路可运行（177/177 测试，dogfooding 单/多窗口真实 LLM 验证通过），不再是设计/方法论。

### 🔴 战略已停（telos 过滤器判定，不再投入）
- embedding 向量召回 / LLM 精炼 / 自主凝练闭环 / G2 常驻注入（v18 证对强模型零收益）。

---

## 四、基线事实（每次更新时刷新）
- 写于：2026-08-04（v24 维护轮刷新）
- 主干 HEAD：`21429e1`（v23 封版验收）+ tag `v23.0`（`5ec25e9`，11 WP 全闭环）
- 已 tag 最末：**`v23.0`（`5ec25e9`）**——三端口架构（Human OS + AI OS + Observer OS）正式交付
- 进行中：无（v23.0 封版，三端口维护稳态）
- wiring：**15/15 全绿**（v22 口径，v23 未触碰接线）
- 最新基线：deepseek 固定序 90.0%（v22 T6 权威值，在 90%±5% 带内；v23 未重测——预算纪律）
- 应力红线：no_panic 24/24（v22 T7）
- 测试：**239 passed**（v23 封版全绿）+ window-framework 208/208
- Observer OS：G3 确定性 / 5 规则 / 熔断 / L2 全测试通过（mock 事件流，零成本）
- Human OS：demo 真实模式面板 + GET /stream 实测通过（VM 端到端）
- 窗口群框架：v1.0.3 208/208 全绿（v23 期间守门员回归确认）
- Docker build：✅ 已验证（v22，codex-rust:v22）——v23 新增 observer crate 未复验 docker，维护期待下次体检
