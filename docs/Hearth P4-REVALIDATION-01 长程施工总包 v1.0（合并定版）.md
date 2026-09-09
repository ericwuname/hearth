# Hearth P4-REVALIDATION-01 长程施工总包 v1.0（合并定版）

## Core Revalidation & SimUser Foundation

**日期**：2026-08-31（合并定版）
**执行窗口**：砺·执行　**测试窗口**：独立（Node 11/14）　**最终 Freeze 裁决**：人工 + 外部 AI 双窗（执行窗口无 Freeze 权）
**本文件地位**：**唯一派工依据**。取代 `Hearth CORE Revalidation & SimUser Foundation Construction Order v2.1-chatgpt.md`（其正文已全部并入并修正）；
`Hearth 拟真用户测试台与自反馈闭环 规划书 v2.0.md` 降为**规格附录**（persona 向量 schema / 检测器判据 / 语料数据细节，冲突时以本文件为准）；
P0/P3 Final Report 为**已并入的状态输入**（结论见 §0，无需回读）。
**当前基线**：v0.2.19（gate 455/0 四 RC=0，本窗已实测复核）；Core Freeze = **SUSPENDED**。
**总目标**：建测试台 → 真实交互重验证 → RC52 因果闭合 → 最小修复 → 独立回归 → 产出 **CORE FREEZE CANDIDATE**（不是 FREEZE）。
**核心原则**：`Test infrastructure → Baseline observation → Attribution → Minimal fix → Independent regression → External review → Freeze decision`

---

# 0. 当前状态快照（执行窗口必读——这是三份报告的合并结论）

## 0.1 P0-ATTRIBUTION-01 结论（已交付，四行记住）

| 问 | 结论 |
|---|---|
| Q-A 裸 ERROR 泄漏 | **CONFIRMED**：`do_plan_inner failed`（tracing::error!）→ subscriber stderr 直写（lib.rs:264）→ 完全绕过 Projection 层。**NEW F1**（Projection isolation+completeness，RC51 拆分）；非 F0（未被投影为成功） |
| Q-B RC52 归因 | **EVIDENCE GAP**：三版 harness 全失真（管道=DenyAllNonInteractive+EOF 忙等刷屏 56MB；script -qec=输入竞争 191MB；resume=恢复旧 goal 致 A 侧失真）。判定矩阵待 **PTY 交互式 driver**（= 本包 Node 01）闭合。部分信号：resume 链 polluted 继续型 3/3 failed（仅登记不作结论） |
| Q-C 截断+共存 | 截断点 = Projection 层终态分支（分析正文只在 run-XXX.md 文件）→ **NEW F1**；give_up/completed 共存 = **FALSE POSITIVE**（判定轨迹 vs 终态投影，反证已写）+ 可观测性缺口并入 RC51 |
| RC53/RC54 | RC53 = **model 层 tool misroute**（定位完成，修法方向：bash 对已知内部工具名返回结构化提示）；RC54 = 定位完成，fixture DEFERRED |

**继承给本包**：①`HEARTH_DEBUG_PLANNER_INPUT=1` dump 已在树（prompt/graph/compact 三类），Node 03 必须开启；②资源优先级 **RC47 族 > 会话残留 > 摘要放大器**；③零修复纪律已解除，但修复只限本包授权范围。

## 0.2 P3-BACKLOG-01 结论 + 内部矛盾裁决（顶层裁定）

- v0.2.19，gate 455/0（本窗实测 ✓）；W2 三连 / W7 边界 / RC34 / RC45 分账全部 CLOSED（锚点经砺实测确认）。
- **§6 vs §9 矛盾裁决**：**以 §6 表（带证据）为准**——RC24-B（EINVAL=0+introspect 可用）、RC2（零 contains 残留）、RC15（被拒投影为 failed）= **完成**；§9 的"剩余 5 项"系笔误。**真实复验剩余 = 4 项**：RC33 长会话 revision 对照 / 复测包（A-5·A-36、12·18 矩阵）/ RC34 终端级快照 / RC13 service 会话真机；另 RC29 = **NOT-IMPLEMENTED**（CLI 无 trust 命令，能力从未构建）→ 转 ledger 修正项，非复验。
- ⚠️ **.133 系统 PATH binary = 0.2.18（本窗实测）** vs source 0.2.19——**E7/PATH 滞后家族第三次复发**。Node 00 必修。

## 0.3 状态口径

ledger v3 §0 八词词汇表（"待复测"禁用）；证据分级七词：`OBSERVED / REPRODUCED / CONFIRMED / LIKELY / UNKNOWN / FALSE POSITIVE / DRIVER-INDUCED`。禁止：Observed→Confirmed、n=1→reproducible、script pass→behavior pass、self-report→evidence。

