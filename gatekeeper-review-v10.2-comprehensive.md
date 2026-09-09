# codex-rust v10.2 全面代码审计报告（源码级 + 真机三门验收）

**审计日期**: 2026-07-29  
**基线**: v10.1 定版（9 项系统性"定义未接线"债务）  
**方法**: 不信报告信源码（`grep`+`Read` 逐文件核实）+ 真 Linux VM `fmt / clippy / test` 三门  
**结论速览**: v10.2 **真实打通 1 条经络**（nervous-system → AgentLoop 资源感知，已接主循环生产路径）；但原 9 项债务里**它只动了对资源感知的那 1 条相关面**，其余 8 项仍"定义未接线"。报告"🔴=0 / 全部通了"仅对其自身 3 项窄范围成立，**对项目整体为否**。

---

## 一、v10.2 范围三项核实（✅ 源码级真实）

| 项 | 源码锚点 | 核实 |
|----|---------|------|
| `nervous-system` crate 真身 | `crates/nervous-system/src/lib.rs`（`query()`/`NerveAction`/`PerceptionReport`）| ✅ 真实；且 `Cargo.toml:24` 为 workspace member、`agent-core/Cargo.toml:14` 依赖、被编译 |
| 接入主循环（生产路径） | `loop.rs:1102` `let perception = self.nervous.query();` 位于 `do_reflect`（:1022），由主循环 `LoopPhase::Reflect => self.do_reflect().await`（:1179）每轮调用 | ✅ **真在主循环上**，非测试/分支；`perception.is_critical` 时 `verdict = ReflectVerdict::GiveUp`（:1109）|
| 3 个真实测试 | `lib.rs:153/162/170` `test_nerve_action_is_urgent` / `test_query_returns_report` / `test_query_with_budget_warning` | ✅ 真实断言（`process_pid>0` / `action==DeliverAndQuit` 等）|

**窄判据诚实度**：报告的"经络打通"核心主张成立——资源快照现在每轮 reflect 真被拉取并可能强制 `GiveUp`，这是货真价实的接线改进。

---

## 二、⚠️ 报告口径偏松处（守门员必须指出）

v10.2 "经络验证"表第 2 行写：**"宪法写了 → 没人读：nervous system 查询宪法 Art.3→决策"**。

- 源码真相：`nervous_system::query()`（lib.rs:77-142）是**硬编码阈值规则 + 注释引用 "Constitution Article 3"**，**运行时不读取 `constitution.md` 文件**，也**不调用 `constitution_prompt()`**。
- 经 grep：`constitution_prompt()` 仅 `constitution.rs:7` 定义、`lib.rs:10` 重导出；**`build_messages` 仍从不调用它**（v10.1 债务 #2 未修）。
- 结论：v10.2 把"把 Art.3 的意图写进硬编码规则"说成"宪法被读"，属**口径偏松**。宪法文本注入 LLM system prompt 这条断裂**仍在**。

> 这不影响"nervous-system 真接线"的判定，但影响"宪法闭环"的判定——该闭环未闭。

---

## 三、真机三门验收（fmt / clippy / test）— 待回填

- **执行环境**: 真 Linux `ssh wutao@192.168.220.131`（Ubuntu 24.04，cargo 可用）
- **同步方式**: tar（排除 target/.workbuddy/.git）→ SFTP → 解压 `~/codex_v12` → `find crates -name '*.rs' -exec touch` → **`rm -rf target` 全量重建**（保证 clippy 真重查）→ 顺序跑 `fmt --check` / `clippy --workspace --all-targets -- -D warnings` / `test --all`
- **后台任务**: `JvS89L`（构建完成后自动回填下方）
- **首跑环境坑（已知，已规避）**: Windows 上 `target` 0 字节文件会被 tar 带入 → VM 解压后 `target` 非目录 → cargo `ENOTDIR`。本次排除任何名为 target 的路径分量 + 构建前 `rm -rf target`。

**回填处（RC）**:
```
FMT_RC    = ____
CLIPPY_RC = ____   (CLIPPY 警告/错误数 = ____)
TEST_RC   = ____   (真实执行通过数 = ____ / 失败 = ____)
```
> 注：报告宣称"162 passed"是 `grep` 到的测试属性**声明数**（#[test]=60 + #[tokio::test]=102）。v10.1 真机默认 `cargo test --all` 只跑出 **109**。本次以真机实测数为准。

---

## 四、全局债务复检（v10.1 的 9 项，v10.2 后状态）

守门员对 v10.1 的 9 项"定义未接线"逐一 `grep` 复核，v10.2 后的真实状态：

