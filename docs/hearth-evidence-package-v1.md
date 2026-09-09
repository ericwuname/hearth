# Hearth 行业差距分析 · 证据材料包 v1

> 交付对象：外部评审方（Claude / 行业差距分析）
> 编制方：执行窗口 AI（有源码访问权限）
> 日期：2026-08-28
> 配套需求清单：`docs/hearth-evidence-request-for-industry-gap-analysis-v1.md`

## 执行声明（必读）

本包按需求清单 A–F 逐项应答。**能跑出来的给原始证据，做不到的显式标注原因，不凑数字。**

- **环境限制（硬约束）**：本沙箱 `cargo` 仅为 rustup 代理，无实际 Rust 工具链，**无法本地编译/运行 `cargo test`**。因此 A/B/D/E 中需要「运行」的项，改为「静态取证 + 明确的条件标注」；如需权威数字，需到你方 VM（`~/.workbuddy/workbuddy.db` 之外无此环境）或 CI 原始日志核对。
- **源码基线**：随手并存 HEAD（仓库 `master` 滞后 78 提交，详见先前 rot 审计）；所有 `file:line` 指向当前工作区源码。
- **诚实标注**：凡静态取证项，均注明「静态/动态」；凡未亲跑的项，均注明「需 VM/运行环境」。

---

## A. 测试总数权威口径 — 已实测（本会话 VM 实跑 `cargo test`）

**结论（权威，本会话 VM 实测，终结「351/339/194/363」四数之争）**：在 **HEAD（滞后约 78 提交，≠ v0.2.3）** 上于 VM（Ubuntu 24.04 / 8 核）实跑：

- 全量 `cargo test --workspace`（不含 doctest）：**272 passed / 10 failed / 1 ignored**。
- 10 个失败**全部位于 `sandbox` crate**，且根因单一：VM 非 root 用户无 cgroup delegation，`/sys/fs/cgroup` 创建被 `Permission denied (os error 13)`，触发 RT4 fail-closed 拒绝 spawn（报错「安全限制无法保证」）。**这是环境限制，非代码缺陷**。
- 用文档化降级开关 `HEARTH_ALLOW_NO_CGROUP=1` 重跑 sandbox crate → **17 passed / 0 failed / 1 ignored**（仅 `test_rt4_cgroup_fail_closed` 因 env 注入竞态被 `ignore`）。
- 因此在「正确环境配置」（cgroup 已 delegation，或显式 `HEARTH_ALLOW_NO_CGROUP=1`）下权威计数：**282 passed / 0 failed / 1 ignored**。

**对照历史口径**：v0.2.3 = 339 passed 来自历史 commit，与 HEAD 不同；且本 VM 未授予 cgroup delegation。四个相互打架的数字在此收敛为：**282（全 crate，含环境兜底）**，并明确标注环境前提。

**已做（实跑命令，VM `~/codex`，复用 23G target 增量缓存）**：
```bash
source ~/.cargo/env && cargo test --workspace            # 272 / 10 failed / 1 ignored
HEARTH_ALLOW_NO_CGROUP=1 cargo test -p sandbox          # 17 / 0 / 1（10 个失败全部转绿 → 证伪「代码缺陷」）
```
失败逐条核对：10 个失败均为 `无法创建 cgroup ... Permission denied (os error 13)——安全限制无法保证（RT4 fail-closed）`，错误完全同源。

**行为级验证（直接回应 Claude 的 P0-2 / F.2 追问）**：
- **P0-2 超时 / deadline**：`test_deadline_exceeded_activates`、`test_deadline_run_produces_deterministic_terminal_state`、`test_t6_replan_hard_cap_forces_giveup` 本轮均 `ok` —— 任务级 wall-clock deadline 与 hard-cap **已行为级验证通过**，非仅存在性取证。
- **F.2 cgroup fail-closed**：10 个失败本身就是「反面实证」——cgroup 创建被拒时沙箱正确 fail-closed（返回 Err「安全限制无法保证」），与「隔离失败=启动失败」一致。`test_rt4_cgroup_fail_closed` 在自动化门禁中被 `ignore`（env 注入竞态），手动验证备注已 PASS。

