# Hearth 长程优化探索规划 v1.0（**内容已修订至 v1.1**）

- **出品**：traecode　**日期**：2026-09-30（v1.1 修订：2026-10-01）
- **签发**：用户（全权授权，无需逐轮汇报）
- **基线**：`p0-usability-01` @ `e4393f2`（= 远端 `origin/main`）　**版本**：0.2.27
- **授权口径（用户原话）**："全部自主，无需汇报，你只看最后的结果"；北极星三项；**agent-core 先体检再定**。

---

## 〇、v1.1 修订记录（2026-10-01）

**二十三张卡已收官**（每卡独立 commit + 全量门禁 + 推送，门禁目标数 **61 → 62 → 60 → 62 → 58**）：
**顶层七项裁决已下（2026-10-01）**——见《P1-03·04 战报》第一节。

| 卡 | commit | 战果 |
|---|---|---|
| P0-01 | `8ad4822` | Windows 路径逃逸（**真实安全漏洞**）+ xray 引擎 UTF-8 缺陷 + hash 锁欠账 + 4 项平台门控 |
| P1-01 | `b0c89bc` | 逐 crate 体检：3 个"只写不读"死接线 + `project-sync` 整 crate 死亡 + 测试盲区 |
| P1-02 | `917e3cf` `1729646` | xray 字符串状态缺陷 + 3 条能力锁定处置 + 死接线加锁 → **全量门禁首次全绿** |
| P0-02 | `503911c` `e4393f2` | **4 项注入/SSRF 安全缺陷**（命令注入 / 选项注入 ×2 / 重定向 SSRF） |
| P0-04 | `539f4fc` | 会话无界泄漏（高 DoS）+ webhook 静默失败 + 打开结果误报；D-23 专项再命中 2 处 |
| P0-05 | `d8da9d0` | **服务端 DoS 面**：D-24/D-25 + 新发现 D-27（限流计数 panic 泄漏 → 整站永久 429）/D-28 |
| P0-06 | `30fc2cb` | **有界输出/有界读取 OOM 面**：D-18/D-21 + 新发现 D-29（Windows 运行时 `Command::output()` 无界）；**并打通 Linux 目标交叉类型检查（D-30）** |
| P0-07 | `e7d94a8` | **子进程/进程树回收**：D-19（fail-closed 留孤儿）/D-20（超时不杀子孙进程，`taskkill /T`）；**两处红→绿实证**；登记 D-31/D-32 |
| P0-08 | `c1e1816` | **无界读入第 4 落点**：D-31 `open_artifact`（HTTP 请求路径）→ `read_text_capped`(8 MiB) + 留痕 |
| P0-09 | `0ffd08b` | **冻结区单卡**：D-32 S12 自检命令——超时不收尸（`cargo check`+rustc 后台空转）+ 输出无界；红检中还抓出**自己引入的挂起风险**；登记 D-33 |
| P0-10 | `e26cf57` | **D-23 专项收敛**：核验 9 处"注释声称的防护层"，**4 处证伪并订正** + D-26；由此暴露 D-35（bash 出网无治理）/D-36 |
| P0-11 | `c55813e`+`be100f9` | **入库卫生**：D-5 `.hearth-diag` 脱离跟踪（40 文件，内容留盘）+ D-6 **密钥扫描门禁**（三轮红→绿，2255 文件）；修正泄露面积 → 登记 D-37 |
| P0-12 | `a60dd3a` | **索引路径病态输入防护**：D-38 无界读入（第 5 落点）+ 红检中量出的 **D-39 二次方膨胀**（8 MiB 平原文件 → 挂 2 分钟以上）；新门禁首跑即抓到自己的源码/战报 |
| P1-03 | `79f0c66` | **裁决3 · 删死 crate**：`project-sync`（12 文件）删除，工作区 62→60 target |
| P1-06 | `9e5db3a` | **门禁口径对齐 CI**：发现 CI clippy 长期红（本地无 `-D warnings`）→ 修 9 处报错（含 3 处我引入）+ 门禁四件套升级 + Linux 交叉检查列入；登记 D-42 flaky |
| P1-05 | `42ae8a7` | **裁决6 · D-33 收敛**：新建 `bounded-io`，四处"有界读入/树杀"合一（60→62 target） |
| P1-04 | `ac31cf0` | **裁决4 · 删死接线**：`retriever`/`lsp_bridge` 三层贯通删除 + 依赖清理 + 退役钉住测试；**顺带停掉"启动时全量扫盘建索引 / 起 rust-analyzer"的真实成本**；登记 D-40 |
| P1-07 | `24a48e9` `ea2da1b` | **CI 转绿（D-43 闭环）**：修好 clippy 后 test 步首次真正执行 → 暴露 **4 例 Linux 用例因 cgroup 不可用被 RT4 fail-closed 打死**；按项目自己的门禁口径在 CI 建 cgroup delegation 子树（`/sys/fs/cgroup/hearth` + 递归 chown + 下发 memory/pids/cpu）→ **本仓 CI 19 天来首次全绿**（fmt/clippy/test/xray wiring/xray scan 七步全过） |
| P1-08 | `cc01df3` | **裁决5 · 追认 2 条退役能力**：xray spec 两处 claim「待顶层追认」→「顶层已追认」+ 门禁提示文案 + 同步重锁 FNV（`0x3854ddaf16e6dd1d`→`0xfa4df73ba26cd516`）；**纯文案，条数/severity/锚点零变化**；顺带删掉债队列中重复过期的 D-16 行 |
| P1-09 | `cd8e00a` | **D-42 收口（flaky 门禁）**：定位到真因 = `test_maybe_compact_folds_old_turns` 触发压缩却不持 env 锁 → 把 8 轮追加进并行测试的归档文件 → 对侧 `archive_digest(max_turns=8)` 按行序截断挤掉自己的 turn#102。**确定性复现 → 修复 → 5× 全跑 0 失败**；顺带修掉"该测试污染真实 HOME" |
| P1-10 | `c771102` | **D-40 收口（LSP/检索残链）**：两个无生产者事件变体 + 其唯一消费方（`agent-runtime` 映射 + 两个恒不触发的 scratch 注入块）+ `PlanContext` 的两个死字段与 planner 注入 + **只剩类型宿主作用的 `retriever`/`lsp-bridge` 两 crate** 一并删除（27→25 包，摘掉 tantivy 与 rust-analyzer JSON-RPC 依赖）；新登记 D-44 |
| P1-11 | `289d952` | **D-44 收口（API 声称 ≠ 实际）**：`GET /api/v1/tools` 的名单与启动注册合一到唯一事实源 `builtin_tools()`——原 13 项里 7 项是幽灵（名字错/不存在/非工具）；**先红后绿**（红：`API 声称了从未注册的工具名 lsp_diagnostics`） |
| P1-12 | 本卡 | **「声称 ≠ 实现」专项 · 第二轮（D-23 续）**：核 6 个尚未体检的"机体" crate（resource-monitor/nervous-system/subconscious/experience/bridge/observer），**证伪 2 处安全/治理声称**——① observer 的「L2 fail-closed」（`Err` 后调用方只 warn，会话照常继续）② 成本治理链整条失效（`with_budget`/`update_cost` 双零调用 → `cost_ratio()` 恒 0 → `CostGuard` 永不触发）；**已订正注释并登记 D-45/D-46/D-47 待顶层裁**；另清点 5 处"定义了但生产无人用"的死能力 |

