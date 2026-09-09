# 【阶段报告】P1 工具+沙箱（v2 终版 — 守门员过闸后修正）

## 1. 实际 LOC（grep 实算）

| crate | src LOC | test LOC |
|-------|---------|----------|
| agent-core | 752 | 0 (tests inline) |
| agent-types | 189 | 0 |
| api | 133 | 0 |
| llm-gateway | 287 | 0 |
| llm-openai | 580 | 0 |
| sandbox | 484 | 0 |
| service | 572 | 614 |
| tool-runtime | 366 | 0 |
| tools-builtin | 637 | 0 |
| **总计** | **4,614** | — |

测试数：**66** (7+4+3+6+5+7+4+6+7+16+1 doc-test)

## 2. 承接强制项 (F1/F2/F3)

| 编号 | 项 | 状态 | 证据 |
|------|-----|------|------|
| F1 | send_message 注入用户消息 | ✅ 已做 | `loop.rs:93` pending_user_messages; `loop.rs:123` add_user_message 压入队列; `loop.rs:376-378` run() 在 ctx_mgr 重置后 drain 重放; `test_p1_f1_multi_turn_user_message_in_context` — SpyingProvider 拦截 ChatRequest，断言 `F1_PROOF_MARKER_42` 进入 messages |
| F2 | 区分 natural Done 与 error/budget | ✅ 已做 | `loop.rs` run() 三路径: natural→Done{ok:true}, budget→Done{ok:false}, error→Done{ok:false}; session.rs 不再合成 Done |
| F3 | MockProvider 推进脚本 | ✅ 已做 | `integration_test.rs` Mutex<MockScriptState> + cursor 推进; 脚本耗尽返回 DONE |

## 3. 验收项

### A1: 真实工具集 [PASS]
**证据**: `test_p1_tools_real_fs` — edit 写文件 → read 读回 → 断言磁盘实际内容
```
test test_p1_tools_real_fs ... ok
```
5 工具: bash(sandbox), read, edit, glob, grep。测试: glob 3 + grep 4 + bash 5 = 12 tool tests。

### A2: 沙箱隔离 [PASS]（v2 修正）
**证据**: sandbox crate **7 tests**:
```
test test_noop_sandbox_echo                         ... ok
test test_noop_sandbox_timeout                      ... ok
test test_noop_sandbox_warn_on_creation             ... ok
test test_sandbox_config_writable_paths_populated   ... ok
test test_linux_sandbox_echo                        ... ok
test test_linux_sandbox_timeout                     ... ok
test test_linux_sandbox_isolation_not_bare_command  ... ok
```
- **LinuxSandbox**: `unshare --mount --pid --fork`（真实 namespace 隔离）+ cgroups v2 (memory.max/cpu.max/pids.max，SandboxConfig 字段被消费)。**不再裸 tokio::process::Command。**
- **landlock FS 限制**: ⚠️ **P1 未强制**。`apply_landlock()` 仅检测 ABI 版本并打日志，无 `restrict_self`/ruleset 调用。此为已知缺口，已作为 **P5 硬🔴 承接项**（非可选）。
- **seccomp**: 明确留 P5。
- NoopSandbox: 非 Linux 降级 + WARN。

### A3: 工具审批闭环 [PASS]
**证据**: `test_approval_flow` + `test_approval_deny` — `do_act` 检查审批 → emit NeedApproval → `check_approval().await` 真实阻塞 → resolve_approval("approve"/"deny")。`submit_approval` 端点落地: session.rs 调用 dispatcher.resolve_approval()。

### A4: 工具调度 [PASS]
**证据**: `test_parallel_execution`（2 工具并行，结果全部回收）+ `test_tool_timeout`（50ms 超时 → error）。7 tests in tool-runtime。

### A5: 错误与预算真实终止 [PASS]
**证据**:
```
test test_p1_budget_exhausted_error ... ok  (Error事件 + Done不含"completed")
test test_p1_natural_done ... ok  (Done{ok:true})
```
- budget 耗尽 → Error 事件 + Done{ok:false, status:"budget_exhausted"}
- 自然完成 → Done{ok:true, status:"completed"}

### A6: 集成测试覆盖 [PASS]
**证据**: `cargo test --workspace` 全部通过
```
66 passed; 0 failed; 0 ignored
```
P0 测试保留且仍 PASS。

## 4. 偏离清单

| 级别 | 项 | 说明 |
|------|-----|------|
| 🔵 | LOC 偏离 | 4,614 vs 规划 200k（P1 仅为工具+沙箱层） |
| 🔵 | 新增依赖 | sandbox: caps 0.5, nix 0.29（当前未消费，留 P5 landlock）；agent-core: uuid; service: tempfile 3 (dev) |
| 🟡 | landlock stub | apply_landlock() 仅检测+日志，无真实施加 → P5 硬🔴 承接 |
| 🟡 | seccomp 未做 | 留 P5 |

🔴 = 0

## 5. 自评估实现率

**6/6 = 1.0** (A1–A6 全 PASS，含 v2 修正)
F1/F2/F3 全真修

## 6. 风险/遗留

- landlock FS 强制 → P5 硬🔴（不可选）
- seccomp syscall 过滤 → P5
- caps/nix 依赖未消费 → P2 或 P5 落地/清理
- 审批闭环"部分 approve" → P5

## 7. 闸门判定

✅ 守门员正式过闸（P1 v2），解锁 P2。
