# Hearth P2-MEMORY-CONTEXT-01 评审包 v1（整合版）

> **用途**：单文件自足评审材料——供砺·评审窗口与外部 AI（ChatGPT）交叉评审。
> **执行窗口**：砺·执行　**日期**：2026-08-31　**代码终态**：v0.2.16（tag v0.2.16，HEAD `253fcd1` 含 docs；代码基线提交 `0e4a9d2`）
> **结论**：**PASS WITH DEVIATIONS**（偏差与 OPEN 见 §15；升级与否由评审/顶层裁定）
> **核验纪律**：所有锚点均经 grep/实测复核；§17 给出评审方可独立执行的核验命令清单。禁止采信 commit message 与"已验证"自述。

---

## 1. Executive Summary（四问答案）

| 问 | 答案 | 证据等级 |
|---|---|---|
| **Q1** 32k 阈值过早压缩？ | **是**——est 口径 32k 字符 ≈ **9,009 tokens = Agnes 512K 的 1.8%**；且测量仪器本身虚增（Debug 枚举包装 + UTF-8 字节计数，1.39-2.49×） | confirmed（API 权威实测） |
| **Q2** Compaction 是失忆主因？ | **修正**——单 run（hearth chat 主路径）**压缩是死代码**：全部工具交换 push 进唯一 Turn 0 → history.len() 恒 1 → keep_from==0 守门恒 false。单 run 真正窗口约束 = `MAX_HISTORY_MSGS=40` 静默切片（无标记无归档）。压缩历史上只在 REPL 多轮生效 | confirmed（单元红 + 真机 A1） |
| **Q3** 阈值调整后 Fact 仍丢？ | **不会**——INV-M01 可失败 fixture：state 层 7 类事实不在压缩路径；旧轮原文先归档后折叠（B 档 grep 可恢复）；真机 A3：1 次压缩 + RESULT 12 数字精确正确 | confirmed |
| **Q4** 参数标定还是架构？ | **参数标定（provider-aware）+ 复用既有 archive（情况 A）**——不建新 Fact Store（M8 遵守） | confirmed |

**Deviation**：① Node 09 resume 前置跑未含压缩（deadline 短于压缩点）——压缩+resume 由 Node 14 Task A 与 Node 09 分证合链，非单跑闭环；② B 档（模型 grep archive）真机 0 使用；③ "继续"续轮 planner give_up（决策层，RC47 候选，登记移交）。

## 2. Token Calibration（Node 01，批-3 仪器纪律）

先量化仪器：`estimate_chars`（context.rs:179 旧行）= `format!("{:?}", m.content).len()` = **Debug 枚举包装 + UTF-8 字节**，双重虚增。实测（Agnes API `usage.prompt_tokens` 权威）：

| 语料 | naive chars | est 口径 | tokens | chars/token | est 虚增 |
|---|---:|---:|---:|---:|---:|
| system（constitution） | 1,430 | 3,562 | 1,063 | 1.35 | **2.49×** |
| code（loop.rs 500 行） | 14,028 | 18,508 | 5,356 | 2.62 | 1.32× |
| 中文对话 | 673 | 1,036 | 560 | 1.20 | 1.54× |
| 英文 tool result | 809 | 826 | 556 | 1.46 | 1.02× |
| mixed | 631 | 766 | 552 | 1.14 | 1.21× |
| TaskGraph/Continuity | 792 | 921 | 552 | 1.43 | 1.67× |
| **COMBINED** | **18,373** | **25,629** | **7,215** | **2.55** | **1.39×** |

- **32,000（est）≈ 9,009 tokens**；按 naive 理解 32k chars ≈ 12,566 tokens。
- 中文 workload 双重放大（字节×3 + chars/token 低）——压缩点最提前。
- provider usage 为唯一权威口径，本地估算仅交叉验证（批-3）。脚本：`docs/data/memory-context-20260830/measure_tokens.py` 可复跑。

## 3. Compaction Audit（Node 02）

真实触发链：`build_messages`（loop.rs:2035-2039 唯一生产调用点）→ `maybe_compact`（context.rs:189）→ est ≥ 阈值 → 保留最近 2 轮 → **先 `archive_compacted_turns` 落盘再折叠**（best-effort）→ 规则式摘要（不调 LLM，goal 60 字截断 + 工具清单 + 写盘文件）→ 摘要注入 grep 检索提示。

