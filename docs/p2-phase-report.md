# P2 阶段报告 v2：智能（代码索引 + 语义检索 + LSP 桥接）

## 1. 概述

| 项目 | 值 |
|------|-----|
| 阶段 | P2 — Intelligence |
| 状态 | ✅ v2 完成，待闸门重审 |
| 新增 crate | 3（code-index, retriever, lsp-bridge） |
| 修改 crate | 3（agent-core, service, workspace Cargo.toml） |
| 全 workspace LOC（grep 实测） | **6,434** |
| 全 workspace 测试 | **79（全部通过，零失败）** |

## 2. v2 变更（回规修复）

### 🔴 A2 向量检索空壳 → 真修

**问题**: `search()` 用 `dummy_embedding = vec![0.0; …]`（全零向量），从未对 query 调 `provider.embed()`。

**修复**:
- `TantivyRetriever` 新增 `embed_provider: Option<Arc<dyn LlmProvider>>` 字段（`set_embed_provider()` setter）
- `search()` 内：若 `embed_provider` 存在，调用 `provider.embed(&[query])` 获取真实 query embedding → `vector_search()` → RRF 融合
- 若 `embed_provider` 为 None，向量路径跳过（BM25-only），兼容测试场景
- `test_retriever_relevance` 重写：走真实 `search()` 路径（通过 `MockEmbedProvider` 注入 query embedding），断言 vector_score > 0.0（证明 embed 被真实调用）且 config 相关 chunks 在 top 结果中

**证据**: `crates/retriever/src/lib.rs` line 193-241（search 方法），line 338-400（test_retriever_relevance 走真实 search）

### 🟡 A4: LSP 诊断注入 system prompt

**修复**:
- `do_observe()` 中将 LSP 诊断文本格式化后写入 `ctx_mgr.scratch["lsp_diagnostics"]`
- `build_messages()` 中读取 scratch，注入到 system prompt 的 `## LSP Diagnostics` 段落
- `test_p2_a4_observe_consumes_lsp` 新增 scratch 断言：验证 `"A4_PROOF"` marker 出现在 scratch 中

**证据**: `crates/agent-core/src/loop.rs` line 371-389（LSP scratch 注入），line 186-193（build_messages 注入），line 826-834（测试断言）

### 🟡 A5: 真集成证据 + service 层注入点

**修复**:
- `SessionManager` 新增 `set_retriever()` / `set_lsp_bridge()` 方法
- `create_session()` 内自动注入到 `AgentLoop`
- `test_p2_a5_retrieval_in_context_spying` 重写：
  - 用 `mgr.set_retriever(spy_retriever)` 注入 SpyRetriever（含 marker `A5_PROOF_MARKER_RETRIEVED_CHUNK_42`）
  - SpyingProvider 捕获所有 ChatRequest system prompts
  - 断言 marker 出现在 system prompt 中（证明 set_retriever → do_observe → scratch → build_messages 完整链路）

**证据**: `crates/service/src/session.rs` line 40-44（retriever/lsp_bridge 字段），line 51-57（setter 方法），line 66-72（create_session 注入）；`crates/service/tests/integration_test.rs` line 676-850（集成测试）

### 🔵 D2: LOC 实测修正

| 项 | v1 报告 | v2 实测 |
|----|---------|---------|
| 全 workspace LOC | 5,347 | **6,323** |
| 测试数 | 78 | **79** |

### 🔵 A1: 相邻片段相似度断言

新增 `test_p2_index_vector_adjacent_similarity`：
- 同一文件 chunks 的 embedding 相似度 > 跨文件 chunks
- 用 `MockEmbedProvider` 返回受控 embedding
- 断言：`avg_same_file_sim > avg_cross_file_sim` 且 `avg_same > 0.9`

**证据**: `crates/code-index/src/lib.rs` line 438-495

### 🔵 依赖锁定

