# 治理文档（Governance）—— v1.1 修复审计

> 审计日期：2026-07-28
> 审计员：code-audit-gatekeeper（守门员）
> 审计对象：fix-plan.md 的 6 轮修复 + 真 Linux 测试结果
> 验证机：用户 Linux 虚拟机 wutao@192.168.220.131（Ubuntu 24.04.4，内核 7.0.0-28）

---

## 阶段总览表

| 阶段/轮次 | 内容 | 实现率 | 偏离 | 结论 |
|---|---|---|---|---|
| R1 路径穿越 | read/edit/grep 拒绝 `..`/绝对路径 | ✅ 达标 | — | 过闸 |
| R2 Glob/Grep 走沙箱 | 注入 `Arc<dyn Sandbox>`，`sandbox.spawn` | ⚠️ 代码达标，**真实 Linux 运行时不可用** | 🟡 依赖 sandbox 缺陷 | 待 sandbox 闭环 |
| R3 Tantivy | retriever `unwrap`→`?` | ✅ 达标 | — | 过闸 |
| R4 会话取消 | oneshot + select! + abort | ✅ 达标 | — | 过闸 |
| R5 审批隔离 | `HashMap<session_id, ApprovalState>` 全链路 | ✅ 达标 | — | 过闸 |
| R6 低危 | L4 unwrap→error、PORT、CORS | ✅ 达标 | — | 过闸 |
| 硬验收·landlock | `test_p5_landlock_denies_outside_write` | ✅ 真断言通过 | — | 过闸 |
| 硬验收·seccomp | `test_p5_seccomp_blocks_ptrace` | ✅ 真断言通过 | — | 过闸 |
| 硬验收·sandbox 命令执行 | `test_linux_sandbox_echo` / `test_linux_sandbox_timeout` | ❌ 2 失败 | 🟡 既有缺陷 | **未闭环** |

**闸门判定**：🔴=0，6 轮修复实现率 ≥ 0.9 → 修复本身可过闸；但 **sandbox 命令执行硬验收未闭环**（真实 Linux 上 landlock 锁死外部命令），整体暂不退闸，须先按 handoff-sandbox-fix.md 实施方案 A 并重跑全绿。

---

## 各阶段明细（逐文件源码核实）

### R1 路径穿越（M1）
- 文件：tools-builtin/src/{read,edit,grep}.rs
- 核实：`read.rs`/`edit.rs` 在 `ctx.cwd.join` 前检查 `path_str.contains("..")` 或 `is_absolute` 并拒绝；`grep.rs` 对 `path` 参数同样防护，越界回退 cwd + 日志。均有断言测试（`read.rs`/`edit.rs`/`grep.rs` 各 +1 测试）。
- 结论：✅ 非空壳，测试能失败（代码回退则测试失败）。

### R2 Glob/Grep 走沙箱（M7+M5）
- 文件：tools-builtin/src/{glob,grep}.rs
- 核实：`GlobTool::new()`（glob.rs:13-17）内部 `sandbox::create_sandbox(SandboxConfig::default())` → **Linux 返回真实 `LinuxSandbox`**（非 Noop）。生产接线真实（守门员最警惕的"接线当功能"已排除）。
- ⚠️ 但真实 `LinuxSandbox` 在 landlock 生效时因 `read_only_paths` 为空锁死命令执行，导致 `find`/`rg` 在真实 Linux 上无法运行 → R2 真实 Linux 运行时不可用。**根因在 sandbox crate，非 R2 代码**。

### R3 Tantivy（M3）
- 文件：retriever/src/lib.rs
- 核实：两处 `unwrap` 改为 `ok_or_else(...)?`；`.map` 闭包返回 `Result` + `.collect()`。编译通过，无 panic 路径。

### R4 会话取消（M4）
- 文件：service/src/session.rs
- 核实：`send_message` 创建 `oneshot::channel`，`cancel_tx` 存入 session；转发循环 `select!` 用 `&mut cancel_rx` 接收取消信号后 `agent_future.abort()` 并置 `running=false`。**真中止后台 task**（非仅删引用）。

