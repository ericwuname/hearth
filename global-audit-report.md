# 代码全局审计报告 · codex-rust v1.0

> 审计日期：2026-07-28
> 审计方法：三路并行 agent 逐文件阅读全部 38 个 `.rs` 文件 + 守门员实算硬指标（Python 计 LOC / 正则计测试数 / crate 间依赖映射）
> 铁律：不信报告信源码 —— 所有结论均有 `grep` + `Read` 溯源，数字为本机 Python 实算

---

## 1. 硬指标

| 指标 | 值 | 来源 |
|---|---|---|
| crate 数 | 17 | Cargo.toml workspace members |
| `.rs` 文件数 | 38 | Python `os.walk` |
| 总 LOC | **13,007** | Python 逐文件 `len(readlines())` |
| 测试函数数 | **139** | 正则 `#\[(?:\s*tokio::)?\s*test` |
| 零断言测试 | **1** | `test_noop_sandbox_warn_on_creation`（冒烟级，非永久 PASS）|
| 生产空壳 | **0** | 3 处仅限 `#[cfg(test)]` mock / 字符串字面量 |

---

## 2. 发现汇总（按严重度）

### 🔴 严重（crash / 数据损坏 / 安全可被外部触发 / 资源必然泄漏）

| # | 编号 | Crate | 位置 | 问题 |
|---|---|---|---|---|
| 1 | **AUTH-0** | service | `routes.rs` 全部 6 个端点 | **API 零认证**：任何人均可创建 session、发消息、订阅 SSE 事件流（含 Token/ToolCall 等敏感数据）。监听 `0.0.0.0:3000` |
| 2 | **OOM-1** | service | `session.rs:155-158` | **Sessions HashMap 永不清除**：完成/取消的 session 永不移除，长时间运行必然 OOM |
| 3 | **SBOX-1** | tools-builtin | `edit.rs:46-77` | **edit 工具绕过 sandbox**：直接调用 `tokio::fs::write`，仅字符串路径检查（`contains("..")` + `is_absolute`），不受 landlock/seccomp 约束。LLM 可写入工作区任意文件 |
| 4 | **APPR-1** | agent-core | `loop.rs:506-511` | **审批触发用 JSON 字符串匹配**：`tc.args.to_string().contains("rm ")` 同时存在误报（`echo "rm"` 触发审批）和漏报（`rm-rf` 不含空格不触发）。`write` 关键字会误匹配任何包含该子串的 args |
| 5 | **STREAM-1** | llm-openai | `lib.rs:309` | **stream spawn 无取消机制**：`tokio::spawn` 后 JoinHandle 被丢弃，调用方 drop stream 时 spawned task 和 HTTP 连接仍存活，直到下次 `tx.send()` 失败才发现。长期 agent 循环累积僵尸 task + 连接 |
| 6 | **STREAM-2** | llm-cn | `lib.rs:476` | 同上 |
| 7 | **STREAM-3** | llm-local | `lib.rs:426` (Ollama) | 同上 |
| 8 | **STREAM-4** | llm-local | `lib.rs:879` (vLLM) | 同上 |
| 9 | **MEM-1** | memory | `memory.rs:77` | **JSONL 非原子写入**：`File::create` 直接覆盖，崩溃或磁盘满导致文件截断、数据丢失。应用 `temp file + rename` |
| 10 | **MEM-2** | memory | `memory.rs:77,179` | **JSONL 并发写入无锁**：多 task 同时 `save_session` 同一 session_id 互相覆盖；`append_events` 多 task 并发追加时写入交错 |
| 11 | **MEM-3** | memory | `memory.rs` | **JSONL 无文件大小限制**：`save_session` + `append_events` 无条件写入，攻击者可耗尽磁盘 |
| 12 | **MEM-4** | memory | `memory.rs:122` | **反序列化单行失败整批丢弃**：`serde_json::from_str(line)?` 使用 `?` 传播错误，任意一行 JSON 损坏 → 全部事件丢失。应逐行容错 + `tracing::warn!` |

### 🟡 高（需在发布前修复 / 安全面硬伤）

