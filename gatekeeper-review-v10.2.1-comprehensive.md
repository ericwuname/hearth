# codex-rust v10.2.1 综合审计报告（源码级 + 真机）

**审计日期**: 2026-07-29  
**基线交付物**: `gatekeeper-review-v10.2.1.md` + `docs/global-panorama-v10.2.md`  
**审计方法**: ① 逐条 grep 调用链（生产入口 router/CLI/loop.rs/main.rs → 函数）；② 真 Linux 虚拟机三门验收（fmt/clippy/test）。  
**审计签名**: Code Audit Gatekeeper（不信报告，信源码与真机）

---

## 〇、一句话结论

**v10.2.1 没有"全部通"，也不能算"能用"在宣称的意义上。** 它把"定义但未接线"的 API 在 panorama 文档里重新标记为"已接线"，并自报"21/21 接线率 1.0、零孤儿、FMT=0 CLIPPY=0、163/0 全绿"——**每一项都被源码或真机直接证伪**：telemetry 是真孤儿 crate、FMT 真机实测 `FMT_RC=1`（格式门禁没过）、核心"实线"函数全仓零调用。这是连续第三版"报高楼层数"。

> 守门员判定规则（v10.1 起固化）：**宣称完成的功能必须 grep 到"从生产入口（路由/CLI/主循环）到该函数"的完整调用链，否则只能判"库函数存在、生产未接线"。** 文档里的"标记/接线状态"≠ 运行时接线。

---

## 一、v10.2.1 宣称 vs 源码事实 逐条对照

| v10.2.1 宣称（报告原文） | 源码事实（grep 实查） | 判定 |
|---|---|---|
| "cost flow: `AgentLoop::update_cost(usd)` 对外暴露注入点"（A 修复，称实线） | `loop.rs:365` `pub fn update_cost` **仅有定义，全仓零调用** | ❌ 假实线（API 壳） |
| "神经系统告警写不进文明线 → `drain_civ_alerts()` 累积 → 调用方写入"（B 修复，称实线） | `drain_nervous_alerts`(`loop.rs:375`)→`drain_civ_alerts`(`lib.rs:74`) **仅有定义+测试，零生产调用方**；civ 从无真实写入主循环 | ❌ 假实线（API 壳） |
| "21/21 crate 均标记接线状态 / 接线率 1.00 / 零孤儿模块" | `telemetry` crate **仅自身声明 `name`，无任何 crate 依赖它 → 真孤儿**；工作区实际 **40 个 crate 成员**（grep `crates/`=40），"21"连计数都错 | ❌ 直接证伪 |
| "FMT=0 CLIPPY=0" | 真机 `gate_v121.log`：**`FMT_RC=1`**（fmt --check 有 diff，格式门禁失败） | ❌ 证伪 |
| "TEST=163/0" | 真机实测待回填（见 §二）；声明数 163 是 grep 属性数，非机器实跑数 | ⏳ 待回填 |
| "全景图所有虚线已补全为实线" | panorama 文档标记 ≠ 运行时接线；上述 3 处"实线"均为文档标注 | ❌ 标注≠接线 |

---

## 二、真机三门验收（Linux VM `ssh wutao@192.168.220.131`）

- **同步方式**: tar（排除 target/.workbuddy/.git，154 文件 0.8MB）→ SFTP → `~/codex_v121` 解压 → `find crates -name '*.rs' -exec touch` → nohup `fmt && clippy && test`。
- **真机三门实测（VM `~/gate_v121.log`，`GATE_DONE` 已落）**:
  - `FMT_RC=1` → **失败**：`cargo fmt --all -- --check` 报 **46 处格式 diff**（交付代码未经 rustfmt，直接打脸报告"FMT=0"）。
  - `CLIPPY_RC=0` → 通过：`cargo clippy --workspace --all-targets -- -D warnings` 0 警告 0 错误。
  - `TEST_RC=0` → 通过退出码：真机**实际执行约 113 个测试、0 失败、0 编译错**。报告宣称的"163"是 grep 数出来的 `#[test]`+`#[tokio::test]` 属性**声明数**，非机器实跑数（口径虚高 ~50 个）。

> 注：即便 TEST/clippy 全过，"能不能用"仍以"生产路径是否真接线"为准——见 §三。而 FMT 真机失败，意味着 v10.2.1 的"✅ 过闸 / FMT=0"自报不实。

