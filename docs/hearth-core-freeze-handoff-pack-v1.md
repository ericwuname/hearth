# Hearth Core 长程可靠性整合报告 v3（全量：FA01 + P2-MC + P2-LR + CFR + CLOSURE + P0 + P3 + P4 八包）

> **用途**：单文件沟通材料——供顶层设计窗口与外部 AI（ChatGPT）评审。本文件为八总包的整合视图与 **CORE FREEZE 全程裁决链**（FREEZE → SUSPENDED → REVALIDATION IN PROGRESS）；各包独立 Final Report / review package 为证据明细（路径见 §9）。
> **执行窗口**：砺·执行　**日期**：2026-08-31　**版本演化**：v0.2.14 → **v0.2.20**（v0.2.18=CFR/CLOSURE 冻结版 → v0.2.19=P3 清欠 → v0.2.20=P4 修复批）
> **工程状态**：**CORE FREEZE SUSPENDED / CORE REVALIDATION IN PROGRESS**（触发=v0.2.18 用户盲测 28.6% / 24 连败吸引子 RC52；P0 归因闭合 → v0.2.20 修复批落地；campaign（Node 11-12）已按窗口分离裁决移交测试窗口，指令书 `docs/hearth-test-window-campaign-directive-v1.md`，见 P4 章）
> **核验纪律**：所有结论可经 §8 命令清单独立复核；禁采信 commit message 与"已验证"自述；mechanism/decision/model/provider/environment 五层归因贯穿全程。

---

## 0. 一页纸总览

| 总包 | 终态 | 版本 | gate | 核心成果 |
|---|---|---|---|---|
| **P1-FAILURE-ADAPTATION-01** | PASS WITH DEVIATIONS / CLOSED | v0.2.15 | 437/0 | F1-F10 taxonomy、GiveUp 五路径收口（T4/预算旁路拦截、deadline 豁免）、双真机闭环 |
| **P2-MEMORY-CONTEXT-01** | PASS WITH DEVIATIONS / CLOSED | v0.2.16 | 443/0 | 单 run 压缩死代码修复、provider-aware 阈值、INV-M01 可失败 fixtures、RC40 改名 |
| **P2-LONG-RUN-ACCEPTANCE-01** | PASS WITH DEVIATIONS / CLOSED | v0.2.17 | 447/0 | RC47 修复、exit code 五态接线（RC46 闭环）、40 切片标记、session 绑定、10 场景真机 |
| **CORE FREEZE REVIEW-01** | READY WITH ACCEPTED DEVIATIONS / CLOSED | v0.2.18 | 447/0 | 阈值优先级修正、Authority Matrix、RC48 根因锁定、QA 边界实测、review package |
| **CORE FREEZE CLOSURE-01** | **CORE FREEZE（最终裁决）** | v0.2.18 | 447/0 | RC49 规范链重跑闭环、F0 六项四层复核、F1×4 全 ACCEPTED DEVIATION、freeze-decision 单一结论 |
| **P3-BACKLOG-01** | PASS WITH DEVIATIONS / CLOSED | v0.2.19 | 455/0 | 配置语义三连（RC16/18/13）、边界收敛（RC25/27）、小修（RC34/45/15）、复验批部分完成、ledger 回填 10 项 |
| **P4-REVALIDATION-01** | **REVALIDATION IN PROGRESS（Node 00-09 完成）** | v0.2.20 | 459/0 | RC52 因果闭合（decision 主因+污染放大器 40%→100%）、SimUser Stage 1（PTY driver+13 检测器+口径冻结）、RC51-A/B+RC53 修复、Node 07-09 三审计零新缺陷 |

**主线一句话**：Hearth 的失败分类→恢复策略→验证→压缩→resume 链路，已在真机（Agnes，budget 40-80，30+ 场景）证明 mechanism 层有界正确——零假完成、零静默丢失、零无限循环、零沙箱绕过、零重教育。残余 4 项 F1 全部"可解释、有界、已声明"。

**历史裁决（已 SUSPENDED）**：CORE FREEZE（CLOSURE-01 freeze-decision.md；F0=0、F1×4 ACCEPTED DEVIATION、零 BLOCKER）→ 被 v0.2.18 用户盲测 28.6% 打穿而 SUSPENDED（见 P0 章）。

**当前状态**：mechanism 层依旧有界正确；盲测暴露的是 decision/model 层交互缺陷（RC52 已归因闭合）与 Projection 完整性缺陷（已修 v0.2.20）。Core 重验进行中——campaign（Node 11-12）归测试窗口，重冻结须走 Node 13（v0.2.21）→ Node 14 独立复测 → Node 15 Freeze Candidate（见 P4 章）。**冻结红线（修改边界）始终有效**——SUSPENDED 解除的是"验收结论"，不是"修改边界"。

---

## 1. P1-FAILURE-ADAPTATION-01（失败适应机制）

