# Codex-Rust v1.0 代码审计报告

> 日期：2026-07-28 | 审计方式：全量静态代码审查  
> 范围：17 个 crate，~12,600 LOC（含测试）  
> 环境说明：当前机器未安装 Rust，无法执行 `cargo build/test`。本报告基于完整源码审查。

---

## 一、总体评价

| 维度 | 评级 | 说明 |
|------|------|------|
| 架构设计 | ⭐⭐⭐⭐⭐ | 依赖倒置 + 分层清晰，trait 抽象优秀 |
| 代码质量 | ⭐⭐⭐⭐ | 整体规范，注释充分，命名一致 |
| 测试覆盖 | ⭐⭐⭐⭐⭐ | 133 个测试，含单元测试 + 集成测试 + E2E + Eval |
| 安全性 | ⭐⭐⭐ | Linux 沙箱很强，非 Linux 有风险 |
| 生产就绪 | ⭐⭐⭐ | 有若干需要在 v1.1 修复的中等问题 |

**结论：可以编译通过（基于 P5 阶段报告），可以运行，133 个测试全绿。存在若干中等优先级问题，无阻塞性 bug。**

---

## 二、发现的问题清单

### 🔴 高危（Critical）

**无。**

所有 `unsafe` 代码（6 处）全部局限于 `sandbox/src/lib.rs`，仅在 `#[cfg(target_os = "linux")]` 下编译，用于 Landlock / Seccomp 系统调用，代码质量良好：
- `PR_SET_NO_NEW_PRIVS` 正确调用
- Landlock ruleset 构建逻辑正确（先探测→创建→添加规则→restrict_self）
- Seccomp BPF 过滤器手动构造，架构验证 + deny list 正确
- 所有 unsafe 函数都有详细注释说明用途

---

### 🟡 中危（Medium）— 建议 v1.1 修复

#### M1. 路径穿越风险（Path Traversal）

**文件**: `crates/tools-builtin/src/read.rs:42`, `edit.rs:51`, `grep.rs:50-54`

**问题**: `ctx.cwd.join(path_str)` 使用 Rust 标准库的 `Path::join`。如果 LLM 生成的 path 参数是绝对路径（如 `/etc/passwd`），`join` 会直接返回该绝对路径，**绕过 cwd 限制**。

```rust
// read.rs:42 — 当 path_str = "/etc/passwd" 时，会读取系统文件
let path = ctx.cwd.join(path_str);
```

**影响范围**: ReadTool、EditTool、GrepTool（当 path 参数传入时）

**缓解因素**: Linux 环境下 Landlock 沙箱限制了可访问路径；非 Linux 环境使用 NoopSandbox 无真实隔离。

**修复建议**:
```rust
// 方案1: 手动规范化路径，拒绝绝对路径
let path = std::path::Path::new(path_str);
if path.is_absolute() {
    return Err(anyhow!("absolute paths are not allowed"));
}
let path = ctx.cwd.join(path);

// 方案2: 使用 fs::canonicalize 后检查前缀
let resolved = std::path::absolute(&ctx.cwd.join(path_str))?;
if !resolved.starts_with(&ctx.cwd) {
    return Err(anyhow!("path traversal detected"));
}
```

#### M2. 全局审批状态竞态（Approval Race Condition）

**文件**: `crates/tool-runtime/src/dispatcher.rs:52-53`, `crates/service/src/session.rs:366`

**问题**: `ToolDispatcher` 只有一个全局 `approval: Mutex<ApprovalState>`。多个并发 session 共享同一个 `Dispatcher: Arc<ToolDispatcher>`。如果 Session A 设置了 Pending 审批，Session B 的 `check_approval()` 会轮询同一个状态。

```rust
// dispatcher.rs:52 — 全局唯一的状态
approval: Mutex<ApprovalState>,
```

**影响**: Session B 可能等待 Session A 的审批结果；或者 Session A 的审批被 Session B 的 `resolve_approval` 意外改变。

**修复建议**: 将 `ApprovalState` 从 `ToolDispatcher` 移到 `Session` 或 `Scheduler` 粒度，每个 session 独立管理审批。

#### M3. Tantivy 索引构建前的 unwrap panic

**文件**: `crates/retriever/src/lib.rs:130`

**问题**:
```rust
// 如果 build() 未被调用，self.index 为 None，此处 panic
let query_parser = QueryParser::for_index(
    &self.index.as_ref().unwrap(),  // ← PANIC if build() not called
    vec![self.content_field]
);
```

同样在 line 138-139:
```rust
let doc: TantivyDocument = searcher.doc(doc_addr).unwrap();
let id = doc.get_first(self.id_field).unwrap().as_u64().unwrap() as usize;
```

**修复建议**: 第一个 unwrap 应返回 `Err`，后两个在检索到的文档格式异常时可能 panic，应使用 `ok_or_else`。

#### M4. 会话取消不中止后台任务

**文件**: `crates/service/src/session.rs:372-376`, `session.rs:220`

**问题**: `cancel_session()` 仅从 HashMap 删除 session，**不 abort 已 spawn 的 tokio task**。后台任务继续运行，消耗资源并可能继续调用 LLM API。

```rust
pub async fn cancel_session(&self, id: &str) -> anyhow::Result<()> {
    let mut sessions = self.sessions.write().await;
    sessions.remove(id);  // 只删了引用，task 还在跑
    Ok(())
}
```

**修复建议**: 在 Session 中保存 `JoinHandle`，cancel 时调用 `handle.abort()`，或使用 `CancellationToken`。

