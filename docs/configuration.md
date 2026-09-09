# Codex-Rust v4.0 配置参考

## 服务端口

| 环境变量 | 默认值 | 说明 |
|---|---|---|
| `PORT` | `3000` | HTTP 服务监听端口 |

## 鉴权

| 环境变量 | 默认值 | 说明 |
|---|---|---|
| `API_KEY` | —（开发模式） | 设为非空字符串启用 Bearer token 鉴权。生产部署**必须**设置 |

## LLM Provider

| 环境变量 | 默认值 | 说明 |
|---|---|---|
| `OPENAI_API_KEY` | — | OpenAI API 密钥（**必填**，无此变量服务拒绝启动） |
| `OPENAI_BASE_URL` | `https://api.openai.com/v1` | 兼容 OpenAI 协议的 API 地址 |
| `OPENAI_MODEL` | `gpt-4o` | 模型名称 |

## Embed Provider（用于 Retrieval）

| 环境变量 | 默认值 | 说明 |
|---|---|---|
| `EMBED_API_KEY` | 与 `OPENAI_API_KEY` 相同 | Embedding 服务的 API 密钥 |
| `EMBED_BASE_URL` | — | Embedding API 地址（缺省同 OPENAI_BASE_URL） |
| `EMBED_MODEL` | `text-embedding-ada-002` | Embedding 模型名称 |

## Retriever

| 环境变量 | 默认值 | 说明 |
|---|---|---|
| `RETRIEVER_ENABLED` | `0` | 设为 `1` 启用语义检索（需要 embed provider） |

## LSP Bridge

| 环境变量 | 默认值 | 说明 |
|---|---|---|
| `LSP_ENABLED` | `0` | 设为 `1` 启用真实 rust-analyzer 桥接（需 VM/容器内安装 `rust-analyzer`）。默认 NoopLspBridge（空诊断） |

## History / State

| 环境变量 | 默认值 | 说明 |
|---|---|---|
| `MEMORY_DIR` | 当前工作目录下的 `_memory_store` | JSONL 持久化存储目录 |
| `SESSION_TTL_SECS` | `3600`（1 小时） | Session 空闲超时（秒），超时后内存 session 被清理 |

## Sandbox

| 环境变量 | 默认值 | 说明 |
|---|---|---|
| `CODEX_SANDBOX_ENABLED` | `1` | 设为 `0` 禁用 sandbox（仅调试用，生产不应关闭） |
| `CODEX_SANDBOX_WORKSPACE` | 当前工作目录 | Sandbox workspace 根目录 |

## 其他

| 环境变量 | 默认值 | 说明 |
|---|---|---|
| `RUST_LOG` | `info` | 日志级别（trace/debug/info/warn/error） |

## 生产部署最小配置

```env
OPENAI_API_KEY=sk-xxxxxxxx
API_KEY=my-production-secret
PORT=3000
LSP_ENABLED=0      # 无需 LSP 时保持关闭
RETRIEVER_ENABLED=1  # 如需语义检索
```
