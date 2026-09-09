# codex-rust v10.4 综合审计报告（源码级 + 真机）

**审计日期**: 2026-07-29 20:42  
**基线交付物**: `gatekeeper-review-v10.4-final.md`  
**对比基线**: `gatekeeper-review-v10.3-comprehensive.md`（2/9 真修，FMT_RC=1）  
**审计方法**: ① 逐条 grep 调用链（生产入口 → 函数）；② 真 Linux 虚拟机三门验收（fmt/clippy/test）  

---

## 〇、一句话结论

**v10.4 是自 v6.0 以来债务清理最彻底的版本：7/9 债务真接线（grep 可证完整生产调用链），0 个新增 API 壳，报告诚实。但 FMT 又红了——4 处 diff 全在新加的 `main.rs` 代码里，又是"加新代码不跑 fmt"。**

---

## 一、v10.4 宣称 vs 源码事实 逐条对照

| # | v10.4 宣称 | 源码 grep 核实 | 判定 |
|---|---|---|---|
| 1 | telemetry ✅ | `routes.rs:108` fetch_add ← POST /sessions → `/api/v1/telemetry` load | ✅ 真接线（v10.3 已修） |
| 2 | 宪法注入 ✅ | `loop.rs:582` constitution_prompt() → build_messages() → chat() | ✅ 真接线（v10.3 已修） |
| 3 | retriever build() ✅ | `main.rs:238` build(&[],&[]) → `set_retriever` → `loop.rs:961` search() → `build_messages` 注入 | **✅ 真接线（新）** |
| 4 | webhook fire() ✅ | `routes.rs:113` fire_event → `webhook.rs:37` curl subprocess → 真发 HTTP POST | **✅ 真接线（新）** |
| 5 | civ 自动触发 ✅ | `routes.rs:130` civ_store.append ← POST /sessions | **✅ 真接线（新）** |
| 6 | Orchestrator 🟡 | `lib.rs:17` re-export execute_plan；test_execute_plan_3_step_chain 存在 | 🟡 诚实标注（有测试、需重构调度器） |
| 7 | WorkLine 60s ✅ | `main.rs:320` tokio::spawn → 60s sleep → list pending → update progress | **✅ 真接线（新）** |
| 8 | 模型发现 ✅ | `main.rs:195` providers.json write+read on startup → auto-discover | **✅ 真接线（新）** |
| 9 | 工具搜索/安装 🔵 | deferred: security audit required | 🔵 诚实标注（无代码，非虚假） |
| 10 | FMT=0 | 真机 `~/gate_v104.log`：**`FMT_RC=1`**（4 处 diff，全在 `main.rs` 新代码） | ❌ 证伪 |

**7/9 债务真清零（grep 可证），0 新增 API 壳，1 项 FMT 证伪。**

---

## 二、新接线调用链详情（grep 可证）

### retriever（语义检索）

```
main.rs:238   retriever.build(&[], &[]).await.expect(...)
                 ↑
main.rs:235   let mut retriever = TantivyRetriever::new();
main.rs:236   retriever.set_embed_provider(embed_provider);
main.rs:239   sessions.set_retriever(Arc::new(retriever));
                 ↓
loop.rs:339   retriever: Option<Arc<dyn Retriever>>        // AgentLoop 字段
loop.rs:429   pub fn set_retriever(&mut self, ...)         // 注入点
loop.rs:961   if let Some(ref retriever) = self.retriever { retriever.search(&query, 5) }
loop.rs:603   // inject retrieved context into system prompt
```

### webhook（HTTP 回调）

```
routes.rs:113  .fire_event("session_created", &json!({"goal": req.goal}))
                 ↑
POST /api/v1/sessions  handler
                 ↓
webhook.rs:37  pub async fn fire(event, payload, hooks) {
webhook.rs:43    tokio::process::Command::new("curl")       // 真发 HTTP POST
webhook.rs:62  pub async fn fire_event(&self, event, payload)
```

### CIV（文明线自动写入）

```
routes.rs:130  let _ = state.civ_store.append(civ_entry);
                 ↑
POST /api/v1/sessions  handler
                 ↓
routes.rs:277  GET /api/v1/civilization — recent entries
routes.rs:287  POST /api/v1/civilization — manual post
```

### WorkLine 60s 调度

```
main.rs:320   let wl_bg = workline_store.clone();
main.rs:321   tokio::spawn(async move {
main.rs:323       tokio::time::sleep(Duration::from_secs(60)).await;
main.rs:324       let pending = wl_bg.list(Some("pending"));
main.rs:325-330  // advance stale pending nodes
```

### 模型自动发现

```
main.rs:195   let providers_path = env::var("PROVIDERS_PATH").unwrap_or("./providers.json");
main.rs:205   std::fs::write(&providers_path, default_providers)   // 首次写入默认
main.rs:208   if let Ok(data) = std::fs::read_to_string(&providers_path) {
main.rs:214       tracing::info!(name, provider, "model auto-discovered");
```

---

## 三、真机三门验收（Linux VM `ssh wutao@192.168.220.131`）

