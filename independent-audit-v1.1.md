# v1.1 独立审计：自审盲区报告

> 背景：v1.1 修复完成后，执行方自己写了一篇审计（`v1.1-audit-report.md`）。  
> 本篇是**第三方独立审计**——不重复检查自审已覆盖的内容，专找自审不会发现的问题。  

---

## 自审者的认知盲区（为什么自审不够）

自审报告的目录结构完全镜像 `fix-plan.md` 的轮次：R1→R2→R3→R4→R5→R6→sandbox。它本质上是一份"修复验收清单"，不是独立安全审计。

自审天然存在的盲区：
1. **目标锚定**：只验证"修了的东西修对了吗"，不验证"修的时候引入了新问题吗"
2. **熟悉盲视**：对自己写的代码太熟悉，看不到明显的设计缺陷
3. **测试确认偏误**：测试全绿 → 结论"没问题"，但不问"测试覆盖了哪些没覆盖哪些"
4. **同构思维**：审查者和修复者是同一人，思维方式一致，想不到"换个人会怎么攻击"

以下是站在独立立场发现的、自审完全未触及的问题。

---

## 🔴 高危（自审遗漏）

### X1. Cgroup 资源泄露——timeout 路径未清理

**文件**：`crates/sandbox/src/lib.rs:607-626`

```rust
match result {
    Ok(Ok(Ok((pid, output)))) => {
        cleanup_cgroup(pid);       // ← 只有成功路径清理
        Ok(SandboxOutput { ... })
    }
    Err(_timeout) => {
        Ok(SandboxOutput { timed_out: true })
        // ← ⚠️ 超时时不调用 cleanup_cgroup！
    }
}
```

**问题**：命令超时时，`/sys/fs/cgroup/codex-sandbox-{pid}/` 目录未被删除。每次超时泄漏一个 cgroup 目录。在生产环境中反复超时（LLM 生成的命令超时很常见）→ cgroup 目录堆积，最终 `/sys/fs/cgroup/` 被污染。

**自审为什么没发现**：自审关注的是"命令能执行了吗"（echo/sleep 成功），没问"超时路径清理了吗"。

**修复**：在 `Err(_timeout)` 分支也调用 `cleanup_cgroup`。但问题在于 timeout 时我们没有拿到 pid——`spawn_blocking` 被 timeout 中断后，内部的 child 可能已经 spawn 但 wait_with_output 还没返回。解决方案：
```rust
// 方案 A：在 spawn_blocking 外用 Atomic 捕获 pid
let pid_cell = Arc::new(std::sync::atomic::AtomicU32::new(0));
// ... 在 child.spawn() 后 set pid
// 在 timeout 分支读取 pid 并清理
```

---

## 🟡 中危（自审遗漏）

### X2. 审批 HashMap 内存泄漏——session 结束后从不清理

**文件**：`crates/tool-runtime/src/dispatcher.rs:53`

```rust
approval: Mutex<HashMap<String, ApprovalState>>,
```

**问题**：`set_approval_pending` 以 session_id 为 key 插入条目。`resolve_approval` 改为 Approved/Denied 后条目仍在。只有 `reset_approval` 会删。但搜索全项目——`reset_approval` 只在测试里调过。生产代码中：
- Session 正常结束 → 不删
- Session cancel → 不删
- Session error → 不删

每创建一个 session，如果触发过审批流程，就会在 HashMap 里留下一个永久条目。1000 个 session = 1000 个永远不会被清理的 key。

**自审为什么没发现**：自审只验证了"两个 session 互不干扰"（隔离成功），没验证"session 结束后数据是否被回收"（生命周期完整）。

**修复**：在 `SessionManager` 的 send_message 结束处理中（正常/error/cancel 三条路径），调用 `self.dispatcher.reset_approval(&session_id).await`。

---

### X3. 发送第二次消息时覆盖 cancel_tx——前一个任务变成不可取消的僵尸

**文件**：`crates/service/src/session.rs:229-233`

```rust
let (cancel_tx, mut cancel_rx) = oneshot::channel::<()>();
{
    let mut s = session.lock().await;
    s.cancel_tx = Some(cancel_tx);  // ← 覆盖！如果前一个 cancel_tx 还在，直接被丢弃
}
```

