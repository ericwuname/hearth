# Hearth（hearth-rs）项目全面审查报告

> **报告日期**：2026-08-27
> **审查范围**：截至本日仓库全部代码、文档、测试与版本历史
> **审查基线**：commit `f0b4a54`（分支 `ux-polish-01`，2026-08-27 02:03）
> **审查方法**：源码直读核验（git log/tag + grep 锚点 + 逐文件抽查），不采信二手报告
> **目标读者**：外部评审机构 / 潜在开发者 / 合作方

---

## 1. 执行摘要

**Hearth**（仓库/CLI 名 `hearth-rs`，旧代号 `codex-rust` 为 deprecated 别名）是一个用 Rust 从零构建的 **Codex-class 自主编码 Agent**——对标 OpenAI Codex CLI / Claude Code，定位为「一个你敢长期委派的自主工程实体」。

一句话定位（用户原话锚定）：

> 硬安全边界之内，过程透明、人硅边界清晰；像炉边一样，是 AI 可以安稳驻留、你也可以放心走开的地方。

**关键数字（2026-08-27 实测）**：

| 指标 | 数值 | 来源 |
|---|---|---|
| Workspace crate 数 | **27** | `Cargo.toml` members + `ls crates/` |
| Rust 源文件 / 总行数 | **91 文件 / 35,124 行** | `find crates -name '*.rs'` |
| 测试函数总数 | **351**（`#[test]` 195 + `#[tokio::test]` 156） | grep 统计 |
| 最近 VM 门禁 | **339 passed 全绿**（fmt/clippy/test） | v0.2.3 发布记录（2026-08-26） |
| Git 提交总数 | **225 commits** | `git rev-list --count` |
| 版本 tag | v12.7→v23.0（旧线）+ demo-v21j-frozen + window-sync-final-v1 | `git tag` |
| 产品版本 | **Hearth v0.2.3**（2026-08-27） | `Cargo.toml` / install.sh |
| 文档资产 | **docs/ 185 篇 .md**（共 189 文件） | `ls docs/` |
| 基准测试资产 | bench/ 295 文件（63 json / 54 jsonl / 40 py / 31 log） | `ls bench/` |

**当前状态一句话**：CLI 产品 Hearth v0.2.3 已可安装运行（Linux x86_64 tarball + install.sh），核心 Agent 循环、G0 安全沙箱、Observer 第三权、PWC 项目协同层均已落地并通过 VM 门禁；主要缺口在「后端真智能」的深化（clarify/approval 闭环完整链路）、桌面版接真流、以及若干已挂账缺陷（详见 §7）。

---

## 2. 项目定位与核心理念

### 2.1 命名与愿景

- **定名**：2026-08-22 用户拍板，**Hearth** = 赫兹（频率共振）+ 炉边（家的安稳）。详见 `docs/hearth-naming.md`。
- **终极目标（Telos）**：可信委托（trusted delegation）——用户能"放心走开"，AI 在硬边界内自主完成工程任务。
- **冰山理论**（用户原话，2026-08-22）：CLI/桌面前端只是冰山一角，后端真智能（规划缺口推导、审批闭环、产物登记、Observer 三权互进）才是水下 9/10。当前主线已向后端倾斜。

### 2.2 三端口架构

| 端口 | 职责 | 载体 | 状态 |
|---|---|---|---|
| **Human OS**（北向） | 人类交互界面 | `docs/codex-desktop-demo.html`（v21j 定版冻结，git tag `demo-v21j-frozen`，39 条回归断言） | 演示版冻结；接真流待做 |
| **AI OS**（南向，唯一事实源） | Agent 内核 | `agent-core` / `agent-runtime` 等 27 crates | **主体已落地**（§4） |
| **Observer OS**（第三权，零执行权） | 独立观察、Finding 报告、G0 熔断 | `crates/observer`（独立 crate，4 文件 1075 行，14 测试） | v23 落地，雏形运行 |

