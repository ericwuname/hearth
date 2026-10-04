# seccomp 白名单契约 v1（seccomp-allowlist-v1.md）

> **版本**：v1 | **日期**：2026-08-22 | **性质**：RT3 B-2 契约附件（前后端/运维唯一依据）
> **策略**：默认 deny——白名单命中 ALLOW，其余一律 `SECCOMP_RET_ERRNO(EPERM)`（阶段一 ERRNO 回退，稳定后切 KILL）
> **采集方法**：VM（Ubuntu 24.04）`strace -f -c` 实测 agent 工具链真实 syscall 面：
> cargo test 全量编译+运行 / cargo build --release / python3 / bash（见 `bench/syscall_probe/`）

---

## 1. 白名单（136 个，实测必需）

> **D-148（2026-10-04, traecode）**：本节此前手抄了一份 99 个名字的清单；白名单随 P5 出网放行等
> 扩容到 **136** 条后本文件未同步——同一事实对外出现两个值（README 早已写 136，本文件与
> `hearth-cli-guide.md` 却写 99）。**权威清单只有一份**：代码常量
> `crates/sandbox/src/lib.rs` 的 `SECCOMP_ALLOWLIST`（`linux_impl` 模块，类型即计数
> `[u32; 136]`）。查看：`grep -A200 'SECCOMP_ALLOWLIST: \[u32;' crates/sandbox/src/lib.rs`。
> 计数一致性由门禁 `crates/sandbox/tests/seccomp_allowlist_doc_gate.rs` 锁死（只改本文件数字、
> 不改代码即红）。
>
> **本文件不再手抄名单**（手抄已实证会漂移）：运维判据 = **计数（136）** + 下方"默认拒"类别 +
> §3 升级规则。名单的权威表达是代码本身。

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
- 白名单变更必须同步**本文件的计数**（门禁 `crates/sandbox/tests/seccomp_allowlist_doc_gate.rs` 锁死：
  代码 `[u32; N]` 与本文声明不一致即红）+ 单测（缺一个 `test_*` 断言即 fail）。

## 4. 验证

- 单测 `test_allowlist_covers_cargo_syscalls`：白名单常量包含全部 136 个（缺→fail）。
- 红队探针 `bench/seccomp-redteam.sh`：6 探针 SAFE=0 过闸。
- 全量季度体检 20×2：白名单无漏（任何任务失败即查白名单）。
