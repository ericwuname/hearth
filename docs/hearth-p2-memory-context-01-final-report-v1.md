# P2-MEMORY-CONTEXT-01 Final Report v1

**日期**：2026-08-30　**执行窗口**：砺·执行　**基线**：v0.2.15（`ceb62b0`）　**终态**：**v0.2.16**（tag v0.2.16，工作树 `463b315` 重建基线）

## 1. Executive Summary

```text
PASS WITH DEVIATIONS
```

四问答案：
- **Q1（过早压缩？）**：**是且更严重**——est 口径 32k 字符 ≈ **9,009 tokens** = Agnes 512K 的 **1.8%**（Node 01 API 权威实测；比先验 3% 更激进，因 estimate_chars 叠加 Debug 包装 + UTF-8 字节双重虚增 1.39-2.49×）。
- **Q2（Compaction 是失忆主因？）**：**修正**——单 `hearth chat` run 内**压缩是死代码**（全部交换 push 进唯一 Turn 0 → history.len() 恒 1 → keep_from==0 守门恒 false；真机 A1 44k est 零压缩）。单 run 真正的窗口约束 = `MAX_HISTORY_MSGS=40` 静默切片（无标记无归档）。压缩历史上只在 REPL 多轮生效。
- **Q3（阈值调整后 Fact 仍会丢？）**：**不会**——INV-M01 确定性 fixture 证明：state 层事实（goal/original_goal/constraints/acceptance_criteria/scratch）不在压缩路径；history 原文先归档（`archive/<sid>.jsonl`）再折叠；B 档经 grep 可恢复。真机 A3：1 次压缩 + RESULT 12 数字全对。
- **Q4（参数标定还是架构？）**：**参数标定（provider-aware）+ 复用既有 archive 即足**（情况 A）——不建新 Fact Store（M8 遵守）。已实施：诚实 estimate + provider-aware 阈值注入 + RC40 改名 + 回滚通道。

**Deviation**：① Node 09 resume 前置跑未含压缩（deadline 短于压缩点）——压缩+resume 组合由 Node 14 Task A（受控触发压缩+完成）与 Node 09（resume 连续性）分证，合证链成立但非单跑闭环；② Node 07 B 档（模型 grep archive）真机 0 使用——检索提示在场但模型未走（OPEN）；③ Node 08 发现"继续"续轮 planner give_up（决策层，登记移交）。

## 2. Token Calibration（Node 01）

| 语料 | naive chars | est 口径¹ | tokens(API) | chars/token | est 虚增 |
|---|---:|---:|---:|---:|---:|
| system（constitution） | 1,430 | 3,562 | 1,063 | 1.35 | 2.49× |
| code（loop.rs 500 行） | 14,028 | 18,508 | 5,356 | 2.62 | 1.32× |
| 中文对话 | 673 | 1,036 | 560 | 1.20 | 1.54× |
| 英文 tool result | 809 | 826 | 556 | 1.46 | 1.02× |
| mixed | 631 | 766 | 552 | 1.14 | 1.21× |
| TaskGraph/Continuity | 792 | 921 | 552 | 1.43 | 1.67× |
| **COMBINED** | **18,373** | **25,629** | **7,215** | **2.55** | **1.39×** |

¹ Rust `format!("{:?}").len()` 复刻（Debug 枚举 + UTF-8 字节）。**32,000（est）≈ 9,009 tokens ≈ 22,940 naive chars**。provider usage 为唯一权威口径（批-3）。测量脚本 `measure_tokens.py` 可复跑。

## 3. Compaction Audit（Node 02）

触发：`build_messages` 每次调用前 → `maybe_compact`（est ≥ 阈值）→ 保留最近 2 轮 → 先归档后折叠 → 摘要注入检索提示。单位 = Debug+bytes 字符（已修）；provider-unaware 确认（llm-cn 表无 Agnes；`/v1/models` 无窗口字段→自动发现不可行，批-2 路线证实）；32,000 = 历史遗留值。**RC40 单列**：`context_fill_pct` 语义错位（距压缩阈值距离冒充窗口填充率，改阈值不治）→ 已改名 `compact_pressure_pct` + 分母动态化 + status.rs:91/101 同步（S-1）。**死代码发现**：见 §1 Q2。详见 `docs/memory-context-compaction-audit.md`。

## 4. A/B Evidence（Node 05/06）

12 命令长任务，A=32k 默认 / B=64k（延迟压缩代理；生产实现走 provider-aware），**A/BAB 交错** 5 配对，每跑记时间戳：

