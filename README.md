# Hearth CLI（v0.1 可装基线）

> 频率共振 · 安稳之地 · 放心走开——Hearth 是一个**单二进制、开箱即用**的本地 AI 代理 CLI。
> 安装后打开终端输入即用：`hearth chat "用 Rust 写一个 add 函数并加测试"`。

Hearth 的设计与决策见 [`docs/hearth-cli-design.md`](docs/hearth-cli-design.md)（本文档与其口径一致）。

---

## 1. 安装

### 方式 A：源码安装（开发/验证，功能一致）

```bash
cargo build --release -p codex-cli
install -m755 target/release/hearth ~/.cargo/bin/hearth
install -m755 target/release/codex ~/.cargo/bin/codex   # 别名（可选）
```

### 方式 B：预编译安装（Linux x86_64，最快）

```bash
# ① 本地 tarball（当前 v0.1 交付模式）
LOCAL_TARBALL=/path/to/hearth-v0.1-x86_64-unknown-linux-gnu.tar.gz sh install.sh

# ② 远程 tarball（发布到公网后）
HEARTH_BASE_URL=https://<host>/hearth/releases/download sh install.sh
```

- 默认装到 `/usr/local/bin`；`HEARTH_BIN_DIR=<dir>` 可改。
- 安装脚本在 `bench/install.sh`，**不留占位地址**——未指定安装源会报错引导，绝不静默下载。
- 平台支持：当前仅 Linux x86_64 有预编译产物；其他平台（Windows/macOS）引导源码安装，见 §5 跨平台说明。

### 验证安装

```bash
hearth --version   # → hearth 0.1.2 (83fbf9d)   —— 版本 + commit hash（R3）
codex --version    # → hearth 0.1.2 (83fbf9d)
```

> **不要用 `strings /usr/local/bin/hearth | grep <函数名>` 验证版本**——release 二进制符号被
> strip，Rust 标识符不在字符串表，`strings` 查不到 ≠ 没修复（v0.1.1 用户踩坑）。
> 正确姿势：看 `--version` 的 commit hash + 跑一个 hello.py 短任务看 write_file 是否成功。

---

## 2. 配置（三层优先级）

| 优先级 | 来源 | 示例 |
|---|---|---|
| 1（最高） | 命令行参数 | `hearth chat --provider deepseek --api-key sk-...` |
| 2 | 环境变量 | `export HEARTH_API_KEY=sk-...` |
| 3 | 配置文件 | `~/.config/hearth/config.toml` |
| 4 | 内置默认 | provider=deepseek, model=deepseek-v4-flash |

配置文件即真相源（不进 git、可备份）；CLI 子命令 = 改文件的界面。

```bash
hearth init                              # 交互式首次引导（provider / api-key / url 三问）
hearth config set api-key <key>          # 写 ~/.config/hearth/config.toml
hearth config set provider deepseek
hearth config get api-key                # 读取
```

可改字段：`api_key` / `provider` / `url` / `model` / `mode` / `feedback-prompt`。

**零配置首跑**：不设 key 直接 `hearth chat "hi"` → 清晰报错（非崩溃、非静默）：
给出 3 步下一步（`hearth config set api-key` / `export HEARTH_API_KEY` / `hearth init`）。

---

## 3. 安全（默认 ON，可验证）

> 设计原则：**不静默降级**——安全限制不可用要么显式报错，要么你明确选择降级。

### 3.1 隔离状态徽章（每次启动打印）

```
🔒 sandbox: linux · landlock + seccomp (fail-closed)   # Linux 真隔离
⚠️ sandbox: noop · 仅开发模式，无真实隔离               # 非 Linux（Windows/macOS）
```

### 3.2 三层纵深（Linux）

| 层 | 默认 | 说明 |
|---|---|---|
| **landlock** | ON | 文件系统权限裁剪：只读 `read_only_paths`、可写 `writable_paths` |
| **seccomp** | **KILL 化** | 白名单（106 个）外 syscall → 直接 SIGSYS 杀子进程（"漏网即死"） |
| **cgroup v2** | **fail-closed** | 内存/CPU/进程数硬上限（默认 512MB / 1.0s / 256）；不可用 → 报错不静默 |

