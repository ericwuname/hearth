# 主干封板与枝干生长施工文档（trunk-freeze-branch-plan）

> 版本：v1.0 ｜ 日期：2026-07-28 ｜ 状态：待执行
> 交付对象：**执行窗口**（拆解执行）＋ **守门员窗口**（验收）
> 前置依据：`docs/trunk-qualification-audit.md`（主干资格全面盘点，5 层源码级探查）
> 基线状态：VM 真 Linux 三门全绿（fmt=0 / clippy -D warnings=0 / test 146 passed 0 failed）

---

## 0. 执行窗口铁律（先读这节，再动手）

1. **不信本计划，信源码。** 本文档所有 `file:line` 锚点已于 2026-07-28 晚间对真实源码核对过，但你开工时可能已漂移。**每个补丁动手前，先用 Grep/Read 现场确认锚点上下文**；对不上就以真实源码为准调整，禁止盲改。
2. **改动范围白名单。** 本次允许触碰的文件**只有**：
   - `crates/agent-core/src/executor.rs`（新建）
   - `crates/agent-core/src/loop.rs`（仅 W1 列出的改动点）
   - `crates/agent-core/src/lib.rs`（仅加 1 行模块声明 + re-export）
   - `docs/optional-branches.md`（新建，纯文档）
   - 受 W1 编译牵连的**测试代码**（`RunReport { .. }` 构造处补字段等）
   任何其他 `crates/**` 生产代码出现 diff = 🔴 阻塞，立即停手上报。
3. **验收标准是真 Linux 三门**，不是 Windows 本机"看起来没问题"。Windows 无 Rust 工具链，sandbox 测试仅在 Linux 编译运行。验证流程见 §7。
4. 导入路径（`use` 语句）以 `loop.rs` 文件头部现有 use 段为准，**照抄同名类型的既有路径**，不要凭记忆写。

---

## 1. 背景与目标

**判定（已由全局盘点坐实）：当前代码够主干资格，可封板执行。**

- 主干真实成形：5 相位状态机（`loop.rs` `LoopPhase`@21、`run()`@1015、do_plan/do_act/do_observe/do_reflect 齐备）、planner（decompose/reflect + replan 硬顶）、sandbox（Linux landlock+seccomp+cgroups v2+孤儿回收）、dispatcher/scheduler、tools-builtin 5 工具、memory 持久化、service 6 端点 + SSE 真实桥接。
- 核心纪律：**封板后主干逻辑体不再改动，一切增强走枝干**（新 crate / 已有 seam / flag），防止"一直改核心骨架，最后没有形状"。
- 封板前允许**唯一一次** trunk 改动（W1）：抽 `SubAgentExecutor` seam ＋ A4 子代理产出合并。这一刀开完，多进程、skills、modes、resilience 全部可以纯枝干生长。

工作包总览：

| 编号 | 内容 | 性质 | 依赖 |
|---|---|---|---|
| W1 | `SubAgentExecutor` seam 抽取 ＋ A4 `output_text` 合并 | 唯一 trunk 改动 | 无 |
| W2 | Noop/默认关能力显式降级声明 | 纯文档 | 无 |
| W3 | 封板仪式（VM 三门 → 守门员只读审计 → 打 tag） | 流程 | W1、W2 |
| B1–B7 | 枝干 backlog | 封板后增量 | W3 |

---

## 2. 冻结清单（v1.3-trunk-frozen 生效后）

### 2.1 主干（冻结，逻辑体字节级不变）

| Crate | 角色 |
|---|---|
| `agent-core` | 5 相位主循环 + scheduler + context（W1 完成后冻结） |
| `planner` | decompose / reflect / replan 硬顶 |
| `sandbox` | landlock + seccomp + cgroups v2 + 超时/孤儿回收 |
| `agent-types` | 核心类型契约 |
| `tool-runtime` + `tools-builtin` | 工具运行时 + 5 内建工具 |
| `llm-gateway` + `llm-openai` + `llm-local` + `llm-cn` | LLM 网关与三家客户端 |
| `memory` | 会话持久化 |
| `service` + `api` | 封装 seam（6 端点 + SSE），**接口冻结**，handler 内部允许修 bug |