- 单位 = Debug+bytes 字符；provider-unaware（llm-cn:412 Hunyuan 32_000 写死、**无 Agnes 条目**）；32,000 = 历史遗留值。
- `/v1/models` 实测无 context_length 字段 → 窗口自动发现不可行 → **批-2 "已知表+覆盖+比例"为唯一可行路线**（证实）。
- **RC40 单列**：`context_fill_pct`（loop.rs:1338-1346 旧行）= 距压缩阈值距离冒充"窗口填充率"——改阈值不治；野外证据：用户实测 83%→65% 回落当场质疑。
- **重大发现（本包核心）**：单 run 压缩死代码（§5）。

## 4. 单 run 压缩死代码（Node 02 执行中重大发现）

**控制流实测**：
1. `record_turn` 生产调用点仅 `loop.rs:4379`（run 启动 `Turn::new(0)`）；
2. `record_tool_exchange`（loop.rs:2307，每 Act 步后 :3464 调用）把消息 push 进 `history.last_mut()`——**永远是同一 Turn 0**；
3. `maybe_compact`：`keep_from = len-2 = 0` → `return false`。

**后果**：hearth chat 主路径压缩从不触发（真机 A1：12×1500 行 seq ≈ 44k est，0 压缩，17 步 completed）；实际窗口约束 = `MAX_HISTORY_MSGS=40`（loop.rs:2200，静默切片无标记无归档——消息留在 state.history 未销毁但 LLM 永不可见）；`context_fill_pct` 单 run 无限涨到 100 失真。RC40 野外证据（83%→65%）来自 REPL——自洽。

**处置**：属 Memory 层 turn 语义对齐（一次交换 = 一个 Turn，与 REPL 一致），不触 STOP-2/6（不动 TaskGraph/TaskGoal/Completion/Terminal）。先红后绿：单元红（修复前恒 false）+ A1 真机 → 修复 → 单元 + e2e（`test_single_run_compaction_e2e`）绿。

## 5. Node 03/04 — BEFORE/AFTER 与持久化判定

- **S-3 存储位置分类前置**：goal/original_goal/goal_revision/constraints/acceptance_criteria/scratch = **state 层不在压缩路径**；对话原文 = history；原文全量 = archive。**禁止把不在 history 的字段判成"压缩后丢失"**——零假阳性。
- **S-7 INV-M01 确定性 fixture**（可失败）：
  - `test_inv_m01_fact_survives_compaction`：12 项清单逐项判级——state 层 preserved（含 original_goal immutable 锚 / acceptance_result / last_failure_class）、旧轮原文 lost-but-archived（B 档断言 archive 含 `FACT-i` 原文）、摘要 reconstructed（goal_short/工具/写盘）、检索提示在场、近 2 轮 A 档保留、旧轮 tool_result 离开 context。
  - `test_inv_m01_loss_mode_without_archive`：archive 写失败（best-effort warn）→ 原文事实确实消失——**fixture 能失败的确定性证明**（断言不能失败 = 断言不存在）。
- **Node 04 判定 = 情况 A**：`archive_compacted_turns` 已满足持久化（unit 级 + 真机 compacted.jsonl 落盘增长 934KB→1017KB 实测），**不新建 Fact Store**（M8）。S-2 边界：archive 落盘实测过（非仅"代码里有调用"）；chat 路径 session_id 绑定晚于首次压缩 → 写入共享 `compacted.jsonl`（隔离弱化，OPEN 登记）。

## 6. Node 05/06 — A/B 配对实验

12 命令 seq 长任务（criteria 冻结：`file: RESULT.txt nonempty`），**A/BAB 交错** 5 配对（S-10），每跑记时间戳。A = 32k 默认；B = 64k（延迟压缩代理；生产实现走 provider-aware，实验测"压缩时机效应"）。

| 组 | 终态 | RESULT 正确（确定性比对） | 步数 mean | tokens mean | 压缩归因 |
|---|---|---|---|---|---|
| A×5 | 4 completed + 1 failed(give_up) | **5/5** | 32.2 | 129,217 | A1×3, A3×1 |
| B×5 | 4 completed + 1 failed(stalled) | 4/5（B1 模型 11 步早夭） | 34.4 | 140,614 | B3×2, B4×2 |

