# Hearth 配置参考

> **本文件的契约（D-89 起由门禁强制）**：用**反引号**标出的环境变量即**现役旋钮**，
> 必须在代码里真的被 `env::var` 读取——`crates/codex-cli/tests/config_doc_gate.rs`
> 会逐项核对，列出"文档有、代码无"的**幽灵旋钮**即门禁红。
> 因此：**不要**在本文件里用反引号写"计划中/已删除能力"的变量名（见文末《已失效》节）。

## 服务端口

| 环境变量 | 默认值 | 说明 |
|---|---|---|
| `PORT` | `3000` | HTTP 服务监听端口 |

## 鉴权

| 环境变量 | 默认值 | 说明 |
|---|---|---|
| `API_KEY` | —（开发模式） | 设为非空字符串启用 Bearer token 鉴权。生产部署**必须**设置（P0-1：无 key 时**默认拒绝启动**，fail-closed） |

## LLM Provider

| 环境变量 | 默认值 | 说明 |
|---|---|---|
| `OPENAI_API_KEY` | — | OpenAI 兼容通道的 API 密钥（无此变量服务拒绝启动） |
| `OPENAI_BASE_URL` | `https://api.openai.com/v1` | 兼容 OpenAI 协议的 API 地址 |
| `OPENAI_MODEL` | `gpt-4o` | 模型名称 |

## History / State

| 环境变量 | 默认值 | 说明 |
|---|---|---|
| `MEMORY_DIR` | `./memory` | JSONL 持久化目录（会话归档 / 经验库 / 文明线同目录） |

## Session 生命周期（OOM-1 / P0-04 / P0-05）

| 环境变量 | 默认值 | 说明 |
|---|---|---|
| `OOM_TTL_SECS` | `3600`（1 小时） | 会话空闲超时（秒）——超时后内存 session 与其 workspace 目录被回收 |
| `OOM_SWEEP_INTERVAL_SECS` | `300`（5 分钟） | 后台回收扫描周期（秒） |

> ⚠️ **D-89 订正**：旧文档把上面第一项写作 `SESSION_TTL_SECS`。**那个名字从未实现**——
> 真实旋钮是 `OOM_TTL_SECS`（默认同为 3600）。照旧文档设置的变量**不会有任何效果**。

## 其他

| 环境变量 | 默认值 | 说明 |
|---|---|---|
| `RUST_LOG` | `info` | 日志级别（trace/debug/info/warn/error）；由 `tracing-subscriber` 读取（本仓代码不直接读） |

## 已失效的环境变量（**不要再配置**）

下表**故意不用反引号**——它们**不是**现役旋钮。设置它们没有任何效果；列在这里只是
为了让既有部署能查到自己一直在设的是**死旋钮**（逐项取证见 D-89 与驱动文档债队列）：

| 失效变量名（纯文本，非现役） | 原意 | 现状 |
|---|---|---|
| RETRIEVER_ENABLED | 启用语义检索 | 检索链已随 P1-04（顶层裁决「删除」）整删 |
| LSP_ENABLED | 启用 rust-analyzer 桥接 | LSP 桥已随 P1-04 整删 |
| EMBED_API_KEY / EMBED_BASE_URL / EMBED_MODEL | Embedding 服务 | embedding 注入已随 v21.0 移除（经验库现为关键词检索） |
| CODEX_SANDBOX_ENABLED | 关闭 sandbox | **从未实现**：`sandbox::create_sandbox` 只按平台选择（Linux→LinuxSandbox，其余→NoopSandbox），**不存在** env 开关；生产不该提供"一键关沙箱" |
| CODEX_SANDBOX_WORKSPACE | 指定 sandbox 根目录 | **从未实现**：workspace 根按会话派生 |
| SESSION_TTL_SECS | 会话空闲超时 | **名字写错**——真名是 `OOM_TTL_SECS`（见上节） |

## 生产部署最小配置

```env
OPENAI_API_KEY=sk-xxxxxxxx
API_KEY=my-production-secret
PORT=3000
# 可选：调小会话空闲回收窗口（默认 1 小时）
# OOM_TTL_SECS=900
# OOM_SWEEP_INTERVAL_SECS=60
```
