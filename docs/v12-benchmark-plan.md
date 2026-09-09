# v12 基准版施工图纸 —— 第一份"任务完成率 × 成本 × 时长"基线

> 版本目标代号：**v12-benchmark（造秤版）**
> 性质：**纯枝干**。不新增任何"器官"，不动 `crates/` 主干一行代码。
> 交付物只有一个：`bench/results/baseline-v12.md` —— 项目历史上第一份真模型端到端基线报告。
> 供审计窗口审计 → 规划窗口拆解 → 执行窗口执行 → 审计窗口验收。

---

## §0 执行铁律（违反任何一条 = 验收直接打回）

1. **白名单**：本版本只允许新增/修改以下路径，其余一律不许碰：
   - `bench/`（全新目录，任务夹具 + 跑分器 + 结果）
   - `docs/v12-benchmark-plan.md`（本文件）
   - `.gitignore`（追加 `bench/results/raw/` 与 `bench/.env`）
   - 验收命令：`git diff --stat <start>..HEAD -- crates/` 输出必须为**空**。
2. **密钥零入仓**：API key 只经环境变量（`bench/.env`，已 gitignore）传入。任何 `.md`/`.rs`/`.sh`/`.json` 里出现 `sk-` 或 `ark-` 前缀字符串 = 🔴 阻塞。
3. **零编译期网络**：夹具项目全部本地 vendored，禁止任何 build.rs 联网依赖（VM 无 github 出网，见 §7）。夹具用 `cargo build --offline` 必须能过。
4. **不许为了跑分改被测系统**：跑分中发现的缺陷一律记录进失败分类（§4），**不许现场修**。修复属于 v12.1 的漏斗迭代，不属于本版本。
5. 真 Linux 验收：整套基准在 VM（`wutao@192.168.220.131`）跑，Windows 本机结果无效。

---

## §1 北极星指标定义

一次基准 = **20 个任务 × 3 遍 = 60 次运行**，产出三元组：

| 指标 | 定义 | 口径 |
|---|---|---|
| **成功率** | 成功运行数 / 60 | 成功 = 会话正常结束 **且** 该任务的 `verify.sh` 退出码 0。两者缺一不可（防"自我感觉良好"） |
| **平均成本** | Σ(prompt_tokens + completion_tokens) / 成功数 | 从会话 `usage` 字段取（loop.rs 已累计），按模型单价折算 ¥ 仅作展示 |
| **平均时长** | Σ 墙钟秒 / 成功数 | 会话创建 → 终态的墙钟时间 |

辅助指标：按任务难度层（§2 的 L1–L5）分层成功率——这是漏斗分析的入口。

**运行参数固定**（写死在 runner 配置，保证三遍可比）：同一模型、同一 `--budget`（建议 40 步）、单任务墙钟上限 30 分钟、温度用 provider 默认。

---

## §2 任务套件设计（T01–T20，梯度递增）

每个任务 = `bench/tasks/T{NN}-{slug}/` 一个自包含目录：

```
T07-fix-off-by-one/
├── fixture/          # 被测小项目（完整可编译，vendored，无网络依赖）
├── goal.txt          # 喂给 agent 的任务描述（自然语言，一段话）
├── verify.sh         # 裁判脚本：在跑完后的 fixture 副本里执行，exit 0 = 成功
└── meta.json         # {"level":"L3","expect_files_changed":1,"timeout_min":30}
```

梯度分布（覆盖编码 agent 的核心动作谱）：

| 层 | 数量 | 类型 | 例子 | verify.sh 判什么 |
|---|---|---|---|---|
| **L1 读懂** | 2 | 只读问答 | "这个 crate 的公开 API 有哪几个函数？写入 ANSWER.md" | grep ANSWER.md 含关键词 |
| **L2 单点改** | 4 | 一行级修改 | "把默认超时从 30s 改成 60s" | grep 新值 + `cargo build --offline` 过 |
| **L3 修 bug** | 6 | 带失败测试的缺陷 | fixture 自带 1 个红测试（越界/逻辑反了/None 未处理） | `cargo test --offline` 全绿 |
| **L4 小功能** | 5 | 加函数+加测试 | "给这个解析器加 `--json` 输出模式，带测试" | 裁判自备的隐藏测试文件拷入后 `cargo test` 全绿 |
| **L5 跨文件** | 3 | 小重构/多文件协同 | "把重复的错误处理提取成公共函数，两处调用点都换掉" | 编译过 + 隐藏测试过 + `grep -c` 旧模式为 0 |

夹具语言：全部 Rust 小项目（每个 ≤ 200 行，依赖只用 std 或已 vendored 的极小 crate），理由：VM 工具链现成、`verify.sh` 判据客观（编译器+测试就是裁判）、与本项目场景同构。

**L4/L5 的隐藏测试**放 `verify-assets/`（不进 fixture，agent 看不见），防止 agent 直接"应试"。

---

## §3 跑分器（runner）设计

**形态：`bench/runner.py`（Python3 + 仅标准库 urllib/json/subprocess，零 pip 依赖）**。不新增 Rust crate——保持 workspace 干净，跑分器不值得进编译图。

单次运行的管线（每个 task × run 重复）：

```
1. cp -r tasks/T{NN}/fixture → /tmp/bench-run-{NN}-{r}/   # 每遍全新副本
2. POST /api/v1/sessions   {goal: goal.txt 内容, cwd: 副本路径, budget: 40}
3. 轮询 GET /api/v1/sessions/{id}  直到终态或 30min 超时（超时则 Cancel）
4. 在副本里执行 verify.sh（L4/L5 先拷入隐藏测试）
5. 落一行 JSONL → results/raw/baseline-v12.jsonl：
   {task, level, run, success, session_state, verify_rc,
    steps, prompt_tokens, completion_tokens, wall_secs, failure_stage}
```

