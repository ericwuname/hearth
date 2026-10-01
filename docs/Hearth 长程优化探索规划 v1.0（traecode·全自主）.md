# Hearth 长程优化探索规划 v1.0（**内容已修订至 v1.1**）

- **出品**：traecode　**日期**：2026-09-30（v1.1 修订：2026-10-01）
- **签发**：用户（全权授权，无需逐轮汇报）
- **基线**：`p0-usability-01` @ `e4393f2`（= 远端 `origin/main`）　**版本**：0.2.27
- **授权口径（用户原话）**："全部自主，无需汇报，你只看最后的结果"；北极星三项；**agent-core 先体检再定**。

---

## 〇、v1.1 修订记录（2026-10-01）

**四十六张卡已收官**（每卡独立 commit + 全量门禁 + 推送，门禁目标数 **61 → 62 → 60 → 62 → 58**）：
**顶层七项裁决已下（2026-10-01）**——见《P1-03·04 战报》第一节。

**第二批六张卡（P1-36 ~ P1-41，D-70 ~ D-75）亦已收官**（2026-10-01，见下表与第九节）——
主题=**"用户/项目文件边界"的第二轮体检**：新开 `experience` / `resource-monitor` /
`observer` / `tool-runtime` / `agent-runtime` / `agent-core` / `agent-types` /
`service` / `llm-gateway` / `llm-replay` 十个 crate（此前未体检）。
产出：**5 修 + 1 订正**（含 agent-core `Hearth.md` 完全无上限并整段注入系统提示、
`constitution.md` 截断前先 OOM 两个**项目文件**落点）；另登记开放债 D-74/D-76/D-77/D-78。
**累计卡数 63**（含 P1-52；本轮 P1-45~P1-52 ＝ D-80~D-83 与 D-85~D-88，均已修；
开放债＝**D-84**（`read_only_view` 现无生产调用方）+ D-78 余项 + D-74（需先定重放降级语义））。

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
| P1-12 | `17aa87c` | **「声称 ≠ 实现」专项 · 第二轮（D-23 续）**：核 6 个尚未体检的"机体" crate，**证伪 2 处安全/治理声称**——① observer 的「L2 fail-closed」（`Err` 后调用方只 warn，会话照常继续）② 成本治理链整条失效（`with_budget`/`update_cost` 双零调用 → `cost_ratio()` 恒 0 → `CostGuard` 永不触发）；已订正注释 + 登记 D-45/D-46/D-47 待裁 |
| P1-13 | `69634e4` | **D-36 收口（webhook 子进程）**：`curl` 从 `.output()`（无界 + 超时不收尸）改为 `run_bounded()`——有界排空到 EOF + 超时树杀收割 + `kill_on_drop` 兜底；新增**哨兵法**回归锁（超时后子进程必须已被杀） |
| P1-14 | `1231ff8` | **D-47 收口（按裁决「删除」）**：5 个 crate 的死能力一次清空——**净 −811 / +33 行**；**顺带暴露 D-48**：wiring 锁 `civ-auto-written` 是假绿（claim 引用已被线C手术删除的相位；锚点命中的是 `self.civ_writer` 的**赋值语句**，字段实际只写不读），已登记待裁 |
| P1-15 | `6149611` | **D-46 收口（真实成本"能算就算"）**：新增 `PriceTable`（内置 DeepSeek **官方价快照**〔联网抓取 2026-10-01〕+ 别名 + env 覆盖）与 `CostMeter::total_usd()`；**未知价 → `None`，显式"不可用"，绝不按 0 冒充**；loop 在 `GuardContext` 前单点把 token 换算成 USD 喂给 `nervous`，预算由 `HEARTH_COST_BUDGET_USD` 注入 —— **成本守卫从"永不触发"变为真可触发**；5 条单测 |
| P1-16 | `eee089a` | **D-45 裁决（联网核实后）**：observer 的「L2 fail-closed」不是缺陷而是**文档错误**——按业界通行划分，**fail-closed 属"策略执行点"**，而 Observer 是零执行权的只读审计组件；让"审计报告写不出来"中止用户会话＝反 HA。故**保留 fail-open + warn 留痕**，把注释从"声称≠实现"改写为如实的 by-design 描述并 close |
| P1-17 | `239163c` | **D-48 收口（重接线，联网核实后）**：`civ-auto-written` 假绿修复——`civ_writer` 字段恢复**唯一读者** `note_civ_outcome`，在 `run()` **收尾唯一挂载点**调用（成功=milestone，其余=reflection）。**裁决依据**：civ 线是一等公民用户可见面（CLI `hearth civ` 子命令 + `GET/POST /api/v1/civilization` + per-user store + readyz 降级计数），**消费链完整、唯一缺生产者**——半接线活特性的正解是接线而非退役；亦契合业界「Read–Write Reflective Learning」(Reflexion/Memento/Hermes)。wiring 锚点从"赋值语句"改为真实调用点，重锁 FNV `0xfa4df73ba26cd516`→`0x237a4ecf1a0244cf`；**先红后绿**（移除接线即 0 条）回归锁 |
| P1-18 | `211b9a6` | **D-49 收口（memory 体检首命中）**：CLI `hearth civ search <q>` **形同 `civ feed`**——服务端 `get_civ_feed` 硬编码 `store.recent(50)`、**忽略 `?search=`**，`CivilizationStore::search` 全仓零调用方。抽出 `civ_feed_entries` 唯一事实源 + handler 提取查询参数；**先红后绿**。**顺带登记 D-50（memory crate 整包 `#![allow(clippy::all)]` = 门禁真空）与 D-51（`load_session` 无界读入，第 6 落点）** |
| P1-19 | `cbfaab5` | **D-50 收口（门禁真空补齐）**：删 `crates/memory/src/lib.rs` 顶部 `#![allow(clippy::all, unused_mut)]`——该豁免使 CI 的 `clippy -D warnings` 对本 crate **完全失效**。逐条修真实告警（`unused_mut` ×1、`lines_filter_map_ok` ×2、`items_after_test_module` ×1→测试模块移至文件末尾），**不新增任何 allow** |
| P1-20 | `b5dfde5` | **D-52 收口（残余 crate 级豁免）**：全仓复查（`#!\[allow(` 检索）又发现 2 处——`agent-core`（3 条 doc lint）与 `codex-cli`（`unused_imports`+`manual_strip`）→ 一并拆除，修 23 处（agent-core 17：悬空 `///` 降 `//` ×9 + `doc_lazy_continuation` ×8；codex-cli 6：死导入 ×3 + `strip_prefix` ×3）。**顺带发现 `clippy::doc_markdown` 豁免本是死豁免**（pedantic、默认未启用） |
| P1-21 | `584f104` | **D-51 收口（无界读入第 6 落点）**：`JsonlMemoryStore::load_session` 由 `std::fs::read_to_string`（整份入内存）改为共享原语 `bounded_io::read_file_text_capped`（cap 与写侧 MEM-3 同值 64 MiB + 截断 UTF-8 边界回退 + **留痕**）；`memory` 接入 `bounded-io`（不再新造轮子）；**先红后绿**（还原无界读即读到 4096 而非 1024）回归锁 |
| P1-22 | `5df53d5` | **D-53 收口（无界读入第 7 落点）**：`apply_patch` 的 `tokio::fs::read_to_string`（同 crate `read.rs` 早已改用有界原语，此系漏网）→ 因 apply_patch 是**读-改-写**，**不能**截断读取（会静默损坏），改**先查大小再决定**：`metadata().len()` 超 8 MiB 显式拒绝（同 D-38 口径）；**先红后绿**（禁用守卫即"patched"成功并改动文件 → 红）回归锁，并断言**被拒不得改动文件** |
| P1-23 | `24a3556` | **D-54 收口（注释订正）**：`check_egress` 的 doc 写「白名单为空 = 全拒」「提取不到主机 → fail-closed」，与实现（`if allow.is_empty() { return Ok(()) }`）**直接矛盾**——真相是 hearth-slim S2 的**语义反转**（空/未设 = 默认放开，用户 2026-09-09 拍板）。会让安全审计把"默认放开"误读成"默认拒绝"（同 D-45 型文档错误）。**纯注释订正、零行为变更** |
| P1-24 | `fa04e01` | **D-56 收口（删死 crate，净 −748/+1 行）**：`code-index`（tree-sitter 增量解析）**零生产消费者**——`agent-core` 的依赖声明是死的（同 D-41 型）。裁决：**接线属"能力扩展"（本规划未授权）→ 删除**。一并清掉仅服务于它的 `tree-sitter`/`tree-sitter-rust` 与**早已孤儿化**的 `tantivy`（P1-10 遗留）→ 三族重型外部依赖退出构建闭包；顺带消除 `walk_and_parse` 的 symlink 无限递归栈溢出隐患 |
| P1-25 | `fa77394` | **D-55 收口（网络侧无界读入，第 8/9 落点）**：`web.rs`/`search.rs` 的 `resp.text()`（整份响应入内存后才截 8000 字符）→ 新增 `read_body_capped`：**Content-Length 超限提前拒绝**（不下载）+ `Response::chunk()` 流式累加**硬上限 1 MiB** + 截断 UTF-8 边界 + **留痕**；**先红后绿**（禁用守卫即拿到 Ok+截断体 → 红）+ 不误伤小响应，共 2 条测试（本地一次性 TCP 服务） |
| P1-26 | `f9d820a` | **D-57 收口（删死 pub 项）**：`tools-builtin` 两个**零调用方**项删除——① `is_allowed_absolute`（全仓零调用；且**直读 `std::env`** 绕过 `ctx.env`，与注入纪律相悖）② `coerce_args`（零生产调用；其"防 raw string 降级"意图**已由真实在位的 `extract_str_arg` 覆盖**，被删的是并行且从未接线的旧机制）+ 其 3 条专属测试；原位留注释说明去留理由。**glob.rs 的 `sandbox` 字段经核实后保留**（源码已注明是 `with_sandbox` 测试脚手架，删除需连带改 3 处测试构造，收益不抵改动面） |
| P1-27 | `5122be8` | **D-58 收口（CLI 侧无界读入，第 10~19 落点）**：`CodexClient` 全部 9 处 `resp.json()/resp.text()` 无上限 + SSE 残行缓冲无上限 → 新增 `json_capped`/`error_body_capped` 全量改走共享原语（`read_body_capped`/`MAX_BODY_BYTES` 提升为 `pub`，**CLI 与工具层共用一份**，D-33 收敛），SSE 缓冲加 1 MiB 上限；**先红后绿**（还原 `resp.text()` 即得"parse json"而非"响应体过大"） |
| P1-28 | `285004d` | **D-59 收口（xray 假绿根治，系统性）**：锚点原为**纯子串匹配且保留字符串**→ **测试代码的字符串能顶绿生产锚点**。实证：删掉生产 `MUTATING_TOOLS` 里的 `"edit"`，red 级 `readonly-view-strips`（"子智能体只读"）仍绿（靠测试第 558 行的同名数组顶住）。**治本**：新增 `blank_test_modules`（复用 `LexState`，字符串/注释里的 marker 不触发；括号配对；无花括号形态抹到 `;`；找不到右括号保守抹到 EOF）→ 锚点**测试免疫**。**端到端实证**：删 `"edit"` 后门禁如期报 `missing(all): ["\"edit\""]`（此前恒绿）。全仓 14 条 red 锚点**零新增断裂** → 确认此前无"已被顶绿"的现存回归，属潜在假绿，该类缺口自本卡关闭 |
| P1-29 | `1ba1221` | **D-65 收口（provider 侧无界读入，第 20+ 落点）+ 原语收敛**：`llm-openai`/`llm-cn`/`llm-local` 共 **11 处** `resp.text()` 无上限（另 3 处流式残行缓冲无上限）→ 全部改走有界读；同时把 `read_body_capped`/`MAX_BODY_BYTES`+测试从 `tools-builtin` **整体迁入 `bounded-io`** 并以**可选 feature `reqwest`** 门控（默认不启用 → `memory` 等不被拉入 reqwest），`tools-builtin` 再导出使既有调用点零改动（D-33 收敛：全仓 HTTP 有界读**只剩一份实现**） |
| P1-30 | `1445fd0` | **D-60/D-61 收口（CLI 输入/展示路径健壮性）**：① `mask_key` 用**字节**切片 → key 含非 ASCII 即 **panic**（`hearth config get api-key` 崩，实测 `byte index 4 is not a char boundary`）→ 改 `chars()`；② URL 只编码空格 → `&`/`#`/`=` 可注入查询参数、`/` 可跳转路径层级 → 新增 `pct_encode`（RFC 3986 unreserved 之外全编码，**零新依赖**）应用 1 处查询值 + 5 处路径段；**先红后绿**（还原字节切片即复现 panic） |
| P1-31 | `f3e6dcc` | **D-62 裁决（by design）+ D-63 收口（白名单注入）**：① G-B「禁止计入验收通过」**不是缺陷**——`loop.rs:4459-4461` 载明"拦截在**报告层**（任务书指定），**不改控制流**"，两条 CLI 投影点已做到"拒发裸 ✓ + 告警"；补裁决注释防止后人误改（同 D-45 型）。② `egress-allowlist` 是**逗号分隔纯文本、无转义**，而原批注却称"转义由既有实现负责"（不实）→ 含 `,` 的 host 被 join 后再 split 会**凭空多放行一个域名**；新增 `is_persistable_egress_host` 源头校验（只收 `[A-Za-z0-9._:-]`、非空、≤255）+ 订正批注；**先红后绿** |
| P1-32 | `2ceda40` | **D-64 收口（CLI 死 pub 项 + 构建期 panic）**：删 7 个零调用方 pub 项（`require_api_key`/`last_session_had_abnormal_signal`/`collect_remaining_from_graph`/`save_turn`/`save_graph`/`load_graph`/`has_run_state`——TaskGraph 删除后的残壳）**净 −139/+5**；测试按"保覆盖"处理（`test_roundtrip_turn` 改写侧为**真实路径** `save_snapshot`）；`CodexClient::new` 的 `.expect("reqwest client build")` → `unwrap_or_else` + warn 降级（构建失败不再让整个 CLI panic）。**service 侧同名鉴权中间件未误删** |
| P1-33 | `e86c74d` | **D-66 收口（删死协作者 planner，净 −1073/+47）**：`crates/planner` 是线C手术删 B 臂后的**死协作者**——`Arc<dyn Planner>` 被存字段、`new()` 收参、子 agent 传递，但 **`decompose()` 全仓零调用**；生产侧只在 4 处构造后即无下文。贯通移除整个 crate + 构造参数 + 5 处依赖声明（含**声明了却从未使用的 `service`**）；测试侧删 `MockPlanner` 与其专用图构造辅助。**wiring spec 无需改**（相关锚点均在 `loop.rs` 且已 yellow 退役） |
| P1-34 | `b754933` | **D-69 收口（xray 改名，名实一致）**：`strip_comments_and_strings` → `strip_comments_keep_strings`（17 处）——旧名读起来像"把字符串也剥掉"，正是 D-59「假绿」的**认知来源**；补文档点明因果与职责边界（"连字符串一起屏蔽"归 `blank_test_modules`）。纯改名 + 文档，**零行为变更** |
| P1-35 | `2053522` | **D-67/D-68 收口（provider 层死代码 + 口径统一，净 −113/+7）**：① `llm-gateway` 删"只写不读"的 S9 分账残留（`last_used` 字段/方法/`note_success`）并**订正两处不实注释**（该能力未接线）；删仅测试调用的固有 `chat()/embed()`——**关键核验**：其内 S9 切换逻辑在 trait 实现里完整保留，**生产行为零变化**；删 `list_aliases()`。**保留** `with_switch_callback` 一族（复核发现 `run_local.rs:85` 有真实生产调用）。② `llm-cn` 删零构造死类型与不读取的 `code` 字段；**超时 300s → 30s**，与 `llm-openai` 口径一致 |
| P1-36 | `44212c2` `a7361a9` | **D-70 收口（无界读入第 21 落点，experience crate 首命中）**：`ExperienceStore::set_path` 用 `tokio::fs::read_to_string` 整份读 `experience.jsonl`——该文件**只增追加**（每 run 一条），长期运行可涨到 GB 级，重启即 OOM。改走 `bounded_io::read_file_text_capped`（64 MiB，与 memory MEM-3 同值）+ 截断留痕；**先红后绿**（还原即 4196≠4096）。`a7361a9` 为 **fmt 补丁**——见本节末"门禁教训" |
| P1-37 | `6e80c3e` | **D-71 收口（resource-monitor 从未接线的 ROI/critical 死项，净 −47/+12）**：`RoiReport`/`compute_roi`（原计划 `GET /api/v1/costs/roi`）与 `ResourceSnapshot::is_critical` 全仓**零生产调用方**（仅自测；`global-panorama-v10.1.md:79` 早已如实记录）。**裁决=删除**（非订正）：`compute_roi` 用**硬编码假价** `tokens*0.000002`，与 D-46 口径（"未知价→显式不可用"）正面冲突；真实成本唯一事实源已是 `PriceTable`+`CostMeter` |
| P1-38 | `09d2e5d` | **D-72 收口（observer「只写不读」+ 不实文案）**：`hearth note --observer-verdict n` 经 `apply_rebuttal` 落盘反审档（写侧现役），但读侧 `observer::rebuttals_for` **全仓零调用方**；而 observer 模块头与 CLI 均称该档"供**下次 run 的 bias**"——文档错误（D-45 同族）。**裁决=删死读取口 + 如实订正**（闭环 bias 需先**设计**机制，而 observer 有零执行权铁律 ⇒ 属新增特性，非补管道）；写侧保留（人可读审计档）；测试改为直接读档保覆盖 |
| P1-39 | `0378fab` | **D-73 收口（tool-runtime 安全声称订正，纯文案）**：模块自称 "secure tool installation"/"…and verification"、`install()` 文档称 "verifying SHA-256 if a file path is provided" ——**全不实**（无校验、无 file path 参数）；`ToolManifest.sha256`/`.command` 全仓**零读取**；`ResourceLedger::snapshot()` 零生产调用方（注释却称 "introspect/报告消费"）。纯订正（同 D-45/D-54/D-63 口径）。**核验保留**：`ToolRegistry` 本体现役（三个 API 面在用） |
| P1-40 | `4d2e043` | **D-74 收口（agent-runtime `events` "ring" 失实注释）**：注释称 "in-memory **ring**"，实为**无上限 `Vec`**（14 处 push / 0 处 clear）。**不能**改 ring——它是**断线续传重放源**（`sse_stream_with_replay` + `get_history`）。如实订正 + 无界增长登记为债（改前须先定"重放降级"语义） |
| P1-41 | `6231956` | **D-75 收口（agent-core 项目文件无界读入，第 22~25 落点）**：三处读的是**正在被处理的仓库/cwd** 的文件（外部边界），两处内容**整段注入系统提示**——`constitution.md`（先整份读再截 6000 字符 ⇒ 截断前先 OOM）、`Hearth.md`（**完全无上限**，cwd 或家目录）、S12 自检回读 ×2。处置：`bounded-io` 补**同步原语** `read_file_text_capped_std`（与 async 版共用 `finalize_text`，语义严格一致 = D-33 收敛）→ constitution 64 KiB（≫6000 字符 ⇒ 正常文件行为不变）/ Hearth.md 64 KiB / 自检 8 MiB，均截断留痕。**先红后绿**。**xray 门禁如实报红**：`constitution-reads-file` 锚定 `read_to_string`，实现换底即断 → 同步锚点 + 复算 FNV（`0x237a4ecf1a0244cf` → `0xdaa53449f2f0f677`，已用 HEAD 版复算比对验证算法一致），能力条数仍 16 |
| P1-42 | `50e3fa8` | **D-78 部分收口（删两个零调用方 pub 项）**：`service::sse::sse_stream`（无 replay 便捷版，两个 SSE 出口都用 `sse_stream_with_replay`）与 `llm-gateway::types::PropertySchema`（**只被自身字段引用**的自引用类型）删除并原位留注。同批余项（`fallback` 固有 `stream()` 重复、`CostMeter` 四个仅自测访问器）**保留待裁**（价值不抵风险，见 D-78） |
| P1-43 | `875ea52` | **D-76 收口（删 `agent-types` TaskGraph 残族，净 −341/+84）**：删 `TaskGraph`/`TaskNode`/`TaskResult`/`PlanContext`/`Observation`/`PlanState` + `impl`（topo/next_ready/next_action/派生标题）+ 5 条单测，以及未接线的注入标注三件套（`ContentSource`/`MAX_INJECTED_CHARS`/`format_injected_content`）。**关键在反向核验**：逐符号 grep 证明 `FileChange`（`merge_file_changes` 在用）、`MessageContent::ToolResults`+`ToolResult`（`context.rs` token 统计在用）**是活的**，不可顺手删。**顺带取证 D-79**（见债队列）——`SessionLedger` 生产"只读不写"，账本恒空 |
| P1-44 | `7b90e9f` | **D-79 收口（给账本接上生产者，净 +122）**：`ledger_record_selfcheck()` 挂在 S12 自检闸的**事实产生点**——自检未过项 → `KnownFailing`（同前缀去重）、通过 ⇒ 复测通过 ⇒ 关闭开放项（"只关不删"）；K-1 交付物类型核对两条出口同登记。**先红后绿**（把生产者改成空实现即红：open 0≠1）+ 5 条断言（登记/去重/注入非 None/通过即关闭/关闭后文本不再含该项）。**这是"半接线活特性"的第二个 D-48（civ）式收口**：读侧用户可见（prompt 注入 + report + CLI）而生产者缺失 ⇒ 接线而非退役。残留 `Pending` 栏 → D-81 |
| P1-45 | `9d2c5f1` | **D-80 收口（仓库文件进系统提示补"来源+权威序"标注）**：先核"是否已被别处覆盖"——`web_fetch` 通道**已自带** `source` / `verifiable` / "默认未验证断言"（web.rs），WS7「Trust-but-Verify」也在位，**缺口只有一处**：`Hearth.md`（取自 cwd = 正在处理的仓库）整段注入**系统提示**（权威最高）却零来源声明。依 OWASP《Secure Coding with AI》§3/§6 补一行来源+权威序；**先红后绿**，并断言"项目约定正文原样保留"（防止把标注写成"忽略本文件"） |
| P1-46 | `48bf080` | **D-81 收口（账本 `Pending` 栏接上生产者，净 +179）**：**关键发现**——系统提示承诺的 `todo_write(todos) your own checklist` 是**真注册工具**，但其清单只活在**工具输出**里，历史一旦被切片/压缩就**永久丢失**。而账本恰在**切片之后**注入 ⇒ `ledger_sync_todos()` 把清单**全量镜像**进 `Pending` 栏 = 让清单**免疫切片**。**先红后绿**（空实现即红）+ 4 组断言（镜像/去重/失败调用不污染/注入可见） |
| P1-47 | `034a75e` | **D-82 收口（civ"文件改动数"恒 0）**：沿 D-80 体检挖出——`note_civ_outcome` 取 `report.files_changed.len()`，而**主 run 的该字段处处为空**（只在子代理路径填）⇒ `hearth civ feed` / `GET /api/v1/civilization` 每条主 run 记录都写"0 个文件改动"，真写了产物也是 0。改用本 run 真实产物事实 `written_files` 按去重路径计数；**先红后绿** |
| P1-48 | `8cbad9d` | **D-83 收口（删死掉的子代理委派子系统，净 −343/+22）**：`spawn_sub_agent` / `collect_sub_agent_results` / `active_sub_agent_count` **全仓零调用方**（唯一触发条件 TaskGraph 的 `delegable` 早已随线C手术消失，原位注释即载明"spawn 块不可达"）；连带 `extract_files_from_tool_calls` / `merge_file_changes` / `RunReport.files_changed` / `agent_types::FileChange` / `depth` / `MAX_DEPTH` 整段删除；顺带清除一处**错位注释**（"Accumulated FileChanges" 误挂在 `pending_user_messages` 上）。**并订正 D-76 里我写过头的一句**（曾称 FileChange"是活的"——它只作为类型被携带，合并逻辑其实零调用方）。xray `subagent-uses-readonly` 如实报红 → 按既有退役惯例降 yellow 留痕（条数仍 16）+ 复算 FNV。19 个 hunk 逐项复核 |
| P1-49 | `3c2152b` | **D-85 收口（删死掉的 `orchestrator`/`replay` 两模块，净 −530/+26）**：沿 agent-core 逐文件体检——`orchestrator`（479 行，`execute_plan`/`TaskOrchestrator`/`PipelineRunner`/`TaskStep`/`TaskReport`/`TaskValidator`）**全部引用都在本文件内**（定义+自带单测），它是**已拆除的 TaskGraph 的执行器**（loop.rs 原位注释即载明"恒走顺序执行臂"）；`replay`（51 行）仅有一行 `pub use` 再导出、无调用方。连带删 `Scheduler::dispatch_single`（唯一用途=给 orchestrator 当 runner closure）。顺带清理两处失效注释。**xray 无锚点涉及 → 无需改 spec** |
| P1-50 | `3d7589b` | **D-86 收口（`archive_digest` 无界读入）**：归档读回通道把归档文件**整份**读进内存，而只用**开头** `max_turns` 条 turn；归档随会话**只增** ⇒ 与 D-51/D-70 同族。改走共享原语（cap 8 MiB，按头截断即保持"取开头 N 条"语义）+ 截断留痕，仍 best-effort。**先红后绿**：构造"头 1 条 turn → 9 MiB 噪声 → 尾 1 条 turn"，绿侧尾部读不到、红侧（还原 `read_to_string`）尾部被读进 digest 立即断言失败；该锁同时验证"best-effort 不误伤" |
| P1-51 | `6a771fb` | **D-87 收口（删孤儿源文件 + 新增「孤儿源文件门禁」，修 + 防复发）**：写脚本全仓枚举 `crates/*/src/**/*.rs` 并逐个反查"谁 `mod` 声明了它"，命中 **1 个真孤儿**——`crates/agent-core/src/cache_telemetry.rs`（216 行）**没有任何 `mod` 声明 ⇒ 从不参与编译**（编译器看不见、测试盖不到、clippy 不管），内容是 `llm-gateway` 现役同名文件的**陈旧副本**。（另两处为误报已排除：`loop.rs` 是 `mod r#loop` raw identifier；`bin/codex.rs` 是 Cargo 自动发现的 bin。）处置：删文件 + **新增门禁** `orphan_source_gate.rs`（本仓"手术残留"模式在**文件粒度**上的最后一道网；文件头自报盲区）。**先红后绿**：先建门禁 → 如实报红且**精确列出唯一孤儿、零误报**；删文件后转绿 |
| P1-52 | `4122ade` | **D-88 收口（清完 TaskGraph 遗留链，净 −122/+49）**：① 删 `SessionLedger::sync_from_task_graph` + 专用 `close_by_prefix` + `TaskStatus`（D-76 当时**仅因**该方法的入参形态而保留它）——其全仓**唯一调用方是单测**；删除正当性经核验：账本现已有**两个真实生产者**（D-79 自检事实 / D-81 清单镜像）完整覆盖其三栏语义，属**冗余**而非"缺生产者的活特性"。② 删 `Turn.actions: Vec<Action>`——**只被写成空 vec（全仓 6 处）、从不被读**的 write-only 字段，连带死类型 `Action`；归档旧键由 serde 默认忽略。③ **订正两处已过时注释**（`SessionLedger` 头注仍写着 D-79 当年的"只读不写/账本恒空"，本身已成新的失实注释；loop.rs 的 TaskStatus 保留说明）。**另**：用脚本做 `api` 事件契约变体审计——**15 个变体全部有生产者/消费方，无死契约项** |

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