- **Failure Taxonomy F1-F10**：`classify_failure` 纯函数（结构化输入，禁错误文本解析）；策略矩阵六策略（Retry 仅 Transient；verification≠retry；resource→Stop）。
- **GiveUp 五路径收口**：planner GiveUp 臂 + T4 停滞拦截 + 预算 headless abort 拦截 + deadline **故意不拦截**（Safety>Deadline，604s 权威实证）+ nervous Abandon。统一不变式：未核验的放弃要么先核验（Reserve ≤1 零和）要么打 `giveup_unverified`。
- **真机**：calcpkg 3 跑（r1/r2 红样本、r3 deadline 权威终止）；mathlib（拦截拒绝假 completed → r3 completed 21 步，GIVE_UP_OVERRIDDEN）；wordcount 46 步全闭环。3 个 failed 全部 model+environment，mechanism 0 缺陷。

## 2. P2-MEMORY-CONTEXT-01（记忆与压缩）

**四问四答**：
- **Q1** est 32k 字符 ≈ **9,009 tokens = Agnes 512K 的 1.8%**（API 权威；estimate_chars Debug+字节双重虚增 1.39-2.49×）。
- **Q2 修正**——单 run 压缩曾是**死代码**（全部交换 push 进唯一 Turn 0 → keep_from==0 守门恒 false，真机 44k est 零压缩）；单 run 真正约束 = MAX_HISTORY_MSGS=40 静默切片。
- **Q3** INV-M01 可失败 fixture：state 层 7 类事实不在压缩路径；旧轮原文先归档后折叠；B 档 grep 通道在。
- **Q4** provider-aware 参数标定 + 复用既有 archive（情况 A），不建新 Fact Store。

**落地**：turn 粒度修复（压缩复活）+ estimate 诚实计数 + provider-aware 阈值 + legacy 回滚 + RC40 改名 `compact_pressure_pct`。

## 3. P2-LONG-RUN-ACCEPTANCE-01（长程验收）

- **RC47 修复**：criteria 空 + 产物在 + 0 errors 的 give_up → 路由 Done（盲区C 兜底）；修 E 反例细化为无产物 QA 形态。
- **exit code 五态接线**（RC46 闭环）：`BashExitError` 类型化 → `ExitNonZero/ExitSignal/ToolTimeout`。
- **40 切片标记** + **chat 主路径 session 绑定**。
- 10 场景真机：mechanism 10/10 有界正确；false stop ×2（→ RC48）。

## 4. CORE FREEZE REVIEW-01（本轮：独立横向审查）

### 4.1 Provenance 实修（批-2）
.133 binary 0.2.16 滞后确认=真滞后 → 重建安装 0.2.17→0.2.18，三查一致 + 冒烟 test_rc47 passed；源码备份在案；df 保险丝全程未触发。

### 4.2 Authority Matrix（批-3：实测基线，非 Roadmap 填空）
15 权项逐项实测：**零双 authority / 零无 authority / 零越级**。唯一特殊结构 = give_up 的**决策权（planner）/否决权（loop 消费端）分离**——修 D 预裁决的有意设计，非冲突。

### 4.3 Decision→Terminal Map（Node 02）
五组语义区分全成立（Stop≠GiveUp 在 reason 层；Escalate≠Approval≠Stop；GiveUp≠Completion）；**Escalate + delegation + 无通道 = 结构化降级 Stop**（n12r2 approval_denied 实证，无隐式行为）。

### 4.4 False Stop 深挖（Node 03/04，最高优先级——RC48 根因锁定）

| 样本 | 终态 | Reserve | GIVE_UP_OVERRIDDEN | 独立复验 |
|---|---|---|---|---|
| n12r1 | failed 21 步 | 1（已耗尽） | 0 | sortlib **passed** |
| **n12r3（可比复现）** | **completed 31 步** | 1 | **1** | sortlib **passed** |

- **假设 A 成立（精确化）**：criteria 非空 + Reserve 零和（早期核验失败耗尽唯一 Reserve）→ 终止时无第二次核验机会。
- **n12r3 决定性反证**：同任务同参数，GiveUp 时点有 Reserve → 拦截→核验**通过**→completed 全链路正确——机制在有核验机会时完美工作。
- 假设 B ✗；假设 C ✅ 贡献因子（planner 无完成事实感知）。
- **与 RC47 分界 = criteria 空与非空**。修复方向（Reserve 分账 / acceptance-passed 消费扩展）**须顶层批准**——已登记不实现。层归：mechanism（有界设计代价）+ model（GiveUp 时点）。

### 4.5 40 切片审计（Node 05，批-1 主战场）

保护对称性对照表：compaction 路径（archive 先行+提示+B 档）vs 切片路径（标记已落✓ / 无归档 / **零恢复通道**）。判级 = **lost-to-LLM（F1）**——"已知有界损失"而非"已保护"（Task Continuity 投影缓解关键事实）。最小改善二选一登记，冻结窗口不动。

### 4.6 Archive Recoverability（Node 06，批-5 防污染）

