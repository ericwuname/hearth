# Hearth 安全纵深任务书（RT4）— seccomp 收口 + cgroup 真限制 + readonly find

> 签发：2026-08-22（顶层 / 守门员）
> 前置：RT3 seccomp fail-closed（commit bcb2080）+ 后端真智能 B2/B3（commit 6c46f33）
> 性质：**安全纵深收口**——把 RT3 遗留的"降级项"变成真牙齿，解决用户核心诉求"安全平稳落地、别出意外的风险"
> 目标 commit 建议：`rt4-hardening`

---

## 0. 背景与用户诉求锚点

用户原话（2026-08-22 上下文）："我的最终期望是项目安全，平稳落地，不要在我使用的过程中，出现各种报错及其他意想不到的风险。"

RT3 交付了 seccomp fail-closed（99 白名单 + ERRNO 回退），但**三处仍是"降级/挂账"**，本次必须收口：

1. **seccomp KILL 化回退为 ERRNO**——RT3 实测 `find` 被 KILL 但 strace 无法定位（seccomp 先于 strace），安全边界完整（白名单外仍 EPERM）但**杀伤力不足**：ERRNO 下某些 syscall 被拒后子进程可能走异常分支而非干净终止。用户要的是"意外风险可控"。
2. **cgroup 资源限制标注为"降级非安全降级"**——当前 cgroup 若不可用只是 warn，无真正内存/CPU/pids 硬上限。**fork bomb / 内存爆涨仍是真风险**。
3. **readonly find 裁剪**——sandbox 的 read_only_paths 含 `/`，但 `find`/`stat` 类遍历在超大目录仍可能拖垮 IO；需确认裁剪生效且无绕过。

---

## 1. 门禁（R1-R5，全部需源码证据 + 可复现）

| 门禁 | 内容 | 验收证据 |
|---|---|---|
| **R1** seccomp KILL 化 | 白名单外 syscall = 进程干净终止（SIGSYS），非 EPERM 软拒；fail-closed 不变 | 红队探针 `bench/seccomp-redteam.sh` 新增 KILL 用例：触发白名单外 syscall → 子进程退出码 = SIGSYS(127+signal) 或 status 显式 killed；strace 定位手法文档化（BPF/perf trace 替代 strace） |
| **R2** cgroup 真限制 | 默认启用 cgroup v2 内存/CPU/pids 硬上限；不可用时**fail-closed 报错退出**（非静默降级） | 单元测试/集成测试：注入 memory.high 触发 OOM → 子进程被杀；无 cgroup 权限 → 启动报原因 exit(1) |
| **R3** readonly 裁剪验证 | read_only_paths 真实生效且无绕过；写尝试被拒有据可查 | 探针：在 sandbox 内尝试 `write` 到 `/tmp` 外路径 → EROFS；find 遍历受限目录不穿透 |
| **R4** 契约同步 | 安全纵深变更若影响事件契约（如新增 sandbox_violation 事件）须先补契约再改 BE，不升 schema_version（新增 type 不升） | ai-os-event-contract-v1.md 版本历史新增 v1.2 条目（如有新增 type） |
| **R5** 通用门禁 | `cargo fmt` 0 / `cargo clippy` 0 / `cargo test` 全绿（基线 248 + 新增）/ `cargo build --release` 0 | CI/local 全绿 |

---

## 2. 任务分解

### A. seccomp KILL 化（R1）

- 当前 `crates/sandbox/src/lib.rs` 的 SECCOMP 规则用 `ERRNO(EPERM)` 回退。改为 **`KILL_PROCESS`**（libseccomp `SCMP_ACT_KILL_PROCESS` 或 raw `SECCOMP_RET_KILL_PROCESS`）。
- **根因定位**：RT3 用 strace 失败（seccomp 先于 ptrace）。改用 **`perf trace` / `bpf` 在内核侧抓 syscall**，或临时把某 syscall 从白名单剔除 + 用 `dmesg`/`auditd` 抓 SIGSYS 报告定位。文档化这套定位手法（写 `docs/seccomp-kill-debug.md`）。
- 红队探针升级：`bench/seccomp-redteam.sh` 增加 KILL 用例——故意触发白名单外 syscall（如 `mkdir`/`chmod`/未知号），断言子进程被 SIGSYS 干净杀死（退出码反映 killed），且**主进程（Hearth）不受影响、不 panic**。
- 保留 ERRNO 作为**可配置回退**（环境变量 `HEARTH_SECCOMP_MODE=kill|errno`），默认 kill。