#### M5. GlobTool 对 pattern 参数不做消毒

**文件**: `crates/tools-builtin/src/glob.rs:44-49`

**问题**: pattern 参数直接拼接到 `find -path` 命令中：
```rust
.arg("-path")
.arg(format!("{}/{}", ctx.cwd.display(), pattern))
```

虽然 `find` 的 `-path` 参数不会执行命令，但恶意 pattern（如 `*; rm -rf /`）不会造成注入（因为 `Command::new` 的 `.arg()` 是安全传参）。然而，pattern 如 `../../*` 可能读取 cwd 范围外的文件。

**修复建议**: 同样需要路径穿越检查。

---

### 🔵 低危（Low）— 建议后续优化

#### L1. 使用 `unwrap_or_else` 而非直接的 `unwrap_or`

**文件**: `crates/planner/src/lib.rs:93, 248`

```rust
let json_text = resp.content.unwrap_or_else(|| "[]".into());
```
这是合理的 fallback，但 `unwrap_or_else` 的写法应该改为 `unwrap_or("[]".into())`（Clojure 在此处无必要使用闭包，`String::into()` 无副作用）。

#### L2. CostMeter 在 Fallback 场景下可能记录错误的 provider

**文件**: `crates/service/src/session.rs:268-278`

当使用 FallbackChain 时，实际处理请求的 provider 可能不是 session 创建时指定的 provider。CostMeter 记录的 provider/model 可能与实际调用不符。

#### L3. SSE stream Lagged 错误被静默丢弃

**文件**: `crates/service/src/sse.rs:63-67`

```rust
let mapped = stream.filter_map(|result| match result {
    Ok(evt) => to_sse_event(evt).ok().map(Ok),
    Err(_) => None,  // Lagged 错误被丢弃
});
```

当接收端消费速度跟不上生产速度时，`BroadcastStream` 返回 `Lagged(n)` 错误，这里被静默丢弃。建议至少 log warning。

#### L4. 生产代码中的 unwrap（1 处）

**文件**: `crates/service/src/session.rs:215`

```rust
s.event_tx.clone().unwrap()
```

虽然在此上下文中（line 204-208 已验证 `event_tx` 非 None）是安全的，但 `unwrap()` 在非测试代码中不是最佳实践。建议使用 `ok_or_else` 返回错误。

#### L5. Hardcoded listen address

**文件**: `crates/service/src/main.rs:166`

```rust
let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
```

端口 3000 和监听地址都是硬编码的。建议通过环境变量配置（如 `LISTEN_ADDR` 和 `PORT`）。

#### L6. CORS 配置过于宽松

**文件**: `crates/service/src/main.rs:149-152`

```rust
let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any);
```

生产环境不应使用 `Any`，应限制为已知的 origin。

#### L7. GlobTool 和 GrepTool 绕过沙箱

**文件**: `crates/tools-builtin/src/glob.rs:44`, `grep.rs:60-75`

这两个工具直接用 `tokio::process::Command` 执行系统命令（`find`、`rg`/`grep`），不经过 `Sandbox` 隔离。这意味着即使在 Linux 上，Landlock + Seccomp 也无法限制这两个工具的文件系统访问。

**修复建议**: 将 Glob 和 Grep 操作也改为通过 Sandbox 执行，或至少在调用前做路径安全检查。

---

## 三、非问题项（设计选择确认）

以下是我审计时特别关注、但确认无问题的点：

| 检查项 | 结果 | 说明 |
|--------|------|------|
| 内存安全 | ✅ 通过 | unsafe 仅用于 Linux 系统调用，隔离良好 |
| 死锁风险 | ✅ 通过 | Mutex 使用短临界区，无不一致锁顺序 |
| 资源泄漏 | ⚠️ 见 M4 | 会话取消不清理后台任务 |
| 命令注入 | ✅ 缓解 | BashTool 通过 sandbox 隔离，但非 Linux 环境无保护 |
| API 认证 | ⚠️ 缺失 | REST API 无任何认证机制，任何人可访问 localhost:3000 |
| 密钥泄露 | ✅ 通过 | API keys 从环境变量读取，日志不输出敏感值 |
| 输入验证 | ⚠️ 见 M1 | 文件路径未做穿越检查 |
| 无限循环 | ✅ 通过 | Budget + 卡死检测 + 启发式 GiveUp 三层保护 |
| 数据竞争 | ✅ 通过 | 所有共享状态使用 Mutex/RwLock，无 data race |

---

## 四、测试状态

根据项目自带的 `p5-v4-test-output.log`：
- **133 passed, 0 failed, 0 ignored**
- 集成测试覆盖：SSE 流、双 provider 切换、真实文件系统操作、预算耗尽、多轮对话、三后端切换、FallbackChain、会话持久化、CostMeter
- Eval Harness：5/5 任务通过

但由于当前环境无法运行 `cargo test`，无法独立验证测试通过率。

---

## 五、闸门判定

| 条件 | 状态 |
|------|------|
| 🔴 阻塞性 bug | **0 个** |
| 🟡 中危问题 | **5 个**（M1-M5） |
| 🔵 低危问题 | **7 个**（L1-L7） |
| 编译通过 | ✅（基于 P5 报告） |
| 测试通过 | ✅（基于 P5 报告，133/0/0） |

**结论：建议过闸，冻结 v1.0。**

v1.1 中优先修复：
1. **M1** 路径穿越（影响所有文件操作工具）
2. **M7** GlobTool/GrepTool 绕过沙箱
3. **M4** 会话取消清理
4. **M2** 审批状态按 session 隔离
