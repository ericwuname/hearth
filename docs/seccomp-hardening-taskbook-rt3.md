# RT3 安全硬化任务书 — seccomp 白名单从「尽力而为」升级为「fail-closed + 可验证」

> **签发**：顶层（守门员）
> **日期**：2026-08-22
> **执行窗口**：后端 dev-execution 窗口（照本任务书施工，跑门禁，交证据）
> **来源**：`acceptance-backlog-clear-2026-08-07.md` 第 4 批挂账 RT3（原"用户挂起"→ 收口主线）；G0 硬边界（与已落地 landlock 配对）
> **性质**：**D 类（改控制流语义：隔离失败必须终止而非降级）** + 安全硬化（G0 物理不可违反）

---

## 0. 源码现状（守门员已核验，避免凭印象）

`crates/sandbox/src/lib.rs` 内 seccomp **已半做**，不是从零：

- `sandbox_pre_exec`（:269）在 child pre_exec 阶段**已真调用** `apply_seccomp_deny_list()`（:287）。
- `apply_seccomp_deny_list`（:383-491）是**完整 BPF 实现**：架构校验（x86_64，错则 KILL，:435）+ deny-list（ptrace/reboot/init_module/delete_module/kexec_load/kexec_file_load 共 6 个，:402-409）+ 其余 ALLOW（:465）。
- 已有测试 `test_p5_seccomp_blocks_ptrace`（:1005）用 python3 直调 syscall 101 验证被 KILL。

### 🔴 已发现的真实缺陷（执行窗口必须修，不是优化）

1. **seccomp 失败 = 静默裸跑（违反 G0 fail-closed）**
   `apply_seccomp_deny_list`（:479-490）：`PR_SET_SECCOMP` 失败仅 `tracing::warn!` 然后返回，child 继续**无任何 syscall 过滤**裸跑。landlock 同理（:316-345 失败仅 warn）。对一个"硬安全边界"项目，隔离失败必须**终止 child**（return `Err` 让 pre_exec 失败 → child 不 exec），不能降级放行。
2. **deny-list 语义偏宽松**：当前是"黑名单 6 个 + 其余全 ALLOW"。对"可信委托"的纵深防御，建议**收敛为默认 deny 白名单**（见 §2 必做项），至少把网络类（`socket`/`connect` 出网、`bind`/`listen` 监听）、进程注入类（`ptrace`/`process_vm_readv`/`process_vm_writev`/`personality`）、特权类（`mount`/`umount2`/`chroot`/`setuid`/`setns`）明确处理——agent 工具（`bind_tcp`/`shell`）需要出网则白名单显式放行，其余默认拒。
3. **BPF 无 `SECCOMP_RET_ERRNO` 回退区分**：当前 deny 直接 KILL_THREAD，对"误拦正常工具"会直接杀进程。需权衡 KILL vs ERRNO——建议**先 ERRNO（便于工具在真实任务里暴露被拦的 syscall 名，便于调白名单），待白名单稳定后切 KILL**，见 §4 渐进策略。

---

## 1. 终态目标

RT3 完成后，Hearth 的 Linux 沙箱满足：

- **seccomp 与 landlock 同为 G0 硬边界**：任一隔离机制**不可用 / 加载失败 → child 进程被终止（fail-closed），绝不放行裸进程**。
- **默认 deny 白名单**：明确列出 agent 工具链需要的 syscalls（含出网 `connect`、必要的 `socket`/`pipe`/`clone`/`execve`/`fork`/`wait4`/`read`/`write`/`open`/`openat`/`stat`/`futex`/`mmap`/`mprotect`/`rt_sigprocmask`/`sched_yield`/`clock_nanosleep`/`clone3` 等），其余一律拒。
- **可验证**：提供 `cargo test` 级断言（已有 ptrace 测试需保留并扩展），以及**一份可在 VM 真跑的红队探针脚本**（见 §3 门禁 R4）。

---

## 2. 必做项（B 批，按依赖排序）

### B-1 seccomp fail-closed（🔴 阻塞，最优先）
- 改 `apply_seccomp_deny_list` / `apply_landlock_in_child`：失败（syscall 返回负 / `prctl` 非零）时**返回 `Err`**，`sandbox_pre_exec` 据此**让 pre_exec 返回 Err**（std 的 pre_exec 返回 Err 会使 `spawn()` 失败，child 不 exec）——即 fail-closed。
- 加 `SandboxConfig` 字段 `fail_closed: bool`（默认 true）。若显式 `false`（仅限 NoopSandbox / 开发调试），才允许 warn 放行；生产路径默认 true。
- **证据**：单测 `test_sandbox_fail_closed_when_seccomp_unavailable`（mock 一个 seccomp 必失败路径，断言 `spawn()` 返回 Err / child exit 非 0）。