**问题**：虽然当前代码中 `agent.take()` 使得同 session 不会被调用两次 send_message（第二次会走 "agent not available" 分支），但这是**隐式依赖**。假如未来扩展支持可恢复的 agent，或者有重试逻辑，这个覆盖就是 bug。更关键的是：如果 agent 完成（Done）后 `agent` 被设回 Option，再调 send_message——当前 `agent` 恢复的逻辑不存在，但如果加上，就会覆盖旧 cancel_tx 导致旧任务不可取消。

**当前影响**：低，因为 session 状态机（agent.take + running flag）阻止了重复调用。但 **架构上这是隐患**——两个机制（agent availability 和 cancel channel）互相依赖且依赖关系不显式。

**修复**：在 `cancel_tx` 被覆盖前检测并警告，或者确认 session 不可重入后直接拒绝第二次 send_message。

---

### X4. 取消后不发送 Done 事件——SSE 连接无正常关闭信号

**文件**：`crates/service/src/session.rs:273-291`

```rust
_ = &mut cancel_rx => {
    let _ = tx.send(AgentEvent::Error {
        message: "session cancelled".into(),
    });
    cancelled = true;
    break;  // ← 跳出循环后，直接 return，不发送 Done 事件
}
```

**问题**：取消路径发送 `Error` 事件后就 return，不发送 `Done` 事件。前端 SSE 连接收到 Error 后不知道流是否真正结束。依赖 15 秒 keepalive 超时来自然断连。与正常结束路径（发送 `Done`）不一致。

**自审为什么没发现**：自审关注"cancel 是否真的停止了 agent"（abort 实现），没关注 SSE 协议完整性。

**修复**：在 return 之前发送 `AgentEvent::Done { report: json!({"ok": false, "status": "cancelled"}) }`。

---

### X5. has_rg() 绕过沙箱执行命令

**文件**：`crates/tools-builtin/src/grep.rs:124-131`

```rust
async fn has_rg() -> bool {
    tokio::process::Command::new("rg")
        .arg("--version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}
```

**问题**：Round 2 声称"Grep 走沙箱"，但实际上 `has_rg()` 在沙箱外执行 `rg --version`。如果攻击者在 PATH 中放置恶意 `rg` 二进制，它在沙箱外运行，landlock/seccomp 无效。

**自审为什么没发现**：自审检查了搜索路径（`self.sandbox.spawn`），但没有审计"辅助函数是否也在沙箱内"。这是一个典型的"修了主路径，漏了旁路"。

**修复**：将 `has_rg()` 也通过 sandbox 执行探测：
```rust
let output = self.sandbox.spawn("rg", &["--version"], &ctx.cwd, &[], Duration::from_secs(1)).await;
output.map(|o| o.exit_code == 0).unwrap_or(false)
```
或简单策略：总是先试 rg（走沙箱），失败时 fallback 到 grep，无需单独探测。

---

## 🔵 低危（自审遗漏）

### X6. GlobTool 的 `find` 命令参数顺序可能在某些实现上错误

**文件**：`crates/tools-builtin/src/glob.rs:63-69`

```rust
let argv: Vec<String> = vec![
    cwd_str,      // find 的起始目录
    "-path".into(),
    path_arg,      // format!("{}/{}", cwd, pattern)
    "-type".into(),
    "f".into(),
];
```

`find` 的标准调用是 `find <paths...> <expression>`。这里 `cwd_str` 作为起始目录，后面跟 `-path ...` 表达式——语义正确。但注意 `path_arg` 的格式是 `{cwd}/{pattern}`（如 `/tmp/workspace/*.rs`），这意味着 `find` 的 `-path` 匹配的是绝对路径。如果 `cwd` 是相对路径，这个拼接会出问题。好在 `sandbox.spawn` 在内部 `current_dir(&cwd_clone)`，且 `ctx.cwd` 在 `create_sandbox` 默认配置中是 `current_dir()`（绝对路径），所以实际中不会触发此 bug。但代码的隐式假设太多，可读性差。

**修复**：用注释说明 `ctx.cwd` 的路径格式假设。

### X7. cancel_session 设置 running=false 但通知与 spawn 任务有竞态窗口

**文件**：`crates/service/src/session.rs:419-425`

```rust
pub async fn cancel_session(&self, id: &str) -> anyhow::Result<()> {
    // ...
    if let Some(tx) = s.cancel_tx.take() {
        let _ = tx.send(());
    }
    s.running = false;  // ← 如果 spawn 还没执行到 s.running = true，这里设为 false 后会被覆盖
    Ok(())
}
```

