# Hearth Core 长程可靠性整合报告（P1-FA01 + P2-MC + P2-LR）

> **用途**：单文件跨总包沟通材料——供顶层设计窗口与外部 AI（ChatGPT）评审。三份独立 Final Report / 评审包为证据明细，本文件为整合视图。
> **执行窗口**：砺·执行　**日期**：2026-08-31　**版本演化**：v0.2.14 → **v0.2.17**（tag v0.2.17，HEAD `3b304dd`）
> **核验纪律**：所有结论可经 §8 命令清单独立复核；禁采信 commit message 与"已验证"自述；mechanism/decision/model 三层归因贯穿全程。

---

## 0. 一页纸总览

| 总包 | 终态 | 版本 | 核心成果 | gate |
|---|---|---|---|---|
| **P1-FAILURE-ADAPTATION-01** | PASS WITH DEVIATIONS / **CLOSED** | v0.2.15 | F1-F10 taxonomy、GiveUp 五路径收口（T4/预算旁路拦截、deadline 豁免）、双真机闭环 | 437/0 |
| **P2-MEMORY-CONTEXT-01** | PASS WITH DEVIATIONS / **CLOSED** | v0.2.16 | 单 run 压缩死代码修复、provider-aware 阈值、INV-M01 fixtures、RC40 改名 | 443/0 |
| **P2-LONG-RUN-ACCEPTANCE-01** | PASS WITH DEVIATIONS（待评审） | **v0.2.17** | RC47 修复、exit code 五态接线、40 切片标记、session 绑定、10 场景真机 | **447/0** |

**主线一句话**：Hearth 的失败分类→恢复策略→验证→压缩→resume 链路，已在真机（Agnes，budget 50-80，10+ 场景）证明 mechanism 层有界正确——零假完成、零无限循环、零沙箱绕过、零静默丢失（切片已打标记）、resume 零重教育。

---

## 1. P1-FAILURE-ADAPTATION-01（Node 00-15，三真机跑）

### 机制成果
- **Failure Taxonomy F1-F10**：`classify_failure` 纯函数（8 结构化输入，禁错误文本解析），每类正例+边界反例；Unknown 真实存在（无信号不冒充）。
- **策略矩阵**：`failure_strategy` 六策略（Retry 仅 Transient；verification≠retry；resource→Stop；model_judgment→Verify）。
- **GiveUp 五路径收口**：planner GiveUp 臂（修 D）+ **T4 停滞拦截**（真机红样本 1）+ **预算 headless abort 拦截**（真机红样本 2）+ deadline **故意不拦截**（Safety>Deadline，604s 权威实证）+ nervous Abandon（保持观察）。统一不变式：未核验的放弃要么先核验（Reserve ≤1 零和）要么打 `giveup_unverified`。
- **F9**：criteria 空的放弃必须可审计（scratch+summary 标记），失败报告不再折叠 "loop error"。

### 真机
- calcpkg 3 跑：r1/r2 红样本（两条旁路来源）；r3 deadline 权威终止+独立复验 criteria 必败（未假 completed）。
- mathlib r1：拦截拒绝假 completed（模型写标记但 bug 未修）；r3：**completed 21 步**（GIVE_UP_OVERRIDDEN，独立复验 2 tests passed）。
- wordcount：**46 步 completed 全闭环**（非 mathlib 样本）。
- **层归**：3 个 failed 全部 model+environment（Agnes 10 次 backoff / 28-55s）；mechanism 0 缺陷。

### 遗留（已入总账）
RC45（verify_replan_count 双上限零和）/ RC46（bash exit code 未接 → **P2-LR Node 02 已闭环**）。

---

## 2. P2-MEMORY-CONTEXT-01（Node 00-15，四问四答）