**事实产生权公理**（全项目宪法级约束）：后端产生事实，前端只投影事实。判定式：数据是否会被他人引用（进报告/被审计/AI 读/grep）→ 后端独占；只影响当前屏幕 → 前端独有。后端禁止返回颜色/HTML/坐标等呈现语义。

### 2.3 基因层级（G0–G3）

规范文档：`docs/gene-expression-system-design.md`

| 层级 | 性质 | 机制 | 牙齿 |
|---|---|---|---|
| **G0** | 物理不可违反 | sandbox（landlock+seccomp fail-closed）、审批门、进程隔离 | 拓扑级强制 |
| **G1** | 代码分支必执行 | 相位钩子 + wiring 断言（15/15） | 编译期/CI 强制 |
| **G2** | 时机召回 | experience 判例卡（KPI 淘汰制） | 召回偏置 |
| **G3** | 无牙齿只偏置 | `constitution.md`（6 条，3488 字节，运行时读取，≤6000 字符上限） | 软约束 |

铁律：单向上升不许空降；Rust struct 最多 G1 非 G0；G3 换血制（≤10 条）。

六条宪法（`constitution.md` 实测内容）：① 真实是第一价值 ② 忠实于原始意图 ③ 资源需要守护 ④ 知道自己不知道 ⑤ 局外人视角 ⑥ （持续存在/改进闭环）。

---

## 3. 版本演进史（两条版本线）

### 3.1 第一线：codex-rust 引擎锻造期（v12.7 → v23.0）

13 个 tag（v12.7、v13–v23.0），2026-08-04 v23.0 收官（`5ec25e9`）。这一阶段锻造了引擎本体：

- **5-gate forge 体系**：benchmark / stress / wiring / replay / 审计 多轮锻造验证
- 每版配套 CHANGELOG（v12.7–v21 于根目录）+ gatekeeper 审计报告 + forge 报告
- v15 铸基：ReplayProvider 31/31；v22：Docker 构建验证（landlock 正常）
- v23 五个 phase：事件流信封（EnvelopedEvent）+ Observer crate + 规划缺口推导 + 产物登记 + 思考摘要 + 前端接真流

### 3.2 第二线：Hearth 产品线（v0.1 → v0.2.3，2026-08-22 起）

2026-08-22 定名 Hearth 并发布 v0.1 可装基线，转为以 CLI 产品形态迭代：

| 版本 | 日期 | 要点 |
|---|---|---|
| v0.1 / v0.1.1 | 08-22 | 可安装单二进制（tarball+install.sh）、`hearth init/config` 三层配置、landlock+seccomp 徽章、RT3 seccomp 99 syscall 白名单 fail-closed |
| v0.1.3 / v0.1.4 / v0.1.6 | 08-22–23 | harness 加固（对标 Codex CLI v0.149.0 fork 回归修复）、CLI UX 打磨 |
| v0.2.0 | 08-23 | 功能级对标 Top-3（80–90 分目标）：WS7 判断力 / WS8 体感 / WS9 预算价值化 / WS10 受控联网 / 盲区 C 产物校验 |
| v0.2.1 | 08-25 | 盲点补全：成本护栏 / REPL 自动恢复 / read_lints / 任务状态持久化 / HTML 骨架校验 |
| v0.2.2 | 08-26 | 天赋基因落地（P0+P1）+ REPL 多行粘贴 / Ctrl-C 取消 / `/file`；PWC project-sync crate（45 测试）并入，VM 门禁 **339 passed** |
| **v0.2.3** | **08-27** | **手工测试 T1–T10 修复**（HEAD `f0b4a54`）：provider 选择 / 错误分类 Fatal 化 / 上下文度量纠偏 / 沙箱 /dev/null / seccomp node/python3 / **出网白名单结构性修复**（详见 §7.2） |

> 上游纠偏结论（v0.2 任务书锚定）：hearth 是 Codex CLI v0.149.0（Apache-2.0）的 fork；同模型（deepseek v4 flash）在 Codex 上游表现远好于 hearth ⇒ 差距在 harness 设计而非模型。v0.2 系列即系统性收敛此差距。

