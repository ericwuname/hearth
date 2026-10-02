# Hearth 配置参考

> **本文件的契约（D-89 起由门禁强制）**：用**反引号**标出的环境变量即**现役旋钮**，
> 必须在代码里真的被 `env::var` 读取——`crates/codex-cli/tests/config_doc_gate.rs`
> 会逐项核对，列出"文档有、代码无"的**幽灵旋钮**即门禁红。
> 因此：**不要**在本文件里用反引号写"计划中/已删除能力"的变量名（见文末《已失效》节）。
>
> **覆盖范围（D-93 起明确）**：本文件覆盖 **service（HTTP 服务）与内核** 的环境旋钮
> ——即运维部署一份 hearth 需要知道的那批。CLI 直跑模式的其余旋钮见 `hearth --help`
> 与 `~/.config/hearth/config.toml`（`hearth config get` 可打印）。
> 默认值一栏以代码为准；`—` 表示默认未设置。
>
> **`.env` 加载（D-106 起）**：**service 与 CLI 都会自动加载工作目录下的 `.env`**
> （`dotenvy`，从 cwd 向上查找；仓库模板见 `.env.example`，`.env` 已被 `.gitignore` 忽略）。
> 优先级：**已存在的进程环境变量 > `.env`**（dotenvy 不覆盖），命令行参数优先级最高。
>
> **provider 惯例名对 CLI 同样生效（D-117 起）**：下表这批 `<PROVIDER>_API_KEY` /
> `_BASE_URL` / `_MODEL`（AGNES / DEEPSEEK / OPENAI / GEMINI / OLLAMA / VLLM）**CLI 直跑
> 也读取**——此前 CLI 只认 `HEARTH_*`，照 `.env.example` 配好却报"未配置 API key"。
> CLI 侧的完整优先级：**命令行参数 > `HEARTH_*`（`HEARTH_API_KEY`/`HEARTH_MODEL`/
> `HEARTH_LLM_URL`/`HEARTH_PROVIDER`）> `<PROVIDER>_*` > `config.toml`**。
> 注意 CLI 默认 provider 是 `deepseek`；配了哪个通道就把 `HEARTH_PROVIDER` 指到它。

## 服务端口

| 环境变量 | 默认值 | 说明 |
|---|---|---|
| `PORT` | `3000` | HTTP 服务监听端口 |

## 鉴权与跨域

| 环境变量 | 默认值 | 说明 |
|---|---|---|
| `API_KEY` | —（开发模式） | 设为非空字符串启用 Bearer token 鉴权。生产部署**必须**设置（P0-1：无 key 且未显式放行时**默认拒绝启动**，fail-closed） |
| `ALLOW_NO_AUTH` | 未设（关） | 设 `1`/`true` 才允许无鉴权启动，且该模式下服务**只绑 127.0.0.1**。仅限本机开发 |
| `CORS_ORIGIN` | `http://localhost:5173` | 允许的浏览器来源（单个）。方法与请求头是**固定白名单**（GET/POST/OPTIONS + Content-Type/Authorization），不受本项影响 |

## LLM Provider（多后端）

**注册规则（D-96）**：每个 provider **仅在其必需旋钮存在且非空时**才注册；
未注册的 provider 不会出现在 `GET /api/v1/models`，客户端请求它会得到明确的
"provider not found"，而不是"能选中但一调用就 401"。唯一例外是 OpenAI——
它是服务启动的硬前置，无 key 直接拒绝启动。

| 环境变量 | 默认值 | 说明 |
|---|---|---|
| `OPENAI_API_KEY` | —（必填） | OpenAI 兼容通道密钥；**无此变量服务拒绝启动**（fail-closed） |
| `OPENAI_BASE_URL` | `https://api.openai.com/v1` | 兼容 OpenAI 协议的 API 地址 |
| `OPENAI_MODEL` | `gpt-4o` | 模型名称 |
| `DEEPSEEK_API_KEY` | — | DeepSeek 密钥（同时驱动 `deepseek` 与 `deepseek-pro` 两个 provider） |
| `DEEPSEEK_BASE_URL` | `https://api.deepseek.com/v1` | |
| `DEEPSEEK_MODEL` | `deepseek-v4-flash` | |
| `DEEPSEEK_PRO_MODEL` | `deepseek-v4-pro` | provider `deepseek-pro` 所用模型（同通道换大模型的天花板对照） |
| `ZHIPU_API_KEY` | — | 智谱 GLM 密钥（同时驱动 `zhipu` 与 `zhipu-max`） |
| `ZHIPU_BASE_URL` | `https://open.bigmodel.cn/api/paas/v4` | |
| `ZHIPU_MODEL` | `glm-4.5-air` | |
| `ZHIPU_MAX_MODEL` | `glm-4.7` | provider `zhipu-max` 所用模型 |
| `GEMINI_API_KEY` | — | Google Gemini 密钥 |
| `GEMINI_BASE_URL` | `https://generativelanguage.googleapis.com/v1beta/openai` | OpenAI 兼容端点 |
| `GEMINI_MODEL` | `gemini-3.6-flash` | |
| `AGNES_API_KEY` | — | Agnes AI 密钥 |
| `AGNES_BASE_URL` | `https://api.agnes-ai.cn/v1` | |
| `AGNES_MODEL` | `agnes-2.5-flash` | |
| `DOUBAO_API_KEY` | — | 豆包（火山方舟）密钥 |
| `DOUBAO_BASE_URL` | `https://ark.cn-beijing.volces.com/api/v3` | |
| `DOUBAO_MODELS` | 5 个模型的逗号列表 | 一个 key 注册多个 provider 做轮换，首个登记为 `doubao`，其余为 `doubao-<前缀>` |
| `HUNYUAN_API_KEY` | — | 腾讯混元密钥 |
| `HUNYUAN_BASE_URL` | —（用厂商默认） | |
| `HUNYUAN_MODEL` | `hunyuan-pro` | |
| `OLLAMA_BASE_URL` | — | 本地 Ollama 地址；**设了才注册**（免 key） |
| `OLLAMA_MODEL` | `qiyuan-8b:latest` | |
| `VLLM_BASE_URL` | — | 本地 vLLM 地址；**设了才注册** |
| `VLLM_MODEL` | `mistral-7b` | |
| `VLLM_API_KEY` | —（可省） | 多数本地 vLLM 不校验鉴权 |
| `FALLBACK_CHAIN` | 未设（关） | 组装降级链并注册为 provider `fallback`。写 `1`/`true` = 取全部已注册 provider 按名字排序；写 `a,b,c` = 按给定顺序，**第一个即主通道**。链中未注册的名字会 warn 跳过（D-97：顺序显式化，此前取哈希序等于随机） |
| `REPLAY_DIR` | — | 确定性回放 provider（bench 专用）：从该目录加载录制会话，注册为 `replay`。**生产不要设置** |