**问题**：send_message 里 spawn 的 task 在异步执行 `s.running = true`（line 238）。cancel_session 设置 `s.running = false` 之后，spawn task 可能之后才执行 `s.running = true`，覆盖取消状态。虽然不影响取消的实际功能性（agent_future 会被 abort），但 `get_status` 返回的 `running` 字段会不准确。

**自审为什么没发现**：自审验证了 agent_future.abort() 被执行，但没检查状态字段的一致性。

**修复**：spawn task 中也检查 running 状态，如果已被取消就不设为 true。或使用更正规的状态机（AtomicU8）。

### X8. 137 个测试全绿——但 VM 端测试数量从 138 降到 137？

v1.1-audit-report 写着 138（§5），但实际上原 133 个 + 新增 7 个应该 = 140。测试报告说是 136 原有 + 测试二进制 36 个。数据来回不干净。这里不是 bug，是审计痕迹的可信度问题——如果连数字都对不上，审计者怎么相信其他结论？

---

## 自审未触及的全局性问题

### Gap 1: 没有针对修复本身的回归测试
自审跑的是 `cargo test --all`，这意味着测试的是**最终代码快照**。但没有做：
- **每轮修复前后测试对比**：R1 改后跑一次、R1+R2 改后跑一次……无法定位具体哪轮引入了回归
- **新增测试是否真的有"能失败"的设计**：自审的守门员审计提到要查"测试必须有断言"，但没有验证"如果把修复代码回退，测试是否会红"

### Gap 2: 没有并发/压力测试
审批隔离（R5）和会话取消（R4）都是并发敏感功能。自审测试是单线程 tokio test（逐个 await），没有模拟：
- 100 个 session 同时设置审批
- 快速 cancel + send_message 交错
- 高频率 send_message → cancel → send_message

### Gap 3: 没有资源泄露测试
`cargo test` 不检测资源泄露。cgroup 目录泄露（X1）、HashMap 内存泄露（X2）全漏。

### Gap 4: 没有负面测试
所有测试都是"正常路径"测试。没有：
- 验证 `find` 命令不存在时 GlobTool 的错误信息
- 验证 `rg` 和 `grep` 都不存在时 GrepTool 的行为
- 验证超大 pattern（10KB）输入

### Gap 5: 没有检查 Cargo.toml 变更
新增依赖？版本变动？可能引入供应链风险。

---

## 对自审报告的修正

| 自审声称 | 实际情况 | 级别 |
|---------|---------|------|
| "R2 沙箱接线：生产真接真实 LinuxSandbox" | has_rg() 仍在沙箱外执行 (X5) | 🟡 |
| "R5 审批隔离：生产路径全链路透传，无空壳" | HashMap 条目永不清理 (X2) | 🟡 |
| "sandbox 命令执行硬验收已闭环" | timeout 路径 cgroup 不清理 (X1) | 🔴 |
| "R4 会话取消：select! + abort() 真中止" | 取消后缺少 Done 事件 (X4) | 🟡 |
| "全仓 test result: FAILED = 0" | 测试全绿不代表没 bug，资源泄露类 bug 测试不覆盖 | — |

---

## 闸门判定（独立视角）

| 条件 | 状态 |
|------|------|
| 自审已覆盖的问题（R1-R6+sandbox） | ✅ 修复落地，可验收 |
| 🔴 自审遗漏的高危问题 | **1 个**（X1 cgroup 泄露） |
| 🟡 自审遗漏的中危问题 | **4 个**（X2-X5） |
| 🔵 自审遗漏的低危问题 | **3 个**（X6-X8） |
| 自审未涉及的测试盲区 | **5 类**（Gap 1-5） |

**结论**：v1.1 的修复本身是正确的，自审对其覆盖范围内的验收结论可采纳。但自审存在 5 类盲区导致遗漏 **8 个实际问题**（1 高危 / 4 中危 / 3 低危）。

**建议**：
1. **先修 X1（cgroup 泄露）**——这是明确的内存/资源泄露，且容易被生产环境触发
2. X2-X5 在 v1.2 计划中排入
3. 补充 Gap 1（回归测试）和 Gap 5（Cargo.toml 变更检查）作为发布前的必要步骤
4. v1.1 的 `read_only_paths=["/"]` 保密性折中——需与项目 owner 明确确认是否可接受