---

## 4. 系统架构

### 4.1 Workspace 全景（27 crates，按职责分组）

**内核层（AI OS 主体）**

| crate | 行数 | 测试 | 职责 |
|---|---|---|---|
| agent-types | 663 | 9 | 共享类型（TaskGraph、Gap、Experience 等） |
| agent-core | 5,882 | 28 | **Agent 循环本体**：`LoopPhase{Init,Plan,Act,Observe,Reflect,Done,Error}` 四阶段白盒状态机（`loop.rs:29`），do_plan/do_act/do_observe/do_reflect 相位钩子；compaction（`loop.rs:950` maybe_compact，WS4） |
| agent-runtime | 1,347 | — | 会话运行时（会话恢复、历史重建 `repl.rs:426-449`） |
| planner | 1,127 | 6 | **规划缺口推导器**（B2）：`derive_gaps()`——每条 gap 构造器强制 from+why；blocking=false ⇒ auto_assumed=true（`planner/lib.rs:15-17`） |
| api | 318 | 3 | 事件契约：`AgentEvent`（9 变体含 clarification payload）+ **`EnvelopedEvent` 信封**（schema_version/ts/seq/span_id/parent_id，`api/lib.rs:103`——G4 断线续传锚点） |
| service | 4,632 | 7 | REST/SSE 服务、Observer fail-closed 协调层 |
| codex-cli | 3,709 | 14 | **Hearth CLI**（二进制名 `hearth`，别名 `codex`）：chat/REPL/config/init/note |
| bridge | 284 | — | 前端桥接 |

**LLM 网关层（多 provider，统一 trait）**

| crate | 行数 | 测试 | 职责 |
|---|---|---|---|
| llm-gateway | 1,189 | 14 | **`LlmProvider` trait**（`provider.rs:11`：name/model/capabilities/chat/stream_chat）+ FallbackChain 有界重试 |
| llm-openai | 839 | 8 | OpenAI 兼容通道（deepseek/agnes/gemini 兼容端点），STREAM-1 截断修复 |
| llm-local | 1,417 | 3 | Ollama + vLLM 本地通道 |
| llm-cn | 793 | 1 | 腾讯混元通道 |
| llm-replay | 621 | 2 | **回放通道**（G2 重放判据基础设施，31/31 用例） |

**工具与沙箱层**

| crate | 行数 | 测试 | 职责 |
|---|---|---|---|
| tool-runtime | 835 | 1 | 工具分发器（`ApprovalState` 四态、session 键控审批） |
| tools-builtin | 2,336 | 7 | 内置工具：bash/write_file/read_file/web_fetch（deny-by-default 出网白名单 `web.rs:93-105`）/introspect/load_lints 等 |
| sandbox | 1,811 | 1 | **G0 硬边界**：Linux landlock（FS 白名单）+ seccomp（syscall deny-list）+ cgroups；**fail-closed**（`force_seccomp_fail` 测试钩子验证隔离失败即终止 child，`sandbox/lib.rs:57-62`） |
| resource-monitor | 105 | 2 | 资源监控（内存/磁盘/成本阈值 → 宪法第三条触发） |

**认知与记忆层**

| crate | 行数 | 测试 | 职责 |
|---|---|---|---|
| memory | 599 | — | 长期记忆（I/O 错误传播已修——v22 勘误闭环） |
| experience | 502 | 6 | G2 判例卡（id/category/problem/solution/success/effectiveness/reference_count） |
| subconscious | 342 | — | 潜意识层（后台低频处理） |
| nervous-system | 247 | 4 | 交感神经（NerveAction 干预执行权：Simplify/ReduceSteps/Abandon/DeliverAndQuit，调用点 `loop.rs:1589`） |
| code-index / retriever / lsp-bridge | 1,474 | 2 | 代码索引 / 检索 / LSP 桥 |

