# 定向回规清单：sandbox 命令执行修复（handoff-sandbox-fix）

> 审计：code-audit-gatekeeper，2026-07-28
> 问题：`test_linux_sandbox_echo` / `test_linux_sandbox_timeout` 在真实 Linux（landlock 可用）失败
> 性质：**sandbox crate 既有缺陷，非 6 轮修复回归**（未改动 sandbox 实现）
> 影响：连带使 R2（glob/grep 走沙箱）在真实 Linux 上不可用（find/rg 被 landlock 阻断）

---

## 根因（已源码核实）

`crates/sandbox/src/lib.rs:58-68`：

```rust
impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            read_only_paths: vec![],          // ← 空！landlock 下无任何只读系统路径
            writable_paths: vec![cwd],
            ...
        }
    }
}
```

`apply_landlock_in_child`（lib.rs:250-328）对 `read_only_paths` 加 `FS_RO`（= READ_FILE|READ_DIR|**EXECUTE**）规则，对 `writable_paths` 加 `FS_RW`。当 landlock `restrict_self` 在真实内核生效，**仅 cwd 可写、无任何只读路径** → unshare 子进程无法读取 `/bin/echo`、`/usr/bin/sleep`、`/usr/bin/rg`、`/usr/bin/find` 及其动态链接库 → 命令瞬间失败。

- `echo` 测试：stdout 为空 → `contains("hello")` 失败
- `sleep 10` 测试：二进制无法执行、进程瞬间退出 → `timeout(100ms)` 根本没机会触发 → `timed_out=false`

---

## 解决方案 A（推荐，最小且安全）

在 `SandboxConfig::default()` 的 `read_only_paths` 加入运行外部命令必需的最小系统只读路径。
- `FS_RO` 已含 `EXECUTE` 位（lib.rs:191-193），加路径即允许读+执行系统二进制。
- **安全模型不变**：这些路径是只读（`FS_RO`），workspace 外仍不可写；网络/危险 syscall 仍由 seccomp 管控。等同于"容器基础镜像可执行的二进制"——是沙箱应有的基线，不削弱隔离。

### 补丁（crates/sandbox/src/lib.rs:61）

```rust
            read_only_paths: vec![
                // 命令二进制（landlock path_beneath 递归覆盖子项）
                std::path::PathBuf::from("/bin"),
                std::path::PathBuf::from("/usr/bin"),
                std::path::PathBuf::from("/usr/local/bin"),
                // 动态链接器 + 共享库
                std::path::PathBuf::from("/lib"),
                std::path::PathBuf::from("/lib64"),
                std::path::PathBuf::from("/usr/lib"),
                std::path::PathBuf::from("/lib/x86_64-linux-gnu"),
                std::path::PathBuf::from("/usr/lib/x86_64-linux-gnu"),
                // ld.so 缓存
                std::path::PathBuf::from("/etc/ld.so.cache"),
            ],
```

> 说明：`add_landlock_rule`（lib.rs:427-459）对不存在的路径仅 `warn` 跳过，不致命 → 跨发行版（路径差异）安全。非 Debian/Ubuntu 系若缺某路径会自动跳过，不影响其余。

### 为什么不是方案 B（#[ignore]）
方案 B 仅让测试"绿"，但 R2 在真实 Linux 上仍不可用（find/rg 被锁死）。方案 A 同时修复测试 + 让 R2 真正生效，是 R2 在真实 Linux 上工作的**必要条件**。故推荐 A。

---

## 验证命令

在用户 Linux 虚拟机（wutao@192.168.220.131，`~/codex`）执行：

```bash
cd ~/codex
source $HOME/.cargo/env
cargo test -p sandbox -- --nocapture 2>&1 | tail -40
# 期望：test_linux_sandbox_echo ok / test_linux_sandbox_timeout ok
cargo test --all 2>&1 | tail -5
# 期望：TEST_DONE rc=0，全部 crate 通过
```

本机一键（已有脚本 `vm_test.py` 保留 target 增量编译，几分钟）：
```bash
# 本机 Git Bash
export VM_PASS=<redacted-set-in-env>
"C:/Users/87465/.workbuddy/binaries/python/envs/default/Scripts/python.exe" \
  "C:/Users/87465/AppData/Local/Temp/vm_test.py"
```

---

## 提交门槛（gate）

- [ ] `cargo test --all` 全绿（rc=0），含 sandbox 2 个 Linux 测试
- [ ] `test_linux_sandbox_echo` 断言 `stdout.contains("hello")` 通过
- [ ] `test_linux_sandbox_timeout` 断言 `timed_out` 通过
- [ ] R2 真验证：`test_linux_sandbox_echo` 通过即证明 landlock 下外部命令可执行 → glob/grep 走沙箱在真实 Linux 可用
- [ ] `landlock_denies_outside_write` / `seccomp_blocks_ptrace` 仍通过（隔离未被削弱）
- [ ] governance.md 追加本次闭环评审记录