| 组 | 终态 | RESULT 正确 | 步数 mean | tokens mean | 压缩归因（archive 行） |
|---|---|---|---|---|---|
| A×5 | 4 completed + 1 failed(give_up) | **5/5** | 32.2 | 129,217 | A1×3, A3×1 |
| B×5 | 4 completed + 1 failed(stalled 11 步) | 4/5（B1 模型早夭） | 34.4 | 140,614 | B3×2, B4×2 |

- **Fact retention：9/10 RESULT 内容精确正确**（确定性比对，非自评）；带压缩的 4 跑中 3 跑 correct（A1 correct 但 failed——Reserve 时序，见 §7）。
- 均值接近、方差大（model 层）——按 §29 只报 mean/min/max（A steps 14-69，B 11-67），**不宣称阈值显著效应**（批-4 层归纪律）。
- 层归单列：A1 failed = mechanism 正确（Reserve 零和）+ model（GiveUp 前未写 RESULT + 2 步无进展触发 budget-low 臂）；B1 failed = model 早夭。
- **异常执行记录**：A1/B3/B4/B5 大量重复 seq（38-55 次 vs 12 次）——模型违反"不得重跑"指令，其 correctness 部分来自重执行（诚实标注：clean 样本 = A2/A3/A5/B2，其中 A3 含压缩）。

## 5/6. Fact / Verification Retention（Node 03/07）

- **确定性 fixture（S-7）**：`test_inv_m01_fact_survives_compaction`——12 项清单逐项判级：state 层 7 项 preserved、history 旧轮原文 lost-but-archived（B 档可恢复）、摘要 reconstructed、检索提示在场；`test_inv_m01_loss_mode_without_archive` 钉死损失模式（fixture 能失败证明）。
- **存储位置分类（S-3）**：goal/original_goal/goal_revision/constraints/acceptance_criteria/scratch = state 层不在压缩路径——零假阳性。
- **真机**：A3（1 压缩 + 12 seq 干净 + RESULT 精确正确 + completed）= 压缩后事实保持的干净闭环样本。
- **分档口径（S-6）**：A 档（上下文内）100%（state 层 fixture）；B 档（grep archive）机制存在（compacted.jsonl 真实落盘增长）但**真机模型 0 次使用**——OPEN。

## 7. Goal Continuity（Node 08/09）

- **Node 08**：REPL 多轮 + 压缩阈值 800 + "继续"——三文件事实跨轮保持（f1=A1/f2=B2/f3=C3 独立验证）、**无 GoalMutation**（修 B 语义保持）；发现续轮 planner give_up 于"已完成+继续"形态（决策层，**登记移交**——RC47 候选）。
- **Node 09**：deadline 中断（13 步）→ `hearth resume <uuid>`（空 goal）→ **6 步完成，seq 重跑 0 次，RESULT 12 数字精确正确，re-teach count = 0**（M6 实证）。注：resume 会话 id 需完整 UUID（banner 显示截断——小 UX 项登记）。

## 8. Changes（逐文件）

| 文件 | 变更 |
|---|---|
| `agent-core/src/context.rs` | estimate_chars 诚实计数（naive chars）；legacy_estimate_chars + effective_estimate（回滚）；compact_char_threshold 动态化（注入 > env > 遗留常量）+ set_compact_threshold；INV-M01 双 fixture；threshold 矩阵测试；honest counting 测试 |
| `agent-core/src/loop.rs` | **turn 粒度修复**（一次工具交换 = 新 Turn——单 run 压缩死代码修复）；provider-aware 阈值注入（env 覆盖 > caps × ratio × 2.55）；RC40 改名 compact_pressure_pct + 动态分母；**T4 停滞状态按 run 重置**（Task B 红样本修复）；e2e compaction 测试 |
| `tools-builtin/src/status.rs` | context_fill_pct → compact_pressure_pct 同步（S-1） |
| `tools-builtin/src/web.rs` | 8000 截断补标记 |
| `Cargo.toml` | 0.2.15 → 0.2.16 |
| docs | memory-context-compaction-audit.md（审计+旁路+截断清单）/ data/memory-context-20260830/（校准脚本+基线+A/B 采集+执行日志） |

## 9. Tests（先红后绿）

| 红 | 绿 |
|---|---|
| 单 run 压缩死代码（单元红 + A1 真机 44k est 零压缩） | turn 粒度修复 → 单元 + e2e 绿 |
| estimate 虚增（Node 01 定量） | honest counting 测试绿（naive=7 vs legacy=29 对照） |
| Task B 首跑 2 个 QA 轮 1 步 stalled（T4 跨轮签名污染） | run 重置修复 → 重跑 11 轮 **0 stalled** |

