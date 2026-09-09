# CLOSURE VERIFICATION REPORT — 砺·评审独立审查验证

> **执行者**：砺·评审（人工独立评审窗口）　**日期**：2026-08-31 14:03–14:25　**窗口性质**：独立执行，非报告复核
> **对象**：`CORE FREEZE`（执行窗口自我声明，CLOSURE-01 / v0.2.18）是否满足 Hearth 自证的 evidence / independence / provenance 纪律
> **方法纪律**：不采信 commit message 与"已验证"自述；每条结论标注证据来源；无法执行者写 NOT EXECUTED；区分 paper review / source inspection / command execution / independent behavioral reproduction
> **结论预告**：**B — CORE FREEZE DECLARED, INDEPENDENT VERIFICATION INCOMPLETE**（CV-01 全过；CV-02 部分；CV-03 未成立）。无 F0，无新增 blocker。

---

## 1. Execution Provenance（本次执行环境）

| 项 | 值 | 取证方式 |
|---|---|---|
| 评审 VM | `.133` = `192.168.220.133`（hostname `wutao-VMware-Virtual-Platform`，user `wutao`） | ssh 实测 |
| 执行 VM（行为复现） | `.131` = `192.168.220.131` | ssh 实测 |
| `.133` source | `~/codex_t`，`Cargo.toml version = "0.2.18"` | `grep -m1 '^version' Cargo.toml` |
| `.133` binary | `/usr/local/bin/hearth --version` = **hearth 0.2.18 (unknown)** | 实测 |
| `.131` binary | **hearth 0.2.18 (unknown)**（CV-02 探针首行记录） | 实测 |
| 本机基线 | `git describe --tags --always` = `v0.2.18-5-g5dd8ea8`，tag v0.2.18 在库 | git |
| 执行身份 | 砺·评审（非执行窗口、非报告作者） | — |
| 落盘日志 | `.133:~/li_cv01_run.log`（9,603 B，130 行）／`.131:~/li_cv02/{provenance.txt,summary.txt,run1-4.log}` | 实测 |

**源码一致性取证（关键，含自我纠错）**：
```
loop.rs      .133 md5=4d3ae790f36da2b10a26ce8f11eea305   local md5=23c4f3d02421226eb25617515c1bdc41
context.rs   .133 md5=3240ca8bfa54192345af90df83bafaa0   local md5=1eef0d3624fa1c7203f4099f04eaaecb
terminal.rs  .133 md5=4b2beee3ef501783c276d38db199ab04   local md5=3c479d46690b9e9d3201be2d926787a8
dispatcher.rs .133 md5=43fbd34b9a3944a9915e006e016a0514  local md5=0f5f1f4d4c63c83dd29e5905cc6f84f6
run_local.rs .133 md5=933b8a29dc69f02b9097d3984893100c   local md5=933b8a29dc69f02b9097d3984893100c  ✅ 同
```
归一化（`\r\n`→`\n`）逐字节 diff 结果：**4 个文件各自只差 1 字节 = 文件末尾多一个空行**（`diff` 输出形如 `9472a9473 > 空`），**无代码漂移**。
⚠️ 我一度据 md5 差异准备报"源码错位"，经 `core.autocrlf` 排查 + 归一化 diff 自纠推翻——**审计工具自身必须被审计**。

---

## 2. CV-01 独立执行 §8 命令清单（.133 `~/codex_t`，全部亲手执行）

