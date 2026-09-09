# 全局透视盘点 — v11.5 时点（2026-07-30）

> 判据延续 v10.1 盘点确立的铁律：**只认"生产路径可达"（router/CLI/loop.rs/main.rs → 函数的 grep 可证调用链）**，不采信提交信息与交付报告自报。
> 本次盘点基于**工作区当前状态**（含未提交的 v11.4/v11.5 改动：`subconscious` crate + loop.rs 信号化改造）。
> 方法：3 路并行源码探查（执行核心+脑区 crate / 服务层+九债务复检 / 支撑层+workspace 健康），全部 file:line 证据。

---

## 一、规模快照（实测，非估算）

| 指标 | v10.1（07-29） | v11.5（07-30 现在） | 变化 |
|---|---|---|---|
| workspace 成员 | 20 | **23**（+experience/+subconscious/+nervous-system） | +3 |
| .rs 文件 / 总行数 | 50 / 18,761 | **56 / 20,440** | +1,679 |
| 测试属性声明 | 162 | **181**（#[test]=67 + #[tokio::test]=114） | +19 |
| loop.rs | 2,290 行 | **2,474 行** | +184（继续长胖） |
| workspace.package.version | 0.1.0 | **仍 0.1.0** | 未动 |
| git tag | 无 | **仍无**（HEAD=6e7b40a v11.3，v11.4/v11.5 未提交） | 未动 |
| 根目录 *.md | ~50 | **62** | 审计产物继续堆积 |
| crate 行数 Top5 | — | service 4,404 / agent-core 3,334 / llm-local 1,415 / llm-gateway 1,044 / sandbox 995 | — |

⚠️ 版本状态：**v11.4/v11.5 的核心改动（subconscious + loop.rs 信号化）尚未提交**（`git status`: M loop.rs / M Cargo.toml / ?? crates/subconscious/），最后提交停在 v11.3。

---

## 二、v10.1 九项债务最终复核 — "9/9 清零"宣称不成立

v10.5 审计宣称九项债务全部清零。逐项 grep 复核当前工作区，**真清零 4 项、半接线 1 项、仍断裂 4 项（其中 1 项是修好后又回归）**：

| # | 债务 | 判定 | 证据 |
|---|---|---|---|
| 1 | telemetry 计数器 | 🟢计数器真接 / 🔴crate 仍孤儿 | fetch_add@routes.rs:114，GET /telemetry 返回真实计数@routes.rs:492/520；但 `crates/telemetry` 无任何 crate 依赖它——实际用的是 service 本地 `TelemetryCollector`（main.rs:378），独立 crate 是死代码 |
| 2 | constitution 注入 | 🔴 **回归！** | v10.3 曾修好（constitution_prompt 真注入），**v11.4 改回硬编码摘要串**（loop.rs:597-601）→ `constitution_prompt()`（constitution.rs:7）重新退化为**零调用**，constitution.md 运行时**不读**（全仓无 include_str/read_to_string） |
| 3 | retriever | 🟢真接线（🟡浅扫描） | main.rs:241-277 真 build + 扫 *.rs + 生产 search@loop.rs:1093；但**仅扫顶层目录非递归**，crates/ 下源码扫不到 |
| 4 | webhook | 🟢真接线 | fire_event 生产调用@routes.rs:119（session_created） |
| 5 | civ 自动触发 | 🔴仍无 | civ_store.append 仅 API 手动路径（routes.rs:136/per_user.rs:322）；loop.rs 无任何 civ 自动写入 |
| 6 | orchestrator | 🟢可达（窄触发） | do_act→execute_plan@loop.rs:927，条件=task_graph 非空 && pending_tool_calls>1@loop.rs:911 |
| 7 | workline 60s 调度 | 🟢真接线 | main.rs:363 tokio::spawn + sleep(60s) |
| 8 | 模型自动发现 | 🟡服务侧半接 / 🔴CLI 缺失 | providers.json 写读存在（main.rs:194-224）但发现结果**只 log 不 registry.register**——发现的模型进不了生产；CLI `model discover` 子命令**不存在** |
| 9 | 工具生态 | 🟢注册链路 / 🔴执行断裂 | search/install/auto-discover 路由真接（routes.rs:527-567 → registry.rs:55/75/89）；但 `InstalledTool.command` **全仓无读取执行路径**，dispatcher 只认内置 `Arc<dyn Tool>`（dispatcher.rs:84-106）→ **安装的工具装完不能用** |