- 压缩归因用 **archive 行 created_at 时间戳**（UTC 归一）——日志 grep INFO 被过滤不可用（教训记录）。
- 均值接近、方差大（A steps 14-69 / B 11-67，model 层）——按 §29 只报 mean/min/max，**不宣称阈值显著效应**（批-4 层归纪律：只消费 mechanism/decision 层稳定跑次）。
- 层归单列：A1 failed = mechanism 正确（Reserve 零和时序）+ model（GiveUp 前未写 RESULT）；B1 = model 早夭。
- **诚实标注**：A1/B3/B4/B5 模型重复 seq 38-55 次（违反"不得重跑"指令）——这些跑的 correctness 部分来自重执行；**clean 样本 = A2/A3/A5/B2（seq 恰 12 次）+ A3 含压缩**。

## 7. Node 07 — Retention 分档（S-6）

- **A 档（上下文内保持）**：state 层事实 100%（INV-M01 fixture 确定性）；真机 A3 clean 样本实证。
- **B 档（经 archive grep 可恢复）**：机制存在（compacted.jsonl 落盘 + 检索提示注入）但**真机模型 0 次真实使用**（4 个压缩跑 grep 计数全 0）——提示存在 ≠ 检索成功，OPEN。
- fact/verification retention 9/10 精确正确（A/B 档混合口径，clean 与 re-run 样本分开列示）。

## 8. Node 08/09 — Continuity 与 Resume

- **Node 08**（REPL 多轮 + 阈值 800 + "继续"）：三文件跨轮保持（f1=A1/f2=B2/f3=C3 独立验证）、**无 GoalMutation**（修 B 语义保持 ✓）；**发现**：续轮 planner give_up 于"已完成+继续"形态（模型重写文件后 give_up——决策层缺陷，**RC47 候选，登记移交不修**——批-7 范围纪律）。
- **Node 09**（deadline 13 步中断 → `hearth resume <uuid>` 空 goal）：**6 步完成、seq 重跑 0 次、RESULT 12 数字精确正确、re-teach count = 0**（M6 ✓）。小项：banner session id 显示截断，resume 需完整 UUID（UX 登记）。

## 9. Node 10 — 静默截断清单（10 处分类，第一阶段只分类）

| # | 常量 | 位置 | 对象 | 标记 | 分类 |
|---|---|---|---|---|---|
| 1 | 200 | codex-cli/render.rs:82 | 显示预览 | 显示层 | SAFE |
| 2 | 200 | loop.rs:3672 | RAG query 拼接 | n/a | SAFE |
| 3 | 60 | context.rs:322 | 摘要 goal_short | 摘要语义 | SAFE（INV-M01 锁定） |
| 4 | 8000 | web.rs:17/126 | 网页正文 | **无** → 本轮补标记 | NEEDS_REVIEW→已修 |
| 5 | 6000 | constitution.rs:20 | constitution 注入 | 无 | NEEDS_REVIEW（defer） |
| 6 | 6000(4000+1500) | loop.rs:4892 | bash 输出入 history | **有** ✓ | NEEDS_REVIEW（archive 存截后文本——B 档上限钳制，登记不修） |
| 7 | 4096 | code-index:351 + 测试 mock | 能力声明 | n/a | **SAFE**（非截断路径——砺批-5 猜想证实） |
| 8 | 40 msgs | loop.rs:2200 | prompt 历史切片 | **无标记无归档** | **FACT_RISK**（超本轮边界，移交） |
| 9 | — | estimate_chars | 仪器虚增 | n/a | **FACT_RISK**（已修） |
| 10 | — | RC40 | 语义错位 | n/a | **FACT_RISK**（已修） |

## 10. Node 11/12/13 — Decision 与施工

**Decision = 参数标定（provider-aware）+ 复用 archive**（§34 纪律：阈值够用就停，不造 Memory System）。Node 12 施工清单（全部先红后绿）：