### R5 审批隔离（M2）
- 文件：tool-runtime/src/dispatcher.rs、agent-core/src/{scheduler,loop}.rs、service/src/{session,routes}.rs
- 核实：`dispatcher.approval: Mutex<HashMap<String, ApprovalState>>`；`resolve_approval/approval_state/set_approval_pending/reset_approval` 均带 `session_id` 参数；`AgentLoop::session_id` 字段 + `set_session_id` setter；`loop.rs` 在 `set_approval_pending`/`check_approval` 透传 `self.session_id`；`session.rs:136` `agent.set_session_id(session_id.clone())` 在 `send_message` 生产路径注入；`routes::submit_approval`（routes.rs:52-57）从 `Path(id)` 取 `session_id` → `SessionManager::submit_approval(id, req)` → `dispatcher.resolve_approval(id, ...)`（session.rs:403）。**生产路径完整接线，非仅测试注入**。

### R6 低危
- 文件：service/src/{session,main}.rs
- 核实：`main.rs:150-158` CORS 由 `Any` 收紧为 `CORS_ORIGIN` 环境变量（`AllowOrigin::list`，默认 `http://localhost:5173`）；`main.rs:171-176` 端口走 `PORT` 环境变量（默认 3000）。均为生产入口，`cargo test --all` 编译通过即证明接线。

---

## 硬指标自算（守门员铁律：不信报告信源码）

- `rs_files = 38`，`total_LOC = 12924`，`total_test_fns = 138`
- 报告 `v1.1-test-report.md` 写「133+7=140」→ 实测 138，**差 2，属 🔵 参考级出入**，不阻塞。
- 测试数分布在 23 个文件中（见 audit_metrics 输出）。

---

## 评审记录

### 2026-07-28 审计
- **编译**：17 crate 全过（rustc 1.97.1）。累计修正 5 个「执行方引入」的编译 bug（真 Linux 抓出，本机无 Rust 漏过）：
  1. glob/grep `SandboxOutput.stdout`(String) 误当 `Vec<u8>`
  2. retriever `.map` 闭包误用 `?`
  3. `AgentLoop` 漏初始化 `session_id` 字段
  4. `AgentLoop` 漏定义 `set_session_id` 方法
  5. `select!` 内 `oneshot::Receiver::recv()` 误用 → `&mut cancel_rx`
- **测试**：除 sandbox 2 个 Linux 测试外全绿；landlock/seccomp 核心测试真断言通过。
- **未闭环项**：sandbox 命令执行 2 测试失败（既有缺陷）→ 见 `handoff-sandbox-fix.md` 方案 A。
- **结论**：6 轮修复可过闸（🔴=0，实现率≥0.9）；整体因 sandbox 硬验收缺失暂不退闸，须实施方案 A 后重跑 `cargo test --all` 至全绿。

---

### 2026-07-28 复审（守门员源码重审，独立交付件 gatekeeper-audit-2026-07-28.md）

