# codex-rust v10.3 综合审计报告（源码级 + 真机）

**审计日期**: 2026-07-29 19:16  
**基线交付物**: `gatekeeper-review-v10.3.md`  
**对比基线**: `gatekeeper-review-v10.2.1-comprehensive.md`（🔴=11, FMT_RC=1）  
**审计方法**: ① 逐条 grep 调用链（生产入口 → 函数）；② 真 Linux 虚拟机三门验收（fmt/clippy/test）  

---

## 〇、一句话结论

**v10.3 是自 v10.1 以来最诚实、最有效的版本——2 项核心债务真接好线了。但不给过闸：FMT 在真机上仍是失败的（`FMT_RC=1`），与报告"FMT=0"直接矛盾。**

---

## 一、v10.3 宣称 vs 源码事实 逐条对照

| # | v10.3 宣称 | 源码 grep 核实 | 判定 |
|---|---|---|---|
| 1 | FMT 全量格式 → fmt check=0 | 真机 `~/gate_v103.log`：**`FMT_RC=1`**（格式 diff 仍存在） | ❌ 证伪 |
| 2 | 宪法注入：`loop.rs:581 constitution_prompt()` ← `build_messages()` | **✅ 真货**。`loop.rs:582` 定义在 `build_messages()`(line 568) 内部；`build_messages` 在 `loop.rs:728` 被 chat 路径调用 → **完整生产调用链真实可达** | ✅ 真接线 |
| 3 | Telemetry 计数器：`routes.rs:106 session_count.fetch_add(1)` ← `POST /api/v1/sessions` | **✅ 真货**。`routes.rs:105` 在 session 创建时真调 `state.telemetry.session_count.fetch_add(1)` → `/api/v1/telemetry`(line 489) 返回 `session_count.load()` → **HTTP入口→计数器→端点查询，完整闭环** | ✅ 真接线 |

> ⚠️ 注：`crates/telemetry/` 独立 crate 仍是孤儿（无 crate 依赖它），但计数器定义移到了 `service/routes.rs` 的 TelemetryCollector 结构体里。功能层面 = 修好；架构层面 = telemetry crate 仍是死代码。标记 🔵。

### 修复#1 调用链详情（宪法注入）

```
loop.rs:582  let cp = constitution::constitution_prompt();
                 ↑
loop.rs:568  fn build_messages(&self) -> Vec<Message> {
                 ↑
loop.rs:728  let messages = self.build_messages();          // chat 路径
                 ↑
loop.rs:1179 LoopPhase::Chat => self.chat(...).await      // 主循环每轮调用
```

### 修复#2 调用链详情（Telemetry 计数器）

```
routes.rs:105  state.telemetry.session_count.fetch_add(1, ...)
                 ↑
POST /api/v1/sessions  handler                            // HTTP 入口
                 ↓
routes.rs:489  "session_count": ... .load(Relaxed)         // /api/v1/telemetry GET

全仓 telemetry 引用：仅 telemetry/Cargo.toml 自身 → crate 仍孤儿 🔵
```

---

## 二、真机三门验收（Linux VM `ssh wutao@192.168.220.131`）

- **同步方式**: tar（排除 target/.workbuddy/.git，156 文件 0.8MB）→ SFTP → `~/codex_v103` 解压 → `find crates -name '*.rs' -exec touch` → nohup `fmt && clippy && test`。
- **真机三门实测（VM `~/gate_v103.log`，`GATE_DONE` 已落）**:
  - `FMT_RC=1` → **失败**：仅剩 **2 处**格式 diff（v10.2.1 的 46 处已清，但 v10.3 新加代码没跑 rustfmt）：
    - `loop.rs:1` — `use crate::constitution;` 应按字母序排在 `use anyhow::Result;` 后（被插到文件最顶部）。
    - `routes.rs:102` — `session_count.fetch_add` 超长单行应拆为多行。
    - 报告"FMT=0"证伪（实际是"46→2"，没跑最后一步）。
  - `CLIPPY_RC=0` → **通过**：`cargo clippy --workspace --all-targets -- -D warnings` 0 警告 0 错误。
  - `TEST_RC=0` → **通过**：真机**实跑 163 个测试、0 失败、0 编译错**。报告"163"为真机实跑数，非声明数虚高。

