# 后端执行窗口任务书 #01 — 内核契约落地 + 真实可用产物形态

> **签发**：顶层（守门员）
> **日期**：2026-08-21
> **执行窗口**：后端 dev-execution 窗口（照本任务书施工，跑门禁，交证据）
> **来源**：`frontend-backend-split.md` §6（BE 七项）+ 用户 2026-08-21 现场指令（"后端整理一下，给执行窗口做；做完看能否推出一般真实环境用的前后端桌面版，我 Linux 用；桌面难就先做 CLI"）
> **性质**：全部 **C 类（只增不改超集）** / 部分触及 **D 类（审批/澄清为人类介入点，已在内核存在，本任务只接真后端、不新增控制流语义）**

---

## 0. 背景与终态目标（用户原话转译）

1. 前端（Human OS 投影层）已收口：冻结 demo v21j + #01 暗色/可达性 + #02 审计面扩展，**109 断言全过**，契约稳定。
2. 后端 = 让 Hearth（原 `codex-rust`）内核**真正产出** `frontend-backend-split.md` §3 定义的 AI OS 事件流（带 `ts/seq/span_id/parent_id` 信封），使**已完成的 demo 能零改动消费真实内核流**——这是"前后端解耦达标"的硬证明。
3. **产物形态（用户明确）**：最终要一个**能在一般真实 Linux 环境使用的前后端桌面版**项目（Hearth），用户本人在 Linux 上用。若直接做桌面版有困难，**先做 CLI 版**作为可达中间态。
   - 桌面版 = 真实后端（Hearth 内核）+ 真实前端（把 demo 的 mock 事件脚本替换成订阅真实 SSE）。
   - CLI 版 = 真实后端 + 终端 TUI/流式文本输出（同样消费同一事件流）。

> 内核与事件流是**唯一事实源**；前端/CLI 只是不同投影。先做内核+事件流，桌面/CLI 共用，互不阻塞。

---

## 1. 源码现状（守门员已核验，避免凭印象）

`crates/agent-core/src/loop.rs` 已存在**部分真接线**：
- `enum LoopPhase { Init, Plan, Act, Observe, Reflect, Done, Error }`（:29）
- `enum Event { Phase, Token, ToolCall, ToolResult, ThinkSummary, Artifact, SpanOpen, SpanClose, ... }`（:41）——**不是从零**：WP-1 span 生命周期、WP-8 产物登记、WP-9 思考摘要均已在 loop 内 emit。
- `set_event_sender` / `emit()`（:562/:699）——事件已能流出到 `tokio::sync::mpsc`。
- `loop.rs:1986-2036` 已有 `t0/t1/duration_ms` 真实时序记录（嵌在 do_act 等相位）。
- `tool_call_needs_approval(&tc)`（:2281+）已有大量 wiring 断言——**审批门判定已在内核**。

**缺口（本任务要补的真活）**：① 事件**没有统一信封**（`ts/seq/span_id/parent_id/schema_version` 缺失，无法表达 span 树）；② 没有 **SSE 出口**（事件停在 mpsc，未序列化推送）；③ **规划缺口推导器**（`plan.draft` + `gaps_found` + `auto_assumed`）尚未落地；④ **need_clarification / need_approval 的双向交互协议**（emit→等人类→resume）未闭环；⑤ `open_artifact` 命令未实现。

---

## 2. 任务清单（分批，按依赖排序）

### 批次 B1 — 事件信封 + SSE 出口（地基，必先做）
- **B1-1 事件信封**：在 `api` crate 定义统一信封 `AgentEventEnvelope { ts, seq, span_id, parent_id, schema_version, type, data }`，把 `agent-core::Event` 映射为带信封的流（服务端分配 `span_id`、补 `parent_id`、自增 `seq`、打 `ts`）。
- **B1-2 SSE 出口**：在 `service` crate 把内核事件统一序列化为 SSE（`text/event-stream`，§3 信封逐行推送）。暴露端点（HTTP `/stream` 或 Unix socket——**Linux 桌面/CLI 均可消费**）。
- **B1-3 契约冻结文件**：产出 `docs/ai-os-event-contract-v1.md`，把 §3 信封/§4 span tree/§5 clarify·approval 协议**钉死版本号**，作为前后端解耦的唯一契约。
- **验收（硬闸门）**：用 `codex-cli`（已有 `client.rs`/`render.rs` SSE 线索）或最小订阅脚本连上 SSE，收到真实 `span_open/span_close/tool_call/artifact/think_summary` 信封事件。

### 批次 B2 — 规划缺口推导器（最高优先级，产品红线）
- **B2-1 `plan.draft` 产出**：`do_plan` 完成后 emit `plan.draft { steps[], gaps_found, gaps_to_ask, auto_assumed, mode }`。
- **B2-2 缺口三元组**：每个 gap 必须带 `{ from(规划步骤), why(具体代价/返工面), blocking, options?, assume? }`。**禁止输出无 `from`/`why` 的通用问题**（产品级红线）。
- **B2-3 `auto_assumed[]`**：把"有唯一答案、无需打扰人类"的判断（读 Cargo.toml 得框架版本等）显式列出随 `plan.draft` 下发。
- **验收（硬闸门）**：随机抽 10 个真实任务，其规划期 gap **100% 带可追溯 `from` 与具体 `why`**；`blocking=false` 项必须带 `assume` 默认值。不满足即判"为问而问"，退回重做。