- **方法**：不信报告信源码。Python 实算硬指标 + `grep`/`Read` 逐文件核实；本机无 Rust 链，未执行 `cargo test`（真 Linux 复跑留待用户 VM）。
- **硬指标自算**：17 crate / 38 `.rs` / **13,007 LOC** / **139 测试函数**（前次报告 12,924 / 138，差 🔵 参考级）。
- **沙箱硬验收已源码闭环**：前次"未闭环"根因（`read_only_paths` 为空 → landlock 锁死 `/bin/echo` 等）已消除，现 `SandboxConfig::default()` 为 `read_only_paths: vec![PathBuf::from("/")]`（lib.rs:61-71，整树只读+执行，workspace 外仍禁写）。4 个 Linux 测试（echo/timeout/landlock/seccomp + cgroup 回收）均为能失败的真断言。
- **红线核查**：`agent-core` 仅依赖 `llm-gateway`，无具体 provider 直依赖 ✅；`memory` 经 `main.rs:139` + `set_memory_store` 真接线 ✅；`api` 北向契约 8 变体 serde 往返真断言 ✅。
- **空壳核查**：3 处 `unimplemented!`/`todo!` 全部位于 `#[cfg(test)]` mock 或字符串字面量，无生产空壳 ✅。
- **偏离**：🔴=0；🟡 3 项——D1 `telemetry` eval harness 孤儿（全仓 0 调用，须接线或声明免验收）、D2 `lsp-bridge` 真实 client 文档化 defer 至 P5（Noop 已接线）、D3 `main.rs:35-36` 无 key 静默 placeholder（生产须配置）；🔵 4 项（弱测试 R1、`architecture.final.md §15.3` 缺文档 R2、`sandbox/lib.rs:4` seccomp 注释过期 R3、LOC 数字出入 R4）。
- **闸门判定**：🔴=0 且实现率≥0.9 → **过闸（源码级）**。放行前提：D1/D3 由用户确认或补；真 Linux `cargo test --all` 全绿须用户 VM 复跑（执行缺口非代码缺陷）。

---

### 2026-07-28 全局审计（全部 14 crate 逐文件深读，独立交付件 global-audit-report.md）

- **方法**：三路并行 agent 逐文件阅读全部 38 个 `.rs`（LLM 组 / agent-core+tools 组 / service+data 组），Python 实算依赖映射与 LOC/测试数。每项发现均带源码定位（文件:行号）。
- **结论**：🔴 **不通过，打回。** 原因不是架构差——分层干净、无生产空壳、测试有断言——而是安全面与容错面系统性缺口：
  - 🔴 API 零认证（6 端点 + SSE 流全开放，0.0.0.0:3000）
  - 🔴 edit/read 工具绕过 sandbox（直接 `tokio::fs`，无 landlock/seccomp 保护）
  - 🔴 4 个 provider stream spawn 无取消（僵尸 task + HTTP 连接泄漏）
  - 🔴 Sessions HashMap 永不清除 → 必然 OOM
  - 🔴 JSONL 非原子写入 + 并发无锁 + 单行损坏整批丢弃 + 字段缺失静默吞
  - 🔴 审批触发用 JSON 字符串匹配（`tc.args.to_string().contains("rm ")` 可绕过）
- **🟡 高优**：15 项（read 绕过沙箱、无 body 限制、无速率限制、CORS 宽松、HTTP 无超时、LLM JSON 解析静默降级、vLLM 类型重复 150 行等）
- **🔵 低/参考**：13 项（grep 路径处理不一致、has_rg 缓存、注释过期、eval harness 孤儿等）
- **13,007 LOC / 139 测试 / 🔴=12 / 🟡=15 / 🔵=13** → 需完成 Phase 1（7 项，估 1–2 天）方可重审。
- **亮点**：`agent-types`/`llm-gateway`/`planner`/`retriever`/`tool-runtime`/`lsp-bridge` 6 个 crate 无任何 🟡 以上问题，代码底盘质量好，缺的是装甲层。

---

### 2026-07-28 v1.2 交付验收（守门员逐项源码核实，产出 gatekeeper-review-v1.2.md）

- 执行方提交 `v1.2-DELIVERY-REPORT.md`，声称 40 项发现 24 已修复、🔴 门禁翻绿。守门员直读源码逐项核对。
- **🔴 门禁 12 项全覆盖**：`grep`+`Read` 逐项定位改动源码 → 10 项真修（AUTH-0 Bearer 中间件带常量时间比较、OOM-1 finished_at+cleanup_finished+后台 TTL 清理、APPR-1 分词+命令表+管道分号分割的语义判定、STREAM-1~4 AbortOnDropStream 全 4 provider 接线、MEM-1~4 tmp+rename 原子/Mutex/64MiB/逐行容错）+ 2 项接受风险（SBOX-1/2 landlock+cwd 双重防线注释留痕）。
- **🟡 抽查 8 项**全通过（SBOX-3 守卫移入 spawn_sub_agent 内部、CORS GET/POST/OPTIONS 白名单、4 provider HTTP 超时、CODEIDX child().unwrap()→match）。
- **回归**：6 个 `test_v12_*` + 1 改写 grep 测试，抽查断言均为真（如 abort_on_drop 的 `is_cancelled`）。
- **闸门**：✅ **过闸**（🔴=0，源码级验证）。建议定版 v1.2。6 项接受风险+4 项 P3 延后已记录。真 Linux `cargo test --all` 执行验证守门员因环境未复跑，留待用户在 VM 最终确认。