### B-2 默认 deny 白名单（替代 deny-list）
- 重写 BPF 为**默认 deny** 结构：`arch 校验 → 加载 nr → 白名单命中则 ALLOW → 末位 `SECCOMP_RET_*`（先 ERRNO 后 KILL）→ 架构错 KILL`。
- 白名单 syscalls 集合：以 agent 工具（`glob`/`grep`/`edit`/`write_file`/`bash`/`cargo test`/`python3`）在 Linux 实测 `strace -f` 抓到的 syscalls 为准，写入 `docs/seccomp-allowlist-v1.md` 作为**契约附件**（前后端/运维唯一依据）。
- 出网相关：`connect`（白名单放行，仅限必要；`bind`/`listen` 默认拒，除非 `bind_tcp` 工具显式需要）、`socket`（AF_INET/AF_UNIX 放行，AF_PACKET 拒）。
- **证据**：`docs/seccomp-allowlist-v1.md` + 单测断言白名单覆盖 `cargo test` 所需 syscalls（缺一个 `test_*` 就 fail）。

### B-3 landlock 同步 fail-closed
- `apply_landlock_in_child` 失败（ruleset 建不出 / restrict 失败）同样改为**受 `fail_closed` 控制**：默认 true 时终止 child。现状已能 probe，主要是把"warn 放行"改成"fail-closed 终止"（与 B-1 共用开关）。

### B-4 红队探针（R4，VM 真跑）
- 写 `bench/seccomp-redteam.sh`：在 VM（Ubuntu 24.04，非特权 userns 受限）起 Hearth service，跑 6 个攻击探针（每个必须**被拦**或**环境不支持时报 INCONCL 单列**，不得混入 SAFE）：
  1. `ptrace(PTRACE_TRACEME)` → 必 KILL/EPERM
  2. `mount(...)` → 必 EPERM
  3. `python3 -c socket socket(AF_PACKET)` → 必 EPERM/被拒
  4. 尝试写 `/etc/passwd`（跨 landlock 边界）→ 必 EACCES
  5. `connect` 到外部 IP（非白名单端口）→ 必被拦（白名单内则放行，探针须区分）
  6. `reboot()` → 必 EPERM
- **铁律（来自 codex-vm-test skill）**：跑探针前先 5 行诊断确认 service `HEALTH_OK=True`；INCONCL 单列不混 SAFE；鉴权探针仅 401/403 算生效。

---

## 3. 门禁（每批交证据）

- **R1（可静态验）**：`grep` 确认 `apply_seccomp_deny_list` 失败路径返回 `Err`；`fail_closed` 字段默认 true；`seccomp-allowlist-v1.md` 存在且被 `lib.rs` 引用（路径/版本注释）。
- **R2（需浏览器/真实 Linux）**：VM 上 `cargo test -p sandbox` 全过（含 B-1/B-2 新增断言）。
- **R3（测试必须能失败）**：`[自检]`——临时把白名单改成空，断言 `test_*` 至少 1 项 fail（证明断言非永真）。
- **R4（红队探针）**：`bench/seccomp-redteam.sh` 在 VM 真跑，6 探针结果分类（BLOCKED / INCONCL / SAFE）；**SAFE=0 才过闸**；INCONCL 单列且附原因。
- 通用：`cargo fmt --check` 0 / `clippy -D warnings` 0 / `test --workspace` 全过（baseline 240，新增断言并入）。

---

## 4. 渐进策略（避免白名单误杀正常工具）

1. **阶段一（本任务 B 批）**：默认 deny + **ERRNO 回退**（被拦 syscall 返回 EPERM，工具报"Operation not permitted"而非被 KILL）。跑真实 10 个任务，grep 日志收集所有触发的 syscall 名，补齐白名单。
2. **阶段二（验收后可选）**：白名单稳定（连续 N 轮零误杀）后，将回退从 ERRNO 切 KILL_THREAD——此时才是"硬 kill"。
3. **阶段二不在本任务范围**：本任务交付"可用 + 可验证 + fail-closed 的 ERRNO 版默认 deny"，KILL 化作为后续 hardening 任务。

---

## 5. 红线（执行窗口不得违反）

- **禁**把 `fail_closed` 默认改 false 来"绕过"测试——这是把 G0 安全边界焊死成装饰。
- **禁**删已有的 `test_p5_seccomp_blocks_ptrace` 等测试来过闸——只增不改，且必须补 B-1/B-2 新断言。
- **禁**引入 `libseccomp` 等额外重依赖除非经顶层确认（当前用 raw BPF 是刻意零依赖，保留）。
- **禁**碰 `codex-rust` 旧名 / crate 名（仍 `codex-cli`/`codex`），与定名约定一致。
- 事实产生权：seccomp 决策是**后端安全事实**，前端/CLI 只投影"隔离生效/失败"，不决定策略。

---

## 6. 验收回顶层

执行窗口交 B-1~B-4 证据后，回守门员独立验收（不读报告，跑 R1-R4 + grep 源码 + VM 红队探针复算）。🔴=0 且 实现率≥0.9 过闸。