C-probe 三版迭代如实记录：v2 用户消息污染 / v3 同（`enqueue_user_message` 永驻保留轮）/ v4 setup 失败。**判级 = C 未证明，机制在**（"archive preserved but model recoverability unproven"）。A（落盘实证）+ B（提示 fixture 锁定）不冒充 M5。防污染设计要点已固化 review-index。

### 4.7 真机 Benchmark（Node 08-11，criteria 全冻结 S-2）

| Benchmark | 终态 | 独立复验 |
|---|---|---|
| Node 08 canonical 单跑链（chainfree：fail→repair→compact→stop→resume→complete） | completed ×2 阶段 | **1 passed** ✓（S-4：P2-MC Deviation① 单跑闭环） |
| Node 09 Task A todoapi（多模块） | completed | **1 passed** ✓ |
| Node 09 Task B units（round-trip 拓扑） | completed | **3 passed** ✓ |
| Node 10 Case A configlib（常量错误） | completed | **3 passed** ✓ |
| Node 10 Case B legacy_app（环境适配拓扑） | completed | ADAPTED_OK ✓ |
| Node 11 QA 16 轮（"继续"两形态+情绪+任务） | 任务产物全达成 | todo/done.txt 精确 ✓ |

### 4.8 运行时压缩发现（本轮最重要的横向发现）

- **v0.2.17 压缩静默失灵**：provider-aware 注入（Agnes caps 128K → 阈值 195,840 字符）压过 env 测试仪器——`COMPACT_DBG est=68 thr=195840` 实锤。
- **v0.2.18 修正**（env > injected > 常量）：probe11 实证压缩恢复（est 73→2784→6234→9762 跨越、archive 增长、session 归档文件落盘）。
- **锯齿式压缩形态**（机制注记）：merge 后历史折叠回单 Turn → 需 ≥2 次新交换再触发——正确性无损（每次触发都归档），效率登记。
- **P2-LR Node 13 "双压缩"声明更正**：该缺陷使其压缩未实际发生（失败/修复/resume 链有效）——如实披露。

## 5. Reliability Matrix（Node 12/14 合并，三层口径）

| 能力 | mechanism | behavior | evidence | status |
|---|---|---|---|---|
| Intent | ✓ 规则分类 | ✓ QA/任务分流 | W8 fixtures+真机 | PASS |
| Plan | ✓ decompose+T4 有界 | ✓（QA stall=F1） | 真机 | PASS* |
| Execution | ✓ 五态结构化 | ✓ | 集成测试+真机 | PASS |
| Fact | ✓ state/archive 分层 | ✓ | INV-M01+真机 | PASS |
| Verification | ✓ O-4 唯一生产者 | ✓ | FA01+CFR fixtures | PASS |
| Failure Recovery | ✓ 分类+策略+有界 | ✓（RC48 边界） | n12 三跑对照 | PASS* |
| Completion | ✓ 双证据门 | ✓ 零假完成 | 真机 | PASS |
| Memory | ✓ archive 先行 | ✓（C 未证明=F1） | probe 三版 | PASS* |
| Resume | ✓ 持久恢复 | ✓ reteach=0 | 三例实证 | PASS |
| Decision | ✓ 拦截链+RC47 路由 | ✓（RC48 时点） | n12r1/r3 对照 | PASS* |
| Terminal | ✓ 九态封闭 | ✓ reason 可区分 | 报告实测 | PASS |
| Projection | ✓ 事实投影 | ✓ | RC20 家族 | PASS |
| Security | ✓ RT4+approval | ✓ 拒绝实证 | n12r2 | PASS |

**False 指标**：false completion **0** / false giveup **0** / false replan **0** / **false stop 2+12**（RC48 两形态，有界）/ silent loss **0** / reteach **0** / duplicate execution：任务轮零。

## 6. 测试与门禁演化

| 版本 | gate | 增量 |
|---|---|---|
| v0.2.15 | 437/0 | FA01 +8 |
| v0.2.16 | 443/0 | P2-MC +6 |
| v0.2.17 | 447/0 | P2-LR +4 |
| **v0.2.18** | **447/0 四 RC=0**（`t_gate_cfr_final.log`） | 阈值优先级修正+测试更新（无新增计数） |
| v0.2.19 | 455/0 | P3-BACKLOG +8（RC16×3 / RC18 / RC25×3 / RC34） |
| **v0.2.20** | **459/0 四 RC=0** | P4 Node 05/06 修复批（RC51-A/B + RC53 + RC54 fixture）+ env 竞争串行化修复 |

## 7. Hard Failure Criteria 复核（逐条）

| 判据 | 结果 |
|---|---|
| 假完成 | **零** |
| 历史事实静默丢失 | **零**（compact=archive 先行；slice=标记+state 存活+Continuity 投影） |
| resume 重教育 | **零**（re-teach=0 三例） |
| 无限循环 | **零**（三重有界） |
| 错误输出投影成功 | **零**（五态+RC20） |
| sandbox/approval 绕过 | **零**（n12r2 拒绝实证） |

## 8. 评审核验命令清单（可直接执行）

`.133:~/codex_t`：