**基线变化**：失败 target **3 → 0**，失败用例 **13 → 0**，门禁从"常红 19 天"转为**全绿**（P1-07 后为**真·CI 绿**：本地口径与 CI 口径均已实证通过）。

### 方法论修正（**本文档最重要的一节**）

四张卡反复验证同一件事，且强度递增：

> **"全量门禁全绿 + clippy 零警告" ≠ "没有缺陷"。**
> P0-02 的 4 项安全缺陷 **100% 存在于 61 target 全绿的状态下**。

**根因**：现有测试只做**正向断言**（"能做到 X"），**从不做边界断言**（"不能做到 Y"）。
两类反复出现的缺口：
- **接线断言**缺口 → "只写不读"能潜伏（P1-01）
- **安全边界断言**缺口 → 注入/越界/SSRF 能潜伏（P0-02）

**据此调整后续方向**：从"读代码找问题"升级为**"补边界断言"**——
每修一个边界缺陷，**同时补一条负向测试**，让同类缺陷**不能再悄悄回来**。

---

## 一、授权与边界

| 项 | 内容 |
|---|---|
| 北极星（用户多选） | ①**真实任务成功率**（产物导向）②**工程质量与架构健康** ③**性能与成本**（token/墙钟/延迟） |
| 未授权 | **能力扩展**（新工具/新能力）——用户未选，本规划**不做**，防止过度设计 |
| 自主度 | 全自主，不逐轮汇报；**仅**在破坏性操作、方向抉择、或卡死时停下 |
| 核心冻结 | agent-core 主循环（8.1k 行）**先出体检报告，改动需单独立卡** |
| 度量环境 | 本机 Windows MSVC（Linux VM `.131/.133` 不可达）；**Linux 专用代码改用 `cargo check --target x86_64-unknown-linux-gnu` 交叉类型检查**（P0-06 打通，能力登记见 D-30） |
| **口径断点** | 模型已切 `agnes-3.0-flash`。**凡 3.0 跑出的数字必须标 "3.0"，禁止与 2.5 时代数据混标**（登记见《P0P1 修复战报》7.2） |