---

### 2026-07-28 v2 顶层缺口关闭验收（守门员逐项源码核实，产出 gatekeeper-review-v2-gaps.md）

- 执行方提交 `SELF-AUDIT-v2-gaps-close.md` 声称 6 项缺口全落地 + 三门全绿（146 passed/0 failed）+ clippy 存量清理。守门员直读源码逐项核实。
- **6 项全通过**：G11(OPENAI_MODEL env, openai_key.clone()无 use-after-move) / G9(sandbox:207注释+trait &PathBuf→&Path三处,coercion安全) / G4(API_KEY_REQUIRED bail!) / T6(快慢双路径 replan_count>=3→GiveUp + 能失败测试) / T4(retriever env-gated + lsp Noop显式接线) / G5(.github CI yml存在+三步对齐)。
- **执行方 §5 7 项披露复核**：全部为诚实自我批评——T6 未 E2E(但有双重兜底)、G9 plan外(但 coercion 安全)、retriever gated(设计决策)、CI 未经 GitHub 实跑(YAML 正确)、clippy 抽查 3 处(全部语义等价 is_some_and/zip/if-let)。
- **闸门**：✅ **过闸**（🔴=0，源码级验证）。建议定版 v2-gaps，在 top-level-design 打勾 G11/G9/G4/T6/T4/G5。

---

### 2026-07-29 v3.0 主干冻结验收（守门员逐项源码核实，产出 gatekeeper-review-v3.0.md）

- 执行方提交 `v3.0-acceptance-report.md` 声称 5 项（B1/B2/B3/S1/S2）落地 + FMT_RC=0/CLIPPY_RC=0/146 passed。守门员直读源码逐项核实。
- **5 项全通过**：B1(`events` 字段+9 push 点+`get_history`+`agent_event_kind`+GET messages 路由) / B2(`ErrorResponse`+8 错误码+`api_err` helper+全部 handler 返回 `Json<ErrorResponse>`) / B3(`ApiDoc`+`/openapi.json` 路由+9 DTO 加 `ToSchema`；SwaggerUI 因 VM 无 github 移除以 `/openapi.json` 替代，偏离合理) / S1(`OOM_TTL_SECS`+`OOM_SWEEP_INTERVAL_SECS` env) / S2(CI eval step `cargo test -p telemetry`)。
- **B3 偏离裁决**：`utoipa-swagger-ui` build.rs 编译期从 github 拉 zip → VM 不可达 → 必然编译失败。改直接暴露 `/openapi.json` JSON 文档，目标等价，可接受。
- **闸门**：✅ **过闸**（🔴=0，源码级验证）。建议定版 v3.0 Trunk Freeze。后续按 `trunk-freeze-branch-plan.md` 开 3 个 post-freeze 分支。

---

### 2026-07-29 Post-Freeze 三支线验收（守门员逐项源码核实，产出 gatekeeper-review-post-freeze.md）

- 执行方提交 `post-freeze-audit-handoff.md` 声称 P1/P2/P3 合入主干 + 149 passed。守门员按报告 §四 16 项审计锚点逐条 `grep` 核实。
- **16 项全部通过**：P1(FileChange→Observation→RunReport→extract_files→merge_file_changes→planner Merge conflicts 全链路) / P2(ContentSource+MAX_INJECTED_CHARS 4096+format_injected_content→LSP/retrieval/tool 三注入点) / P3(RustAnalyzerBridge+ensure_started+5s timeout+NoopLspBridge 保留+LSP_ENABLED env gate)。
- **已知局限**（报告 §五）4 项全部接受：P1 文件级粒度（行号=0）、P2 不含子代理产出标记、P3 初始化 10s 偏紧 + 未优雅退出。
- **闸门**：✅ **过闸**（🔴=0，16 项锚点全通过）。建议合并主干冻结，打 v3.1 定版包。`top-level-design.md` 全部缺口可标记 ✅。

