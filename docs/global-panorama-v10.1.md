# 全局透视盘点报告（v10.1 时点）

> 日期：2026-07-29 ｜ 方法：5 路并行源码级探查（执行核心 / 服务层 / 新能力 crate / LLM+支撑层 / workspace 健康），全部 `file:line` 证据，不信提交信源码
> 上一次全局盘点：2026-07-28 `docs/trunk-qualification-audit.md`（当时 17 crate）

---

## 一、执行摘要

**全局判定：功能面真实且庞大，但进入"宣称超前于接线"的冲刺债务期（SPRINT-DEBT-ACCUMULATING）。**

- 规模：**20 crate / 18,761 LOC / 159 个测试**，CI 四门（fmt / clippy -D warnings / test / eval-mock）离线安全。
- 一天内从 v3.0 冻结冲到 v10.1（git 历史 20+ 提交），核心链路（会话→SSE→循环→工具→沙箱→审批→历史回放→CLI）**全部真实可用**。
- **但**：多个 git 提交宣称"完成"的功能，源码里**并未接入生产路径**（详见 §四）。这是本次盘点最重要的发现——提交信息通胀已经出现，如果不纠正，后续规划会建立在虚假地基上。
- 主干 `loop.rs` 膨胀到 **2,290 行、约 11-12 个关注点**，昨日封板计划里的 `SubAgentExecutor` seam **始终没抽**——冻结纪律"勉强维持"，god-file 临界。

---

## 二、演进时间线与版本态

```
v1.2 安全基线 → v2 六功能 → v3.0 契约冻结(B1-B3/S1-S2) → v3.1 post-freeze(P1-P3)
→ v4.x 生产化(健康探针/限流/Docker/配置文档) → v5.0 CLI + E1-E4
→ v6.0 四线(Bridge/文明/工作线/Provider v2) → v6.1 运行时集成(宣称)
→ v7.0 可观测(宣称部分) → v8.0 多用户 → v9.0 编排引擎 → v10.0 自我认知层 → v10.1 P1+P2(宣称)
```

**版本态断裂**：
- `Cargo.toml` workspace version = **0.1.0**（从未随版本演进）
- `git tag` = **空**（v3.0 冻结、v10.0 定版都没打 tag）
- 无 CHANGELOG / VERSION 文件
- 版本真相只存在于提交信息和 50 个根目录 md 文件里

---

## 三、能力地图（三档分类，全部源码核实）

### 🟢 真实且已接线（生产路径可用）

| 能力 | 证据 |
|---|---|
| 5 相位主循环（Init→Plan→Act→Observe→Reflect） | loop.rs: LoopPhase@23-31, run()@1173, step()@1152, do_*@612/760/834/1018 |
| 子代理（进程内 tokio::spawn + 深度护栏 + FileChange 回传） | spawn_sub_agent@438-507, MAX_DEPTH=2@74, RunReport.files_changed@165-173 |
| P1 内容级合并 + 冲突检测 | extract_files_from_tool_calls@177-256, merge_file_changes@260-285, planner "Merge conflicts:"@224-254 |
| P2 注入标记 + 4096 截断 | format_injected_content：LSP@928 / retrieval@967 / sub-agent@987,1035 / planner@239 |
| 语义审批门（destructive bash 检测） | tool_call_needs_approval@76-101, bash_cmd_is_destructive@104-156, do_act@769 |
| sandbox（Linux landlock+seccomp+cgroups v2；每个 Bash 都过） | sandbox/lib.rs:84-93,240-689；bash.rs:15 |
| 三家 LLM 客户端（超时+流式） | openai 10s/300s@211-214；local 10s/600s@263-265；cn 10s/300s@278-279 |
| ProviderRegistry v2 核心（provider:model 键+别名+三级 get） | registry.rs:57-78, main.rs:88-89 |
| service 核心 8 路由 + SSE + B1 历史回放 + OpenAPI + 离线 SwaggerUI | routes.rs:100-193, sse.rs:56, session.rs:28/133, main.rs:48/393 |
| Bridge 多模型社群（RoundRobin/Debate/MajorityVote/SingleSpeaker） | bridge/lib.rs:43-58 → session.rs:97 → POST /api/v1/bridge@380 |
| codex-cli 16 个子命令（全真实 HTTP+SSE） | codex-cli/main.rs:27-104, stream_chat@141-195 |
| 存储层（Jsonl 会话/文明/工作线/用户/模板/PerUser 复用器） | memory/lib.rs:34/446/517；service user.rs:6 / per_user.rs:9 / templates.rs:8 |
| observer 每小时资源快照 + instance_id | main.rs:312-325, main.rs:267 |
| CODEX_DISABLE_TOOLS（E3）、LSP_ENABLED 闸门、FALLBACK_CHAIN 闸门 | main.rs:163-184 / 215-223 / 135-156 |

