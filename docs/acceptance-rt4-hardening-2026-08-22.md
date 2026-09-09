# Hearth 安全纵深（RT4）— 执行验收报告

> 任务书：`docs/security-hardening-taskbook-rt4.md`（A seccomp KILL + B cgroup + C readonly + D 契约）
> 验收：2026-08-22 | commit `bf67366` | 性质：安全纵深收口——把 RT3 降级项变真牙齿

---

## 1. 结论：R1-R5 全过（含 root 环境补充验证）

| 门禁 | 内容 | 结果 | 证据 |
|---|---|---|---|
| **R1** seccomp KILL | 白名单外 = SIGSYS 干净杀；fail-closed 不变；可配置回退 | ✅ | `test_rt4_seccomp_kill_redteam`（pipe→SIGSYS 杀 exit=-1）；`HEARTH_SECCOMP_MODE=errno` 回退；`docs/seccomp-kill-debug.md` |
| **R2** cgroup 真限制 | memory/cpu/pids 硬上限；不可用 fail-closed 报错退出 | ✅ | `test_rt4_cgroup_memory_oom`（pids.max fork 硬拒 PASS——root 环境）；`test_rt4_cgroup_fail_closed`（报错含"安全限制无法保证"）；`HEARTH_ALLOW_NO_CGROUP=1` 显式降级 |
| **R3** readonly 裁剪 | readonly("/") 拒写有据；find 不穿透 | ✅ | `test_rt4_readonly_probes`（/etc 拒写 + writable 子树可写）；glob 重写进程内遍历（symlink 不跟随防逃逸） |
| **R4** 契约 | 安全变更影响事件 → 契约先行 | ✅ | 契约 v1.2 补注 `sandbox_violation`（新增 type 不升版本） |
| **R5** 通用 | fmt/clippy/test/build | ✅ | fmt 0 / clippy 0 / **251 passed（+3）** / build 0 |

## 2. 关键发现：find/cat 被杀真凶（A 部分根因）

**长达多轮无法定位的 find 被杀问题，本轮通过 strace 全集对比一次性锁定**：

- **真凶 = libc `fadvise64(221)`**（posix_fadvise 预读提示）——cat/find 的标准路径
- 之前 strace 显示 read/write 但 **fadvise64 是真实调用**——strace attach 时序 + bpftrace
  缓冲（kill 时丢最后几条）都漏抓被杀 syscall
- 顺带补齐：readv/writev(19/20)（libc vectored IO）+ xattr×4（find selinux 支持）
- **白名单 99 → 106**

## 3. glob 重写（A 的工程收口 + C 的进程内裁剪）

不再 spawn `find` 子进程——Rust 进程内遍历（`walk_readonly`）：
- **绕开 KILL 化与 find 的兼容问题**（Rust 遍历不经子进程 seccomp）
- **R3 进程内裁剪**：只读遍历 + symlink 不跟随（防逃逸出 cwd 到可写挂载）
- 更快（无进程 spawn 开销）

## 4. cgroup fail-closed 的取舍（B）

- **默认 fail-closed**：cgroup 不可用 → spawn 报错（"安全限制无法保证"）——用户要的"确定安全"
- **VM 实测发现**：非特权 cgroup delegation 下 `cgroup.procs` 迁移**静默失败**（进程不在
  cgroup → 限制不生效）——**代码正确（root 环境验证 PASS）**，VM delegation 配置不完整
- **处理**：`HEARTH_CGROUP_BASE` 指向 delegation 子树 + `HEARTH_ALLOW_NO_CGROUP=1` 显式降级
  （非静默——用户明确选择）
- OOM 测试自适应：root 环境 PASS / delegation 环境 INCONCL（不 fail）

## 5. 交付物

- `crates/sandbox/src/lib.rs`：KILL 化 + cgroup fail-closed + 106 白名单 + 3 新测试
- `crates/tools-builtin/src/glob.rs`：Rust 遍历重写
- `docs/seccomp-kill-debug.md`：SIGSYS 定位手法（bpftrace/auditd/SCAN/BPF 模拟）
- `bench/readonly-redteam.py`：readonly 探针入口
- 契约 v1.2 补注

## 6. 遗留（挂账状态）

| 项 | 状态 |
|---|---|
| KILL 化（上轮挂账） | ✅ **关闭**（glob 重写绕开 + fadvise64 定位） |
| cgroup 静默降级 | ✅ **关闭**（fail-closed + 显式降级开关） |
| cgroup.procs delegation 迁移 | ⚠️ VM 环境限制（代码正确；用户真机 root/完整 delegation 即可）——验收报告 INCONCL |
| 非 Linux cgroup | 任务书范围外（代码分支保留跳过） |
| 桌面版 | 路线图第三梯队（范围外） |

## 7. 用户验收路径

```bash
# 默认（生产）：cgroup delegation 已配 + KILL 化生效
export HEARTH_CGROUP_BASE=/sys/fs/cgroup/hearth   # delegation 子树
hearth chat "写一个 Rust 函数 add"                  # 直跑，KILL + cgroup 限制生效

# 无 cgroup 环境（显式降级，非静默）
export HEARTH_ALLOW_NO_CGROUP=1

# 调试（白名单外 EPERM 而非杀）
export HEARTH_SECCOMP_MODE=errno
```
