# Hearth P0-ATTRIBUTION-01 长程施工总包 v1.0（定稿）

## 盲测反证定向复现与归因（零修复纪律）

**日期**：2026-08-31（v1.0 定稿：砺复核四点 + 两条一手数据补充全部采纳——判级
口径化 / fresh 组重跑 / 截止点定死 run-012 / 主机差 / 摘要链假设降级为放大器 /
"继续"型入口 + RC47 族判定实验。评审记录：`评审：P0-ATTRIBUTION-01 与拟真测试台
（砺·评审 复核+补充）.md`，其关键一手数据经本窗实测复核属实）
**执行对象**：砺·执行
**性质**：定向复现 / 归因 / 判级，**不是修复批**
**当前基线**：v0.2.18（tag v0.2.18；.133/.131 binary 已对齐 0.2.18）
**上游输入**：
- `release/手工测试v0.2.18.txt`（用户盲测原始日志，3703 行，一手证据；**盲测
  主机 = .133**，原文 L4 `ssh wutao@192.168.220.133` 实锤）
- `docs/hearth-v0.2.18-blind-test-report-v1-Claude.md`（现象层报告）
- `docs/Hearth v0.2.18 独立验证阶段评审报告-chatgpt.md`（P0-A/B/C 框架）
- `docs/Hearth CORE FREEZE 顶层评审总览-砺.md`（RC52-54 登记）
- `.131:~/li_probe/`（fresh 侧对照数据——**⚠️ one-shot 形态 + 无 planner dump，
  按 §1.2 不可直接复用为 Condition A，仅作旁证**）

---

# 0. 使命

CORE FREEZE 确认已 SUSPENDED。唯一可能翻盘的项是 **RC52（会话级持续失败吸引子：
同一 polluted REPL 会话连续 24 轮失败，T4 同图停滞 27 次；fresh 同任务 2/2
completed）**。本总包不修任何东西，只回答三个问题：

> **Q-A**：裸 `ERROR agent_core::r#loop` 泄漏的完整路径是什么（源码调用点 →
> level → destination → 终端可见性）？
> **Q-B**：24 连败的因果是"会话状态污染"吗？污染载体在哪一层（压缩摘要 /
> anchor / state）？
> **Q-C**："内部完成但用户不可见"（BUG-012）的截断点在哪？`give_up` 字样与
> `Task completed` 共存（砺 probe 附带观察）是否为拦截链可观测性缺口？

---

# 1. 全局纪律（本包与既往总包的差异条款）

1. **零生产代码修复**。发现任何缺陷只登记 + 最小复现，不修（含 RC53/54）。
2. **允许的唯一代码改动 = env-gated 观测代码**（仿 `COMPACT_DBG` 先例）：
   如 `HEARTH_DEBUG_PLANNER_INPUT=1` 时 dump planner 收到的原始 prompt 快照、
   TaskGraph fingerprint、压缩摘要全文到 session 目录。**不得改变任何控制流
   行为**；dump 关闭时零开销零输出。
3. **采集对象必须含 planner 实际输入**——这是本包与既往所有实验的最大差异。
   只看终态无法区分"任务难"与"输入退化"；fresh vs polluted 的因果判定靠
   **同一请求在两种会话状态下 planner 收到的东西的 diff**。
4. 沿用五件套交付格式：①原始复现证据（命令/环境/原始输出/时间戳/session id）
   ②源码定位（mechanism/decision/model/provider/environment 五层归因）
   ③反事实对照 ④invariant mapping（原 CLOSURE claim → 新观察 → 冲突与否 →
   reopen F0 还是新增 F1）⑤最终 disposition（CONFIRMED CONTRADICTION / NEW F0 /
   NEW F1 / EVIDENCE GAP / FALSE POSITIVE / DEFERRED——禁止模糊措辞）。
5. 真机在 .131；.133 做复验；双 VM binary 已 0.2.18 对齐（本窗已实测），Node 00
   复核即可。Agnes 供应商；budget/deadline 沿用盲测条件以便可比。

---

# 2. Node 00 — Baseline / 环境快照 / 重放基线

**（v1.0 承接砺 §1.4/§1.3）环境快照必须先固定，三组实验统一复用**：