| # | 命令 | 执行 | RC | 实际结果 | 与报告一致 |
|---|---|:--:|:--:|---|---|
| 1 | `grep -n "pub enum FailureKind" terminal.rs` | ✅ | 0 | `261:pub enum FailureKind {` | ✅ |
| 2 | `grep -n "GIVE_UP_ROUTED_TO_DONE\|…\|giveup_unverified" loop.rs` | ✅ | 0 | 10 处命中：2566/2585/2600/2603/4124/4152/4180/4197/4200/**4223(ROUTED_TO_DONE)** | ✅ |
| 3 | `grep -n "一次工具交换 = 一个新 Turn" loop.rs` | ✅ | 0 | `2366:` | ✅ |
| 4 | `grep -n "HEARTH_COMPACTION_MODE\|CHARS_PER_TOKEN" context.rs loop.rs` | ✅ | 0 | context:201/213/223/226/243/**264(CHARS_PER_TOKEN=2.55)**/1150-1180；loop:4393/4407 | ✅ |
| 5 | `grep -rn "compact_pressure_pct" crates/` | ✅ | 0 | loop.rs:1333/1349、status.rs:91 | ✅ |
| 6 | `grep -rn "context_fill_pct" crates/` | ✅ | 1（无命中） | 空输出 → **旧名全库清零** | ✅ |
| 7 | `grep -n "pub struct BashExitError" dispatcher.rs` | ✅ | 0 | `20:pub struct BashExitError {` | ✅ |
| 8 | `grep -n "history-slice-note" loop.rs` | ✅ | 0 | `2217:` | ✅ |
| 9 | `grep -n "set_session_id" run_local.rs` | ✅ | 0 | `194:`、`328:`、`331:` | ✅ |
| 10 | `grep -n "env 测试仪器必须压过注入值" context.rs` | ✅ | 0 | `1168:` | ✅ |
| 11 | `cargo test -p agent-core --lib inv_m01` | ✅ | 0 | **2 passed / 0 failed**（122 filtered） | ✅ |
| 12 | `cargo test -p agent-core --lib test_rc47` | ✅ | 0 | **1 passed / 0 failed** | ✅ |
| 13 | `cargo test -p agent-core --lib test_five_state` | ✅ | 0 | **1 passed / 0 failed** | ✅ |
| 14 | `cargo test -p agent-core --lib test_history_slice_marker` | ✅ | 0 | **1 passed / 0 failed** | ✅ |
| 15 | `bash ~/run_gate_r2c.sh` | ✅ | 0 | **FMT_CHECK_RC=0 / CLIPPY_RC=0 / RT4_SOLO_RC=0 / TEST_RC=0**；gate 段合计 **447 passed / 0 failed / 1 ignored**（65.73s 主套件） | ✅ 与报告 447/0 精确一致 |

原始标记（摘自 `~/li_cv01_run.log`）：
```
[TEST_RC_inv_m01=0] [TEST_RC_test_rc47=0] [TEST_RC_test_five_state=0] [TEST_RC_test_history_slice_marker=0]
FMT_CHECK_RC=0  CLIPPY_RC=0  RT4_SOLO_RC=0  TEST_RC=0
=== GATE_END === [GATE_RC=0]
GATE_SECTION passed=447 failed=0 ignored=1
```

**CV-01 判定：SATISFIED。** 结构化门禁与四项关键测试在本窗口亲手复现，数字与报告逐项吻合。

**`.131` 真机命令**：本次**未**执行报告 §8 的 `.131` 段（`cargo test --manifest-path /tmp/cfr_n09/...` 等），原因：那些是历史 benchmark 产物路径，重跑它们不构成本次独立执行，且本窗口改用**更强的替代**——用原任务脚本做 4 次全新行为复现（见 §3）。故：**`.131` 历史命令 = NOT EXECUTED（以独立行为复现代替，不等价、不冒充）**。

---

## 3. CV-02 RC48 可重复性（4 次独立复现，.131，同任务同参数）

**条件**：复用 P2-LR Node 12 原始任务脚本（sortlib 冒泡排序受控失败，`--budget 60`，两条 acceptance：`cmd: cargo test` + `file: lib.rs contains n - 1 - i`），`HEARTH_ALLOW_NO_CGROUP=1`、`HEARTH_TASK_TIMEOUT_SECS=480`、`unset HEARTH_URL`，全新目录 `/tmp/li_cv02_r{1..4}`，binary 0.2.18。

| Run | GiveUp 发生 | 拦截链 | 核验结果 | Terminal | 独立复验（产物 cargo test） | 判定 |
|---|:--:|---|---|---|---|---|
| 1 | 否（无 GiveUp 标记） | — | — | **failed**（24 步，`approval_denied_noninteractive`） | `test tests::t_sort ... ok` / **1 passed** | 非 RC48 形态 |
| 2 | 是 | VERIFICATION_RESERVE → **GIVE_UP_OVERRIDDEN（Case D）** | passed | **completed** | **1 passed** | 机制正确 |
| 3 | 是 | VERIFICATION_RESERVE → **GIVE_UP_INTERCEPTED**（`failures=["file sortlib/src/lib.rs 不含 \"n - 1 - i\""]`） | **failed（当时修复未落盘）** → replan → 后续 `approval denied (non-interactive)` | **failed** | **1 passed** ← 事后产物有效 | ⭐ **RC48 false stop 复现** |
| 4 | 是 | VERIFICATION_RESERVE → **GIVE_UP_OVERRIDDEN（Case D）** | passed | **completed** | **1 passed** | 机制正确 |

