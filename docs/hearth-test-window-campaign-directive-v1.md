# Hearth 测试窗口 Campaign 测试指令书 v1（P4 Node 11-12：窗口分离裁决）

> **签发**：顶层 → 测试窗口　**日期**：2026-08-31　**依据**：P4-REVALIDATION-01 总包 v2.1 §12/§13 + 窗口分离裁决
> **前序指令**：《P4-Node03独立复现测试指令 v1.1（顶层→测试窗口）》——**必须先闭环**（见 §2-T1），本指令书与其不冲突、是其后续阶段
> **执行范围**：Node 11（SIMUSER Controlled Campaign）+ Node 12（Failure/Attractor Campaign）
> **版本冻结**：**v0.2.20**（tag v0.2.20；.131 binary sha256 前 16 位 `fc39c27c…`——P-1 对版本以此为准）
> **一句话**：执行窗口已把测试台建好、修复做完、宣布收工；现在轮到测试窗口当独立数据来源——**跑、录、测、归档，不自判，不修生产码**。

---

## 0. 前因后果（为什么单独开一个窗口干这件事——按时间线读）

你是测试窗口，没有本会话上下文，所以这段必须读完整。整件事的因果链是：

1. **CLOSURE 宣告 FREEZE（2026-08-31 上午）**：脚本化验证全绿——gate 447/0、假完成 0、静默丢失 0、无限循环 0，F1×4 全部"有界已声明"。裁决 = **CORE FREEZE**（v0.2.18）。理论上 Core 已"冻结可交付"。
2. **用户 90 分钟盲测打穿冻结**：真人交互（.133 主机，3703 行日志）暴露脚本没覆盖的东西——**24 连败吸引子（RC52）**、裸 ERROR 直接刷进终端、结果截断（用户原话"我需要猜你的结果"）、introspect 误路由。逐轮归因成功率 **28.6%（10/35）**。**CORE FREEZE 随即 SUSPENDED**。
3. **P0-ATTRIBUTION-01（零修复纪律）立项查三问**：想重放盲测做归因时，遭遇 **harness 三版失真**——①管道 stdin 非 TTY → 交互审批全拒 → 12/12 瞬失败 + EOF 忙等刷 56MB；②`script` PTY 输入消费竞争 → 191MB 刷屏；③resume 恢复**旧 goal** → 重放输入全部错位。RC52 归因卡在 **EVIDENCE GAP**。
4. **P4-REVALIDATION-01 定案 PTY driver（Node 01）**：真实 PTY + send→wait_turn 定步调 + resume 语义显式——三陷阱规避后矩阵跑通，RC52 归因闭合：**decision 层（RC47 族，planner 不知道任务已完成）主因 + 会话污染放大器（fresh 40% → 污染 100%）**，CAUSE LIKELY。**但这归因是执行窗口自跑自判的**——这正是你存在的理由。
5. **v0.2.20 修复批落地（Node 05/06）**：RC51-A（裸 telemetry → `~/.config/hearth/diagnostics.log`，终端零裸输出）、RC51-B（终态产物清单投影，不再"猜结果"）、RC53（bash 内部工具名结构化提示）。gate 459/0，双 VM 对齐。**注意：decision 层（RC52 主因）有意未修**——修复授权在 Node 13 批。
6. **评审窗（砺）查出 C 条件阻断项**：执行窗口矩阵的 C resume 侧 session id 提取正则（`run_rc52_matrix.py:99`）恒失效 → resume 的是空 id → **C 侧 67% 旧数据全部作废**。仪器修复单（Node03 指令附录 A）改由**你**实施（v1.1 顶层裁量：执行窗口已收工，仪器属 dev 侧不在冻结区；单独 commit、diff 留痕、交砺复核后方可跑 C）。
7. **窗口分离裁决（本指令书的由来）**：campaign 是对 v0.2.20 的**验收行为**。执行窗口自己修的自己验 = 既当运动员又当裁判，违反本项目"审计工具须被审计"红线。所以 **campaign 由测试窗口执行，执行窗口不得自任 judge**；你只交数据与现象，**正式归因判读 = 砺·评审，验收 = 顶层**。

