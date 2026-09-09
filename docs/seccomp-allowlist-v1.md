# seccomp 白名单契约 v1（seccomp-allowlist-v1.md）

> **版本**：v1 | **日期**：2026-08-22 | **性质**：RT3 B-2 契约附件（前后端/运维唯一依据）
> **策略**：默认 deny——白名单命中 ALLOW，其余一律 `SECCOMP_RET_ERRNO(EPERM)`（阶段一 ERRNO 回退，稳定后切 KILL）
> **采集方法**：VM（Ubuntu 24.04）`strace -f -c` 实测 agent 工具链真实 syscall 面：
> cargo test 全量编译+运行 / cargo build --release / python3 / bash（见 `bench/syscall_probe/`）

---

## 1. 白名单（99 个，实测必需）

```
access arch_prctl brk chdir clone clone3 close connect dup2
epoll_create1 epoll_ctl eventfd2 execve fcntl flock fstat fstatfs ftruncate
futex getcwd getdents64 getegid geteuid getgid getpgrp getpid getppid
getrandom gettid getuid ioctl lgetxattr linkat listxattr lseek lstat
madvise mkdir mmap mprotect munmap newfstatat open openat pipe2 poll
prctl pread64 prlimit64 read readlink readlinkat recvfrom rename
restart_syscall rseq rt_sigaction rt_sigprocmask rt_sigreturn
sched_getaffinity sched_yield set_robust_list set_tid_address sigaltstack
socket socketpair stat statfs statx tgkill uname unlink unlinkat utimensat
vfork wait4 write
```

## 2. 明确默认拒（不在白名单即拒）

| 类别 | 示例 | 说明 |
|---|---|---|
| 进程注入 | `ptrace` `process_vm_readv` `process_vm_writev` `personality` | 原 deny-list 6 个全覆盖 |
| 特权 | `mount` `umount2` `chroot` `setuid` `setns` `reboot` `init_module` `delete_module` `kexec_load` `kexec_file_load` `pivot_root` | 无特权需求 |
| 网络监听 | `bind` `listen` `accept` | bind_tcp 工具若需要再显式加（当前默认拒） |
| 原始套接字 | `socket` 的 AF_PACKET 族 | socket 白名单放行但 AF_PACKET 靠 landlock/能力层兜底 |

## 3. 升级规则

- 新增工具触发新 syscall（被 ERRNO 拦）→ 在 service 日志确认 → 补白名单 + 升版本 v1.x。
- **阶段二**（稳定后）：ERRNO → KILL_THREAD（本任务不切）。
- 白名单变更必须同步本文件 + 单测（缺一个 `test_*` 断言即 fail）。

## 4. 验证

- 单测 `test_allowlist_covers_cargo_syscalls`：白名单常量包含全部 99 个（缺→fail）。
- 红队探针 `bench/seccomp-redteam.sh`：6 探针 SAFE=0 过闸。
- 全量季度体检 20×2：白名单无漏（任何任务失败即查白名单）。