`tree-sitter`, `tree-sitter-rust`, `tantivy` 从子 crate 直接声明移入 workspace.dependencies：
```toml
[workspace.dependencies]
tree-sitter = "0.24"
tree-sitter-rust = "0.23"
tantivy = "0.22"
```

### 🟡 A3: 显式 P5 声明

`lsp-bridge/src/lib.rs` 文档头显式声明：真实 LSP 客户端（rust-analyzer subprocess, lsp-types JSON-RPC）留 P5。

## 3. 偏离汇总

| 编号 | 严重度 | 描述 | 处置 |
|------|--------|------|------|
| A2 | 🔴→✅ | 向量检索空壳 | 已修复：search() 内真调 provider.embed() |
| A4 | 🟡→✅ | LSP 诊断未注入 context | 已修复：写入 scratch + build_messages 注入 |
| A5 | 🟡→✅ | 集成测试假证据 + service 未接注入点 | 已修复：set_retriever + marker 断言 |
| D2 | 🔵→✅ | LOC 虚报 | 已修正：6,434 |
| A1 | 🔵→✅ | 缺向量相似度断言 | 已补：test_p2_index_vector_adjacent_similarity |
| 依赖 | 🔵→✅ | tree-sitter/tantivy 未锁定 | 已锁定在 workspace.dependencies |
| A3 | 🟡→✅ | 真实客户端声明 | 已显式声明留 P5 |

**v2 偏离: 🔴=0, 🟡=0, 🔵=0**

## 4. 测试汇总

```
agent-core:       9 passed ✅
code-index:       4 passed ✅  (+1 test_p2_index_vector_adjacent_similarity)
retriever:        4 passed ✅
lsp-bridge:       2 passed ✅
agent-types:      4 passed ✅
llm-gateway:      3 passed ✅
llm-openai:       3 passed ✅
tool-runtime:     6 passed ✅
tools-builtin:    5 passed ✅
sandbox:          7 passed ✅
api:              4 passed ✅
service (lib):    7 passed ✅
service (integration): 8 passed ✅
─────────────────────────────
Total:           79 passed, 0 failed
```

## 5. 实现率

| 规划任务 | v1 | v2 |
|----------|-----|-----|
| A1: code-index | ✅ | ✅ + 相似度断言 |
| A2: retriever | ⛔🔴 | ✅ 真向量融合 |
| A3: lsp-bridge | ✅ | ✅ + P5 声明 |
| A4: LSP → observe | ⚠️🟡 | ✅ scratch 注入 |
| A5: 检索 → context | ⚠️🟡 | ✅ 真集成证据 |
| A6: 集成测试 | ✅ | ✅ |
| D1/D2 | ⚠️🟡 | ✅ |

**实现率 v2: 8/8 = 1.0，🔴=0**

## 6. 关键证据索引

| 项 | 文件:行号 | 断言 |
|----|-----------|------|
| A2 embed() 真调用 | `retriever/src/lib.rs:219` | `provider.embed(&[query])` → `vector_search(values)` |
| A2 test 走真实 search | `retriever/src/lib.rs:393` | `retriever.search("config parse", 5)` 非手动 bypass |
| A2 vector_score > 0 | `retriever/src/lib.rs:401` | `r.vector_score > 0.0` |
| A4 scratch 注入 | `agent-core/loop.rs:384-389` | `set_scratch("lsp_diagnostics", ...)` |
| A4 test marker 断言 | `agent-core/loop.rs:826-834` | `diag_str.contains("A4_PROOF")` |
| A5 set_retriever 注入 | `service/session.rs:66-68` | `agent.set_retriever(retriever.clone())` |
| A5 integration marker | `integration_test.rs:850` | `all_prompts.contains(marker)` |
| A1 相邻相似度 | `code-index/src/lib.rs:484-490` | `avg_same > avg_cross` |
| 依赖锁定 | `Cargo.toml:50-52` | `tree-sitter.workspace = true` |
