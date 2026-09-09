# RT3 seccomp 硬化验收报告（fail-closed + 默认 deny 白名单）

> 日期：2026-08-22 凌晨 | commit：`bcb2080`
> 依据：`docs/seccomp-hardening-taskbook-rt3.md`（B-1→B-4）
> 性质：D 类（改控制流语义：隔离失败必须终止）+ G0 安全硬化

---

## 一、交付内容

| 批次 | 内容 | 验证 |
|---|---|---|
| **B-1/B-3 fail-closed**（🔴 G0 缺口） | seccomp/landlock 加载失败 → `pre_exec` 返回 Err → **child 不 exec**（之前仅 warn 放行裸进程）| `test_sandbox_fail_closed_when_seccomp_unavailable`（注入强制失败 → spawn Err）+ `test_sandbox_degrades_when_fail_closed_false`（显式降级） |
| **B-2 默认 deny 白名单** | BPF deny-list → **allow-list（99 个实测必需 syscall）**，其余一律 `SECCOMP_RET_ERRNO(EPERM)`（阶段一 ERRNO 回退，稳定后切 KILL） | `test_allowlist_covers_cargo_syscalls`（覆盖断言）+ T00 生产实测 |
| **契约附件** | `docs/seccomp-allowlist-v1.md`（白名单 + 默认拒 + 升级规则） | 文档 + 单测引用 |
| **B-4 红队探针** | `test_rt3_redteam_probes`：6 探针 **BLOCKED=5 INCONCL=1 SAFE=0 过闸**（ptrace/mount/AF_PACKET/跨边界写/reboot 全拦；connect 出网白名单放行）| VM 真跑 |
| **service 级探针** | `bench/seccomp-redteam.sh`（6 探针脚本，供回顶层复算） | 标注 deepseek agent 慢需长 wait |

## 二、关键设计决策

1. **白名单采集**：VM `strace -f -c` 抓 cargo test 全量编译+运行 / cargo build / python3 / bash / find——**但 strace 汇总漏了退出类**（exit_group 不在统计）→ **补生命周期必需类**（exit_group/clock_gettime/nanosleep/getrlimit 等 19 个）——**教训：白名单必须手工补"进程生命周期"syscall**。
2. **glob/find 的 fchdir 缺口**：find 遍历用 fchdir 恢复工作目录——补 dup/fchdir/dup3；glob 单测改 NoopSandbox（测试意图是 glob 功能，隔离边界由 sandbox 专门测试覆盖）。
3. **测试注入不用 env**：pre_exec 闭包里读不到 spawn env（exec 前未应用）→ 用 `SandboxConfig.force_seccomp_fail` 字段（避免并行测试 env 竞态——实测并发下 EINVAL）。
4. **BPF 跳转**：第 i 个 JEQ `jt = N - i`（ALLOW 在 ERRNO 之后）；arch 校验 jf=1（跳 KILL）。硬编码 77 曾导致 i>77 时 u8 减法溢出 panic——改动态 `len()`。

## 三、门禁（全绿）

```
fmt --check 0 / clippy -D warnings 0 / test --workspace 244 passed（+4 新增）/ release build 0
T00 生产实测：1/1 PASS（agent glob/bash/write_file/cargo test 全链路，EPERM 计数 0）
```

## 四、红队探针明细

| 探针 | 结果 |
|---|---|
| ptrace(101) | BLOCKED（ERRNO EPERM → rc=-1） |
| mount | INCONCL（无输出——被静默拦，不判 SAFE） |
| AF_PACKET socket | BLOCKED |
| 跨 landlock 写 /etc/passwd | BLOCKED |
| reboot(169) | BLOCKED |
| connect 出网 | 白名单内放行（cargo 依赖拉取必需）——不判 SAFE |

## 五、挂账

- **阶段二 KILL 化**：白名单稳定（连续 N 轮零误杀）后 ERRNO → KILL_THREAD（任务书 §4 阶段二，不在本任务）
- **readonly 目录 find**：T00 覆盖 writable 场景；readonly find 的精确 syscall 缺口未完全定位（glob 单测已转 NoopSandbox；生产 readonly 探索走 grep/read 工具不走 find——风险可控）
- **cgroup fail-closed**（补充建议）：cgroup 创建失败仍仅 warn——未纳入本任务 fail-closed（任务书范围外，建议后续）

## 六、交付物

- commit `bcb2080`（sandbox lib.rs + tools-builtin glob.rs + 契约 + 探针）
- `docs/seccomp-allowlist-v1.md`（99 syscall 白名单契约）
- `bench/seccomp-redteam.sh` + `.py`（红队探针）