```bash
# FA01：taxonomy + 拦截链
grep -n "pub enum FailureKind" crates/agent-core/src/terminal.rs
grep -n "GIVE_UP_ROUTED_TO_DONE\|GIVE_UP_OVERRIDDEN\|GIVE_UP_INTERCEPTED\|giveup_unverified" crates/agent-core/src/loop.rs | head
# P2-MC：turn 粒度 + honest counting + provider-aware + RC40
grep -n "一次工具交换 = 一个新 Turn" crates/agent-core/src/loop.rs
grep -n "HEARTH_COMPACTION_MODE\|CHARS_PER_TOKEN" crates/agent-core/src/context.rs crates/agent-core/src/loop.rs
grep -rn "compact_pressure_pct" crates/ | head -3; grep -rn "context_fill_pct" crates/ || echo "旧名清零 ✓"
# P2-LR：RC47 + 五态 + 切片标记 + session 绑定
grep -n "pub struct BashExitError" crates/tool-runtime/src/dispatcher.rs
grep -n "history-slice-note" crates/agent-core/src/loop.rs
grep -n "set_session_id" crates/codex-cli/src/run_local.rs
# CFR：阈值优先级 + Authority Matrix 素材
grep -n "env 测试仪器必须压过注入值" crates/agent-core/src/context.rs
# 测试
cargo test -p agent-core --lib inv_m01        # 2 passed
cargo test -p agent-core --lib test_rc47      # 1 passed
cargo test -p agent-core --lib test_five_state # 1 passed
cargo test -p agent-core --lib test_history_slice_marker  # 1 passed
bash ~/run_gate_r2c.sh                        # 四 RC=0, 447/0
```

`.131` 真机（日志已归档 `docs/data/` 两处）：

```bash
cargo test --manifest-path /tmp/cfr_n09/todoapi/Cargo.toml    # 1 passed
cargo test --manifest-path /tmp/cfr_n09b/units/Cargo.toml     # 3 passed
cargo test --manifest-path /tmp/cfr_n10a/configlib/Cargo.toml # 3 passed
cat /tmp/cfr_n08/FREEZE_RESULT.txt                            # CHAIN_OK
grep -ac VERIFICATION_RESERVE ~/fa/lr_n12r1.log               # 1（RC48 证据）
python3 /home/wutao/fa/collect_lr.py                          # Matrix 重算
```

## 9. 证据明细索引