**下一步**：若需「339」级别可复现绿色门禁，需在 VM 配置 cgroup delegation（或把 `HEARTH_ALLOW_NO_CGROUP=1` 定为测试环境标配）并固化进 CI。

---

## B. 真实运行可靠性遥测 — 已实测回填（2026-08-28 · VM 自跑）

**结论**：本会话已在 VM 对 `hearth chat` / `hearth resume` 做了**分层**可靠性遥测，按「URL+key 组合通不通」判定通道（不按 provider 名字，名字可乱起、但 URL/key 错就真不通）。三层结论**严重分化**：

| 层级 | 内容 | 结果 | 崩溃 / 挂起 / 后端错 |
|---|---|---|---|
| **Tier 1 — 一次性简单对话** | 固定极简目标（一句话/2+2 类）| **30/30 成功**（§D-回填 5 次 + R1 15 次 + R2-A chat 10 次）| 0 / 0 / 0 |
| **Tier 2 — resume 续跑** | chat 建 session → 取 uuid → resume 续跑 | **10/10 成功**（rc=0，生成 run-002.md）| 0 / 0 / 0 |
| **Tier 3 — 长程自主任务** | 推理密集、压近 step 预算、不依赖外部工具 ×10 | **0 完成 / 3 挂起 / 7 任务失败** | 0 / 3 / 0 |

- Tier 1 单次 2.8–33.8s（中位 ~5s，少数冷启 ~33s），每次均套 landlock+seccomp、出执行报告。
- Tier 2 续跑能干净接上上下文并落新报告。
- **Tier 3 无一 `✓ Task completed`**——这是本批最重要的信号（详见 B-1）。

### B-1 长程自主任务可靠性（Tier 3）专项 — 核心发现

**结论：Hearth 的「地基」（一次性对话、resume）已数过且稳；但「长程自主任务」这一层 100% 不能干净完成。** 这恰好印证外部评审（Claude）点名的「瓶颈已从读代码转到 B 类真实遥测」——而本批就是把这个空白第一次数了出来。

**10 轮失败明细（原始 tail 证据在 `C:/tmp/hearth_telemetry_r2.txt`）：**
- **挂起 ×3（`rc=124`，触 200s 硬超时）**：
  - B03：卡在 `Reflect → Plan` 死循环（tail：`◆ Reflect / continue / ▸ span [plan]` 反复，无终止）。
  - B04 / B06：后端 `read body failed — error decoding response body`（provider=deepseek）重试退避后超时——属**后端/传输层返回体无法解码**，客户端无限退避直至超时。
- **任务失败 ×7**：均 `✗ Task failed — status=failed（13–17 步）`，会生成**部分报告**并提示「可直接下指令继续」——即 agent 自认未达成目标，但能优雅降级而非进程崩溃。

**两类根因归类：**
1. **agent 控制流缺陷**：长任务在 Plan/Reflect 间空转（B03），或步数预算内未收敛（7 次 task_failed 均停在 13–17 步）——属 planner/loop 在「长程自主」场景的未过关项。
2. **后端传输脆弱性**：deepseek 偶发返回体解码失败 → 客户端退避无上限 → 挂起（B04/B06）——属 LLM 网关对「畸形响应体」缺少快速失败/熔断。

**诚实边界（未覆盖的真风险）：**
- Tier 3 目标**刻意隔离了外部工具链**（纯推理），「长任务 + 文件/命令类工具」的稳定性未压。
- 步数预算上限（~40 步）附近的**压缩触发**与**挂起关联**未单独隔离观测。
- 「长任务中途崩溃 → resume 续跑」的**组合场景**未测（Tier 2 用的是极简目标续跑，非长任务续跑）。

### B-2 遥测方法与诚实披露