| 问 | 答案 |
|---|---|
| Q1 过早压缩？ | est 32k 字符 ≈ **9,009 tokens = Agnes 512K 的 1.8%**（API 权威；estimate_chars Debug+UTF-8 字节双重虚增 1.39-2.49×） |
| Q2 压缩是失忆主因？ | **修正**——单 run 压缩曾是**死代码**（交换全 push 进唯一 Turn 0 → 守门恒 false，真机 44k est 零压缩）；单 run 真正约束 = MAX_HISTORY_MSGS=40 静默切片 |
| Q3 调整后事实会丢？ | **不会**——INV-M01 可失败 fixture：state 层 7 类事实不在压缩路径，旧轮原文先归档后折叠，B 档可恢复 |
| Q4 参数还是架构？ | **provider-aware 参数标定 + 复用既有 archive**（情况 A），不建新 Fact Store |

### 落地
turn 粒度修复（单 run 压缩复活，红=单元+A1 真机 → 绿=单元+e2e）；estimate 诚实计数 + legacy 回滚（`HEARTH_COMPACTION_MODE=legacy`）；provider-aware 阈值（`HEARTH_CONTEXT_TOKENS` > caps × ratio 0.6 × 2.55 chars/token）；RC40 改名 `compact_pressure_pct`（旧名全库清零）。

---

## 3. P2-LONG-RUN-ACCEPTANCE-01（Node 00-15，10 场景真机）

### 四项修复（先红后绿）
1. **RC47 最小修复**（Node 01）：criteria 空 + written_files 非空 + 0 errors 的 give_up → 路由 Done（盲区C 产物校验兜底）；修 E 反例细化为无产物 QA 形态（QA 不被路由 completed）。
2. **exit code 五态接线**（Node 02，RC46 闭环）：`BashExitError` 类型化载体 → `ExitNonZero/ExitSignal/ToolTimeout`；五态矩阵 + Unknown 不冒充。
3. **40 消息切片标记**（Node 05，批-1"打标记"分支）：切片注入 System `[history note]`——保护对称性改善（压缩有 archive 先行，切片此前两样皆无）。
4. **chat 主路径 session 绑定**（Node 07，P2-MC OPEN-4 闭环）：`run_local_continue` run 前 `set_session_id`——归档落 `archive/<sid>.jsonl`。

### Reliability Matrix（三层成功口径）

| 场景 | 跑数 | mechanism | behavior | evidence（独立复验） |
|---|---:|---|---|---|
| Product long-run（textkit/geoutil） | 2 | 2/2 有界 | 1/2 | geoutil 1 passed ✓；textkit 未收敛（model） |
| Controlled failure ×2（sortlib） | 2 | 2/2 分类+有界 | 0/2（**false stop 2 例**） | **sortlib 2/2 passed（独立）** |
| QA 15+ 轮 | 1 | 1/1 零进 TaskGraph | 1/1 | 零 GoalMutation / 零 give_up（RC47 修复后） |
| Compact+resume 单跑链 | 1 | 1/1 | 1/1（**reteach=0**） | chainlib 1 passed ✓ |
| Multi-compaction stress | 1 | 1/1（2 压缩+修复+resume） | 1/1 | mathnotes 2 passed ✓ |
| Archive C-probe | 1 | 1/1（A+B 证） | 1/1（答案确定正确） | **C 未证明**（路径=保留上下文非 grep） |
| Approval/denial | 1 | 1/1 拒绝正确 | 0/1 | 沙箱不可绕过 ✓ |
| Deadline | 2 | 2/2 权威 | — | — |

**False 指标**：false completion **0** / false giveup **0**（RC47 路由正确）/ false replan **0** / **false stop 2**（登记 Node 01 后续）/ reteach **0**。

### Memory 两路径保护对称性（批-1 对照表）

| 维度 | Compaction 路径 | 40 切片路径 |
|---|---|---|
| 标记 | giveup/F9 场景有 | ✅ 已补 `[history note]` |
| 归档 | ✅ archive 先行 | 无（DEFER） |
| 恢复 | B 档 grep（模型 0 使用=OPEN） | 零通道（DEFER 入 archive） |

---

## 4. 测试与门禁演化

