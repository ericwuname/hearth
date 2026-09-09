# Hearth CLI 使用指南

> 版本：2026-08-22（hearth-cli 任务书 D1-D5）| 正名 **Hearth**（docs/hearth-naming.md）
> 二进制：`hearth`（旧名 `codex` 为 deprecated 别名，逻辑相同）

## 1. 安装

```bash
# 从源码安装（推荐）
cd hearth-rs
cargo build --release -p codex-cli
install -m755 target/release/hearth ~/.cargo/bin/hearth

# 验证
hearth --help
```

## 2. 首次配置（三种方式任选）

```bash
# 方式一：交互式引导（推荐）
hearth init

# 方式二：config 子命令（写 ~/.config/hearth/config.toml）
hearth config set provider deepseek
hearth config set api-key sk-xxxxxxxx

# 方式三：环境变量（仅当前 shell 生效）
export HEARTH_API_KEY=sk-xxxxxxxx
export HEARTH_PROVIDER=deepseek
```

配置优先级（高→低）：**命令行参数 > 环境变量（`HEARTH_*`）> `~/.config/hearth/config.toml` > 内置默认**。

支持字段：`provider`（deepseek/openai/ollama/vllm）、`url`、`api-key`、`mode`（auto 直跑/remote 远程）、`feedback-prompt`（true/false）。

## 3. 日常用法

```bash
# 单次任务（进程内直跑内核，无需起 service）
hearth chat "写一个 Rust 函数 add(a,b) 返回 a+b，并写测试验证"

# 指定 provider / 步数预算
hearth chat "..." --provider deepseek --budget 20

# 连远程 service（可选）
hearth chat "..." --url http://localhost:3000

# 交互式 REPL（远程模式）
hearth --url http://localhost:3000 repl

# 查看配置
hearth config get            # 全部
hearth config get api-key    # 单项（打码显示）
```

## 4. 安全透明（隔离徽章）

启动时打印隔离状态：

- 🔒 `landlock+seccomp (fail-closed)` —— Linux 真实隔离
- ⚠️ `noop · 仅开发模式` —— 非 Linux（无真实沙箱，明确告知）

**fail-closed 语义**：seccomp/landlock 加载失败 → CLI 启动失败并报原因，**绝不"假装隔离"**。
工具执行在白名单内（默认 deny，99 个实测必需 syscall，见 `docs/seccomp-allowlist-v1.md`）。

## 5. 审批流程

agent 需要批准时（如澄清、高危操作）：

```
⛔ clarification — 批准? [y/N]
```

输入 `y` 批准 / `n` 拒绝（回车=拒绝）。审批在进程内直接喂回内核，无需额外命令。

## 6. Observer 素材（人类侧观察）

```bash
# 通用素材
hearth note "这个审批流程卡了我三次"

# 关联 session + 标注
hearth note "理解偏差" --session 123e4567 --self "我的表述有歧义"
hearth note "不该拒批那么多次" --observer-verdict n "Observer 误判了"
hearth note "很沮丧" --mood frustrated
```

落盘：`~/hearth/observer/human-<session>.jsonl`（人类侧）+ `~/hearth/observer/<session>.jsonl`（AI 侧）。

**零执行权**：Observer 只记/只回放/只建议，**绝不自动改你的配置或内核**。

`hearth chat` 开头轻探：仅当上次 session 有异常信号（高拒批/负面情绪）时出现；关闭：
`hearth config set feedback-prompt false`

## 7. 排错（错误可行动——每条报错都含"下一步"）

| 症状 | 原因 | 下一步 |
|---|---|---|
| `未配置 API key` | 无 key | `hearth config set api-key <key>` 或 `hearth init` |
| `未知 provider: xx` | provider 名错 | `hearth config set provider deepseek` |
| `vllm 需要 url` | vllm 未配地址 | `hearth config set url http://localhost:8000/v1` |
| 模型 4xx / 网络不可达 | key 无效 / 网络 | 检查 key 与网络；`hearth config get url` |
| 该命令需要 --url | 非 chat 命令在 auto 模式 | `hearth chat "..." --url http://localhost:3000` |

## 8. 架构（一句话）

```
hearth CLI ──进程内──→ agent-runtime（会话/loop/LLM/沙箱）──→ 内核 AgentLoop
                          ↑
service = agent-runtime 的 HTTP 包装层（保留独立部署能力）
```

红线：不动 `docs/ai-os-event-contract-v1.md` 事件契约；沙箱 fail-closed 不可降级；Observer 零执行权。