| # | 编号 | Crate | 位置 | 问题 |
|---|---|---|---|---|
| 13 | **SBOX-2** | tools-builtin | `read.rs:42-48` | **read 工具绕过 sandbox**：直接 `tokio::fs::read_to_string`，仅字符串路径检查。LLM 可读取 `.env`/`config.toml` 等敏感文件（虽受 `cwd` 子树限制） |
| 14 | **SBOX-3** | agent-core | `loop.rs:210-259` | `spawn_sub_agent` **公开方法无递归深度守卫**：守卫仅存在于调用方 `do_plan()`，外部直接调用可绕过 |
| 15 | **API-1** | service | `routes.rs` 全部 POST 端点 | **JSON body 无大小上限**：`axum::Json<T>` 默认无限制，攻击者可发超大 JSON 耗尽内存 |
| 16 | **API-2** | service | 全部路由 | **无速率限制**：所有端点可被无限次调用，支持暴力遍历 `session_id`（尽管 UUID v4 空间大） |
| 17 | **API-3** | service | `main.rs:157-158` | **CORS 过于宽松**：`allow_methods(Any)` + `allow_headers(Any)`，应限制为 `GET,POST,OPTIONS` + 所需 headers |
| 18 | **LLM-1** | llm-openai | `lib.rs:210` | **HTTP 客户端无超时配置**：`Client::new()` 默认无超时，网络故障可无限阻塞。`llm-cn:276`、`llm-local:261,689` 同样存在 |
| 19 | **LLM-2** | llm-openai | `lib.rs:280` | **JSON 解析失败静默降级**：`serde_json::from_str(&tc.function.arguments).unwrap_or(Value::String(...))` — LLM 返回畸形 JSON 时下游收到 `Value::String` 而非预期的 `Value::Object`，工具调用签名校验静默失败 |
| 20 | **MEM-5** | memory | `memory.rs:126-139` | **反序列化字段缺失静默变 ""/0**：`val["session_id"].as_str().unwrap_or("")` — 字段缺失 → 数据静默丢失，无日志无告警 |
| 21 | **CNCL-1** | service | `session.rs:309` | **`abort()` 对阻塞操作无效**：agent_future 在等待 LLM HTTP 响应（同步阻塞）时 abort 不能立即生效，需配合 `tokio::time::timeout` |
| 22 | **DUP-1** | llm-local | `lib.rs:550-710` | **vLLM wire types 完全重复 llm-openai**：~150 行相同代码（`VllmRequest`=`OpenAiRequest` 等），应提取共享 OpenAI-compatible 类型到 `llm-gateway` |
| 23 | **DUP-2** | llm-openai, llm-cn, llm-local | `lib.rs` | **`ContentToText` trait 三份完全相同**（各 ~20 行），应提升到 `agent-types` |
| 24 | **DUP-3** | llm-cn, llm-local | `lib.rs` | **`consume_sse_stream` 两份完全相同**（各 ~30 行），应提升到 `llm-gateway` |
| 25 | **CODEIDX-1** | code-index | `lib.rs:116,159,178` | **3 处 `child(i).unwrap()`**：理论上 `child_count` 保证索引有效，但 tree-sitter bug → panic。应用 `child(i).ok_or_else(||...)` |

### 🔵 低 / 参考级（不阻塞，择机改进）