| 包 | Final Report | 评审包 | 数据 |
|---|---|---|---|
| FA01 | docs/hearth-p1-failure-adaptation-01-final-report-v1.md | docs/hearth-p1-failure-adaptation-01-review-pack-v1.md | docs/data/fa01-*/ |
| P2-MC | docs/hearth-p2-memory-context-01-final-report-v1.md | docs/hearth-p2-memory-context-01-review-pack-v1.md | docs/data/memory-context-20260830/ |
| P2-LR | docs/hearth-p2-long-run-acceptance-01-final-report-v1.md | docs/hearth-p2-long-run-acceptance-01-review-pack-v1.md | docs/data/long-run-20260831/ |
| **CFR** | docs/hearth-core-freeze-review-01-final-report-v1.md | **docs/core-freeze-review/**（11 文件，review-index.md 为入口） | docs/data/core-freeze-review-20260831/ |
| CLOSURE | docs/hearth-core-freeze-closure-01-final-report-v1.md | docs/hearth-core-freeze-closure-01-review-pack-v1.md | docs/data/core-freeze-closure-20260831/ |
| P0 | docs/hearth-p0-attribution-01-final-report-v1.md | —（零修复归因包） | docs/data/p0-attribution-20260831/ |
| P3 | docs/hearth-p3-backlog-01-final-report-v1.md | —（施工包） | docs/data/p3-backlog-20260831/ |
| **P4** | docs/hearth-p4-revalidation-01-final-report-v1.md | **docs/core-revalidation/**（baseline / decision-status / projection-audit / node07-09-audits） | .133:~/fa/p4/ |

## 10. OPEN 汇总（F0/F1/F2 最终分级）

**F0 = 0**（零假完成/零静默丢失/零重教育/零无限循环/零沙箱绕过/零 Projection 失真/零 provenance 错位）。

| F1（4 项，全部有界+已声明） | 处置方向 |
|---|---|
| RC48 false stop（Reserve 零和 × GiveUp 时点） | Reserve 分账 / acceptance-passed 消费扩展——**须顶层批准** |
| QA 轮 T4 2-node stall 高频（planner 无完成事实感知） | planner 完成感知注入——**须顶层批准** |
| Archive C 未证明（探针污染 ×2） | 按批-5 设计要点重试（不阻塞） |
| 40 切片 lost-to-LLM（标记已落） | 入 archive 或标记修正（二选一，小改） |

F2 DEFER：constitution 6000 标记 / interaction 样本补全 / archive 存截后输出 / 锯齿压缩效率 / RC45 双上限 / resume banner UUID。

## 11. Provenance

| VM | source | binary | gate |
|---|---|---|---|
| .133（评审） | `/home/wutao/codex_t` = 0.2.18 | **0.2.18**（滞后已实修） | `~/t_gate_cfr_final.log` 447/0 |
| .131（执行） | `/home/wutao/codex` = 0.2.18 | **0.2.18**（干净重建，无诊断） | 真机日志已归档 docs/data/ |
| 本机 | tag v0.2.18 → HEAD `41cc070` | — | — |

备份 `/home/wutao/hearth-v0.2.16-backup-20260831.tar.gz`；**git 历史自 463b315 起可信**（.git 损坏重建声明延续）；df 保险丝全程未触发。

## 12. 事故披露汇总

1. .133 磁盘满（清理弃用树 76G 回收；满盘期间假失败已澄清）。
2. 本地 C: 盘满 → .git 损坏重建（工作树完好；git 历史 463b315 前断链，CHANGELOG+docs 为准）。
3. 探针 pkill 自我残杀链（导致一度误判压缩失灵——后经 COMPACT_DBG 与干净探针澄清）。
4. **P2-LR Node 13 "双压缩"声明失真**（v0.2.17 注入缺陷致压缩未实际发生）——本轮更正。

## 13. Final Freeze Recommendation（已由 CLOSURE-01 裁决）

# **CORE FREEZE**

五层验收全部成立：Structural ✅（447/0）+ Behavioral ✅（mechanism 13/13 PASS，4 项 PASS* 带 F1 边界）+ Long-run ✅（完整链真机，RC49 重跑闭环后 compact 腿恢复）+ Evidence ✅（全独立复验+criteria 冻结+归档可反查）+ Independent Review ✅（CLOSURE-01 closure-audit + review package；最终独立裁决见 `docs/core-freeze-review/freeze-decision.md`）。

残余 F1 均为 model/decision 层或有界机制边界，无一触及 Core 事实模型；两项修复方向已明确并待顶层批准——批准与否不改变 Freeze 边界，只改变 F1 数量。残余真实限制（archive C、切片恢复）已列为**委托方需知**，非脚注。

**防伪声明**：所有"completed"可独立复跑（§8）；所有"failed"如实层归；"压缩发生"以 archive 落盘为权威信号；clean 与"重跑混入"样本分开列示；false stop ×14 主动披露；无一次成功外推为"全部可靠"。

---

# 终章：CORE FREEZE CLOSURE-01（2026-08-31，最终裁决）

> 第五包执行完毕——**freeze-decision = CORE FREEZE**（`docs/core-freeze-review/freeze-decision.md` 单一结论）。完整证据：`docs/hearth-core-freeze-closure-01-final-report-v1.md` + `docs/hearth-core-freeze-closure-01-review-pack-v1.md`（grep+预期输出核验清单）。

## C.1 RC49 闭环（本包最重产出，批-1 实锤处置）

CFR Node 08 规范链原跑于阈值优先级修复前 73 分钟（compact 腿无效——RC49 成立）。**v0.2.18 干净二进制重跑闭环**：ARCHIVE_BEFORE=120 → AFTER=120 lines + **新 session 归档文件 8335e03c（17KB）** + compacted.jsonl **+18KB @13:18** = 压缩真实发生（archive 落盘权威信号，非日志 grep）；双阶段 completed、CHAIN_OK ✓、chainfree 独立复验 **1 passed** ✓。Long-run 柱恢复完整。

## C.2 诊断构建矛盾消解（批-2）

.131 binary `strings COMPACT_DBG = 0`（干净重建，sha256 `9e92dfc9…`）；CFR 时代真机证据 = 显式接受含诊断构建（诊断点不改变控制流）。

## C.3 F1 Disposition 表（最终裁决，零 OPEN）

| F1 | Classification | Disposition |
|---|---|---|
| RC48 false stop | mechanism(bounded)+model（n12r1/r3 对照；假设 B 排除） | **ACCEPTED DEVIATION + RC48-FOLLOWUP RFC** |
| QA completion-awareness | model/decision（注入=authority duplication） | **ACCEPTED DEVIATION** |
| Archive C | evidence gap（C=UNKNOWN，不升格） | **ACCEPTED DEVIATION** |
| 40-slice | mechanism declared（bounded loss-to-LLM ≠ silent loss） | **ACCEPTED DEVIATION** |

## C.4 最终状态

- **gate 447/0 四 RC=0**（`~/t_gate_closure.log`，Node 01 复现）；**生产代码改动 = 0**（no-code gate，v0.2.18 保持）。
- F0 = 0（六项四层复核）；F1 ×4 ACCEPTED DEVIATION；F2 ×6 DEFER；零 BLOCKER、零 STOP。
- freeze-decision.md 三段齐全：guarantees / does-NOT-guarantee（archive C、切片 lost-to-LLM、RC48 复发率、Agnes variance）/ future RFC（RC48-FOLLOWUP、QA 只读投影、切片入 archive、C-probe 重试）。

## C.5 冻结后边界

Core 14 domain 全 FROZEN。冻结后修改须经独立 RFC；Evolution backlog（Subagent/TUI/MCP/Persona/Semantic Memory/动态 Verification 等）不再视为 Core 缺口。**Hearth Core v0.2.18 = FREEZE 基线。**


---

# P0 章：P0-ATTRIBUTION-01（2026-08-31，盲测反证定向归因——零修复）

> **背景**：CLOSURE 宣告 CORE FREEZE 后，用户 90 分钟盲测（v0.2.18，.133 主机，3703 行日志）暴露脚本化验证未覆盖的真实交互缺陷：**24 连败吸引子（RC52）**、裸 ERROR 泄漏、内容截断（BUG-012）、introspect 误路由（RC53）。逐轮归因口径成功率 **28.6%**（10/35）。**CORE FREEZE 随之 SUSPENDED**，P0 归因包（零修复纪律）奉命查明三问。
> 完整报告：`docs/hearth-p0-attribution-01-final-report-v1.md`；数据：`docs/data/p0-attribution-20260831/`。

## P0.1 三问裁决

| 问 | 结论 | disposition |
|---|---|---|
| **Q-A** 裸 ERROR 泄漏路径 | **CONFIRMED**——`do_plan_inner failed`（loop.rs:2381 tracing::error!）→ subscriber stderr 直写（lib.rs:264）→ SSH 终端；**完全绕过 Projection 层**。真机复现 3 处 + 盲测用户两次粘贴原文 | **NEW F1**（Projection isolation+completeness 违约）→ RC51 拆分建议：isolation / completeness / correctness |
| **Q-B** 24 连败因果 | **EVIDENCE GAP**（STOP-① 触发）——三版重放 harness 失真：①管道 stdin 非 TTY→DenyAllNonInteractive→12/12 瞬失败+EOF 忙等刷 56MB；②PTY 输入消费竞争 191MB；③resume 恢复旧 goal→A 侧填充轮全部重复回答第一问 | RC52 维持 "observed attractor, attribution incomplete"；**正确驱动器（PTY 交互式）= SimUSER-01 依赖** |
| **Q-C** 截断点 + give_up/completed 共存 | 截断点 = **Projection 终态分支**（只渲染 ✓ Done 行，分析正文困在 run-XXX.md——盲测 run-010 用户原话"我需要猜你的结果"实锤）；give_up/completed 共存 = **FALSE POSITIVE**（give_up 是报告"反思轨迹"历史记录，✓ Done 是 RC47 路由后经产物校验的终态——不同层级，反证已写） | FALSE POSITIVE + **NEW F1**（completion projection completeness） |

## P0.2 唯一许可改动：env-gated 观测代码（零控制流）

`HEARTH_DEBUG_PLANNER_INPUT=1` → `~/.config/hearth/debug/<session>/` 三 dump：planner prompt 快照 / TaskGraph sig+节点 / 压缩摘要全文。双 VM 构建记录（.133 `bffe2dc0…` / .131 `efc512fa…`）；真机 dump 实证（session 5406164f）。

## P0.3 附加定位（只定位不修 ✓）

- **RC53**：model 层 tool misroute（`bash: introspect: 未找到命令` exit 127；工具存在且曾正常调用）——修复方向：bash 对内部工具名返回结构化提示。
- **RC54**：apply_patch 空白/缩进脆弱匹配——源码定位完成，最小复现 fixture DEFERRED（下批首件）。

## P0.4 NEW F1 汇总（三项）

1. **Projection isolation+completeness**（裸 telemetry 绕过投影 + 终态行不含分析正文）→ RC51 三行拆分。
2. **REPL 不可程序化驱动**（EOF 忙等 + 非 TTY 审批降级 + 输入消费竞争）——**SimUser 测试台对 repl 形态的阻塞项**。
3. **completion projection completeness**（用户需"猜结果"）。

## P0.5 对排期的输入

RC52 归因闭合依赖正确驱动器 → **建议下一批 = SIMUSER-01 阶段 1（检测器+PTY driver+指标口径）与 FIX-01 立项评审并行**（规划书 v1.1 §6 排期不变；规划书待 ChatGPT/顶层审批）。

---

# P3 章：P3-BACKLOG-01（2026-08-31，总账清欠施工——v0.2.19）

> **性质**：ledger v3 清欠（配置语义 / 边界收敛 / 小修 / 复验）；**RC52/RC48/T4/progress 四域零触碰** ✓。完整报告：`docs/hearth-p3-backlog-01-final-report-v1.md`。
> **终态**：PASS WITH DEVIATIONS；gate **455/0**（447 基线 + 8 新测试）；v0.2.19 双 VM 对齐（.133 `8ec50103…` / .131 `730e17de…`——顶层发现滞后后重新对齐）。

## P3.1 配置语义三连（W2，先红后绿，测试锁定）

| 项 | 实现 | 测试 |
|---|---|---|
| **RC16**（D4 方案 A） | `HEARTH_LLM_URL`（LLM base 新名）/ `HEARTH_SERVICE_URL`（service 端点）；`HEARTH_URL` 保留一版兼容=LLM base+stderr 弃用警告；**service 触发只认显式 flag / HEARTH_SERVICE_URL** | ×3（含红：修复前 lib.rs:273 `env HEARTH_URL` 命中） |
| **RC18** | `check_provider_url_mismatch` 纯函数（deepseek/agnes/gemini/ollama/openai 端点关键词）+ CLI stderr 打印 | ×3（不匹配 Some / 匹配 None / 未知 None） |
| **RC13** | service 出网白名单补读 `~/.config/hearth/config.toml`（与 CLI 同构） | 代码 ✓；service 会话真机 **NEEDS-RERUN** |

## P3.2 边界收敛（W7）+ 小修批

- **RC25**：读限域落**工具层**——`is_allowed_absolute_roots`（`HEARTH_READ_ROOTS` 设置时**替换**默认根 cwd+HOME；未设=不回归；越界=结构化拒绝）；landlock 层收敛 **DEFERRED**（工具链白名单方案另行评估）。**RC27**：PATH 同名护栏（非标准路径 → stderr 告警，不阻断）。**D-防线B**：SENDTO 不加位裁定 → CLOSED-DECIDED。
- **RC34**：render `done()` 去重（同 (steps,ok) 连续重复折叠）+ 测试；终端级快照 NEEDS-RERUN。
- **RC45**：`verify_replan_count` **分账**（新 `act_verify_replan_count` bound<1 / Done bound<3 独立）——只消除"Act 消耗 Done 预算"零和，RC48 GiveUp 时点定性**不变**；INV-M01 归档行数断言 4→≥4。

## P3.3 真机复验批（部分完成，剩余登记）

- ✅ 完成：压缩腿（RC49 后续：archive +13KB + resume + STRESS_ALL_OK + mathnotes 2 passed）、RC15 被拒场景（结构化拒绝 → **Task failed 非 PASS**，PASS=0）、RC24-B TC-8b（EINVAL=0 + introspect 可用 + 零裸 ERROR + completed）、RC36（goal_revision=0）、RC2 渲染回归（零 contains 残留）。
- **NEEDS-RERUN**（纯测试，可独立成批）：RC33 长会话 revision 对照 / 复测包（A-5·A-36/12·18 矩阵）/ RC34 终端快照 / RC13 service 会话。**RC29 trust on = NOT-IMPLEMENTED**（能力从未构建——ledger 修正项，非复验）。

## P3.4 常量体检 + ledger 回填

- 常量体检（audit-only）：32,000 遗留阈值 DECISION"下版可上调或移除"；其余维持；**零常量修改**。
- ledger 回填 **10 项**（RC16/18/13/25/27/防线B/RC34/RC45/RC15/E6 alias）——全部以源码+测试实际状态为准。

---

# P4 章：P4-REVALIDATION-01（2026-08-31，Core Revalidation & SimUser Foundation——进行中）

> **性质**：冻结后重验（15 Node）。触发 = v0.2.18 盲测 28.6%（P0 章）。**本整合时点 Node 00-09 完成**，Node 10-15 排期见 P4.5。完整报告：`docs/hearth-p4-revalidation-01-final-report-v1.md`；滚动状态：`docs/core-revalidation/decision-status.md`。
> **版本**：v0.2.20（双 VM 对齐 .133 `8a662f23…` / .131 `fc39c27c…`）；gate **459/0** 四 RC=0。

## P4.1 SimUser Stage 1（Node 01——测试台落地）

- **PTY driver**（`tools/simuser/driver.py`）：真实 PTY（T1 管道 DenyAll/EOF 忙等规避）+ send→wait_turn 定步调（T2 输入竞争规避）+ resume 语义显式（T3 旧 goal 规避）；expect **窗口相对搜索**修复（初始 banner 提前命中 → A1 作废重跑的教训）。矩阵 13 会话实证。
- **13 检测器**（`analyzer.py`）：projection_leak / consecutive_failure_run / t4_stall_burst / revision_explosion / compaction_adjacency / truncated_output / giveup_completed_coexist / tool_misroute / introspect_failure / user_visible_completion / anaphora_resolution_fail / escalation_run / repetition_amplifier。自检双向：盲测日志 225 findings 全命中已知缺陷 / 干净样本 0。
- **metrics.yaml v1.0 冻结**：逐轮归因口径 / archive 三项判定（字节+mtime+文件数，**禁 wc -l**）/ 七层归因 / DRIVER-INDUCED 规则 / 异源驱动绑定 / Q11 单主体声明。
- 证据：`.133:~/fa/p4/`（13 会话 log + events + results）。

## P4.2 RC52 因果闭合（Node 03/04——本包最重产出）

| 条件 | 结果 | 失败率 |
|---|---|---|
| **A fresh**（全新 REPL+小任务完成后继续）×5 | completed×3 / failed×2 | **40%** |
| **B contaminated**（逐字重放 run-001..012 污染前缀）×5 | failed×5 | **100%** |
| ~~C resume~~ ×3 | completed×1 / failed×1 / timeout×1 | ~~67%~~ **已作废**（见下注） |

**⚠️ 后续修正（砺·评审阻断项，《P4 Node 03 独立复现测试指令 v1.1》附录 A）**：C 条件 session id 提取正则（`run_rc52_matrix.py:99`）恒失效 → resume 的是**空 id**——**C 侧 67% 全部无效**。A 40% / B 100% 维持（独立复现容差 ±1 样本）；C 条件由测试窗口实施仪器修复后补跑（`.131:~/fa/p4-rerun/`，与执行窗口 `~/fa/p4/` 物理隔离）。RC52 归因主结论（decision 主因+污染放大器）不依赖 C 侧数据，维持 CAUSE LIKELY。

**裁决：CAUSE LIKELY——decision 层（RC47 族）主因 + 会话污染放大器（40%→100%）**。fresh 无污染即 40% 失败 = planner 不知道任务已完成；七层归因 decision+model（GiveUp 时点）；mechanism 有界非缺陷。CONFIRMED 需决策路径机制级单测复现（入 Node 13 fixture）。修复方向：planner 完成事实注入（只读投影边界，防 authority duplication）或 give_up 消费端扩展——**Node 13 批**。

## P4.3 修复批（Node 05/06 → v0.2.20）

| 项 | 内容 | 对应 P0 缺陷 |
|---|---|---|
| **RC51-A** | tracing subscriber 写 `~/.config/hearth/diagnostics.log`——终端零裸 telemetry | 裸 ERROR 泄漏（Q-A） |
| **RC51-B** | 终态产物清单投影（`collect_written_files` ≤5 项） | 用户"猜结果"（Q-C） |
| **RC53** | bash 对内部工具名返回结构化提示 | introspect 误路由 exit 127 |
| **RC54** | whitespace 边界 fixture——**ledger 记载过时已更正**（P1-8 lenient 容错 v0.2.4 已落地） | apply_patch 空白脆弱 |
| 附加 | env 竞争测试串行化（`ENV_SER: Mutex`，毒化容忍）——修复 123+1 failed×2 flake | 过程缺陷 |

## P4.4 三审计（Node 07-09，零新 F0/F1）

- **Node 07 Intent**：活体抽查（material dump 喂料 → completed 零异常；vague delegation 悬空指代 → 诚实 failed 有界）；对抗黄金集 = 盲测输入逐字剧本（冻结于 `docs/data/p0-attribution-20260831/`，campaign 逐条回放即覆盖）。Intent 路由在 v0.2.20 无新缺陷。
- **Node 08 Decision/Terminal**：映射重验 + 漂移检查（gate 455→459 无回归），**无新增 authority**。
- **Node 09 Memory**：40 切片标记在树（lost-to-LLM deviation 维持）；archive 三件事（exists=PROVEN / searchable=通道在 / model can use=UNKNOWN 维持）；压缩真实发生（n13v2 +13KB 实证）；**COMPACT_DBG 修正：Agnes provider caps 实报 128K（非 512K 理论）→ 阈值 195,840 字符 ≈ 真实窗口 15%，保守方向安全；caps 申报值登记待核（供应商侧）**。

## P4.5 剩余排期（Node 10-15）与窗口分离裁决

| Node | 内容 | 归属 |
|---|---|---|
| 10 | SimUser Stage 2（B1 相关系数→联合分布采样，G1 门禁） | 执行窗口 |
| **11-12** | **SIMUSER Campaign 30 runs + Failure/Attractor Campaign 七场景** | **测试窗口**（指令书 `docs/hearth-test-window-campaign-directive-v1.md`） |
| 13 | 最小修复批（RC52 fixture 单测 + 截断标记规范）→ **v0.2.21** | 执行窗口（以 campaign 证据为输入） |
| 14 | 独立复测 campaign（v0.2.21） | **测试窗口** |
| 15 | CORE FREEZE CANDIDATE package | 执行窗口 + 顶层双签 |

**窗口分离裁决**：campaign 是对 v0.2.20 的**验收行为**——执行窗口自己修的自己验 = 既当运动员又当裁判，违反"审计工具须被审计"红线。故 campaign 由测试窗口执行，执行窗口不得自任 judge；版本冻结绑 tag（v0.2.20 → v0.2.21 链）。

**测试窗口任务序列（两份指令书，先后承接）**：①**Node 03 独立复现**（《P4-Node03独立复现测试指令 v1.1（顶层→测试窗口）》：A/B 复现 + 仪器修复单附录 A 由测试窗实施（diff 留痕交砺复核）+ C 条件补跑；环境 `.131`，数据落 `~/fa/p4-rerun/`）→ ②**Node 11-12 Campaign**（《hearth-test-window-campaign-directive-v1.md》：黄金集回放 + persona 分布对齐 + 七场景三样本）。判读分离：测试窗口只交数据与现象，正式归因判读 = 砺·评审，验收 = 顶层。