## 服务侧目录与后台任务

| 环境变量 | 默认值 | 说明 |
|---|---|---|
| `MEMORY_DIR` | `./memory` | JSONL 持久化目录（会话归档 / 经验库 / 文明线同目录） |
| `TOOLS_DIR` | `./tools` | 工具注册表启动扫描目录：每个子目录放一份 `manifest.toml` 即被登记，可在 `GET /api/v1/tool-registry` 看到 |
| `CODEX_TEMPLATES_DIR` | `./templates` | Agent 提示模板目录（`GET /api/v1/templates` 读它） |
| `CODEX_OBSERVER_DIR` | `./observer` | Observer 落盘目录：每小时追加 `daily-<日期>.jsonl`，会话结束写 `reports/<sid>/` |

## 内核行为

| 环境变量 | 默认值 | 说明 |
|---|---|---|
| `HEARTH_MAX_STEPS` | `50` | 单任务最大步数（预算上限）。非法值/`0` 回落 50——护栏不得被配没 |
| `HEARTH_EGRESS_ALLOWLIST` | 未设（空） | 逗号分隔的出网 host 白名单；bash 出网与 webhook 投递共用同一份 |
| `HEARTH_TASK_TIMEOUT_SECS` | `900`（15 分钟） | **CLI 直跑**的单任务墙钟上限（秒，与步数预算是两层独立护栏：步数管轮次、墙钟管时间）。`0` = **显式关闭**（不限时，会打印一行风险提示）；未设/非法值回落 900 |

## 沙箱（仅 Linux 生效；非 Linux 为 noop）

| 环境变量 | 默认值 | 说明 |
|---|---|---|
| `HEARTH_CGROUP_BASE` | `/sys/fs/cgroup` | cgroup v2 基目录。容器 / 非 root 部署应指向**可写的 delegation 子树**，否则 cgroup 限制建不起来 |
| `HEARTH_ALLOW_NO_CGROUP` | 未设（关） | 设 `1`/`true` 显式接受"进程无内存/CPU/进程数上限"的降级；未设时 cgroup 不可用即 fail-closed 报错、不开工 |
| `HEARTH_SECCOMP_MODE` | 未设（杀进程） | 设 `errno` 时白名单外的系统调用返回 EPERM 而不是终止进程——**排障用**，会削弱 fail-closed，勿用于生产 |

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
为了让既有部署能查到自己一直在设的是**死旋钮**（逐项取证见 D-89/D-95 与驱动文档债队列）：

| 失效变量名（纯文本，非现役） | 原意 | 现状 |
|---|---|---|
| RETRIEVER_ENABLED | 启用语义检索 | 检索链已随 P1-04（顶层裁决「删除」）整删 |
| LSP_ENABLED | 启用 rust-analyzer 桥接 | LSP 桥已随 P1-04 整删 |
| EMBED_API_KEY / EMBED_BASE_URL / EMBED_MODEL | Embedding 服务 | embedding 注入已随 v21.0 移除（经验库现为关键词检索） |
| CODEX_SANDBOX_ENABLED | 关闭 sandbox | **从未实现**：`sandbox::create_sandbox` 只按平台选择（Linux→LinuxSandbox，其余→NoopSandbox），**不存在** env 开关；生产不该提供"一键关沙箱" |
| CODEX_SANDBOX_WORKSPACE | 指定 sandbox 根目录 | **从未实现**：workspace 根按会话派生 |
| SESSION_TTL_SECS | 会话空闲超时 | **名字写错**——真名是 `OOM_TTL_SECS`（见上节） |
| PROVIDERS_PATH | 指向 providers.json 做"模型自动发现" | **已退役**（D-95）：那套"发现"只打日志、解析结果被丢弃，发现的模型既不注册也不影响任何路由；且默认清单里的 anthropic 本仓无实现。现役 provider 全部由上述 `<PROVIDER>_*` env 显式注册 |

## 生产部署最小配置

```env
OPENAI_API_KEY=sk-xxxxxxxx
API_KEY=my-production-secret
PORT=3000
# 可选：调小会话空闲回收窗口（默认 1 小时）
# OOM_TTL_SECS=900
# OOM_SWEEP_INTERVAL_SECS=60
```

> **注意**：`SessionCreate.provider` 是**必填**字段——服务端没有"默认 provider"这个概念
> （D-94：`ProviderRegistry` 的 default 语义已删）。客户端必须显式指定 `provider`
> （如 `openai`、`deepseek`），且该 provider 必须已按上表配好旋钮，否则返回 not-found。