| # | 编号 | Crate | 位置 | 问题 |
|---|---|---|---|---|
| 26 | **CONS-1** | tools-builtin | `grep.rs:66-75` | grep 路径穿越**静默回落 cwd**（vs read/edit/glob 拒绝），行为不一致，LLM 可能被误导 |
| 27 | **CONS-2** | tools-builtin | `edit.rs:47,read.rs:43,glob.rs:54` | `contains("..")` 误拦合法文件名（如 `test..txt`），应用 `.components()` 检查 `ParentDir` 组件 |
| 28 | **PERF-1** | tools-builtin | `grep.rs:81` | `has_rg` 每次 grep 调用都执行沙箱探测（`rg --version`），应缓存结果 |
| 29 | **SSE-1** | service | `sse.rs:65` | `to_sse_event(e).ok().map(Ok)` 静默吞错误——未来 AgentEvent 增加变体未更新此函数时，事件将被静默丢弃 |
| 30 | **STALE-1** | sandbox | `lib.rs:4` | 注释"seccomp deferred to P5"，但 seccomp 已实现并上线 → 文档过期 |
| 31 | **TRACE-1** | api | `lib.rs:8` | 北向契约注释 "MUST match architecture.final.md §15.3"，但 `docs/` 无此文件 |
| 32 | **WEAK-1** | sandbox | `lib.rs:698` | `test_noop_sandbox_warn_on_creation` 无显式断言（冒烟级） |
| 33 | **ORPHAN-1** | telemetry | `eval.rs:57` | `EvalRunner` 全仓 0 调用（仅在 crate 内部测试被构造），eval harness 未集成进 CI/测试管线 |
| 34 | **FALLBACK-1** | service | `main.rs:35-36` | 无 `OPENAI_API_KEY` 时静默用 `"sk-placeholder"`，生产环境应 fail-fast |
| 35 | **URL-1** | llm-openai, llm-cn, llm-local | 多处 | `format!("{}/chat/completions", base_url)` 未处理 base_url 末尾路径，自定义 URL 可能构造错误 |
| 36 | **SEC-1** | llm-local | `lib.rs:242-247` | `OllamaProvider` 无 `api_key` 字段——指向远程实例时无法认证 |
| 37 | **CONC-1** | agent-core | `loop.rs:100-101` | `current_phase` 字段 `#[allow(dead_code)]`，设置后从未读取 |
| 38 | **DUP-4** | llm-gateway | `fallback.rs:61-172` | `FallbackChain` 独立方法 + trait impl 两套几乎相同的 chat/embed/stream 逻辑 |

---

## 3. 跨领域架构关切

### 3.1 沙箱覆盖不均 — 🔴
```text
bash  → sandbox.spawn("bash",  ["-c", cmd], ...)     ✅ 走沙箱
glob  → sandbox.spawn("find",  &args, ...)            ✅ 走沙箱
grep  → sandbox.spawn("rg",    &args, ...)            ✅ 走沙箱
edit  → tokio::fs::write()                             ❌ 绕过沙箱
read  → tokio::fs::read_to_string()                    ❌ 绕过沙箱
```
edit/read 是 LLM 频繁使用的工具，绕过 sandbox 意味着 landlock/seccomp 存在盲区。更合理的做法：让 edit/read 也走 sandbox（通过 `cat`、`dd` 或带 `--sandbox` flag 的编辑器），或在 agent 进程本身也加 landlock。

### 3.2 API 安全面全零 — 🔴
当前 6 个路由 **均无认证、无速率限制、无 body 限制、无 session 超时清理**。这 4 项缺失叠加后，从"开发辅助工具"变为"对外暴露的攻击面"。代码质量虽高，但 API 安全面是硬伤。

### 3.3 数据持久化缺乏容错 — 🔴
`JsonlMemoryStore` 存在非原子写入、并发无锁、单行损坏整批丢弃、字段缺失静默吞 — 四项叠加意味着在真实服务重启场景下，session 数据几乎必然会丢失或损坏。建议在测试中增加"模拟崩溃后恢复"的集成测试来暴露这些问题。

### 3.4 代码重复散落 — 🟡
`ContentToText` trait ×3、`consume_sse_stream` ×2、vLLM/openai wire types 几乎完全重叠。这些应该在 `llm-gateway` 或 `agent-types` 中定义一次，各 provider 引用。

### 3.5 Stream 取消普遍缺失 — 🔴
4 个 provider 共 5 处 `tokio::spawn` 丢弃 JoinHandle，无取消令牌，无 AbortHandle。在 stream 被 drop（agent 循环中频繁发生）后，僵尸 task 和 HTTP 连接持续消耗资源。应在 `llm-gateway` 提供统一的 `spawn_cancellable_stream` 工具函数。

---

## 4. 逐 Crate 质量评分

