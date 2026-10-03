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
**累计卡数 110**（含 P1-99）。最新五轮 **P1-56 ~ P1-99 ＝ D-92 ~ D-135**（2026-10-02/04）：
①**"用户可见面 vs 代码"第三轮**（API 契约缺项 / 配置参考 / provider 注册与降级链）；
②**"未体检 crate"体检**（subconscious / tool-runtime / experience / observer）；
③**长程债务收口 + CLI/provider 面**（D-74 会话事件缓冲；CLI 选项被静默忽略；多数票
子串误判；出站限流覆盖不全）；
④**onboarding/上手面 + 经验复用裁决**（`.env` 接线；experience 复用接口按联网核实结论退役）；
⑤**service/文档可见面**（文明线公告写进无人读取的档 ⇒ 用户看不到；README 安装命令等
多处与代码不符）；
⑥**入库卫生第 3 轮**（误跟踪的 Python 字节码缓存 ⇒ 摘除 + 门禁防复发）；
⑦**agent-core 未体检模块体检**（terminal/context/scheduler/talent/constitution）——
发现并退役一整层"被结构性架空"的失败分类/恢复策略决策支持层；
⑧**零调用方小项收口**（D-78 余项重复 `stream()` 删除 + D-84 `read_only_view` 裁决保留）；
⑨**D-109 先设计后接线**（civilization 写入补 session→owner 归属链：写入器工厂按
owner 绑定 per-user 档 + 全局档停用 + 行为锁 + xray 锚点复算）；
⑩**D-117 CLI env 名字对齐**（用户给出可用 Agnes key 后实测发现"401"其实是
CLI 只认 `HEARTH_*`、不认 `.env` 里的 `AGNES_*` ⇒ 照模板配好仍"未配置"；
修复后**活体端到端跑通**：provider=agnes、答 4、exit 0）；
⑪**D-116/D-118 经验复用前置**（取数口 + 失败教训真信号 + 默认关的注入；发现并修
"条目非教训 + 质量为结构常量"）；
⑫**D-119/D-120 打通基准语料链**（CLI 接经验库 + 延迟加载；并修掉"落盘静默丢条"
与 fire-and-forget 竞态——**实测**发现 CLI 跑完 exit 0 但经验文件根本没生成）；
⑬**D-116 A/B 实测裁决**（T07 fixture + `agnes-3.0-flash`，N=5/臂、注入 5/5 ⇒
过检率两臂同为 5/5、步数未降反略升 ⇒ **保持默认关**）；
⑭**"未体检面"第四轮体检**（`bridge` 端到端核实已挂载 / `nervous-system` / `resource-monitor` /
`bounded-io` / `llm-replay` / `api` / `codex-cli/src/run_local.rs`）——命中 **D-121**
（`HEARTH_TASK_TIMEOUT_SECS=0` 注释称"显式关闭"，实现却是"第一步即超时"，
**声称与实现正好相反**）；
⑮**CLI 参数面实测**——命中 **D-122**（顶层参数未标 `global` 且被重复声明 ⇒
**CLI 自己印了 14 次的指引命令跑不起来**；`chat --api-key` 在远程模式下被静默忽略）；
⑯**门禁自身体检（xray 引擎）**——命中 **D-123**（`severity` **无取值校验**：
spec 里写 `"Red"`/拼错会被下游按"非 red ⇒ 不阻断"**静默放行**，一行 typo
即可关掉一条 red 红线；同时证伪一条"哈希锁不进门禁"的疑似项——CI 的 `test`
步与 `xray wiring` 步**都跑**，锁在 `cargo test` 侧已生效）；
⑰**CLI 写操作面**——命中 **D-124**（HTTP **状态码从不检查**：4xx/5xx 的 JSON
错误体被当成成功响应；`tasks done` 因此**无条件谎报成功**；反向"200 + 空体"
又解析失败——两个方向都拧着）；
⑱**CLI 文件读入面**——命中 **D-125**（会话档/状态档/`/file`/note 档四处
`std::fs::read_to_string` **无上限**，与 D-51/D-70/D-86 同族）；
⑲**非 Rust 侧（ember）体检**——命中 **D-126**（工具集 `read`/`edit` 的
"整份读入再截断"、HTTP `resp.read()` 无上限、`edit` 读-改-写无尺寸门；
以及"未知工具"提示**硬编码三工具**而实际四工具 ⇒ 模型拿到错的可用名单）；
⑳**可复现性门禁**——命中 **D-127**（CI 的 cargo 步骤**不带 `--locked`**：
`Cargo.lock` 与 `Cargo.toml` 不一致时 cargo **静默重写锁**并让 CI 假绿——
P1-90 漏提交锁时正是如此；已补 flag + 门禁）；
㉑**上手面（onboarding）实测**——命中 **D-128**：`hearth setup` 会
**截断重写 `.env`**，把用户按 `.env.example` 配好的 provider key **静默清空**
（实测 3 行 → 1 行，随后只报"未配置 API key"）——**用户数据销毁级**可用性缺陷；
㉒**CLI 选项来源全覆盖**——命中 **D-129**：D-102 只把 `--mode` 校验接在 **flag** 上，
`HEARTH_MODE` / `config set mode` 两个来源仍**既不生效也不报错**
（实测 `HEARTH_MODE=remote` 无 `--url` 仍**本地直跑**、`bogus` 连错都不报）；
㉓**service 启动期死物**——命中 **D-130**：`/tmp/codex.pid` 写入的注释称
"for Observer health checks"，但**该消费者不存在**（全仓无读取方）+ 硬编码
`/tmp` 在 Windows 必然静默失败 ⇒ 删死物 + 新增退役门禁。
㉔**无界读入"确有边界者"收口（第 26~28 落点）**——命中 **D-131**：把 D-33/D-51/D-70/
D-75/D-86/D-125 未覆盖、且**确有边界**的三处裸读收口——`setup` 读 `.env`（**兼修
静默清空**：旧 `read_to_string(..).unwrap_or_default()` 在读失败〔权限/非 UTF-8〕时
退化成空串 ⇒ merge+write 把用户 `.env` 覆盖成 1~2 行，＝ **D-128 修掉的数据丢失换
触发面**）、`coverage` 读 tarpaulin 报告（含逐行明细、随仓库规模增长）、`/cost` 读
会话累积报告。**本卡同时是一次口径纠偏（如实记）**：动手时曾把 service 的
`config.toml`/模板清单与 tool-runtime 的 manifest 一并"有界化"，**回查债队列才发现
正是 D-77 已裁决「不改」者**（操作者本机文件、无增长特性、加 cap 反有"合法大文件被
截断 → 解析失败"的静默降级风险）⇒ **已按 D-77 全量回退**，并把该口径**钉进代码**
（`bounded-io-exempt:` 标记 + 理由）+ **新增门禁** `unbounded_read_gate`（**只扫
`crates/codex-cli/src`**——service/tool-runtime 在 D-77 裁决范围内，不该被门禁逼着加
cap；先红后绿已复验）。
㉕**门禁尺子自身体检（第 2 例）**——命中 **D-132**：`window-framework/gate_window.py`
的判定只看**解析出的数字**，退出码仅在"一个数字都解析不出"时才参与
（`if rc != 0 and passed == 0 and failed == 0`）⇒ 「套件先打印 `N passed, 0 failed`
再崩（退出码非零）」被判 **PASS**——而每个套件末行恰是 `sys.exit(1 if failed else 0)`，
**退出码才是权威信号**（同 D-123/D-127 的"防线自己失守"）。处置：抽纯函数
`parse_suite_result`——非零退出码一律 FAIL + 正则**锚定 `RESULT:` 汇总行** + 无汇总行
FAIL + **零断言 FAIL**（vacuous green）；新增**尺子自检套件**（13 断言，纳入门禁
208→221）。**双重红侧实证**：旧逻辑跑主案打印 PASS；端到端注入"打印 5 passed 后 exit 1"
探针 ⇒ 门禁如实 FAIL + exit 1（历史验收只验过"失败套件/缺失套件"两种红，第三种无人验）。
㉖**不实注释（window-framework）**——命中 **D-133**：`framework.py` 头注称
"snapshot/rollback/conflict/budget/export/import/template 在 --help 占位"，实测**除
`budget` 外**这些命令早已实现并注册进 `main()`；`FrameworkCheck` docstring 称"4 条断言"
实为 **5 条** ⇒ 均据代码订正（同 D-45/D-54/D-111④ 口径）。
㉗**用户可见面静默失败留痕（`let _ =` 丢弃 Result 族）**——命中 **D-134**：REPL 审批提交
三处（`approve`/`deny`/超时 auto-deny）与 AI 事件落盘各一处 `let _ = ...` **丢弃 `Result`**。
① 审批提交失败（service 掉线 / 404 / **非 2xx 空体**）时用户**看不到任何提示**，误以为已批准，
而会话其实仍在等服务端决策（随后卡住/超时）；② `persist_ai_events` 落盘失败
（`~/hearth/observer` 建不出 / 写不进 / 磁盘满）时 Observer 素材**静默丢失**、任务照常 exit 0
（同 D-120「静默丢条」族）。两处均改失败 `render::error` 留痕、**不阻断主流程**
（审批/落盘属可恢复的外部动作，不让交互会话或任务陪葬；与紧邻 transcript 落盘同款处置）。
㉘**交互提交路径"谎报成功 + 静默丢条"（同族再命中）**——命中 **D-135**：`run_local.rs`
的 `render_agent_event` 内三处 `resolve_interaction` 调用写法各异、其中两类有缺陷：①**澄清分支**
先 `.is_ok()` 转 bool、随后 **`let _ = resolved;` 丢弃 Result**，却**无条件**打印「✓ 澄清已提交」
——而该函数在 `interaction_id mismatch` / `no pending interaction`（重复提交/已解决）时会
**真实返回 Err**，此时内核并未收下澄清，用户却以为已生效（会话随后走自己的超时/reassess 路径）；
②两处 **headless 放弃/拒绝分支** `let _ = resolve_interaction(..)` **静默丢弃失败**（D-120「静默丢条」
族）。处置＝① 澄清分支改为 `.context("澄清提交失败")?`，**对齐紧邻审批分支**（早已是
`.context("审批提交失败")?` 的 fail-closed 语义——同一函数内两分支行为本不该拧着）；
② 两处 headless 分支改为失败 `render::error` 留痕、保持**不阻断**（放弃为既定意图）。
共 **43** 张卡均已修/订正，每卡独立 commit + 门禁四件套 + 推送（另加
**活体端到端冒烟**一次：`.env(AGNES_*)`→CLI→provider=agnes→答出结果→exit 0）；
开放债＝**无**（D-116 已经 A/B 实测并裁决「保持默认关」；D-119/D-120 已收口）。
唯一后续选项（非阻塞）：用 D-118 已能捕获的**真实错误细节**重建更富信息的语料，
再重测 D-116（当前语料只有"预算耗尽"一类弱教训）。
（D-78 余项 / D-84 / D-109 / D-116 / D-117 / D-118 / D-119 / D-120 均已收口；
LLM key 已可用——实测 `agnes-3.0-flash` HTTP 200。）

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
| P1-53 | `4c3c161` | **D-89 收口（订正面向用户的配置参考 + 新增配置文档门禁）**：用"环境变量旋钮：文档承诺 vs 代码读取"配对扫描，发现 `docs/configuration.md`（**活配置参考**）有 **8 个幽灵旋钮**——用户照文档设置**毫无效果**：`RETRIEVER_ENABLED`/`LSP_ENABLED`（能力已随 P1-04 整删）、`EMBED_*`（embedding 已随 v21.0 移除）、`CODEX_SANDBOX_ENABLED`/`CODEX_SANDBOX_WORKSPACE`（**从未实现**，`create_sandbox` 只按平台选）、`SESSION_TTL_SECS`（**名字写错**，真名 `OOM_TTL_SECS`）。处置：据实重写（并补上此前未收录的 `OOM_TTL_SECS`/`OOM_SWEEP_INTERVAL_SECS`）+《已失效》节（**故意不用反引号**）+ **新增门禁** `config_doc_gate.rs`（反引号标出的旋钮必须真被代码读取；`RUST_LOG` 走白名单）。**先红后绿**：门禁先如实报红、**精确列出 8 个幽灵、零误报**。另核验 README 的 `HEARTH_BASE_URL`/`HEARTH_BIN_DIR` 确由 `bench/install.sh` 读取 ⇒ 非谎报 |
| P1-54 | `56faba3` | **D-90 收口（`/openapi.json` 是"零端点的空契约"→ 补全 27 条 paths + 漂移门禁）**：`ApiDoc` 只声明 schemas、**零 paths** ⇒ 对外是一份**声明了 0 个端点的 OpenAPI 文档**，而实际有 **27 条**路由；`/swagger-ui`（**早已 vendor**，原注释却称"可日后再 vendor"）读的正是它 ⇒ 浏览器打开即**空列表**。处置：新增 `service/src/openapi.rs`（`API_ROUTES` 镜像表 + `build_openapi_document()` 运行时注入 paths，**不逐个改 handler** 以免再漂移；镜像表置于 **lib** 便于测试直调；**术语据实**：镜像表≠唯一事实源，保真交门禁）+ 去掉 handler 里的 `expect` panic（改 500 + 留痕）+ 订正过期注释。**新增门禁** `openapi_route_gate.rs`：**双向**比对 `.route("…")` 字面量、并断言"paths 条数==镜像表条数"（**专治"零 paths 也照样绿"**）。**先红后绿**：临时关注入 → 如实 `left:0/right:27`（修复前行为）→ 恢复转绿；红检钩子已移除。**⚠️ 该提交推送后 CI 立即变红——但根因与 D-90 无关（未改动的 llm-gateway 被 1.99 新 lint 命中），详见 P1-55 / D-91** |
| P1-55 | `4945210` | **D-91 收口（钉住工具链：消除"本地绿 / CI 红且不可复现"）**：D-90 推送后 CI **35 秒即红**，报错在**未改动**的 `llm-gateway/src/provider.rs`——`clippy::double_must_use`（**1.99.0 新增**）指向 `#[async_trait]` 宏展开处，而本地 1.98.1 **看不到**。根因＝**工具链漂移**：CI 用 `dtolnay/rust-toolchain@stable`、仓库无钉版文件，stable 于 9-28 由 1.98 滚到 1.99。处置：① 新增仓库根 `rust-toolchain.toml`（`channel="1.98.1"`＝已验证 65 张卡全绿那版，含**升级仪式**）；② CI 改 `@master` + 显式 `toolchain:"1.98.1"`；③ **新增门禁** `toolchain_pin_gate.rs`（钉版文件必须存在且为具体版本号／工作流不得用浮动通道／两处版本必须一致）；④ **不撒 allow**——实测该 lint 命中**每一个** `#[async_trait]`+`Result` 的 trait（5+ crate），属宏产物误报，给多 crate 撒豁免不如钉版；障碍与处置建议已写入钉版文件。**本地 `rustc 1.98.1` 四件套 FMT=0 CLIPPY=0 TEST=0 LINUX=0，CI 恢复绿（2m34s）** |
| P1-56 | `6af10f0` | **D-92 收口（接线 `GET /api/v1/tool-registry`）**：`routes::registry_list` **handler 完整存在**、`tool_runtime::registry` 契约文档把它与**已接线的** `search`/`install` 并列为三件套，但 `main.rs` 的 Router **从未挂载** ⇒ 用户按文档调用必 **404**；其 `ToolRegistry::list()` 因此也零生产调用方（`TOOLS_DIR` 自动发现的清单只有 `search?q=` 一条野路可查）。处置＝**接线而非退役**（与 D-90 同向）：挂路由 + 同步 `API_ROUTES` 行 + 订正 openapi.rs 头注过期计数（27→28）。**先红后绿**：只挂路由不补镜像表时 `openapi_route_gate` 精确报"路由有、文档无：[/api/v1/tool-registry]" |
| P1-57 | `b3334bb` | **D-93 收口（配置参考补全为与代码一致的完整参考，6 → 30+ 旋钮）**：`docs/configuration.md` 只列 6 项，而全仓实际读取 80+ —— 运维**无法从文档发现**多 provider（`OLLAMA_*`/`VLLM_*`/`DEEPSEEK_*`/`GEMINI_*`/`ZHIPU_*`/`AGNES_*`/`DOUBAO_*`/`HUNYUAN_*`）、`TOOLS_DIR`/`CODEX_TEMPLATES_DIR`/`CODEX_OBSERVER_DIR`/`CORS_ORIGIN`/`ALLOW_NO_AUTH`、内核 `HEARTH_MAX_STEPS`/`HEARTH_EGRESS_ALLOWLIST`、沙箱 `HEARTH_CGROUP_BASE`/`HEARTH_ALLOW_NO_CGROUP`/`HEARTH_SECCOMP_MODE` 等部署必需项。处置＝四节新增（默认值**逐项取自代码**）+ 写明覆盖范围（service/内核；CLI 专属指向 `--help`）+ 把 D-96/D-97/D-94 的行为契约写进正文。遵守本文件契约（表格行反引号 = 必须真被 `env::var` 读取），`config_doc_gate` 复跑绿；`EPERM`/`GET`/`KILL` 等非变量名**一律不加反引号**（否则门禁会把它们当幽灵旋钮） |
| P1-58 | `e826a3e` | **D-94 收口（删 `ProviderRegistry` 的死 `default` 机制，净 −78/+47）**：`default` 字段 + `set_default` 参数 + `get_default()` 一整套"默认 provider"语义，启动期被写 3 次 `true`，但 **`get_default()` 全仓唯一调用者是它自己的单测**——service 侧 `SessionCreate.provider` 是**必填**字段，`create_session` 直接 `registry.get(&provider)`，响应体里根本没有"默认 provider"。即那 3 处 `true` 只是往**永不读取**的字段写值。且 main.rs 把参数注成 "required"、宣称 "OpenAI primary"，而按后写覆盖真"默认"会落在 deepseek——**三重不实**。处置：删字段/方法/参数/专属单测（16 + 10 处调用点去实参）+ 订正注释；顺带删 `run_local::registry_smoke` 里"建了即丢"的空壳注册表。**零行为变更** |
| P1-59 | `9340cd4` | **D-95 收口（退役 providers.json"模型自动发现"）**：该块在 `./providers.json` 不存在时写默认清单、再读回来逐个 `tracing::info!("model auto-discovered")` 后**丢弃**——发现的模型既不注册也不影响任何路由（`docs/global-panorama-v11.5.md:39` 早已如实记为半接线债）。**为何退役而非接线**：默认清单里就写着 `{"provider": "anthropic"}`，而本仓**没有 anthropic 实现**，"发现即注册"按设计不成立（还缺一层 provider-name → 构造器 工厂）；给一个与 env 注册重复、且不会工作的机制补工厂＝过度设计。处置：删整块（留 D-95 说明）+ `PROVIDERS_PATH` 记入《已失效》节（纯文本非反引号） |
| P1-60 | `333b58d` | **D-96 收口（provider 注册收紧为"有非空 key 才注册"）**：agnes/zhipu/gemini/deepseek(+pro)/zhipu-max 六个 provider 用 `unwrap_or_default()` **无条件注册** ⇒ 未配 key 时注册的是一个**空 key provider**：它会出现在 `GET /api/v1/models`（客户端据此选中）却任何调用都 401；若开 `FALLBACK_CHAIN` 还会被组装进链**在中途失败**。即"可选但不能用"＝最坏的一种可用性缺陷（hunyuan/doubao 的 `KEY=""`、ollama/vllm 的 `BASE_URL=""` 同类空串误判）。处置：统一 `.ok().filter(\|k\| !k.is_empty())` 收紧，与 P0-1「无 key 拒启」同一条"不静默降级"红线；顺带订正两处已失效的"⭐首选/⭐主力"排名注释。**有意行为变更**：`/models` 只列真能用的 provider，未注册者得到明确 not-found |
| P1-61 | `46f78e5` | **D-97 收口（fallback 链顺序显式化）**：`FallbackChain` 语义是"**第一个 = 主通道**，其后按序降级"（fallback.rs 模块头 + S9 通道切换投影都依赖此序），但 main.rs 组装时直接消费 `registry.list()`＝**HashMap 键迭代序** ⇒ 每次进程/构建顺序都可能不同，等于"谁是主 provider"**在运行期随机**（`[fallback] provider: a → b` 的投影也随之飘，同尺基准不可比）。处置：新增**列表写法** `FALLBACK_CHAIN=a,b,c`（按给定顺序，a 即主通道；未注册的名字 warn 跳过）；`1`/`true` 保留为"全部已注册 + 按名字排序"的兼容写法；空链不再静默（warn 且不注册）；启动日志补 `primary` 字段便于运维核对 |
| P1-62 | `b2177da` | **D-98 收口（两个上下文结构体里"只写不读"的字段批次，净 −45/+26）**：① `subconscious::GuardContext` 的 `goal_text` / `step_count` / `last_success` / `constitution_summary` 由 agent-core 每步填好后**无人读取**——两个现役守卫（`ConstitutionGuard`/`CostGuard`）只读 `last_action` 与 `cost_ratio`；② tool-runtime `ToolContext.deadline_clamped` 唯一写入点是 dispatcher、**全仓零读取方**（dispatcher 判定超时归属用的是**同名局部变量**），连同零调用方的 `remaining_task_time()`（dispatcher 内联同一算法）一并删除。**零行为变更**；xray `cost-guard-live` 锚点未受影响（13/16、0 red） |
| P1-63 | `e045595` | **D-99 收口（observer 澄清跳过率恒 0，修 + 回归锁）**：metrics 汇总 `NeedApproval` 时把请求类型**硬编码成 `"approval"`**，而 WP-0 早已把内核事件泛化为 `InteractionRequested { kind }`、对外把 **kind 放在 `NeedApproval.action`**（agent-runtime 映射点注释即写"action=kind"，CLI 亦按 `action == "clarification"` 分流）⇒ `req_kind` 永远只有 "approval" ⇒ `kind == "clarification"` 分支恒不进入 ⇒ `clarify_req`/`clarify_skip` 恒 0 ⇒ 报告与规则引擎长期读到一个**结构性 0**。改为读事件自带的 kind（数据一直在事件里，只是没读——D-92/D-98 同族）；顺带去掉同分支无意义的 `if true {…}` 包裹。**先红后绿**：新增 `test_d99_clarify_skip_rate_from_action_kind`，红侧如实 `left: 0.0, right: 1.0` |
| P1-64 | `e9265d4` | **D-100 收口（experience 复用回路已断，如实订正 + 登记待裁，零行为变更）**：`experience` 模块头自称"JSONL 持久化 + **关键词检索**"却**无任何检索 API**——唯一会 `reference_count += 1` 的 `search()` 已随 D-47「死代码清理」删除 ⇒ `reference_count` 无自增点恒 0 ⇒ `upgrade_core()`（`>=3` 条件）恒空、`metrics().reuse_rate` 恒 0（**经 `GET /api/v1/experience/metrics` 对外暴露**）；注入侧同源：`injected_experience` 每 run/step 恒复位 None（非 None 唯一写入在测试里，生产者随 D-9 线C手术删除）。即"存/剪/计数活着，**'用起来'这一环缺失**"，且 `self-evolution-proof-v17.md` 的招牌结论（reuse_rate=0.49 证明生产路径真实工作）**已失效**。处置：订正模块头 + 三处字段/方法文档 + openapi 摘要如实标注口径 + 给 v17 证明与 CHANGELOG **加 dated 勘误**（历史正文按惯例不改）；**不单方改行为**——该 crate 牵着对外端点 + observer 定时任务 + red 级 wiring 锚点，真正的分歧是"要不要把经验重新注入提示"（v17 利弱模型 / v18 害强模型，当年才做成自适应门控），故登记 **D-100 待顶层裁**（接线 vs 退役） |
| P1-65 | `6b6c00f` | **D-74 收口（会话事件缓冲加上限 + 绝对 seq 基址，净 +143/−25）**：`Session.events` 无上限只推不减（`get_history` + SSE 断线续传重放的共同数据源）。此前不敢加限的**真隐患**是：重放侧用 `EnvelopeState::new()` **按位置重新编号**后再按 `env.seq <= last_event_id` 过滤 ⇒ 一旦截断，编号整体前移，**早于保留窗口的客户端会静默收到 0 条**（丢事件且无感知）。处置（先定"重放降级"语义再动手）：① 常量 `MAX_SESSION_EVENTS = 50_000` + 唯一入队口 `Session::push_event`——越界**批量**丢最旧 MAX/10 条（摊还 O(1)，避免逐条 `remove(0)` 的 O(n) 搬移）+ 推进基址 + **首次**越界 warn 一次（不刷屏）；② 新增 `events_base_seq`（= 已丢弃数），重放侧 `env.seq = replay_base_seq` ⇒ 重放与实时流**绝对编号一致**：未截断（0）时与历史版本**逐字节相同**，截断后落后客户端**收到保留窗口内全部事件并看到 id 跳变**（标准 Last-Event-ID 降级，可感知）；③ 录制导出与 observer 报告落盘同样以基址起编号；④ 10 处 `events.push` 全部收口，两处 SSE 出口改用 `session_events_with_base()`。回归锁 `test_d101_event_buffer_capped_with_absolute_base`（断言 `base + len == 总 push 数`，绝对编号**不丢不重**） |
| P1-66 | `55c0249` | **D-102 收口（CLI "选项被静默忽略"两处）**：① 远程 `chat`/`repl` 把 provider **硬编码 "deepseek"** 且不传 model ⇒ 用户 `--provider openai --model gpt-4o` 实际仍按 deepseek 建会话（帮助文本却宣称生效）；② `--mode`（含 `config set mode`/`HEARTH_MODE`）只被塞进 `ResolvedConfig.mode` 后**全仓无人读取** ⇒ `--mode remote` 不给 `--url` 时仍**本地直跑**，未知取值也被吞。处置：`create_session` 增可选 `model`（仅 Some 非空才下发，不改变请求形状）+ 远程两条路径改走与直跑**同一套三级解析** + 新增 `validate_mode()`（auto=按 `--url` 判定、remote=强制远程且必须 `--url`、其它值拒绝）+ 帮助文本订正。回归锁 `test_d102_validate_mode_has_real_semantics` |
| P1-67 | `7cc52d7` | **D-103 收口（bridge 多数票方向反了）**：`MajorityVote` 用 `content.contains("agree")` 判赞成，而 **"disagree" 里就含子串 "agree"** ⇒ **每一张反对票都被计成赞成票**，`Majority: N agree` 的数字是假的；另：无 agree/disagree 关键词的回复（含 `content` 缺失被 `unwrap_or_default()` 变空串）**静默计入反对**、零留痕。处置：抽出 `classify_vote()`（**先判否定词再退回肯定词**，皆无 ⇒ `None`），调用点 `None` 时**保守计反对 + warn 留痕**。**先红后绿**：红侧如实 `left: Some(true), right: Some(false)`（"I disagree" 被判赞成） |
| P1-70 | `f709d66` | **D-106 收口（CLI 从不读 `.env`，接线）**：`hearth setup` 会把 `CODEX_URL`/`CODEX_API_KEY` 写进 `./.env`，仓库里也有既定模板 `.env.example`（"复制为 .env 并填入真实值"，且明确写着这是 **CLI/Service** 的模板）——**但只有 service 调 `dotenvy::dotenv()`，CLI 从不加载**（连依赖都没有）⇒ 用户照模板配好 key，`hearth chat` 仍报"未配置"，**整条 onboarding 路径对 CLI 是死的**。处置＝接线（`.env` 已在 `.gitignore` 首行、`git check-ignore` 已核验；service 早已加载；`dotenvy` **不覆盖**进程 env ⇒ 与"参数 > env > config > 默认"不冲突）：`codex-cli` 增 `dotenvy` 依赖 + `hearth_main()` 最开头（**任何 env 读取之前**）加载。顺带订正 `.env.example` 两处过期描述（"CLI chat 写死用 deepseek" 已被 D-102 修；"OpenAI 可选/留空则占位 key" 实为 **service 启动硬前置**）；`docs/configuration.md` 头部补 `.env` 加载与优先级说明 |
| P1-72 | `655d3e8` | **D-108 收口（文明线公告写进"无人读取的档"）**：读接口 `get_civ_feed` 读 `per_user.civ_for(uid)`（v8.0 多用户隔离后的**唯一可见** store ＝ `MEMORY_DIR/<uid>/civ.jsonl`），而 `create_session` 写**全局**档（`MEMORY_DIR/civilization.jsonl`，**另一个文件**）⇒ "session created" 公告写进无人读取处，`hearth civ feed` 永远看不到（D-48 重接线的可见面因此仍未真正生效）。处置：写侧对齐读侧（handler 增 `HeaderMap` 解析 uid → 写 per-user；失败 best-effort 但 **warn 留痕**）；**刻意不采用"读侧合并全局档"**（全局档含各用户 goal 文本 ⇒ 跨租户泄露，已写进函数文档）；顺带清掉两个**只写不读**的 `AppState` 字段（`civ_store`/`workline_store`）。**新增门禁** `service/tests/civ_visibility_gate.rs`（钉住 `create_session` 必须写 `per_user.civ_for`、不得出现 `civ_store`），**先红后绿** |
| P1-73 | `655d3e8` | **D-109 登记（agent-loop civ 写入缺归属链）**：`CivWriterAdapter` 仍写全局档（与 D-108 同一病灶的另一半）⇒ 经它写入的 milestone/reflection 在 API 上不可见。**不能**靠读侧合并全局档修（跨租户泄露），必须补 `session → 归属用户` 链，而 `Session` 现无 owner 字段（`create_session` 也不解析 uid）⇒ **需先设计的接线**（同 experience 复用：先设计再接线）。已写入适配器文档头（不静默） |
| P1-76 | `3ba9492` | **D-111① 收口（service 错误码/静默失败/不实注释）**：① `open_artifact`/`open_external` 的 `Err` **一律**映射 `ERR_SESSION_NOT_FOUND` **配 400**——把"路径穿越被拒/产物读失败"报成"会话不存在"，且码与状态自相矛盾；而 SessionManager 的错误只能靠文本区分，本项目**明令禁止**文本判定（RC20）⇒ 改**先显式判存在性**（404 vs 400），与文件内 `session_stream` 既有做法同款；② `install_tool` 失败返回 `ERR_INTERNAL` **配 400** ⇒ 改 `ERR_INVALID_PARAM`；③ `templates.rs::load` 单文件读/解析失败被两层 `if let Ok` **静默丢弃**（用户只见"模板不见了"）⇒ 逐项 warn 留痕；④ `webhook.rs` 注释称"per-user keyed by user_id"而实现是裸 `Vec` ⇒ 如实订正 |
| P1-77 | `82f1d3e` | **D-111② 收口（service 死项/不实文档）**：① 删 `service::user::UserContext`（自称"每请求注入"，实为**无构造方无读取方**的死类型；真实身份解析是 `get_user_id` + `PerUserStore::civ_for(uid)`）；② 删 `TemplateManager::get`（零调用，`/templates` 只用 `list()`）；③ 删 `AppState.observer`（请求态**零读取方**；真正的第三权接线是 `sessions.set_observer(...)`，拷贝只会让人误以为每请求都过 Observer）。**保留而非删除**并如实留痕：`api` 的四个零生产者错误码常量（对外错误码词表，删掉＝单方缩小契约面；标注"现役 5 个 / 预留 4 个，启用须同时接线"）。**D-111 至此全部收口** |
| P1-78 | `d47641f` | **D-113 收口（入库卫生第 3 轮：解释器字节码缓存）**：`.gitignore` 只窄忽略 `window-framework/src/__pycache__/` ⇒ `bench/`、`docs/data/`、`ember/` 三处共 **4 个 `__pycache__/*.pyc` 被误跟踪**（CPython 自动生成的字节码，非源码）。处置：`git rm --cached` 4 文件（内容留盘）+ 窄规则改全局 `__pycache__/`+`*.pyc`+`*.pyo` + 门禁 `gitignore_runtime_products_gate.rs` 新增 `interpreter_caches_are_gitignored`（**先红后绿**：移除规则即如实报红）。**与 D-5/D-112 同一族**（运行期产物不得入库） |
| P1-79 | `e67ae48` | **D-114 收口（agent-core 未体检模块体检：退役"被结构性架空"的失败分类/恢复策略层）**：`terminal.rs` 四组纯函数（`classify_failure`/`FailureKind`、`failure_strategy`/`RecoveryStrategy`、`strategy_suggestion`、`completion_readiness`/`CompletionReadiness`）**全仓零消费者**（仅自测）；其唯一下游通道（scratch 键 `last_failure_class`/`last_recovery_strategy` 的"R5-1 中转注入块"）早于 R6-5 依"判定权归还范式"主动删除、Reflect 相位分类块亦随线C手术（D-10）消失 ⇒ 生产者结构性消失且 R6-5 明载"勿复活旧通道" ⇒ **退役删除**（同 D-66/D-83/D-85 口径，非重新接线）。连带移除 loop.rs 中**恒空**的两处 report 投影 + handover 字段 + 快照白名单两键，据实订正 3 处指向已删生产者的过期注释；保留现役 `normalize_terminal_state` 与 G1 九态封闭集（并订正 `is_terminal_state` 的不实 doc）。**新增门禁** `retired_failure_channel_gate.rs`（逐行剥 `//` 后文本级拦截退役标识符；**先红后绿**——注入代码即报红）。净 −432/+122 |
| P1-80 | `a9b34c7` | **D-78 余项 + D-84 收口（零调用方小项，须先确认再删）**：① `FallbackChain` 的**固有** `stream()` 与 `impl LlmProvider` 的 `stream()` 函数体**逐字重复**、且零生产调用方（生产一律经 `Arc<dyn LlmProvider>` 走 trait；具体类型调用点仅在单测）⇒ **删除固有份**（同体两份是漂移陷阱），删除后具体类型调用由 trait 兜底、语义一致（llm-gateway 33 测全过）；`CostMeter` 四访问器按原判保留（其断言承担 `record()` 覆盖）。② `ToolDispatcher::read_only_view()` 唯一生产调用方（子代理）已随 D-83 消失、现仅单测调用 ⇒ **裁决保留**并如实登记——它是 xray **red 锚点** `readonly-view-strips`（安全属性锁）锁定的只读视图原语，删之即失去回归防线，且子代理重启可复用 |
| P1-81 | `6634ea9` | **D-109 收口（先设计后接线：civilization 写入补 session→owner 归属链）**：agent-loop 的 civ 自动写入经 `CivWriterAdapter` 落**全局**档，而 API 读侧读 **per-user** 档 ⇒ 自动写入的 milestone/reflection 在 `hearth civ feed` 不可见（D-108 病灶的另一半）。**设计**：把归属用户在会话创建时绑进写入器——新增 `SessionManager::set_civ_writer_factory`，在 `create_session_with_owner(req, owner)` 以该会话 owner 调用一次工厂；`PerUserStore::civ_for` 是**同步**方法，故同步 `append_civ` 内可直接落档（无需异步查表），组合根闭包绑定 owner（依赖倒置保持）。**处置**：`CivWriterAdapter` 改为 holder `per_user` + `owner`，经 `civ_for(owner)` 落档；全局 `civilization.jsonl` store **停止构造**（无人读）；`create_session` 增 owner 变体（旧调用方委托 owner="default"，零改）；`routes` 传 uid；xray `civ-auto-written` 后两环锚点改 `set_civ_writer_factory(` + 复算 FNV（`0x8d515d8fb29c18f9`→`0x457ac49206c92a81`，条数仍 16 / severity 仍 red）。**防复发**：civ_visibility_gate 增 `civ_auto_write_goes_to_visible_per_user_store`（**先红后绿**）+ main.rs 行为锁（写入落 owner 档 + 跨租户隔离 + `civ_for` 读回可见） |
| P1-82 | `7eb8372` | **D-117 收口（CLI env 名字对齐——"401"真因是 CLI 不认 `.env` 的名字）**：用户提供 Agnes key 后先**直接验证**（HTTP 200，真实补全）⇒ 早前"key 被服务端 401 拒"的结论**证伪**；真因是 **CLI 只认 `HEARTH_*`**（`HEARTH_API_KEY`/`HEARTH_MODEL`/`HEARTH_LLM_URL`），而 `.env`/`.env.example`/`docs/configuration.md`/service 用的是**provider 惯例名**（`AGNES_API_KEY`/`DEEPSEEK_BASE_URL`…）⇒ D-106 让 CLI **加载**了 `.env` 却没对齐**名字**，照模板配好仍报"未配置 API key"（D-105③/D-106 同族）。处置：新增 `provider_env_prefix`/`provider_env`，`resolve()` 按 provider 补惯例 env（api_key/model/url，优先级 arg > `HEARTH_*` > `<P>_*` > file，不倒退）；降级链每通道 key/url/model 同样认惯例 env；`.env.example`/`configuration.md` 如实说明。**先红后绿**回归锁 + **活体端到端冒烟**（key 仅来自 `.env` 的 `AGNES_API_KEY` ⇒ provider=agnes、答 4、exit 0）。key 只在 gitignored `.env`，未入任何被跟踪文件 |
| P1-83 | `6378dc5` | **D-116 实现 + D-118 收口（经验复用的**前置**：真教训 + 真质量 + 默认关的注入）**：① **D-118 新缺陷**——条目 `solution` 只有 `steps=N ok=<bool>`、`effectiveness` 恒为 ok?0.7:0.3 ⇒ 常量质量让"质量过滤"形同虚设（D-107 退役结构 0 指标同族），且无可行动信息 ⇒ 复用即噪声（正是 v18"全局注入害强模型 −5pt"）。修法：失败条目落 `report_failure_signature`（只读报告既有 reason/error/status/*_detail 字段，按字符截断并标注；不编造），质量由"是否捕获到信号"决定（捕获不到 = **0.0**，不按常数冒充）。② **D-116 实现**：`experience` 新增 `recent_failures`（失败专属 + 质量过滤 + **有界窗口**取最新；**刻意不做相似度匹配**——MemGate 实证纯相似度检索是信任边界）；`agent-core` 的 `experience_hint` 重接线——时机门改用**存活**的 `same_tool_repeat >= 2`（原 `consecutive_errors` 门维护者已随 B 臂删除，注入点沦为无生产者死码），注入带**来源 + 权威序**（D-80），开关 `HEARTH_EXPERIENCE_REUSE` **默认关**（构造期读一次存字段）。③ 测试：取数口三前置 / 签名只读事实+截断标注 / 门控+来源+质量下限 |
| P1-85 | （见下） | **D-116 A/B 基准验证（实测，2026-10-02）——裁决：保持默认关**。**设置**：任务 = `bench/tasks/T07-fix-logic-invert`（真实 fixture，`cargo test` 计分）；模型 = `agnes-3.0-flash`；主通道固定（agnes）；语料 = 2 条**失败**条目（同目标、`失败原因=budget_exhausted`、effectiveness 0.6，由 D-118 生产者真实产出）。**第一轮（N=2/臂）**：B 臂 `INJECTED=0/2` ⇒ 未触发失败时刻、对照无效（该模型一次修好）。**第二轮（N=5/臂，B 臂 `INJECTED=5/5`，`count=2 same_tool_repeat=2`）**：**过检率两臂同为 5/5**；步数均值 A=10.4 / B=11.2（**复用未降步数、略升**）；墙钟均值 A=59.7s / B=73.2s（B 更慢，由单个 120.6s 离群值驱动）。**裁决**：**保持 `HEARTH_EXPERIENCE_REUSE` 默认关**——未观察到收益、步数略升，与 v18"无门控注入害强模型（−5pt）"方向一致。**限制（如实）**：N=5、单任务、单模型 ⇒ 统计功效不足，结论是"**不足以支持默认开启**"，不等于"证明有害"。**后续**：语料教训偏弱（只有"预算耗尽"事实、无可行动细节）是主要限制；机制已就位，可用 D-118 已能捕获的**真实错误细节**（`verify_failed` + 编译/测试错误文本）重建语料后重测 |
| P1-84 | `7e1f2e1` | **D-119 收口 + D-120 收口（打通基准语料链）**：① **D-119**——`codex-cli` 全仓零 `ExperienceStore` 引用 ⇒ 经验写入/复用**只在 service 侧存在**，而 `bench/` 跑 CLI ⇒ D-116 的 A/B **语料产不出来**。处置：CLI 增 `experience` 依赖 + `run_local::attach_experience`，在**三处** AgentLoop 构造点接线（`HEARTH_EXPERIENCE_FILE`，默认 `<cwd>/memory/experience.jsonl`）；因构造点混有同步上下文而 `set_path` 是 async ⇒ 新增 `set_path_deferred` **延迟加载**（仅登记路径，首次 `append`/`recent_failures` 才读盘、只读一次）。② **D-120（实测新发现，两处）**——(a) `append_to_disk` 用 `if let Ok(open)` + `let _ = writeln` ⇒ 父目录不存在时**静默丢条**（实测：CLI 跑完 answer 正确、exit 0，但 `experience.jsonl` 根本没生成；`memory/` 默认不存在）⇒ 改**自动建父目录 + 逐处 warn 留痕**；(b) run 收尾的经验追加原为 `tokio::spawn` fire-and-forget，短命 CLI 进程可能在落地前退出 ⇒ 改 **awaited**。**实证**：活体探针（`agnes-3.0-flash`）指向**不存在的嵌套目录**，跑完后目录与条目均按预期生成（含 D-118 真教训字段） |
| P1-86 | （本卡） | **D-121 收口（`HEARTH_TASK_TIMEOUT_SECS=0`：「显式关闭」与实现正好相反）**：CLI 直跑的墙钟上限注释写"env 可覆盖；**0 = 显式关闭**（不建议）"，而解析是 `parse::<u64>().ok().or(Some(900))` ⇒ `0` 落成 **`Some(0)`**；`ContextManager::deadline_exceeded()` 的判据是 `run_elapsed_secs() >= cap`，**`Some(0)` 恒真** ⇒ run 在**第一步**就 `deadline_exceeded`：照注释做的用户得到的不是"不限时"，而是"立刻失败"（用户可见面 ★）。处置：把解析抽成纯函数 `parse_task_timeout_secs`，在**CLI 边界**把 `0`（含 `"00"`/空白）翻译成 `None`（不限时）+ 关闭时**打印一行风险提示**（非静默）；`None`（未设）/非法 仍回落 900，**旧语义在这两个分支零变化**。**刻意不动 `agent-core`**：那里 `Some(0)` 是**测试夹具**用来构造"起点即超时"（`loop.rs` deadline 用例），属内部用法——用户可触达的关只翻译一次。**先红后绿**：临时把该分支改回 `Some(0)` ⇒ 断言如实 `left: Some(0), right: None`。同步 `docs/configuration.md` 内核行为表补该旋钮（含 `0=不限时`），`config_doc_gate` 复跑绿；`deadline exceeded` 的收尾提示补上"（0=不限时）"可行动指引 |
| P1-87 | （本卡） | **D-122 收口（CLI 自己推荐的命令跑不起来 + `--api-key` 远程静默失效）**：**实测** `hearth chat "目标" --url http://127.0.0.1:9 --budget 1` ⇒ `error: unexpected argument '--url' found / Usage: hearth.exe chat <GOAL>`，而 CLI 在**十几处**错误提示里推荐的正是这条**后置**写法（`"下一步: hearth chat \"目标\" --url http://localhost:3000"`，见 `lib.rs:340/1018/1108/…`）⇒ 用户照抄必错，且"连远程 service"这条路径对后置写法**完全不可达**。真因＝**参数被声明两套**：`--provider/--model/--mode/--api-key` 在 `Chat` 变体里另有一份（只认后置），顶层那套未标 `global`（只认前置）。处置：顶层五个参数统一加 `global = true`（clap 语义＝前后置均认），**删掉 `Chat` 里的重复声明**（一份定义一处，`Cli::command().debug_assert()` 防复发）并删随重复而来的冗余 `--mode` 二次校验。**并修出同源第二处**：远程客户端在 `match` **之前**构造、只吃**顶层** `cli.api_key`，而 `chat --api-key` 落在子命令字段且只被喂给 `resolve()`（远程分支只取 `resolved.provider/model`）⇒ **远程模式下 `--api-key` 被静默忽略**（给了 key 仍 401）——参数归一后自动修好。**活体实证**（本地一次性 HTTP 桩）：`chat "hi" --url :38217 --api-key SECRETKEY-PROBE` ⇒ 服务端实收 `Authorization: Bearer SECRETKEY-PROBE`（sessions 与 messages 两请求均带）。**先红后绿**：临时去掉 `global = true` ⇒ 锁如实报 `UnknownArgument("--url")`。附带核实：`chat --help` 仍列出全部 global 选项（可发现性不降）；`--mode remote` 无 `--url` 后置写法同样报出可行动错误 |
| P1-88 | （本卡） | **D-123 收口（防回归门禁自身 fail-open：`severity` 取值写错即静默失守）**：xray 的"是否阻断"判据有两处（`wiring.rs::has_red_break`、`main.rs` 计数行），都写死 `== "red"`；而 spec 是**手写 TOML**，`load_spec` 只校验 `schema==1` 与"非空"，**从不校验 `severity` 取值** ⇒ 写成 `"Red"`/`"RED"`/`"rde"` 时两处比较**同时**不成立，该能力被当作"非 red = 不阻断"**静默放行**：一条 red 红线被一行 typo 关掉，输出仍照常打印 `(Red)` 与 `"0 red"`，肉眼与 CI 都看不出异常（**防线自己失守**，与 D-121 同属"声称≠实现"但在**门禁**层）。处置：`severity` 收敛为两个常量（[`SEVERITY_RED`]/[`SEVERITY_YELLOW`]，四处字面量统一）+ `load_spec` **fail-closed** 拒绝未知取值（含仅大小写不同），错误信息点名 capability 与合法取值。**先红后绿**：临时把校验条件置真（模拟修复前"不看 severity"）⇒ 锁如实 `severity="Red" 必须被 fail-closed 拒绝`。**同批证伪一条疑似项**：体检报告称"哈希锁只在 `cargo test` 生效、`codex-xray wiring` 不校验 ⇒ 改 spec 可静默过门"——核对 `.github/workflows/ci.yml` 后**不成立**（CI 同时跑 `cargo test --workspace` 与 `xray wiring` 两步，锁在 test 步已生效），故**不改**、只如实登记结论（避免为伪问题加复杂度）。spec 本身零改动（16 条、severity 仍 **13 red + 3 yellow** 的分布不变，`xray wiring` 复跑 13/16、0 red） |
| P1-89 | （本卡） | **D-124 收口（CLI 写操作：HTTP 状态码从不检查 ⇒ 失败被当成功 + `tasks done` 无条件谎报）**：三处同族缺陷。① **`client::json_capped` 只看体不看码**——而 service 的错误响应**本身就是 JSON**（`ErrorResponse` = `{"error":{"code","message"}}`）⇒ **4xx/5xx 带 JSON 错误体被当作成功响应返回**，调用方（`get_json`/`post_json` 全体）据此误判；反向地，"**200 + 空体**"（`POST /api/v1/workline/nodes/:id` 更新即返回 `StatusCode::OK` 空体）又解析失败——**两个方向都拧着**。② `tasks done` 用 `let _ = post_json(...)` 丢弃 Result **再无条件** `println!("marked done")` ⇒ 无论 404/500/空体，**一律宣告成功**（与同分支 `add/list` 用 `?` 自相矛盾）。③ `whoami`/`template`/`tools` 把错误 `println!("error: {e}")` 打 **stdout 且 exit 0** ⇒ 脚本无法用退出码判失败（与本 crate 其他子命令不一致）。处置：`json_capped` **先查状态码**（非 2xx 上抛，带状态码 + 有界错误体）；新增只认状态码、不解析体的写原语 **`post_ok`**（"写操作无回执"端点专用）；`tasks done` 改走 `post_ok` + `?`（并顺带按 D-61 口径对用户提供的 id 做 `pct_encode`，旧实现直拼进路径）；三处 stdout 错误改 `return Err(e)`（**stderr + 非零退出**，与全 crate 一致）。**先红后绿**：临时把 `json_capped` 的状态检查置假 ⇒ 锁如实 FAILED（404 的错误体被当成功体）；**活体实证三例**（本地一次性 HTTP 桩）：404 → `Error: server error 404 …` + **exit 1** 且**不再**打印 `marked done`；**200 空体 → 仍 `marked done` + exit 0**（证明 `post_ok` 不可省）；`tools` 404 → exit 1 |
| P1-90 | （本卡） | **D-125 收口（CLI 侧文件读入有界化，第 26~30 落点）**：CLI 的四个（共五处）读口一律 `std::fs::read_to_string`（**无上限**），而它们读的都是**只增**的档：`session_store::load_turns`（`<sid>.jsonl` 每轮写整份历史快照）/`load_graph_with_revision`/`load_taskgoal`/`load_run_state`（`runs/<id>.json` 含产物清单）、`lib::headless_answer`（把整份会话档读进来只为取**最后一条** assistant 文本）、`repl` 的 `/file <路径>`（把整份内容当用户消息提交）、`note::recent_human_abnormal`（扫 `human-*.jsonl`）——一个被撑大/被外部写坏的档就能让 CLI 在开工前先 OOM（与 D-51（memory 会话档）、D-70/D-86（归档）**同族**）。处置：`codex-cli` 接 `bounded-io`，新增共用读口 `read_optional_capped`（**不存在/不可读 → `None` 不炸**；**超限 → 截断 + `tracing::warn` 留痕**——CLI 启动期已装 subscriber 写 stderr，故该 warn **用户可见**，非静默）+ 两个具名上限：会话档 **64 MiB**（与 D-51 对**同一类文件**取值一致）、小块状态档/`/file`/note 档 **8 MiB**（=`MAX_CAPTURED_BYTES`）。`/file` 截断时**明确告知**只提交了前 N 字节。**刻意不"只读头部"**：`headless_answer` 要的是最后一条，头截断会静默给出**错的**答案 ⇒ 用大 cap + 留痕。**先红后绿**：把共用读口临时写回 `read_to_string` ⇒ 锁如实 `left: 4106, right: 4096`（无界）。**不误伤核实**：单测（K-4 往返 / turn 往返）走新读口全过；活体 `hearth sessions`（空目录）与 `hearth replay <不存在>` 行为不变 |
| P1-91 | （本卡） | **D-126 收口（非 Rust 侧 ember 体检：工具有界读入 + 工具名单漂移）**：`ember`（可双击使用的 Python 版小 CLI）三处同族缺陷。① **`tools.py::read` 两条路径都无界**——整份模式 `f.read()` **先整份入内存再截 64KB**（正是仓储侧 D-38/D-53 的病），分页模式 `f.readlines()` 把**全部行**读进内存（分页本就是为了避免这一点）；`edit` 是**读-改-写**、同样 `f.read()` 无上限。② **`ember.py` 三处 HTTP 无界读**——成功路径两处 `json.loads(resp.read())`、错误路径 `e.read().decode()[:300]`（先整份读再切片）⇒ 对端/中间人返回超大响应即打爆进程（与 D-55/D-58 网络侧同族）。③ **"未知工具"提示硬编码 `bash / read / write`**，而 M5 早已加入第 4 个工具 `edit`（`TOOL_IMPL` 四键、`TOOLS_SCHEMA` 也有 edit 条目）⇒ 模型一旦调错工具名，拿到的"可用工具"名单是**错的**（同 D-121/D-122 的"声称≠实现"，且这条错信息是**喂给模型**的）。处置：`read` 整份模式改**有界读**（`READ_MAX_BYTES` = 8 MiB；≤ 上限的文件输出与旧实现**逐字一致**，超限只返回开头并**明确告知**"不再声称保留尾 1/3"——那需要整份入内存）；分页模式改**流式扫描**（内存与文件大小无关）且**提前中断时不编造"共 N 行"**；`edit` 加**尺寸门**（> 8 MiB 显式拒绝 + 给 bash 替代路径，与 D-53 对 apply_patch 同款，**读-改-写不能截断**）；`ember.py` 三处改有界（成功路径超限**显式报错**而非静默截断——截断的 JSON 只会把真因盖成"解析失败"）；提示改**从 `TOOL_IMPL` 派生**（不再随工具增减漂移）；顺带订正 `tools.py` 模块 docstring（"三工具"）与 README（"三个工具"/能力一览漏 `edit`）、`read` 的 schema 边界描述补 8 MiB。**实证**（直接跑真代码）：分页输出逐字一致（`第 2-3 行（共 5 行）`）、越界 offset 报错一致、**64KB~8MB 输出与旧实现逐字一致（65661==65661 字节）**、>8MB 只返回开头且带 `[oversize]`、`>8MB edit` 显式拒绝且**文件未被改动**、`_read_body` 超限报错/正常原样/恰好等于上限不误判、两文件语法有效。**另补 D-125 的漏项（如实）**：P1-90 只 `git add` 了 `crates/codex-cli/Cargo.toml` 而**漏了 `Cargo.lock`**（新增内部依赖会在锁里加一行 `bounded-io`）⇒ 本卡一并补上，并据此在 P1-92 加 `--locked` 门禁防复发 |
| P1-92 | （本卡） | **D-127 收口（可复现性：CI 的 cargo 步骤不带 `--locked` ⇒ lockfile 漂移被静默掩盖）**：**由本会话自身的漏项引出**——P1-90 给 `codex-cli` 加内部依赖 `bounded-io` 时只 `git add` 了 `Cargo.toml` 而**漏了 `Cargo.lock`**；而本地与 CI **都不会**报错：cargo 默认**静默重写**锁文件，于是"门禁四件套全绿"掩盖了"仓库里的锁与工程已不一致"（下一个 clone 的人拿到的是被 cargo 悄悄改过的依赖图 ⇒ 可复现性受损，属"本地绿≠真绿"的又一形态，与 D-43/D-91 同族但落在**依赖图**上）。处置：CI 的 4 条构建/测试步骤（`clippy`/`test`/`xray wiring`/`xray scan`）一律加 `--locked`（`cargo fmt` 不接受该 flag，是唯一例外）——此后锁与工程不一致会在 CI **直接失败**。**防复发门禁**：`toolchain_pin_gate.rs` 增 `ci_cargo_steps_are_locked`（逐行只认**真正会执行的命令行**：`run: cargo …` 或块内以 `cargo ` 开头者；YAML 注释/`name:`/`uses:` 一律跳过——首版实现就在注释里的 "cargo test" 上误报过，已据实修掉并写进注释）。**先红后绿**：删掉 `test` 步的 `--locked` ⇒ 门禁如实报 `ci.yml:69 的 cargo test 缺少 --locked`。**机制实证**：临时给 `codex-cli` 加一个未入锁的依赖 ⇒ `cargo metadata --locked` **立即失败**（`cannot update the lock file … because --locked was passed`）；不带 `--locked` 时 cargo 转入**下载/改写锁**（本机无网故同失败，CI 有网即静默假绿）。另证锁已同步：`cargo check --locked --workspace --all-targets` = exit 0。**教训（如实登记）**：过程中我误用 `git checkout -- .github/workflows/ci.yml` 把该文件**未提交**的改动一并还原，已重新应用并复跑门禁转绿——"未提交改动上不要跑 checkout 还原" |
| P1-93 | （本卡） | **D-128 收口（`hearth setup` 会**销毁**用户的 `.env`——onboarding 链上的数据销毁级缺陷）**：**实测**——按 `.env.example` 把 `AGNES_API_KEY` / `HEARTH_PROVIDER` / `AGNES_MODEL` 三行配进 `./.env` 后跑一次 `hearth setup`（文档里的**首次上手**入口），`.env` **被截断重写成 1 行**（只剩 `CODEX_URL=http://localhost:3000`）——用户的 provider key **静默消失**，而随后 CLI 只会报"未配置 API key（通道 …）"，用户**根本无从知道是自己的 `.env` 被 setup 清空了**（与 D-106/D-117 同在 onboarding 链：D-117 刚让 CLI 认 `AGNES_*` 惯例名，setup 一转手就把它们删了）。真因＝`std::fs::write(".env", out)` **无条件截断重写**（且只写它自己那两个键）。处置：新增**唯一事实源** `config::merge_env_assignments`（纯函数、可单测）——**其余每一行逐字节保留**（注释/空行/别人的变量）；已存在同名键**就地替换**（注释行不算赋值、不追加第二份、沿用原行 `\r\n`/`\n` 行尾）；未命中才追加（原文末尾缺换行先补）。`setup` 改走合并 + **如实告知**："已更新 .env（就地改写 N 项，**保留原有其它变量 M 项**——未触碰你的 provider key）"/"已新建 .env"。**先红后绿**：临时把该函数置为修复前的"截断重写" ⇒ 锁如实 FAILED（原 3 行不再存在）。**活体实证**：3 行用户配置 + 追加 `CODEX_URL`（三行逐字保留）；**再跑一次** ⇒ 就地替换（`就地改写 1 项`）且 `^CODEX_URL=` 计数**仍为 1**（不产生第二份） |
| P1-94 | （本卡） | **D-129 收口（`--mode` 只修了一半：env/config 两个来源既不生效也不报错）**：D-102 的修复把校验接在 **`--mode` flag** 上，而它自己的注释写的是"该 flag（**与 `config set mode`、`HEARTH_MODE`**）此前只被塞进 `ResolvedConfig.mode` 后全仓无人读取"——实际只接了 flag 一路。**实测**：`HEARTH_MODE=remote` 不给 `--url` ⇒ **静默本地直跑**（用户以为连的是远程 service；本地直跑会真的落盘/跑命令，与"连远程"预期相反）；`HEARTH_MODE=bogus` ⇒ **连报错都没有**（静默忽略）。且 `resolved.mode` 全仓只被 `config get` **打印**，从不参与路由（半接线）。处置：② 抽出**唯一事实源** `Config::effective_mode`（arg > `HEARTH_MODE` > `config.toml` > `auto`）并返回**来源字符串**，`resolve()` 改用它（优先级不再各写一份——否则"被校验的有效值"与"实际生效值"又会分叉）；② 校验对象从 flag 换成有效值，报错**点名来源**（"（来源: 环境变量 HEARTH_MODE）"，用户一眼知道该去改哪）；③ **作用域刻意收窄**到真正依赖本机/远程路由的四条子命令（chat/repl/resume/replay）——若对所有子命令都拦，一旦 config.toml 存了坏值，连**修它用的** `hearth config set mode auto` 都会被挡，用户被锁死在 CLI 之外（只能手改文件）。**先红后绿**：把 `effective_mode` 临时改成"只看 flag" ⇒ 锁如实 `left: ("auto","默认值") / right: ("remote","config.toml 的 mode")`。**活体实证**：`HEARTH_MODE=remote` 无 url → 报"需要 `--url`（来源: 环境变量 HEARTH_MODE）"；`HEARTH_MODE=bogus` → 报未知取值 + 来源；`HEARTH_MODE=bogus` 下 **`config get mode` 与 `tools` 仍正常**（逃生门与非路由命令未被误伤） |
| P1-95 | （本卡） | **D-130 收口（service 启动期死物：`/tmp/codex.pid`）**：`main.rs` 启动期有一行 `let _ = std::fs::write("/tmp/codex.pid", process::id())`，注释称 **"Write PID file for Observer health checks"**——核对后**该消费者不存在**：全仓检索（observer crate / CLI / 脚本 / 文档）**无任何读取方**（运维面的 pid 文件由 `run-hearth.sh` 自己维护 `.hearth-service.pid`，是**另一个文件**）；且硬编码 `/tmp` 在 Windows 上不存在 ⇒ 这行**永远静默失败**，错误还被 `let _ =` 吞掉。属 **只写不读 + 不实注释** 的死物（同 D-45/D-66/D-83/D-85/D-111② 口径）。处置＝**删除**并原位留注（写清"曾存在 / 为何删 / 真要恢复需先做什么"）；顺带**如实标注现役部分**——`instance_id` **是活的**（经 `GET /api/v1/resources` 与 `hearth whoami` 对外暴露），故只删 pid 那行、不动它。**同批核验两条相邻声称（结论：均为真，不动）**：① `ALLOW_NO_AUTH=1` 是否真"只绑回环"——`main.rs` 的 `bind_ip` 在无鉴权模式下确实取 `[127,0,0,1]`（且 `allow_no_auth` 在 `routes.rs:116` 被真实读取）；② `observer/daily-*.jsonl` 的"无消费者"——核实**早已登记为待顶层裁的历史债**（`docs/design-nightwatch-autonomic-layer.md:75`、`top-level-plan-v23.md:302`、`requirements-ledger.md:176`），**不属本次可单方处置范围**，故原样保留、只登记结论。**新增退役门禁** `service/tests/retired_pid_file_gate.rs`（逐行剥 `//` 注释后扫"往 `*.pid` 写文件"的形态；文件头自报盲区与"恢复前须先接线消费者 + 可移植路径 + 留痕"的恢复条件）。**先红后绿**：把该行加回 ⇒ 门禁如实报 `main.rs:667 已退役的 PID 文件写入回归了` |
| P1-96 | `6948d83` `fdd7b31` | **D-131 收口（无界读入"确有边界者"收口；含一次口径纠偏）**：先按"D-33/D-51/D-70/D-75/D-86/D-125 未覆盖的剩余裸读"清点，**首版（`6948d83`）把 8 处一次性有界化**（含 service 启动期 `config.toml` 白名单与模板清单、tool-runtime 工具 manifest），随后**回查债队列发现其中三处正是 D-77（2026-10-01 顶层裁决）明令「不改」**——它们读的是**操作者自己的本机配置/清单**（非不可信来源、无无界增长特性），加 cap 无安全收益、反有"合法大文件被截断 → 解析失败 → 静默降级"的回归风险。**已裁决过的就不要改** ⇒ 第二提交（`fdd7b31`）**按 D-77 全量回退** service/tool-runtime 四处 + 新增的 `bounded-io` 依赖（`Cargo.lock` 同步回退，`cargo metadata --locked` 复核通过），并把同属该类的 CLI `config.toml` 读、快照 meta 读也**改为不加 cap**、只加 `bounded-io-exempt:` 标记钉住理由。**保留三处确有边界者**：① `setup` 读 `.env`——**兼修静默清空**：旧 `read_to_string(..).unwrap_or_default()` 在读失败（权限/非 UTF-8）时退化成空串 ⇒ `merge_env_assignments("")` + `fs::write` 把用户既有 `.env` **覆盖成 1~2 行**（＝ D-128 修掉的数据丢失换触发面）；现读**有界** + 任何失败/超限**拒绝改写**并留痕；② `coverage` 读 `tarpaulin-report.json`（含**逐行**覆盖明细、随仓库规模增长，大仓数十 MB）；③ `/cost` 读会话累积报告 `.md`。**门禁**：`codex-cli/tests/unbounded_read_gate.rs`——扫 `crates/codex-cli/src`，非测试代码出现裸 `read_to_string(`/`fs::read(` 即红，除非该行或**前 3 行内**带 `bounded-io-exempt:` 标记；`#[cfg(test)]` 模块豁免（大括号深度跟踪）。**范围刻意只到 codex-cli**（service/tool-runtime 在 D-77 裁决内）。**先红后绿**：注入一行裸读 ⇒ 门禁列出命中并失败，移除后转绿。门禁四件套全 0；xray 基线不变（13/16、0 red）。**教训（如实登记）**：动手前须先回查债队列的"已裁决不改"项——本卡首版即因漏查而越权，靠第二提交纠正 |
| P1-97 | `bd70f68` | **D-132 收口（门禁尺子 fail-open）+ D-133 收口（不实注释）**：承 D-123/D-127"防线自己失守"同族，本轮把尺子对准**另一把尺子**——`window-framework/gate_window.py`（9→10 套件的常驻回归门禁）。**D-132**：其 `run_suite` 只看**解析出的数字**，退出码仅在"一个数字都解析不出"时才参与（`if rc != 0 and passed == 0 and failed == 0: failed = 1`）⇒ 「套件先打印 `=== RESULT: N passed, 0 failed ===` 再崩/再失败（退出码非零）」被判 **PASS**；而每个套件末行恰是 `sys.exit(1 if failed else 0)`——**退出码才是权威信号**，旧实现把它丢了。另：`re.search(r"(\d+)\s+passed", out)` 取**第一处**匹配，汇总行之前的 "N passed" 字样会被当总数。处置：抽纯函数 **`parse_suite_result(returncode, out)`**——① **非零退出码一律计 FAIL**（即使汇总行写 `failed=0`）并点名退出码；② 正则**锚定 `RESULT:` 汇总行**；③ 无汇总行 ⇒ FAIL；④ **零断言 ⇒ FAIL**（vacuous green，同 xray「空链视为断裂」口径，堵住"把套件清空成 exit 0 就骗绿"）。**防复发**：新增**尺子自检套件** `tests/test_gate_window.py`（13 断言）并**纳入 SUITES**（门禁 208→**221** passed）。**双重红侧实证**：① 照抄旧逻辑跑主案 ⇒ 打印 `mark=PASS`（fail-open 复现）；② 端到端注入"打印 5 passed 后 `sys.exit(1)`"的探针套件 ⇒ 新门禁如实 `[FAIL] probe: 5 passed / 1 failed` + 备注 `退出码 1（非零即失败，不采信 failed=0）` + `GATE_RESULT: FAIL` + exit 1。**历史验收盲区（如实）**：`acceptance-gatekeeper-p2.md` 只验证过"失败套件→exit 1 / 缺失套件→exit 1"两种红，第三种红（本卡）此前无人验。**D-133**：`framework.py` 头注称"snapshot/rollback/conflict/budget/export/import/template 在 --help 占位"——实测**除 `budget` 外**这些命令早已实现并注册进 `main()`（`window snapshot/rollback/snapshot-list/status/compress/export/import/analyze/run/resume` + `template`/`conflict`/`workflow`）；`FrameworkCheck` docstring 称"4 条断言"而 `run()` 实为 **5 条**（多 `stage-role-unique`）⇒ 均据代码订正（同 D-45/D-54/D-111④ 口径）。本轮仅动 Python + 文档；Rust 门禁四件套复跑仍全 0；window-framework 门禁 **221/221 PASS** |
| P1-98 | `06f6e52` | **D-134 收口（用户可见面静默失败留痕）**：同一族病灶——`let _ = <fallible>()` **丢弃 `Result`**（声称≠实现 / 静默降级），本轮命中两处**用户可直接感知**的面。① **`codex-cli/src/repl.rs` REPL 审批提交三处**（`approve`/`deny`/超时 auto-deny）此前 `let _ = client_arc.submit_approval(..)`：提交失败（service 掉线 / 404 / **非 2xx 空体**）时用户**看不到任何提示**，误以为已批准，而会话其实仍在等服务端决策（随后卡住/超时）⇒ 改为失败 `render::error` 留痕 + 提示"会话可能仍在等待该审批——请检查 service 是否在线后重试"，且**不中断 REPL**（一次网络动作不该把交互会话杀掉）。② **`codex-cli/src/run_local.rs` `let _ = crate::note::persist_ai_events(..)`**：落盘失败（`~/hearth/observer` 建不出 / 写不进 / 磁盘满）时 Observer 素材**静默丢失**、任务照常 exit 0，用户与观测面皆无痕迹（同 D-120「静默丢条」族）⇒ 改为失败 `render::error` 留痕（含"不阻断本次任务"），与紧邻 transcript 落盘同款处置（`persist_ai_events` 内部已 `context(..)` 带失败原因，此处只需呈现）。**取舍**：两处均只"留痕不阻断"——审批/落盘属可恢复的外部动作，不让交互会话/任务主流程陪葬，留痕即守住"不静默降级"红线。门禁四件套全 0（fmt=0 / clippy=0 / test --workspace=0 / sandbox linux check=0） |
| P1-99 | `1a1f9e9` | **D-135 收口（交互提交路径"谎报成功 + 静默丢条"）**：承 D-134 同族，本轮扫 `run_local.rs` `render_agent_event` 内的三处 `resolve_interaction` 调用，见**同一函数内三种写法、两类缺陷**。① **澄清（clarification）分支**：`let resolved = dispatcher.resolve_interaction(..).await.is_ok();` 先转 bool，随后 **`let _ = resolved;`** 丢弃，却**无条件** `render::info("  ✓ 澄清已提交（…）")`——属**写操作谎报成功**（D-124 红线）。取证确认该函数**会真实返回 Err**（`dispatcher.rs` 的 `resolve_interaction`：`interaction_id mismatch` / `no pending interaction`（重复提交/已解决）/ `no interaction state for session`），故谎报有真实触发面：提交失败时内核未收下澄清，用户却以为已生效。修复＝改为 **`.context("澄清提交失败")?`**——**对齐紧邻审批分支**（钉子：该分支早已是 `.context("审批提交失败")?` 的 fail-closed 语义，同一函数内两分支行为本不该拧着），失败即留痕并经调用点（`run_local.rs:657`，`?` 传播）中断。② **两处 headless 放弃/拒绝分支**（clarification headless guard + approval noninteractive deny）：`let _ = dispatcher.resolve_interaction(..)` **静默丢弃失败**（D-120「静默丢条」族）⇒ 改为失败 `render::error` 留痕、**保持不阻断**（headless 放弃/拒绝是既定意图，不该因一次提交失败把整轮 headless run 杀掉）。**边界判断**：三处同卡——同一函数、同一 API、同一失败族；①取 fail-closed（用户正盯着"是否已生效"）、②取留痕不阻断（无人可告警的既定放弃），处置分寸按"用户是否在等回执"分层。门禁四件套全 0（fmt=0 / clippy=0 / test --workspace=0 / sandbox linux check=0） |

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
| **D-74** | **`agent-runtime` 会话事件缓冲无界增长**：`Session.events` 注释称 "ring"，实为无上限 `Vec`（14 push / 0 clear）；且它是**断线续传重放源**，不能简单裁剪（重放侧按位置重新编号，一旦截断会让落后客户端**静默收到 0 条**） | agent-runtime 体检 | **已修**（P1-65/D-101）：上限 `MAX_SESSION_EVENTS = 50_000` + **批量**丢弃（摊还 O(1)，避免逐条 `remove(0)`）→ 收口到唯一入队口 `Session::push_event`；重放侧引入**绝对 seq 基址** `events_base_seq`（未截断=0 ⇒ 与历史版本**逐字节一致**；截断后客户端按 Last-Event-ID 过滤仍正确，**看到 id 跳变**而非静默丢失）。回归锁断言"绝对编号不丢不重"（`base + len == 总 push 数`）。注释订正见 P1-40，增长处置见 P1-65 | **close** |
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
| **D-89** | **面向用户的配置参考有 8 个幽灵旋钮**：`docs/configuration.md` 列出的 `RETRIEVER_ENABLED` / `LSP_ENABLED` / `EMBED_API_KEY` / `EMBED_BASE_URL` / `EMBED_MODEL` / `CODEX_SANDBOX_ENABLED` / `CODEX_SANDBOX_WORKSPACE` / `SESSION_TTL_SECS` **全仓零读取**——用户照文档设置**毫无效果**（前两个能力已随 P1-04 整删、EMBED 随 v21.0 移除、沙箱两项**从未实现**、`SESSION_TTL_SECS` 是 `OOM_TTL_SECS` 的**错名**）；且文档标题仍写 "Codex-Rust v4.0" | 环境变量"文档承诺 vs 代码读取"配对扫描 | **已修**（P1-53）：据实重写 + 补录两个真实旋钮 + 《已失效》节 + **新增门禁** `config_doc_gate.rs`（**反引号 = 现役旋钮** 的契约，防整类复发）；**沙箱开关显式决定不实现**（生产不该提供一键关沙箱，fail-closed 是设计）。先红后绿 | **close** |
| **D-90** | **`/openapi.json` 是一份"零端点的空契约"**：`ApiDoc` 的 `#[openapi(...)]` 里只有 `components(schemas(...))`（注释自称 "path annotations deferred"）⇒ 该端点声明 **0 个端点**，而服务实际注册 **27 条**路由；`/swagger-ui`（早已 vendor 的交互 UI）读的正是它 ⇒ 打开即空。另：该 handler 用 `.expect(...)` 在**请求路径里 panic**（D-60 同族） | "对外契约（HTTP 路由）vs 代码"审计 | **已修**（P1-54）：**接线而非退役**（同 D-48/D-79 口径——端点与消费者都活着、缺的是内容）——新增 `service/src/openapi.rs`（`API_ROUTES` 27 条镜像表 + 运行时注入 paths）+ panic→500 留痕 + 订正"UI 可日后再 vendor"过期注释；**新增漂移门禁** `openapi_route_gate.rs`（双向比对 + paths 条数断言，专治"零 paths 也绿"）。先红后绿（红检钩子已移除） | **close** |
| **D-91** | **CI 工具链漂移 ⇒"本地绿 / CI 红 / 本地不可复现"**：CI 用 `dtolnay/rust-toolchain@stable`、仓库无钉版文件；stable 于 2026-09-28 由 1.98 滚到 **1.99**，其新增 `clippy::double_must_use` 命中工作区**每一个** `#[async_trait]`+`Result` 的 trait（`subconscious`/`tool-runtime`/`sandbox`/`memory`/`llm-gateway` 全线）——**宏产物误报**，本地 1.98.1 完全看不到。**本项目第二次 CI/本地口径分裂**（首次 D-43 为 flags 口径） | D-90 推送后 CI 35 秒即红（报错落在未改动文件） | **已修**（P1-55）：**钉版**（`rust-toolchain.toml` + CI 显式 `toolchain`）+ **新增门禁** `toolchain_pin_gate.rs`（防退回浮动通道 / 防两处版本不一致）；**明确决定"不撒 allow"**（5+ crate 的宏产物误报，钉版优于散豁免），升级障碍与处置建议已写入文件头。CI 恢复绿 | **close** |
| **D-92** | **`GET /api/v1/tool-registry` 是"契约承诺但从未挂载"的端点**：`routes::registry_list` 实现完整（`state.tool_registry.list()`）、`tool_runtime::registry` 模块头也把它与已接线的 `search`/`install` 并列为三件套，但 `main.rs` 的 `.route(...)` 里**没有它** ⇒ 全仓零调用方、用户按文档调用必 404；`ToolRegistry::list()` 因之零生产调用方（`TOOLS_DIR` 自动发现的清单只余 `search?q=` 一条野路可查） | "对外契约（HTTP 路由）vs 代码"审计（承 P1-53/P1-54） | **已修**（P1-56）：**接线而非退役**（端点+handler+契约文档三者在位、缺的只是挂载，同 D-48/D-79/D-90 口径）；同步 `API_ROUTES` + 订正头注计数 27→28。**先红后绿**（漂移门禁精确报"路由有、文档无"） | **close** |
| **D-95** | **providers.json"模型自动发现"只打日志、解析即丢**：写默认清单 → 读回 → `tracing::info!("model auto-discovered")` → **丢弃**；发现的模型既不注册进 registry 也不影响任何路由。且默认清单含 `{"provider":"anthropic"}`，而本仓**无 anthropic 实现** ⇒ "发现即注册"按设计不成立 | 承 P1-58 的 provider 面体检（`global-panorama-v11.5.md:39` 早有如实记录） | **已退役**（P1-59）：删整块 + `PROVIDERS_PATH` 记入《已失效》。**判定为"退役"而非"接线"**：给一个不会工作、且与 env 注册完全重复的机制补 provider 工厂＝过度设计 | **close** |
| **D-96** | **六个 provider "无条件注册空 key" ⇒ 可选但不能用**：agnes/zhipu/gemini/deepseek(+pro)/zhipu-max 用 `unwrap_or_default()` 注册，未配 key 时 provider 存在（`/api/v1/models` 列出）但任何调用 401；开 `FALLBACK_CHAIN` 时还会被组装进链**在中途失败**（而非被跳过） | 承 P1-58 的 provider 面体检 | **已修**（P1-60）：统一收紧为**旋钮存在且非空才注册**（含 hunyuan/doubao 的空串、ollama/vllm 的 BASE_URL 空串），与 P0-1/D2/D4 的"不静默降级"同一条红线。**有意行为变更**：`/models` 只列真能用的 provider | **close** |
| **D-97** | **fallback 链主通道顺序不确定**：`main.rs` 用 `registry.list()`（**HashMap 键序**）组装 `FallbackChain`，而该类语义是"第一个 = 主通道、其后按序降级"（fallback.rs 头注 + S9 通道切换投影都依赖此序）⇒ "谁是主 provider"在运行期**随机**，`[fallback] a → b` 投影随之飘、同尺基准不可比 | 承 P1-60（读 `FALLBACK_CHAIN` 分支时发现） | **已修**（P1-61）：新增列表写法 `FALLBACK_CHAIN=a,b,c`（按给定顺序，a 即主通道；未注册名 warn 跳过）；`1`/`true` 保留为"全部已注册 + 名字排序"兼容写法；空链 warn 且不注册；日志补 `primary` | **close** |
| **D-98** | **两个上下文结构体里"只写不读"的字段**：`subconscious::GuardContext` 的 `goal_text`/`step_count`/`last_success`/`constitution_summary`（agent-core 每步填好、守卫从不读）；`tool_runtime::ToolContext.deadline_clamped`（唯一写入点是 dispatcher、零读取方——dispatcher 用的是**同名局部变量**）+ 零调用方的 `remaining_task_time()` | 未体检 crate 体检（subconscious / tool-runtime） | **已修**（P1-62）：全部删除（含 5 处测试构造同步收窄）；两处均留说明"若日后要成为真守卫/真标记，需先设计规则再收回字段" | **close** |
| **D-99** | **observer `clarify_skip_rate` 恒 0**：metrics 汇总 `NeedApproval` 时把 kind **硬编码为 `"approval"`**，而 WP-0 契约把 kind 放在 `NeedApproval.action`（`action=kind`）⇒ `kind == "clarification"` 分支恒不进入 ⇒ `clarify_req`/`clarify_skip` 恒 0，而该指标被 observer 报告与规则引擎真实消费（对外长期报结构性 0） | 未体检 crate 体检（observer） | **已修**（P1-63）：改为 `req_kind.insert(id, action.clone())`；**先红后绿**回归锁（红侧 `left: 0.0, right: 1.0`）；顺带去掉无意义的 `if true {…}` | **close** |
| **D-102** | **CLI 两处"选项被静默忽略"**：① 远程 `chat`/`repl` 把 provider **硬编码 "deepseek"** 且不传 model ⇒ `--provider/--model` 在远程模式失效；② `--mode` 全仓无人读取 ⇒ `--mode remote` 不给 `--url` 时仍本地直跑，未知值被吞 | CLI 用户可见面审计 | **已修**（P1-66）：`create_session` 增可选 model + 远程改走三级解析 + `validate_mode()` 给 `--mode` 唯一语义（含回归锁） | **close** |
| **D-103** | **bridge 多数票把反对计成赞成**：`contains("agree")` 对 `"disagree"` 也为真（子串）⇒ 反对票全被计入赞成，结论可能反向；且无关键词的回复（含 content 缺失）静默计反对 | provider/bridge 面审计 | **已修**（P1-67）：抽 `classify_vote()` 先判否定词；`None` ⇒ 保守计反对 + warn；**先红后绿** | **close** |
| **D-104** | **`llm-openai` 出站限流只覆盖 `chat()`**：`RateGate` 声称进程级限流（为突发 429 设计），但 `stream()`/`embed()` 绕过，而 agent-core **主回合走 `stream()`** ⇒ 声称的防护在主要路径失效 | provider crate 审计 | **已修**（P1-68）：三条出口统一取槽 + 文档订正为如实覆盖率 | **close** |
| **D-105** | **CLI 帮助/实现三处不一致**：① `config get model`/`read-roots` 报"未知字段"（set 却支持）⇒ 能设不能读；② `note` 帮助写 `[--observer-verdict n "反审"]`（两 token）但参数只收一值，且判定是 `verdict == "n"` 精确等值 ⇒ 帮助所写用法**永不成立**；③ `setup` 写 `.env` 但 CLI **从不读**（仅 service 加载 dotenv）⇒ "配好了却不生效" | CLI 用户可见面审计 | ①② **已修**（P1-69）；③ **登记待办**：`.env` 需在 CLI 侧接线（或改为不再写 .env）——属行为面，单独立卡 | **close（①②）· ③ 登记** |
| **D-106** | **CLI 从不加载 `.env`**：`hearth setup` 写 `./.env`、仓库有 `.env.example`（明写是 **CLI/Service** 模板），但只有 service 调 `dotenvy::dotenv()` ⇒ 上手路径对 CLI 是死的（配好 key 仍报"未配置"） | CLI 用户可见面审计（承 D-105③） | **已修**（P1-70）：CLI 接线 `.env` 加载（`.env` 已 gitignore；dotenvy 不覆盖进程 env，优先级不变）；`.env.example` 与 `docs/configuration.md` 同步订正 | **close** |
| **D-107** | **experience"复用"接口退役**（D-100 裁决落地）：`reference_count`（无写入方）/`reuse_rate`（对外暴露的结构性 0）/`upgrade_core()`（恒空，调用点只把 0 打进日志） | D-100 裁决（**联网核实** Reflexion + OEP 后，非拍脑袋） | **已退役**（P1-71）：三项删除、经验库定位为**只写审计档**；xray 锚点收窄 + FNV 复算（条数仍 16 / severity 仍 red）。**真正的复用**列为设计课题：须满足①失败专属②质量过滤+有界窗口③来源标注（D-80 纪律）并先过基准验证 | **close（接口）· 复用课题另立** |
| **D-108** | **文明线公告写进"无人读取的档"**：读接口读 `per_user.civ_for(uid)`（唯一可见 store），写侧写**全局**档（另一文件）⇒ 公告在 API 上不可见（D-48 重接线的可见面未真正生效）；另 `AppState.civ_store`/`workline_store` 两个只写不读字段 | service 内部模块审计 | **已修**（P1-72）：写侧对齐读侧 + warn 留痕 + 清死字段 + **新增门禁**（先红后绿）。**刻意不采用"读侧合并全局档"**（跨租户泄露） | **close** |
| **D-111** | **service 内部批次**：① `install_tool` 用 `ERR_INTERNAL`+400（码与状态自相矛盾）；② `open_artifact`/`open_external` 的"路径非法"与"会话不存在"混用 `ERR_SESSION_NOT_FOUND`+400；③ `templates.rs` 两层 `if let Ok` 静默丢弃；④ `api` 四个零消费者错误码常量；⑤ `service::user::UserContext` 零构造零读取（doc 却称"每请求注入"）；⑥ `TemplateManager::get` 零调用；⑦ `webhook.rs` 注释称 per-user 实为裸 `Vec`；⑧ `AppState.observer` 只写不读 | service 内部模块审计 | **已全部收口**（P1-76 ①②③⑦；P1-77 ⑤⑥⑧ + ④ 改为**如实留痕保留**〔对外错误码词表，删掉＝单方缩小契约面〕） | **close** |
| **D-109** | **agent-loop 的 civ 写入（`CivWriterAdapter`）缺 `session → 归属用户` 链**：它仍写全局档 ⇒ 自动写入的 milestone/reflection 在 API 上不可见；修它需给 `Session` 加 owner（现在没有）并打通 `create_session`，属**需先设计的接线** | P1-72 连带发现（同一病灶另一半） | **已修**（P1-81，**先设计后接线**）：写入器**工厂**按 owner 构造（`set_civ_writer_factory` + `create_session_with_owner`），适配器经 `per_user.civ_for(owner)` 落档；全局 `civilization.jsonl` store 停用；xray 锚点复算 + 门禁（先红后绿）+ 行为锁。**未采用**"读侧合并全局档"（跨租户泄露） | **close** |
| **D-113** | **误跟踪的解释器字节码缓存**：`.gitignore` 只窄忽略 `window-framework/src/__pycache__/` ⇒ `bench/`、`docs/data/memory-context-20260830/`、`ember/` 三处的 `__pycache__/*.pyc`（共 4 文件）被误跟踪——CPython 自动生成的字节码，非源码、可随时重建 | 入库卫生第 3 轮（承 D-5/D-112） | **已修**（P1-78）：`git rm --cached` 4 文件（内容留盘）+ 窄规则改全局 `__pycache__/`/`*.pyc`/`*.pyo` + 门禁 `gitignore_runtime_products_gate.rs` 增 `interpreter_caches_are_gitignored`（先红后绿） | **close** |
| **D-114** | **agent-core「失败分类 / 恢复策略」决策支持层被结构性架空**：`terminal.rs` 的 `classify_failure`/`FailureKind`、`failure_strategy`/`RecoveryStrategy`、`strategy_suggestion`、`completion_readiness`/`CompletionReadiness` **全仓零消费者**（仅自测）；其唯一下游通道（scratch 键 `last_failure_class`/`last_recovery_strategy` 的"R5-1 中转注入块"）已被 R6-5 依"判定权归还范式"主动删除、Reflect 相位分类块随线C手术（D-10）消失 ⇒ loop.rs 仍**读**这两键却**无写入方**（恒 null，且投影进 run report/handover） | agent-core 逐文件体检（terminal/context/scheduler/talent/constitution） | **已退役删除**（P1-79，同 D-66/D-83/D-85 口径——生产者结构性消失且 R6-5 明载"勿复活旧通道"，故**不重新接线**）：删四组纯函数 + 上下游恒空字段 + 快照白名单两键 + 订正 3 处过期注释；保留现役 `normalize_terminal_state` 与 G1 九态封闭集；**新增门禁** `retired_failure_channel_gate.rs`（先红后绿）钉住退役 | **close** |
| **D-110** | **README/命名文档与代码不符**（用户第一触点）：`sh install.sh`（实际在 `bench/install.sh`）、seccomp 106↔136、cgroup 数值、`--observer-verdict` 两 token 示例必报错、`hearth chat` 示例缺 goal、版本示例过期、徽章文案、`hearth-rs` 非可执行名 | README/文档审计 | **已修**（P1-74）：逐项据代码订正 + **新增 README 路径门禁**（先红后绿）。**未覆盖**：文档里数值/行为描述的持续一致性（需逐项人工核对，本轮已手工订正 seccomp/cgroup 两处） | **close** |
| **D-77** | **低危无界读入（配置/manifest/replay，均未走共享原语）**：`service/main.rs:411`(config.toml)/`:459`(providers.json)、`service/templates.rs:31`、`tool-runtime/registry.rs:100`(manifest.toml)、`llm-gateway/cost.rs:276`(`HEARTH_PRICE_FILE`)、`llm-replay/lib.rs:84`(replay 夹具) | 本轮体检归纳 | **裁决「不改」（by design，2026-10-01）**——理由：这六处读的是**操作者自己的本机文件**（配置/清单/价表/replay 夹具），既不来自不可信来源、也**不具备无界增长特性**（对比 D-70 的只增日志、D-75 的 cwd 项目文件、D-55/D-58 的网络响应）。给它们加上限**不会带来任何安全收益**，却有**真实的回归风险**：合法的超大 `providers.json`/夹具一旦被截断 → JSON 解析失败 → provider 发现/夹具加载**静默降级**（比"读全"更糟）。故**不改**；"全仓文件读只剩 `bounded-io` 一份"的收敛目标限于**确有边界的读**，不为此把每处配置读都套上 cap（2026-10-03 追记：P1-96 首版曾误将其中 service `config.toml`/模板清单与 tool-runtime manifest 三处一并加 cap，**已按本裁决全量回退**；同类的 CLI `config.toml` 读与快照 meta 读亦改为不改，并以 `bounded-io-exempt:` 标记把裁决理由钉在代码处 + `unbounded_read_gate`〔只扫 codex-cli〕防再越权） | **close · 裁决「不改」** |
| **D-132** | **`window-framework/gate_window.py` 门禁 **fail-open**（尺子自己失守）**：`run_suite` 只看**解析出的数字**，退出码仅在"一个数字都解析不出"时才参与（`if rc != 0 and passed == 0 and failed == 0`）⇒ 「套件先打印 `=== RESULT: N passed, 0 failed ===` 再崩/再失败（退出码非零）」被判 **PASS**；而每个套件末行恰是 `sys.exit(1 if failed else 0)`——**退出码才是权威信号**。另 `re.search(r"(\d+)\s+passed", out)` 取**第一处**匹配，汇总行之前的 "N passed" 字样会被当总数。**历史验收盲区**：`acceptance-gatekeeper-p2.md` 只验过"失败套件→exit 1 / 缺失套件→exit 1"两种红，第三种无人验 | 尺子自身体检（承 D-123/D-127 同族"防线失守"） | **已修**（P1-97）：抽纯函数 `parse_suite_result`——非零退出码**一律** FAIL（即使 `failed=0`）+ 正则锚定 `RESULT:` 行 + 无汇总行 FAIL + **零断言 FAIL**（vacuous green，同 xray「空链视为断裂」）；新增**尺子自检套件** `tests/test_gate_window.py`（13 断言）并纳入 SUITES（门禁 208→**221**）。**双重红侧实证**：旧逻辑跑主案打印 `mark=PASS`；端到端注入"打印 5 passed 后 `sys.exit(1)`"探针 ⇒ 门禁如实 FAIL + exit 1 | **close** |
| **D-133** | **`window-framework/src/framework.py` 不实注释**：头注称"snapshot/rollback/conflict/budget/export/import/template 在 `--help` 占位"——实测**除 `budget` 外**这些命令早已实现并注册进 `main()`（`window snapshot/rollback/snapshot-list/status/compress/export/import/analyze/run/resume` + `template`/`conflict`/`workflow`）；`FrameworkCheck` docstring 称"**4 条断言**"而 `run()` 实为 **5 条**（多 `stage-role-unique`，WT19 audit-fix 加的） | 同卡体检 | **已订正**（P1-97）：两处据代码订正（同 D-45/D-54/D-111④ 口径） | **close** |
| **D-134** | **用户可见面静默失败（`let _ =` 丢弃 `Result` 族）**：① `codex-cli/src/repl.rs` REPL 审批提交三处（`approve`/`deny`/超时 auto-deny）`let _ = client_arc.submit_approval(..)`——提交失败（service 掉线 / 404 / 非 2xx 空体）时用户**看不到提示**，误以为已批准而会话仍在等服务端决策（卡住/超时）；② `codex-cli/src/run_local.rs` `let _ = crate::note::persist_ai_events(..)`——落盘失败时 Observer 素材**静默丢失**、任务照常 exit 0 | 用户可见面体检（承 D-120「静默丢条」/「不静默降级」红线） | **已修**（P1-98）：两处均改失败 `render::error` 留痕、**不阻断主流程**（审批/落盘属可恢复的外部动作，不让交互会话/任务陪葬） | **close** |
| **D-135** | **交互提交路径"谎报成功 + 静默丢条"**（D-134 同族）：`codex-cli/src/run_local.rs` 的 `render_agent_event` 内三处 `resolve_interaction` 写法不一——① **澄清分支** `.is_ok()` 转 bool 后 `let _ = resolved;` 丢弃、却**无条件**打印「✓ 澄清已提交」，而 `dispatcher.resolve_interaction` 在 `interaction_id mismatch` / `no pending interaction` / `no interaction state for session` 时**真实返回 Err** ⇒ **写操作谎报成功**（D-124 红线）；② 两处 **headless 放弃/拒绝分支** `let _ = resolve_interaction(..)` **静默丢弃失败**（D-120 族）；对照紧邻审批分支早已是 `.context("审批提交失败")?` 的 fail-closed 语义 | `let _ =` 族全网复扫（承 D-134 同族） | **已修**（P1-99）：① 澄清分支改 `.context("澄清提交失败")?`（fail-closed，对齐审批分支）；② 两处 headless 分支改失败 `render::error` 留痕、**不阻断**（放弃/拒绝为既定意图） | **close** |
| **D-78** | **零调用方小项批次**：`service::sse.rs:51 sse_stream`（routes 只用 `_with_replay`）、`llm-gateway::types::PropertySchema`（仅自引用）、`llm-gateway::fallback.rs:94` 固有 `stream()` 与 trait 实现**函数体重复**；`CostMeter` 的 `total_prompt_tokens`/`total_completion_tokens`/`entry_count`/`iter` 仅自测调用（删除需同删其断言） | 本轮体检归纳 | **已全部收口**：P1-42 删 `sse_stream` + `PropertySchema`；P1-80 收口余项——**确认无生产调用方**（仅单测）后删 `fallback` 固有 `stream()`（与 trait 实现逐字重复，是漂移陷阱）；`CostMeter` 四访问器**裁决保留**（其断言承担 `record()` 覆盖） | **close** |
| **D-116** | **经验"复用"课题（设计已定，待基准验证）**：D-100/D-107 已退役无生产者的复用接口；真正的复用＝**失败时**把过往经验有界注入。**设计**（依 2026-10-02 联网核实，非拍脑袋）——①**失败专属**：仅在出现失败事实（连续错误/失败核验）时检索，**不做**无条件相似度检索；②**质量过滤 + 有界窗口**：只取 `success=false` 且 `effectiveness` 达阈的条目，窗口硬上限 N（防 prompt 膨胀——本仓已有"常驻注入致 prompt 基底 +23.6%"的定量教训）；③**来源标注**：注入文本显式标注"来源=历史经验档（非本次事实）"，与 D-80 同纪律。机制取向对齐业界：把"相似度检索"升级为**任务条件化准入**（MemGate 主张：相似度检索是信任边界，会引入跨域泄漏/漂移），并做**写路径过滤**。④**时效防护**（STALE 提示：过期记忆有害 ⇒ 需时间戳 + 仅失败族限定）。**基准验证方案**：取 `bench/tasks` 中"改错/修复"类（如 T07-fix-logic-invert、T13-fix-index）做 A/B（with-reuse vs without），指标=首次修复成功率/步数/token 成本，主通道固定（D-97 保证同尺）。**当前阻断**：本机 `.env` 的 key 被服务端 401 拒（无法跑活体基准）⇒ **只登记设计、不接线**（口径：先过基准验证） | 承 D-107「复用课题另立」+ 2026-10-02 联网核实 | **已收口（P1-85）**：实现（P1-83）三前置 + 默认关开关 + 来源标注；**A/B 已实跑**（T07 fixture、`agnes-3.0-flash`、N=5/臂、B 臂注入 5/5）——**过检率两臂同为 5/5，步数 A=10.4 / B=11.2（未降反略升）** ⇒ **裁决保持默认关**（与 v18 一致）。限制：N=5/单任务/单模型、语料教训偏弱（仅"预算耗尽"事实）⇒ 结论"不足以支持默认开启"；后续可用 D-118 捕获的真实错误细节重测 | **close（裁决：默认关）** |
| **D-117** | **CLI 不认 `.env` 里的 provider 惯例 env 名**：`.env`/`.env.example`/`configuration.md`/service 用 `AGNES_API_KEY`/`DEEPSEEK_BASE_URL`…，CLI 只认 `HEARTH_*` ⇒ 照模板配好仍报"未配置 API key"（D-106 只对齐了"加载"，漏了"名字"）。附带**结论订正**：早前"key 被服务端 401 拒"**不成立**（直接验证 HTTP 200） | 用户提供可用 key 后实测发现 | **已修**（P1-82）：`resolve()` 按 provider 补惯例 env（含降级链每通道）；优先级不倒退；**先红后绿** + **活体端到端冒烟**（`AGNES_API_KEY` 驱动 CLI 跑通） | **close** |
| **D-118** | **经验条目不是"教训"、质量是结构常量**：`solution` 只有 `steps=N ok=<bool>`；`effectiveness` 恒为 ok?0.7:0.3 ⇒ ① 常量质量让"质量过滤"形同虚设（D-107 退役结构 0 指标同族）；② 无可行动信息 ⇒ 复用即噪声（v18：全局注入害强模型 −5pt） | P1-83 取证（D-116 实现前置） | **已修**（P1-83）：失败条目落 `report_failure_signature`（只读既有事实字段、按字符截断并标注、不编造）；质量由"是否捕获到信号"决定（捕获不到 = 0.0） | **close** |
| **D-119** | **CLI 侧根本没有经验库**：`codex-cli` 全仓零 `ExperienceStore` 引用 ⇒ 经验写入/复用只在 service 侧存在；而 `bench/` 跑的是 **CLI** ⇒ **D-116 的 A/B 语料产不出来**（这是当前唯一阻塞） | P1-83 取证（准备 A/B 时发现） | **已修**（P1-84）：CLI 增 `experience` 依赖 + `attach_experience` 接三处构造点；`set_path_deferred` 延迟加载（不把 async 传染到同步构造点）。活体探针实证条目按预期落盘 | **close** |
| **D-120** | **经验落盘"静默丢条"（实测发现，两处）**：① `append_to_disk` 用 `if let Ok(..open(path))` + `let _ = writeln` —— 父目录不存在时 open 直接失败**被静默吞掉**（CLI 默认路径的 `memory/` 常不存在）⇒ 实测 CLI 跑完 answer 正确、exit 0，但 `experience.jsonl` **根本没生成**、语料永远长不起来；② run 收尾的追加是 `tokio::spawn` fire-and-forget，短命 CLI 进程可能在落地前退出 | P1-84 活体探针实测 | **已修**（P1-84）：自动建父目录 + 逐处 warn 留痕（不再静默）；追加改 **awaited** | **close** |
| **D-84** | **`ToolDispatcher::read_only_view()` 现无生产调用方**：其唯一生产消费者是子代理委派（D-83 已整段删除），现仅单测调用 | D-83 连带发现 | **裁决「保留」**（P1-80）：它是 xray **red 锚点** `readonly-view-strips` 锁定的只读视图**安全原语**（删原语即失去该回归防线），子代理能力若重启可原样复用；已在源码 doc 如实登记"当前无生产消费者" | **close · 裁决保留** |
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
