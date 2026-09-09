# Codex-Rust v1.0 → v1.1 修复规划

> 基于 2026-07-28 审计报告 (audit-report.md)  
> 目标：v1.1 发布前消除 🟡 中危项，顺手收掉常用 🔵 低危项

---

## 总览

| 轮次 | 修复项 | 涉及文件数 | 预计时间 | 风险 |
|------|--------|-----------|---------|------|
| Round 1 | Fix-1 路径穿越 | 3 | 15min | 低 |
| Round 2 | Fix-2 Glob/Grep 走沙箱 | 2 | 25min | 中 |
| Round 3 | Fix-3 Tantivy unwrap | 1 | 5min | 低 |
| Round 4 | Fix-4 会话取消清理 | 1 | 20min | 中 |
| Round 5 | Fix-5 审批状态隔离 | 3 | 40min | 高 |
| Round 6 | Fix-L 低危收尾 | 2 | 10min | 低 |

**每轮结束后执行**：`cargo test --all` 验证 133 个测试全绿，然后再进下一轮。

---

## Round 1：路径穿越防护 (M1)

### 问题回顾
`ReadTool`、`EditTool`、`GrepTool` 使用 `ctx.cwd.join(path_str)`，当 LLM 生成绝对路径或 `../` 穿越时，`Path::join` 不防护。

### 修复策略
在每个工具的 `execute()` 方法中，`path_str` 获取之后、`join` 之前插入消毒逻辑。**不改变现有接口**。

### 改什么

**文件 1**: `crates/tools-builtin/src/read.rs`

在 line 40（`let path_str = ...`）之后插入：

```rust
// 路径穿越防护
if path_str.contains("..") || std::path::Path::new(path_str).is_absolute() {
    return Err(anyhow!("path traversal denied: {}", path_str));
}
```

**文件 2**: `crates/tools-builtin/src/edit.rs`

同 read.rs，在 line 44 之后插入相同代码。

**文件 3**: `crates/tools-builtin/src/grep.rs`

在 line 48（`let pattern = ...`）之后，`search_path` 构建处。GrepTool 的 path 参数是可选的，需要对用户传入的 path 做同样检查：

```rust
// 在 line 50-54 的 search_path 构建处：
let search_path = args
    .get("path")
    .and_then(|v| v.as_str())
    .map(|s| {
        if s.contains("..") || std::path::Path::new(s).is_absolute() {
            // 别 panic，返回 safe 的 cwd 但记日志
            tracing::warn!("path traversal attempt denied: {}", s);
            ctx.cwd.display().to_string()
        } else {
            s.to_string()
        }
    })
    .unwrap_or_else(|| ctx.cwd.display().to_string());
```

### 验证方式
- 新增测试：`read` 工具传入 `../etc/passwd` → 返回错误
- 新增测试：`edit` 工具传入 `/absolute/path` → 返回错误
- 新增测试：`grep` 工具指定恶意 path → 回退到 cwd
- `cargo test --all` —— 全部通过（133 + 新增）

---

## Round 2：Glob/Grep 走沙箱 (M7/L7)

### 问题回顾
`GlobTool` 和 `GrepTool` 直接调用 `tokio::process::Command`，绕过 Sandbox。Linux 上的 Landlock/Seccomp 对此无效。

### 修复策略
给 GlobTool 和 GrepTool 注入 `Arc<dyn Sandbox>`，和 BashTool 保持一致的模式。同时修改 `main.rs` 中的构建代码。

### 改什么

**文件 1**: `crates/tools-builtin/src/glob.rs` — 重构

```
现在:
pub struct GlobTool;

改后:
pub struct GlobTool {
    sandbox: Arc<dyn Sandbox>,
}

impl GlobTool {
    pub fn new() -> Self {
        Self { sandbox: Arc::from(sandbox::create_sandbox(SandboxConfig::default())) }
    }
    pub fn with_sandbox(sandbox: Arc<dyn Sandbox>) -> Self {
        Self { sandbox }
    }
}
```

execute 方法中，把 `tokio::process::Command::new("find")` 换成 `self.sandbox.spawn("find", &[...], &ctx.cwd, &[], Duration::from_secs(30))`。

**文件 2**: `crates/tools-builtin/src/grep.rs` — 同 glob 模式重构

**文件 3**: `crates/tools-builtin/src/lib.rs` — 确认 `pub use` 导出没变

**文件 4**: `crates/service/src/main.rs` — 构建代码不用改（`.new()` 内部创建 sandbox）

