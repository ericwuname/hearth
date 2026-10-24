# 任务书 · 从可评测内核到可日常使用的终端 IDE v1.0（顶层 · 交付版）

> 签发时间：2026-10-18
> 签发背景：用户对本仓长程优化提出明确终点——**"做出一个稳定产出的 IDE，我可以真的上手试用，并且交付真实、高质量的成果"**，并授权由执行方**自行调研 + 给出可执行任务书**（用户自述"我无法判断，因为我不够专业"）。
> 本文件是**方向裁决与施工总包**，不是进度日志。进度日志见 [Hearth 长程优化探索规划 v1.0（traecode·全自主）.md](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/docs/Hearth%20%E9%95%BF%E7%A8%8B%E4%BC%98%E5%8C%96%E6%8E%A2%E7%B4%A2%E8%A7%84%E5%88%92%20v1.0%EF%BC%88traecode%C2%B7%E5%85%A8%E8%87%AA%E4%B8%BB%EF%BC%89.md)。

---

## 0. 一句话结论

**先修"卷尺"，再量"身高"，最后才做"衣服"。**

当前最优路径不是直接做 HA/多租户、也不是直接做 IDE，而是：

```
阶段0  修评测台（当前"通过率"不可信，因为判据能恒真）   ← 不做这步，后面所有数字都是自欺
   ↓
阶段1  真实端到端验证 + "稳定产出"定量基线（含"假完成率"）  ← 这是"交付真实"的唯一证据来源
   ↓
阶段2  终端 IDE v1（本地进程内 TUI，可直接上手试用）      ← 这是你要"上手试用"的交付物
   ↓
阶段3  HA / 多租户产品化 + 性能/吞吐专题（在阶段1的回归网之上"放大"）
```

理由（一句话版）：**用户要的三件事——"稳定产出""可上手试用""真实高质量"——分别对应 ①可信的度量、②可运行的客户端、③不撒谎的架构。而这三件事的公共前置条件是同一件：先证明我们量出来的东西是真的。**

---

## 1. 目标翻译：把"稳定产出的 IDE"变成可验收判据

用户的表述是主观感受，必须先译成**可判定、可复现、可拒收**的判据，否则无法验收。

### 1.1 "稳定产出" → 5 个一等指标（新增，进 CI 报表）

| 指标 | 定义 | 目标 | 现状 |
|---|---|---|---|
| **任务成功率（held-out）** | 在**从未被开发过程见过**的 held-out 任务集上的通过率 | ≥ 当前公开集通过率 − 10pp | **不存在**（无 held-out 集） |
| **假完成率** ⭐ | 会话自报 `done`，但外部 `verify` 判 FAIL 的比例 | **0** | **无度量**（这是本仓最该防的病） |
| **静默失败率** | 工具/子进程失败但未进入 `stderr`/事件/日志的比例 | **0** | 已修 2 例（D-155），**无系统性度量** |
| **崩溃/挂起率** | 压力场景下 panic / 死锁 / 无输出挂起的比例 | 0 / 0 / 0 | 有 `stress` 24/24 无 panic，**但未纳入常态回归** |
| **每解成本** | `total_cost_usd ÷ 成功解数`（未知则该解不计入分母并单独标记） | 不劣化 | 仅有单次会话成本（D-219 刚接线），**无"每解"口径** |

> ⭐ **"假完成率"是本任务书新增的第一等指标**，也是用户"交付真实"诉求的最直接对应物。业界研究（见 §3）显示：**agent 在未读完文件的情况下就有 67.9% 的运行发出误导性陈述，其中 80.4% 是可证伪的过度声称**。"自报成功"与"实际成功"必须由**两个互不可见的系统**分别判定。

### 1.2 "可上手试用" → 5 条验收动作（可人工执行，5 分钟内）

1. 用户在本机**单条命令**启动（不依赖 VM、不依赖 `service` 进程）。
2. 用户给出一个自然语言任务 → 看到**流式**执行过程与工具调用。
3. 用户**中途按 `Esc`/`Ctrl-C`** → 会话可恢复（`resume`），不丢历史、不丢已改文件。
4. 用户面对**审批请求**时，界面明确显示"要改什么、为什么、批不批"。
5. 用户能**一条命令导出**本次会话的可复核证据（改动 diff + 命令日志 + verify 结果）。

### 1.3 "真实、高质量" → 3 条拒收线

- **拒收1**：任何"通过"若无法在**独立进程**中复现，判为无效证据。
- **拒收2**：任何指标若其**判据可被恒真满足**（见 §2.1 T03 实例），判为无效指标。
- **拒收3**：任何能力若**只有测试引用、没有生产调用方**（既有病灶口径"零调用方死物"），不得计入交付。

---

## 2. 有证据的现状盘点（本仓，只列已核实事实）

### 2.1 评测台：**骨架完整，但判据强度参差**（这是最紧的瓶颈）

已有资产（真实存在）：
- `bench/tasks/T00 ~ T19`（20 个任务，各含 `goal.txt` / `meta.json` / `verify.sh` / `fixture/`）。
- [runner.py](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/bench/runner.py)（SSH 到 VM `192.168.220.131`，HTTP 驱动，落 `bench/results/raw/*.jsonl`）、[report.py](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/bench/report.py)、[replay.py](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/bench/replay.py)。
- **确定性回放**：`llm-replay` crate + `REPLAY_DIR` 环境变量（provider 名 `replay`）。
- 历史量化数据：deepseek 36/40 = 90%、zhipu 31/40 = 77.5%、replay 31/31 = 100%、stress 24/24 无 panic。

已核实的**缺口**（逐条有证据）：

| # | 缺口 | 证据 |
|---|---|---|
| G-1 | **判据可恒真** | [T03-fix-off-by-one/verify.sh](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/bench/tasks/T03-fix-off-by-one/verify.sh) 全文 5 行：无 `set -e`、无断言、`echo VERIFY_PASS` 后**退出码恒 0** ⇒ **无论对错都判 PASS** |
| G-2 | 判据强度**不统一** | 对比 [T00-smoke/verify.sh](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/bench/tasks/T00-smoke/verify.sh)（`set -e` + `grep -q multiply` + 失败 `exit 1`）与 [T13-fix-index/verify.sh](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/bench/tasks/T13-fix-index/verify.sh)（无 `set -e`，靠 `grep "test result: ok"` 兜底）——**三种写法并存** |
| G-3 | **无 held-out 测试** | `meta.json` 示例 [T00-smoke/meta.json](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/bench/tasks/T00-smoke/meta.json) 仅 `{"level","expect_files_changed","timeout_min"}`：**没有 `provider` / `model` / held-out 套件**字段 ⇒ 无法抗 reward hacking，也无法归因"改进来自模型还是来自 harness" |
| G-4 | **评测台不可本地运行** | runner 依赖 SSH 到 VM ⇒ **用户无法在自己机器上复现"通过率"** ⇒ 与"可上手试用"直接冲突 |