**你测的是两个具体问题（不是泛泛"找 bug"）**：

- **Q1（修复效果验收）**：v0.2.20 是否真的消除了 P0 盲测暴露的**用户可见缺陷**——裸 ERROR 刷屏 / 需要猜结果 / introspect 误路由？
- **Q2（RC52 复现率定量）**：RC52 吸引子在 v0.2.20 下的逐轮口径复现率是多少？（有效基线：fresh **40%** / contaminated **100%**；~~resume 67%~~ 已作废待你补跑。）

**预注册预期（falsifiable prediction，防止误读结果）**：v0.2.20 只修了 Projection 层与误路由提示，**有意未动 decision 层**——所以 A fresh 条件预期**仍在 ~40% 量级、不会显著下降**，这不是"修复失败"，而是归因模型的因果预测。你的任务是把这个数**测准**，作为 Node 13 修复批的因果依据；B 条件同理预期维持 100%。若实测与预期显著偏离（如 fresh 升到 80% 或降到 10%），先按 §6 查 DRIVER-INDUCED 与 provider variance 再定层——**那是大发现，如实报告，不得凑数重跑**。

---

## 1. 环境与版本锁定（T0，先做，sha 不符立即停）

- **环境 = `.131`**（`ssh wutao@192.168.220.131`）——沿用 Node03 指令 v1.1 裁决（driver 需真 PTY + 真 LLM + hearth binary）。执行窗口在此树有在制品：**只跑，不改它的树**；你的改动只限 §9 授权的仪器文件且单独 commit。
- 版本验证（结果记入报告头）：

```bash
bash -lc 'hearth --version'            # 必须输出 0.2.20（bash -lc 模拟交互，防 PATH 分叉假结论）
sha256sum /usr/local/bin/hearth        # 前 16 位 = fc39c27c…；不符 = 停，报顶层，禁自作主张重建
```

- 运行环境模板（Node03 指令 P-2）：

```bash
unset HEARTH_URL
export HEARTH_CGROUP_BASE=/sys/fs/cgroup/hearth
source ~/.cargo/env
```

- **cgroup 探明（P-4，先测再定）**：`run_rc52_matrix.py` 现设 `HEARTH_ALLOW_NO_CGROUP=1`——先在上述模板下去掉该开关跑 1 个 A 样本：若 bash/glob/grep 可用、无 fail-closed 打死 → **正式跑不带此开关**；确不可用才保留，且必须在 results.json 元数据与报告头显式标注"无 cgroup 环境"。
- **目录隔离**：Node 03 复现数据 → `.131:~/fa/p4-rerun/`（Node03 指令已定）；本 campaign 数据 → `.131:~/fa/campaign/`（新建，与 `~/fa/p4/`、`~/fa/p4-rerun/` 物理隔离）。每跑 results.json 头部写明 tag / commit / binary sha256。
- **不可触碰区**：`crates/` 全部源码（发现缺陷**只登记不修**——修复权在执行窗口 Node 13 批）；`metrics.yaml` 口径定义（冻结 v1.0——改定义=升版本，旧数据不得与新数据混表）；`~/run_gate_r2c.sh`。
- **T0 准备件**：盲测黄金集全量剧本（41 条输入行 = 35 逻辑 run）尚未落成独立文件——从原始盲测日志 `release/手工测试v0.2.18.txt`（本机，3703 行；**release/ 不随源码同步，VM 上没有**）按"`hearth>` 行"口径提取，落成 `.131:~/fa/campaign/golden/replay41.txt`，sha256 记入报告头。P0 重放前缀（R01-R15，13 逻辑 run）已在源码树 `docs/data/p0-attribution-20260831/node00-baseline.md` 冻结（含剧本 sha256；.131 上路径 = `~/codex/docs/data/...`）。