| 版本 | gate | 新增 |
|---|---|---|
| v0.2.15（FA01） | 437/0 | +8（taxonomy/策略/INV-M 前身/同工具重复） |
| v0.2.16（P2-MC） | 443/0 | +6（INV-M01 双 fixture/turn 粒度 e2e/阈值矩阵/honest counting） |
| **v0.2.17（P2-LR）** | **447/0 四 RC=0** | +4（rc47/五态矩阵/集成投影/slice marker） |

全部先红后绿（每项修复有真机或单元红样本记录）。

---

## 5. OPEN 汇总（三包合并，按层归属）

| # | 项 | 层 | 去向 |
|---|---|---|---|
| 1 | B 档 archive grep 模型 0 主动使用（"archive preserved but model recoverability unproven"） | model+机制 | Freeze Review 裁定（提示改进或工具化检索） |
| 2 | false stop ×2（修复完成后规划停滞终止） | decision | RC47 修复后续（规划停滞消费端复核） |
| 3 | MAX_HISTORY_MSGS 入 archive | 机制 | DEFER（已打标记） |
| 4 | chat 压缩归档 session 绑定时序 | — | **已修**（Node 07） |
| 5 | RC45 verify_replan_count 双上限 | 机制 | 总账待独立单 |
| 6 | constitution 6000 / interaction 样本补全 / n12r2 审批误触 / model 收敛方差 | 低风险 | DEFER/open |
| 7 | progress 语义扩展 | — | DEFER（须顶层批准，二阶效应清单在案） |

## 6. 事故披露汇总（两轮如实记录）

1. **.133 磁盘满**（96%→100%）：`codex_t/target` 为 symlink（共享缓存 76G）；满盘期间 9 个测试假失败（陈旧二进制）、target 变文件、gate RC=101——清理后全量重建，gate 通过。教训：Node 00 磁盘风险项即时清理。
2. **本地 C: 盘满（99%）→ .git 损坏**：refs 丢失 + 今日 loose objects 丢失。工作树完好，仓库重建（v0.2.16 基线 `463b315`）。**git 历史自 463b315 起可信；历史连续性以 CHANGELOG + docs 为准。**
3. paramiko/pkill 操作陷阱 ×3（channel 挂起、自匹配误杀、查错 VM）——已固化进执行窗 skill。

## 7. Provenance

| VM | source | version | binary | gate |
|---|---|---|---|---|
| .133（评审） | `/home/wutao/codex_t` | 0.2.17 | 0.2.16（**Node 00 对齐后跑 LR 前已建；gate 编译 0.2.17**） | `~/t_gate_mc_final.log`（443/0）+ `~/t_gate_lr_final.log`（447/0） |
| .131（执行） | `/home/wutao/codex` | 0.2.17 | `/usr/local/bin/hearth` = 0.2.17（sha256 记录） | 真机日志 `~/fa/lr_*.log` 等（已归档 docs/data/） |
| 本机 | tag v0.2.17 → `3b304dd` | 0.2.17 | — | — |

备份：`/home/wutao/hearth-v0.2.16-backup-20260831.tar.gz`。

## 8. 评审核验命令清单（三包合并，可直接执行）

`.133:~/codex_t`：

```bash
# FA01：taxonomy + 拦截链 + F9
grep -n "pub enum FailureKind" -A 12 crates/agent-core/src/terminal.rs
grep -n "GIVE_UP_ROUTED_TO_DONE\|GIVE_UP_OVERRIDDEN\|GIVE_UP_INTERCEPTED\|giveup_unverified" crates/agent-core/src/loop.rs | head
# P2-MC：turn 粒度 + honest counting + provider-aware + RC40
grep -n "一次工具交换 = 一个新 Turn" crates/agent-core/src/loop.rs
grep -n "legacy_estimate_chars\|effective_estimate\|HEARTH_COMPACTION_MODE" crates/agent-core/src/context.rs crates/agent-core/src/loop.rs
grep -rn "compact_pressure_pct" crates/agent-core/src/loop.rs crates/tools-builtin/src/status.rs; grep -rn "context_fill_pct" crates/ || echo "旧名清零 ✓"
# P2-LR：RC47 + 五态 + 切片标记 + session 绑定
grep -n "GIVE_UP_ROUTED_TO_DONE" crates/agent-core/src/loop.rs
grep -n "BashExitError" crates/tool-runtime/src/dispatcher.rs crates/agent-core/src/scheduler.rs
grep -n "history-slice-note" crates/agent-core/src/loop.rs
grep -n "set_session_id" crates/codex-cli/src/run_local.rs
# 测试
cargo test -p agent-core --lib test_inv_m01        # 2 passed
cargo test -p agent-core --lib test_rc47           # 1 passed
cargo test -p agent-core --lib test_five_state     # 1 passed
cargo test -p agent-core --lib test_history_slice  # 1 passed
bash ~/run_gate_r2c.sh                             # 四 RC=0, 447/0
```