---

# 1. 全局红线与 STOP

**不可触碰区**（v2.1 §0.3 全文有效）：TaskGraph/TaskGoal/Terminal 九态/Approval·HardRedline/sandbox fail-closed/seccomp 总策略/ContextBuilder 核心模型/单一 Continuity 注入路径/Bridge/Subagent/TUI/MCP/Persona 功能化/Semantic Memory/动态 Verification 权威。

**特别禁令**：T4 阈值（loop.rs ~2642，仍=2）**禁止修改**（RC52 归因未闭合）；`MAX_HISTORY_MSGS=40` 入 archive 维持 CLOSURE disposition；file_issue **不进 Core**（§X）；不为 benchmark 好看降标准。

**STOP 条件**：安全模型设计级问题 / 单一事实源无法维持 / 测试台无法区分 Core 与 Driver 行为 / 需新增权限或外部副作用能力 / 越权改冻结语义 / 证据链与代码无法对账。普通失败、provider 波动、模型失败一律 Node 内归因，不得停包。

**执行节奏**：一次下发 → 连续施工 → 节点内自测验收归档 commit → 自动推进 → STOP 才停 → 最终一次报告。**Node 是施工边界，不是反馈边界。**

**环境纪律**：门禁只用 `~/run_gate_r2c.sh`；磁盘 `/home < 10G → STOP-RESOURCE`；**所有行号锚点引用前必须重新 grep**（砺实测两处 +4 漂移——报告成文后代码仍在动）。

---

# 2. Node 00 — Baseline Lock & Freeze Status Reset

1. 双 VM provenance 三查 + **sha256**；**`.133` 系统 PATH binary 重装对齐 v0.2.19 + 最小冒烟**（E7 家族第三次复发，本窗实测 .133=0.2.18）；记录 `driver` 与 `target` 两侧构建标识；
2. gate 基线 = **455**（记账基线）；
3. 读五份状态文件建索引：ledger v3 / P0 report / P3 report / SimUser v2.0（规格附录）/ 砺顶层总览；
4. 建 `docs/core-revalidation/` 证据总目录；
5. **P3 复验剩余精确清单（4+1）登记进 decision-status.md**：RC33 / 复测包 / RC34 终端快照 / RC13 service 真机（4 项 NEEDS-RERUN）+ RC29 NOT-IMPLEMENTED（ledger 修正）；
6. 正式改写工程状态：`CORE FREEZE SUSPENDED / CORE REVALIDATION IN PROGRESS`——旧 Freeze 不得再被引用为有效验收结论。
产物：`baseline.md` + `decision-status.md`。

---

# 3. Node 01 — SimUser Stage 1：测试台骨架（第一主战场）

## 3.1 双通道采集
用户侧 Reality（stdout/stderr/ANSI 渲染/裸 ERROR/状态行/截断内容）× Hearth 侧 Reality（telemetry/run report/event log/scratch/archive 时间戳/planner dump（debug 开关）/TaskGraph fingerprint/goal_revision/terminal），按 `session_id + run_id + timestamp` 对齐 → 产出 `internal_event ↔ user_visible_event` diff。

## 3.2 PTY Driver（**P0 三版失真陷阱清单——逐条规避，这是用三版废品换来的数据**）
1. **管道 stdin = DenyAllNonInteractive**（非 TTY 审批降级）+ **EOF 后 REPL 忙等刷 prompt（56MB）**——禁止 piped stdin 伪装交互；
2. **`script -qec` 输入消费竞争**（191MB 刷屏）——PTY 必须解决输入竞争；
3. **resume goal 语义**：`hearth resume <id> "<text>"` 恢复**旧 goal**（text 只入队消息，goal 不更新）——重放器/驱动器必须按此语义设计，禁止假设"resume 后 goal = 新输入"。
Driver API：`send / expect / wait_terminal / capture / timeout / interrupt`；保证**用户输入只被消费一次**；真实 PTY 驱动 `hearth chat / resume / repl`。
**⚠️ approval 通道状态必须显式记录（interactive / non-interactive / denied）——S4：campaign 中 `approval_denied` 默认归 DRIVER-INDUCED，需对照证据才可升格 Core defect。**

