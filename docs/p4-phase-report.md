# P4 阶段报告 v2：多后端 LLM（llm-local + llm-cn + 网关降级/记账）

**日期**: 2025-01  
**阶段**: P4 — Multi-Backend LLM Providers  
**状态**: ? 全部通过（v2 修复完成）

---

## 1. 阶段概述

P4 实现 Codex Rust 的第四个核心阶段：多后端 LLM 支持。包括：
- **llm-local crate**: OllamaProvider（原生协议）+ VllmProvider（OpenAI 兼容 thin wrapper）
- **llm-cn crate**: HunyuanProvider（腾讯混元，OpenAI 兼容 + 特有错误格式）
- **FallbackChain**: 降级链，primary 失败 → 自动切 fallback，有界重试
- **CostMeter**: session-scoped token 计费累加器，按 (provider, model) 维度记录

---

## 2. D2 数据

### 2.1 代码量 (grep 实测)

| Crate | LOC | v1→v2 变化 |
|-------|-----|------------|
| `agent-types` | 447 | 0 |
| `llm-gateway` | 762 | 0 |
| `llm-openai` | 580 | 0 |
| `llm-local` | 1,366 | 0 |
| `llm-cn` | 774 | 0 |
| `tool-runtime` | 366 | 0 |
| `tools-builtin` | 637 | 0 |
| `sandbox` | 484 | 0 |
| `agent-core` | 1,852 | 0 |
| `api` | 133 | 0 |
| `service` | 738 | +126 |
| `code-index` | 514 | 0 |
| `retriever` | 493 | 0 |
| `lsp-bridge` | 106 | 0 |
| `planner` | 505 | 0 |
| **全 workspace (src/)** | **9,757** | +126 |

### 2.2 测试统计

| Crate | 测试数 | v1→v2 变化 |
|-------|--------|------------|
| `agent-types` | 9 | 0 |
| `llm-gateway` | 19 | 0 |
| `llm-openai` | 5 | 0 |
| `llm-local` | 11 | 0 |
| `llm-cn` | 6 | 0 |
| `tool-runtime` | 7 | 0 |
| `tools-builtin` | 16 | 0 |
| `sandbox` | 7 | 0 |
| `agent-core` | 13 | 0 |
| `api` | 3 | 0 |
| `service` (unit) | 4 | 0 |
| `service` (integration) | 9 | +1 |
| `code-index` | 4 | 0 |
| `retriever` | 4 | 0 |
| `lsp-bridge` | 2 | 0 |
| `planner` | 4 | 0 |
| **总计** | **123** | +1 |

### 2.3 编译与测试

```
$ cargo test --workspace
# 全部 123 个测试通过，0 失败
```

---

## 3. A1–A6 证据矩阵

### A1 — llm-local crate

| 证据 | 描述 |
|------|------|
| **OllamaProvider** | 实现全部 6 个 LlmProvider 方法，使用 Ollama 原生 /api/chat + /api/embeddings 协议 |
| **VllmProvider** | 独立 struct（非 pub use 空壳），OpenAI 兼容 thin wrapper，支持可选 API key |
| **Wire tests (8)** | test_ollama_chat_wire_format / test_ollama_chat_with_tools_wire_format / test_ollama_embed_wire_format / test_ollama_chat_error_status / test_vllm_chat_wire_format / test_vllm_chat_with_auth / test_vllm_embed_wire_format / test_vllm_chat_error_status |
| **Unit tests (3)** | test_ollama_provider_creation / test_vllm_provider_creation / test_vllm_provider_no_auth |
| **Mock server** | axum Router + TcpListener::bind("127.0.0.1:0") 随机端口，零外部依赖 |

### A2 — llm-cn crate

| 证据 | 描述 |
|------|------|
| **HunyuanProvider** | 独立实现全部 6 个 LlmProvider 方法；embed 正确返回 unsupported 错误 |
| **Hunyuan 特有错误解析** | try_extract_hunyuan_error() 解析 Response.Error.Message 格式（不同于 OpenAI 的 error.message） |
| **Wire tests (4)** | test_hunyuan_chat_wire_format / test_hunyuan_chat_with_tools / test_hunyuan_error_hunyuan_format / test_hunyuan_error_generic_format |
| **Unit tests (2)** | test_hunyuan_provider_creation / test_hunyuan_embed_unsupported |
| **Capabilities 诚实** | embeddings=false，embed() 如实报错，未虚报 |

### A3 — 三后端切换测试（v2 修复）

| 证据 | 描述 |
|------|------|
| **test_p4_three_backend_switch** | 三会话分别使用 ollama/vllm/hunyuan 三个 HitMock provider |
| **行为验收** | 三个 session 全部到达 Done（ok:true），事件序列一致（各 12 events） |
| **调用验证** | 每个 HitMock 的 hit flag 均被设为 true —— 证明三个 provider 都真实收到 chat() 调用 |
| **非类型检查** | v1 的"编译期保证合约一致"已替换为真实验收 |

### A4 — FallbackChain