- **通道判定**：VM 实际配置且**实测能出真实 completion** 的 URL+key 组合（本轮为 `api.deepseek.com` + 有效 `sk-...` key）。R1 结果文件未记录 provider，按「实际产出 15 个真实 completion 的那套组合是通的」报，**不杜撰品牌名**。
- **两个 harness bug 已如实披露、未污染结论**：
  1. R2 首跑 Part A 的 10 次 resume 全 `TASK_FAIL`——脚本误将 `--budget` 传给 `resume <ID>`（clap 报 `unexpected argument '--budget'`），修正重跑得 10/10；原错数未上报。
  2. R1 曾口称「Agnes」，但结果文件无 provider 记录，已按 URL+key 口径更正。
- **原始逐轮证据**：`C:/tmp/hearth_telemetry_result.txt`（R1 15/15 + R2 原始）、`C:/tmp/hearth_resume_fix.txt`（resume 修正后 10/10）、`C:/tmp/hearth_telemetry_r2.txt`（长任务 10 轮原始 tail）。

---

## C. 关键源码原文摘录 — 已采集（可定位到函数级别）

### C.1 主循环（Plan→Act→Observe→Reflect）与超时机制

`LoopPhase { Init, Plan, Act, Observe, Reflect, Done, Error }`（agent-core/src/loop.rs，按先前审查确认）。**是否已有时钟上限**：**有**。

- 任务级 wall-clock deadline（顶层超时）——`agent-core/src/loop.rs:3059`：
  ```rust
  // H2 (v0.2.4): 任务级 wall-clock deadline——先于步数检查（时间不可逆）。
  if self.ctx_mgr.deadline_exceeded() {
      let elapsed = self.ctx_mgr.run_elapsed_secs();
      ... "deadline exceeded: {elapsed}s >= {cap}s（任务级 wall-clock 上限）"
      ... "status": "deadline_exceeded"
  }
  ```
- 重试上限——`agent-core/src/loop.rs:1967`：`const TOTAL_RETRY_CAP: Duration = Duration::from_secs(120);`（30s × 2 容错 → 撞 120s 上限提前终止）。
- 交互请求超时（WP-0）——`loop.rs:1038` `timeout: Some(60)`、`on_timeout: Some("deny")`；`loop.rs:1665/2255/3111` 多处 `on_timeout: Some("abort")`。

> **完整 `run()` 函数（约 5000 行）未全文摘录**。如需逐行，可单独 dump `loop.rs` 全部内容——请确认是否需要。

### C.2 `resume` 实现路径

`agent-core/src/loop.rs`（核心恢复逻辑）：
```rust
// X1-4 (v0.1.6): resume——把落盘的 Turn 历史灌回 ctx_mgr（重建会话上下文）。
pub fn restore_history(&mut self, turns: Vec<agent_types::Turn>) {
    self.ctx_mgr.state_mut().history = turns;
}

// 任务状态持久化 (v0.2): resume 恢复任务图——节点状态保留，不重新 decompose。
pub fn restore_task_graph(&mut self, graph: serde_json::Value) {
    if graph.is_null() { return; }
    if let Ok(tg) = serde_json::from_value::<TaskGraph>(graph) {
        if !tg.nodes.is_empty() {
            self.task_graph = tg.clone();
            self.plan_state.task_graph = tg;
            self.needs_decompose = false;   // 有已恢复图 → 不再强制首轮 decompose（甘特图反复横跳根因修复）
        }
    }
}
```

调用路径（codex-cli/src/lib.rs:535-605）：
- `rebuild_agent(...)` → `agent.restore_history(turns)`（L558）
- `load_graph_with_revision` + `load_taskgoal` 做 `state_revision` 一致性校验（不一致 → 标 stale + warning，**不阻断**）（L565-600）
- `agent.restore_taskgoal(...)` + `agent.restore_task_graph(...)`（L583/601）
- 最后 `run_local::run_local_continue(agent, &id, &msg, REPL_BUDGET)`（L603）

repl.rs:435/449 也有同样的两段 restore 调用。

### C.3 `llm-openai` usage 结构体与请求体拼接

**结构体（响应侧反序列化，`llm-openai/src/lib.rs:111-121`）**：
```rust
#[derive(Deserialize, Debug)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
    /// P1-
 (v0.2.4): DeepSeek 系扩展——缓存命中/未命中（缺失时 None，不误报 0）。
    #[serde(default)] prompt_cache_hit_tokens: Option<u32>,
    #[serde(default)] prompt_cache_miss_tokens: Option<u32>,
}
```