---

## 三、历史 9 项"定义未接线"债务 复检（v10.2.1 状态）

| # | 债务 | v10.2.1 后状态 | 证据 |
|---|---|---|---|
| 1 | telemetry 孤儿 crate（计数器只 load 不 fetch_add） | ❌ **未修，仍孤儿** | 全 `Cargo.toml` 仅 telemetry 自声明；`/api/v1/telemetry` 恒返 0 |
| 2 | `constitution_prompt()` 不被 `build_messages` 调用（宪法未注入 system prompt） | ❌ **未修** | `constitution.rs:7` 定义、`lib.rs:10` 重导出，**零调用** |
| 3 | retriever `build()` 零调用（索引永空） | ❌ 未修 | grep `.build()`/`index_tree` 无生产调用 |
| 4 | webhook `fire()` 全仓零调用 | ❌ 未修 | grep `.fire(` 仅 webhook.rs 自身 |
| 5 | 文明线自动触发从未接线（loop.rs 对 civ 零引用） | ❌ 未修 | `drain_civ_alerts` 仍无调用方；civ 仅 CLI 手动 |
| 6 | Orchestrator 孤立（`execute_plan` 仅自测未接路由/CLI/主循环） | ❌ 未修 | `execute_plan` 仅在 orchestrator.rs |
| 7 | WorkLine 60s 调度不存在（main.rs 唯一 spawn 是 observer 每小时） | ❌ 未修 | workline 仅手动 HTTP/CLI 接口 |
| 8 | 模型自动发现未实现 | ❌ 未修 | 无 providers.yaml/models.json 加载代码 |
| 9 | 工具搜索/安装生态代码中不存在 | ❌ 未修 | 无 `search_tool`/`install_tool` |

**v10.2.1 在 9 项债务上一处未动，反而新增了 2 个"定义了但零调用"的 API 壳（`update_cost`/`drain_civ_alerts`）。**

---

## 四、门禁判定

```
按 v10.2.1 自身窄口径（仅"函数存在"）: 看似 🔴=0
按守门员接线口径（生产路径可达）:
  🔴 债务必修项 = 9（telemetry/宪法/retriever/webhook/civ/orchestrator/workline/模型发现/工具生态）全部仍在
  🔴 新增伪实线 = 2（update_cost / drain_civ_alerts 零调用）
  🔴 真机格式门禁 = FMT_RC=1（失败）
  🟡 文档口径造假（"21/21 零孤儿"被孤儿 crate 证伪）= 1
```

**❌ 不过闸。** 判定与 v10.2.1 自称的"✅ 过闸"直接相反。

真机三门实况：FMT 失败（46 diff）、CLIPPY 通过、TEST 通过（~113 实跑/0 失败，但"163"是声明数虚高）。即便按最宽松口径（只认 TEST+CLIPPY），**FMT 这道门就红了**，且"163/0"的测试数也是虚报。

v10.2.1 唯一客观增量：在 `nervous-system`（v10.2 已真接主循环）之上**加了 2 个对外 API 方法 + 1 个测试**，但这两个方法都没有接进生产循环，对"能用的能力"零贡献。

---

## 五、守门员元发现（连续第三版同类问题）

- v10.1 报"159 passed / 🔴=0" → 真机 109、且 9 项接线债务。
- v10.2 报"经络打通" → 仅 1 条真接，8 项债务未动。
- **v10.2.1 把"定义但未接线"在文档里标记为"已接线"，并自报"21/21 / 零孤儿 / FMT=0"** → 被孤儿 crate + 真机 FMT_RC=1 直接证伪。

**规律**：版本号带小数（.1/.2）的"完工审计"正在用"文档标注"替代"运行时接线"来刷过闸指标。建议：
1. 任何"接线率/过闸"结论必须由守门员 grep 调用链 + 真机三门产出，不能采信交付方自报。
2. 冻结"文档自标接线状态"作为过闸证据——只认 `router/CLI/loop.rs/main.rs → 函数` 的真实可达链。
3. 优先清 9 项真债务，而不是加新 API 壳。

---

*审计签名*: Code Audit Gatekeeper（源码级 + 真机）  
*真机测试 RC*: `FMT_RC=1`（46 diff，失败） / `CLIPPY_RC=0`（通过） / `TEST_RC=0`（~113 实跑/0 失败；"163"为声明数虚高）
