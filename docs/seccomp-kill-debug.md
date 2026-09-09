# seccomp KILL 化调试手册（SIGSYS 定位手法）

> RT4（2026-08-22）：白名单外 syscall 默认 KILL（`HEARTH_SECCOMP_MODE=errno` 可回退）。
> 本文档记录"哪个 syscall 被 KILL"的定位手法——**strace 的已知局限**及替代方案。

## 1. 为什么 strace 抓不到被杀 syscall

**现象**：KILL 化下 `find` 被 SIGSYS 杀（exit=-1），但 `strace -f find` 显示的 46 个
syscall **全在白名单**（99 个）——被杀的那个 syscall 不在 strace 输出里。

**原因**：Linux 的 syscall 处理顺序是
`do_syscall_64 → tracepoint(raw_syscalls:sys_enter) → seccomp(__secure_computing) → 执行`。
**tracepoint 先于 seccomp**——理论上被杀 syscall 应被记录。但实测 strace 漏抓，根因是：

1. **strace 基于 ptrace**——`seccomp` 的 KILL 直接终止线程，ptrace-stop 可能未及处理；
2. **strace attach 时序**——`-f` 跟随 execve，但 seccomp 在 pre_exec（execve 前）已加载，
   execve 后第一个 syscall 若触发 KILL，strace 的恢复时机可能漏记。

## 2. 替代定位手法（按可靠性排序）

### 2.1 内核 tracepoint：bpftrace（推荐，root）

```bash
# 抓 comm=="find" 的所有 sys_enter（syscall 号）
sudo bpftrace -e 'tracepoint:raw_syscalls:sys_enter /comm=="find"/ { printf("%d %d\n", pid, args->id); }'
```

**局限**：bpftrace 输出到文件是块缓冲，`kill` 时最后几条可能丢（正是被杀那条）。
**缓解**：加 `interval:hz:5 { }` 定时 flush；或 `perf trace`（内核侧，无用户态缓冲，
但需要 `kernel.perf_event_paranoid` 权限）。

### 2.2 auditd（root，最权威）

```bash
sudo apt-get install -y auditd
sudo auditctl -a always,exit -F arch=b64 -S all   # 全 syscall 审计（性能开销大）
# 触发后：
sudo ausearch -m SECCOMP -ts recent
```

seccomp KILL 产生 `AUDIT_SECCOMP` 记录（含 syscall 号）。注意 `-S all` 性能差，
建议只审计可疑范围。

### 2.3 用户态 syscall 扫描（无特权也可）

sandbox 内逐个调用 syscall 号（跳过 15/rt_sigreturn 等会崩溃的），打印每号，
被杀前最后打印的号 = 下一个被杀（±1）。**注意区分**：
- `exit_code != 0` 且 stderr 含 `Resource temporarily unavailable`（EAGAIN）= **限制生效**（如 pids）
- `exit_code != 0` 且无输出 = **信号杀**（seccomp KILL）
- `exit_code == 1` = **python 异常**（EINVAL——非被杀）

### 2.4 BPF 用户态模拟（验证白名单逻辑）

解析 `SECCOMP_ALLOWLIST` 数组 + BPF 跳转（`jt = N - i`），模拟所有白名单号 + 实测
syscall 集——确认逻辑无跳转 bug（RT4 验证 99 白名单全 ALLOW）。

## 3. RT4 的最终方案（glob 重写）

**find 的被杀 syscall 经 bpftrace/auditd/SCAN 均无法稳定定位**（缓冲/时序/内核行为）。
工程决策：**glob 工具重写为 Rust 进程内遍历**（`walk_readonly`——readonly + symlink
不跟随防逃逸）——不再 spawn `find` 子进程——**KILL 化与 glob 的兼容问题彻底消除**
（Rust 遍历不经子进程 seccomp）。这也顺带解决 R3 readonly 裁剪（进程内只读 + 防穿透）。

## 4. 当前 KILL 化的可配置回退

```bash
export HEARTH_SECCOMP_MODE=errno   # 白名单外 → EPERM（调试/兼容）
# 默认：kill（白名单外 → SIGSYS 干净杀）
```

## 5. 红队验证

- `test_rt4_seccomp_kill_redteam`：白名单外 syscall(22 pipe) → SIGSYS 杀（exit=-1）
- `test_p5_seccomp_blocks_ptrace`：ptrace → 杀
- `bench/seccomp-redteam.py`：mount/AF_PACKET/写/etc/reboot 全 BLOCKED