> G-1 与 G-2 是本任务书的**最高优先级**：它们意味着**历史通过率（90% / 77.5% / 100%）的可信度未知**。业界同类问题的严重性见 §3.1。

### 2.2 终端客户端：**不存在产品化 TUI**

- CLI 现为**行式 REPL + reedline**（19 个子命令），**无 `tui` / `ide` 子命令**。
- 仓内**唯一**含 `ratatui` / `crossterm` 的东西是 [regression/fixtures/tui-sample](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/crates/regression/fixtures/tui-sample)，其 `Cargo.toml` 里 `name = "hearth-tui"` —— 但它是**回归语料的项目上下文 fixture**，README 自述 **"not a product"**。
- 真实的 TUI 存在过的证据只在 VM（`/home/wutao/hearth-tui-new/`），**仓内无源码**。

### 2.3 HA / 多租户：**零覆盖**

- 全仓**没有任何**多租户、配额、隔离、并发租户相关的测试或度量。
- 部署面只有 `Dockerfile` / `docker-compose.yml`（单实例形态），**无隔离边界设计**。

### 2.4 性能/吞吐：**零度量**

- 有"有界读入/有界输出/进程树回收"类**安全性质**的修复（P0-06/07/08、D-33、D-155），方向是对的。
- 但**没有任何时延（TTFT / 每步耗时）、吞吐（并发会话）、资源（内存/CPU 峰值）度量**，也无 `bench` 中的性能任务。

### 2.5 制度层面的**冲突**（必须显式处理，不能装作没看见）

