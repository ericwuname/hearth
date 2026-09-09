# codex-rust v10.1 全面代码审计报告（源码级 + 真机测试）

**审计日期**: 2026-07-29  
**基线**: v10.0 定版（156 tests, 🔴=🟡=0）  
**方法**: 不信报告信源码（`grep`+`Read` 逐文件核实）+ 真 Linux VM `cargo test --all`  
**结论速览**: v10.1 两项功能真实且带真断言；但**项目级存在系统性"定义未接线"债务（≥9 项）**，此前多份版本"过闸"在严格接线判据下需降级。

---

## 一、v10.1 范围内两项功能核实（✅ 真实）

| 项 | 源码锚点 | 核实 |
|----|---------|------|
| P1 `execute_plan` | `orchestrator.rs:340` | ✅ 真实泛型实现；`test_execute_plan_3_step_chain`（:363）真断言 `ok`/`completed==3`/`call_log` 顺序 |
| P2 `compute_roi` + `is_critical` | `resource-monitor/lib.rs:65,78` | ✅ 真实；`test_roi`/`test_critical`（:85,87）真断言 |
| 测试数 | — | grep `#[test]` 属性命中 **159**（与报告"159 passed"计数吻合，但"passed"需真机跑确认——见 §二） |

**范围诚实度**：报告把 L2 panic 捕获 / tool search·install / Observer 独立二进制标为 🔵 延后，与代码一致。`execute_plan` 与 `compute_roi` 目前均**仅被自身测试调用**，生产路径未接入（🔵2 集成、资源"行为层"告警均延后）——这点在报告"偏离"表里已如实披露。

---

## 二、真机测试 `cargo test --all`（✅ 已回填，RC=0）

- **执行环境**: 真 Linux `ssh wutao@192.168.220.131`（Ubuntu 24.04，cargo 可用）
- **同步方式**: tar（排除 target/.workbuddy/.git）→ SFTP → 解压 `~/codex_v11` → `find crates -name '*.rs' -exec touch` → `cargo test --all`（日志 `~/test_v11.log`）
- **首跑失败（环境错误，非代码）**: tar 把 Windows 上 0 字节的 `target` **文件**打包进去，VM 解压后 `target` 是普通文件，cargo 建 `target/debug` 报 `ENOTDIR` → `rc=101`。已修复（排除任何名为 target 的路径分量 + 构建前 `rm -rf target`）重跑。
- **最终结果**: **`TEST_DONE rc=0`**（全绿，无编译错误、无失败用例）。
- **真实执行口径（重要，纠正交付报告）**:
  - 日志 `test result:` 汇总：**109 个测试函数执行且全部通过，0 失败**（41 个 test-result 段落：16 个非零、合计 109 通过；其余 25 个为 0-passed 的 doc-test / 空 bin 段落）。
  - 代码库 `grep` 到的测试属性声明：**162**（`#[test]`=60 + `#[tokio::test]`=102）。
  - **交付报告宣称的"159 passed"不可直接复现为机器执行数**：实测默认 `cargo test --all` 只跑出 109 个通过，与 162 个声明之间存在 ~53 个缺口（疑为 feature-gated / 非默认成员目标未在默认 `--all` 跑出，或报告计数口径偏松）。守门员以机器实测 **109 passed / 0 failed** 为准——报告"159 passed"属未机器验证的口径偏松，记为一项⚠️诚实度问题（不计入 🔴 接线债务，但记入文档口径纪律）。

---

## 三、🔴 系统性"定义未接线"债务（独立核实，全景图文档指控成立）

守门员不盲信任何报告，对 `docs/global-panorama-v10.1.md` 的 🔴 指控逐条 `grep` 复核，**9 项全部成立**：