### 🟡 存在但半接线 / 默认空转

| 能力 | 真实状态 | 证据 |
|---|---|---|
| retriever（E1 宣称"默认开"） | set_retriever 确实默认执行，**但 build()/index_tree 生产路径从未调用 → 索引永远为空，search 恒返回空** | main.rs:198-212 有注入、session.rs 无 build |
| PerUserStore 多用户隔离 | 只覆盖 civ/workline 端点；**agent 会话仍走全局单体 state.sessions** | routes.rs:104/135 vs 255-371 |
| lsp-bridge RustAnalyzerBridge | 真实 JSON-RPC 子进程+握手+超时，但 **OPT-IN（LSP_ENABLED 默认关→Noop）** | lsp-bridge/lib.rs:121-314, main.rs:221-222 |
| webhook（v8.0.4） | register 可用，**WebhookManager::fire 全仓零调用 → 永不触发** | webhook.rs:33，grep 无命中 |
| readyz | 注册了但 `let _ = &state.sessions;` 什么都不验 | routes.rs:220 |
| /api/v1/tools | 硬编码数组 | routes.rs:493 |
| AppState.civ_store / workline_store | 声明了但 handler 全走 per_user → **两个死字段** | routes.rs:26-45 |

### 🔴 提交宣称"完成"但源码未接线（最关键发现）

| 宣称 | 源码真相 | 证据 |
|---|---|---|
| v6.1 L2 "Civ自动触发 civ_tx+maybe_civ" | **loop.rs 无 civ_tx/maybe_civ，全仓源码为零**（只在审计文档里出现）；civilization 后台 consumer 未 spawn | loop.rs 全文 grep 零命中 |
| v6.1 L3 "WorkLine 60s 调度器 tokio::spawn" | main.rs 唯一 spawn 是 observer 每小时任务，**无 60s 调度** | main.rs:313-325 |
| v7.0 "TelemetryCollector 接入 /api/v1/telemetry" | 计数器 AtomicU64 存在，但 **session_count/error_count/steps_total 全仓无一处 fetch_add → 端点恒返回 0**；telemetry crate 本体只剩 eval harness，且是孤儿 crate（service/cli 都不依赖） | routes.rs:434-451；telemetry/lib.rs:8 |
| v7.0 3.1 "loop.rs tracing spans" | 只有散点 `tracing::info!`，**无 span 埋点** | loop.rs:218,669,997,1110,1248 |
| v9.0/v10.1 P1 "TaskOrchestrator + execute_plan 完成" | orchestrator.rs 代码真实存在，但 **execute/execute_plan 仅被自身测试调用；无路由、无 CLI 命令、未接 AgentLoop → 生产不可达** | orchestrator.rs:63/340/248-395 |
| v10.0 10C "基因宪法 constitution_prompt" | 定义并导出了，**但 build_messages@547-609 未引用 → 从未注入 system prompt** | constitution.rs, lib.rs:10 |
| v10.1 P2 "资源告警 critical alerts" | compute_roi()/is_critical() 仅自测调用；observer 只写快照，**无阈值告警逻辑** | resource-monitor/lib.rs:65/78 |
| v6A "模型自动发现 + providers.yaml + models.json 缓存" | **全部未实现**（/api/v1/models 只回内存注册表） | 全仓无 yaml/cache 代码 |
| v10.1 "工具搜索/安装生态" | **代码中不存在** | grep 零命中 |

---

## 四、主干健康诊断