## 3.3 检测器（Stage 1 = 12 个）
v2.0 §4.8 的 1-10（projection_leak / consecutive_failure_run / t4_stall_burst / revision_explosion / compaction_adjacency / truncated_output / giveup_completed_coexist / tool_misroute / introspect_failure / user_visible_completion）**+ 前移 2 个**（砺 S5）：`anaphora_resolution_fail`、`escalation_run`——它们直接服务 Node 03/04 的 RC52 归因，属 P0 关键路径，不等到 Node 10。
每个检测器必须写明：input / algorithm / output / severity / run scope / false-positive notes。
**file_issue dry-run（砺裁决附条件）**：检测器 2/7/8/9 + projection_leak 的输出记录格式与未来 file_issue JSONL schema **同构**——用真实数据实证校准事件分类法，FIC-01 立项时不再是拍脑袋设计。

## 3.4 指标口径冻结
`tools/simuser/metrics.yaml`（版本随 persona/scenario 冻结，改定义=升版本，旧数据不混表）：success_rate = **逐轮归因口径**（run 边界取首个终态）；压缩"是否发生"只允许**字节数+mtime+文件数**；跨版本只在 v0.2.8+ 可比。**禁止** grep 关键词计数直接当成功率。

## 3.5 驱动模型登记（S1，强制）
**默认异源**（智谱 / 本地 Ollama qiyuan-8b）；同源 Agnes 仅作标注对照组单独成组（分离"模型共模"与"系统缺陷"）；每次 run 的 event log 记录 `driver_model / target_model`。
**S3**：driver timeout / interrupt 默认标记 DRIVER-INDUCED 或 SYSTEM-STALL——真实用户极少显式放弃（历史仅 1 次），只有 persona 显式生成放弃语才记 user_abandon。

## 3.6 测试台自检
未修复样本 → detector 应命中；干净样本 → 应归零。无法复现已知历史缺陷 → `STOP-SIMUSER-SELF-TEST`。
**SimUser 三形态验收**（v2.0 §7A，缺一不可）：①前段成功+后段连败形态；②入口事件="继续"型输入失败；③压缩晚于入口 ≥10 轮。

---

# 4. Node 02 — Projection Reality Audit

- **A isolation**：全库 `println!/eprintln!/tracing::error!/warn!` 清点（生产路径），确认绕过 Projection 直达终端的完整清单（P0 已定位 `do_plan_inner failed` 一条，补全其余）；
- **B completeness**：`internal_summary_size` vs `user_visible_body_size` 双通道 diff——"有内部结果但用户只见 Done 行"单独识别；
- **C correctness**：failed≠completed / error≠success / give_up≠hidden——不许只验颜色符号；
- **D Debug 泄漏**：`Text("...")` / Debug enum / 内部结构外泄清点；
- **E 截断族专项审计（砺 S7，实测 ≥8 处）**：constitution.rs:80 / context.rs:431 / loop.rs:1269,1665,3752 / agent-types:565 / client.rs:343 / lib.rs:661——逐点三问：有标记？用户可感知？可恢复？
产物：`docs/core-revalidation/projection-audit.md`。**先证明问题，再决定修复。**

---

# 5. Node 03 — RC52 因果实验（**P0 未竟矩阵的续作，不是重做**）

**禁止修改 T4 阈值。** 判定矩阵（P0 因驱动器失真未闭合，本 Node 用 Node 01 的 PTY driver 闭合）：

| 条件 | 内容 | n |
|---|---|---|
| A fresh 继续型 | 全新 REPL：先完成一个小任务 → "继续你的提议吧"（P0 侧因 goal 失真无效，**重跑**） | ≥5 |
| B contaminated 继续型 | 逐字重放 run-001..012 污染前缀 → 同一"继续"请求 | ≥5 |
| C resume 继续型 | P0 已有 3/3 failed（登记为 B/C 侧先验），补 fresh 对照 | ≥3 |

元诊断型作第二维佐证（P0 fresh 元诊断 2/2 旁证仍有效，但须按 REPL 形态重验）。
每跑记录：TaskGraph fingerprint / goal / goal_revision / compact events / **planner prompt dump**（`HEARTH_DEBUG_PLANNER_INPUT=1`，已在树）/ TaskControl·Conversation·Mutation 分类 / T4 count / give_up / terminal / projection。
**判定**（继承 P0 优先级）：fresh 败 → **RC47 族决策层主因**；fresh 过 + contaminated/resume 败 → **会话污染主因**（载体沿 Context/Anchor/Compact/Continuity 查）；三条件全同 → task type / decision / T4 方向。

---

# 6. Node 04 — RC52 Attribution Decision

六层归因 **mechanism / decision / model / provider / environment / DRIVER-INDUCED**（第七层为本包新增——SimUser 上线后 driver 缺陷必须与 Core 缺陷分开）。
输出五档：`CAUSE CONFIRMED / CAUSE LIKELY / CAUSE UNKNOWN / FALSE POSITIVE / DRIVER-INDUCED`。
**不得事先写"T4 是主因"；n=1 不得推结论。**