| 项 | 要求 |
|---|---|
| **主机** | **统一用 `.133`**（原始盲测主机，原文 L4 实锤；砺 probe 在 .131 属机器差，禁止混用） |
| binary | `--version` + **sha256**（0.2.18 (unknown) 不足以锁定） |
| env | `HEARTH_ALLOW_NO_CGROUP` / `HEARTH_URL` 必须 unset / `HEARTH_TASK_TIMEOUT_SECS` / dump 开关——逐项记录，缺一项即漂移 |
| 运行参数 | budget / deadline / cwd / repl 形态（与盲测对齐） |
| provider | Agnes / agnes-2.5-flash + config.toml 快照 |
| 时间口径 | UTC；Agnes 历史延迟 28-55s 波动 → **错峰执行 + 每条件 ≥3 跑** |

- **重放截止点定死：run-012 结束处**（砺一手数据，本窗已复核：首个 T4 事件在
  run-003 且该轮成功，run-004~012 连续 9 轮成功零 T4——若按"首个 T4 触发点"
  截断，重放出的是历史上并不失败的会话，B 组失去意义）。
- 从 run-001…run-012 提取用户输入序列与终态，形成逐字重放剧本（hash 归档）。
- 输出：`docs/data/p0-attribution-20260831/node00-baseline.md`。

---

# 3. Node 01 — Q-A：裸 ERROR 泄漏路径定位

1. 源码定位 `do_plan_inner failed` 的 tracing 调用点（level、target、是否
   `#[cfg(test)]` 外的生产路径）；
2. 确认 subscriber/格式化配置：该 ERROR 走 stdout 还是 stderr、是否绕过
   Projection 层、SSH 终端可见条件；
3. 真机复现：构造 T4 stalled 场景（可重放盲测 run-013 前缀），保存原始终端
   输出（含裸 ERROR 行 + 格式化 ✗ 行的先后关系）；
4. **判级口径（只写怎么判，不写判成什么——承接砺 §1.1）**：
   - 若实测满足"错误输出被投影为成功"（错误被渲染成成功态）→ **NEW F0**，
     立即上报顶层；
   - 若仅泄漏而未被投影为成功 → 按 **Projection isolation + completeness**
     登记（F1/F2 由执行窗口按影响面判），并给出 CLOSURE §7 指标拆分修订建议
     （isolation / completeness / correctness 三行，RC51）。

---

# 4. Node 02 — Q-B：fresh vs polluted 对照（本包钥匙）

**（v1.0 承接砺 §1.2/§2.B 重写）**

## 0. 请求型矩阵（两种都要测，"继续"型优先）

砺一手数据实锤：**进入 24 连败的入口是"继续"型输入**（run-012 completed →
run-013 输入"继续你的提议吧" → failed；run-014 起才是元诊断追问）。元诊断失败
是**继发**不是入口。因此：

| 请求型 | fresh 侧 | polluted 侧 | 优先级 |
|---|---|---|---|
| **① 继续型**（"继续你的提议吧"，已完成+继续形态） | **新增，必须测** | 重放 run-013 | **最高（RC47 族判定实验）** |
| ② 元诊断型（读失败报告解释根因） | 已有 2/2（one-shot，不可直接复用） | 重放 run-014+ | 高 |

## 1. Condition A（fresh，必须重跑）

- **会话形态与 B 对齐（都用 REPL）**——砺 probe 是 one-shot（独立进程），
  与盲测的 REPL 长会话差了会话形态这个混杂变量，**旧数据降为旁证**；
- A 侧先跑 **N 轮非污染填充对话**（N = B 侧前缀轮数 12，内容为与主任务无关的
  中性问答），再发同一请求 → **唯一变量 = 前缀内容是否污染**；
- 两请求型各 ≥3 跑，全部开 `HEARTH_DEBUG_PLANNER_INPUT=1`。

## 2. Condition B（polluted，重放）

1. 逐字重放污染前缀 run-001…**012**（截止点定死，见 Node 00）；
2. 在污染会话中分别发出 ①继续型 ②元诊断型请求（与 fresh 侧逐字相同）；
3. 采集字段清单（两组统一，预先写死）：planner 原始 prompt 快照 / TaskGraph
   fingerprint / 压缩摘要全文 / anchor / goal_revision / T4 触发点 / Reserve
   状态 / 终态。

## 3. 判定矩阵（RC52 归因的最终判据）