**请求体拼接（`build_request`，`llm-openai/src/lib.rs:594-720`）**：
- 请求体从 `req.messages`（agent 的上下文）**逐轮重建**为 `Vec<OpenAiMessage>`（L595-660）；`tools` 由 `req.tools` 转换（L693-709）。
- **关键观察（针对 E 的「缓存命中」猜测）**：消息用 `Vec`（有序），**无 HashMap**，不存在「HashMap 顺序随机」导致的 prefix 断裂风险；system prompt 内嵌在 message 列表首部，随每轮重新序列化。prefix 最可能的断裂点是「每轮新增的 assistant/tool 轮」——这是语义上必然变化的部分，属正常。是否真正命中缓存需运行时 `usage.prompt_tokens_details.cached_tokens` 实测（见 E）。

> 注意：需求原文把 `OpenAiUsage` 描述为「请求体」字段，实际它是 **provider 响应**的 usage 解析（含 `prompt_cache_hit_tokens`）。请求体与响应 usage 是两个独立结构。

### C.4 `maybe_compact`（上下文压缩）— 压缩不丢数据

`agent-core/src/context.rs:176-255`：触发压缩（>`COMPACT_CHAR_THRESHOLD`）时，**先落盘归档、再折叠**：
```rust
pub fn maybe_compact(&mut self) -> bool {
    if self.estimate_chars() < Self::COMPACT_CHAR_THRESHOLD { return false; }
    let keep_from = ...; if keep_from == 0 { return false; }
    let old: Vec<Turn> = self.state.history.drain(..keep_from).collect();
    // H3: 先归档原文（best-effort），再折叠摘要
    if let Err(e) = archive_compacted_turns(&self.session_id, &old) {
        tracing::warn!(error = %e, "compaction archive write failed (non-fatal)");
    }
    // ... 折叠摘要插入 history 最前，并在首条注入归档检索提示 ...
}
```
`archive_compacted_turns`（`context.rs:242-255`）按会话隔离写 `archive/<session_id>.jsonl`（best-effort，在压缩困境仅 warn 不阻塞）。**结论：被压缩的原始消息有落盘归档，不是直接从内存丢弃**——纠正了「压缩即丢失」的担忧。

> ⚠️ **已知缺口（归档 ≠ 可检索，回应 Claude 提出的 P0-5 核查点）**：`archive_compacted_turns` 仅负责**写入**；全仓**生产代码不存在任何读取器**把归档喂回 agent 上下文或提供检索接口。唯一的 `std::fs::read_to_string(&archive)` 出现在 `context.rs:574/641/663`，且均在 `#[test]`（`test_compaction_archive_contains_originals`、`test_archive_per_session_isolation`）内——是测试自证，不是生产组件。
> **结论分层**：①「压缩数据不丢」成立（落盘了）；②「agent 能否把归档读回来恢复/检索」**不成立**——当前没有任何生产路径能把 archive 重新载入上下文。这是「压缩不丢」之后的真实下一步缺口：若要「可恢复」，需补一个 archive 读取/检索接入点（例如 resume 时回填或 `/context restore` 命令）。这与「压缩即丢失」是两个不同层面的问题，不可混淆。

### C.5 Observer「零执行权」— 成立（代码级 + 下游链路）

`observer/src/lib.rs:1-59` 铁律注释 + 实现：
- 「零执行权：只消费事件流（`Vec<EnvelopedEvent>`），产出 Finding / 报告 / 熔断事件。不调用任何工具、不修改计划、不尝试修复。」
- `Observer::run(&[EnvelopedEvent]) -> Result<(Vec<Finding>, Vec<CircuitBreak>)>`（L37）——纯函数式；仅做 seq 单调递增校验（L2 fail-closed），然后 `metrics::compute` / `rules::evaluate` / `circuit::evaluate`。
- 下游消费（唯一执行点）：`agent-runtime/src/session.rs:722-755` 在会话结束后调用 `observer.run(events)`，结果**仅落盘报告**（`run_and_report` 写 `<dir>/reports/<session_id>/`），无任何「调用工具/执行命令/网络请求」分支。
- `circuit::evaluate`（circuit.rs）只产出 `CircuitBreak` 事件；`rules::evaluate` 只产出 `Finding` 文本。全部为「表达事实」，无可执行副作用。