原始证据（run3 拦截行，摘自 `~/li_cv02/run3.log`）：
```
WARN agent_core::r#loop: VERIFICATION_RESERVE(give_up interception): criteria unverified — verifying before accepting give_up reserve_used=1
WARN agent_core::r#loop: GIVE_UP_INTERCEPTED: acceptance failed — replan to fix (reserve consumed) failures=["file sortlib/src/lib.rs 不含 \"n - 1 - i\""]
```
复验输出（四跑一致）：
```
running 1 test
test tests::t_sort ... ok
test result: ok. 1 passed; 0 failed; 0 ignored
```

### 判定（严格按纪律措辞）
- **拦截机制可重复性 = ESTABLISHED**：GiveUp 发生的 3 次（run2/3/4）中，拦截链 **3/3 触发**，且行为分支与 Reserve/核验结果一致 —— 措辞用 `reproduced 3/3 under the tested condition`，**不外推为"机制普遍可靠"**。
- **RC48 false stop = 本样本中复现 1 次（1/4 跑；占拦截触发样本的 1/3）**：run3 终态 failed 而事后独立复验证明产物有效，与报告所述"2/14 同族复发"量级吻合（本样本 25%）。**不可"按需复现"**——它依赖 GiveUp 时点（model 层）与修复落盘时序，不能强制构造。措辞：`single reproduction observed in this sample (1/4); recurrence rate consistent with prior 2/14; not on-demand reproducible`。
- **RC48 的 F1 disposition 不变**：mechanism(bounded) + model，非 Core defect → **ACCEPTED DEVIATION**。本轮证据**不升级也不降级**。

---

## 4. CV-03 Audit Closure

### 4.1 Interaction Surface Audit

| 检查项 | 结果 |
|---|---|
| 独立 artifact 文件 | **不存在**（`docs/` 下无 interaction/surface 命名文件；`grep -rl InteractionRequest docs/` 仅命中 `core-freeze-review/authority-matrix.md`、`decision-terminal-map.md` 等叙述性文档） |
| 唯一相关产出 | P2-LR 评审包 §5（Node 03 Interaction 表面审计）—— **叙述 + 单测**，非独立 artifact |
| 覆盖范围 | 自述"8 类样本中 6 类有单测/真机覆盖，纯情绪/长混合 2 类仅单测——测试补全 DEFER" |
| CLOSURE Final Report 覆盖 | **零提及**（`grep -n Interaction hearth-core-freeze-closure-01-final-report-v1.md` = 0 命中） |
| 真实执行证据 | 仅单测（W8 fixtures），无本次可复跑的命令/日志 |

**状态：`Interaction Surface Audit = NOT ESTABLISHED AS INDEPENDENT EVIDENCE`。**
历史隐患仍在案（情绪吐槽被机械切成两个语法破碎选项），虽无真实 bug 证据，但"无证据"不等于"已验证无缺陷"。

### 4.2 Decision-Terminal Mapping Audit

| 检查项 | 结果 |
|---|---|
| artifact | ✅ `docs/core-freeze-review/decision-terminal-map.md`（存在，13 文件 review package 之一） |
| 内容质量 | 高：6 行映射表 + 五组语义区分 + Escalate/delegation/no-channel 专项，含源码层锚点说明 |
| **可执行证据密度** | **0**（`grep -cE "grep -n\|cargo test\|~/fa/\|\.log\|exit\|RC="` = **0 命中**） |
| 性质判定 | source inspection + narrative review，**非 command execution / 非 behavioral reproduction** |
| 诚实加分项 | 明确写出 `Stop ≠ Failed` 不成立（Stop 与 GiveUp 同落 failed，区分只在 reason 层）——未粉饰 |

**状态：`Decision-Terminal Mapping Audit = PARTIAL`。** 作为映射表成立；作为"逐项有证据"的独立验证不成立——每一行都缺一条评审可复跑的命令/测试锚点。

---

## 5. Independence Status（本次证据分层）

