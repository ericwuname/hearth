# 守门员源码审计报告 · 2026-07-28

> 审计员：code-audit-gatekeeper（守门员）
> 方法：不信报告信源码 —— 全部结论基于 `grep` + `Read` 逐文件核实与 Python 实算，未抄任何报告数字
> 审计对象：`codex-rust-v1.0-final` 全仓（17 crate）
> 环境限制：本机（Win11）无 Rust 工具链，无法执行 `cargo test`；真 Linux 复跑须由用户在 VM `wutao@192.168.220.131` 完成（见末节）

---

## 1. 硬指标（守门员实算，非报告数字）

| 指标 | 实算值 | 前次报告 | 判定 |
|---|---|---|---|
| crate 数 | 17 | 17 | ✅ |
| `.rs` 文件数 | 38 | 38 | ✅ |
| 总 LOC | **13,007** | 12,924 | 🔵 差 83，参考级 |
| 测试函数数 | **139** | 138 | 🔵 差 1，参考级 |

实算脚本：Python `os.walk` 计行数 + 正则 `#\[(?:\s*tokio::)?\s*test` 数测试函数。

---

## 2. 逐类核实结论

### 2.1 空壳（函数/模块只有签名无实现）—— ✅ 无生产空壳
全仓 `grep` `unimplemented!|todo!|unreachable!` 命中 3 处，逐一核对：
- `code-index/src/lib.rs:327` `unimplemented!()` —— 位于 `#[cfg(test)]` 的 `MockEmbedProvider`，其 `capabilities().chat=false` 且测试只调 `embed`，属**测试 mock**，非生产路径。
- `retriever/src/lib.rs:319` `unimplemented!()` —— 同上，测试 mock。
- `retriever/src/lib.rs:345` `todo!()` —— 位于**字符串字面量**（测试 fixture 内容），非代码。

→ 无生产代码空壳。

### 2.2 生产接线（库建好 ≠ 已集成）—— ✅ 关键路径均已接线
- **provider 直依赖红线**：`agent-core` 仅依赖 `llm-gateway`（grep 全仓确认 `agent-core` 内无 `llm_openai::`/`llm_cn::`/`llm_local::` 任何引用）→ **未违反**"core 直依赖具体 provider → 🔴"。`service` 作为组装根依赖 `llm-cn/local/openai` 属正常。
- **memory 已真接线**：`service/src/session.rs:10` 导入 `MemoryStore`；`service/src/main.rs:139` 创建 `JsonlMemoryStore`；`session.rs:79 set_memory_store` 在生产路径注入（非仅测试）。
- **api 北向契约**：`api/src/lib.rs` `AgentEvent` 8 变体，serde 往返测试 `test_agent_event_all_variants`（lib.rs:116）断言恰好 8 个变体可序列化往返 → 真断言，能失败。
- **lsp-bridge 已接线**：`agent-core/src/loop.rs:110` 字段、`loop.rs:152` 默认 `NoopLspBridge`、`loop.rs:638-641` 生产路径 severity 映射。

### 2.3 孤儿库（建好但无任何调用方）—— 🟡 1 项
- **`telemetry` crate 全程零调用**（grep `telemetry::|init_telemetry` 全仓 0 命中）。其导出的 **eval harness**（`telemetry/src/eval.rs` `EvalRunner`）仅在 crate 内部测试被构造（eval.rs:374），生产代码与 `cargo test` 管线均不调用。文档宣称的"硬验收 eval harness（真实文件系统断言）"在源码层存在且 `eval.rs` 自带测试，但**未被集成进验收流程**。
  - 性质：🟡 范围偏离（未接线）。须用户确认：要么接进 CI/集成测试作为门禁，要么本阶段明确声明 eval harness 不在验收范围内。
- 其余 16 个 crate 均有 ≥1 个调用方，非孤儿。

### 2.4 零断言测试 —— 🔵 1 项（弱测试，非永久 PASS）
全仓扫描 139 个测试函数，仅 1 个无显式断言：
- `sandbox/src/lib.rs:698 test_noop_sandbox_warn_on_creation` —— 仅构造 sandbox 后无断言。但该测试在 `create_sandbox` panic 时会失败，故属"冒烟测试"而非"永远 PASS"，按本门铁律（"从不断言、永远 PASS 才判 🔴"）**不构成 🔴**，列为 🔵 参考级。建议补显式断言（如捕获 tracing 日志验证 WARN/INFO 行为）。

### 2.5 北向契约文档可追溯性 —— 🔵 1 项
`api/src/lib.rs:8` 注释"MUST match architecture.final.md §15.3"，但 `docs/` 仅有 `p0`–`p4` 阶段报告与 `gatekeeper-p0-reexamination.md`，**无 `architecture.final.md`**。契约本身字段/断言完整（见 2.2），仅文档可追溯性缺失。