## 2. 任务序列（T1 前序 → T2 campaign → T3 attractor）

### T1 — Node 03 独立复现（前序指令，先闭环）

按《P4-Node03独立复现测试指令 v1.1》执行：A/B 复现 ×10 → 附录 A 仪器修复（单独 commit + diff 全量贴报告 + **交砺复核通过后方可跑 C**）→ C 补跑 ×3。前置检查 P-1~P-5 全绿才许开跑。

**其 13 跑结果 = 本 campaign 的矩阵块 A，不重复跑**——T2 的 ≥30 runs 已把它计入。

### T2 — Node 11：SIMUSER Controlled Campaign（≥30 runs，含 T1 的 13 跑）

**纪律**：campaign 内**禁止边测边改**（不改剧本/不改口径/不换版本）；测试窗口只做 **run → capture → detect → archive**；指标稳定后是否扩到 100 runs 由顶层批，勿自行扩。

| 块 | 数量 | 内容 | 来源 |
|---|---|---|---|
| **A. RC52 矩阵** | 13 | A fresh ×5 / B contaminated ×5 / C resume ×3（修复后补跑）——RC52 复现率定量（对照 40% / 100% / C 待定） | **T1 已完成，直接引用 `~/fa/p4-rerun/` 数据** |
| **B. 盲测黄金集回放** | ≥10 | `replay41.txt` 逐条回放（分 3-4 会话），**逐条标注 复发/未复发**——Q1 的直接验收 | PTY driver 手工驱动 |
| **C. persona 分布对齐** | ≥7 | 按七类 persona 占比采样（规划书 v1.1 §2.1：继续型 26.9% / 纠偏 10.4% / 状态 8.1% / 高收益探针 5.6% 放大等），**三会话形态（fresh repl / resume / chat 单目标）必须都有覆盖** | PTY driver |

时间预算提示：Agnes 单轮 28-55s 起；B 条件前缀重放 13 轮 × 每轮完整 LLM 循环，单跑可达数十分钟——排串行窗口，勿并行抢状态目录。

### T3 — Node 12：Failure/Attractor Campaign（七场景 × 三样本）

专项追：**RC52 / RC48（false stop）/ QA stall / goal revision churn / projection leaks / tool misroute**。

| # | 场景 | 构造方法 |
|---|---|---|
| 1 | Fresh | 全新 REPL，单任务直入 |
| 2 | Long-session | 单会话 ≥15 轮连续交互 |
| 3 | After-stall | 先制造 T4 停滞（重复/含糊输入），再给正常指令 |
| 4 | After-compaction | 长会话喂料直至 archive 落盘（字节+mtime+文件数判定），再继续交互 |
| 5 | Resume | `hearth resume <id> "<text>"`（T3 陷阱：恢复**旧 goal**，text 只入队消息——goal 状态以 goal_revision 事件为准，勿假设；用修复后的 id 提取） |
| 6 | Continue | 任务完成后发"继续"类输入（RC52 主形态） |
| 7 | Failure-diagnostic | 主动制造不可满足任务，观察失败投影与恢复路径 |

**每一类必须保留三样本：successful case + failure case + counterexample——禁止只保存失败样本**（只有失败样本无法区分"缺陷"与"这任务本来就会失败"）。

## 4. 驱动与检测（工具用法）

- **driver**（`~/codex/tools/simuser/driver.py`——**必须用它，勿用管道/script 替代**，三陷阱会全部复发）：

```python
import sys; sys.path.insert(0, "/home/wutao/codex/tools/simuser")
from driver import HearthDriver, DriverConfig

d = HearthDriver(DriverConfig(
    ["hearth", "repl"], cwd="/home/wutao/<run_workspace>",   # 每 run 独立工作目录，隔离状态污染
    env_extra={"HEARTH_CGROUP_BASE": "/sys/fs/cgroup/hearth"}))
d.start()
d.send_and_wait_turn("你的输入")   # 定步调：发出一行必须等 turn 终态（Task completed / Task failed），未等完绝不发下一行
d.close()
```

  要点：单轮超时 600s（记 timeout 事件）；每个 run 独立 cwd；历史教训——driver `expect()` 已是窗口相对搜索（修复过 banner 提前命中 bug），**别改回全量搜索**。