**观察与协同层**

| crate | 行数 | 测试 | 职责 |
|---|---|---|---|
| **observer** | 1,075 | 14 | **第三权**：零执行权（只消费 `Vec<EnvelopedEvent>` 产出 Finding/报告/熔断事件）；独立 crate 铁律（禁塞 nervous-system，`observer/lib.rs:5-11`）；含 circuit（熔断）/metrics（四象限）/rules（规则引擎）三模块 + 人类反审记录（`hearth note --observer-verdict`） |
| project-xray | 758 | 19 | 项目透视 |
| **project-sync** | 2,219 | 45 | **PWC 项目协同**（v0.2.2 新增）：EVENT_LOG（JSONL+O_APPEND+fs2 锁+prev_hash 哈希链）/UUIDv7 事件/Task Ledger（乐观锁+终态不可回流+权限矩阵）/registry（心跳+水位线）/受限 fs API（拒 symlink 逃逸）/IMPORT default-deny+trust_level 四类 |
| llm-replay（复用） | — | — | 确定性回放 |

### 4.2 核心控制流

```
用户 goal
  ↓
[Plan]  planner.derive_gaps() → gap 必带 from/why/blocking
        ↓ 空响应静默降级单步（planner 瘦身，v0.2 设计铁律）
[Act]   工具调用（tool-runtime 分发 → sandbox 内执行 → 审批门 NeedApproval）
        ↓ web_fetch 走 HEARTH_EGRESS_ALLOWLIST deny-by-default
[Observe] 结果回喂 + read_lints 结构化编译错误
        ↓ compaction：history ≥32k 触发旧轮折叠（ctx_mgr.maybe_compact）
[Reflect] 反思 verdict（纯问答无工具轮跳过——v0.2.3 T5 降本）
        ↓
[Done]  产物登记 + verify（写盘后校验：存在性→字节数→可运行，WS13）
```

全程事件以 `EnvelopedEvent`（schema_version+ts+seq+span_id+parent_id）流向：service SSE 出口（G4 断线续传）+ observer 消费（G0 红线熔断）+ 录制重放（G2 判据）。

### 4.3 与上游 Codex CLI 的关系

- fork 自 Codex CLI v0.149.0（2026-08-19 开源，Apache-2.0）
- **差异化核心**：四阶段白盒状态机（vs Codex/Claude 黑盒循环）——Plan 是显式阶段，planner 瘦身而非删除（v0.2 设计铁律：外部顾问+守门员共识）
- 重实现策略：`apply_patch`（上游内嵌 Lark 文法紧耦合）→ 自实现 SEARCH/REPLACE（Aider 已证明无需 Lark）；thread 续接复用自有 transcript.rs+session_id，不整搬上游重型 crate

---

## 5. 安全模型（G0）

| 层 | 机制 | 状态 |
|---|---|---|
| 文件系统 | landlock 白名单（readonly + writable 目录集） | ✅ 运行 |
| 系统调用 | seccomp deny-list（v23 RT3：99→121 syscall 白名单） | ✅ 运行（ERRNO 回退；KILL 化在路线图） |
| 隔离失败 | **fail-closed**：seccomp/landlock 加载失败 → 终止 child（有测试钩子 `force_seccump_fail` 验证） | ✅ |
| 资源 | cgroups（max_memory 等） | ⚠️ cgroup fail-closed 未做（挂账） |
| 出网 | web_fetch deny-by-default + HEARTH_EGRESS_ALLOWLIST（config.toml `egress_allowlist` + 进程 env，env 优先合并） | ✅ v0.2.3 T10 修复注入链；⚠️ agent 提议+审批流（T11）未做 |
| 人工介入 | 审批门（ApprovalState 四态，session 键控）+ WP-0 `InteractionRequest{blocking,timeout,on_timeout}` 通用原语（内核只认 blocking/timeout/id，禁 match kind） | ✅ 原语落地；clarify/approval 完整闭环在路线图 |
| 隔离透明 | 每次启动打印隔离徽章（linux 真隔离 / noop 仅开发），不静默降级 | ✅ |