| 证据 | 描述 |
|------|------|
| **FallbackChain** | chat() 按顺序尝试 provider，成功即返回 (provider_name, response)，全部失败返回最后错误 |
| **有界重试** | 链长度 = 重试上限，无无限循环 |
| **Stream 策略** | 仅使用 primary provider（mid-stream fail 无法透明重试） |
| **生产接线** | main.rs 中 env FALLBACK_CHAIN=1 开关激活，缺省不启用 |
| **Tests (7)** | test_primary_succeeds / test_fallback_after_primary_fails / test_all_fail / test_empty_chain / test_embed_fallback / test_chain_metadata / test_stream_uses_primary |

### A5 — CostMeter

| 证据 | 描述 |
|------|------|
| **CostMeter 本体** | record(provider, model, usage) 按 provider/model key 累加 |
| **查询接口** | get() / total_tokens() / total_prompt_tokens() / total_completion_tokens() / total_calls() / iter() / reset() |
| **生产接线** | SessionManager 持有 Arc<Mutex<CostMeter>>；send_message 完成后自动 record |
| **Tests (6)** | test_record_single / test_record_multiple_same_provider / test_record_multiple_providers / test_get_missing / test_reset / test_iter |

### A6 — 回归 + 抽象纯度

| 证据 | 描述 |
|------|------|
| **全量回归** | 123/123 测试通过，0 失败 |
| **Frozen crates** | 13 个 crates（含 llm-local/llm-cn/llm-gateway/src/fallback.rs/cost.rs）零修改 |
| **LlmProvider trait 签名** | 6 方法签名未变，与 P3 基线一致 |
| **P0–P3 集成测试** | 8/8 通过（含 P0 A1/A3、P1 budget/tools/multi-turn、P2 LSP/retrieval） |

---

## 4. 生产接线（v2 新增）

### 4.1 main.rs — 三后端 env 注册

```
// 1. OpenAI (always registered — primary)
registry.register(openai, true);

// 2. Ollama (optional — skip with warn if OLLAMA_BASE_URL not set)
if let Ok(base_url) = std::env::var("OLLAMA_BASE_URL") { ... }

// 3. vLLM (optional — skip with warn if VLLM_BASE_URL not set)
if let Ok(base_url) = std::env::var("VLLM_BASE_URL") { ... }

// 4. Hunyuan (optional — skip with warn if HUNYUAN_API_KEY not set)
if let Ok(api_key) = std::env::var("HUNYUAN_API_KEY") { ... }
```

| 环境变量 | 作用 | 缺省行为 |
|----------|------|---------|
| `OPENAI_API_KEY` | OpenAI API key | warn + placeholder |
| `OPENAI_BASE_URL` | OpenAI 自定义 base URL | 官方 API |
| `OLLAMA_BASE_URL` | Ollama 服务地址 | warn + skip |
| `OLLAMA_MODEL` | Ollama 模型名 | llama3.2 |
| `VLLM_BASE_URL` | vLLM 服务地址 | warn + skip |
| `VLLM_MODEL` | vLLM 模型名 | mistral-7b |
| `VLLM_API_KEY` | vLLM API key（可选） | 无认证 |
| `HUNYUAN_API_KEY` | 混元 API key | warn + skip |
| `HUNYUAN_MODEL` | 混元模型名 | hunyuan-pro |
| `HUNYUAN_BASE_URL` | 混元自定义 base URL | 官方 API |
| `FALLBACK_CHAIN` | 启用降级链（1/true） | 不启用 |

### 4.2 session.rs — CostMeter 记账

- SessionManager 持有 Arc<Mutex<CostMeter>>
- send_message() 在 agent 完成后调用 cost_meter.record(provider, model, usage)
- 通过 SessionManager::cost_meter() 可外部查询累计用量
- list_providers() 方法暴露 registry 中所有注册的 provider（供 /api/v1/models 使用）

### 4.3 routes.rs — list_models 动态化

GET /api/v1/models 不再返回静态 ["openai"]，而是通过 SessionManager::list_providers() 返回所有已注册 provider。

---

## 5. v2 修复记录

| # | 问题 | 修复 | LOC | 证据 |
|---|------|------|-----|------|
| ?① | A3 三后端切换测试缺失 | 新增 test_p4_three_backend_switch：三个 HitMock provider，断言全部 Done + hit flag | +105 (integration_test.rs) | 测试通过，三个 session 各 12 events，三个 provider 均被调用 |
| ?② | 生产接线全缺 | main.rs 按 env 注册 ollama/vllm/hunyuan + FallbackChain env 开关；session.rs 挂 CostMeter + list_providers；routes.rs list_models 动态化 | +126 (main.rs/session.rs/routes.rs) | grep 确认 llm-local/llm-cn/llm-gateway 符号存在于 main.rs；CostMeter 在 send_message 完成后 record |

### 变更范围

- **仅 service crate 变更**（4 文件：main.rs, session.rs, routes.rs, integration_test.rs）
- **13 个 frozen crates 零修改**（含 llm-local/llm-cn/llm-gateway/src/fallback.rs/cost.rs）
- **Cargo.toml/Cargo.lock** 仅新增 service 对 llm-local/llm-cn 的依赖

---

## 6. P5 承接项

| 项目 | 描述 |
|------|------|
| landlock 沙箱 | 替换当前 noop/linux 沙箱为 Landlock LSM |
| 真实 LSP bridge | 替换 noop bridge 为真实 tower-lsp 实现 |
| A4 flaky observation | 子 agent 结果合并的观察时序一致性 |