**Final Gate**（`.133:~/run_gate_r2c.sh` → `/home/wutao/t_gate_mc_final.log`，v0.2.16）：**FMT=0 / CLIPPY=0 / RT4_SOLO=0 / TEST=0，443 passed / 0 failed**。

## 10. Real-machine（Node 14）

- **Task A**（calc2 受控失败 + 阈值 8000 强制压缩）：completed；独立复验 cargo test **1 passed**、修复在位（`a * 2`）、RESULT=N14_RECOVERED；archive 105→110 行（5 turn 压缩）。链路覆盖：write/test/failure/repair/verification/compact/completion（resume 环节由 Node 09 实证）。
- **Task B**（QA 11 轮）：修复后 **0 stalled / 0 failed**，QA 全程不进 TaskGraph/recovery（INV-FA01-H/INV-M 保持）。

## 11. Regression（Node 15）

Gate 443/0 覆盖全部历史主线（R2-C / W3-W4 / RC24 / W8 / P1-LTR / P1-TASK-TRUTH / P1-EXECUTION-DECISION / P1-FAILURE-ADAPTATION 评审包 §14 清单 / N1-SBX）。INV-ED01-*/INV-FA01-* 单测未回改全绿；INV-M01 落为可失败 fixture（S-7）、INV-M02/03/04 为实验纪律与范围约束（不强制单测）。

## 12. Telemetry（Node §22-24）

新增观察项全部 **internal-only**（S-8：scratch/summary，不进 LLM 可见 body）：compact 计数经 archive 行归因、goal_revision 经既有事件、retention 经确定性比对。`compact_pressure_pct` 为唯一 LLM-visible 字段（改名后语义如实，服务模型疲劳判断）。§23/24 只观察不设阈值。

## 13. OPEN / UNKNOWN / DEFER

| # | 项 | 等级 |
|---|---|---|
| 1 | B 档归档检索：模型 0 次真实使用 grep 提示——提示措辞或触发时机需改进 | open |
| 2 | MAX_HISTORY_MSGS=40 prompt 静默切片（无标记无归档） | open（FACT_RISK，涉 prompt 结构，超本轮边界） |
| 3 | "继续"续轮 planner give_up（已完成状态+继续） | open（决策层，RC47 候选，移交） |
| 4 | chat 路径压缩归档写共享 compacted.jsonl（session_id 绑定晚于首次压缩） | open（小，会话隔离弱化） |
| 5 | archive 存截后工具输出（B 档上限受 6000 截断钳制） | defer（涉 archive schema） |
| 6 | constitution 6000 截断无标记 | defer（NEEDS_REVIEW） |
| 7 | RC45/RC46（FA01 移交） | 不属 P2，只登记（批-7） |

## 14. Provenance

| VM | source | version | binary | gate |
|---|---|---|---|---|
| .133（评审） | `/home/wutao/codex_t` | 0.2.16 | —（gate 编译） | `/home/wutao/t_gate_mc_final.log`（443/0 四 RC=0；磁盘满后全量重建编译） |
| .131（执行） | `/home/wutao/codex` | 0.2.16 | `/usr/local/bin/hearth` = 0.2.16（23:29 重建安装） | —（真机日志 `~/fa/n08.log` / `n09a.log` / `n09b.log` / `ab_A*.log` / `ab_B*.log` / `n14a.log` / `n14c.log`，已归档 docs/data/memory-context-20260830/） |
| 本机 | tag v0.2.16 → `463b315`（仓库重建基线，见 §15 事故记录） | 0.2.16 | — | — |

## 15. Final Decision + 执行事故披露

**Final Decision：parameter calibration（provider-aware）+ 复用既有 archive（情况 A）——不建 Fact Persistence 新架构。** 依据：A/B 无劣化证据 + INV-M01/真机证明压缩不丢事实 + 死代码修复后压缩在单 run 正常工作。

**执行事故（如实披露）**：
1. **.133 磁盘满**（96%→100%）：`codex_t/target` 实为指向 `codex/target` 的 symlink（共享构建缓存 76G）；磁盘满期间出现陈旧测试二进制假失败（9 个）、target 变文件——已清理弃用树构建产物（76G 回收，磁盘 34%），gate 全量重建后 443/0。**教训：Node 00 磁盘风险项应触发即时清理而非仅登记。**
2. **本地 C: 盘满（99%）→ .git 损坏**：stash 触发 gc → refs 目录丢失 + 今日 loose objects 丢失。工作树完好；已重建仓库（`463b315` v0.2.16 基线）+ 迁移旧 packs。**今日 P2 提交的 git 历史不可恢复（对象级丢失），工作树与 tag 完整**——历史连续性以 CHANGELOG 与 docs 为准。
3. pkill 自匹配误杀启动 shell ×3（pattern 技巧已记录）。