**唯一执行点（全仓 grep 实证，回应「唯一」的成立条件）**：
- `Observer::run` 定义于 `observer/src/lib.rs:37`，生产代码中仅被同一 struct 的方法 `run_and_report`（`lib.rs:64-70`，`self.run(events)?`）调用；
- `run_and_report` 的全仓**唯一生产调用点**是 `agent-runtime/src/session.rs:752`（会话结束后的报告落盘，落在 `session.rs:722-755` 区块）；
- 其余 `obs.run(&events)` / `Observer::run(&events)` 全部位于 `observer/src/lib.rs:231/255/283/285` 的 `#[test]` 自测内（非运行时）。
→ 事件流的消费出口只有 `session.rs:722-755` 一处，且该处输出仅写 `<dir>/reports/<session_id>/`，无任何工具调用/命令执行/网络分支。「唯一执行点」成立。

**结论**：「零执行权」在代码层面成立。建议（如需求所提）后续补一条自动化链路追踪，把「事件 → Observer → 落盘」全路径跑一遍，但当前静态链路已足以支撑「无任何副作用路径」判断。

### C.6 `bridge` crate — 完整但未接线

`matching`：`crates/bridge/src/lib.rs`（285 行**全文已读**）。`BridgeSession` 实现了 `run_round_robin` / `run_majority` / `run_debate` / `run_single` 四种多模型协商策略，逻辑完整。**但 `BridgeSession` 在 `agent-runtime/Cargo.toml:19` 与 `service/Cargo.toml:24` 声明依赖，grep 全仓 `BridgeSession`/`use bridge` 零调用** → **「完整实现但未接线」结论准确**。这是一个「完整但未接入主干」的半成品，不是空壳。

---

## D. Hearth vs Codex CLI / Claude Code 对照实验 — 需第三方环境

**结论**：本沙箱无 Codex CLI / Claude Code 额度与环境，也无法运行 hearth 多轮任务。**无法横向对比**。

**待办**：若你方有 Codex CLI / Claude Code 可用额度，按计划跑 3-5 个真实任务（bug 修复 / 多文件重构 / 需测试验证）并填表；若均无，改为「Hearth 同一任务跑 5-10 次」测自身稳定性方差。两种路径都请给出逐次原始记录，不要只给百分比。

### D-回填（2026-08-28 · 本 VM 自跑 5 次稳定性方差——handoff §4 落地）

**D-1 稳定性方差（v0.2.8，同任务 5 连跑，原始记录 `data/stability-5run-20260828.csv`）**：

| run | rc | 耗时(s) | 写盘字节增量 |
|---|---|---|---|
| 1 | 0 | 10 | 3304 |
| 2 | 0 | 13 | 3278 |
| 3 | 0 | 11 | 3280 |
| 4 | 0 | 11 | 3299 |
| 5 | 0 | 14 | 3296 |

**统计**：成功率 **5/5 = 100%**；耗时均值 11.8s、极差 4s（无挂起、无 timeout 触发）；写盘增量稳定在 ~3.3KB/任务——与 57G 失控事故（2.7G/步）形成基线对照，R2-C ResourceLedger 从此有正常值参考。

**D-2 端到端行为观测**：本批 5 次均为「一句话任务→明确终态投影（✓ Task completed）→干净收尾」的真实运行观测，且 G1 终态投影 / Task Continuity（R2-D/R2-E 交付）在其中直接生效。但「会挂起的任务类型在配置时间点自动干净收尾」仍属 Tier-2 开放项（见 §2 两级台阶标注），本批不声称覆盖。

---

## E. Prompt 请求体稳定性原始 dump — 需运行时