| Crate | LOC | 测试 | 空壳 | 错误处理 | 并发安全 | 🔴 | 🟡 | 🔵 | 评分 |
|---|---|---|---|---|---|---|---|---|---|
| agent-types | 447 | 9 | ✅ | ✅ | n/a | 0 | 0 | 0 | 🟢 |
| agent-core | 1920 | 11+ | ✅ | ✅ | 2 问题 | 1 | 1 | 1 | 🟡 |
| api | 133 | 3 | ✅ | ✅ | n/a | 0 | 0 | 1 | 🟢 |
| code-index | 514 | 4 | ✅ | 3 unwrap | ✅ | 0 | 1 | 0 | 🟡 |
| llm-cn | 774 | 6 | ✅ | 2 静默吞 | 1 spawn 泄漏 | 1 | 2 | 2 | 🔴 |
| llm-gateway | 847 | 19 | ✅ | ✅ | ✅ | 0 | 0 | 1 | 🟢 |
| llm-local | 1366 | 9+ | ✅ | 3 静默吞 | 2 spawn 泄漏 | 2 | 3 | 3 | 🔴 |
| llm-openai | 580 | 4 | ✅ | 2 静默吞 | 1 spawn 泄漏 | 1 | 2 | 1 | 🟡 |
| lsp-bridge | 106 | 2 | ✅ | ✅ | n/a | 0 | 0 | 0 | 🟢 |
| memory | 295 | 4 | ✅ | ✅ | 无并发保护 | 4 | 1 | 0 | 🔴 |
| planner | 505 | 5 | ✅ | ✅ | ✅ | 0 | 0 | 0 | 🟢 |
| retriever | 502 | 4 | ✅ | ✅ | ✅ | 0 | 0 | 0 | 🟢 |
| sandbox | 899 | 12 | ✅ | ✅ | ✅ | 0 | 0 | 2 | 🟢 |
| service | 2512 | 16 | ✅ | 1 静默 | 1 abort 惰性 | 3 | 4 | 1 | 🔴 |
| telemetry | 402 | 1 | ✅ | 1 unwrap | ✅ | 0 | 0 | 1 | 🟢 |
| tool-runtime | 419 | 8 | ✅ | ✅ | ✅ | 0 | 0 | 0 | 🟢 |
| tools-builtin | 786 | 20 | ✅ | ✅ | ✅ | 1 | 1 | 2 | 🟡 |

**评分说明**：🟢 = 无中等以上问题；🟡 = 有 🟡 发现问题需修复；🔴 = 有 🔴 发现问题必须修。

---

## 5. 闸门判定

```
🔴 计数 = 12（含跨 crate 大类问题的多实例）
🟡 计数 = 15
🔵 计数 = 13

实现率 = 达标模块 / 总模块 = 7 / 17 ≈ 0.41
```

### **闸门结论：🔴 不通过，打回。**

不通过的原因不是"代码质量差"——恰恰相反，核心架构清晰、无生产空壳、测试有断言、分层规范。不通过是因为**安全面和容错面存在系统性缺口**：

1. **API 零认证**（AUTH-0）：任何人均可访问全部端点 + 实时 SSE 流
2. **edit/read 绕过 sandbox**（SBOX-1/2）：landlock/seccomp 存在盲区
3. **4 个 provider 的 stream 取消缺失**（STREAM-1~4）：长期 agent 循环累积僵尸 task
4. **Sessions HashMap 永不清除**（OOM-1）：必然触发 OOM
5. **JSONL 四项并发/容错缺失**（MEM-1~4）：数据持久化不可靠
6. **审批触发用字符串匹配 JSON**（APPR-1）：可绕过

---

## 6. 修复优先级路线图

### Phase 1：阻塞部署（必须在首次生产 deploy 前修）

| 次序 | 问题 | 工作量 | 建议方案 |
|---|---|---|---|
| P1-1 | API 零认证 | 小 | 中间件检查 `Authorization: Bearer <key>` → 从环境变量 `API_KEY`/`AUTH_TOKEN` 读取；SSE 连接也需在握手时验证 token |
| P1-2 | Sessions HashMap 永不清除 | 小 | 在 session 完成/取消/错误三条路径各加 `self.sessions.write().remove(&session_id)` |
| P1-3 | edit/read 绕过 sandbox | 中 | 两种路径：A) 给 edit/read 也注入 `Arc<dyn Sandbox>`，通过 sandbox.spawn 执行；B) 在 agent 进程 pre_exec 也套 landlock（全局防护）。推荐 B（少量改动） + A（长期） |
| P1-4 | Stream spawn 泄漏 (4 provider) | 中 | `llm-gateway` 提供 `CancellableStream` wrapper：`tokio::spawn` + `CancellationToken`，Drop 时 `token.cancel()`。各 provider 统一接入 |
| P1-5 | JSONL 非原子写入 | 小 | `File::create` 改为 `write_to_temp + fs::rename`（同一文件系统保证原子性） |
| P1-6 | JSONL 并发写入 | 中 | `JsonlMemoryStore` 内部加 `Mutex<()>` 或 `tokio::sync::Mutex` 保护写入 |
| P1-7 | JSONL 反序列化单行失败整批丢弃 | 小 | 改 `?` 为 `match`，单行失败 `tracing::warn!` + `continue`，不传播错误 |