**规律确认（第 4 次）**：并行执行窗口的"清零宣称"仍系统性高于源码事实；且出现了首例**修好又改坏的回归**（#2 constitution），说明缺回归防线（无接线断言测试）。

---

## 三、三个新"脑区" crate 的真实成色

### nervous-system（v10.2，脑↔体桥） — 🟢 主链真接，2 个 API 壳
- `query()` 每轮 Reflect 真实调用（loop.rs:1242），资源临界时 `Abandon|DeliverAndQuit|Simplify → ReflectVerdict::GiveUp` 覆盖真实可达（loop.rs:1246-1250）。**这是三脑区中接线最扎实的。**
- 🔴 壳：`update_cost`（loop.rs:375）零调用 → `set_cost` 永不被调，**cost 恒 0.0，成本分支永不触发**；`drain_nervous_alerts→drain_civ_alerts`（loop.rs:385-386）零生产调用 → **civ 告警只积累永不消费**。
- 测试 4 个，真断言。

### experience（v11.0-v11.3，经验自进化） — 🟢 闭环真实，🟡 降级运行
- **生产注入链真实**：main.rs:295 构造 ExperienceStore → session.rs:219 set 进 AgentLoop（不是只有测试在 set）。
- **Phase 5 自主闭环真实存在**：do_plan 真 search(loop.rs:782)+reinforce(:786)+注入 build_messages(:604/792)；`run()` 结束**无条件 append 经验**（成功/失败都写，loop.rs:1453-1476，fire-and-forget spawn）。搜→用→强化→记，四步全通。
- 🟡 三处降级：①全仓无 `set_embed_fn` 调用 → **embedding 恒 None，生产只跑 keyword 匹配**，cosine_similarity(lib.rs:259) 是真函数但生产用不到；②store 是内存 `RwLock<Vec>`，**无持久化——重启即失忆**；③宣称的"五层记忆"实为**单层 Vec**，无分层架构。
- 测试 11 个，真断言。

### subconscious（v11.4 未提交，潜意识信号门） — 🟡 接了，但近乎假信号
- `check()` 在 do_plan 真实调用（loop.rs:754-761），`Abandon → LoopPhase::Error` 真可达（:763）。骨架是对的。
- 但生产中**这道门实际永远不会 override**，四处断点：
  1. `GuardContext.last_success` **硬编码 true**（loop.rs:758，注释说 "updated by caller" 但无人更新）→ 假信号；
  2. `last_action: None` 传入 → ConstitutionGuard 永不命中；
  3. 生产 gate 只挂了 ConstitutionGuard，**CostGuard/RepetitionDetector 从未 add_guard**，repetition 从不 record；
  4. `DeliverAndQuit` 被 `_ => {}`（loop.rs:773）**静默吞掉**。
- 同时 v11.4 的 prompt 瘦身（宪法 ~500t→~50t 摘要）已生效，但代价是引入债务 #2 的回归。
- 测试 7 个，真断言。

---

## 四、三档能力全景（v11.5 时点）

### 🟢 真接线（生产路径 grep 可证）
5 相位循环（run@1331 / do_plan@662 / do_act@855 / do_observe@978 / do_reflect@1162）｜子代理 spawn+FileChange 合并｜审批门｜sandbox（landlock+seccomp，VM 验证过）｜4 个 LLM Provider 全真（OpenAI / Ollama+Vllm / **腾讯混元**(llm-cn) / FallbackChain 网关）｜service 21 条 /api/v1 路由 + SSE + replay + OpenAPI + 离线 Swagger｜3 个后台任务（cleanup / workline-60s / observer-每小时）｜PerUserStore 按 user_id 懒建分区｜codex-cli 21 个叶命令｜telemetry 计数器（service 本地版）｜webhook fire｜retriever build+search｜orchestrator（窄触发）｜nervous-system 资源临界 GiveUp｜experience 搜-用-强化-记闭环｜bridge 4 策略｜六 store。