---

## 6. 质量保障体系

### 6.1 测试与门禁

- **VM 门禁**（Ubuntu 24.04 真机，非本地模拟）：`cargo fmt --check` + `clippy` + 全仓 `cargo test`。最新全绿：**339 passed**（v0.2.2 发布时）；v0.2.3 T1–T10 修复后 VM 单测全绿（commit message 载明）。
- 测试含**故意失败负面用例**（如 project-sync 哈希链锚点跟踪、sandbox fail-closed、wiring 断言「断言不能失败=断言不存在」原则）。
- **重放确定性**：llm-replay 录制/回放（G2 判据），31/31 用例。
- Docker 构建已验证（v22：healthz/readyz 200 + landlock 正常）。

### 6.2 审查方法论（项目自有的守门员体系）

- **「不信报告信源码」**：所有验收以源码接线为准（生产路径真接线、非空壳、测试有断言），执行窗口报告仅作线索。
- **四层递进审查**：①存在性（grep）→ ②行为（真做?）→ ②.5 **可达性**（用户够得着?——CSS 裁剪容器血泪教训）→ ③表达力。
- **审计工具自身必须被审计**：v22 红队探针 5 项 bug 的教训已沉淀为流程（正式探针前先跑 5 行诊断确认 HEALTH_OK）。
- **诚实披露文化**：失败与根因显式记载（如 v22 /readyz 假阴性勘误：曾记"已真修"有误，双根因 memory 吞 I/O 错 + session 无 store 假健康，后实证修复闭环）。

### 6.3 多角色协同（PWC）

- 分工铁律：顶层（全局透视/评审/验收/任务书，唯一）→ 规划（拆排期）→ 施工（默认，照任务书写码跑门禁交证据）。
- v0.2.2 落地 **project-sync** crate（45 测试）将此流程工具化：EVENT_LOG 哈希链防篡改、Task Ledger 终态不可回流、registry 心跳水位线、IMPORT default-deny。

---

## 7. 当前能力与已知限制（诚实披露）

### 7.1 已具备的能力（可验证）

1. **可安装 CLI**：`hearth chat "目标"` 单命令直跑；三层配置（CLI 参数 > env > config.toml > 内置默认）；零配置首跑清晰报错引导。
2. **多 provider**：OpenAI 兼容（deepseek/agnes/gemini）+ Ollama/vLLM 本地 + 混元 + replay；`--provider/--model` 三级覆盖（v0.2.3 T1）；死通道判 Fatal 不再无限重试（T2）。
3. **G0 沙箱**：landlock+seccomp fail-closed；v0.2.3 修 /dev/null 误杀（T6）+ seccomp 放行 node/python3（T7，121 项白名单）。
4. **受控联网**：web_fetch 白名单机制结构性修复（T10：config 加 `egress_allowlist` 字段 + run_local/repl/service 三处 ToolContext 注入 + 正例测试）。
5. **会话韧性**（WS11）：REPL 每轮自动落盘 + panic 隔离 + 重启自动恢复（rebuild_agent+restore_history+restore_task_graph 真接线，源码核验过）。
6. **确定性验证**（WS13）：写盘后三重校验（存在性→字节数→可运行），agent 自报 Done 须过 verify；core_circuit_wiring 测试（故意 fail 设计）。
7. **成本可见**：tokens 显示 + introspect 油箱表（v0.2.3 T3 纠偏：纳入 system_text，暴露 history_chars/system_chars/total_chars）。
8. **REPL 体验**：多行粘贴（`{...}`）、Ctrl-C 取消后恢复、`/file` 读文件、`/help`。
9. **Observer 第三权雏形**：事件流消费→Finding/四象限指标/熔断，人类可反审（`hearth note`）。
10. **事件契约冻结**：EnvelopedEvent 信封 + schema_version + 只增不改演进规则 + FE 未知宽容（禁白屏）。