---

## 二、运行机制（每轮固定节拍）

```
取证体检 → 排优先级 → 立一张卡 → 先红后绿 → 全包门禁 → 独立 commit → 下一轮
```

### 门禁四件套（**P1-06 起，按 CI 口径**）

> 教训（P1-06）：P0 阶段报的"全绿"用的是**更弱的本地口径**（clippy 未加 `-D warnings`），
> 而 CI 一直是红的。**"我这边绿" ≠ "CI 绿"。**

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings    # ← 必须带 -D warnings（与 CI 同）
cargo test --workspace --no-fail-fast
cargo check -p sandbox --target x86_64-unknown-linux-gnu --all-targets   # Linux 专用代码交叉检查（裁决 7）
```

**注意**：`cargo test --workspace` 存在**已知 flaky**（D-42，`HEARTH_ARCHIVE_FILE`
进程全局竞态，约 1/3 概率出现）——报"全绿"时须注明"本次跑"。

- **每轮 = 1 张卡**（小步、单一主题、可独立回滚）
- **每阶段 = 一组卡 + 一份阶段报告**
- 严禁"顺手改"：发现的新问题一律**立卡**，不在当前卡内夹带

---

## 三、阶段划分

| 阶段 | 目标 | 完成判据 |
|---|---|---|
| **P0 · 收拢已知债** | 把散落各处的申报项核成事实队列，逐项 close；清零既有失败 target；止血入库卫生 | 债队列全部有终态（已修 / 已归因 / 已登记） |
| **P1 · 逐层体检** | 27 个 crate 按体量/风险排序体检：死代码、未接线模块、依赖健康、测试盲区、跨平台 | 每个 crate 一份体检结论 |
| **P2 · 性能与成本** | token 消耗 / 墙钟 / 延迟 / 并发门控的量化与优化 | 有基线、有对照、有改善（全部标 3.0） |
| **P3 · 真实任务成功率** | 端到端跑真实任务，看产物是否交付 | 受限（需真机/真 key），能做多少做多少 |

**顺序推进，不并行。**

---

## 四、护栏（红线，沿用项目既有）

1. **沙箱 crate 语义零触碰**（landlock/seccomp/cgroup 的隔离承诺不动）
2. **`a_arm_act_tally` 9 处原位**
3. **key 永不入 git**（本次已出事故，见第八节）
4. **S5 已拆机制禁止复活**
5. **不静默降级**——限制不可用要么报错要么显式选择
6. 每卡**先红后绿** + 全包门禁 + 独立 commit

---

## 五、停手升级条件

遇到以下情况**停下并上报**，不擅自继续：

- 破坏性 / 不可逆操作（强推、历史重写、批量删除、改远端设置）
- 需要产品方向抉择（改行为语义、动用户可见契约）
- 违反任一红线
- 连续多轮零收益（防止为动而动）

---

## 六、事后追加：本次会话的三项纪律补强

| # | 教训 | 补强 |
|---|---|---|
| 1 | 文档记载的"待办"可能是**旧账**——`max_tokens` env 化在文档里仍标"待顶层裁定"，实测**代码里早已落地**（`agent-types::max_output_tokens()`，默认 65536，带单测） | 债队列**逐项核对代码**，不照抄文档 |
| 2 | 一次 `git ls-remote` 暴露出本地 `origin/main` 是**过期引用**，差点误判同步状态 | 同步判断一律以 `fetch` 后为准 |
| 3 | **脱敏是一次性动作**——`6871f7a` 用一次"文档归档"就把它撤销了 | 需要**常设**入库前扫描门禁，而非事后补救 |

---

## 七、基线（本机 Windows MSVC，2026-09-30 实测）

`cargo test --workspace --no-fail-fast`：

| 包 | 结果 |
|---|---|
| `agent-core` | **148 passed / 0 failed** |
| `tools-builtin` | **74 passed / 11 failed** |
| `sandbox` | **5 passed / 1 failed** |
| `project-xray` | **1 passed / 1 failed** |
| 其余各包 | 全绿 |
| **合计** | **失败 target 3 个，全部既有/环境，零新增**（与《P0P1 修复战报》第八节记载一致） |

**3 个失败 target 的性质（初步）**：均为 **Windows vs POSIX 语义差**——
- `tools-builtin` 11 项：POSIX bash + Unix 绝对路径语义
- `sandbox::test_noop_sandbox_echo`：需要 POSIX `echo`
- `project-xray::real_workspace_wiring_all_green`：读已被删除的 `docs/xray/wiring-v13.toml`

**待判**：这些是"测试应当平台门控"，还是"代码真有跨平台缺陷"——**这决定修法，是 P0 第一张卡的核心问题**。

---

## 八、安全事项登记（已裁决，不再重复上报）

2026-09-30 GitHub 同步核查中发现：公开仓库 `github.com/ericwuname/hearth` 的 HEAD 中含 **3 把明文 API 密钥**（`cpk-f4UBH3Na…` / `cpk-TbY5hUhE…` / `sk-omwq7hA6…`），公开约 10 天（09-20 → 09-30），**根因是 `6871f7a` 的文档归档提交把两天前已脱敏的密钥重新引入**。

**用户裁决（2026-09-30）：不轮换、保持 public、不做历史清理——登记为显式风险接受。** 详情与复发风险见
`P0安全事故报告·公开仓库明文密钥泄露 v1.0（2026-09-30）.md`（仓库根目录，**有意不 commit**）。

**复发机制未消除**：`.hearth-diag/` 未 gitignore 且已被跟踪，是下一次误入库的现成入口 → 已立 P0 卫生卡。

---

## 九、债队列（核对后的初版）

| # | 项 | 来源 | 真实状态（核对后） | 处置 |
|---|---|---|---|---|
| D-1 | `max_tokens` env 化 | P0P1 战报申报 3 | **已完成**（`agent-types::max_output_tokens`，默认 65536，带单测表） | **close（旧账）** |
| D-2 | `tools-builtin` 11 项测试红 | P0P1 战报申报 2 | **已核**：其中 **8 项是真实安全漏洞**（见 D-9 修复），**4 项是 POSIX 平台依赖** | **close** |
| D-3 | `sandbox::test_noop_sandbox_echo` 红 | 同上 | **已核**：POSIX `echo` 依赖，非 noop 后端缺陷 | **close（平台门控）** |
| D-4 | `project-xray::real_workspace_wiring_all_green` 红 | 同上 | **原归类错误**（"读已删除文件"不成立）。真因 = hash 锁未随 `26d760e` 更新 + 3 条能力锁定冲突 | **部分 close**，余项见 D-11 |
| **D-5** | `.hearth-diag/` 入库卫生 | 本次发现 | **已修**：`.gitignore` + `git rm -r --cached` 40 个文件（**内容留盘**）；`git status` 恢复干净 | **close**（删盘上文件/清历史仍待裁决） |
| ~~**D-16**~~ | `retriever`/`lsp_bridge` 只写不读死接线 | P1 体检 | **已删除**（顶层裁决「删除」，P1-04）：三层贯通（agent-core/agent-runtime/service）+ 依赖清理 + 退役钉住测试；**顺带停掉"启动时全量扫盘建索引 / 起 rust-analyzer"的真实成本** | **close** |
| **D-41** | `project-sync` 整 crate 死亡 | P1 体检 | **已删除**（顶层裁决「删除」，P1-03）：12 文件 / ~90 KB，工作区 62→60 target | **close** |
| **D-40** | `Event::LspDiagnostics` / `Event::Retrieval` 事件变体**全仓无生产者**（提示词格式化仍有消费方） | P1-04 执行中发现 → P1-10 收口 | **已修**：整条残链一并删除——两个事件变体 + `agent-runtime` 的两条映射臂 + `build_messages` 两个 scratch 注入块（**同样无写入方**，属"读方还在、生产方已无"）+ `PlanContext.retrieval_context`/`lsp_diagnostics` 两字段与 planner 注入 + **仅剩"类型宿主"作用的 `retriever`/`lsp-bridge` 两个 crate**（裁决4 收尾）。工作区 27→25 包，顺带摘掉 tantivy / rust-analyzer JSON-RPC 客户端依赖 | **close** |
| **D-44** | `service/src/routes.rs` `GET /api/v1/tools` **硬编码**列出 `lsp_diagnostics` / `lsp_hover` / `code_index_search` 三个**全仓从未注册**的工具名（真正注册的只有 bash/read_file/write_file/edit_file/grep/glob/web_search/…）——API 对外"声称可用"而实际不可调用（与 D-23「注释声称的防护层」同型：声称 ≠ 存在） | P1-10 体检发现 → P1-11 修复 | **已修**（P1-11）：名单改为由**唯一事实源** `builtin_tools()` 派生，启动注册也改从该表取——两份各自维护的名单合一，**结构上不可能再漂移**。实测该表只有 6 项：`bash` / `read` / `write_file` / `apply_patch` / `glob` / `grep`；原 13 项里 7 项是幽灵（`read_file`/`edit_file` 名字错，`lsp_*`/`code_index_search` 不存在，`send_message`/`approve` 是 API 动作）；连 `web_search`/`web_fetch` 在 service 侧实际也**未注册** | **close** |
| **D-45** | **observer 的「L2 fail-closed」声称不成立**：`observer/src/lib.rs` 模块注释写"`Observer::run()` 返回 `Err` → 由 service 层拒绝继续"；实测 `agent-runtime/src/session.rs:785-790` 拿到 `Err` 后**只 `tracing::warn!`，会话照常跑完**。另一半"构造失败 → 拒启"亦恒不触发（`Observer::new()` 无失败路径，main.rs:652 自述承认）。文档 `version-iteration-manual.md` 同样宣称"seq 断档 → Err，协调者拒启" | P1-12 核验（D-23 续） | **已证伪 + 已订正注释**（源码注释如实写明"不存在该保证"并指向本条）。**是否真做 fail-closed（`Err` → 中止会话/拒启）＝产品方向抉择** | **待顶层裁** |
| **D-46** | **成本治理链整条失效（`CostGuard` 在生产中永不触发）**：① `NervousSystem::with_budget()` **仅测试调用** → `cost_budget_usd` 恒 `None` → `cost_ratio()` 恒 `0.0`（`AgentLoop` 用 `NervousSystem::new()`，loop.rs:1627）；② 累加侧 `AgentLoop::update_cost()`（loop.rs:1098）**全仓零调用者**→ 即便设了预算分子也恒 0；③ `loop.rs:2874` 把该恒 0 值喂给 subconscious `CostGuard`（`cost_ratio > 0.80/0.95`）→ 分支永不成立。而 `CHANGELOG-v16.md` / `gatekeeper-audit-v16.md` / `forge-v16-plan.md` 均宣称"CostGuard 真值接地 ✅"——**只接地了"读"，没接地"预算"与"累加"** | P1-12 核验 | **已证伪 + 已订正注释**。**接通真实成本核算需定价口径（token→USD）＝产品方向抉择** | **待顶层裁** |
| **D-47** | **机体 crate 体检清单（"定义了但生产无人用"）**：① `subconscious` 的 `LspGuard` / `ExperienceMatcher` 从未 `add_guard` 注册（`add_guard` 仅测试调用）→ 恒不生效，`repetition` 字段"mutated externally"但全仓无写入方；② `experience::search()`（语义检索）全仓仅测试调用 → 整块检索能力死（`reinforce`/`evaluate` 同）；③ `bridge` **整 crate 零测试**，且 `let (event_tx, _) = ...` 丢弃接收端 → `BridgeEvent` 事件流无任何消费者；④ `nervous-system::drain_civ_alerts` 读的 `civ_pending` 唯一写方是 `query()`，而 `query()` 生产零调用 → agent-core 侧恒取到空；⑤ `observer::is_tripped/tripped` 零调用零赋值（P1-01 已记） | P1-12 体检 | 逐项**已核**（见各 crate 源码注释）；性质=架构健康/死能力，非缺陷 | **待裁**（逐项：接线 / 删除 / 保留留痕） |
| **D-42** | **全量测试 flaky（≈1/3 概率）**：`HEARTH_ARCHIVE_FILE` 是进程级全局 env，agent-core 内十余个测试在读写它，`ENV_SER` 串行锁**未覆盖全部触点** → 并行执行时互踩 | P1-06 全量跑实测 → P1-09 定位 | **根因已定位 + 已修**：`test_maybe_compact_folds_old_turns` **会触发压缩**（→ 经 `archive_path()` 读进程全局 env）却**不持锁**，把本测 8 个轮次追加进并行测试 `test_r56_*` 的归档文件；对侧 `archive_digest(sid, 8)` 有 `max_turns=8` 上限且**按行序截断** → 对侧自己的 turn#102 被挤出窗口 → 假红。**已确定性复现**（digest 打印 100/101 后接 0..5，102 消失）→ 修复 = 持双锁 + 显式临时归档 + 复原（顺带消除"压缩不设 env 时污染真实 HOME"）。纪律写入 tests 模块头 | **close** |
| **D-43** | **本仓 CI 长期红**（clippy 步 `-D warnings`）与本地口径不一致——已在 P1-06 修掉全部报错；P1-07 修好 clippy 后 test 步**首次真正执行**，又暴露 4 例 Linux 红（cgroup） | P1-06 发现 → P1-07 闭环 | **已修 · 已实测**：CI run `36765112148` **七步全过**（cgroup delegation/fmt/clippy/test/xray wiring/xray scan）——**19 天来首次绿**。根因链：clippy 红 → test 步从未跑 → 4 例 cgroup 依赖用例长期无人知 | **close** |
| **D-6** | 无入库前密钥扫描门禁 | 安全事故根因 | **已建**：`crates/codex-cli/tests/secret_scan_gate.rs`——扫 `git ls-files`、三轮红→绿、白名单 3 项、文件头自报盲区；门禁目标数 61→62 | **close** |
| **D-38** | `code-index` `parse_rust_file` 无界读入（第 5 个"无界读入"落点；遍历的是会话 workspace） | P0-12 扫描 | **已修**：先 `metadata().len()` 再看要不要读（8 MiB 上限），超限**跳过并留痕** | **close** |
| **D-39** | `code-index` 索引路径**二次方膨胀**：`for i in 0..child_count() { node.child(i) }`（O(n²)）+ 每 chunk `source.lines().skip(row)`（O(L·k)）——**不给大小守卫就永远看不到**（症状被守卫掩盖） | P0-12 红检实测 | **已修**：`children(&mut cursor)` + 预切行；红侧实测 **>2 分钟 → 2.40s** | **close** |
| **D-37** | **真 key 的实际落点是 3 个文件**（此前只记 1 个）：`r12-dot-bashrc-anchor.txt`(×4) / `v20-user-transcript-f852f409.jsonl`(×1) / `TUI-polish3…v1.1`(×1) | P0-11 扫描 | 确证（**修正泄露面积判断**） | **close · 顶层裁决「不清理」（显式风险接受，2026-10-01）**；已入密钥扫描门禁白名单 |
| D-7 | `.131/.133` VM config 未同步 3.0-flash | 战报申报 1 | **阻塞**（VM 不可达） | 挂起 |
| D-8 | 执行窗复验 5 项（PC-1/2/3、回归、S12 等） | 战报第九节 | **属执行窗/用户** | 不代办，仅备件 |
| **D-9** | **`tools-builtin` 四工具路径守卫在 Windows 失效 → 越界写文件（真实安全漏洞）** | **本卡新发现** | **已修**（`is_rooted_path`，5 处） | **close** |
| **D-10** | **xray 剥离器 UTF-8 缺陷：`u8 as char` 使所有非 ASCII 锚点静默失效** | **本卡新发现** | **已修** + 回归测试 | **close** |
| **D-11** | **xray 3 条能力锁定 vs 已被手术删除的能力冲突**（`all-done-requires-write`/`sub-budget-not-halved`/`no-toolcalls-requires-write`） | 本卡新发现（**《线C手术验收报告》当年已"呈顶层裁"，未销账**） | **已处置**：1 条收窄到存活防线（转绿）、2 条登记退役（yellow 留痕、不阻断）。**顶层已追认「退役」（2026-10-01，裁决5）**：spec 两处 claim 的「待顶层追认」改为「顶层已追认」，并同步重锁 spec FNV 哈希（`0x3854ddaf16e6dd1d` → `0xfa4df73ba26cd516`；**纯文案，条数/severity/锚点零变化**） | **close** |
| **D-12** | **xray 剥离器不跟踪字符串状态：字符串中的 `/*` 被误判为块注释 → 整文件 79% 被吞，锚点假阴性（`bash.rs` 实测 43132→9081 字节）** | 本卡新发现 | **已修**：引入字符串/原始字符串/字符字面量词法状态 + 2 条回归测试；**bash.rs 环重挂并通过 = 端到端验证** | **close** |
| **D-15** | xray 集成测试与引擎 `has_red_break()` **口径不一致**（前者无条件要 `broken.is_empty()`，后者仅 red 阻断） | 本卡新发现 | **已修**：对齐为"仅 red 失败"，yellow 断裂**打印留痕**（不静默） | **close** |
| **D-13** | 仓库提交态非 fmt-clean（`cargo fmt --all` 改动了未触碰的 `codex-cli/src/session_store.rs`） | 本卡发现 | 既有 | P1 立卡 |
| **D-14** | `SPEC` 顶部注释亦为旧口径（写"15 能力/24 链"，实测 16 / 27） | 本卡发现 | **已订正**（commit `1729646`） | **close** |
| **D-17** | **会话泄漏 → 无界内存 + 磁盘**（从未 `send_message` 的会话 `finished_at` 恒 `None` → 落入"Active: always keep" → 永不回收；工作区目录同泄漏） | P0-02 审计 → P0-04 | **已修**：`Session` 加 `created_at`，`None` 分支按 idle TTL 回收非运行中者（`running=true` 永不触碰）；回归测试已重写 | **close** |
| **D-24** | `service/src/per_user.rs` 请求路径上的 `.expect()` + **panic 持锁 → 锁中毒 → 其后所有 `civ.write().unwrap()` 二次 panic → civ 接口全站 DoS** | P0-04 专项扫描 | **已修**：签名改 `Result` 上抛 + `lock::recover` 中毒不升级；`webhook`/`user` 同类项一并清理 | **close** |
| **D-25** | `service/src/routes.rs` 限流中间件 `lock().unwrap()`——中毒后**每个请求** panic（影响面全站） | P0-04 专项扫描 | **已修**：`recover` | **close** |
| **D-27** | **限流在途计数在 handler panic 时单调泄漏**（尾部手动 `fetch_sub` 在 unwind 中不执行）→ 同一 IP 泄漏 50 次即该 IP 永久 429、全局泄漏 500 次即**整站永久 429** | P0-05 新发现 | **已修**：改 RAII `InflightGuard`（`Drop` 在 unwind 时仍执行）+ 单测锁定 | **close** |
| **D-28** | `service/src/user.rs`（**每请求鉴权路径**）与 `webhook.rs` 的 `lock().unwrap()`——同 D-24/D-25 缺陷类，残留即全站级失败通道 | P0-05 新发现 | **已修**：`recover` | **close** |
| **D-26** | `tools-builtin/src/web.rs:9` 注释声称"bash curl 后门治理**依赖全局代理 env**"——**该机制全仓零实现**（与 landlock 案例同型） | P0-04 专项扫描 | **已修**：订正为"出网旁路已知未治理"并指向 D-35 | **close** |
| **D-35** | **bash 出网无任何治理**：`web_fetch` 有 fail-closed 白名单，但 bash 的 `curl`/`wget` 可直连任意地址（原被 D-26 的虚构机制遮住） | P0-10 专项 | 确证（**真实防护缺口**） | **close · 顶层裁决「接受」（显式风险接受，2026-10-01）** |
| **D-36** | `service/src/webhook.rs:69` `curl … .output()` 无界输出 + `kill_on_drop` 默认 false（超时不收尸），与 D-18/D-32 同类（在 fire-and-forget 任务里） | P0-10 专项 | 确证 | 低 · 待修 |
| **D-18** | **子进程输出全量入内存 → OOM**（读线程 `read_to_end` 无上限，截断发生在其后） | P0-02 审计 | **已修**：`drain_capped_std`——继续排空到 EOF 但只留前 8 MiB + 留痕 | **close** |
| **D-29** | **NoopSandbox（Windows 运行时路径）`Command::output()` 无界读** → 同 D-18 的 OOM，且落在**本项目主力平台** | P0-06 新发现 | **已修**：spawn + 两路 `drain_capped_async` 并行有界排空 | **close** |
| **D-30** | （**能力，非债**）**Linux 专用代码的交叉类型检查**：`rustup target add x86_64-unknown-linux-gnu` + `cargo check --target …` 实测可跑通（本次已验证），补上"`cfg(target_os=linux)` 代码本机不参与编译、改了没人验"的缺口 | P0-06 附带产出 | 可用；**顶层裁决7「列入」** → 已并入门禁四件套（P1-06 起，见本档第二节） | **close · 已入常规门禁** |
| **D-19** | **cgroup 失败路径泄漏子进程**（`Child` 被 drop，Rust 不 kill 不 wait） | P0-02 审计 | **已修**：失败分支组杀 + `wait()` 收割 + 清理 cgroup 再上抛；Linux-only 测试已补（本机交叉类型检查） | **close** |
| **D-20** | **NoopSandbox 超时不杀进程组**（Windows 走此路径，孙进程成孤儿） | P0-02 审计 | **已修**：`taskkill /T /F` 树杀（无新依赖）；**本机实测红→绿**（孙进程哨兵法） | **close** |
| **D-31** | `agent-runtime/src/session.rs` `open_artifact`（**HTTP 请求路径**）`read_to_string` 无字节上限 | P0-07 扫描 | **已修**：`read_text_capped`（8 MiB）+ UTF-8 边界回退 + 响应体末尾留痕；3 条边界测试 | **close** |
| **D-32** | `agent-core/src/loop.rs` `run_check_cmd`（S12 自检）`Command::output()` 无上限且**超时不收尸/不树杀** | P0-07 扫描 | **已修**：spawn + 有界并行排空 + 超时树杀（taskkill /T）+ **有界**收割；先红后绿实证 | **close** |
| **D-33** | 三处各自持有一份"有界排空"实现（`sandbox` / `tools-builtin::read` / `agent-core`）——是否收敛到共享 crate | P0-09 归纳 | 确证（**架构重复**，非缺陷） | **close**（顶层裁决6「收敛」，P1-05：新建 `bounded-io`，四处合一，60→62 target） |
| **D-21** | `read` 无**字节**上限（限的是"2000 行"，单行超大整行入内存） | P0-02 审计 | **已修**：`take(8 MiB + 1)` 有界读取 + UTF-8 边界回退 + 输出末尾留痕 | **close** |
| **D-22** | **家目录放行依赖 landlock 兜底，而该兜底只在 Linux 存在** —— 复核定级为**条件性真洞**（原生 Windows 因 `HOME` 为空而不显现；**Git Bash 启动时 Git for Windows 设置 `HOME` → 洞出现**） | P0-02 审计 → P0-03 专项复核 | **已修**：改为 `is_allowed_absolute_roots_for_write`（写路径与兜底绑定；读路径不变） | **close** |
| **D-23** | **"平台假设"专项**：凡注释出现"由 X 兜底/由 Y 保证/会被 Z 拦/依赖 W"，需验证 X/Y/Z/W 在目标平台是否存在 | P0-01~03 归纳 | **已收敛**（P0-10）：**逐条核验 9 处**——**4 处证伪并订正**（web.rs 代理 env、edit.rs「landlock 双防线」、edit.rs 两处过期 landlock 表述、loop.rs 三处理由加平台条件），**4 处证实保留**（L3 证据构造器 / `--` 选项终止 / Node 03 禁名单 / dispatcher 一次性消费） | **close**（专项检查项已固化为三条，见 P0-10 战报第六节） |

---

## 十、本规划的执行原则

- **不碰用户/执行窗的动作**（D-7/D-8 只做代码侧准备）
- **不把环境问题伪装成代码问题**（沿用 P0P1 战报的克制）
- **拒绝为动而动**：每张卡必须能说清"改了之后什么变好了"