`failure_stage` 由 runner 按证据自动初判（§4 分类），人工复核允许改判但要留注记。

**preflight 子命令**（`runner.py preflight`，正式跑前必过）：
- `curl $OPENAI_BASE_URL` 可达（VM 出网受限，见 §7——先证明模型端点可达再开跑）
- service `GET /readyz` 通过；用一个 3 步内完成的冒烟任务 T00 走通全管线
- 60 个副本的磁盘预算检查（≥ 5GB 空闲）

---

## §4 失败分类法（漏斗分析的原材料）

每次失败必须归入且仅归入一类（runner 初判 + 人工复核）：

| 代号 | 含义 | 判据线索 |
|---|---|---|
| `retrieval-miss` | 没找到该改的文件/位置 | 会话历史里从未 Read/grep 到目标文件 |
| `wrong-edit` | 找到了但改错 | 目标文件被改、verify 红、diff 与预期无交集 |
| `no-recover` | 编译/测试红后未能修复 | 历史含红→重试→仍红→终止 |
| `gave-up` | 主动放弃（GiveUp/DeliverAndQuit） | 终态非正常完成，nervous/subconscious 触发记录 |
| `budget-out` | 40 步耗尽 | steps == budget |
| `timeout` | 30 分钟墙钟超限 | runner 主动 Cancel |
| `infra` | 与 agent 无关的环境故障 | 网络断/服务崩——此类要**重跑补足**，不计入分母 |

产出要求：报告必须给出**失败漏斗直方图**（哪类失败最多 = v12.1 第一优先修什么）。这是"基准拉动优化"的接力棒。

---

## §5 交付物与报告格式

```
bench/
├── tasks/T00…T20/            # 21 个（含冒烟）
├── verify-assets/            # L4/L5 隐藏测试
├── runner.py
├── report.py                 # raw JSONL → baseline-v12.md
└── results/
    ├── raw/baseline-v12.jsonl   # gitignore，VM 上归档
    └── baseline-v12.md          # ✅ 入仓，唯一正式交付物
```

`baseline-v12.md` 必含：三元组总表、L1–L5 分层成功率表、失败漏斗直方图（文本条形图即可）、每个失败 run 的一行摘要（task/遍次/分类/一句话现场）、运行环境快照（模型名、budget、commit hash、日期）。

---

## §6 阶段拆解（供规划窗口分派）

| 阶段 | 内容 | 依赖 | 量级 |
|---|---|---|---|
| **B1** | 20+1 任务夹具 + goal.txt + verify.sh + meta.json | 无 | 主要工作量，可 2 窗口平分（B1a: L1-L3 / B1b: L4-L5+隐藏测试） |
| **B2** | runner.py（preflight/run/resume）+ report.py | 无（与 B1 并行） | ~400 行 Python |
| **B3** | 联调：T00 冒烟走通全管线（真模型） | B1 的 T00 + B2 | 半天级 |
| **B4** | 正式跑 60 次（VM，后台，可分 3 晚每晚 20 次） | B3 | 机器时间为主 |
| **B5** | report.py 出报告 + 人工复核失败分类 + 归档 | B4 | 半天级 |

**B3 是关口**：T00 走不通不许开 B4（防止烧 60 次 token 才发现管线断）。

---

## §7 VM 与模型接入注意（历史坑复用）

- 上传仍用 tar + 解压后 `touch` 所有源文件刷 mtime（防 target 缓存假结果——本版本不编译主干，但夹具编译同理）。
- **VM 出网现状：crates.io 通、github.com 不通、模型端点未验证**。preflight 第一项就是验证 `$OPENAI_BASE_URL` 可达：
  - 首选国内端点（Ark/混元等 OpenAI 兼容 base_url，`service/main.rs:73-129` 已支持 env 装配，无需改代码）；
  - 若 VM 全部模型端点不通 → 降级方案：service 跑在 Windows 宿主机不可行（无工具链），改为 **VM 跑 service+runner、模型走宿主机代理转发**，此决策 B3 阶段现场定，写进报告环境快照。
- 60 次运行的 token 成本预估要在 B3 冒烟后按 T00 实测外推，超预算先砍遍数（3→2）不砍任务数。

---

## §8 验收红线（审计窗口用）

| 级别 | 判据 |
|---|---|
| 🔴 阻塞 | `git diff -- crates/` 非空；密钥入仓；verify.sh 有一个不可独立执行；**秤未自证**（见下） |
| 🔴 秤的自证 | ① 用一个故意做不完的任务喂 runner → 必须记为失败且分类正确；② 用一个预先改好的 fixture 喂 verify.sh → 必须判成功。裁判先证明自己会判对错，才有资格判 agent |
| 🟡 需评审 | 任务难度分布偏离 §2 表；失败分类出现"大杂烩"（>40% 归入同一类需复核判据是否太粗） |
| 🔵 参考 | 基线数字本身**不设及格线**——v12 的目的是知道体重，不是减到多少斤 |

---

## §9 刻意排除（防范围蔓延）

- ❌ 修任何跑分中发现的 agent 缺陷（属 v12.1）
- ❌ 多模型对比（先单模型基线，对比属 v13+）
- ❌ 并行/多进程加速跑分（60 次串行跑，避免资源争抢污染时长指标）
- ❌ SWE-bench 等外部基准接入（github 不可达 + 夹具不可控，自建套件先行）
- ❌ 把基准接进 CI（60 次真模型跑不适合 CI 频率，属季度性体检）