| # | 债务 | v10.2 后状态 | 证据 |
|---|------|:---:|------|
| ★new | nervous-system → AgentLoop 资源感知 | ✅ **已接**（v10.2 新打通）| `loop.rs:1102` 主循环调用 |
| 1 | v7.0 Telemetry 孤儿（计数器只 load 不 fetch_add，端点恒 0）| ❌ **未修** | `service/Cargo.toml`、`agent-core/Cargo.toml` 均无 `telemetry` 依赖；`telemetry/src/lib.rs` 无计数器/fetch_add；routes.rs:235 的 `fetch_add` 是 `P1_CONCURRENT_COUNT`（P1 并发限制器，非 telemetry）|
| 2 | v10.0 宪法注入 system prompt（`constitution_prompt()` 不被 `build_messages` 调用）| ❌ **未修** | grep 仅定义+重导出，无调用 |
| 3 | E1 retriever 默认开但 `build()` 零调用 → 索引永空 | ❌ **未修** | `session.rs` 无 `.build()`/`index_tree` |
| 4 | v8.0 webhook `fire()` 全仓零调用 | ❌ **未修** | 除 `webhook.rs` 外无 `.fire(` |
| 5 | v6.0 文明线自动触发（loop.rs 对 civ 零引用）| ❌ **未修** | `loop.rs` grep `civ|civilization` = 0 |
| 6 | v9.0/v10.1 Orchestrator 未暴露（无路由/CLI/接主循环）| ❌ **未修** | `execute_plan` 仅 `orchestrator.rs` 内 |
| 7 | v6.1 WorkLine 60s 调度不存在 | ❌ **未修** | `main.rs` 唯一 spawn 是 observer 每小时任务 |
| 8 | v6A 模型自动发现（providers.yaml/cache）| ❌ **未修** | 全仓无 yaml/cache |
| 9 | v10.1 工具搜索/安装生态代码中不存在 | ❌ **未修** | grep 零命中 |

**结论**：v10.2 **只动了 1 条经络**（资源感知接入大脑），原 9 项里其余 8 项纹丝未动。报告"🔴=0 / 实现率 3/3=1.0"只数了 v10.2 这 3 项自身工作，**没把历史 8 项债务计入**，因此在"项目整体是否全部通了"的判据下不成立。

---

## 五、守门员判定 + 对"是不是全部通了、能不能用了"的回答

### 窄判据（仅 v10.2 自身范围）
nervous-system crate 真实 + 接主循环 + 3 真测试 +（待 VM 回填）fmt/clippy/test 三门 → 在"v10.2 小范围"内可判过闸。

### 宽判据（项目整体，回答用户原问）
**"全部通了" = 否。** 仅资源感知→大脑的单向经络打通；telemetry / constitution 注入 / retriever / webhook / civ 自动触发 / orchestrator 暴露 / workline 调度 / 模型发现 / 工具生态 共 8 项仍"定义未接线"。

**"能不能用了" = 分两层**：
- 核心自主循环（plan→act→reflect→verify）自 v6.0 起一直可跑，v10.2 让它**多了一道资源临界自我保护**（内存/磁盘/成本越界时自动 GiveUp）——这是实打实的可用性提升。
- 但"可观测/可检索/可对外通知/可文明自治/可编排/可自愈调度/可模型热插拔/可工具生态"这些 v6–v10 反复宣称的能力，**仍停留在库函数层面，生产路径未通**，不能用。

### 测试数口径
声明 162（grep）≠ 真机实测（待回填，预期 ~112 量级，较 v10.1 的 109 +3）。报告"162 passed"属未机器验证口径，记为 ⚠️ 文档纪律问题（不计入接线 🔴）。

---

## 六、建议（接 v10.1 对账清单，未变）

1. **宪法闭环**：`build_messages` 真调 `constitution_prompt()`（0.5 天，闭合债务 #2）
2. **真接线三件套**：telemetry 计数器埋点 / retriever `build()` / webhook `fire()`（1–2 天，闭合 #1/#3/#4）
3. **补或正式砍**：civ 触发 / orchestrator 暴露 / workline 60s / 模型发现 / 工具生态（2–3 天，闭合 #5–#9）
4. **版本定锚**：打 `v10.2` tag + CHANGELOG + 同步 workspace version（当前仍 0.1.0）
5. **文档纪律**：交付报告测数一律以真机实测为准，禁止用 grep 声明数冒充 passed

**一句话**：v10.2 给"大脑"装了"神经"，资源越界会真喊停——这是真进步；但"身体"其余 8 根线还悬着，距离"全部通了"差得远。先闭环宪法 + 三件套，再谈新功能。

---

*审计签名*: Code Audit Gatekeeper（源码级 + 真机）  
*真机三门 RC*: 待回填（后台任务 `JvS89L`，见 §三）