| 层 | 本轮覆盖 |
|---|---|
| paper review / narrative review | 整合报告 v2、CLOSURE Final Report、freeze-decision、review package 13 文件（已读） |
| source inspection | 本机 grep 复验 + `.133` 实测 10 条锚点 + 源码一致性归一化比对 |
| **local command execution（本窗亲跑）** | ✅ `.133`：10 grep + 4 targeted cargo test + 全量 gate（447/0/1，四 RC=0） |
| **independent behavioral reproduction（本窗亲跑）** | ✅ `.131`：sortlib 原任务 ×4 全新复现（拦截 3/3、false stop 1/4、产物四跑全部独立 `cargo test` 复验通过） |
| 仍仅为执行窗口证据 | RC49 重跑链（我以外的方式复核，见 §7-N3）、CFR/CLOSURE 全部真机 benchmark、archive C-probe、QA 16 轮、Node 09/10/11 产物、P2-LR Node 13 双压缩 |

---

## 6. Freeze Procedural Status

# **B — CORE FREEZE DECLARED, INDEPENDENT VERIFICATION INCOMPLETE**

理由（逐条对应 ChatGPT 的 A/B/C 判据）：

- **CV-01 = SATISFIED**：门禁与关键测试由本窗口独立复现，447/0/1 四 RC=0 精确吻合 → Structural + 部分 Evidence 柱**已独立验证**。
- **CV-02 = PARTIAL**：拦截机制 3/3 复现（可重复）；RC48 false stop 复现 1 次但**不可按需复现**（依赖 model 时点），样本仍小 → 足以支持"F1 有界"判断，**不足以支持"机制普遍可靠"**（报告也未如此声称）。
- **CV-03 = NOT SATISFIED**：Interaction Surface Audit 未建立为独立证据；Decision-Terminal Mapping 仅 PARTIAL（零可执行锚点）。
- **无 F0、无新增 blocker**：本轮未发现假完成、静默丢失、沙箱绕过、无限循环、双 authority。

因此：代码级冻结证据（门禁/测试/锚点/行为复现）**已被独立验证**；但**两个 audit 面仍属执行窗口声明**。按"reviewer cannot self-certify"纪律，不得给出 A。

### 从 B 到 A 的最小路径（均非架构，预计小时级）
1. **Interaction Surface Audit 立为独立 artifact**：8 类样本逐类给出（输入样例 / 触发判定 / 选项完整度 / 是否机械切句 / 证据命令或日志）；2 类仅单测者显式标 F2 DEFER 并说明理由；文件落在 `docs/core-freeze-review/interaction-surface-audit.md`。
2. **Decision-Terminal Mapping 补可执行锚点**：映射表每行附一条 `grep`/`cargo test`/日志路径 + 预期输出（沿用三包已验证的"硬格式"惯例）。
3. 两项完成后重跑一次 gate（应仍为 447/0）即可申报 A。**不需要新增代码、不需要新真机 benchmark。**

---

## 7. F0 / F1 / F2 与新发现

**F0 = 0**（本轮独立复核：假完成 0（四跑复验产物全有效且 completed 均有核验支撑）／静默丢失 0（切片=bounded declared loss）／reteach 0／无限循环 0／沙箱绕过 0（run1/run3 的审批拒绝恰恰证明门生效）／Projection 失真 0／provenance 错位 0（源码仅差尾随空行））。

**F1（4 项，与报告一致，本轮不升不降）**：RC48 false stop（ACCEPTED DEVIATION，本轮复现 1 次佐证其真实性）／QA completion-awareness／Archive C 未证明／40 切片 lost-to-LLM。

**F2（DEFER，与报告一致）** + 本轮新增观察：