| # | 宣称"完成" | 源码真相 | 证据 |
|---|-----------|---------|------|
| 1 | v7.0 Telemetry 接入 | 计数器只 `load` 不 `fetch_add` → 端点恒返 0；telemetry crate 是孤儿（service/cli 都不依赖） | `routes.rs:435-445,456-458`；全仓无 `tc.session_count.fetch_add` |
| 2 | v10.0 基因宪法注入 | `constitution_prompt()` 定义+`pub use`，但 `build_messages` 从不调用 → 从未注入 system prompt | `constitution.rs:7`,`lib.rs:10`；`loop.rs:547` 无引用 |
| 3 | E1 retriever 默认开 | `set_retriever` 默认注入，但 `build()`/`index_tree` 在 session.rs **0 命中** → 索引永空、search 恒空 | `session.rs` grep 无 `build()`/`index_tree` |
| 4 | v8.0 webhook | `WebhookManager::fire` 定义，全仓零调用 → 永不触发 | `webhook.rs:33`；无 `.fire(` 调用 |
| 5 | v6.0 文明线自动触发 | `loop.rs` 2290 行对 civilization/civ **零引用** → 只能 CLI/路由手动发 | `loop.rs` grep 无 `civ`；`civ_store.append` 仅 `routes.rs`/`codex-cli` |
| 6 | v9.0/v10.1 Orchestrator | `execute`/`execute_plan` 仅自身测试调用；无路由、无 CLI、未接 AgentLoop | `orchestrator.rs:63/340` |
| 7 | v6.1 L3 WorkLine 60s 调度 | `main.rs` 唯一 `tokio::spawn` 是 observer 每小时任务，**无 60s 调度** | `main.rs:313-325` |
| 8 | v6A 模型自动发现 | `/api/v1/models` 只回内存注册表；无 `providers.yaml`/缓存代码 | 全仓无 yaml/cache |
| 9 | v10.1 工具搜索/安装生态 | 代码中不存在 | grep 零命中 |

---

## 四、守门员方法论自省（重要）

我此前出具的 `gatekeeper-review-v6.0` / `v10.0-final-audit` 等报告，判据是**"函数存在 + 测试通过"**。本次复核发现该判据过松：
- v6.0 审查时我核对了 `CivEntry`/`CivilizationStore`/路由/CLI 存在 → 给了 ✅，但**没验证 `loop.rs` 是否调用** → 文明线自动触发实际从未接线。
- v10.0 审查时核对了 `constitution.rs` 存在 → 给了 ✅，但**没验证 `build_messages` 是否引用** → 宪法从未注入。

**修正规则（即日起）**：每个宣称"完成"的功能，必须 `grep` 到从**生产入口**（HTTP 路由 / CLI 命令 / `loop.rs` 主循环）到该函数的**完整调用链**，否则只能判"已实现库函数，未接线"，不得判"过闸完成"。

---

## 五、版本卫生（全景图 §二）

- `Cargo.toml` workspace version = **0.1.0**（从未随版本演进）
- `git tag` = **空**（v3.0 冻结、v10.0 定版均未打 tag）
- 无 CHANGELOG / VERSION 文件；版本真相只存在于提交信息与 50 个根目录 md

---

## 六、闸门判定与建议

### 窄判据（仅 v10.1 自身范围）
P1/P2 真实 + 真机 `cargo test --all` **109 passed / 0 failed (rc=0)**，🔴=🟡=0 在"功能存在性"层面成立（但"通过数"口径以机器实测 109 为准，非报告宣称的 159）。

### 宽判据（项目整体）
存在 🔴 **系统性接线债务 ≥9 项**，且 v6.0/v7.0/v9.0/v10.0 的"过闸"在严格接线判据下需降级为"库函数存在、生产未接线"。

### 建议（按序，正式盖新楼前必须先对账）
1. **接线对账**：把 §三 9 项逐条标记"真接线 or 撤销宣称"，更新 governance.md（0.5 天）
2. **版本定锚**：打 `v10.1` tag + CHANGELOG + 同步 workspace version（0.5 天）
3. **真接线三件套**（小改动即可闭合一半 🔴）：telemetry 计数器埋点 / retriever `build()` / constitution 注入 `build_messages`（1–2 天）
4. **补齐或正式砍**：civ 触发 + workline 调度 + webhook fire + orchestrator 暴露（2–3 天）
5. **文档大扫除**：活文档刷新 + 一次性审计产物归档 `docs/archive/`（0.5 天）
6. **loop.rs 瘦身**（2290 行 god-file，抽 seam，含 SubAgentExecutor 旧债）（2–3 天）
7. 此后才谈新功能（模型发现 / 工具生态 / 多进程执行）

**一句话**：v10.1 的两项功能本身货真价实；但项目已冲进"宣称超前于接线"的债务期——地基是真的，楼层数被报高了。先对账、再补线、后盖楼。

---

*审计签名*: Code Audit Gatekeeper（源码级 + 真机）  
*真机测试 RC*: `TEST_DONE rc=0`（109 passed / 0 failed，默认 `cargo test --all`，见 §二）