### B. cgroup v2 真限制（R2）

- 在 sandbox 启动 child 前，创建 cgroup v2 子树（`/sys/fs/cgroup/hearth-<sid>`），设 `memory.max` / `cpu.max` / `pids.max`（值来自 config，默认内存 512M、CPU 1.0、pids 256）。
- **fail-closed**：若 cgroup 挂载不可写 / 权限不足 → 启动报 "cgroup 不可用，安全限制无法保证" 并 exit(1)。**移除"降级非安全降级"的静默 warn**——要么真限制，要么不开工（用户要的是确定安全，不是"可能安全"）。
- 测试：注入 `malloc` 超大 / fork bomb → child 被 cgroup 杀；无权限环境 → 启动失败。
- 仅在 Linux + cgroup v2 生效；非 Linux（用户暂不用，但代码分支保留）显式跳过并报已知限制。

### C. readonly 裁剪验证（R3）

- 核验 `read_only_paths=["/"]` 在 landlock + seccomp 双重下真实生效：sandbox 内 `open(O_WRONLY)` 任意非 tmpfs 路径 → EROFS。
- 确认 `find`/大目录遍历不穿透到可写挂载；若发现绕过（如通过 `/proc`/`/dev` 写），补 landlock 规则。
- 探针写 `bench/readonly-redteam.sh`。

### D. 契约与文档（R4 + 收口 RT3 挂账）

- 若新增 `sandbox_violation` 事件（推荐：child 被杀/拒绝时 emit 事实，Observer 可记 autonomy/安全事件）→ 契约补 v1.2 条目（新增 type 不升 schema_version）。
- 更新 `docs/ai-os-event-contract-v1.md` 版本历史。
- 收口 RT3 报告 §5 的 "seccomp KILL 化" 挂账 → 本次关闭。

---

## 3. 红线（守门员铁律）

1. **不信报告信源码**：R1-R3 每个门禁必须有可复现探针脚本 + 真实退出码/行为证据，禁止"应该没问题"式结论。
2. **fail-closed 优先**：R2 cgroup 不可用必须 exit(1)，不得静默降级回"无限制运行"。
3. **契约先行**：任何影响事件流的改动先补契约再改 BE；新增 type 不升 schema_version（§7 规则）。
4. **生产路径真接线**：安全逻辑必须在实际 sandbox child 启动路径上，禁止只在测试桩里做。
5. **不破坏既有门禁**：RT3 的 99 白名单 + ERRNO 回退作为 KILL 的可配置回退保留，不得删除。

---

## 4. 交付清单（执行窗口须提交）

- `crates/sandbox/src/lib.rs`：KILL_PROCESS + cgroup v2 硬限制 + readonly 双重核验。
- `bench/seccomp-redteam.sh`（升级）、`bench/readonly-redteam.sh`（新增）。
- `docs/seccomp-kill-debug.md`：SIGSYS 定位手法（perf/bpf/auditd）。
- 契约补注（如需）。
- 验收报告 `docs/acceptance-rt4-hardening-<date>.md`（含每个探针的真实输出）。

---

## 5. 范围外（明确不属本轮）

- 第三梯队桌面版（B4-2 Tauri）——路线图后续，等安全纵深收口、CLI 真在用户 VM 跑稳后再开。
- 多用户/公开部署（战略已排除）。
- 非 Linux 平台的 cgroup（代码分支保留跳过，不实现）。