**结论**：请求体组装逻辑已静态分析（见 C.3），但「连续两轮逐字节 diff」「缓存命中字段实测」需要**运行时注入 debug 日志**。
- `llm-openai/src/lib.rs:318-320`：`msg_count <= 3` 时打印 `req_body`（含完整 system prompt）；`L713-714` 输出 `messages`/`tools`。
- 待办：在真实多轮场景每行 `debug/trace` 落盘请求体 JSON，`diff` 两轮的精确断裂位置；并捕获 provider 返回的 `usage.prompt_tokens_details.cached_tokens`（字段名以实际 provider 文档为准）。

---

## F. 安全边界行为级证据 — 已采集（含纠正）

### F.1 Observer 零执行权：见 C.5（代码级 + 下游链路已给出，无需重复）。

### F.2 cgroup fail-closed：**已实现，并非「未做」**（纠正需求文档的旧判断）

需求文档称「cgroup fail-closed 是已知未做缺口」——**与代码不符**。实际：

- `sandbox/src/lib.rs:811`：`RT4: fail-closed——cgroup 不可用/创建失败 → Err（安全限制无法保证，不开工）`。
- `apply_cgroups_impl`（L814）：cgroup v2 不可用时，若未设 `HEARTH_ALLOW_NO_CGROUP=1` → 返回 Err（spawn 直接失败）；设了该 env 才**显式降级**（warn「进程无内存/CPU/进程数上限」）。
- 真实写入 `memory.max` / `cpu.max` / `pids.max`（L878-912），读写失败同样 fail-closed 报错。
- 行为级测试 `test_rt4_cgroup_fail_closed`（L1631）断言：base 指向不可写 → 报错且必须含「安全限制无法保证」/「cgroup」字样。

**结论**：cgroup fail-closed **已落地且有测试覆盖**。真实行为是「cgroup 初始化失败 → 启动失败（fail-closed），除非显式 `HEARTH_ALLOW_NO_CGROUP=1` 降級」。所谓「缺口」应更正为「真限制生效依赖 cgroup delegation 配置，非 root 默认环境需授权」——这是运维前提，不是代码缺口。

---

## 差距与下一步

| 类别 | 状态 | 说明 |
|---|---|---|
| A 测试口径 | ✅ 已实测 | HEAD 实跑 = 282 passed / 0 failed / 1 ignored（含 `HEARTH_ALLOW_NO_CGROUP=1` 兜底；原 10 失败系 VM cgroup 权限，已证伪为环境） |
| B 可靠性遥测 | ✅ 已实测（分层） | Tier1 简单对话 30/30 + Tier2 resume 10/10 全绿；**Tier3 长程自主任务 10 轮全败（3 挂起/7 失败）**——真实风险已定位（B-1） |
| C 源码摘录 | ✅ 已采集 | C.1/C.2/C.3/C.4/C.5/C.6 均已定位到函数级；C.1 全文待确认是否需要 |
| D 对照实验 | 🔴 需环境 | 无 Codex/Claude Code |
| E Prompt dump | 🔴 需运行 | 需运行时 debug 注入 |
| F 安全边界 | ✅ 已采集 | Observer 零执行权成立；cgroup fail-closed 已实现（纠正「未做」误判） |

**最高信号**：A/C/F 三项已实测/采证——A 在 HEAD 实跑得 282/0/1（原 10 失败已证伪为 VM cgroup 环境限制，非代码缺陷）、C 源码摘录（含 C.5 唯一执行点 grep 实证）、F 安全边界（含 cgroup fail-closed 行为实证）；**B 已于本会话回填**（分层遥测：Tier1 30/30 + Tier2 10/10 全绿，Tier3 长程自主任务 10 轮全败——3 挂起/7 失败，根因归为 agent 控制流空转 + 后端畸形响应体退避无上限）；D-回填（5 次稳定性方差 100%）已完成；E 仍待运行时 debug 注入。F.2 纠正了需求文档「cgroup 未做」的过时判断。**下一步最高优先级：立项修 Tier3 长程自主任务可靠性**（控制流收敛 + LLM 网关熔断），这是当前唯一被数出来的、通向「项目真落地」的硬阻塞，比再补任何扩展性功能都值钱。