> **教训（P1-36，新增，务必遵守）**：不许把四件套**串成一条命令**再只看整体退出码。
> 本轮曾写成 `fmt; clippy; test; check` 一条链——PowerShell/`Select-Object` 的退出码
> **只反映最后一条命令**，导致 `cargo fmt --check` 的失败被 clippy 的 0 掩盖，
> 于是一个 fmt 不干净的提交被推了上去（CI 23 秒即红）。**每一步必须单独取
> `$LASTEXITCODE` 并打印**（本轮已改为 `GATE FMT=? CLIPPY=? TEST=? LINUX=?` 一行汇总）。
> 同理：`Select-Object -Last N` 只截显示、不改变退出码，别拿它当"通过"的证据。

> **运行环境注意（P1-36 实测）**：本机 `C:` 盘一度 **0 字节可用**（`target/debug/incremental`
> 已涨到 **29.8 GB**）→ 编译直接报"磁盘空间不足"。清 `incremental` 后恢复（26 GB）。
> **跑门禁前建议** `CARGO_INCREMENTAL=0`，避免缓存再涨破盘。

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
| **D-45** | **observer 的「L2 fail-closed」声称不成立**：`observer/src/lib.rs` 模块注释写"`Observer::run()` 返回 `Err` → 由 service 层拒绝继续"；实测 `agent-runtime/src/session.rs:785-790` 拿到 `Err` 后**只 `tracing::warn!`，会话照常跑完**。另一半"构造失败 → 拒启"亦恒不触发（`Observer::new()` 无失败路径） | P1-12 核验 → P1-16 裁决 | **已裁决：by design 不做 fail-closed（审计组件 fail-open），注释已订正**。依据（联网核实业界通行划分）：**fail-closed 属于「策略执行点」**——"决定做不出来就必须拒绝"；而本项目的执行点另在沙箱 fail-closed / 审批门 / `ConstitutionGuard`+`CostGuard`。Observer 是**零执行权的只读审计组件**，让"报告写不出来"去中止用户会话＝把可用性白送给一个与安全无关的故障（正是反 HA 模式）。**且审计降级本身已 warn 留痕**（非静默丢弃），符合"降级必须可审计"的要求。⇒ 旧注释系**文档错误**，已改为如实的 by-design 描述 | **close（裁决：保留 fail-open + 留痕）** |
| **D-46** | **成本治理链整条失效（`CostGuard` 在生产中永不触发）**：① `NervousSystem::with_budget()` **仅测试调用** → `cost_budget_usd` 恒 `None` → `cost_ratio()` 恒 `0.0`（`AgentLoop` 用 `NervousSystem::new()`）；② 累加侧 `AgentLoop::update_cost()` **全仓零调用者**；③ 该恒 0 值被喂给 subconscious `CostGuard`（`>0.80/>0.95`）→ 分支永不成立。而 v16 的 CHANGELOG/审计都标"真值接地 ✅"——**只接地了"读"** | P1-12 核验 → P1-15 接通 | **已修**（顶层"能真算就算"）：① `llm-gateway` 新增 **`PriceTable`**（USD/1M tokens，内置 **DeepSeek 官方价快照 2026-10-01**〔联网抓取，高峰价保守上限〕+ 旧名别名映射；`HEARTH_PRICE_TABLE`/`HEARTH_PRICE_FILE` 可覆盖）与 `CostMeter::total_usd()`；② **加 `None` 语义 = 诚实**：任一条目查不到价即返回 `None` → 显式报"成本不可用"并 warn 一次，**绝不按 0 冒充**；③ loop 在 `GuardContext` 构造前**单点**用 cost_meter 换算 USD → `nervous.set_cost()`；④ 预算由 `HEARTH_COST_BUDGET_USD` 注入 `with_budget()`；⑤ `.env.example` 补齐三个配置项与语义说明。**5 条单测**（未知模型→None / flash 价=0.30+1.20 / 旧名别名 / 混入未知→None / JSON 覆盖生效） | **close** |
| **D-47** | **机体 crate 体检清单（"定义了但生产无人用"）**：① `subconscious` 的 `LspGuard` / `ExperienceMatcher` 从未 `add_guard` 注册（`add_guard` 仅测试调用）→ 恒不生效，`repetition` 字段"mutated externally"但全仓无写入方；② `experience::search()`（语义检索）全仓仅测试调用 → 整块检索能力死（`reinforce`/`evaluate` 同）；③ `bridge` **整 crate 零测试**，且 `let (event_tx, _) = ...` 丢弃接收端 → `BridgeEvent` 事件流无任何消费者；④ `nervous-system::drain_civ_alerts` 读的 `civ_pending` 唯一写方是 `query()`，而 `query()` 生产零调用 → agent-core 侧恒取到空；⑤ `observer::is_tripped/tripped` 零调用零赋值（P1-01 已记） | P1-12 体检 → P1-14 删除 | **已删除**（顶层裁决「删除」，P1-14）：五处全清，**净 −811 行 / +33 行**——`nervous-system` 删感知-决策链路（`query`/`NerveAction`/`PerceptionReport`/`civ_pending`/`drain_civ_alerts`/`pid`/`snapshot_override`）只留成本比率；`subconscious` 删 3 个未注册 guard + 2 个无生产者枚举变体；`bridge` 删死事件流；`experience` 删检索/嵌入整块（`search`/`reinforce`/`evaluate`/`append_raw`/`Applicability`/`embedding`/`context`）；`observer` 删 `tripped`/`is_tripped` | **close** |
| **D-48** | **wiring 锁 `civ-auto-written` 是「假绿」**（P1-14 删除死链路时暴露）：该能力 claim 写"**do_reflect/do_observe** 自动写文明线"，但这两个相位**早已被线C手术（D-6/D-7）删除**；而真正调用写手的唯一函数 `civ_note` 的唯一调用点是 `drain_civ_alerts` 循环——该循环因 `query()` 生产零调用而**恒为空**，即该能力**在删除前就已不工作**。删除后 `civ_writer` 字段变成**只被赋值、从不被读取**（唯一读者 `civ_note` 已随之删除），而引擎锚点 `self.civ_writer` 命中的恰是**赋值语句**（loop.rs:1699）→ **门禁为绿、语义已空**。（与 D-23「声称 ≠ 存在」同型；本锁建立的初衷正是"清掉'从未接线'的五版旧债"，结果自己成了其中一例） | P1-14 暴露 → P1-17 收口 | **已收口（重接线，联网核实后）**：裁决走 **(a) 重新接线**。依据——civ 线是**一等公民用户可见面**（CLI `hearth civ` 子命令 + `GET/POST /api/v1/civilization` + per-user store + readyz 降级计数），**消费链完整、唯一缺生产者**；半接线的活特性应接线而非退役（`(b)` 会删掉一个用户可见特性）。亦契合业界「Read–Write Reflective Learning」(Reflexion / Memento / Hermes)——「观察→持久化→回读」是长程 agent 核心。实现：新增 `note_civ_outcome` 为字段**唯一真读者**，在 `run()` 收尾唯一挂载点调用（成功=milestone / 其余=reflection）；wiring 锚点从"赋值语句"改为真实调用点；重锁 FNV（条数仍 16）。**先红后绿**（移除接线 → 断言 0 条 → 红） | **close** |
| **D-49** | **CLI `hearth civ search <q>` 声称"检索文明线"、服务端忽略查询参数**：`get_civ_feed` 硬编码 `store.recent(50)`，从不读 `?search=`；`CivilizationStore::search` **全仓零调用方** → `civ search` 实为 `civ feed`（永远返回最近 50 条） | memory crate 体检发现 → P1-18 修复 | **已修**（P1-18）：抽出 `civ_feed_entries(store, search)` 作 civ 检索**唯一事实源**（非空 search → `search(q,50)`，否则 `recent(50)`），handler 加 `Query` 提取 `?search=`；**先红后绿**（还原旧行为即 2 条 → 红） | **close** |
| **D-50** | **`memory` crate 整包禁用 clippy**：`lib.rs:1` 的 `#![allow(clippy::all, unused_mut)]` → CI 的 `clippy --workspace -D warnings` 对本 crate **完全失效**（门禁真空）；`unused_mut` 亦掩盖真实告警（如 `WorkLineStore::add(&self, mut node)` 的多余 `mut`） | memory crate 体检发现 | **已修**（P1-19）：删豁免 + 逐条修真实告警（`unused_mut`/`lines_filter_map_ok`/`items_after_test_module`），不新增 allow | **close** |
| **D-51** | **`JsonlMemoryStore::load_session` 无界读入**（`std::fs::read_to_string` 全量入内存）——**无界读入第 6 落点**；写入侧有 64 MiB 上限（MEM-3），但 `append_events` 是"先查后写"、单次 append 可越界，且外部放置/导入的文件不受限 | memory crate 体检发现 | **已修**（P1-21）：改走共享有界读入原语 `bounded_io::read_file_text_capped`（cap=写侧上限 + 截断留痕），`memory` 接入 `bounded-io` | **close** |
| **D-52** | **残余 2 处 crate 级 lint 豁免**（D-50 修完后全仓 `#!\[allow(` 复查发现）：`agent-core/src/lib.rs` 豁免 3 条 doc lint、`codex-cli/src/lib.rs` 豁免 `unused_imports`+`manual_strip` → CI `clippy -D warnings` 对这些 lint **局部失效**。其中 `clippy::doc_markdown` 属 pedantic、默认未启用 → 该条**本就是死豁免** | D-50 同批复查发现 | **已修**（P1-20）：两处豁免拆除，修 23 处真实告警（agent-core 17 doc 类 / codex-cli 6：死导入 ×3 + `manual_strip` ×3），**不新增 allow**；冻结区只动注释 | **close** |
| **D-53** | **`apply_patch` 无界读入（第 7 落点）**：`tools-builtin/src/patch.rs` 用 `tokio::fs::read_to_string` 整份入内存——同 crate `read.rs` 早已改用 `bounded_io::read_file_text_capped`，此系漏网。特殊点：apply_patch 是**读-改-写**，截断读取会**静默损坏**内容（既有测试 `test_patch_no_truncation_on_large_file` 正锁此语义） | tools-builtin 体检发现 | **已修**（P1-22）：改"**先查大小再决定**"——`metadata().len()` 超 8 MiB 显式拒绝（同 D-38 口径），绝不截断后照改；**先红后绿**（禁用守卫即"patched"成功且改动文件 → 红）+ 断言被拒零副作用 | **close** |
| **D-54** | **`check_egress` 注释与实现矛盾（会误导安全审计）**：doc 写"白名单为空 = 全拒""提取不到主机 → fail-closed"，实现是 `if allow.is_empty() { return Ok(()) }`——hearth-slim S2 **语义反转**（空/未设 = 默认放开，用户 2026-09-09 拍板）；紧邻内联注释已如实记录，唯 doc 头部未同步 | tools-builtin 体检发现 | **已修**（P1-23）：doc 按 S2 现状订正（含订正说明），与实现/内联注释三者一致；**纯注释、零行为变更** | **close** |
| **D-55** | **HTTP 响应体无界读入**：`tools-builtin/src/web.rs`（`resp.text()`）与 `src/search.rs`（同）——reqwest 把**整份响应**读进内存后才截到 8000 字符；恶意/超大响应可致 OOM（与已修的 7 处"读入无界"同族，但落在**网络**侧） | tools-builtin 体检发现 | **已修**（P1-25）：新增 `read_body_capped`（Content-Length 提前拒绝 + 流式硬上限 1 MiB + 截断留痕），两处调用点改走它 | **close** |
| **D-56** | **`code-index` 整 crate 无生产消费者**：`agent-core/Cargo.toml:11` 声明依赖，但全仓 `TreeSitterIndex`/`code_index::` **零使用**（仅 crate 自身与测试）→ 依赖声明是死的（同 D-41 `project-sync` 型）。附带隐患：`walk_and_parse`（lib.rs:322）用 `path.is_dir()` **跟随 symlink** 递归且无深度/visited 守卫 → symlink 成环即栈溢出；死字段 `Symbol.parent`、零构造枚举变体 `SymbolKind::{Method,Variable,Other}` | tools-builtin/code-index 体检发现 | 确证（**整 crate 死**）。注意：**接线属"能力扩展"（本规划未授权）** → 默认处置为**删除**（同 D-41） | **close**（已删除，P1-24：整 crate + `tree-sitter`/`tree-sitter-rust`/`tantivy` 一并清除，净 −748/+1） |
| **D-57** | **`tools-builtin` 死 pub 项**：`lib.rs` `is_allowed_absolute`（全仓仅定义处；且**直读 `std::env`** 绕过 `ctx.env`，属遗留）、`coerce_args`（仅定义+同文件测试）、`glob.rs` 的 `sandbox` 字段（`#[allow(dead_code)]`，只写不读） | tools-builtin 体检发现 | 确证（"定义了但无人用"；`is_allowed_absolute` 另有"绕过注入 env"隐患） | **已修**（P1-26）：删 `is_allowed_absolute` + `coerce_args`（零生产调用；后者的防线已由 `extract_str_arg` 真实覆盖）及 3 条专属测试；`glob.rs` 的 `sandbox` 字段**核实后保留**（已注明为测试脚手架） | **close** |
| **D-58** | **CLI 客户端侧无界读入**：`codex-cli/src/client.rs` 全部 9 处 `resp.json()/resp.text()` 无字节上限；SSE 残行 `buf.push_str` 无上限 | codex-cli 体检发现 | **已修**（P1-27）：`json_capped`/`error_body_capped` 全量改走共享 `read_body_capped`（提升为 `pub`）；SSE 缓冲 1 MiB 上限；**先红后绿** | **close** |
| **D-59** | **xray 锚点"假绿"根因（系统性）**：锚点是**纯子串匹配**且 `strip_comments_and_strings` **保留字符串** → **测试代码的字符串可顶绿生产锚点**。实证：删掉生产 `MUTATING_TOOLS` 的 `"edit"`，red 级 `readonly-view-strips`（子智能体只读）**仍绿**（靠同文件测试第 558 行顶住） | project-xray 体检发现 | **已修**（P1-28，治本）：新增 `blank_test_modules`（复用 `LexState`、括号配对、无花括号形态抹到 `;`、找不到右括号保守抹到 EOF）→ 锚点**测试免疫**；端到端实证删 `"edit"` 即精确报断；全仓 14 条 red 锚点零新增断裂 | **close** |
| **D-60** | **CLI `mask_key` 多字节 UTF-8 panic**：`codex-cli/src/lib.rs:1331` 用字节切片 `&k[..4]` / `&k[k.len()-4..]`——key 含非 ASCII 时非字符边界 → **直接 panic**（`hearth config get api-key`） | codex-cli 体检发现 | **已修**（P1-30）：改 `chars()` 计数与截取；**先红后绿**回归锁（还原字节切片即复现 `byte index 4 is not a char boundary` panic） | **close** |
| **D-61** | **CLI URL 参数/路径注入**：`lib.rs:1268` 仅 `replace(' ', "%20")`，`&`/`#`/`=` 未编码 → `?search=` 参数注入；用户 `id` 未编码直接拼路径（`lib.rs:1224`、`client.rs:74/108/155/175`） | codex-cli 体检发现 | **已修**（P1-30）：新增 `pct_encode`（零新依赖），应用 1 处查询值 + 5 处路径段（`lib.rs`×2 / `client.rs`×4） | **close** |
| **D-62** | **CLI「G-B 拦截」声称 ≠ 实现**：`run_local.rs:869-874` / `1165-1171` 注释称"禁止计入验收通过"，实际只 `render::error` 打印，**未改 `report.ok/status`**（仍 completed）→ 下游按退出码/ok 判定**感知不到**该门 | codex-cli 体检发现 | **已裁决：by design，非缺陷**——`loop.rs:4459-4461` 明载"拦截在报告层（任务书指定），不改控制流"；两条投影点已做到"拒发裸 ✓ + 告警"。**只补裁决注释**（防后人误改为非零退出=真契约变更） | **close** |
| **D-63** | **出口白名单可被逗号注入**：`config.rs:113-121` `set_field("egress-allowlist")` 仅按 `,` 切分、**零转义**；而 `run_local.rs:449` 批注却称"转义由既有实现负责" → 审批放行的 host 含逗号即可**注入额外白名单项** | codex-cli 体检发现 | **已修**（P1-31）：新增 `is_persistable_egress_host` 源头校验（拒绝落盘并留痕）+ 订正不实批注；**先红后绿** | **close** |
| **D-64** | **CLI 死 pub 项 + 构建期 panic**：`config.rs:292 require_api_key`、`session_store.rs:27 save_turn`/`:65 save_graph`/`:89 load_graph`/`:205 has_run_state`、`report.rs:107 collect_remaining_from_graph`、`note.rs:128 last_session_had_abnormal_signal` 均**仅测试引用**；`client.rs:35 .expect("reqwest client build")` 构建失败即 panic（TLS/系统环境） | codex-cli 体检发现 | **已修**（P1-32）：7 项全删（净 −139/+5），测试按"保覆盖"改写；`expect` → `unwrap_or_else` + warn 降级 | **close** |
| **D-65** | **`llm-*` provider 无界读入（第 20+ 落点）**：`llm-openai/src/lib.rs:377,534,589`、`llm-cn/src/lib.rs:431,515`、`llm-local/src/lib.rs:399,471,502,864,949,987` 全部 `resp.text()` 无上限；流式 `buffer.push_str`（openai:546）无行内上限。仓内已有 `bounded-io` 却未引用 | llm-* 体检发现 | **已修**（P1-29）：11 处改走有界读 + 3 处流式缓冲加 1 MiB 上限；原语迁入 `bounded-io`（可选 feature `reqwest`）实现**全仓单一实现** | **close** |
| **D-66** | **`planner` 生产不生效（WP-3 门禁空转）**：`derive_gaps`（注释自称"门禁契约"）**全仓仅本文件测试调用**（`loop.rs:2875` 明载 B 臂已删、生产调用归零）；`Planner` trait 文档称含 `reflect` 但 trait **只有 `decompose`**（`ReflectVerdict` 已删）；`DefaultPlanner::decompose` 生产调用为零；`lib.rs:315/393/457` `fail_cache.lock().unwrap()` 中毒即 panic | planner 体检发现 | **已修**（P1-33）：整 crate 贯通删除（含构造参数与 5 处依赖声明），净 −1073/+47；`fail_cache.lock().unwrap()` 等缺陷随 crate 一并消失 | **close** |
| **D-67** | **`llm-gateway` 死 API 与未接的分账**：`fallback.rs:103-132,152-171` 固有 `chat()/embed()` 仅测试调用；`fallback.rs:63-66` 注释称"报告标注本轮用哪条通道"，但 `last_used()` **零生产消费者**（S9 通道分账实际未接）；`registry.rs:103 list_aliases()` 全仓零调用 | llm-gateway 体检发现 | **已修**（P1-35）：删 `last_used` 一族（只写不读）+ 订正不实注释；删仅测试调用的固有 `chat()/embed()` 与 `list_aliases()`；**保留** `with_switch_callback`（有真实生产调用） | **close** |
| **D-68** | **`llm-cn` 死类型/死字段 + 超时口径不一致**：`HunyuanEmbeddingRequest/Response/Data` 零构造（`embed()` 直接 `Err`）、`HunyuanErrorDetail.code` 从不读取（被 `#[allow(dead_code)]` 掩盖）；`llm-cn:280` timeout **300s** 而 `llm-openai:315` 已收紧到 **30s** → 挂起防护口径不一致 | llm-* 体检发现 | **已修**（P1-35）：删死类型与不读取的 `code` 字段；超时 300s → 30s 对齐 `llm-openai` | **close** |
| **D-70** | **`experience` 经验库无界读入（无界读入第 21 落点）**：`ExperienceStore::set_path` 用 `tokio::fs::read_to_string` 整份读 `experience.jsonl`（**只增追加**，长期运行无上限）→ 重启 OOM | experience 体检 | **已修**（P1-36）：改走 `bounded_io::read_file_text_capped`（64 MiB）+ 截断留痕；先红后绿 | **close** |
| **D-71** | **`resource-monitor` ROI/critical 从未接线**：`RoiReport`/`compute_roi`/`is_critical` 全仓零生产调用方（仅自测），且 `compute_roi` 用**硬编码假价**估成本，与 D-46 口径冲突 | resource-monitor 体检 | **已修**（P1-37）：**删除**（不订正） | **close** |
| **D-72** | **`observer::rebuttals_for` 只写不读 + "供下次 run bias"文档错误**：写侧 `apply_rebuttal` 现役（CLI `hearth note --observer-verdict`），读侧零调用方 | observer 体检 | **已修**（P1-38）：删死读取口 + 如实订正两处文案（写侧审计档保留） | **close** |
| **D-73** | **`tool-runtime` 安全声称不实**：自称 "secure …verification"、`install()` 称 "verifying SHA-256 if a file path is provided"（无校验、无 file path 参数）；`sha256`/`command` 零读取；`ResourceLedger::snapshot()` 零生产调用方（注释称 "introspect/报告消费"） | tool-runtime 体检 | **已订正**（P1-39）：纯文案（零行为变更）；注册表本身现役（3 个 API 面） | **close** |
| **D-74** | **`agent-runtime` 会话事件缓冲无界增长**：`Session.events` 注释称 "ring"，实为无上限 `Vec`（14 push / 0 clear）；且它是**断线续传重放源**，不能简单裁剪 | agent-runtime 体检 | 注释已**如实订正**（P1-40）；**增长处置开放**——须先定"重放降级"语义（丢哪些重放、是否落盘再读回），单独立卡 | **登记·待裁** |
| **D-75** | **`agent-core` 读"正在处理的仓库/cwd"文件无界**（第 22~25 落点，性质高于配置读）：`constitution.md`（整份读后再截 6000 字符 ⇒ 截断前先 OOM）、`Hearth.md`（**完全无上限**，且整段注入系统提示）、S12 自检回读 ×2 | agent-core 体检 | **已修**（P1-41）：`bounded-io` 补同步原语；64 KiB / 64 KiB / 8 MiB，均截断留痕；先红后绿；xray 锚点+FNV 同步 | **close** |
| **D-76** | **`agent-types` 死类型簇（TaskGraph 残族）**：`TaskNode`/`TaskResult`/`TaskGraph`(+impl)/`PlanContext`/`Observation`/`PlanState` 零生产消费者；另 `ContentSource`/`MAX_INJECTED_CHARS`/`format_injected_content`（**提示注入来源标注**）零调用方——防护**未接线** | agent-types 体检 | **已修**（P1-43，净 −341/+84）：删上述类型 + `impl` + 5 条单测；**逐符号反向核验**保留 `MessageContent::ToolResults`+`ToolResult`（活：`context.rs` token 统计）、`TaskStatus`（`sync_from_task_graph` 入参）；当时判 `FileChange`"是活的"**判错了**——它只作为类型被 `RunReport` 携带，合并逻辑其实零调用方（**已由 D-83 订正并删除**）。注入能力缺口 → **D-80** | **close（含一处判断订正）** |
| **D-79** | **`SessionLedger` 在生产路径"只读不写"**（**比"D-76 死类型"更重的发现**）：写侧 `add` / `sync_from_task_graph` 全仓**无生产调用方**（仅单测；原生产者 TaskGraph 已拆除）；读侧**活的且用户可见**——`loop.rs:2554` prompt 注入（`render_for_prompt`）、`ledger_pending_texts()` 进 report、CLI `run_local.rs:702` 的 `ledger_pending`。⇒ **账本恒空**，`render_for_prompt()` 恒 None，"零遗漏/零遗忘"机制**实际不生效** | agent-types 体检（P1-43 顺带取证） | **裁决「接线」（联网核实业界做法后，2026-10-01）**：业界通行做法是把**任务态（todo/scratchpad/open-items）由 harness 追踪或经显式工具由模型维护，并每轮重新注入 prompt**（LangChain Deep Agents todo-state、Claude Code 重注入 todo、MAGE 执行态记忆、InfiAgent thinking record、LangGraph 类型化 state / `todoListTools`）。本账本的五栏 + 每栏上限 + "只关不删"设计**与业界一致**，缺的只是生产者 ⇒ 退役会白扔一个被业界证明有效的机制。**已实现（P1-44）**：`ledger_record_selfcheck()` 在 S12 自检闸的事实产生点登记 `KnownFailing`（未过→登记、通过→关闭，同前缀去重）；**先红后绿**回归锁 5 断言。⇒ **`known_failing_open` / G-B 门 / prompt 注入从"恒空"变真生效**。**残留**：`Pending` 栏（`ledger_pending_texts()`，CLI 收尾"还剩什么"行）仍无生产者 → **D-81** | **close（KnownFailing 栏）· Pending 栏另立 D-81** |
| **D-81** | **`SessionLedger` 的 `Pending` 栏无生产者**：D-79 接线只覆盖 `KnownFailing`；`ledger_pending_texts()`（CLI / report 的"还剩什么"）恒空——原生产者 TaskGraph 的"未完成节点"已不存在 | P1-44 收口时如实标注 | **已修**（P1-46）：**找到现成事实源**——`todo_write`（真注册工具，Claude Code TodoWrite 语义）的清单只活在工具输出里、**会被切片裁掉**；账本恰在切片之后注入 ⇒ `ledger_sync_todos()` 全量镜像清单进 `Pending` 栏 = **让清单免疫切片**。先红后绿 + 4 组断言 | **close** |
| **D-82** | **civ 条目"文件改动数"恒为 0**：`note_civ_outcome` 取 `report.files_changed.len()`，而**主 run 的该字段处处为空**（只在子代理路径填）⇒ `hearth civ feed` / `GET /api/v1/civilization` 每条主 run 记录都写"0 个文件改动"，与事实不符 | P1-45 体检（顺带） | **已修**（P1-47）：改用本 run 真实产物事实 `written_files`（按**去重路径**计数）；先红后绿 | **close** |
| **D-83** | **子代理委派子系统整段死亡**：`spawn_sub_agent` / `collect_sub_agent_results` / `active_sub_agent_count` **全仓零调用方**（唯一触发条件 TaskGraph 的 `delegable` 已随线C手术消失，原位注释即载明"spawn 块不可达"）；连带 `extract_files_from_tool_calls` / `merge_file_changes` / `RunReport.files_changed` / `agent_types::FileChange` / `depth` / `MAX_DEPTH` 全为死物 | P1-47 体检（顺带） | **已修**（P1-48）：**整段删除**（与 D-66 删 planner 同口径——生产者结构性消失、无从"接线"）；净 −343/+22。xray `subagent-uses-readonly` 如实报红 → 按既有退役惯例降 yellow 留痕（条数仍 16）+ 复算 FNV。**顺带订正 D-76 里我写过头的一句**（曾称 `FileChange`"是活的"） | **close** |
| **D-80** | **仓库文件进系统提示无来源标注**（间接提示注入面）：`Hearth.md` 取自 cwd（**正在被处理的仓库**）却整段注入**系统提示**（权威最高），**零来源声明、零权威序**。注：`web_fetch` 通道**已自带** `source`/`verifiable`/未验证标注，WS7「Trust-but-Verify」亦在位——**真正缺口只有 Hearth.md 这一处** | agent-types 体检（P1-43） | **已修**（P1-45）：依 OWASP《Secure Coding with AI》§3/§6 补**来源 + 权威序**一行（"来源=仓库内文件…与用户/GOAL 冲突时以用户与 GOAL 为准"）；**不否定项目约定**（约定照用），只切断"文件里的命令冒充用户命令"。先红后绿 + 3 断言（含"约定正文原样保留"） | **close** |
| **D-85** | **`orchestrator` / `replay` 两模块整段死亡**：`orchestrator`（479 行）是**已拆除的 TaskGraph 的执行器**，其全部符号（`execute_plan`/`TaskOrchestrator`/`PipelineRunner`/`TaskStep`/`TaskReport`/`TaskValidator`）引用**只在本文件内**；`replay`（51 行）仅一行再导出、零调用方；连带 `Scheduler::dispatch_single`（唯一用途=给 orchestrator 当 runner closure） | agent-core 逐文件体检 | **已修**（P1-49）：**整段删除**（与 D-66/D-83 同口径——生产者结构性消失、无从接线）；净 −530/+26；xray 无锚点涉及 | **close** |
| **D-86** | **`archive_digest` 无界读入**：归档读回通道把 `.hearth_archive/*.jsonl`（随会话**只增**）**整份**读进内存，却只用**开头** `max_turns` 条 turn——自述"清单不膨胀"，膨胀的是**读**（D-51/D-70 同族） | agent-core 逐文件体检 | **已修**（P1-50）：改走 `bounded_io::read_file_text_capped_std`（cap 8 MiB，按头截断保持语义）+ 截断留痕，仍 best-effort；**先红后绿**（头/9 MiB 噪声/尾三段构造，红侧尾部会被读进 digest） | **close** |
| **D-87** | **孤儿源文件 `agent-core/src/cache_telemetry.rs`**（216 行）：全仓**无任何 `mod` 声明** ⇒ **从不参与编译**——编译器/测试/clippy 全都看不见它，却会被当成现役代码阅读；内容是 `llm-gateway` 现役同名文件的**陈旧副本** | 系统性扫描（脚本枚举 src 下所有 .rs 反查 mod 声明） | **已修**（P1-51）：删文件 + **新增「孤儿源文件门禁」**（`codex-cli/tests/orphan_source_gate.rs`）**防整类复发**；先红后绿（门禁先如实报红且零误报） | **close** |
| **D-88** | **TaskGraph 遗留链的最后一环**：`SessionLedger::sync_from_task_graph`（TaskGraph 时代的账本 ingest，全仓唯一调用方是单测）+ 其专用 `close_by_prefix` + `TaskStatus`（D-76 曾仅为前者入参形态而保留）；另 `Turn.actions: Vec<Action>`（只写空 vec、从不被读的 write-only 字段 + 死类型 `Action`） | "pub 项跨文件零引用"脚本扫描 + 人工核验 | **已修**（P1-52）：全部删除；**核验为冗余而非缺失**（账本已有 D-79/D-81 两个真实生产者覆盖三栏语义）；并订正两处已过时注释。**该链至此清完**：D-76 → D-83 → D-85 → D-88 | **close** |
| **D-84** | **`ToolDispatcher::read_only_view()` 现已无生产调用方**：其唯一生产调用点在 `spawn_sub_agent` 内，随 D-83 一并消失；函数本体与"剥离变异工具"行为仍在（`tool-runtime/src/dispatcher.rs`，自身有单测），且 xray red 能力 `readonly-view-strips` 仍锚定它 | P1-48（D-83）连带发现 | **登记·待裁**：保留（子代理能力日后重启即复用；且它仍是"只读分发器"这一**通用原语**，非子代理专属）还是删除（须同步退役 `readonly-view-strips`）。**倾向保留**，但如实登记免成隐形死码 | **登记** |
| **D-77** | **低危无界读入（配置/manifest/replay，均未走共享原语）**：`service/main.rs:411`(config.toml)/`:459`(providers.json)、`service/templates.rs:31`、`tool-runtime/registry.rs:100`(manifest.toml)、`llm-gateway/cost.rs:276`(`HEARTH_PRICE_FILE`)、`llm-replay/lib.rs:84`(replay 夹具) | 本轮体检归纳 | **裁决「不改」（by design，2026-10-01）**——理由：这六处读的是**操作者自己的本机文件**（配置/清单/价表/replay 夹具），既不来自不可信来源、也**不具备无界增长特性**（对比 D-70 的只增日志、D-75 的 cwd 项目文件、D-55/D-58 的网络响应）。给它们加上限**不会带来任何安全收益**，却有**真实的回归风险**：合法的超大 `providers.json`/夹具一旦被截断 → JSON 解析失败 → provider 发现/夹具加载**静默降级**（比"读全"更糟）。故**不改**；"全仓文件读只剩 `bounded-io` 一份"的收敛目标限于**确有边界的读**，不为此把每处配置读都套上 cap | **close · 裁决「不改」** |
| **D-78** | **零调用方小项批次**：`service::sse.rs:51 sse_stream`（routes 只用 `_with_replay`）、`llm-gateway::types::PropertySchema`（仅自引用）、`llm-gateway::fallback.rs:94` 固有 `stream()` 与 trait 实现**函数体重复**；`CostMeter` 的 `total_prompt_tokens`/`total_completion_tokens`/`entry_count`/`iter` 仅自测调用（删除需同删其断言） | 本轮体检归纳 | **部分已修**（P1-42）：删 `sse_stream` + `PropertySchema`（原位留注）。余项**保留待裁**：`fallback` 固有 `stream()` 删除需先确认无类型推断依赖；`CostMeter` 四个访问器删除会连同断言拿掉、降低 `record()` 覆盖 ⇒ 倾向保留 | **close（部分）· 余项登记** |
| **D-69** | **`project-xray` 函数名与行为不符**：`wiring.rs` `strip_comments_and_strings` 实际**只剥注释、保留字符串**（名与文档均称"字符串"） → 误导维护者（也正是 D-59 假绿的认知来源） | project-xray 体检发现 | **已修**（P1-34）：改名 `strip_comments_keep_strings` + 文档点明因果，零行为变更 | **close** |
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
| **D-13** | 仓库提交态非 fmt-clean（`cargo fmt --all` 改动了未触碰的 `codex-cli/src/session_store.rs`） | 本卡发现 | **已自然解决**：P1-06 门禁口径对齐 CI 后 `cargo fmt --all -- --check` 长期稳定通过（每次卡均以 `FMT_OK` 实证） | **close** |
| **D-14** | `SPEC` 顶部注释亦为旧口径（写"15 能力/24 链"，实测 16 / 27） | 本卡发现 | **已订正**（commit `1729646`） | **close** |
| **D-17** | **会话泄漏 → 无界内存 + 磁盘**（从未 `send_message` 的会话 `finished_at` 恒 `None` → 落入"Active: always keep" → 永不回收；工作区目录同泄漏） | P0-02 审计 → P0-04 | **已修**：`Session` 加 `created_at`，`None` 分支按 idle TTL 回收非运行中者（`running=true` 永不触碰）；回归测试已重写 | **close** |
| **D-24** | `service/src/per_user.rs` 请求路径上的 `.expect()` + **panic 持锁 → 锁中毒 → 其后所有 `civ.write().unwrap()` 二次 panic → civ 接口全站 DoS** | P0-04 专项扫描 | **已修**：签名改 `Result` 上抛 + `lock::recover` 中毒不升级；`webhook`/`user` 同类项一并清理 | **close** |
| **D-25** | `service/src/routes.rs` 限流中间件 `lock().unwrap()`——中毒后**每个请求** panic（影响面全站） | P0-04 专项扫描 | **已修**：`recover` | **close** |
| **D-27** | **限流在途计数在 handler panic 时单调泄漏**（尾部手动 `fetch_sub` 在 unwind 中不执行）→ 同一 IP 泄漏 50 次即该 IP 永久 429、全局泄漏 500 次即**整站永久 429** | P0-05 新发现 | **已修**：改 RAII `InflightGuard`（`Drop` 在 unwind 时仍执行）+ 单测锁定 | **close** |
| **D-28** | `service/src/user.rs`（**每请求鉴权路径**）与 `webhook.rs` 的 `lock().unwrap()`——同 D-24/D-25 缺陷类，残留即全站级失败通道 | P0-05 新发现 | **已修**：`recover` | **close** |
| **D-26** | `tools-builtin/src/web.rs:9` 注释声称"bash curl 后门治理**依赖全局代理 env**"——**该机制全仓零实现**（与 landlock 案例同型） | P0-04 专项扫描 | **已修**：订正为"出网旁路已知未治理"并指向 D-35 | **close** |
| **D-35** | **bash 出网无任何治理**：`web_fetch` 有 fail-closed 白名单，但 bash 的 `curl`/`wget` 可直连任意地址（原被 D-26 的虚构机制遮住） | P0-10 专项 | 确证（**真实防护缺口**） | **close · 顶层裁决「接受」（显式风险接受，2026-10-01）** |
| **D-36** | `service/src/webhook.rs` `curl … .output()` 无界输出 + `kill_on_drop` 默认 false（超时不收尸），与 D-18/D-32 同类（在 fire-and-forget 任务里） | P0-10 专项 → P1-13 修复 | **已修**（P1-13）：改走新的 `run_bounded(cmd, timeout, cap)`——两路 `bounded_io::drain_capped_async` 并发**有界排空到 EOF**（不再 `.output()` 全量入内存）+ 超时 `kill_process_tree` **树杀 + `wait()` 收割** + `kill_on_drop(true)` 兜底；截断留痕不静默。**哨兵法**回归锁：超时后子进程必须已被杀（哨兵文件不得出现） | **close** |
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