### 2.2 枝干（可自由生长，不触发主干重评审）

- 已存在但降级为可选：`retriever`、`lsp-bridge`（当前 Noop）、`telemetry`（无 sink）、`code-index`、fallback 链
- 未来新建：`executor-process`（多进程）、`skills`、modes 层、resilience 适配层、TS 前端

### 2.3 冻结规则

1. 主干 crate 的**函数体不加逻辑**。允许：修真实崩溃 bug（须守门员确认是 bug 不是增强）、加 `#[cfg(test)]` 测试、加注释。
2. 枝干只能通过三类接缝接入主干：① `SubAgentExecutor` trait（W1 产出）；② 既有 setter（`set_retriever`@285、`set_lsp_bridge`@280、`set_cost_meter`@306、`set_event_sender`@290）；③ `service` HTTP/SSE 接口。
3. 枝干 PR 验收命令（红线）：`git diff v1.3-trunk-frozen -- crates/agent-core crates/planner crates/sandbox crates/agent-types crates/tool-runtime crates/tools-builtin crates/llm-gateway crates/llm-openai crates/llm-local crates/llm-cn crates/memory` 输出必须为空（`service`/`api` 单看接口签名）。

---

## 3. W1：`SubAgentExecutor` seam ＋ A4 产出合并（唯一 trunk 改动）

### 3.0 现状（已核对，2026-07-28）

- `loop.rs:227` 句柄类型硬编码：`sub_agent_handles: Vec<(String, tokio::task::JoinHandle<RunReport>)>`
- `loop.rs:347` 直接 `tokio::spawn`；`loop.rs:378-399` `collect_sub_agent_results` 直接轮询 `JoinHandle::is_finished()`
- `RunReport`（`loop.rs:159-165`）只有 `steps/ok/summary/usage`，**子代理自由文本产出被丢弃**（A4 缺口）：`do_observe`@849-852、`do_reflect`@889-892 的 `node.result.output` 只写 `"sub-agent {id}: ok={}, steps={}"` 格式串
- `run()` 内 `RunReport` 构造共 4 处：`loop.rs:1039`（budget_exhausted）、`:1062`（step error）、`:1098`（Done）、`:1112`（Error phase）
- `lib.rs` 模块声明：`pub mod context; pub mod r#loop; pub mod scheduler;`

### 3.1 新建 `crates/agent-core/src/executor.rs`