| fresh 继续型 | polluted 继续型 | 结论 |
|---|---|---|
| **失败** | 失败 | **主因 = RC47 族（决策层：planner 不知道任务已完成）**，会话污染为放大器 |
| 成功 | 失败 | **主因 = 会话污染**，载体按 Node 03 定位 |
| 成功 | 成功 | 吸引子条件未复现 → EVIDENCE GAP，回 Node 00 检查重放保真度 |

（元诊断型同矩阵，作为第二维佐证。）

**每条件 ≥3 跑**（方差纪律：报 mean/min/max；Agnes 延迟波动 → 错峰）。

---

# 5. Node 03 — 污染载体定位（Node 02 数据分析）

对 fresh/polluted 的 planner 输入做逐层 diff，回答：

1. polluted 侧的压缩摘要是包含陈旧/退化内容？是否逐字重复（对齐盲测 16 行
   Debug 外泄现象）？
2. anchor/goal 是否指向已完成的旧任务（盲测 run-032"回复我一下情况"→ 旧
   ledger 步骤）？
3. T4 的 `last_graph_sig` 比较对象在 polluted 会话中为何反复相似——是输入
   退化导致，还是签名计算本身受会话状态影响？
4. **摘要链假设——已被时序数据否证为起因（v1.0 降级，承接砺补充 A）**：
   首次压缩摘要出现在 run-024（本窗实测 L2810），**晚于吸引子起点 run-013
   11 轮** → "摘要链 → 同构图 → T4 → 吸引子"作为**起因不成立**。降级为
   **放大器假设**保留（失败轮堆积 → 上下文膨胀 → 触发压缩 → 摘要质量差
   （60 字墓碑 + Debug 格式，L2810/L2843 同文重复注入实锤）→ 更难恢复）。
   Node 03 分析资源按此优先级：**RC47 族决策层路径（Node 02 判定矩阵） >
   会话状态残留 > 摘要放大器**。
   另：**T4 是必要非充分条件**（run-003 曾 T4 而成功）——不得把 T4 当作
   吸引子的定义特征。
5. 输出：污染载体判级表（哪一层、什么证据、置信度）。

---

# 6. Node 04 — Q-C：用户可见完成证据 + give_up-completed 共存

1. 复现 BUG-012（分析内容存在、用户只见 `✓ Done (N steps)`）：定位截断点
   （Projection 层缺分支？summary 替换正文？）；
2. 核查砺 probe 附带观察（fresh 探针日志 1 次 `give_up` 字样却 `completed`）。
   **判级口径（承接砺 §1.1，先定性再判级）**：
   - 是报告文本命中 → **FALSE POSITIVE**，须写出反证；
   - 是拦截行为却无痕迹 → 登记拦截链可观测性缺口（F1/F2 按影响面判）；
   - 两者都排除 → **EVIDENCE GAP**。
3. 提出验收模型修订建议：completion 四层
   （false_completion / completion_evidence / completion_projection /
   user_verifiable_completion）中后两层纳入 Reliability Matrix 的具体测法。

---

# 7. Node 05 — RC53/RC54 定位（只定位不修）

- RC53：`introspect` 被当 bash 拼接（盲测 5 次，exit 127）——定位 tool
  selection/序列化路径的具体分支；
- RC54：`apply_patch` 空白/缩进脆弱匹配——最小复现 fixture；
- 各给出修复方向一句话（供下批立项），不修。

---

# 8. Node 06 — Final Report

五件套格式，含：
- F0/F1 重新判级表（哪些 CLOSURE claim 被 reopen、哪些维持、哪些新增）；
- RC52 因果结论 + 污染载体 + 修复方向建议（供 RC52-FIX 批立项）；
- RC51 指标拆分修订案（isolation/completeness/correctness 三行）；
- 完整 provenance（重放剧本 hash、planner 输入 dump 归档 `docs/data/p0-attribution-20260831/`）。

---

# 9. STOP / 成功标准

**STOP**：①重放无法稳定复现污染态（重放器本身失真）→ 停在证据边界；②发现
F0 级新事实（假完成/沙箱绕过）→ 立即上报顶层；③需要改生产控制流才能继续归因
→ STOP（归因不需要改行为，遇到即说明采集方案错了）。

**成功标准**：Q-A/B/C 三问各有 evidence-backed 答案；RC52 从
"observed attractor" 升级为"归因完成、可立项修复"；零生产代码变更。