- **analyzer**（13 检测器，每份 log 必跑）：

```bash
python3 ~/codex/tools/simuser/analyzer.py <session.log>          # 人读
python3 ~/codex/tools/simuser/analyzer.py <session.log> --json   # 机读，随 run 归档
```

  自检锚点：盲测原日志应出 ~225 findings（全命中已知缺陷）；干净样本应 0 findings。若偏差大 = 检测器漂移，停。
- **planner dump**：`HEARTH_DEBUG_PLANNER_INPUT=1` → `~/.config/hearth/debug/<session_id>/` 三件套（prompt 快照 / TaskGraph sig / 压缩摘要）——每 run 整目录拷贝归档。注意 v0.2.20 起 tracing 已收敛到 `~/.config/hearth/diagnostics.log`（终端不再裸输出——**终端干净本身 = Q1 验收点之一**）。

## 5. 指标口径（metrics.yaml v1.0 冻结——违反 = 数据作废）

| 指标 | 唯一口径 | 禁止 |
|---|---|---|
| success_rate | **逐轮归因**：run 边界 = 用户输入行（driver send 事件）；分子 = 该轮**首个终态**为 completed 的 run 数 | `grep -c "Task completed"`（重复计+跨轮串）；字符串口径与逐轮口径混用 |
| consecutive_failure | 连续 failed 的 run 数（同边界） | 按失败关键词计数 |
| compaction_occurred | **字节数 + mtime + 文件数**三项判定 archive 落盘变化（`compacted.jsonl` / `archive/<sid>.jsonl`） | `wc -l` 行数（末行无换行即系统性少计——RC49 已证伪） |
| revision_churn | goal_revision 事件数 / 执行轮数 | — |
| attribution | 每条 finding 归七层之一：mechanism / decision / model / provider / environment / **DRIVER-INDUCED** / detector | 未归层的 finding = 无效 |
| 版本可比性 | 成功率只与 v0.2.8+ 比 | 与 v0.1.x / v0.2.0-0.2.5 混比（终态文案不同，0/0 会被读成"全失败"） |

## 6. DRIVER-INDUCED 判定规则（防"测出来的缺陷其实是测法不对"）

P0 三版 harness 失真（56MB/191MB/输入错位）+ C 条件空 id 的教训已固化为规则：

1. **approval_denied 默认归 DRIVER-INDUCED**——除非有 driver events 对照证据证明审批通道正常，才可升格 Core defect。
2. 每条 finding 必须带 driver events（send/terminal/timeout 时序）对照；无 events 佐证的环境/驱动侧嫌疑 = 归 DRIVER-INDUCED 或重跑。
3. 疑似驱动问题：同剧本同环境**先重跑一次**再定层；重跑消失 = DRIVER-INDUCED，如实记录（不算你的失败，算数据质量）。
4. **session id 以日志/事件内实际出现的 uuid 为准**——提取逻辑必须有非空断言，空 id 即该 run 作废重跑（C 条件教训）。
5. 清理进程用 `[h]earth` 转义（防 pkill 自匹配自杀）；每 pattern 单独 exec；勿在 campaign 目录外删除任何东西。

## 7. 报告与归档（判读分离）