### 批次 B3 — clarify / approval 双向闭环（D 类，但内核已存在语义，本任务只接真后端）
- **B3-1 clarify**：规划前批量 emit `need_clarification`，收齐 `clarification` 后 resume 后续链路；执行期仅在「运行时才可知 + 不可逆/超范围」双条件才中途打断。
- **B3-2 approval**：危险操作（write_file / shell / bind_tcp）前 emit `need_approval { op, risk, impact, why }`，应用裁决（human 或 policy 自动，按协作模式）。
- **B3-3 `open_artifact`**：实现命令——file → `xdg-open`/系统默认程序，url → 默认浏览器（Linux 用 `xdg-open`）。
- **验收（硬闸门）**：端到端跑一个需审批的真实任务，内核 emit→等待→人类/策略裁决→续跑，链路不丢事件（G4 断线重连不丢）。

### 批次 B4 — 产物形态：CLI 优先，桌面版随后
- **B4-1 CLI 版（先做，用户授权的中间态）**：基于 B1 的 SSE，做终端流式客户端——消费同一事件流，渲染 span 树缩进 / 相位 / 工具调用 / 产物卡片 / 审批-澄清交互（TUI 或富文本流式）。Linux 直接 `cargo run` 可用。
- **B4-2 桌面版（CLI 打通后做）**：把已完成 demo（Human OS 投影层）的 mock 事件脚本替换为订阅真实 SSE（**demo 零改动契约验证**——这是 §6 验收闸门）。技术选型评估：Rust + Tauri（Linux 友好，WebView 渲染现有 HTML/CSS）优先；若 Tauri 在目标 Linux 环境构建受阻，回退到 `wry`/`egui` 自绘或保持 CLI。
- **验收（硬闸门，桌面版）**：真实后端 + 真实前端 demo，**demo 源码零改动**即能消费真实事件流（三栏/联动/折叠/暗色全工作）。

---

## 3. 硬约束（同前端任务书，不可妥协）

1. **事实产生权公理**：后端产生事实，前端/CLI 只投影。BE 给语义（如 `"warn"`），FE 给呈现（黄）；BE **禁返回颜色/HTML/坐标**。
2. **超集不改**：所有新增走 C 类（只增不改）；D 类（审批/澄清为人类介入点）仅接真后端，**不新增控制流语义**、不破坏现有 `tool_call_needs_approval` wiring。
3. **契约稳定优先**：`ai-os-event-contract-v1.md` 钉死后，前端/CLI 按契约消费；内核改事件结构必须升 schema_version，不得静默破坏消费方。
4. **门禁**：每批交付前 `cargo fmt --check` + `cargo clippy -D warnings` + `cargo test --workspace`（维持 240+ 基线，不得退） + 本批硬闸门（B1/B2/B3/B4 各自验收项）。
5. **测试必须能失败**：每批新增门禁断言配 `[自检]` + 旧写法样例（继承前端方法论）。

---

## 4. 与既有挂账的关系（不重复造轮子）

- **RT3 seccomp 白名单**：属 G0 安全层，独立于本任务书；建议在 B1 之前或并行排期（硬安全边界），但**不阻塞** B1-B4 推进。
- **Q7 熔断恢复 / Q4 时间轴回滚 / T19 通过率 / Docker 复验**：属运维/验证挂账，归顶层/验证窗口，不在本执行窗口范围。
- **Q1/Q2/Q5/Q6 业务决策**：用户拍板，不在本任务书。

---

## 5. 交付与验收节奏

- 后端执行窗口**按 B1→B2→B3→B4 顺序**施工，每批独立完成门禁 + 硬闸门证据，回顶层守门员验收（同前端：不信报告信源码，独立核验）。
- 每批交付物：代码 PR/commit + 本批验收证据（B1 的 SSE 抓取样本 / B2 的 10 任务 gap 审计表 / B3 的端到端审批链路日志 / B4 的 CLI 运行录屏或桌面版 demo 零改动消费证明）。
- **最终交付目标**：一个 Linux 可用的真实项目（桌面版优先；若受阻则以 CLI 版为可达态），用户本人可在 Linux 上跑真实任务。

---

## 6. 红线（越界即打回）

- 不得为"过测"在 BE 里硬编码前端展示细节（颜色/HTML/坐标）——违反事实产生权。
- 不得静默破坏 `ai-os-event-contract-v1.md` 契约（未升 schema_version 就改事件结构）。
- 规划缺口推导器**不得输出无 `from`/`why` 的通用问题**（产品红线，B2 硬闸门）。
- 桌面版不得为了迁就 WebView 而破坏冻结的 demo 联动契约 / 浅色区视觉（前端 #01/#02 红线同样适用）。