- **N-1（新，建议登记 RC50）｜非交互审批拒绝会抵消恢复**：run1（24 步）与 run3（拦截触发 replan 之后）均以 `approval_denied_noninteractive` 终止——模型在 replan 后提交的 `bash, write_file, write_file, bash` 工具批被一次性模式的审批门结构化拒绝。**这是 RC24 的设计行为（非缺陷）**，但其副作用此前未登记：一次性模式下"有界恢复"可能被审批门吞掉，恢复链断在最后一步。建议：①写入 freeze-decision 的 "does not guarantee"（一次性模式需 `hearth repl` 或 `--approve-within session` 才能跑完整恢复链）；②登记为 F2 观察+后续单（非本轮施工）。
- **N-2（新，F2）｜review package 未同步到评审 VM**：`.133:~/codex_t` 下 `docs/core-freeze-review/` **不存在**（`ls` 报无此目录），即在评审 VM 上无法检视 Independent Review Package 的 13 份文件；本窗改用本机仓库审阅。属证据可及性缺陷，不阻塞但应修（同步或明确"评审 package 以本机仓库为准"）。
- **N-3（新，F2 文档缺陷）｜RC49 证据文件内部自相矛盾**：`docs/data/core-freeze-closure-20260831/node00-provenance.md` 写 `ARCHIVE_BEFORE=120 / MID=120 / AFTER=120 lines`，字面读像"压缩未发生"，与同文件自身 `compacted.jsonl +18KB @13:18（1225012→1244350）`、新 session 文件 `8335e03c-…jsonl 17,335B @13:19` 矛盾。
  **本窗独立核实（.131 实测）**：`compacted.jsonl` = **1,244,350 B @13:18**、`.config/hearth/archive/8335e03c-401b-4cf6-9f2c-21cb4520d322.jsonl` = **17,335 B @13:19**、`.config/hearth/sessions/8335e03c-…jsonl` = 11,733 B —— **字节数与 mtime 与报告精确吻合 → 压缩确已发生，RC49 已真正闭环**（我上一轮提出的质疑已被正确处置）。**但"120 lines"指标必须修正**（它很可能是 `wc -l` 取错了对象），否则下一位评审会据字面判定"重跑也没压缩"。
- **N-4（本窗自曝，审计工具自身必须被审计）**：CV-02 首次产物复验用 `tail -5` 截取 `cargo test` 输出，读到的是 **doc-tests 段**的 `running 0 tests`，一度逼近"验收空洞通过／存在假完成"的错误结论。改取全量输出后确认四跑均为 `test tests::t_sort ... ok / 1 passed`。**已自纠并披露**——若不自纠，会把一个 F0 级假警报写进本报告。

---

## 8. 提交给顶层规划窗口的裁决建议

1. **程序状态判为 B**（非撤销 Freeze、非确认 Freeze），并把 §6 的三条最小路径作为申报 A 的门槛——三条都是文档/审计工作量，不涉及架构与代码。
2. **F0 = 0 与 447/0 门禁已由独立窗口亲手复现**，这是本轮最有分量的正面结论：CLOSURE 的结构性主张站得住。
3. **RC48 是真的**：本轮独立复现了一次（产物有效而终态 failed），复发率与报告自述一致——它应当作为 ACCEDED DEVIATION 保留在"委托方需知"，**不得因为 n12r3 一次成功而降级**。
4. **RC49 已闭环**（我上一轮的质疑成立，且已被正确处置）；遗留的是该证据文件的指标表述缺陷（N-3）。
5. **新增 N-1（审批门抵消恢复）值得顶层注意**：它是"设计正确的安全机制"与"长程恢复"之间的真实张力，建议写入 does-not-guarantee 而非改安全边界（任何放宽审批的修法都应 STOP）。

**最后一句**：本轮不是为了证明 Hearth 已经完美，而是为了证明它是否达到自己定义的"可以冻结"的证据门槛——**代码证据够，两项 audit 证据不够**。

---

---

## 9. 增补：对外部评审回函（ChatGPT / Claude）的处置

> 处置原则：外部意见**不因权威而采纳，只因可复核而采纳**。以下四条，能查的我已查，不能查的写明。

### P-1（Claude 第 1 点）—— 接受：把「2/4」显式写出

原报告把成功率拆在表格行里，确实被表格格式吞掉了。显式重述：

> **4 次独立复现中，完整走完「受控失败→修复→核验→完成」且终态 completed = 2/4（50%）。**
> run1 止于审批拒绝（RC50，未走到 GiveUp）；run3 止于 RC48 false stop（终态 failed，事后产物有效）。

**必须附三条限定，否则 50% 会被误读**：
1. **n=4**，样本极小，任何外推为"系统成功率"都是错的；
2. 两个失败形态**均属已分级偏差**（RC50 / RC48），不是未知缺陷——这恰恰是 Freeze 想要的状态：失败可归类；
3. 任务固定为 sortlib 单一拓扑 + Agnes，历史同参数跑间步数方差达 14–76 步。
建议统一措辞：`2/4 completed under this protocol; both failures map to pre-classified deviations (RC50, RC48); not a system-wide success rate.`

### P-2（Claude 第 2 点）—— 接受并升档：RC50 由 F2 → **F1（Accepted Deviation）**

