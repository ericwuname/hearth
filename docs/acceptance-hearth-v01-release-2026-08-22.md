# Hearth v0.1 发布验收报告（可装基线）

> 日期：2026-08-22 | 执行窗口：施工 | 任务书：`docs/hearth-v01-release-taskbook.md`
> 前置基线：CLI 派 A（9bc2715）+ 后端真智能（6c46f33）+ RT4 安全纵深（bf67366）
> 判据：**本报告过闸 ≠ 用户验收过闸**——用户真机跑通 §5 才算 v0.1 收口。

---

## 1. 门禁结果总览

| 门禁 | 内容 | 结果 | 证据 |
|---|---|---|---|
| **R1** 源码安装可用 | build 产双 bin + install 到 ~/.cargo/bin 任意目录可调 | ✅ | VM 实测：hearth/codex --version = 0.1.0 |
| **R2** 预编译 release | tarball + install.sh 去占位 | ✅ | 6.2MB tarball + LOCAL_TARBALL 实测装成 |
| **R3** 零配置直跑 | 无 key 可行动错误 + init/config set 引导 | ✅ | 实测 3 步下一步 + config set 后 🔒 直跑 |
| **R4** 安全默认 ON | KILL 化默认 + cgroup fail-closed + 降级指引 | ✅ | 开关就位 + README 安全段 |
| **R5** 文档齐全 | README 五段 | ✅ | README.md 交付 |
| **R6** 通用门禁 | fmt/clippy/test/build 全绿 | ✅ | fmt 0 / clippy 0 / **251 passed** / build 0 |

---

## 2. 实测输出（VM 192.168.220.131）

### R1 源码安装

```
cargo build --release -p codex-cli → Finished release in 25.62s
target/release/codex 7113712 / target/release/hearth 7113712（双 bin 7.1MB）
install → ~/.cargo/bin
cd /tmp && hearth --version → hearth 0.1.0
cd /tmp && codex --version → hearth 0.1.0
```

### R2 预编译 tarball + install.sh

```
hearth-v0.1-x86_64-unknown-linux-gnu.tar.gz = 6,241,556 字节（6.2MB）
LOCAL_TARBALL=<path> HEARTH_BIN_DIR=/tmp/hbin sh install.sh
  → 从本地 tarball 安装 ... ✓ 已安装: /tmp/hbin/hearth（+ 别名 codex）
  → /tmp/hbin/hearth --version = hearth 0.1.0
无源调用（红线验证——不留 example.com 占位）：
  sh install.sh → die "未指定安装源。二选一：① 本地模式 LOCAL_TARBALL=... ② 远程模式 HEARTH_BASE_URL=..."
```

### R3 零配置

```
env -i HOME=$HOME hearth chat "hi"（无 key）
  → ✗ 未配置 API key。下一步（任选其一）：
    1. hearth config set api-key <你的key>
    2. export HEARTH_API_KEY=<你的key>
    3. hearth init（交互式首次引导）
config set 后：🔒 landlock+seccomp (fail-closed) (session ...) → 直跑
```

### R4 安全默认

```
grep HEARTH_SECCOMP_MODE crates/sandbox/src/lib.rs → 3 处（默认 KILL + errno 回退）
grep HEARTH_ALLOW_NO_CGROUP → 8 处（fail-closed + 显式降级）
README 安全段：landlock/seccomp(KILL)/cgroup(fail-closed) 三层 + 降级开关表 + VM delegation 指引
```

### R6 门禁

```
FMT_RC=0 / CLIPPY_RC=0 / TEST_RC=0（251 passed，--test-threads=2 全量）/ BUILD_RC=0
注：--test-threads=4 时 service integration test_p1_budget_exhausted_error 偶发失败
（RT4 已知 mock 时序竞态；--test-threads=2 全绿，单独跑恒 PASS——挂账待后续根治）
```

---

## 3. 交付清单

| 交付 | 路径 | 说明 |
|---|---|---|
| README | `README.md` | 安装/配置/安全/Observer/反馈 五段 + 快速开始 |
| 安装脚本 | `bench/install.sh` | LOCAL_TARBALL 本地模式 + HEARTH_BASE_URL 远程模式，**无 example.com 占位** |
| 预编译包 | `release/hearth-v0.1-x86_64-unknown-linux-gnu.tar.gz` | 6.2MB（hearth + codex + README.txt） |
| 验收报告 | 本文档 | 实测输出全记录 |

---

## 4. 用户真机验收路径（你来执行——v0.1 收口判据）

VM Linux（root 或完整 cgroup delegation 优先）：

```bash
# 1. 安装（二选一）
cargo install --path crates/codex-cli
# 或
LOCAL_TARBALL=release/hearth-v0.1-x86_64-unknown-linux-gnu.tar.gz sh bench/install.sh

# 2. 版本确认
hearth --version && codex --version      # → hearth 0.1.0

# 3. 零配置报错（应给可行动错误，不崩溃）
hearth chat "hi"                          # → 3 步下一步

# 4. 配 key 后直跑一个真实任务
hearth config set api-key <你的key>
hearth chat "用 Rust 写一个 add 函数并加测试" --budget 6
# → 应看到 🗺 规划草案 + ❓/🤖 + 产物落盘

# 5. 安全验证（可选但推荐）
# 观察 🔒 徽章；delegation 环境按 README 设 HEARTH_CGROUP_BASE / HEARTH_ALLOW_NO_CGROUP=1
```

**收口标准**：步骤 2/4 无报错跑通 + 产物真实落盘 = `hearth-v0.1` 真机验收 PASS。

---

## 5. 挂账

| 项 | 状态 | 说明 |
|---|---|---|
| service integration 并发偶发（budget 测试） | 挂账 | mock 时序竞态；--test-threads=2 全绿，单独跑恒 PASS |
| GitHub Release 远程安装 | 待发布 | install.sh 已支持 HEARTH_BASE_URL；发布后填真实地址即可 |
| Windows/macOS 安装包 | v0.2 候选 | 交叉编译 + MSI/NSIS/homebrew；非 Linux 无真实沙箱（noop 徽章） |