### 2.6 硬验收（沙箱 / seccomp / 命令执行）—— ✅ 源码已闭环，待真 Linux 复跑
| 硬验收项 | 测试 | 断言（能失败） | 源码状态 |
|---|---|---|---|
| landlock 拒绝越界写 | `test_p5_landlock_denies_outside_write` (lib.rs:796) | 真断言 | ✅ |
| seccomp 拦截 ptrace | `test_p5_seccomp_blocks_ptrace` (lib.rs:867, assert@893) | 真断言 | ✅ |
| 沙箱命令执行 echo | `test_linux_sandbox_echo` (lib.rs:711) `stdout.contains("hello")` `exit_code==0` | 真断言 | ✅ |
| 沙箱超时 | `test_linux_sandbox_timeout` (lib.rs:726) `timed_out` | 真断言 | ✅ |
| 超时 cgroup 回收 | `test_linux_sandbox_timeout_reaps_cgroup` (lib.rs:739) | 真断言 | ✅ |

**前次治理记录"硬验收·sandbox 命令执行 ❌ 未闭环"已在源码层解决**：`SandboxConfig::default().read_only_paths` 现为 `vec![PathBuf::from("/")]`（lib.rs:61-71，整树只读+执行，workspace 外仍禁写），等价于 `firejail --ro-root`。根因（旧值空 → landlock 锁死 `/bin/echo` 等）已消除。

---

## 3. 偏离率三级汇总

| 编号 | 类别 | 严重度 | 说明 | 源码定位 |
|---|---|---|---|---|
| D1 | 孤儿库 | 🟡 | telemetry eval harness 未集成进验收管线 | `telemetry/src/eval.rs:57`；全仓 0 调用 |
| D2 | 范围偏离 | 🟡 | lsp-bridge 真实 LSP client 明确 defer 至 P5（Noop 为生产默认，已接线） | `lsp-bridge/src/lib.rs:6-9` |
| D3 | 范围偏离 | 🟡 | `main.rs` 无 `OPENAI_API_KEY` 时用 `"sk-placeholder"` 静默回退，生产须配置真实 key | `service/src/main.rs:35-36` |
| R1 | 弱测试 | 🔵 | `test_noop_sandbox_warn_on_creation` 无显式断言（冒烟级） | `sandbox/src/lib.rs:698` |
| R2 | 文档可追溯 | 🔵 | 北向契约引用的 `architecture.final.md §15.3` 不在 `docs/` | `api/src/lib.rs:8` |
| R3 | 文档过期 | 🔵 | `sandbox/src/lib.rs:4` 注释"seccomp deferred to P5"，但 seccomp 已实现（`apply_seccomp_deny_list` @342） | `sandbox/src/lib.rs:4` |
| R4 | 数字出入 | 🔵 | LOC/测试数实算与前次报告差 1–83 | 见 §1 |

**🔴 计数 = 0。**

---

## 4. 闸门判定

```
🔴 = 0  且  实现率 ≥ 0.9（6 轮修复 + 沙箱硬验收源码闭环均达标，仅 🟡 待确认）
```

**判定：过闸（PASS）。** 源码层面无 🔴 阻塞项，无生产空壳、无未接线红线、无永久 PASS 测试、北向契约真断言、沙箱硬验收代码已闭环。

**放行前提（🟡 须用户在交付/部署前确认或补）**：
- D1：telemetry eval harness 接进验收管线，或本阶段声明其不在验收范围内。
- D3：生产部署配置真实 `OPENAI_API_KEY`（建议无 key 时 fail-fast 而非静默 placeholder）。
- D2 已文档化，可接受。

**执行验证缺口（非代码缺陷，须真 Linux 复跑）**：
本审计为**源码级**（守门员铁律：不信报告信源码）。沙箱硬验收修复已落源码且测试均为能失败的真断言，但 `cargo test --all` 全绿须由用户在 VM 复跑确认（本机无 Rust 链）。建议复跑命令：

```bash
cd ~/codex && source $HOME/.cargo/env
cargo test -p sandbox -- --nocapture 2>&1 | tail -40   # 期望 echo/timeout/landlock/seccomp 全 ok
cargo test --all 2>&1 | tail -5                          # 期望 TEST_DONE rc=0
```

---

## 5. 提交门槛（gate，留给执行/复跑方）
- [ ] `cargo test --all` 全绿（rc=0），含 sandbox 4 个 Linux 测试
- [ ] `test_linux_sandbox_echo` 断言 `stdout.contains("hello")` 通过
- [ ] `test_linux_sandbox_timeout` 断言 `timed_out` 通过
- [ ] `test_p5_landlock_denies_outside_write` / `test_p5_seccomp_blocks_ptrace` 仍通过（隔离未被削弱）
- [ ] D1 决策落地（接线 or 声明免验收）
- [ ] D3 生产 key 策略确认
- [ ] 🔵 项（R1–R4）择机清理（不阻塞）