不只是同意，我补一层论证：Hearth 的核心承诺"可信委托、放心走开"，**在操作上等价于"非交互/无人值守模式下跑得通完整恢复链"**。实测显示该链的最后一步可能被审批门以 non-interactive 为由整体拒绝（run1 24 步、run3 拦截触发 replan 之后），**命中的正是核心承诺最薄弱的那个具体模式**，与 RC48 同属"在最需要可靠的场景（没人盯着）里发作"的类别。因此应与 RC48 同等可见度。

**但不是 blocker**：唯一"修法"方向是放宽审批门 = 触碰 STOP-3 / STOP-5 安全边界，禁止。处置只能是：写入 does-not-guarantee + 调用方指引（一次性模式需 `hearth repl` 或 `--approve-within session`；run1 日志中系统已自带该指引，缺的是冻结文件的可见度）。

**我已核实升级依据**：`docs/core-freeze-review/freeze-decision.md` 对 approval/审批/headless/非交互的命中数 = **2**，其中 does-not-guarantee 段**零覆盖**（仅 guarantees 段有一行泛泛的"approval 结构化拒绝，不可绕过"）。→ 升档有据，非情绪判断。总账 RC50 行已由 F2 改为 F1。

### P-3（Claude 第 3 点）—— 接受，且**已执行反查**（本轮新产出）

按"坏测量方法是否被复用"这一信号筛查同类指标：

| 实例 | 位置 | 测法 | 现状 |
|---|---|---|---|
| RC49 归档证据 | `docs/data/core-freeze-closure-20260831/node00-provenance.md` | `ARCHIVE 120 lines / 3→4 files` | ❌ **已被证伪**：行数 120 不变，而字节 +18KB（我已用字节+mtime 独立核实底层事实为真） |
| P2-MC Task A 归档证据 | `docs/hearth-p2-memory-context-01-review-pack-v1.md:126` | `archive 105→110 行` | ⚠️ **同一测法用于同一类主张**；但该柱另有字节证据 `compacted.jsonl 934KB→1017KB`（同文件 §5）→ **结论可采信，行数那一句应删或加注** |

**结论：被证伪的不是某个数字，是某一类测量方法。** 建议立为硬规则（写入后续包的执行纪律）：
> 压缩/归档"是否发生"的主张，**证据只允许 字节数 + mtime + 文件数**；行数指标**一律不得单独作为发生性证据**（`wc -l` 统计换行符，末行无换行符即系统性少计——这正是 RC49 那处最可能的机理）。
并建议对 P2-MC Node 14 Task A 的行数指标做一次低优先级回勘（字节证据已成立，不影响结论）。

### P-4（Claude 第 4 点）—— 接受，改判表述

原表述"到 A 只差三步文档工作，不涉及代码"确实预设了 A1 结果无害。**改为**：
> 预计只需文档/审计工作；**若 Interaction Surface Audit 坐实真实缺陷，按其分级重新评估路径**——F1 则仍可判 A（带新增 Accepted Deviation），F0 则转 NOT READY。不得在审计完成前预设其结果无害。

**该隐患的判定口径预写**（历史隐患：情绪吐槽被机械切成两个语法破碎选项）：
- 坐实标准 = 能用**真实输入原文**复现"机械切句 / 语义丢失 / 选项破碎"；
- 若坐实 → 通常按 **F1**（交互层缺陷不触碰 Core 事实模型，一般不构成 F0）；
- 例外：若该缺陷导致**事实污染**（如把模型生成的片段当作用户意图回灌进事实层），则按 F0 处理。
另给执行窗口一条防"补文档表演"的约束：**A1 的 8 类样本每类必须附「输入原文 + 系统实际响应原文 + 判定」**，只写结论一律退回；2 类仅单测者必须显式标注"仅单测、未真机验证"。

### 对 ChatGPT 回函的回应
接受其五层状态重画与"下一步机械执行 A1/A2/A3、不再让任何 AI 评价报告"的处置。补一条执行约束：A2 每行锚点必须是 **命令 + 预期输出**（沿用三包已验证的硬格式），不接受"见源码"式锚点。

---

*本报告全部结论可经§1 环境复现；命令与日志路径已在文中给出。砺·评审零代码改动、零测试修改。*
*§9 为对 ChatGPT / Claude 回函的处置；其中 P-3 为本轮新增的独立反查产出。*