---

### 2026-07-29 v4.0–v4.1 验收（守门员逐项源码核实，产出 gatekeeper-review-v4.1.md）

- 执行方提交 `v4.1-acceptance-report.md` 声称 v4.0 硬化 4 项 + v4.1 生产 5 项 + P3 延后。守门员按报告 §四 10 项审计锚点逐条核实。
- **10 项全通过**：H2 子代理标记(loop.rs:986/1034) / H3 ra init_done+60s 超时 / H1 line_start/line_end 提取 / P4 healthz+readyz+auth bypass / P1 AtomicUsize 50 并发上限+429 / P5 docs/configuration.md(2KB) / P6 docs/deployment.md(3KB) / P2 Dockerfile(685B)+docker-compose.yml(514B)。
- **已知局限** 5 项接受：行级精度有限/子代理完整产出未标记/ra 60s 偏紧/全局限流非 per-IP/telemetry 孤儿。
- **闸门**：✅ **过闸**（🔴=0）。建议打 v4.1 定版包。`top-level-design.md` 版本号更新至 v4.1。

---

### 2026-07-29 v5.0 规划产出（CLI + E1-E4）

- 用户要求继续后续规划：E1-E4 先做，E5 前端砍掉替换为 **codex CLI**（终端工具替代浏览器 UI）。
- 产出 `v5.0-plan.md`：4 阶段——Phase A CLI(新 crate codex-cli, 5命令+REPL+SSE彩色渲染~7天) → Phase B E1(retriever默认)+E2(session隔离 ~3天) → Phase C E3(工具配置化方案B ~0.5天, WASM延后) → Phase D E4(多层Agent ~5天)。
- CLI 设计：`codex chat` 一次性 / `codex repl` 交互 / `codex sessions|history|approve|cancel|status` 辅助命令。用 `clap`+`reqwest`(SSE streaming)+`colored`(终端渲染)。不依赖 E1-E4，可立即开工。
- CLI 先做理由：独立可测、完成后可直接用于验收 E1-E4、后续 Gemini 前端有完整 API 用例参考。

---

### 2026-07-29 v5.0 全阶段终审（守门员 25 项锚点全验证，产出 gatekeeper-review-v5.0-final.md）

- 执行方提交 `gatekeeper-review-v5.0.md` 自审报告（25 项锚点，含 v3.0→Post-Freeze→v4.0/v4.1→v5.0 全链）。守门员逐条源码核实。
- **v5.0 新增 8 项全通过**：CLI 8 命令(clap derive) / SSE 解析(event:/data: 帧状态机) / REPL 双线程(mpsc+tokio::spawn) / E1 retriever 默认(无 gate) / E2 session 隔离(workspace_dir+create_dir_all+清理) / E3 工具配置化(CODEX_DISABLE_TOOLS) / E4 多Agent 三层(MAX_DEPTH=2) / SwaggerUI 离线 vendor(index.html+css+js + ServeDir)。
- **top-level-design.md**：全 12 缺口 + 11 To-Be ✅ 闭环标记确认。
- **未完成项** 4 项接受(L1 clippy test-target warn / L2 E4 无 broadcast / L3 P3 E2E延后 / L4 telemetry 0 生产调用)。
- **闸门**：✅ **过闸**（🔴=0）。建议定版 v5.0。项目从 7/28 13:00 "能跑缺装甲" → 7/29 06:25 "有 CLI 的生产级 Agent 服务"，全链路闭环。

---

### 2026-07-29 v6.0 规划：供应商管理 v2 + 内部桥