- **每 run 产物四件套**：终端侧全量 log + driver events（json）+ planner dump（`~/.config/hearth/debug/<session>/` 整目录）+ analyzer `--json` 输出。
- **归档**：`.131:~/fa/campaign/`（原始）→ 同步本机 `docs/data/p4-campaign-20260831/`（报告引用此路径）。
- **数据报告**：`docs/hearth-p4-campaign-testwindow-report-v1.md`——必含：①版本+sha 头（tag/commit/binary sha256）；②Q11 声明；③逐轮归因表；④RC52 复现率对照表（A/B/C × v0.2.20 vs 基线 40%/100%/C 待定）；⑤七场景 × 三样本索引；⑥七层归因汇总；⑦DRIVER-INDUCED 披露清单；⑧黄金集逐条 复发/未复发 表；⑨仪器改动 diff（若实施了附录 A 或新发现修复）。**你交数据与"测试窗口视角的初步结论"——正式归因判读 = 砺·评审，验收 = 顶层，你不出具 PASS/FAIL 裁决。**
- **Q11 单主体声明（报告头强制）**：persona 语料来自**单一人类主体**——禁止"真实用户成功率"表述，只允许"该主体语料下"。28.6% 盲测基线同为该主体人工口径，只作叙事参照，**不作统计基线**。
- **异源绑定**：每 run events 记录 driver_model 与 target_model（scripted 回放 / 智谱 / Ollama qiyuan-8b 均可作 driver 侧；同源 Agnes 仅作标注对照组）。

## 8. 验收标准（Weak / Strong 两层——供顶层验收你，非你自判）

| 层 | 标准 |
|---|---|
| **Weak（最低合格）** | ≥30 runs（含 T1 的 13 跑）全部执行且四件套归档；每份 log 跑过 analyzer；逐轮归因表完成；每条 finding 归层；DRIVER-INDUCED 单列；Q11 声明在位。**发现多少缺陷不影响合格判定——求真优先，如实披露即可。** |
| **Strong（达标）** | Weak 全部 + RC52 复现率定量（A/B/C 逐轮口径，与基线同表对照，C 补跑闭环）；七场景 × 三样本齐全；黄金集逐条 复发/未复发；counterexample 可复跑（复跑命令写进报告）；对 Q1/Q2 的数据支撑完整。 |

## 9. 红线（违反即作废）

1. **禁改生产码**：`crates/` 零改动；发现缺陷**只登记不修**——修复权在执行窗口 Node 13 批（零修复纪律，与 P0 同款）。
2. **仪器改动边界**（Node03 指令 v1.1 裁量）：`tools/simuser/` 的 driver/analyzer/runner **授权你按附录 A 实施**修复；附录 A 之外的新问题**只登记不修**；确需新修 = 单独 commit + diff 全量贴报告 + 交砺复核后方可使用其产出的数据。`metrics.yaml` 口径定义不在此授权内（改定义 = 升版本，须顶层批）。
3. **禁边测边改**：campaign 中途不改剧本、不改口径、不换版本；剧本变更 = 该 run 作废重跑。
4. **禁自建口径**：success_rate / compaction 判定一律按 metrics.yaml v1.0。
5. **禁采信自述**：终态以日志终态标记为权威；completed 以产物 + 逐轮口径为准；"我已验证"是零证据陈述。
6. **禁混表**：v0.2.20 数据不得与 v0.2.18 盲测数据直接混比（版本不同 + 口径不同——盲测 28.6% 是人工口径）；`~/fa/p4/`（执行窗旧数据，C 侧已作废）、`~/fa/p4-rerun/`（你的复现）、`~/fa/campaign/`（本 campaign）三目录物理隔离，禁止互相覆盖。

## 10. 依赖与排期接口（你完成后发生什么）

- **T1（Node 03 复现）先行**；T2/T3 campaign 不依赖 Node 10（Stage 2 联合分布采样另行派发）——黄金集 + 分布对齐 + 矩阵引用用现有工具即可执行。
- 你的 campaign 报告 = **Node 13 修复批的直接输入**（修复优先级 P0：RC52 confirmed root cause → fixture 单测；截断标记规范等）。
- Node 13 出 **v0.2.21** 后 → Node 14 独立复测 campaign 将再次派发测试窗口（沿用本指令书框架，版本绑 v0.2.21；**那时 fresh 侧失败率预期才应显著下降**——decision 层修复后）。
- 验收链：砺·评审判读 → 顶层验收 → Node 15 CORE FREEZE CANDIDATE package 组装。