### 注意点
- `find -path` 和 `rg --glob` 的参数拼接方式不变，只是执行路径走 `sandbox.spawn()`
- 沙箱 timeout 默认 30s，和之前一致
- `rg` 探测 (`has_rg()`) 需要在 sandbox 外执行一次；实际搜索通过 sandbox

### 验证方式
- 现有 glob/grep 测试应全部通过（因为构造时自动创建 sandbox）
- Linux 上：验证 Landlock 能限制 glob/grep 的文件访问范围
- macOS/Windows：NoopSandbox 跑通即可（测试已有覆盖）

---

## Round 3：Tantivy unwrap → 错误 (M3)

### 问题回顾
`crates/retriever/src/lib.rs:130` 直接 `.unwrap()` 可能导致 panic。

### 修复策略
`unwrap()` → `ok_or_else(anyhow!("..."))?`

### 改什么

**文件**: `crates/retriever/src/lib.rs`

line 130:
```rust
// 改前
let query_parser = QueryParser::for_index(&self.index.as_ref().unwrap(), vec![self.content_field]);

// 改后
let index = self.index.as_ref()
    .ok_or_else(|| anyhow::anyhow!("bm25 index not built yet"))?;
let query_parser = QueryParser::for_index(index, vec![self.content_field]);
```

line 138-139 (同文件中，检索结果解析):
```rust
// 改前
let doc: tantivy::TantivyDocument = searcher.doc(doc_addr).unwrap();
let id = doc.get_first(self.id_field).unwrap().as_u64().unwrap() as usize;

// 改后
let doc: tantivy::TantivyDocument = searcher.doc(doc_addr)?;
let id = doc.get_first(self.id_field)
    .and_then(|f| f.as_u64())
    .ok_or_else(|| anyhow::anyhow!("document missing id field"))? as usize;
```

### 验证方式
- 现有 retriever 测试（4 个）全部通过
- `cargo test -p retriever`

---

## Round 4：会话取消清理后台任务 (M4)

### 问题回顾
`cancel_session()` 只是 `sessions.remove(id)`，已 spawn 的 tokio task 继续跑。

### 修复策略
利用 `tokio::sync::oneshot` 通道发送取消信号。

### 改什么

**文件**: `crates/service/src/session.rs`

Step 1：Session 结构体增加取消通道：

```rust
use tokio::sync::oneshot;

pub struct Session {
    // ... 现有字段 ...
    /// v1.1: Cancellation signal for the running agent task.
    pub cancel_tx: Option<oneshot::Sender<()>>,
}
```

Step 2：`send_message()` 中，spawn 前创建 oneshot channel，传给 task：

```rust
// 在 tokio::spawn 之前：
let (cancel_tx, mut cancel_rx) = oneshot::channel::<()>();

// 存到 session
{
    let mut s = session_clone.lock().await;
    s.cancel_tx = Some(cancel_tx);
}

// task 内部的事件转发循环改为 select!：
tokio::select! {
    Some(evt) = internal_rx.recv() => { /* 现有逻辑 */ }
    _ = &mut cancel_rx => {
        // 收到取消信号，提前退出
        let _ = tx.send(AgentEvent::Error { message: "session cancelled".into() });
        break;
    }
}
```

Step 3：`cancel_session()` 发送取消信号：

```rust
pub async fn cancel_session(&self, id: &str) -> anyhow::Result<()> {
    let session = self.get_session(id).await
        .ok_or_else(|| anyhow::anyhow!("session not found: {}", id))?;
    
    let mut s = session.lock().await;
    if let Some(tx) = s.cancel_tx.take() {
        let _ = tx.send(());  // 发信号给 running task
    }
    
    // 不立即 remove，让 task 自然结束
    // 如果 task 已在运行但未到 select! 点，取消信号在下次循环生效
    Ok(())
}
```

### 验证方式
- 新增测试：创建 session → 发送消息 → 立即 cancel → 验证 session 最终标记为 cancelled
- `cargo test -p service`

---

## Round 5：审批状态按 Session 隔离 (M2)

### 问题回顾
`ToolDispatcher` 全局共享一个 `ApprovalState`，多 session 并发时互相干扰。

### 修复策略
将审批状态从 `ToolDispatcher` 移到 `Scheduler` 层（每个 AgentLoop 一个 Scheduler）。`ToolDispatcher` 退化为纯工具路由，不再管理审批。

### 改什么

**文件 1**: `crates/tool-runtime/src/dispatcher.rs`