---

# 7. Node 05 — Projection Fix Batch（只修 Node 02 已确认项）

- **RC51-A isolation**：后端诊断日志可存在，但不再直灌用户投影通道（tracing writer 收敛/分级）；
- **RC51-B completeness**：终态 ≠ 一行状态——用户至少得到结果摘要/行动信息（**不是内部日志原样倾倒**，是"事实 → 用户可理解投影"）；
- 验收 = **Node 02 baseline vs 修复后同场景 before/after**；
- 修复完成 → **打 tag（v0.2.20）** + decision-status.md 登记（S6：campaign 必须声明所测 tag，跨 tag 数据不得混表）。

---

# 8. Node 06 — RC53 / RC54 最小修复

**RC53**：`introspect` 被 序列化进 bash（exit 127）——P0 已定位 model 层 tool selection；修法方向 = bash 工具对已知内部工具名返回**结构化提示**（"introspect 是内部工具，请用工具调用"），**禁止几十个黑名单关键词**——找 structured intent → shell serialization 的真正边界。
**RC54**：先建确定性 whitespace/indentation fixture，按 fixture 决定是否加容错；**不得让 patch matcher 变成"猜用户想改哪"**——保持确定性。
完成后 → tag 更新 + 登记。

---

# 9. Node 07 — Intent Adversarial Audit

用 SimUser 语料重建：情绪吐槽 / 闲聊 / 自我指涉 / 真歧义 / 吐槽+任务混合 / **material dump**（陈述喂料 38.3%——只给材料不下指令）/ **vague delegation**（3.6%）/ 高频真实输入（"你能回复我一下情况吗" / "上一轮为什么失败" / "继续" / "帮我看下刚才那个"）。
验收不只看分类准确，必须同时看：revision 是否变化 / 是否错误进 TaskGraph / 是否错误进 Recovery / 是否错误触发 Approval。

---

# 10. Node 08 — Decision / Terminal Mapping Audit

五 Decision × Terminal 正式映射表；五组语义区分（Stop≠GiveUp / Escalate≠Approval≠Stop / GiveUp≠Completion / Approval Denied≠一般 Tool Failure）；**Escalate + delegation active + 无人响应**的处置必须显式（不得隐式 fallback）。

---

# 11. Node 09 — Memory / Context Reality Audit

A. 40 切片：被切信息用户可知？可恢复？B. archive 三件事分开：exists / searchable / **model can actually use**；C. compaction：何时/为何/前后 token/archive/session binding；D. compression calibration 继续用真实 provider usage 交叉验证（32K/512K 不是理论结论）。

---

# 12. Node 10 — SimUser Stage 2

**B1 计算维度相关系数 → 联合分布采样器（G1 硬门禁：相关矩阵评审通过方可进 B2——独立采样拼出来的是假人）**；B2 persona 向量 schema + strategic 种子 + 10 矩阵场景（G2）；B3 补 3 个检测器（multi_intent_drop / paste_error_probe_quality / longpaste_truncation，总数 15）+ 自检；B4 四维验收（三形态 + 双模态分布对齐 + 任务域覆盖 + 结构覆盖，v2.0 §7B-D）。
**Q11 强制**：persona 语料（4,681 条）来自**单一人类主体**——campaign 报告头部必须附加声明，禁止"真实用户成功率"表述。

---

# 13. Node 11 — SIMUSER Controlled Campaign

版本冻结（绑定 tag）；**执行窗口停止修 Core**；30 runs 起步，指标稳定升 100；campaign 内禁边测边改；**测试窗口负责 run/capture/detect/archive，执行窗口不得自任 campaign judge**。报告头部强制 Q11 声明。

---

# 14. Node 12 — Failure / Attractor Campaign

专追：RC52 / RC48 / QA stall / revision churn / projection leaks / tool misroute。场景七类：Fresh / Long-session / After-stall / After-compaction / Resume / Continue / Failure-diagnostic。**每类必须留 success + failure + counterexample 三样本**（禁只存失败——幸存者偏差）。

---

# 15. Node 13 — Minimal Fix Batch

只有 Node 03-12 证据确认后执行。优先级：**P0** = Projection isolation/completeness + RC52 confirmed 根因；**P1** = RC53 / RC54 / Intent 边缘；**P2** = Memory slice / Archive recoverability / minor UX。每个修复：failure fixture → red → minimal diff → green → **independent rerun**。禁止为成功率改 benchmark。完成后 → **tag（v0.2.21）**。

---

# 16. Node 14 — Final Independent Campaign