### 3.3 环境变量开关（显式降级，非静默）

| 变量 | 作用 |
|---|---|
| `HEARTH_SECCOMP_MODE=errno` | 调试：白名单外返回 EPERM（拒绝但**不杀**），便于看被拦的 syscall |
| `HEARTH_CGROUP_BASE=<子树>` | 指向可写的 cgroup delegation 子树（VM 非特权环境需要） |
| `HEARTH_ALLOW_NO_CGROUP=1` | 显式接受"无资源限制"（安全边界 landlock/seccomp 仍生效） |

**VM 非特权 delegation 环境**（如 Ubuntu VM 默认）：
cgroup 需要 root 或完整 delegation 才真正生效；VM 下按 README 指引设 `HEARTH_CGROUP_BASE` 或 `HEARTH_ALLOW_NO_CGROUP=1`（这是**你明确选择**的降级，不是偷偷关掉）。

### 3.4 无 key / 认证错误

- 无 key：可行动报错（见 §2 零配置首跑）。
- 401/403：**立即失败**（不重试浪费 14s）+ 附"下一步"（config set api-key / export）。

---

## 4. Observer（第三权 · 零执行权）

> 用户原话："CLI 运行中出现的问题——理解偏差、思考缺失等——能否收集成查缺补漏的素材？"

**Observer 只看事件流，不改任何行为**（零执行权）；素材落盘 `~/hearth/observer/`：

| 侧 | 通道 | 落盘 |
|---|---|---|
| AI 侧（自动） | 事件流（plan_draft/tool_call/need_approval/...） | `<sid>.jsonl` |
| 人类侧（主动） | `hearth note` | `human-<sid>.jsonl` |
| 反审（主动） | `hearth note --observer-verdict n "理由"` | `rebuttals/<sid>.jsonl` |

```bash
hearth note "它把配置当成了可写路径，很危险"                        # 纯观察
hearth note --session s1 --self "理解偏差" "我的表述有歧义"          # 自我标注
hearth note --session s1 --mood frustrated "审批流程卡了我三次"      # 情绪标注
hearth note --session s1 --observer-verdict n "Observer 判错了"      # 反审（只落盘不改规则）
```

---

## 5. 反馈通道（主动 · 零打扰 · 可关）

- **主通道 `hearth note`**：随时敲一行（见 §4），正常流程完全无感。
- **辅通道：开头轻探**：仅当**上次 session 有异常信号**（重试率高 / give_up / 高拒批）时，
  下次 `hearth chat` 开头轻问一句"上次有要补充的吗？"；正常 session 绝不弹。
- **彻底关闭**：`hearth config set feedback-prompt false`。

原则：**系统不催你填表**——只有自己看出"上次可能不对"时才轻轻探一下，且随时可关。

---

## 6. 跨平台（v0.1 范围）

- **Linux x86_64**：完整安全纵深（landlock/seccomp/cgroup）+ 预编译包。
- **Windows / macOS**：代码分支保留（`cfg(target_os)` 平台分支 + noop 徽章），
  交叉编译产物与安装包（MSI/NSIS/homebrew）列为 v0.2 候选；**非 Linux 无真实沙箱**，
  启动即打 ⚠️ noop 徽章，绝不假装隔离。

---

## 快速开始

```bash
# 1. 安装（见 §1）→ 2. 配 key（见 §2）→ 3. 直跑
hearth chat "用 Rust 写一个 add 函数并加测试" --budget 6
# → 🗺 规划草案（steps=… gaps=… 待问=… 自动假设=…）
#   ❓ 要问你 [ambiguous_option]: 目标含未澄清技术选项…
#   🤖 ⚡ 假设[missing_goal_source]: …
# → 产物真实落盘
```

CLI 直跑无需起 service、无需 .env——`HEARTH_API_KEY` 一个 env 或 `hearth config set api-key` 即跑。
