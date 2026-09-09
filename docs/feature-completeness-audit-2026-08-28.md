# Hearth（hearth-rs）功能全景与完善度审计

> **审计日期**：2026-08-28 ｜ **范围**：全 27 crate「功能是否完善」+「最初设想的变化」
> **方法**：源码直读（模块文档 + 源码锚点 + 依赖图 + 测试统计）；本地 cargo 不可用，运行时验证以 VM 门禁（v0.2.3 339 passed）为代理
> **基线**：commit `f0b4a54`（ux-polish-01），225 commits，27 crates / 35,124 行 Rust

---

## 0. 一句话结论

**代码本体整洁、核心能力闭环，功能完成度高于「会写代码」的基准线；但有 1 处「声明未接线」（bridge）、若干「已规划未做」（子代理 / 桌面版 / cgroup fail-closed / 白名单审批流），以及一批「迭代新增、超出最初设想」的能力（Observer 第三权、天赋基因、受控联网、PWC 协同）。** 你最初设想的「可信委托」骨架已全部落到代码里，形态从「后端服务平台」演化为「单二进制 CLI + 三端口」，是迭代而非偏离。

---

## 1. 你最初要什么（原始设想）

从顶层设计与你的 Telos 锚点还原：

- **本质目标**：一个「你敢长期委派的自主工程实体」——硬安全边界之内、过程透明、人硅边界清晰、可以「放心走开」。
- **最初形态**（codex-rust 时代）：REST/SSE 服务 + 多用户 + 自主编码 + 子代理（对标 Codex / Claude Code），代码规模目标 50–100 万行。
- **最初架构**：24 crate 后端平台（API / sandbox / LLM gateway / CLI / planner）。

---

## 2. 功能全景地图（27 crate）

| 域 | crate | 一句话功能 | 公开接口 | 测试 | 状态 |
|---|---|---|---|---|---|
| 内核 | agent-core | Agent 四阶段状态机（Plan/Act/Observe/Reflect）+ compaction | 48 | 28 | ✅ 核心闭环 |
| 内核运行时 | agent-runtime | 会话/循环驱动/LLM 调度/沙箱调用（从 service 抽出的内核） | 6 | 0 | ✅ |
| 类型 | agent-types | TaskGraph / Gap / Experience 等共享类型 |  ️30 | 9 | ✅ |
| 契约 | api | `AgentEvent` 事件 + `EnvelopedEvent` 信封（schema_version/seq/span） | 14 | 3 | ✅ |
| 桥接 | bridge | 多模型实时讨论/共识（BridgeSession） | 5 | — | ⚠️ 依赖已声明，源码零调用 |
| 代码索引 | code-index | tree-sitter 增量解析 + 符号图 + chunk 嵌入 | 9 | — | ✅（未深度验证） |
| CLI | codex-cli | `hearth` 二进制（chat/init/config/note/repl/serve） | 62 | 14 | ✅ |
| 判例 | experience | G2 判例卡（problem/solution/effectiveness） | 5 | 6 | ✅ |
| 网关 | llm-gateway | `LlmProvider` 统一 trait + FallbackChain 有界重试 | 23 | 14 | ✅ |
| 混元 | llm-cn | 腾讯混元 provider | 1 | 1 | ✅ |
| 本地 | llm-local | Ollama + vLLM 本地通道 | 2 | 3 | ✅ |
| OpenAI | llm-openai | OpenAI 兼容（deepseek/agnes/gemini）+ 错误分类 T2 | 1 | 8 | ✅ |
| 回放 | llm-replay | 确定性回放（G2 重放判据，31/31） | 3 | 2 | ✅ |
| LSP | lsp-bridge | rust-analyzer 诊断（无则优雅降级） | 5 | — | ✅ |
| 记忆 | memory | 会话持久化 + 6C 文明存储 | F6 | 0 | ✅ |
| 神经 | nervous-system | 交感神经（NerveAction 干预执行权） | 3 | 4 | ✅ |
| Observer | observer | **第三权（零执行权）** + circuit/metrics/rules | 17 | 14 | ✅ 独立 crate 落地 |
| 规划 | planner | 任务拆解 + 反省阈值 + `derive_gaps`（from/why） | 3 | 6 | ✅ |
| 协同 | project-sync | PWC EVENT_LOG/Task Ledger/registry（45 测试） | 35 | 45 | ✅ 新落地 |
| X光 | project-xray | CI 第四门：wiring 断裂扫描（red→exit1） | 14 | 19 | ✅ |
| 资源 | resource-monitor | 内存/磁盘/成本阈值 → 宪法第三条触发 | 4 | 2 | ✅ |
| 检索 | retriever | BM25 + 向量 RRF 融合 + rerank | 3 | — | ✅（未深度验证） |
| 沙箱 | sandbox | landlock + seccomp + cgroups（fail-closed） | 5 | 1→含 fail-closed 测试 | ✅ |
| 服务 | service | REST/SSE（/healthz /readyz /sessions /stream /approval） | 20 | 7 | ✅ |
| 潜意识 | subconscious | 后台低频处理 | 11 | 0 | ✅（未深度验证） |
| 工具运行时 | tool-runtime | 工具分发 + ApprovalState 四态 + session 键控 | 11 | 1 | ✅ |
| 内置工具 | tools-builtin | bash/read/edit/patch/glob/grep/introspect/web_fetch | 19 | 7 | ✅ |