版本冻结；执行窗口不再修代码；**独立测试窗口**执行：SimUser campaign + 历史回放 + 真人盲测回归。覆盖：Operational / Strategic / QA / Multi-intent / Anaphora / Failure diagnosis / Continue / Resume / Compaction / Projection。

---

# 17. Node 15 — Core Freeze Candidate Package

**禁止宣布 CORE FREEZE**——只能产出 **CORE FREEZE CANDIDATE**。生成 `docs/core-freeze-candidate/`：architecture-status / behavioral-status / projection-status / simuser-status / open-deviations / evidence-index / independent-review-request / freeze-candidate 八文件。最终裁决 = 人工评审窗 + 外部 AI 评审窗 + 至少一次独立命令/证据复核。

---

# 18. file_issue = DEFERRED（附 dry-run 条件）

本包不实施 file_issue（先证明 SimUser+检测器产生哪些高价值问题，再决定哪些事件值得进 Core 通道）。**条件**：五类确定性事件（stalled / give_up / approval_denied / verify_failed / 工具五态异常）的分类法在 Node 01 检测器层跑通并用真实数据验证（§3.3 dry-run）。不得因"以后需要"顺手进 Core。

---

# 19. 测试台也可能错（强制七分）

SimUser 结果必须区分：**Core defect / Driver defect / Detector defect / Environment / Provider / Model / Unknown**。测试台不是自动真理机；driver/detector 缺陷与 Core 缺陷分开记账。

---

# 20. Final Acceptance：两层标准

**Weak**（只证明结构完整）：所有 Node 执行成功 + gate 全绿 + 工具正常。
**Strong**（进入 Freeze Review 的唯一资格）：
1. Projection internal/user-visible 一致；2. RC52 在至少一个真实场景被因果解释；3. 无新的无限失败吸引子；4. Intent/QA/Task 路由稳定；5. Resume reteach=0；6. Compaction 不静默吞事实；7. Verification 不被 self-report 替代；8. 无 sandbox/approval 绕过；9. SimUser 30+ run campaign 指标稳定；10. 真人盲测不出现已知 F0 级缺陷。

---

# 21. Final Report 格式

Executive Summary / Version+Provenance / Nodes executed / Test infrastructure status / SimUser campaign / Projection comparison / RC52 attribution / Fixes / Regression / Strong Acceptance 逐项 / OPEN·ACCEPTED DEVIATION·UNKNOWN·DEFERRED / Evidence index / Independent review handoff / **Recommendation = CORE FREEZE CANDIDATE 或 NOT READY**。

---

# 附 A：本包取代与引用关系（冲突裁决规则）

- 本文件 **supersedes**：`...Construction Order v2.1-chatgpt.md`（正文并入并修正）；
- `规划书 v2.0.md` = **规格附录**（persona schema / 检测器判据 / 语料细节；冲突以本文件为准）；
- P0 / P3 Final Report = 状态输入（§0 已合并，无需回读；需要原始证据时按其 §10 溯源）；
- 砺复核（`评审：ChatGPT v2.1...砺.md`）= 评审轨迹；其 S1-S7 / A1-A3 / file_issue 裁决**全部采纳并入**（S1→§3.5 / S2→§12 / S3→§3.5 / S4→§3.2 / S5→§3.3 / S6→§7·8·13 / S7→§4E / A1→§6 / A2→§20 / A3→§14）。

# 附 B：顶层（本窗）补充清单（ChatGPT v2.1 与砺复核之外的新增）

1. **P3 §6/§9 矛盾裁决**（§0.2）：复验剩余 = 4 项 NEEDS-RERUN + RC29 NOT-IMPLEMENTED，以 §6 证据为准；
2. **.133 binary 滞后第三次复发**（§0.2/Node 00）：E7 家族纪律再强调——每次开工三查 + sha256，不是只在发版时；
3. **P0 三版 harness 失真陷阱清单**（§3.2）：管道/伪 PTY/resume goal 语义三条，driver 设计必须逐条规避——这是三版废品换来的数据；
4. **RC52 实验定位为"P0 续作"**（§5）：不是重做——P0 的 dump 仪器、部分信号、资源优先级全部继承；
5. **file_issue dry-run 具体化**（§3.3）：分类法校准前置到检测器层，FIC-01 立项时 schema 已被实证；
6. **版本 tag 链**（S6 具体化）：v0.2.19（现）→ v0.2.20（Node 05/06 修复）→ v0.2.21（Node 13 修复），campaign 逐 tag 绑定；
7. **P0 观测仪器衔接**：`HEARTH_DEBUG_PLANNER_INPUT` dump 已在树，Node 03 直接开启，不得重造。