```rust
//! W1: SubAgentExecutor seam — 封板前唯一 trunk 改动。
//! 把"子代理如何被执行"（今天是进程内 tokio task，将来是独立 OS 进程）
//! 收进一条稳定 trait 后面，让未来的执行后端成为纯枝干。

use std::sync::Arc;

use async_trait::async_trait;

use crate::r#loop::{AgentLoop, Goal, RunReport};
// 以下类型的 use 路径照抄 loop.rs 头部现有 use 段（铁律 4）：
// Budget、Planner（trait）、ToolDispatcher、tool_runtime::ToolContext、LlmProvider

/// 启动一个子代理所需的全部材料（按值捕获，可跨进程序列化的部分未来再拆）。
pub struct SubAgentSpec {
    pub task_id: String,
    pub goal_text: String,
    pub budget: Budget,
    pub depth: u32,
    pub provider: Arc<dyn LlmProvider>,
    pub planner: Arc<dyn Planner>,
    pub dispatcher: Arc<ToolDispatcher>,
    pub ctx: tool_runtime::ToolContext,
}

/// 子代理运行句柄：非阻塞探询 + 异步汇合。
#[async_trait]
pub trait SubAgentHandle: Send {
    fn is_finished(&self) -> bool;
    async fn join(self: Box<Self>) -> Result<RunReport, String>;
}

/// 执行后端 seam。今天只有 InProcessExecutor；
/// 未来 ProcessExecutor（多进程 + IPC + 每进程独立 sandbox）在枝干 crate 里实现本 trait。
pub trait SubAgentExecutor: Send + Sync {
    fn spawn(&self, spec: SubAgentSpec) -> Box<dyn SubAgentHandle>;
}

/// 现状行为的 1:1 搬移：tokio::spawn 进程内执行。
pub struct InProcessExecutor;

struct InProcessHandle(tokio::task::JoinHandle<RunReport>);

#[async_trait]
impl SubAgentHandle for InProcessHandle {
    fn is_finished(&self) -> bool {
        self.0.is_finished()
    }
    async fn join(self: Box<Self>) -> Result<RunReport, String> {
        self.0.await.map_err(|e| format!("sub-agent join error: {e}"))
    }
}

impl SubAgentExecutor for InProcessExecutor {
    fn spawn(&self, spec: SubAgentSpec) -> Box<dyn SubAgentHandle> {
        let SubAgentSpec {
            task_id,
            goal_text,
            budget,
            depth,
            provider,
            planner,
            dispatcher,
            ctx,
        } = spec;
        let sub_goal = Goal::with_budget(goal_text.clone(), budget.clone());
        let handle = tokio::spawn(async move {
            // == 原 loop.rs:348-370 闭包体原样搬移（含 task_id 注入 summary 的 Ok/Err 两臂）==
            let mut sub_agent = AgentLoop::new(provider, planner, dispatcher, ctx, sub_goal);
            sub_agent.set_depth(depth); // 见 3.2-e：depth 字段私有，需加 pub(crate)/setter
            match sub_agent.run(Goal::with_budget(goal_text, budget)).await {
                Ok(report) => RunReport {
                    ok: report.ok,
                    steps: report.steps,
                    summary: serde_json::json!({
                        "task_id": task_id,
                        "goal": report.summary.get("goal"),
                        "steps": report.steps,
                        "ok": report.ok,
                    }),
                    usage: report.usage,
                    output_text: report.output_text, // A4：产出不再丢弃
                },
                Err(e) => RunReport {
                    steps: 0,
                    ok: false,
                    summary: serde_json::json!({"error": format!("{e:#}"), "task_id": task_id}),
                    usage: None,
                    output_text: String::new(),
                },
            }
        });
        Box::new(InProcessHandle(handle))
    }
}
```

### 3.2 修改 `crates/agent-core/src/loop.rs`

**(a) `RunReport` 加字段（@159-165）**

```rust
pub struct RunReport {
    pub steps: u64,
    pub ok: bool,
    pub summary: Value,
    pub usage: Option<llm_gateway::CostEntry>,
    /// A4: 本次 run 的最终自由文本产出（最后一条 assistant 消息），
    /// 供父代理做内容级合并；空串表示无产出。
    pub output_text: String,
}
```

**(b) 句柄字段换类型（@227）＋ executor 字段（struct 尾部 @234 附近）＋ 初始化（@267/@270 附近）**

```rust
// @227 旧：
sub_agent_handles: Vec<(String, tokio::task::JoinHandle<RunReport>)>,
// 新：
sub_agent_handles: Vec<(String, Box<dyn crate::executor::SubAgentHandle>)>,
// struct 尾部新增：
/// W1: 子代理执行后端（默认进程内；多进程后端由枝干 crate 注入）。
sub_agent_executor: Arc<dyn crate::executor::SubAgentExecutor>,
```

`new()` 初始化列表新增：`sub_agent_executor: Arc::new(crate::executor::InProcessExecutor),`
并加 setter（与 `set_retriever` 同排风格）：

```rust
/// W1: 注入子代理执行后端（默认 InProcessExecutor）。
pub fn set_sub_agent_executor(&mut self, ex: Arc<dyn crate::executor::SubAgentExecutor>) {
    self.sub_agent_executor = ex;
}
```