| 冲突 | 出处 | 与本任务书的矛盾 |
|---|---|---|
| U-40 冻结 | [unified-ledger.md](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/docs/core-freeze-candidate/unified-ledger.md#L107-L107)：`U-40 ｜ MCP / subagent / TUI ｜ v0.3 功能层，非地基` | 用户终局目标**就是要 TUI/IDE** |
| 前端排后 | [acceptance-cli-fix-pretty-2026-08-22.md](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/docs/acceptance-cli-fix-pretty-2026-08-22.md)（TUI 范围外、"Claude Code 式底部固定输入框"明确不做） | 同上 |
| 后端优先 | [hearth-roadmap-next.md](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/docs/hearth-roadmap-next.md)（后端智能 + 安全纵深 **优先于** 前端/CLI） | 同上 |

**处理原则（不违规、不硬闯）**：
- U-40 的原始语义是"**不是地基**"，**不是"永不做"**。本任务书**不修改历史文档**，而是**新增一条解冻条目**：`U-40' —— TUI 前置条件已满足（后端智能基线 + 阶段1回归网落地）后，TUI 作为"首个产品化客户端"解锁`。
- 前端优先级的排序**保持成立**：IDE 排在阶段 2，**晚于**阶段 0/1 的后端与度量收口。
- 历史反向证据必须尊重：[evidence-index.md](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/docs/core-freeze-candidate/evidence-index.md) 记载过一次 TUI 开发会话被判"**不可用 + 数据造假**、0/16 标 100%"。**这正是为什么阶段 0 必须先行**——否则 IDE 只会把"假完成"从后端放大到 UI。

---

## 3. 联网业界的结论（决定"怎么选"的外部依据）

### 3.1 关于"评测台可信度"——**业界公认这是头号问题**

- **SWE-Bench Pro Verified** 指出：即便上游 benchmark，其**评估本身被削弱**（evaluation is undermined）。[来源](https://arxiv.org/pdf/2609.08149v2)
- **BenchJack** 提出"评估缺陷八分类"，并在含 **SWE-bench、OSWorld** 在内的 10 个常用 benchmark 上，**在不尝试解决任何任务的情况下拿到接近满分**。[来源](https://arxiv.org/html/2605.12673)
- **SWE-ABS**：对 SWE-Bench Verified 有 **50.2%** 的实例可被"加强判据"，强化后各系统解出率**平均下降 14.56 个百分点**，**top-30 榜单发生 30 次名次变动**。[来源](https://arxiv.org/html/2603.00520)
- **Berkeley RDI** 的 exploit agent 通过 pytest hook 强制全过、容器内 parser 覆写、配置泄漏 + DOM 注入等手段，在 **SWE-bench Verified / Pro / WebArena 上拿到 100%**。[来源](https://rdi.berkeley.edu/blog/trustworthy-benchmarks-cont/)

> **直译到本仓**：`T03` 的 `exit 0` 恒真判据 = Berkeley 报告中"不解决任何任务即可 100% 分"的**同一个病**。**阶段 0 不是可选项，是本仓评测可信度的生死线。**

### 3.2 关于"防 Reward Hacking"的成熟配方

- **双套件**：可见 validation suite 用于开发，**隐藏 held-out suite** 用于判定（SpecBench 实践）。
- **验证者与被验证者分离**：**不让 agent 给自己判作业**；证据以 **artifact** 形态落盘（diff、命令日志、verify 原始输出），可被独立进程复算。
- **acceptance contract + 重试预算**：先声明"什么算完成"，再给有限次重试。

### 3.3 关于"HA / 多租户"的成熟配方

- **Salesforce BYOP**：处理 **7K+ 会话**的多租户 agent 平台，核心动作是**隔离每个推理引擎**——"teams own their code、独立扩缩、**零跨团队干扰**"。[来源](https://engineering.salesforce.com/building-a-multi-tenant-ai-agent-platform-handling-7k-sessions-without-cross-team-interference/)
- **SpecBox**：多租户 agent 的工具跑在 **OS 隔离容器或 microVM** 中，**跨租户内存访问由现有访问控制阻断**，并做 speculative sandbox 预热（预热产物在 agent 真正提交前**不产生副作用**）。[来源](https://arxiv.org/html/2607.23933)
- **GKE Agent Sandbox**：kata containers + Cloud Hypervisor（**microVM**）实现隔离。[来源](https://cloud.google.com/blog/products/containers-kubernetes/seaverse-chooses-gke-agent-sandbox)
- **kvwarden**：单 GPU 上做**租户公平**（tenant-fair）推理编排，解决"吵闹邻居"导致 TTFT 抖动。[来源](https://pypi.org/project/kvwarden/0.1.3/)

> **直译到本仓**：本仓已有的 `sandbox`（含 `NoopSandbox` + Linux 侧隔离）**正好是 SpecBox/GKE 说的那个隔离面**。所以 HA/多租户**不需要重造架构**，而是**把已有 sandbox 提升为租户边界，并补上"租户公平 + 隔离不可越界"的可测判据**。

### 3.4 关于"客户端架构"（IDE 怎么做最省力）

- **OpenCode** 的架构：**单一后端 server + 多客户端**（TUI 为主，另有桌面、SDK、远程）。

> **直译到本仓**：本仓已有 `service`（后端）+ `agent-runtime`/`run_local`（进程内直跑）。**IDE v1 应走"进程内 client-of-core"**（快、无部署负担、可上手），**而不是**"client-of-service"（要起服务、要 VM）。多租户所需的 client-of-service 形态留到阶段 3。

---

## 4. 方向裁决：把你列的三项 + 我补的一项排成序

| 序 | 方向 | 裁决 | 理由（一句话） |
|---|---|---|---|
| **M0** | **真实端到端验证（先修评测台）** | **立即执行，最高优先** | 它是另外三项的公共前置：没有可信判据，"HA 做完了""吞吐达标了""IDE 能用了"全是自报 |
| **M1** | **稳定产出基线（E2E + 一等指标）** | **紧随 M0** | 它产出"交付真实"的直接证据；也是 IDE 每个交互的回归钉 |
| **M2** | **终端 IDE v1（可上手试用）** | **M1 之后** | 用户要"上手试用"的实物；但**必须建在 M1 的回归网之上**，否则重演 evidence-index 的"0/16 标 100%" |
| **M3** | **HA / 多租户产品化** | **M2 之后（可并行推进设计）** | 复用已有 `sandbox` 作租户边界即 ROI 最高；在 M1 的指标框架上"放大"即可度量 |
| **M3′** | **性能/吞吐专题** | **并入 M3**（不单独立项） | 吞吐/时延**只有在多租户语义下才有意义**（单用户测吞吐是空的）；并入 M3 可避免"为测而测" |
| — | 你说的"其他我不知道的方向" | 见 §4.1 | 我替你把候选列全并给了裁决 |

### 4.1 "其他方向"的候选池与裁决（替用户穷举，避免遗漏）

| 候选方向 | 裁决 | 理由 / 触发条件 |
|---|---|---|
| 记忆/上下文压缩质量（memory） | **暂不做** | 已有多个长程任务书与实验数据（`memory-context-20260830`）；非当前瓶颈 |
| 拟真用户测试台（simuser） | **降级为 M1 的输入** | [Hearth 拟真用户测试台与自反馈闭环 规划书 v2.0.md](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/docs/Hearth%20%E6%8B%9F%E7%9C%9F%E7%94%A8%E6%88%B7%E6%B5%8B%E8%AF%95%E5%8F%B0%E4%B8%8E%E8%87%AA%E5%8F%8D%E9%A6%88%E9%97%AD%E7%8E%AF%20%E8%A7%84%E5%88%92%E4%B9%A6%20v2.0.md) 已有 v2.0；把它**变成 held-out 任务生成器**，价值立刻兑现 |
| MCP / subagent | **不做**（维持 U-40） | "人力差不是架构差"；与"稳定产出"无因果关系 |
| 多模型/多 provider 路由 | **并入 M3′** | 与吞吐/成本强相关，单独做是空转 |
| 可观测性/observer 深化 | **不做**（D-219 已收口） | 已接线，继续投入边际产出骤降 |
| 安全纵深（沙箱逃逸） | **并入 M3 的隔离判据** | 与多租户隔离**是同一个面**，合并可省一半工 |
| 跨平台（macOS/WSL） | **推迟到 M2 之后** | 与"本机可试用"相关但非阻塞（用户在 Windows） |
| 文档/知识库治理 | **不做** | 非瓶颈，且已有大量历史文档 |

---

## 5. 施工总包：里程碑 / WBS / 门禁 / 交付物 / 退出判据

> 每个里程碑的**通行规则**（沿用本仓既有纪律）：
> 1. **取证 → 立 D 号 → 先红后绿**（先构造真实失败 exit≠0，再修复证绿）；
> 2. **门禁四件套**（`cargo fmt --check` / `clippy -D warnings` / `test --no-fail-fast` / `sandbox` Linux 交叉 `check`）逐条取退出码；
> 3. **独立 commit + 推送 + 同步唯一活文档**；
> 4. **无红不修**：红侧构造不出来 ⇒ 只登记负结果/开放债，不落改动。

---

### M0 · 让"通过率"变可信（评测台判据加固）

> **进度（2026-10-19）**：**M0 全部完成**（M0-1/2/3/4/5/6）——D-222/P1-189 `477bb25`、D-223/P1-190 `17308dd`、D-224/P1-191 `f68d4c7`、D-225/P1-192 `68e7aa4`、D-226/P1-193 `c7b94f2`、D-227/P1-194 `db7afe3`、**D-228/P1-195 `41a2db4`**。
> **M0-6（本地可跑）已完成**：`python bench/run_local.py` 在本机起 service（只绑回环）+ `provider=replay` 驱动，**全量 31 条录制语料 31/31、用时 ~98 秒、零 API 成本、不连 VM**。⇒ **"我可以真的上手试用"这条现在有了硬前提。**（解读边界：replay 钉死 LLM ⇒ 测的是 harness 有没有回归，**不是模型能力**。）
> **M1 已开工（2026-10-19）**：**D-229/P1-196 `705e9db` + D-230/P1-197 `5ac5401`（订正）—— 一等指标「假完成率」落地**。
> 口径：`judge.classify_run(claim, external_ok)`，其中 **`claim` 是 agent 自己的结论**（transcript 里 `done.report.ok`，三态：是/否/未知），**不是** HTTP `phase`——`phase=="done"` 只表示循环正常终止。分母＝**自称做成的会话数**；`summarize_runs` 在分母为 0 时返回 `None`（未知≠0）。
> **两次自查发现的错误（都记在案）**：① 初版把 `phase` 当"自报完成"——31 条录制语料里 `report.ok==true` 其实只有 1 条，另 30 条自报 `budget_exhausted`，旧口径把分母灌水 30 条并吞掉反向不一致；② 对照实现用 `copytree(dirs_exist_ok=True)` 只覆盖不删新建文件，实测 T11 打回后判据仍 PASS ⇒ 对照失效，改整目录重建才成立。
> 基线（replay 全量 31 条）：**自称完成 31、外部通过 31、假完成率 0/31、反向不一致 0、未知 0**；对照（T11 整目录重建）**1/1 = 100%** ⇒ 量具能报正例。
> **边界**：replay 钉死 LLM ⇒ 这个 0.0% 是"harness 在录制轨迹下无假完成"，**不是真实运行下的水平**；真实值要等 M1-1 双轨活体。（`agnes-3.0-flash` 端点已实测可用：HTTP 200。）
> **M1-1 活体试跑已开跑（2026-10-19，D-231/P1-198 `c8002ab`）**：`provider=agnes` 本机跑 T00/T03/T11 各 1 条 ⇒ **3/3 判通过、自报与外部判据全一致、假完成率 0/3**（首个**真实**读数；n=3 仅冒烟，不作能力结论）。**活体揪出一个生产级 bug**：`done.report.usage` 恒 null——主回合走流式，llm-openai 请求不带 `include_usage`、解析层丢弃 usage 片 ⇒ CostMeter 流式路径永不记账 ⇒ **D-219/observer ④ 在生产从未生效**（门禁绿＝只测了非流式）。已修（请求层 + 解析层 + `FinishBuffer` 合并）并活体验证：usage null → **`{calls:4, prompt:17487, completion:293, total:17780}`**；`cost_usd=None`（agnes 未定价，未知≠0）。
> **M1-1 首张可信成绩单（2026-10-22，D-232/D-233）**：活体全量重测两轮完成——
> **公开集 31 条：24/31 通过（77.4%）、假完成率 7/31 = 22.6%、反向不一致 0；均 14 步 / 51.5s / 每解 ~41k tokens**。
> **held-out 8 变体：8/8 通过、假完成率 0/8、均 14 步 / 37.9s / 每解 ~37k tokens**。
> ① **FAIL 逐一核对工作区源码 ⇒ 判据零误伤**（首轮 8 条、复测 7 条，全是 agent 真没做成却自报 completed），含首例"**改测试断言迁就错误代码**"作弊形态（T02：常量 30 不动、反把测试断言 30s→60s）。两轮 FAIL 名单不同（T03/T08/T16 首轮挂、复测过；T00/T02/T12 两轮都挂）⇒ 同题成功率有真实波动——这正是"稳定产出"要度量的对象。
> ② **连带揪出 CostMeter 跨会话污染（D-233）**：首轮 31 条 total_tokens **严格单调递增** 48,097 → 428 亿 ⇒ `usage` 是进程累计值而非会话值。已修（彻底退役共享 meter 接线 + 复利回写；回归门 RED A=30/B=105 → GREEN）并复测验证：v2 两套件 usage 均不再单调、量级 13k~63k（会话级）；**"每解 token 成本"自此可信**（USD 列如实未知——agnes 未定价，未知≠0，不按 0 冒充）。
> 工具：`bench/scorecard.py`（吃 `run_local.py` 的 jsonl 产出本成绩单）；明细数据 `bench/results/raw/live-agnes-{full,heldout}-v2.jsonl`（gitignored）。
> **M1-1 双轨录制完成（2026-10-22，D-234/D-235）**：`run_local.py --record-dir` 在**活体轨**把每条会话 transcript 固化成 replay fixture（31 条，入 `bench/replay/live/`）；`bench/replay_fidelity.py` 一条命令验两件事——① **回放结论与活体一致 31/31**；② **篡改一条响应（把写操作改道）⇒ 结论必须翻转（PASS→FAIL）**。录制侧两条纪律：展示文本脱敏（`<WORK>`/`<HOME>`）、工具参数**可移植化**（会话目录绝对前缀 → 相对路径 / `cd .`，参数是语义载荷，占位化会让回放访问不存在的路径）。门禁抓出真 bug：`ReplayProvider::stream` 恒写 `index: 0` ⇒ 双调用回合被消费端合并（D-235），修复后保真由 26/31 升至 **31/31**。
> **M1-3 静默失败率落地**：口径＝「`is_error` 但 output 空白（空错误）+ `tool_call` 无配对 `tool_result`（结果丢失）」÷ 工具调用总数；分母 0 ⇒ **未知**。首份读数：公开集 **1/252 = 0.4%**、held-out **0/41 = 0.0%**。该 1 例直接催生 **D-234**（预算在工具执行前耗尽时，pending 调用被静默丢弃）——已修为逐条补 `is_error=true` 的 `[not executed]` 结果事实。
> **M1-5 每解成本落地**：`已定价成功解总成本 ÷ 已定价成功解数`；**未定价解不计入分母、单列**；分母 0 ⇒ **未知**（绝不报 `$0`）。当前 agnes 无价表 ⇒ 两套件均如实"未知（全部未定价）"——这不是 0 成本，是**未定价**。
> **最新读数（v3，2026-10-22）**：公开集 **30/31（96.8%）、假完成率 1/31 = 3.2%**、均 16 步 / 54.1s / 每解 ~44k tokens、静默 0.4%；held-out **6/8（75%）、假完成率 2/8 = 25%**、静默 0.0%。
> **统计诚实性（必须记住）**：held-out 同一批实例两次实测 **8/8 → 6/8** —— **单次 8 条样本不足以下能力结论**，波动本身就是"稳定产出"要度量的对象；公开集 31 条三轮：77.4% → 74.2% → 96.8%，同样有波动。下一步若要给出可信区间，必须做**多次重复**（N≥5/任务）并报均值与方差。
> **M1 退出判据达成（v4，2026-10-22）**：完整活体跑 **31/31 通过、假完成率 0/31 = 0.0%**、静默失败 0/191 = 0.0%、反向不一致 0；held-out v4 **8/8、0/8**、静默 0/30。报表 5 项指标齐备 ⇒ 任务书 M1 的三条退出判据（报表 / **假完成率=0 在至少一轮完整活体跑成立** / 同 trace 回放逐条一致）**全部成立**。均 11 步 / 36.3s / 每解 ~29.8k tokens（held-out 8 步 / 30.1s / ~21.3k）。
> **这轮 0.0% 不是"运气好"**：v3 的 1/31 假完成被逐事件归因到三个真 bug——**D-237**（Windows 默认 `bash` 指向 WSL ⇒ 测试从未真正执行）、**D-238**（跨会话 cwd 泄漏 ⇒ bash 编译了别的会话）、**D-239**（子进程 `env_clear` 后无 PATH/TEMP ⇒ cargo 找不到、link.exe LNK1104）。修复后端到端复验：T00 **1P/1F → 2/2**（工具失败 4→0）、T03 **2/2**，再跑完整 v4。**单轮语义如实登记**：v4 是"修复后的一轮"，不构成稳定成功率估计（历史 77.4%→74.2%→96.8%→100%）；可信区间仍需 N≥5/任务。
> **稳定基线首测（10 任务 × 5 轮，2026-10-24）**：**47/50 = 94.0%**，Wilson 95% 区间 **83.8% ~ 97.9%**；假完成 **3/50**（T02/T03/T06 各 1 次，逐条核对：T02 **零写操作**、T06 编译 E0308、T03 修成 21 而非 55——**判据零误伤**）；静默失败 **0/290**；任务通过率均值 **94.0%**、标准差 **9.7pp**、范围 **80%~100%**；均 **10.7 步 / 15.9s / 每解 ~27.9k tokens**（总 1.40M）。**口径**：这是 **10 任务子集**，不外推为全量 31 条的稳定率；工具 `bench/stability.py`。**结论**：稳定产出**还没有 100%**——3 次失败全是"没做成却自报完成"，后续优化优先压这类失败。
> **v4 语料刷新**：`bench/replay/live/` 31 条实录由 v4 重录（环境修复前的旧 trace 会让"回放 vs 活体"分叉）；`bench/replay_fidelity.py` 复跑 **31/31 一致 + 篡改翻转**。压力场修复后复跑 8×3：参与 21 / 跳过 3（ST5）/ 死亡 0 / panic 0 / 挂起 0 ⇒ **不可用率 0.0%**。
> **D-240（读数卫生）**：`run_local.py --out` 曾 append ⇒ 一次填错 `--tasks-dir` 的失败跑（8 条）与正确跑（8 条）叠成 **8/16 = 50%**（真实 8/8）。已改**默认清空式** + `--append` 显式开启；Python 门禁新增 2 条（**70 条全过**）。
> **M1-4 崩溃/挂起率落地（2026-10-22，D-236）**：新增 `bench/stress_local.py`——8 场景在**本机**跑（不再 SSH VM）：ST1 预算耗尽 / ST2 越界删除被路径守卫挡下 / ST3 空工程 / ST4 并发（5 会话 + 300 并发请求）/ ST5 磁盘满 / ST6 畸形输入 / ST7 会话隔离 / ST8 审批闸。provider=replay + **按场景合成的最小回放**（压力场考 harness 韧性，不考模型能力；合成 fixture 不是实录，也不参与能力读数）。
> 读数（8 场景 × 3 轮）：**参与 21 轮（ST5 在 Windows 无 tmpfs 如实跳过 3 轮）、服务死亡 0、panic 0、挂起 0 ⇒ 不可用率 0.0%**；行为断言全过（ST8 三轮都观察到 `need_approval` 且目标文件存活）。**量具红侧自证**：真 `kill` 服务 ⇒ `alive=False`、不可用率 1.0（不是只靠合成数据自证）。
> **M1-6 关键路径 E2E（2026-10-22）**：新增清单门禁 `bench/tests/test_keypath_e2e.py`——6 条路径（红线 `blocked` / observer ④ / `Simplify` / `resume` / 审批闸 / 非 UTF-8 输出）各钉「门禁文件在场 + 锚点函数在场」，**挪走任一锚点即 FAIL**（已自证）。如实登记：**审批路径没有 Rust 集成测试**，真正验证"审批拦住执行"的是 ST8 运行时 E2E（真 service、真 SSE、真调度器超时），它留在 M1-4 常态跑。
> 实测确认了这个方向是对的：**未做任何修复的 T03 也会被判「通过」**——fixture 上 `cargo test` 退出码 **101**、`test result: FAILED. 0 passed; 2 failed`，而判据打印 `VERIFY_PASS` 且退出码 **0**，runner 又只看输出子串 ⇒ 记「成功」。
> **自查抓出我自己的一次漏改**（D-224）：D-222 改了 `runner.py` 却漏了同源的 `replay.py`（旧口径 + 不上传判据库）⇒ 修好后回放路径 31 条语料会**全判失败**。已收口为唯一实现 `bench/verify_exec.py` 并加**源码级门禁**。**教训**：漏改的后果是"静默的错误读数"，不报错。
> **补上了判据的另一半方向**（D-225/D-226，原任务书没写）：D-222 只堵了"恒真"（未修复却判通过），**"做对了却判失败"无人管**——那会让通过率被**系统性低估**，且 M1 分不清"agent 不行"与"判据有问题"。现新增 `bench/solutions/`（参考解）并把门禁改**双向**：① 未修复 **20/20** 必判失败；② 参考解 **20/20** 必判通过（**全覆盖**）。另有 `REQUIRE_FULL_COVERAGE=1`（CI 开）——**新增任务若不带参考解，CI 会红**，不允许覆盖率被静默摊薄。
> 门禁当场抓出一条真问题：**T13 的 fixture 里 `t_safe_bug` 断言的是 bug 行为**，所以"修对函数"必然让该测试红——该任务实际**蕴含**"编码 bug 的测试期望也要一并修正"（任务描述没写这一点，属**任务设计歧义**，已登记）。
> **补上了 held-out（M0-4）**：公开集是固定的 ⇒ 通过率**无法区分**"能力提升"与"记住这 20 题"；而公开仓里"把 held-out 藏进另一个目录"只是**假 held-out**。现改为**只公开生成器、实例按种子现场生成**（`bench/gen_heldout.py`，4 个模板家族，产物不入库），并让同一套双向门禁**换根复用**（CI 对公开集与 held-out 集各跑一遍）。held-out 实测 ① **8/8**、② **8/8**。**局限已如实登记**：生成器公开 ⇒ 非密码学保密；每家族只有一种"技能形状"，同族变体上分涨**不能**推断泛化到新任务类型。
> **⇒ 结论：历史通过率（deepseek 90% / zhipu 77.5% / replay 100%）在判据修好前不可采信**；其中 `replay 100%` 更要重述——它是在**旧判据**与**被打断的路径**下测出来的。**现在已可在本机重测**（`python bench/run_local.py`）：本地实测 **31/31**（新判据下、约 98 秒）。
> **下一步 = M1**：真实端到端 + "稳定产出"定量基线（含一等指标**假完成率**、双轨录制回放、崩溃/挂起率、每解成本）。M0 遗留两项如实登记：① 本地回放**接进 CI** 需先处理 GitHub runner 的 cgroup 口径；② held-out 家族目前只有 4 个，覆盖远小于公开集。

**退出判据**：20/20 任务的 `verify.sh` 满足**统一强判据模板**；且**故意做错**的 agent 在同一判据下**判 FAIL**（这是 M0 的红侧）。

| 编号 | 任务 | 交付物 | 红侧构造（先红后绿） |
|---|---|---|---|
| M0-1 | 定稿**统一 `verify.sh` 模板**：`set -euo pipefail` + 显式断言 + 失败 `exit 1` + 打印 `VERIFY_PASS` 仅作为**最后一行** | `bench/verify-template.sh` + 规范文档 | 用 T03 现状跑一次"错误实现"，记录它仍报 PASS（= 红） |
| M0-2 | 逐任务加固 `verify.sh`（20 个），并按模板断言**产物**（不只断言 `cargo test` 绿） | 20 份改写后的 `verify.sh` | 对每个任务注入"最小错误改动"，断言必须 FAIL |
| M0-3 | 补 **`meta.json` 字段**：`provider` / `model` / `suite`（`public` \| `held-out`） / `acceptance`（判据摘要） | 20 份 `meta.json` | 现状缺字段 ⇒ 断言必须失败 |
| M0-4 | 新增 **held-out 套件**：从 `simuser` 规划 v2.0 与现有 20 题的"变体"生成 ≥10 题，**不进入开发可见目录** | `bench/tasks-heldout/` + 生成脚本 | 用公开集调优的 prompt 在 held-out 上应当掉分；若不掉分则说明 held-out 无效（负结果也要登记） |
| M0-5 | **门禁化**：`bench/` 判据自身进 CI（含"判据可恒真"检测：每个 `verify.sh` 必须在"错误实现"下退出非 0） | `.github/workflows/ci.yml` 新增 job | 故意放一个恒真判据 ⇒ CI 必须红 |
| M0-6 | **本地可跑**：runner 支持 `--local`（不经 VM，直接进程内跑 `hearth`），保留 VM 模式 | `bench/runner.py` 改造 | 无网络/无 VM 环境下跑通 smoke |

**风险对冲**：若 M0-4 的 held-out 无法生成有效题目（负结果），**不硬造**，改为"公开集 + 判据加固 + 独立复算"三重证据，并在文档如实登记"held-out 暂缺"。

---

### M1 · 稳定产出基线（真实端到端 + 一等指标）

**退出判据**：`bench/report` 能输出 §1.1 全部 5 项指标；**假完成率 = 0** 在至少一轮完整活体跑中成立；同 trace 在 `replay` 下**逐条一致**。

| 编号 | 任务 | 交付物 | 红侧构造 |
|---|---|---|---|
| M1-1 | **双轨录制**：活体跑一次（真 provider）→ 固化 trace 进 `REPLAY_DIR` → 常态回归只用 `replay`（确定性、零成本、可离线） | 录制/回放脚本 + 至少 3 条真实 trace | 篡改 replay 中的一条响应 ⇒ 断言结果必须改变（证明回放真的在起作用，而非"录了不看") |
| M1-2 | **假完成率**度量：会话自报 `done` 与外部 `verify` 结果**跨进程比对**，输出 `false_completion_rate` | `bench/report.py` 新增指标 + 报表 | 构造一次"自报 done 但 verify FAIL"的会话 ⇒ 指标必须 > 0 |
| M1-3 | **静默失败率**度量：统计"工具返回 Err / 子进程非 0 但无用户可见提示"的比例 | 同上 | 复用 D-155 的 RED 手法（把提示临时置空）⇒ 指标必须上升 |
| M1-4 | **崩溃/挂起率**：把 `stress`（24 场景）纳入常态回归并计入报表 | `bench/stress/` 独立 job + 报表字段 | 注入一个 panic ⇒ 回归必须红 |
| M1-5 | **每解成本**：`total_cost_usd ÷ 成功解数`（未定价解**排除分母并单列**，遵循 D-46/D-176 未知≠0） | 报表字段 | 造一个未定价 provider ⇒ 必须落"未知单列"，不得当 0 |
| M1-6 | **关键路径 E2E 覆盖**：红线 `blocked`（D-208）、observer ④（D-219）、简化步（D-214）、`resume`、审批、非 UTF-8 输出（D-155）——每项一条 E2E 断言 | `bench/e2e/` 或 `crates/*/tests/` | 逐项回退实现 ⇒ 对应 E2E 必须红（沿用已建的门禁文件） |

---

### M2 · 终端 IDE v1（可上手试用）

**架构裁决**：**本地进程内 TUI（client-of-core）**，复用 `run_local` / `agent-runtime`，**不依赖 `service` 与 VM**。

**退出判据**：§1.2 的 5 条验收动作**全部通过**，且每个交互动作在 M1 的 trace 上有一条 E2E 钉住。

| 编号 | 任务 | 交付物 | 说明 |
|---|---|---|---|
| M2-1 | **技术选型定稿**：`ratatui` + `crossterm`（仓内已有 fixture 依赖，证明编译面可行） | 选型记录（含与 `tui-sample` fixture 的**明确划界**：那是语料，不是产品） | — |
| M2-2 | **TUI 骨架**：单命令启动、流式输出、工具调用可视化、状态栏（含 `phase`，含新增 `blocked`） | `crates/hearth-tui`（新 crate，非 fixture） | 复用 `feedback` 事件流，不新造协议 |
| M2-3 | **中断与恢复**：`Esc`/`Ctrl-C` → `resume` 可用；已改文件不丢 | TUI 交互 + E2E | 红侧：杀掉进程后 `resume` 必须能续（无则登记负结果） |
| M2-4 | **审批交互**：清晰展示"要改什么/为什么/批不批"，含超时与拒绝路径 | TUI 面板 + E2E | — |
| M2-5 | **一键证据导出**：会话结束输出 diff + 命令日志 + verify 结果 | `hearth session export` | 与 M1-2 的跨进程比对共用同一产物格式 |
| M2-6 | **上手脚本**：`docs/onboarding.md`（≤1 页）+ 3 个真实可跑示例（T00 类） | 文档 + 示例 | 验收 = 用户能在 5 分钟内跑通第 1 例 |

**风险对冲**：若 `resume` 在进程内形态下无法保证（负结果），**如实降级**为"会话持久化 + 重启后可选恢复"，并在文档登记；**不得**声称已可中断恢复。

> **M2 v1 交付（2026-10-22，D-241）**：`hearth tui` 已可启动并跑真实任务（headless E2E：**completed / 3 步 / 1 次工具 / 66 token 事件**）；审批面板、`Esc` 优雅中断（**paused → 会话档落盘 → `hearth resume` 实测续跑**）、`hearth session export <id>`（diff + 报告 + 命令日志单 JSON）已落地。**审批链 headless E2E**：`--auto-approve` + 真破坏性命令 ⇒ `approvals=1`、目标目录被删、completed（InteractionRequested → dispatcher 放行全链）。**视觉验收已通过（2026-10-22）**：真 TTY（Windows Terminal）下全屏渲染、审批面板（含参数/原因，按 `n` 后目标目录未被动）、`Esc` 中断（`paused`·上下文保留）均**人工目测通过**；验收中抓到并修复 **D-242**（`RUST_LOG` 日志画花全屏 ⇒ TUI 日志强制落文件）。**负结果**：硬杀进程时当前未完成轮不落盘，只能恢复到上一轮。
> 上手指南：[onboarding.md](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/docs/onboarding.md)。**安装与启动**：`hearth` 装到 `%USERPROFILE%\.cargo\bin\` 后单命令可用；Windows 上"规范位置被 PATH 护栏误报"已修（D-244），启动不再刷 WARN。

#### M2-G · 原生桌面 GUI（2026-10-23，用户决策：IDE 形态改为 GUI，TUI 保留）

**决策变更**：§7 第 3 项的默认值原为"终端 TUI（非 GUI / 非 Web）"；用户明确要求 **GUI**。处置＝**新增** GUI 前端（不删除 TUI），两者共用同一内核与同一套会话落盘，**不引入 service/VM 依赖**。

**选型**：`eframe/egui 0.29`（纯 Rust、单进程、无 WebView/Node 工具链）。最新 0.37 要求 Rust ≥1.95 且为全新 Ui-first API（`TopBottomPanel` 不存在、`App` 必须实现 `ui`）——本仓钉 0.29，避免为一版 UI 框架抬高整仓工具链。

**架构裁决**：`codex-cli → hearth-gui → hearth-tui`——GUI **复用** `TuiState` 的事件→状态模型（单一实现，GUI 只换渲染层）；provider / dispatcher / 会话落盘仍由 CLI 单一构造。GUI 持独立 tokio Runtime，agent 在 `rt.spawn` 跑，事件/完成经 channel 回流；**不用 `block_on`**。

**交付**：`hearth gui [--goal] [--budget]`；顶栏（session / phase / 步数 / 运行中·就绪 / 中断 / 一键导出证据）、流式正文 + 工具行、底部输入（Enter 发送）、**审批条**（要执行什么 + 具体参数 + 批准/拒绝）。CJK 字体从系统加载。

**视觉验收（Computer Use 截图）**：mock provider 驱动真实 agent 内核 ⇒ 流式正文、`write_file`/`bash` 工具行与真实输出、破坏性命令审批（点「批准」→ 执行 → 最终答复 → `completed · 7 步`）、导出证据按钮——全部实测通过。

**验收中修掉的 GUI 独有缺陷**：① floating `egui::Window` 的审批按钮落在裁剪区外 ⇒ 点击无效（只能等内核 60s 超时被拒）→ 改**底部固定审批条**；② run 结束（含超时拒绝）后未清 `pending_approval` ⇒ 残留失效审批条 → done 分支清理并如实提示；③ 首帧未给输入框焦点 → 首帧 `request_focus`。

**如实登记**：GUI 无 headless 渲染单测（需窗口环境），本轮以真机截图验收为准；用户机器当前 Agnes key 被服务端 401 拒绝（与 GUI 无关，换有效 key 后即可使用）。

**产品化第一批（2026-10-25，D-247）**：顶栏第二行显示 provider/model 与工作区（字符级截尾、悬停看全路径）；预算 `DragValue`（4~200 步，仅影响后续提交）；相邻「工具调用 + 结果」合成**可折叠卡片**（`✓/✗/…` + 工具名；最新一张展开，新卡出现自动折叠上一张）；结果像 diff 时逐行着色；快捷键 `Esc`（拒绝待审批 / 中断运行）、`Ctrl+E`（导出证据）、`Ctrl+Q`（空闲退出）。5 条纯函数单测（相邻配对/无结果独立/diff 识别/工具名/长 CJK 路径）。真 Agnes 实跑 create→run→patch→run ⇒ 四张卡片 + `completed · 9 步`；`sleep 60` 运行中 `Esc` ⇒ 已请求中断。

**DSH 生态接入（2026-10-25，D-248，用户决策 A 试点）**：`integrations/dsh-plugin-hearth` 零依赖 host 插件，向 DeepSeek Harness 注册 `hearth_sessions / hearth_export / hearth_status / hearth_run`。默认 fail-closed（不委托审批）、`execFile` 不经 shell、cwd 必须绝对路径。安装到 `~/.dsh/profiles/desktop`（备份后加 `link:` 依赖 + bundles + junction），重启 DSH 后插件页可见且 ON；模型实际调用三工具并引用 59 个本地会话/审计闭环/预算/审批差异。**后续产品路线（用户已选）**：**B. Fork DSH Desktop（MIT）** 作为正式外壳；OpenCode Desktop 仅作测试对照壳。

**B 路线首个可运行 fork（2026-10-25，D-249）**：源码 `deepseek-harness @22ddee37` 浅克隆到独立目录；品牌改为 **Hearth**（productName/appId `com.hearth.desktop`/artifact/协议显示名）、自绘房子+火焰图标与托盘 ICO、侧栏 Hearth 字标（不再 gate official profile）、19 处用户可见文案与安装器语言、本地构建标签 `Hearth 本地构建`；unsigned 本地构建跳过 mandatory-update policy。`build:official` 10/10 gates；`package-target win-x64 --dir --unsigned`（electron-builder 镜像走 npmmirror）产出 `Hearth.exe` 并启动成功，窗口标题显示 `… — Hearth 本地构建`；packaged smoke 通过。**边界**：未签名、无 NSIS 安装器；与 DSH 共用 `~/.dsh`（互斥运行）；插件暂未打进 runtime；macOS 未验证。

**B 路线第二批（2026-10-25，D-250）**：① 未签名 NSIS 安装器 `hearth-0.2.1-rc.3-win-x64-unsigned.exe`（314 MB）；② `dsh-plugin-hearth` 打进 runtime（`contrib/` 副本 + 额外 pack + `ROOT_PACKAGES` + `web` 模板），新 profile 离线可用；③ 默认 home 改 `~/.hearth`，`scripts/migrate-dsh-home.ps1` 迁移 sessions/credentials/provider patch（修掉 PS 5.1 UTF-8 BOM 导致的 Host JSON 解析失败）；④ 桌面 locale/CLI/安装器品牌审计完成，Chromium userData 迁到 `%APPDATA%\Hearth Desktop`。**防漂移**：`engines.hearth>=0.2.27`、`hearth_status` 兼容性行、`test/contract.mjs`（版本/会话行/导出 JSON 形状）；fork 钉上游 commit。

---

### M3 · HA / 多租户产品化 + 性能/吞吐专题

**架构裁决**：**把已有 `sandbox` 提升为租户边界**（对齐 SpecBox/GKE 的 microVM/容器隔离思路），容器/进程隔离优先于重造。

**退出判据**：≥3 个租户并发跑同一 benchmark，**隔离不可越界**（租户 A 不可读租户 B 的任何文件/环境/日志）+ **租户公平**（无"吵闹邻居"导致某租户 TTFT 劣化超过阈值）+ 全指标不劣化。

| 编号 | 任务 | 交付物 | 红侧构造 |
|---|---|---|---|
| M3-1 | **租户边界定义**：tenant id 贯穿 workspace / home / 日志 / observer 目录 | 设计文档 + 代码 | 构造"跨租户读"尝试 ⇒ 必须被拒（复用 D-208 红线的失败路径思路） |
| M3-2 | **隔离判据 E2E**：A 不可读 B 的文件/env/日志/observer 报告 | `tests/multitenant_isolation_e2e.rs` | 故意放行一次 ⇒ 必须红 |
| M3-3 | **并发跑批**：N 租户 × M 任务，输出成功率/时延/成本**按租户**分解 | `bench/multitenant/` | — |
| M3-4 | **租户公平**：定义 TTFT p95 劣化阈值（对齐 kvwarden 的 tenant-fair 思路）；超阈值判红 | 报表指标 | 注入"吵闹邻居"（满载租户）⇒ 指标必须可测出劣化 |
| M3-5 | **吞吐/时延度量**：TTFT / 每步耗时 / 峰值内存 / 并发会话数 | 报表字段 | 构造一次无界读入回潮 ⇒ 资源指标必须可测出（与 D-33 呼应） |
| M3-6 | **共享资源调度**：模型调用配额、并发上限、队列——**未定价/未知一律不参与阈值**（D-176） | 调度层 + 单测 | — |

---

## 6. 需显式修订的冻结条目（不修改历史文档，只新增解冻条目）

| 原条目 | 出处 | 修订方式 |
|---|---|---|
| **U-40**（MCP/subagent/TUI = 非地基，v0.3） | [unified-ledger.md#L107](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/docs/core-freeze-candidate/unified-ledger.md#L107-L107) | **新增 `U-40'`**：TUI 的"非地基"判定**不变**；但其**解锁前置条件**明确为"M0 + M1 完成"。MCP/subagent **维持不做**。 |
| TUI 范围外 / 底部固定输入框不做 | [acceptance-cli-fix-pretty-2026-08-22.md](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/docs/acceptance-cli-fix-pretty-2026-08-22.md) | 该冻结是**当时**对"后端优先"的排序保护；M2 生效后**不再适用**。新增注记指向本任务书 §4 的排序。 |
| 后端智能 + 安全纵深优先于前端/CLI | [hearth-roadmap-next.md](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/docs/hearth-roadmap-next.md) | **排序保持成立**（IDE 排 M2，晚于 M0/M1）；仅**新增**"终局交付物 = 可上手 IDE"这一目标陈述。 |

> 纪律：以上**均以新增条目/注记方式**落在**本文件**与唯一活文档，**不改写**任何历史文档正文。

---

## 7. 需用户决策 / 提供的事项（尽量少，且都给了默认值）

| # | 事项 | 我的默认选择（若你不回复即按此执行） | 为什么需要你 |
|---|---|---|---|
| 1 | **活体 E2E 的 provider** | 用 `.env` 中已配置的 provider 先跑通（key 已在 `.gitignore`，任何 key 不进被跟踪文件） | 影响成本与真实度；若你只想离线，则 M1-1 只做 replay |
| 2 | **成本上界** | 单轮活体跑批设 `HEARTH_COST_BUDGET_USD` 上界，超限即停（复用 D-214 的预算机制） | 防止跑批烧钱 |
| 3 | **IDE 形态** | **已决策（2026-10-23）**：原生桌面 GUI（eframe/egui）+ 保留 TUI 双前端 | 用户明确要 GUI；实现见 M2-G（D-246） |
| 4 | **是否允许改 `bench` 判据** | 允许（这是 M0 的全部内容） | 会改变历史通过率口径，历史数字需重跑 |
| 5 | **是否接受"历史通过率需重测"** | 接受——**重测后可能变低**，但那才是真实值 | 这是诚实性选择，需你知情 |

---

## 8. 与既有活文档的引用关系

- **本文件 = 阶段方向与施工总包**（唯一权威）。
- [Hearth 长程优化探索规划 v1.0（traecode·全自主）.md](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/docs/Hearth%20%E9%95%BF%E7%A8%8B%E4%BC%98%E5%8C%96%E6%8E%A2%E7%B4%A2%E8%A7%84%E5%88%92%20v1.0%EF%BC%88traecode%C2%B7%E5%85%A8%E8%87%AA%E4%B8%BB%EF%BC%89.md) = **逐轮进度与 D 号台账**（只记做了什么，不做方向裁决）。
- [Hearth Roadmap-chatgpt.md](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/docs/Hearth%20Roadmap-chatgpt.md) = **设计基线**（Hearth Core 蓝图 v2），不改。
- 本任务书的执行进展，**逐条回写**到活文档的卡表与 D 表（沿用既有节拍：取证 → 立 D 号 → 先红后绿 → 门禁四件套 → commit/push）。

---

## 9. 一句话给非专业读者（你的版本）

你现在缺的不是"更多功能"，而是**一把准的尺子**。

现在的情况像这样：你用一把**刻度是错的尺子**量身高，量出"90 分"，于是不停加功能想把分数推到 95——但尺子本身会把"做错了"也判成"通过"（`T03` 就是活证据：**不管做对做错都报 PASS**）。业界的独立审计显示，这类问题能让一个不解决任何任务的 agent 在 SWE-bench 上拿到 **100 分**。

所以顺序是：

1. **先把尺子校准**（M0）：让"做错"必然判失败。
2. **再量一次真实成绩**（M1）：这次数字可能**比 90 分低**，但它是真的；同时量出**"假完成率"**——这才是"交付真实"的核心。
3. **然后做你能上手的东西**（M2）：终端里的 IDE，一条命令启动，能中断能恢复，能一键导出证据。
4. **最后才放大**（M3）：多租户 + 隔离 + 吞吐，在已经准了的尺子上面量。

我会按这个顺序**自主推进**，每完成一项就提交、推送、同步台账；每项都遵守"**先红后绿**"（先证明它能坏，再证明修好了），不会出现"我说做完了"而没有证据的情况。