---

## 3. 原始设想 ⇄ 当前实现：演变脉络

| 维度 | 最初设想 | 当前状态 | 性质 |
|---|---|---|---|
| 产品形态 | REST/SSE 后端服务 | **单二进制 CLI**（派 A 自托管，进程内直跑） | 🔄 架构范式变化 |
| 命名 | codex-rust（代号） | **Hearth**（2026-08-22 定名，Telos=可信委托） | 🔄 定位升级 |
| 子代理 / 多代理 | 计划中有（顶层设计 §3.4） | **明确不做**（v0.2 记分卡延后，Aider 无子代理仍顶尖） | ⚪ 范围收缩（非偏离） |
| 多人/多用户 | 规划中有 | **后置**（单人优先策略） | ⚪ 战略后置 |
| 安全隔离 | sandbox 规划 | **已落地**：landlock+seccomp+cgroups（fail-closed） | ✅ 达成 |
| Observer 第三权 | 无此概念 | **新增**：独立 crate，零执行权 + 熔断 + 人类反审 | 🔺 超出设想 |
| 天赋基因 G0–G3 | 无 | **新增**：宪法级元能力分层 | 🔺 超出设想 |
| 受控联网 | 无 | **新增**：web_fetch 白名单 + introspect 体感 | 🔺 超出设想 |
| 跨窗口协同 | 无 | **新增**：project-sync（PWC） | 🔺 超出设想 |
| 桌面版 | 前端 demo（v21j 冻结模拟数据） | demo 仍未接真流（B4-2 挂账） | ⏳ 未达 |

**一句话**：从「大而全的后端平台」收缩为「小而稳的单二进制 + 三端口」，砍掉了子代理/多人，补上了安全、透明、协同三层「超出设想」的能力——这是健康的迭代收敛，不是偏离原始意图（可信委托的核心一直未变）。

---

## 4. 重点功能完善度（spec↔impl↔边界↔测试）

### ✅ 完善度高的
- **G0 沙箱**：landlock FS 白名单 + seccomp deny-list + cgroups（`sandbox/lib.rs`），`force_seccomp_fail` 注入测试证明「隔离失败=启动失败」（`sandbox/lib.rs:673`）。缺口：cgroup **fail-closed** 仍 ERRNO 回退未做、KILL 化未做（路线图第二梯队）。
- **LLM 网关**：6 个 provider 统一 `LlmProvider`；T2 错误分类（429→Transient / 5xx→Fatal / 4xx→Param）已落地（`llm-openai/lib.rs:371-381`），死通道不再无限重试。
- **Observer 第三权**：独立 crate、零执行权铁律（`observer/lib.rs`），fail-closed 在 service 层落地——**真零执行权已核验**（grep 无反向依赖 nerve-action）。
- **Planner 缺口推导**：`derive_gaps` 构造器强制 `from+why`、`auto_assumed`（planner:15-17），B2 最高优先项已落地。
- **web_fetch**：T10 真接线（`run_local.rs:174/:273` 两处 `ctx.env` 注入 + service/main.rs:406 + repl.rs:250）。
- **REPL / CLI**：多行粘贴、Ctrl-C 取消恢复、`/file`/`/help`、零配置首跑清晰报错。

### ⚠️ 已知缺口 / 未做
- **`bridge`（多模型共识）**：已在 `agent-runtime`/`service` 的 `Cargo.toml` 声明依赖，但 `BridgeSession`/`use bridge` 在**生产代码零调用**——典型的「声明未接线」。若意在「内部桥」愿景，当前是占位；若不打算做，应清理依赖以免静默膨胀。（待测：确认是否 intended 未激活）
- **T11 白名单审批流**：deny 无 `InteractionRequest` 出口，用户须手工改 config（D 类控制流，挂账）。
- **WS12 长文自动分块**：流式+prompt 级拆分存在，但「harness 自动分块累积」专用路径未经真机长文验证。
- **cgroup fail-closed / seccomp KILL 化 / readonly find**：安全纵深残余。

### 🔺 超出最初设想（已实现）
Observer 第三权、天赋基因、受控联网、PWC 协同、REPL 体感工具——这些在最初的「codex-class 编码 Agent」设想里没有，是迭代中长出来的差异化能力。

---

## 5. 给你的「记忆恢复」速查

- **你现在拥有什么**：一个能直接 `hearth chat "目标"` 跑起来的本地 AI 编码 Agent；硬安全沙箱、多 LLM、规划缺口推导、Observer 透明观察、跨窗口协同、受控联网——并且后端架构（AI OS / Human OS / Observer OS 三端口）是结构性区别于 Codex/Claude 的差异化资产。
- **哪些「以为有、其实还没」**：子代理（明确不做）、桌面版接真流（demo 仍是模拟数据）、cgroup 真正 fail-closed、白名单审批流。
- **哪块「偷偷长大了」**：bridge 依赖已引入但没接——值得你拍板「是要做内部桥，还是删依赖」。
- **下一步**：封版前建议先决「bridge 去留」与「T11 审批流」，再做六维全项审计。

---

*本报告为「功能完善度」审计，非封版全项审计；运行时行为（沙箱真拦截、回放一致）以 VM 门禁为准，已在前文如实标注。*