**(c) `spawn_sub_agent` 瘦身（@316-374）**：保留 @326-333 递归守卫与 @336 depth 钳制**一字不动**，把 @338-371 的克隆+闭包替换为：

```rust
let spec = crate::executor::SubAgentSpec {
    task_id: task_id.clone(),
    goal_text: format!("[sub:{}] {}", task_id, task_desc),
    budget,
    depth,
    provider: self.provider.clone(),
    planner: self.planner.clone(),
    dispatcher: self.scheduler.dispatcher().clone(),
    ctx: self.scheduler.tool_context().clone(),
};
let handle = self.sub_agent_executor.spawn(spec);
self.sub_agent_handles.push((task_id, handle));
```

**(d) `collect_sub_agent_results` 改走 trait（@378-399）**：循环体改为

```rust
if handle.is_finished() {
    match handle.join().await {
        Ok(report) => results.push((task_id, report)),
        Err(e) => tracing::error!("Sub-agent failed: {e}"),
    }
} else {
    remaining.push((task_id, handle));
}
```

**(e) depth 可注入**：`depth` 字段是私有的，executor.rs 在 crate 内，最简做法是给 `AgentLoop` 加 `pub(crate) fn set_depth(&mut self, d: u32)`（或直接把字段改 `pub(crate)`，二选一，执行窗口按编译结果定）。

**(f) A4 合并落点（@847-854 与 @887-894 两处对称）**：`node.result.output` 改为优先用产出文本：

```rust
output: if report.output_text.is_empty() {
    format!("sub-agent {}: ok={}, steps={}", task_id, report.ok, report.steps)
} else {
    report.output_text.clone()
},
```

**(g) `run()` 4 处 `RunReport` 构造补字段（@1039/@1062/@1098/@1112）**：各加一行 `output_text: self.last_assistant_text(),`，并新增私有辅助函数（放 `emit` 附近）：

```rust
/// A4: 从对话历史倒序找最后一条 assistant 文本作为本次 run 的产出。
fn last_assistant_text(&self) -> String {
    use agent_types::{MessageContent, Role};
    for turn in self.ctx_mgr.state().history.iter().rev() {
        for msg in turn.messages.iter().rev() {
            if matches!(msg.role, Role::Assistant) {
                if let MessageContent::Text(t) = &msg.content {
                    return t.clone();
                }
            }
        }
    }
    String::new()
}
```

> 注意：`Message` 字段名（`role`/`content`）与 `MessageContent` 变体名需现场对照 `agent-types/src/lib.rs`，此处按 `context.rs:57-75` 的用法推定。

**(h) 编译牵连**：全仓 grep `RunReport {`，所有测试/调用处构造补 `output_text` 字段。已知至少 `loop.rs` 测试区（@1132 起，含 @2018 附近子代理测试）会命中。**只允许补字段，不许改断言语义。**

### 3.3 修改 `crates/agent-core/src/lib.rs`

```rust
pub mod context;
pub mod executor;   // W1 新增
pub mod r#loop;
pub mod scheduler;

pub use context::ContextManager;
pub use executor::{InProcessExecutor, SubAgentExecutor, SubAgentHandle, SubAgentSpec};
pub use r#loop::{Agent, AgentLoop, Event, Goal, LoopPhase, RunReport, StepOutcome};
pub use scheduler::Scheduler;
```

### 3.4 W1 新增测试（必须有会失败的断言）

在 `loop.rs` 测试区或 executor.rs 内新增：