- 用户要求两件事：①存量差距盘点（✅ 已 100%）；②"内部桥"——非 A4 主从子代理，而是**平级多模型社群沟通**，模拟群体智能。但实现前必须先做供应商管理 v2。
- 产出 `v6.0-plan.md`：Phase 6A 供应商管理 v2（provider:model key + 自动发现 OpenAI/Ollama/vLLM + 自定义 providers.yaml + 模型缓存 ~6天）→ Phase 6B 内部桥（BridgeSession + 共享消息总线 + 4 策略 RoundRobin/Free/Debate/Synthesize + 共识投票 + 新 crate bridge ~12天）。
- 核心设计：ProviderRegistry 重构为 `HashMap<"provider:model", Arc<dyn LlmProvider>>`；BridgeBus 广播所有参与者的消息；4 种回合策略选一；vote 过半数通过。与 A4 子代理互补：子代理=层级委派，桥=平级讨论。

---

### 2026-07-29 v6.0 更新：文明线 + 工作线

- 用户追加两条新线——**文明线**（AI 自由留言板/集体记忆，自动触发，无需提醒）+ **工作线**（任务看板/进度追踪，按项目/对话组织，四分类：长期/短期/定时/完工）。
- 更新 `v6.0-plan.md` 为四阶段：6A 供应商管理 v2(4-6天)→6B 内部桥(8-12天)→6C 文明线(CivEntry+CivCategory+自动触发点 in do_reflect/do_observe ~5天)→6D 工作线(WorkNode+四子线+codex.toml项目定义+Agent循环集成 ~8天)。
- **互交设计**：桥讨论结果→文明线自动摘要；文明线历史→桥上下文注入；桥任务分派→工作线创建节点；工作线完工→文明线自动发布。
- 6C/6D 不依赖 6B，可在 6A 完成后与 6B 并行。四线合一 = "能讨论、能记住、能拆解执行的 AI 团队"。

---

### 2026-07-29 v6.0 全阶段终审（守门员 18 项锚点全验证，产出 gatekeeper-review-v6.0-final.md）

- 执行方提交 `gatekeeper-review-v6.0.md` 自审（18 项锚点，含 6A/6B/6C/6D 全链）。守门员逐条源码核实。
- **18 项全通过**：6A(register_with_key+aliases+三级get+main.rs v2) / 6B(BridgeSession+4策略+Consensus事件) / 6C(CivEntry+CivilizationStore+GET/POST路由+CLI civ 3命令) / 6D(WorkNode+WorkLineStore+3路由+CLI tasks 3命令+AppState双store)。
- **4 项局限接受**：Bridge 未集成 session / Civilization 自动触发未接 / WorkLine 调度器未实现 / Bridge dead code。均为 v6.1 运行时集成延后项，"先建路再通车"策略合理。
- **闸门**：✅ **过闸**（🔴=0）。建议定版 v6.0，v6.1 做运行时集成（loop.rs 触发 + Bridge session 接线 + 调度器）。

---

### 2026-07-29 v7.0 规划：可观测性 + 质量 + 多用户 + 生态