`.131` 真机（日志已归档 `docs/data/` 两处）：

```bash
cargo test --manifest-path /tmp/lr_n13/mathnotes/Cargo.toml   # 2 passed
cat /tmp/lr_n08/CHAIN_RESULT.txt /tmp/lr_n13/STRESS_RESULT.txt /tmp/lr_n06/secret.txt
python3 /home/wutao/fa/collect_lr.py                           # Matrix 重算
```

---

## 9. Core Freeze 建议（五层结构现状）

| 层 | 状态 |
|---|---|
| Structural | ✅ 447/0 四 RC=0（三连 gate 演化可追溯） |
| Behavioral | ✅ mechanism 10/10 有界正确（LR Matrix）；历史主线 INV 全绿 |
| Long-run | ✅ 完整链（Intent→…→Compact→Resume→Complete）真机三例（chainlib/textkit*/mathnotes） |
| Evidence | ✅ 全部独立复验 + criteria 冻结 + 归档可反查 |
| Independent Review | ⏳ **待人工 + 外部 AI 双窗**（本文件即输入） |

**建议**：三包评审（本文件 + 三份 review pack）完成后，若残余 OPEN（§5）被裁定不阻塞，Hearth 进入 **CORE FREEZE REVIEW** 阶段。残余项均为 model/decision 层或已有界机制，无一条触及 Core 事实模型边界。

---

# 补章：CORE FREEZE REVIEW-01（2026-08-31，v0.2.18）

> 第四包执行完毕——**CORE FREEZE READY WITH ACCEPTED DEVIATIONS**（执行窗口建议；最终裁决=人工+外部 AI 双窗）。完整证据：`docs/hearth-core-freeze-review-01-final-report-v1.md` + `docs/core-freeze-review/`（11 文件 review package，review-index.md 为硬格式核验清单入口）。

## 核心结论

- **F0 = 0**（10+ 场景实证：零假完成/零静默丢失/零重教育/零无限循环/零沙箱绕过）。
- **F1 ×4**（全部"可解释、有界、已声明"）：①RC48 false stop（Reserve 零和 × GiveUp 时点；n12r3 反证机制在有 Reserve 时全链路正确 completed）②QA 轮 T4 2-node stall 高频（planner 无完成事实感知；n11 12/16）③archive C 未证明（探针污染 ×2，防污染设计要点已固化）④40 切片 lost-to-LLM（标记已落，恢复通道 DEFER）。
- **两项代码修复**：压缩阈值优先级修正（env 测试仪器 > provider-aware 注入——真机 COMPACT_DBG 实证 Agnes caps 128K→195,840 字符压垮仪器）→ **v0.2.18**；.133 provenance 滞后实修。
- **既有修复全部真机复证**：RC47（n12r3 全链路 completed）、五态接线、切片标记、session 绑定（n06v3 独立归档文件实证）。
- 真机 benchmark：chainfree canonical 单跑链（compact→stop→resume→complete，reteach=0）/ todoapi+units+configlib+mathnotes 独立复验 7 tests 全 passed / QA 16 轮（任务产物全达成）。

## 版本终态

**v0.2.18**（tag），gate **447/0 四 RC=0**（`.133:~/t_gate_cfr_final.log`），双 VM binary 0.2.18 对齐。残余 F1 修复方向两项**待顶层批准**（RC48 消费扩展 / planner 完成感知注入）——批准与否不改变 Freeze 边界。