1. `test_w1_inprocess_executor_preserves_behavior`：用 mock provider/planner 跑 `spawn_sub_agent` + `collect_sub_agent_results`，断言 task_id 回传、summary 含 `task_id` 键——证明 seam 抽取是行为等价搬移。
2. `test_w1_custom_executor_injectable`：写一个 `FakeExecutor`（spawn 立即返回 `is_finished()==true`、`join()` 给定制 `output_text` 的 handle），`set_sub_agent_executor` 注入后走 observe 路径，断言 `node.result.output` == 定制文本——同时验证 seam 可注入 **和** A4 合并生效。
3. `test_w1_depth_guard_intact`：depth≥1 时 `spawn_sub_agent` 仍拒绝（守卫没被搬丢）。

### 3.5 W1 验收口径

- 真 Linux 三门全绿（§7），测试数 ≥ 146 + 新增 3。
- `service`/`session.rs` **零改动**仍编译通过（AgentLoop::new 签名未变）。
- 守门员抽查：@326 守卫、@336 钳制原样；`tokio::spawn` 在 `loop.rs` 中不再出现（已收进 executor.rs）。

---

## 4. W2：可选能力显式降级声明（纯文档，新建 `docs/optional-branches.md`）

把以下 4+1 项白纸黑字declared为"**可选枝干，默认关闭/未实装**"，禁止在对外叙事中算作主干能力：

| 项 | 现状（source of truth） | 枝干化方向 |
|---|---|---|
| `lsp-bridge` | 生产默认 `NoopLspBridge`（`service/main.rs:166`、`loop.rs:255`） | 真 LSP 客户端实装后经 `set_lsp_bridge` 注入 |
| `retriever` | 默认 `None`（`loop.rs:256`），仅 flag 开启时注入（`service/main.rs:160`） | 保持 opt-in |
| fallback 链 | 存在但非默认执行路径 | 作为 provider 装饰器枝干接入 |
| `telemetry` | 无任何 sink，纯空壳 crate | R9 时接 OTLP/file sink，从事件流外挂 |
| sandbox 非 Linux | Windows/macOS 下为 Noop 直通，**隔离只在 Linux 生效** | 文档醒目声明，防误用 |

文档还须包含：非 Linux sandbox Noop 的安全警告、每项的启用条件与验收标准。

---

## 5. W3：封板仪式（顺序执行，缺一步不算封板）

1. **W1 合入** 后，本地无未提交改动。
2. **VM 三门全绿**：按 §7 流程跑 `fmt + clippy -D warnings + test --all`，判读 `rc=0` 且 `0 failed`。
3. **守门员只读审计**：另开窗口用 `code-audit-gatekeeper` 方法论，只读主干 crate，确认：无 unimplemented!/todo! 在生产路径、W1 改动范围未越白名单、测试非空壳。产出简版审计记录附在本文档末尾或独立文件。
4. **打 tag**：
   ```bash
   git add -A && git commit -m "W1: extract SubAgentExecutor seam + A4 output merge; W2: declare optional branches"
   git tag -a v1.3-trunk-frozen -m "Trunk frozen: 5-phase loop + planner + sandbox + tools + llm + memory + service seam. All enhancements grow as branches via SubAgentExecutor / setters / HTTP seam."
   ```
5. **此后所有 PR 走 §2.3 红线检查。**

---

## 6. 枝干 backlog（封板后按需领取，每项独立 handoff、独立验收）

| 编号 | 枝干 | 挂载 seam | 说明 |
|---|---|---|---|
| B1 | `executor-process` 多进程执行器 | `SubAgentExecutor` trait | 新 crate；子代理 = 独立 OS 进程 + IPC(stdio/socket) + 每进程独立 sandbox 策略；`SubAgentSpec` 需先解决 provider/dispatcher 跨进程重建（传配置不传对象） |
| B2 | `skills` crate（PDCA 技能链） | 新 crate + `service` 编排层 | 见 `docs/skills-pdca-design.md`；Facilitator 在 service 层编排多次 AgentLoop 调用，不进 loop.rs |
| B3 | 四模式（对话/计划/执行/探索） | `service` 层会话状态机 | 模式=对 HTTP 请求的编排策略（是否冻结 TaskGraph、是否自动审批），不改 LoopPhase |
| B4 | Resilience R1/R3/R7/R9 | R1=llm 客户端 wrapper crate；R3=service 层校验门；R7=`set_cost_meter` 外置熔断；R9=telemetry sink | 见 `docs/resilience-fmea.md`；全部枝干化，不进 do_* 函数体 |
| B5 | telemetry 真 sink | 事件流外挂 | OTLP 或 file sink，订阅 `set_event_sender` 出口 |
| B6 | retriever / lsp-bridge 实装 | 既有 setter | 实装后仍 opt-in |
| B7 | TS 前端 | `service` HTTP/SSE 接口 | 契约=`api` crate DTO，接口已冻结 |