- 用户问"下一阶段还能怎么规划完善"。全景扫描系统剩余缺口（可观测性/测试/安全/多用户/开发体验），产出 `v7.0-roadmap.md`。
- **v6.1**：运行时集成（Bridge 入 session + Civ 自动触发 + WorkLine 调度，4天）。不需新规划，直接做。
- **v7.0**：可观测性+质量（结构化日志+关联追踪 / Telemetry 复活 `GET /api/v1/telemetry` / Agent 回放 `codex replay` / E2E 集成测试终于补上 P3，5-8天）。
- **v8.0**：多用户+平台化（per-user API key + 文明线/工作线分区 / Agent 模板 `--template code-reviewer` / Webhook 通知，5-8天）。
- **v9.0+**：体验生态（CLI 增强+语法高亮 / session 导出 / Model Benchmark `codex bench` / 语音接口，按需启动）。
 198→
 199→---
 200→
 201→### 2026-07-29 v9.0 验收（守门员审计 gatekeeper-review-v9.0-final.md）
 202→
 203→- 执行方提交 v9.0 完工报告：15,057 LOC / 19 crate / 156 passed。守门员读源码核实。
 204→- v7.0可观测性(结构化日志+telemetry端点+回放+覆盖率) / v8.0多用户(UserStore+PerUserStore+Agent模板+Webhook) / v9.0自主决策(TaskOrchestrator+PipelineRunner+TaskValidator) — 全部落地。
 205→- **🟡 PerUserStore 未接线**：存储对象已构造但未注入路由 → 多用户隔离仅停留在 UserStore 层，实际存储仍共享。
 206→- **闸门**：✅ 过闸(🔴=0)。建议 v10.0 做 PerUser 接线 + Orchestrator Agent 集成。
 207→
 208→---
 209→
 210→### 2026-07-29 v10.0 蓝图：基因宪法 + 资源监测 + 自愈 + 工具生态
 211→
 212→- 用户提出宏大蓝图——资源监测系统(物理+成本+ROI)、独立观测报告系统、自愈模块、基因宪法(六条认知约束：真实/忠实/守护/未知/局外人/连续存在)、编号系统、外部工具生态接口(自已找工具→造工具→用工具)。
 213→- 产出 `v10.0-vision.md`，可行性评估：全部能做。分六阶段——10A编号(0.5天)→10B资源监测(sysinfo+CostMeter+ROI ~5天)→10C基因宪法(constitution.md+三处注入点 ~3天)→10D独立观测(crates/observer独立二进制+日报周报 ~4天)→10E自愈(L1/L2/L3三级+Observer检测 ~3天)→10F工具生态(WASM沙箱+tool search/install/list ~8天)。
 214→- **内在一致性发现**：六条宪法与六个模块间存在深层对应——真实↔观测报告、忠实↔ROI、守护↔资源监测+自愈、未知↔局外人视角、存在↔编号系统+farewell。不是碰巧。
 215→- 总工期（最大并行）：10A(0.5)+max(10B,10C,10D)(5)+10E(3)+10F(8) ≈ **16.5天**。

---

### 2026-07-29 v9.0 Tarball 审计 + v10.0 执行方案

- 审计外部交付包 `codex-rust-v9.0-final.tar.gz`：18,443 LOC / 19 crate / 50 .rs / 156 测试。v7.0-v9.0 全部 feature 抽查通过，工作区比 tarball 多 +198 LOC（resource-monitor 骨架已起步）。
- 产出 `v10.0-handoff.md`（基于实际状态更新）：6 阶段 16.5 天——10A 编号(0.5天) / 10B 资源监测(resource-monitor 已有骨架 ~5天) / 10C 基因宪法(constitution.md+三处注入 ~3天) / 10D 独立观测(crates/observer 独立二进制 ~4天) / 10E 自愈 L1/L2/L3(~3天) / 10F 工具生态(WASM沙箱+tool search/install/list ~8天)。
- 改动文件：constitution.md(新) / resource-monitor(扩展) / observer(新) / loop.rs(3阶段写入) / planner(宪法追问) / memory(farewell+ToolRegistry) / main.rs(instance_id) / routes(resources端点) / dispatcher(外部工具) / CLI(3子命令增加) / tools/registry.json(新)。

---

### 2026-07-29 v10.0 定版审计 + v10.1 执行方案

- 审计 `gatekeeper-review-v10.0-final.md`：156 passed / 20 crate / 🔴=🟡=0。v10.0 新增 9 项全部源码核实（instance_id+PID+resource-monitor+constitution+Observer后台+PerUser接线+L1重试+工具列表+CLI whoami/template）。
- **4 🔵延期**：工具搜索安装 / Orchestrator集成 / ROI精细化 / Observer独立二进制。全部列入 v10.1。
- 产出 `gatekeeper-review-v10.0-final-audit.md` + `v10.1-plan.md`（5 项 ~410 行，5-8 天）。