### 🟡 半接线 / 降级运行
subconscious 门（接了但信号全假，永不 override）｜experience（无 embedding、无持久化、单层）｜retriever 仅顶层浅扫描｜模型发现（只 log 不 register）｜工具生态（可装不可用）｜readyz 仍假探针（routes.rs:254-258）。

### 🔴 宣称/期望 vs 源码事实
constitution_prompt 回归为零调用（constitution.md 运行时不读）｜civ 自动触发始终不存在（v6.1 起连续 5 版未兑现）｜telemetry 独立 crate 孤儿｜CLI model discover 不存在｜InstalledTool 无执行路径｜nervous update_cost/drain_alerts 双壳｜AppState.workline_store 死字段｜**SubAgentExecutor seam（trunk-freeze W1）始终未抽**——executor.rs 不存在，tokio::spawn 仍硬编码@loop.rs:508。

### 代码卫生（好消息）
生产路径 **0 处 unimplemented!/todo!**（仅存的 3 处全在测试 Mock/字符串内）｜0 处 `clippy::all` 全量豁免（v7.0.3.5 的豁免已清）｜panic! 仅 2 处测试断言｜unwrap 密度 service 0.30% / agent-core 0.81%｜无孤儿 crate（telemetry 除外）。

---

## 五、顶层判断

**一句话：躯干持续变强，新脑区"形先于神"——结构都搭对了，但神经末梢（信号真值、持久化、执行路径）普遍没接满。**

1. **主干依然健康且从未被破坏**。5 相位循环、sandbox、LLM 层、service 面全部真实，卫生指标反而在改善（unimplemented 清零、clippy 豁免清零）。冻结主干的资格仍然成立。
2. **新增的三脑区呈梯度**：nervous-system（最实）→ experience（闭环真实但降级）→ subconscious（骨架真、信号假）。共同模式是**"最后一厘米"缺失**：experience 差 set_embed_fn 一行 + 持久化；subconscious 差 last_success/last_action 两个真值回填；工具生态差 dispatcher 认 InstalledTool 一段。
3. **首例回归出现**（constitution v10.3 修好 → v11.4 改坏），暴露最大制度缺口：**没有"接线断言测试"**——债务清零后无测试锁住调用链，下一版重构就能无声退化。
4. **版本治理是最落后的一环**：0.1.0 / 无 tag / 无 CHANGELOG / v11.4-11.5 未提交 / 根目录 62 个 md。工程在跑，账本没记。

## 六、行动序建议（优先级从高到低）

1. **先提交再谈别的**：v11.4/v11.5 工作区改动提交 + 补打 tag（至少 v11.3、v11.5 两个锚点）。未提交状态下一切审计都在流沙上。
2. **接线断言测试（防回归，最高杠杆）**：为九债务中已清零的 4 项 + 三脑区主链各写 1 个"调用链存在性"测试（如：build_messages 输出必含宪法摘要标记；run() 结束 experience 计数+1；nervous query 在 Reflect 被调）。约 8-10 个测试锁死现状。
3. **最后一厘米三件套**（每件都是小改）：①subconscious 回填 last_success/last_action 真值 + 处理 DeliverAndQuit；②experience 挂 set_embed_fn（哪怕先用本地 hash embed）+ 文件持久化；③dispatcher 支持执行 InstalledTool.command。
4. **决断两笔烂账**：constitution_prompt 与 telemetry crate——要么真接、要么删除，不许再挂着；civ 自动触发连续 5 版未兑现，建议正式砍出 backlog 或降级为"手动为设计意图"。
5. **W1 seam（SubAgentExecutor）**：trunk-freeze-branch-plan 的 W1 仍然有效且未执行；loop.rs 已 2,474 行，越晚抽越贵。
6. **文档扫除**：根目录 62 个 md 归档到 docs/audits/，建 CHANGELOG。

---
*盘点方法：3 路并行 Explore 代理，全部结论 file:line 锚定；数字为实测统计。本文档自身不构成"通过"或"不通过"判定，供顶层设计决策使用。*