- **5 相位骨架完好**，重特性大多有 Option/Noop/闸门守卫，热路径没被污染——这是好的一面。
- **坏的一面**：`loop.rs` 2,290 行，堆了约 11-12 个关注点（审批门/子代理/FileChange 合并/LSP/检索/CostMeter/多轮/注入标记/深度护栏/事件/日志）。
- **昨日封板计划（trunk-freeze-branch-plan.md）的 W1 从未执行**：`executor.rs` 不存在，`spawn_sub_agent` 仍硬编码 `tokio::spawn`@470，`RunReport` 走了另一条路（加了 `files_changed`，没有 `output_text`）。执行窗口实际走的是 v3.0 契约冻结路线，A4 用 FileChange 结构化回传替代了自由文本回传——**目标等价可接受，但 seam 债仍在**：下次想加多进程执行还是得改 loop.rs。
- planner / sandbox 稳定未膨胀。

## 五、债务清单 TOP 10（按优先级）

1. **提交信息通胀**（🔴 流程级）：§三-🔴 共 9 项"宣称完成实未接线"。必须建立规矩：**每个提交宣称的功能，守门员抽查接线后才能算完成**。否则规划地基是假的。
2. **telemetry 双重虚假**：孤儿 crate + 恒零计数器端点。二选一：真接线（session.rs 埋 fetch_add）或正式退役标注 eval-only。
3. **retriever 空转**：默认注入但索引永空。要么生产路径补 `build()`（session 创建时对 workspace_dir 索引），要么退回 OPT-IN 并如实标注。
4. **版本态断裂**：Cargo.toml 0.1.0 / 无 tag / 无 CHANGELOG。补打 `v10.1` tag + 建 CHANGELOG.md + 同步 workspace version。
5. **loop.rs god-file 临界**：2,290 行。下一轮任何 loop 增强前，先抽 seam（SubAgentExecutor 或至少把审批门/FileChange 合并拆成模块）。
6. **文档漂移**：top-level-design.md / architecture-summary.md 还写着"17 crate""无 CI"，且两文档对 retriever 接线一事**自相矛盾**（一个说已闭环@407、一个说从未调用@133）。活文档必须刷新或降级为历史存档。
7. **根目录 50 个 md**：约 40 个一次性审计/handoff/计划产物。建 `docs/archive/` 归档（不删），根目录只留活文档。
8. **memory crate `#![allow(clippy::all)]`**：整 crate 关掉 lint 是债务旗，放开并清理。
9. **PerUserStore 半吊子**：会话主路径仍全局单体。要么把 sessions 也纳入 per-user，要么在文档里明确"多用户仅覆盖 civ/workline"。
10. **残桩**：unimplemented! ×2（code-index/lib.rs:355、retriever/lib.rs:318）、13 处 `#[allow(dead_code)]`、AppState 两个死字段、bridge/codex-cli 零测试。

## 六、行动建议（按序）

| 序 | 动作 | 性质 | 量级 |
|---|---|---|---|
| 1 | **对账清单落地**：把 §三-🔴 九项逐条标记"接线 or 撤销宣称"，更新 governance.md | 流程 | 0.5 天 |
| 2 | **版本定锚**：打 tag v10.1 + CHANGELOG + Cargo version 同步 | 卫生 | 0.5 天 |
| 3 | **真接线三件套**：telemetry 计数器埋点 / retriever build / constitution 注入 build_messages（三个都是小改动，接完 🔴 少一半） | 代码 | 1-2 天 |
| 4 | **civ 触发 + workline 调度 + webhook fire + orchestrator 暴露**：v6.1/v9 欠账补齐或正式砍掉 | 代码 | 2-3 天 |
| 5 | **文档大扫除**：活文档刷新 + 归档一次性产物 | 卫生 | 0.5 天 |
| 6 | **loop.rs 瘦身**（seam 抽取，含 SubAgentExecutor 旧债） | 重构 | 2-3 天 |
| 7 | 此后再谈新功能（模型发现/工具生态/多进程） | 功能 | — |

**一句话**：地基是真的、很能打；但楼层数被报高了。先对账、再补线、后盖楼。

---

## 附：五路探查代理结论存档

- 执行核心：结构连贯但 loop.rs 达 god-file 临界（2290 行/11+关注点），冻结纪律勉强维持
- 服务层：核心 agent 路径真实连贯；v8-v10.1 平台面半截接线，"功能标记已打、后端未落地"
- 新能力：codex-cli/bridge/stores/observer 真接线；telemetry/orchestrator/constitution 孤立
- LLM+支撑：三客户端+registry v2 核心真实；模型发现/yaml/cache 未实现；retriever 空转
- 健康度：SPRINT-DEBT-ACCUMULATING（版本断裂+文档漂移+孤儿 crate+clippy 抑制）