- **同步方式**: tar（排除 target/.workbuddy/.git，158 文件 0.8MB）→ SFTP → `~/codex_v104` 解压 → `find crates -name '*.rs' -exec touch` → nohup `fmt && clippy && test`。
- **FMT_RC=1**（确认）：4 处 diff，全在 `service/src/main.rs`：
  - `main.rs:202` — `std::fs::write` 超长单行 → 需拆多行
  - `main.rs:210` — `let provider = ...` 链式调用超长 → 需拆多行
  - `main.rs:235` — `retriever.build(...).await.expect(...)` 超长 → 需拆多行
  - `main.rs:323` — `tracing::info!` 参数过长 → 需压缩单行
- **真机三门实测（VM `~/gate_v104.log`，`GATE_DONE` 已落）**:
  - `FMT_RC=1` → **失败**：4 处 diff，全在 `service/src/main.rs`（见上方 §三）。
  - `CLIPPY_RC=0` → **通过**：0 警告 0 错误。
  - `TEST_RC=0` → **通过**：真机**实跑 163 个测试、0 失败、0 编译错**。

---

## 四、9 项历史债务终局（v10.4 后状态）

| # | 债务 | v10.2.1 | v10.3 | v10.4 | 证据 |
|---|---|:---:|:---:|:---:|------|
| 1 | telemetry 孤儿 | ❌ | ✅ | **✅** | `routes.rs:108` fetch_add → `/api/v1/telemetry` |
| 2 | 宪法未注入 | ❌ | ✅ | **✅** | `loop.rs:582` → `build_messages()` → `chat()` |
| 3 | retriever build() | ❌ | ❌ | **✅** | `main.rs:238` build → `loop.rs:961` search → `build_messages` 注入 |
| 4 | webhook fire() | ❌ | ❌ | **✅** | `routes.rs:113` fire_event → curl subprocess |
| 5 | civ 自动触发 | ❌ | ❌ | **✅** | `routes.rs:130` civ_store.append ← POST /sessions |
| 6 | Orchestrator | ❌ | ❌ | **🟡** | test_execute_plan_3_step_chain；需重构调度器 |
| 7 | WorkLine 60s | ❌ | ❌ | **✅** | `main.rs:320` tokio::spawn 60s loop |
| 8 | 模型自动发现 | ❌ | ❌ | **✅** | `main.rs:195` providers.json write+read |
| 9 | 工具搜索/安装 | ❌ | ❌ | **🔵** | deferred: security audit |

**清仓率: 7/9（7 ✅ + 1 🟡 + 1 🔵）**

---

## 五、门禁判定

```
源码接线口径（守门员规则）:
  🔴 必修债务 = 0（7/9 已清，🟡 orchestrator 有测试、🔵 tool 诚实延期）
  🔴 真机 FMT_RC=1（4 处 diff，全在 main.rs 新代码）

真机三门（实测结果）:
  FMT    = RC=1 ❌（4 处 diff，全在 main.rs 新代码；宣称"FMT=0"证伪，连续第 4 版同一坑）
  CLIPPY = RC=0 ✅（0 警告 0 错误）
  TEST   = RC=0 ✅（163 实跑/0 失败）
```

**❌ 不过闸。唯一阻挡项是 FMT_RC=1（4 处 diff，全在 `main.rs` 新代码）。** 源码接线方面 7/9 债务真清零，0 新增 API 壳，报告诚实。CLIPPY/TEST 均真机全绿。

---

## 六、FMT 反复失败根因

| 版本 | FMT diff 数 | 位置 | 原因 |
|---|---|---|---|
| v10.2.1 | 46 | 全仓 | 大量历史代码未格式化 |
| v10.3 | 2 | loop.rs:1, routes.rs:102 | 新加代码没跑 fmt |
| v10.4 | 4 | main.rs:202/210/235/323 | 新加代码没跑 fmt |

**规律：每版修旧 diff，但新代码又不格式化就交付。** 根本解决：在 `cargo fmt --all` 后跑一次 `cargo fmt --all --check`，确认 RC=0 再提交。

---

## 七、闸门后推荐动作

### 立即（30 秒）

```bash
cargo fmt --all
# 重跑真机 FMT=0 → 过闸
```

### 本轮 v10.4（FMT 过后即可定版）

- 7/9 债务真清零（grep 可证），CLIPPY/TEST 预期全绿
- 剩余 orchestrator 🟡（有测试）+ tool 🔵（诚实延期）

### 下轮 v10.5

| 优先级 | 任务 | 说明 |
|---|---|---|
| P0 | orchestrator 调度器重构 | `execute_plan` 接路由 `/api/v1/plan/execute` 或 CLI 子命令 |
| P1 | 工具搜索/安装安全设计 | 白名单 + 签名验证 + 沙箱执行 |
| P2 | retriever 索引持久化 | 当前 `build(&[],&[])` 只建空索引，需实际文档/代码入库 |
| P3 | civ 自动触发扩展 | 当前仅 session_create，扩展到 task_complete/fail 等事件 |

---

*审计签名*: Code Audit Gatekeeper（源码级 + 真机）  
*真机测试 RC*: `FMT_RC=1`（4 处 diff，失败） / `CLIPPY_RC=0`（通过） / `TEST_RC=0`（163 实跑/0 失败）