### 7.2 已知限制与挂账（截至 2026-08-27）

| # | 事项 | 严重度 | 状态 |
|---|---|---|---|
| 1 | **T11 出网白名单"agent 提议+用户审批"流**：deny 时无 InteractionRequest 出口，用户须手工改 config | P1 | 任务书已备（`hearth-manualtest-v023-taskbook.md`），涉 D 类控制流待顶层出任务书 |
| 2 | **T4 系统提示词瘦身**未做：每请求背负固定 system_text，agentic loop 放大成本（Agnes 测试 14 元异常高的根因之一） | P1 | 决策=数据驱动（system_chars 已可见，攒数据再瘦） |
| 3 | **桌面版（Human OS）未接真流**：demo v21j 冻结但仍是模拟数据；B4-2 挂账（Tauri/CORS 代理评估） | P1 | 路线图第三梯队 |
| 4 | **B2 后端真智能深化**：clarify 规划期批量+收齐 resume 完整链路、产物登记+open_artifact 未全做 | P0/P1 | 路线图第一梯队 |
| 5 | **安全纵深残余**：cgroup fail-closed 未做；seccomp ERRNO→KILL 化未做；readonly find 缺口未定位 | P1 | 路线图第二梯队 |
| 6 | **WS12 长内容生成**：流式+prompt 级拆分存在，但"harness 自动分块累积"专用路径未经真机 V2-14 长文验证 | P1 | 待 VM 真机测试 |
| 7 | **多智能体/子代理**：本版明确不做（Aider 无子代理仍顶尖的论据） | — | 延后 |
| 8 | **平台覆盖**：预编译产物仅 Linux x86_64；Windows/macOS 引导源码安装（sandbox 降级为 noop 并显式徽章） | P2 | 已知 |
| 9 | **"地基稳"观测期**：7 天零 panic 门槛（v0.2.2 08-26 起算）未满；量化指标（断网恢复≥95%）是声明非独立复测 | P1 | 观测中 |
| 10 | **工作区未提交物**：3 modified + 58 untracked（多为 08 月审计/验收文档），HEAD 即 v0.2.3 修复 commit | 低 | 待整理提交 |

### 7.3 历史缺陷的修复闭环示例（证明体系自愈能力）

- **/readyz 假阴性**（v22 勘误）：曾误记"已真修"→ 复查发现双根因（memory 吞 I/O 错 + session 无 store 假健康）→ 修复（`?` 传播 + 不再 Ok(vec![])）→ 薄地基审计源码实证闭环（2026-08-26）。
- **v0.2.3 手工测试十连修**：真人驱动测试（Agnes 通道）暴露 10 缺陷 → 逐一定位源码根因（如 T10 白名单 `ctx.env` 从未注入的结构性缺陷 + 测试假绿只证"空→拒"不证"配了能放行"）→ 一 commit 修复 + VM 单测全绿。

---

## 8. 工程资产与文档索引

### 8.1 根目录

- `README.md`：Hearth CLI 安装/配置/安全/使用（面向使用者，与 `hearth-cli-design.md` 口径一致）
- `CHANGELOG-v12.7.md` … `CHANGELOG-v21.md`：引擎锻造期逐版变更
- `constitution.md`：六条宪法（运行时读取）
- `Dockerfile` / `docker-compose.yml`：容器化（v22 验证）
- `install.sh` 于 `bench/`：安装脚本（不留占位地址，未指定源报错引导）
- 历史审计/锻造报告：`gatekeeper-audit-*.md`、`forge-report-*.md`、`final-audit-*.md` 等

### 8.2 docs/（185 篇）按主题导航