| 修复 | 内容 |
|---|---|
| turn 粒度对齐 | 一次工具交换 = 新 Turn（单 run 压缩复活；红=单元+A1 真机，绿=单元+e2e） |
| estimate 诚实计数 | naive chars；`legacy_estimate_chars` 保留（回滚通道专用）；honest counting 测试（naive=7 vs legacy=29 对照） |
| provider-aware 阈值 | 注入（env `HEARTH_CONTEXT_TOKENS` > caps.max_context_tokens）× `HEARTH_COMPACT_WINDOW_RATIO`(0.6) × 2.55；未知窗口回落 32k |
| 回滚通道（§30） | `HEARTH_COMPACTION_MODE=legacy` 整保旧行为（legacy est + 32k），保留一个 release cycle |
| RC40 | 改名 `compact_pressure_pct` + 动态分母 + status.rs:91/101 同步（S-1） |
| S-9 仪器 | `HEARTH_COMPACT_CHAR_THRESHOLD` env 覆盖（测试专用，生产默认零变化） |
| T4 跨轮修复 | Task B 红样本：`last_graph_sig`/`graph_stall_count` 按 run 重置（跨轮签名污染致 QA 轮 1 步误杀）→ 重跑 11 轮 0 stalled |
| web 标记 | 8000 截断补 `[N chars truncated]` |

**Node 13 = 情况 A，无新架构**：Compaction 不删 Fact（archive 先行）/ Resume 可恢复（Node 09）/ Continuity 从 state 层派生（未动）/ 零新事实源。

## 11. Node 14 — Long-run Real-machine

- **Task A**（calc2 受控失败 + 阈值 8000 强制压缩）：completed；独立复验 cargo test **1 passed**、修复在位、RESULT=N14_RECOVERED；archive 105→110 行。链路：write/test/failure/repair/verification/**compact**/completion（resume 由 Node 09 分证）。
- **Task B**（QA 11 轮 REPL）：首跑红样本（2 轮 1 步 stalled）→ T4 跨轮修复 → **重跑 0 stalled / 0 failed**，QA 全程不进 TaskGraph/recovery。

## 12. Node 15 — Final Gate 与回归

- **Final Gate**（`.133:~/run_gate_r2c.sh` → `/home/wutao/t_gate_mc_final.log`，v0.2.16）：**FMT=0 / CLIPPY=0 / RT4_SOLO=0 / TEST=0，443 passed / 0 failed**（基线 429 + FA01 8 + P2 6）。
- S-4 版本纪律遵守：bump + CHANGELOG **在 gate 之前**完成；Final Report §8 与 CHANGELOG 逐条对账。
- 回归覆盖：R2-C / W3-W4 / RC24 / W8 / P1-LTR / P1-TASK-TRUTH / P1-EXECUTION-DECISION / P1-FAILURE-ADAPTATION（评审包 §14 清单）/ N1-SBX；INV-ED01-*/INV-FA01-* 单测未回改全绿。

## 13. Memory Acceptance Hard Requirements 对照（M1-M8）

| 项 | 状态 | 证据 |
|---|---|---|
| M1 chars↔tokens 实测 | ✅ | §2 校准表（API 权威） |
| M2 压缩触发可解释 | ✅ | §3 审计 + 死代码发现 |
| M3 至少一次真触发 | ✅ | A/B 4 跑压缩（archive 归因）+ Task A 5 turn |
| M4 goal/Continuity 不丢 | ✅ | INV-M01 fixture（A 档 100%）+ Node 08/09 |
| M5 Fact/Verification 不丢 | ✅ | fixture + A3/A2/A5/B2 clean 真机（B 档机制在、0 使用=OPEN） |
| M6 Resume 零重教育 | ✅ | re-teach=0（Node 09） |
| M7 无"无限扩窗"掩盖 | ✅ | INV-M03：阈值=窗口×比例，未硬塞 512K |
| M8 无第二套事实模型 | ✅ | 情况 A 复用 archive；零新事实源 |

## 14. Telemetry 可见性（S-8）

新增观察项全部 **internal-only**（scratch/summary，不进 LLM 可见 body）：compact 计数（archive 归因）、goal_revision（既有事件）、retention（确定性比对）。`compact_pressure_pct` 为唯一 LLM-visible（改名后语义如实）。§23/24 只观察不设阈值。

## 15. OPEN / UNKNOWN / DEFER

| # | 项 | 等级 |
|---|---|---|
| 1 | B 档归档检索模型 0 使用（提示措辞/触发时机改进） | open |
| 2 | MAX_HISTORY_MSGS=40 静默切片（FACT_RISK，涉 prompt 结构） | open（移交） |
| 3 | "继续"续轮 planner give_up（决策层） | open（RC47 候选，移交） |
| 4 | chat 压缩归档写共享 compacted.jsonl（session 绑定时序） | open（小） |
| 5 | archive 存截后工具输出（B 档上限钳制） | defer（涉 schema） |
| 6 | constitution 6000 无标记 | defer |
| 7 | RC45/RC46（FA01 移交） | 不属 P2，只登记（批-7） |