> 领取规则：一次一个枝干、一份 handoff、一轮 VM 验收。禁止打包多个枝干一次交付。

---

## 7. 验证命令（真 Linux VM 标准流程）

VM：`ssh wutao@192.168.220.131`（Ubuntu 24.04，凭据见项目 MEMORY.md，已授权直用）。

```bash
# 1) 本机打包（Git Bash，排除 target/.git/.workbuddy；--force-local 防冒号误判）
tar --force-local -czf /tmp/codex_w1.tgz -C /c/Users/87465/Desktop \
    --exclude='codex-rust-v1.0-final/target' \
    --exclude='codex-rust-v1.0-final/.git' \
    --exclude='codex-rust-v1.0-final/.workbuddy' \
    codex-rust-v1.0-final

# 2) 上传并解压到 ~/codex（保留 VM 端 target 增量）
scp /tmp/codex_w1.tgz wutao@192.168.220.131:~/
ssh wutao@192.168.220.131 'mkdir -p ~/codex && tar -xzf ~/codex_w1.tgz -C ~/codex --strip-components=1'

# 3) ⚠️ 必做：刷新 mtime，否则 cargo 复用旧缓存产生"假通过/假报错"
ssh wutao@192.168.220.131 'find ~/codex/crates -name "*.rs" -exec touch {} +'

# 4) 前台阻塞跑三门（不要 nohup 后台，避免日志竞争）
ssh wutao@192.168.220.131 'source ~/.cargo/env && cd ~/codex && \
  cargo fmt --all --check; echo FMT_RC=$?; \
  cargo clippy --workspace --all-targets -- -D warnings; echo CLIPPY_RC=$?; \
  cargo test --all; echo TEST_RC=$?' 2>&1 | tee gate-w1.log

# 5) 判读：FMT_RC=0 且 CLIPPY_RC=0 且 TEST_RC=0，且汇总行无 failed
grep -E '^test result|_RC=' gate-w1.log
```

---

## 8. 验收红线（守门员用）

- 🔴 阻塞：白名单外的 `crates/**` 生产代码 diff；@326 递归守卫/@336 depth 钳制被改动或搬丢；新增测试无断言或断言恒真；三门任一非 0。
- 🟡 需评审：`RunReport` 字段语义变化超出"加 output_text"；service 层出现"顺手优化"。
- 🔵 参考：注释/文档措辞、测试命名风格。
- 通过条件：🔴=0，🟡 逐条评审放行，VM 三门全绿，测试数 ≥ 149。

---

## 附：本文档锚点核对记录（2026-07-28 23:5x）

`RunReport`@loop.rs:159-165 ✅ ｜ 句柄字段@227 ✅ ｜ 守卫@326-333、钳制@336 ✅ ｜ `tokio::spawn`@347、闭包体@348-370 ✅ ｜ `collect`@378-399 ✅ ｜ A4 丢弃点@847-854、@887-894 ✅ ｜ `run()`@1015、4 处构造@1039/1062/1098/1112 ✅ ｜ `lib.rs` 3 模块 ✅ ｜ `session.rs` AgentLoop::new@129、setter 注入@139/142 ✅ ｜ workspace 17 成员（root Cargo.toml:3-21）✅