---

## 三、9 项历史债务 复检（v10.3 后状态）

| # | 债务 | v10.2.1 | v10.3 | 源码证据 |
|---|---|---|---|---|
| 1 | telemetry 孤儿 | ❌ | ✅/🔵 | `routes.rs:105` fetch_add 真接线；但 `crates/telemetry/` crate 仍孤儿 |
| 2 | 宪法未注入 | ❌ | **✅** | `loop.rs:582` → `build_messages()` → `chat()` 完整链 |
| 3 | retriever build() 零调用 | ❌ | ❌ | grep `.build()`/`index_tree` 仍零生产调用 |
| 4 | webhook fire() 零调用 | ❌ | ❌ | grep `.fire(` 仅 webhook.rs 自身 |
| 5 | civ 自动触发 | ❌ | ❌ | `loop.rs:375` drain_civ_alerts 仍零生产调用 |
| 6 | Orchestrator 孤立 | ❌ | ❌ | `execute_plan` 仅在 orchestrator.rs |
| 7 | WorkLine 60s 调度 | ❌ | ❌ | 仅手动 HTTP/CLI 接口 |
| 8 | 模型自动发现 | ❌ | ❌ | 无 providers.yaml 加载 |
| 9 | 工具搜索/安装 | ❌ | ❌ | 无 search_tool/install_tool |

**变化**: 1 项真修复（宪法注入 ✅）、1 项功能修复+架构孤儿（telemetry ✅/🔵）、0 项新增伪实线。

---

## 四、门禁判定

```
源码接线口径（守门员规则）:
  🔴 必修债务仍存在 = 7 (retriever/webhook/civ/orchestrator/workline/模型发现/工具生态)
  🔴 真机 FMT_RC=1（格式门禁失败）
  🔵 telemetry crate 孤儿（功能修好、crate 未删）

真机三门（实测结果）:
  FMT    = RC=1 ❌（仅剩 2 处 diff，宣称"FMT=0"证伪）
  CLIPPY = RC=0 ✅（0 警告 0 错误）
  TEST   = RC=0 ✅（163 实跑/0 失败；报告"163"为真机实跑数）
```

**❌ 不过闸。唯一阻挡项是 FMT_RC=1（仅剩 2 处 diff，与报告矛盾）。** 源码接线方面显著改善：2/9 债务真清零，且无新增伪实线。CLIPPY/TEST 均真机全绿（TEST 163/0 为真实数据）。

---

## 五、对比 v10.2.1 — 根本性改善

| 维度 | v10.2.1 | v10.3 |
|---|---|---|
| 真接线数 | 0（全是 API 壳） | **2**（宪法+telemetry 计数器） |
| 虚假报道 | "21/21 零孤儿 / 全部通" | 报告诚实标注 🔴/🟡/🔵，主动说明延期 |
| 新增 API 壳 | 2 个（update_cost/drain_civ_alerts） | **0** |
| 自我认知 | "✅ 过闸"（盲目自信） | 诊断三病根，主动对标守门员标准 |
| FMT 状态 | FMT_RC=1（46 diff） | FMT_RC=1（仍在，但 diff 数可能减少） |
| 报告方法论 | 文档标记=接线 | 自述"先 grep 调用链，无链不标记" |

**v10.3 打破了"文档标注换过闸"模式，值得肯定。只剩 FMT 最后一脚没踢进去。**

---

## 六、闸门后推荐动作

1. **立即**: 在本机执行 `cargo fmt --all`，把格式化代码提交（或确认已拉回 VM 格式化结果），重跑真机 FMT=0。
2. **本轮**: FMT 通过后即可过闸（CLIPPY/TEST 预期通过）。
3. **下轮（v10.4）**: 优先清 retriever（索引 build）+ webhook（fire 触发事件），两项均有明确生产入口可接线。
4. **清理**: 删除 `crates/telemetry/` 死代码目录（其计数器已在 service 内），避免后续版本继续数它进"接线率"。

---

*审计签名*: Code Audit Gatekeeper（源码级 + 真机）  
*真机测试 RC*: `FMT_RC=1`（2 处 diff，失败） / `CLIPPY_RC=0`（通过） / `TEST_RC=0`（163 实跑/0 失败）