## 16. Provenance

| VM | source | version | binary | gate/日志 |
|---|---|---|---|---|
| .133（评审） | `/home/wutao/codex_t` | 0.2.16 | — | `/home/wutao/t_gate_mc_final.log`（443/0 四 RC=0） |
| .131（执行） | `/home/wutao/codex` | 0.2.16 | `/usr/local/bin/hearth` = 0.2.16（23:29 重建，sha256 `d9d94f46…`） | `~/fa/n08.log`/`n09a.log`/`n09b.log`/`ab_*.log`/`n14a.log`/`n14c.log`（已归档 `docs/data/memory-context-20260830/`） |
| 本机 | tag v0.2.16 → `463b315` 基线（HEAD `253fcd1` 含 docs） | 0.2.16 | — | — |

criteria 冻结件：`docs/data/memory-context-20260830/`（run 前冻结未修改）。

## 17. 评审核验锚点与命令清单（评审窗/外部 AI 可直接执行）

`.133:~/codex_t`：

```bash
# 1) turn 粒度修复（单 run 压缩复活）
grep -n "一次工具交换 = 一个新 Turn" crates/agent-core/src/loop.rs
grep -n "test_single_run_compaction_e2e" crates/agent-core/src/loop.rs
# 2) estimate 诚实计数 + legacy 回滚
grep -n "legacy_estimate_chars\|effective_estimate" crates/agent-core/src/context.rs
# 3) provider-aware 阈值（批-2 最小切片）
grep -n "HEARTH_CONTEXT_TOKENS\|HEARTH_COMPACT_WINDOW_RATIO\|HEARTH_COMPACTION_MODE\|CHARS_PER_TOKEN" crates/agent-core/src/loop.rs crates/agent-core/src/context.rs
# 4) RC40 改名（S-1 连带）
grep -rn "compact_pressure_pct" crates/agent-core/src/loop.rs crates/tools-builtin/src/status.rs
grep -rn "context_fill_pct" crates/ || echo "旧名已清零"
# 5) INV-M01 fixtures（可失败双测试）
cargo test -p agent-core --lib inv_m01            # 2 passed
cargo test -p agent-core --lib test_single_run_compaction_e2e  # 1 passed
# 6) 全量 gate
bash ~/run_gate_r2c.sh                            # 四 RC=0, 443/0
```

`.131` 真机日志（已归档本仓库 `docs/data/memory-context-20260830/`）：

```bash
# 死代码红样本（修复前）
grep -ac "context compacted" ~/fa/ab_A1.log  # 0（44k est 历史）——注：修复后压缩用 archive 归因
# 压缩归因（archive 时间戳窗口）
python3 /home/wutao/fa/collect_ab.py
# resume 零重教育
cat /tmp/p2_n09/RESULT.txt                   # 12 数字精确正确
grep -ac "cmd: seq" ~/fa/n09b.log            # 0（resume 后零重跑）
# Task B 修复后零误杀
grep -acE "stalled|Task failed" ~/fa/n14c.log # 0
```

## 18. 执行事故披露（如实）

1. **.133 磁盘满**（96%→100%）：`codex_t/target` 实为指向 `codex/target` 的 symlink（共享构建缓存 76G）；满盘期间出现陈旧测试二进制假失败（9 个）、target 变文件、gate RC=101——清理弃用树构建产物后全量重建，gate 443/0。**教训：Node 00 磁盘风险项应即时清理。**
2. **本地 C: 盘满（99%）→ .git 损坏**：stash 触发 gc → refs 目录丢失 + 今日 loose objects 丢失。工作树完好；仓库重建（`463b315` v0.2.16 基线），**今日 git 提交历史不可恢复**，历史连续性以 CHANGELOG + docs 为准。
3. pkill 自匹配误杀启动 shell ×3（`pattern[ ]` 括号技巧 + pkill/launch 分离已写入执行窗 skill）。

**防伪声明**：本包所有"completed"均可独立复跑验证（§17 命令）；所有"failed"如实层归；"压缩发生"以 archive 落盘为权威信号（日志 INFO 不可靠已披露）；无一次成功外推为"全部可靠"。