- 删除 `approval: Mutex<ApprovalState>` 字段
- 删除 `set_approval_pending`、`resolve_approval`、`approval_state`、`reset_approval` 方法
- `ToolDispatcher` 回归纯工具路由

**文件 2**: `crates/agent-core/src/scheduler.rs`

- Scheduler 结构体增加 `approval: Mutex<ApprovalState>`
- 搬入原来的审批管理方法
- `check_approval()` 逻辑不变，但从 self 读取

**文件 3**: `crates/service/src/session.rs`

- `submit_approval()` 改为通过 session → agent → scheduler 路径设置审批结果
- 或直接在 session 中存 `approval_tx: oneshot::Sender` 直接通知等待方

### 简化方案（推荐）

最轻量的改法：不在 Dispatcher/Scheduler 间搬状态，而是用 `Arc<Mutex<HashMap<String, ApprovalState>>>` 替代单个 `Mutex<ApprovalState>`，key 用 session_id：

```rust
// dispatcher.rs
approval: Mutex<HashMap<String, ApprovalState>>,

// 所有审批方法加 session_id 参数
pub async fn set_approval_pending(&self, session_id: &str, approval_id: String, action: String) { ... }
pub async fn resolve_approval(&self, session_id: &str, decision: &str) -> Result<ApprovalState> { ... }
```

改动量：dispatcher.rs 改 6 个方法签名 + HashMap 替代，约 20 行改动。Session 层改调用传参。

### 验证方式
- 现有审批相关测试（2 个）全部通过
- 新增测试：两个 session 同时 set approval → 互不干扰
- `cargo test -p tool-runtime`

---

## Round 6：低危收尾 (L2-L6)

### L4: session.rs:215 unwrap → error
**文件**: `crates/service/src/session.rs:215`

```rust
// 改前
let tx = { let s = session.lock().await; s.event_tx.clone().unwrap() };

// 改后
let tx = {
    let s = session.lock().await;
    s.event_tx.clone()
        .ok_or_else(|| anyhow::anyhow!("session {} event stream closed", id))?
};
```

### L5: 端口可配置
**文件**: `crates/service/src/main.rs:166`

```rust
// 改前
let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

// 改后
let port: u16 = std::env::var("PORT")
    .ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or(3000);
let addr = SocketAddr::from(([0, 0, 0, 0], port));
info!("Codex Agent Service starting on {addr}");
```

### L6: CORS 收紧
**文件**: `crates/service/src/main.rs:149-152`

```rust
// 改前
let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any);

// 改后
let cors = CorsLayer::new()
    .allow_origin(
        std::env::var("CORS_ORIGIN")
            .ok()
            .map(|o| o.parse::<axum::http::HeaderValue>().ok())
            .flatten()
            .map(|v| vec![v])
            .unwrap_or_else(|| vec!["http://localhost:5173".parse().unwrap()])
    )
    .allow_methods(Any)
    .allow_headers(Any);
```

### L2: CostMeter provider 校准 — 跳过
这个改动需要深入 cost_meter 和 fallback 的交互，且影响很小（只是统计可能偏 1 行），建议延后到 v1.2。

### L1/L3/L7 — 跳过
代码风格问题 / 已在 Round 2 修复 / 影响微小。

---

## 执行检查清单

```
Round 1 ☐ 路径穿越：read.rs + edit.rs + grep.rs
          ☐ cargo test --all 全绿
          
Round 2 ☐ Glob/Grep 走沙箱：glob.rs + grep.rs
          ☐ cargo test --all 全绿

Round 3 ☐ Tantivy unwrap：retriever/src/lib.rs
          ☐ cargo test -p retriever 全绿

Round 4 ☐ 会话取消清理：session.rs
          ☐ cargo test -p service 全绿

Round 5 ☐ 审批隔离：dispatcher.rs + scheduler.rs + session.rs
          ☐ cargo test -p tool-runtime 全绿
          ☐ cargo test --all 全绿

Round 6 ☐ 低危收尾：session.rs + main.rs
          ☐ cargo test --all 全绿
```

---

## 发布建议

| 版本 | 内容 | 闸门条件 |
|------|------|----------|
| v1.1 | Round 1-3 + Round 6 | 所有现有测试通过 + 新增安全测试 |
| v1.2 | Round 4-5 | 并发测试 + 压力测试通过 |

**v1.1 可以快速发布**（Round 1-3 改动小、风险低），先把安全面补上。v1.2 改架构，多花时间测试。