| 主题 | 代表文档 |
|---|---|
| **顶层设计** | `top-level-design.md`、`top-level-plan-v23.md`、`top-level-telos-anchor.md`、`requirements-ledger.md`（需求总账，BE-01~15） |
| **命名与定位** | `hearth-naming.md` |
| **CLI 产品** | `hearth-cli-design.md`、`hearth-cli-guide.md`、`README.md` |
| **对标与路线** | `hearth-harness-benchmark-top3.md`、`hearth-roadmap-next.md`、`hearth-v02-taskbook.md` |
| **架构规范** | `gene-expression-system-design.md`（G0–G3）、`frontend-backend-split.md`、`ai-os-event-contract-v1.md` |
| **基因/天赋** | `hearth-meta-capability-genes-final.md`（终版 v2 权威件）、`hearth-talent-different-view.md` |
| **协同协议** | `project-window-sync-design.md`（FINAL v1）、`dev-test-coop-protocol.md`、`rfc-authoring-conventions.md` |
| **验收记录** | `acceptance-*.md` 系列（v23 五 phase / hearth v0.1 release / rt3 seccomp / rt4 hardening 等 30+ 篇） |
| **审计追踪** | `audit-findings-v22.md`、`audit-fix-tracker-v22.md`、`audit-full-v24-post.md` |
| **UI 冻结基线** | `codex-desktop-demo.html` + `.regress.js`（39 断言）+ `human-os-design.md` |

### 8.3 bench/（295 文件）

基准任务集（tasks/）、结果（results/、63 json）、回放（replay/）、红队脚本（`seccomp-redteam.sh`、`syscall_probe/`）、安装脚本（`install.sh`）。

---

## 9. 路线图（截至 2026-08-27 的顶层排序）

**第一梯队：后端真智能**（离 Telos 靶心最近）
- B2 深化：clarify 规划期批量 + 收齐 resume 完整链路；禁输出无 from/why 的废话问题
- B3：approval 决策点按模式自动/等人

**第二梯队：安全纵深收尾**
- cgroup fail-closed；seccomp KILL 化；readonly find 缺口定位

**第三梯队：CLI → 桌面版**
- B4-2：demo 接真实 SSE（同源/CORS 代理 + Tauri 评估）——总验收判据：**demo 零改动接上真流即跑通**

**第四梯队：收口动作**
- 稳定 release tag；目录名/历史从 codex-rust 平滑标注；季度体检常态化（Observer 入体检）
- v0.2.3 遗留任务书：T11（白名单审批流）、T4（提示词瘦身数据驱动）、WS12 真机长文验证

**长期愿景（v10.0 方向）**：「内部桥」对等多模型消息总线协作（区别于父子 sub-agent）；天赋基因 v3 候选提案 P-A~P-J 待评估。

---

## 10. 结论

1. **引擎成熟度高于产品成熟度**：27 crate / 35k 行 / 351 测试函数 / VM 门禁全绿的内核与质量体系是项目最坚实的资产；CLI 产品 v0.2.3 已可用但仍在快速迭代期（十连修刚落）。
2. **差异化真实存在且可辩护**：四阶段白盒状态机 + Observer 第三权（零执行权独立 crate）+ G0–G3 基因层级 + 事实产生权公理，构成与 Codex/Claude 上游的结构性差异，非营销话术。
3. **质量文化是护城河**：不信报告信源码、审计工具自审计、故意失败测试、诚实披露勘误——这套守门员体系在 v22 /readyz 勘误与 v0.2.3 十连修中两次自愈验证。
4. **主要风险**：后端真智能（B2/B3）未完成前，"值得委派"未证；桌面版未接真流前，三端口架构仍是 2/3 落地；单人项目 bus factor = 1。
5. **对外部开发者的建议入口**：`README.md`（装/用）→ `docs/hearth-cli-design.md`（设计）→ `docs/top-level-design.md` + `requirements-ledger.md`（全局）→ `docs/gene-expression-system-design.md`（理念）。

---

*本报告全部数据于 2026-08-27 由源码直读实测产生（git log/tag、grep 锚点、wc 统计），未采信任何二手执行报告。审查人：顶层守门员（AI 架构师角色）。*