### Phase 2：安全加强（首次部署前修）

| 次序 | 问题 | 工作量 | 建议方案 |
|---|---|---|---|
| P2-1 | 审批触发改用语义匹配 | 中 | 放弃 `tc.args.to_string().contains(...)`，改为基于 tool name 的审批策略表（如 `{"bash": "always_approve", "edit": "always_approve", "read": "never", ...}`） |
| P2-2 | JSON body 大小限制 | 小 | axum `DefaultBodyLimit::max(10 * 1024 * 1024)` 限制 10MB |
| P2-3 | 速率限制 | 中 | `tower::limit::RateLimitLayer` 或 `governor` crate，每 IP 每秒 N 次 |
| P2-4 | CORS 收紧 | 小 | `allow_methods([Method::GET, Method::POST, Method::OPTIONS])` + `allow_headers([CONTENT_TYPE, AUTHORIZATION])` |
| P2-5 | HTTP 客户端超时 | 小 | 所有 provider 加上 `.timeout(Duration::from_secs(120)).connect_timeout(Duration::from_secs(10))` |
| P2-6 | spawn_sub_agent 递归守卫 | 小 | 在方法开头加 `if self.depth >= MAX_DEPTH { return Err(...) }` |

### Phase 3：技术债清理（择机，不阻塞）

| 次序 | 问题 |
|---|---|
| P3-1 | vLLM wire types 提取到 `llm-gateway` |
| P3-2 | `ContentToText` trait 提升到 `agent-types` |
| P3-3 | `consume_sse_stream` 提升到 `llm-gateway` |
| P3-4 | tree-sitter `child(i).unwrap()` → `ok_or_else` |
| P3-5 | 路径穿越检查改用 `.components()` 检查 `ParentDir` 组件 |
| P3-6 | `has_rg` 结果缓存 |
| P3-7 | grep 路径穿越拒绝（保持一致） |
| P3-8 | `to_sse_event` 错误加 `tracing::warn!` |
| P3-9 | sandbox/lib.rs:4 seccomp 注释更新 |
| P3-10 | `architecture.final.md §15.3` 恢复或删除引用 |
| P3-11 | telemetry eval harness 接线进构建管线 |
| P3-12 | `main.rs` 无 key 时 fail-fast |
| P3-13 | `current_phase` dead_code 清理 |
| P3-14 | FallbackChain 代码去重 |
| P3-15 | `test_noop_sandbox_warn_on_creation` 加断言 |
| P3-16 | abort() 配合 timeout 使用 |
| P3-17 | JSONL 反序列化字段缺失加 warn 日志 |

---

## 7. 结论

**代码质量**：核心架构设计水平高，trait 抽象合理（`LlmProvider` / `Tool` / `Sandbox` / `MemoryStore`），分层干净（`agent-core` → `llm-gateway` 从不越界），无生产空壳，测试 139 个函数仅 1 个无断言（且是冒烟级）。

**硬伤**：安全面和容错面存在系统性缺口。API 零认证 + sesions 泄露 + edit/read 绕过沙箱 + stream 取消缺失 + JSONL 不可靠 — 5 项跨模块问题叠加后，当前形态**不宜对外部署**，仅适合受控开发环境。

**闸门**：🔴=12 → **不通过**。需完成 P1（7 项，估 1–2 天）方可重新审计。

**亮点**：问题集中度高、根因清晰、修复方案成熟（多为加锁/加中间件/加 wrapper），无架构级返工——意味着"打回不通过"不是灾难，而是一个可管理的修复清单。代码底盘是好的，缺的是装甲。
