# Hearth CORE FREEZE 顶层评审总览（砺·评审 整合版）

> **用途**：**单文件转发件**。供顶层设计窗口一次评审，无需同时打开多份材料。所有结论均标注来源文件与本窗独立取证结果。
> **日期**：2026-08-31　**窗口**：砺·评审（人工独立评审窗口）　**基线**：v0.2.18（`41cc070` / tag v0.2.18）
> **本窗立场**：不替顶层拍板；只负责把证据、冲突与可裁决选项摆到一处。所有"我同意/我反对"均给出可复核依据。

---

# 0. 一页纸裁决

## 建议结论

# **CORE FREEZE CANDIDATE — CONFIRMATION SUSPENDED（暂缓确认，非撤销）**

**触发的规则不是我发明的，是 CLOSURE-01 自己写的 NOT READY 条件之一**：

> 「长程失败具有稳定机制原因且未处置」→ NOT READY

**证据**：同一冻结版本 v0.2.18 的**真实用户盲测**（约 90 分钟、36 轮）显示——
**成功率 10/35（28.6%）；第 13 轮起至会话结束，连续 24 轮零成功。**
该失败序列可归因到一个**具体、可复现、可解释**的机制条件（T4 同图停滞 + 会话状态），而它**既未被修复，也未被登记为 F1/F2**。

**同时必须说清三件事（避免误读）**：
1. **不是回滚 v0.2.18**——代码基线继续有效（447/0 门禁、RC48 有界、RC49 闭环均未被推翻）；
2. **不是发现 F0 级假完成/静默丢失/沙箱绕过**——这三项本窗仍未见到反证；
3. **不是否定脚本化验证**——而是它证明了：**脚本化验证的覆盖范围 ≠ 真实交互可靠性**（ChatGPT 阶段报告 §2 的同名发现，本窗独立复核成立）。

## 与我上一份报告的关系（公开修订）

我的 `hearth-core-freeze-closure-verification-砺.md` 判定为 **B（独立验证未完成）**。
**本窗现将其修订为：B 保留，但在盲测证据并入后，按 CLOSURE 自订规则应进一步判为 CONFIRMATION SUSPENDED。**
原因：我当时核验的是**脚本化条件下的 F0 六项**（成立），而盲测暴露的是**脚本化未覆盖的真实交互条件**——两者不矛盾，但后者触发了 NOT READY 中的"稳定机制原因未处置"条款。**我的 §6 三步文档路径（A1/A2/A3）因此不再足以升 A**，须先完成 §6 的 P0 三项定向复现。

---

# 1. 四方证据对照（一句话看懂分歧在哪）

| 来源 | 性质 | 核心结论 | 本窗处置 |
|---|---|---|---|
| 执行窗口 CLOSURE-01 | 脚本化自验 | F0=0、447/0、CORE FREEZE | 门禁与结构层结论**成立**（我亲手复现）；Long-run/Behavior 层**覆盖不足** |
| 砺·评审 CV 报告 | 独立命令执行 + 4 次行为复现 | B；447/0 复核通过、RC48 复现 1/4、两项 audit 未闭环 | 保留；**程序结论上调为 SUSPENDED** |
| Claude 盲测报告 | 非脚本化真实交互（36 轮） | 直接冲突：裸 ERROR 泄漏、元诊断死循环、内容截断 | **核心事实确认**；**两处定性纠正**（见 §4） |
| ChatGPT 阶段评审 | 综合裁决 | P0-A/B/C；CONFIRMATION SUSPENDED | **同意其程序结论**；P0-B 因果假设获本窗实验**部分支持** |

---

# 2. 手工测试原始证据（本窗独立挖掘，非转述）

源文件：`release/手工测试v0.2.18.txt`（3,703 行，189 KB，用户真实 REPL 会话）

| 指标 | 实测值 | 取证方式 |
|---|---|---|
| 会话轮数 | 36 轮（run-001…run-036，run-031 无标记） | 逐 run 重建 |
| **成功率** | **10/35（28.6%）** | 每轮首个 `Task completed` / `Task failed` 判定 |
| **连续失败** | **run-013 起 24 轮零成功**（末次成功 = run-012） | 成败序列 |
| T4 同图停滞事件 | **27 次**（`2 consecutive replans produced identical TaskGraph`） | 全文匹配 |
| 裸后端 tracing 泄漏 | **6 处行首级**（`ERROR agent_core::r#loop: do_plan_inner failed…`），另有终端粘贴形态 | 正则匹配 |
| `introspect` 当 bash 调用 | 5 次（`exit code: 127` / `introspect: 未找到命令`） | 匹配 |
| 压缩摘要 Debug 外泄 | 16 行（`[compacted 会话摘要] 轮次目标: Text("…")`） | 匹配 |
| 目标修订 | revision 10 | 匹配 |

**成败序列（本窗重建，供复核）**：
```
成功：001 003 004 005 006 007 009 010 011 012（10）
失败：002 008 013 014 015 016 017 018 019 020 021 022 023 024
      025 026 028 029 030 032 033 034 035 036（24）
```
→ **Claude 报告「第 13 轮起 24 连败」的核心事实，经原始日志逐轮重建完全确认。**

**元诊断自激循环的原文证据**（T4 事件前最近用户输入）：
- L2818 用户：「又出现✗ Done (17 steps)。在开发 hearth 的 AI 说已经能完成长程任务了。目前看了是有问题的……」
- L2884 / L2906 用户：把上一轮的 `✗ stalled: 2 consecutive replans produced identical TaskGraph (N nodes)` **原样贴回追问** → 系统再次生成同构分解 → 再次停滞。
→ **"用户追问失败原因 → 系统复现同构分解 → 再次停滞"的自激形态成立**（Claude §1.2 的现象描述为真）。

---

# 3. 本窗补充实验：fresh vs polluted 的关键对照（新产出，此前无人做）

**问题**：24 连败是"元诊断任务类型天然易停滞"，还是"会话被污染后才停滞"？
**实验**：在 `.131` 用**全新 one-shot 会话**（每次独立进程、无历史）执行同类元诊断任务——读取上一轮失败报告并解释根因。
产物：`/home/wutao/li_probe/{prov.txt,summary.txt,probe1.log,probe2.log}`（binary 0.2.18）

| 探针 | 输入 | 结果 | 步数 |
|---|---|---|---|
| probe1 | 读 `/tmp/li_cv02_r3/.hearth/reports/641f7060-…/run-001.md` 解释失败根因 | **completed** | 6 步 |
| probe2 | 同上 | **completed** | 6 步 |

**结论（重要且反直觉）**：
> **元诊断任务本身不会停滞（fresh 2/2 completed）。** 因此 Claude §1.2 的"这类任务系统性地更容易撞上停滞检测"作为**任务类型归因不成立**；真正的相关变量指向**会话状态/上下文污染**（ChatGPT P0-B 的假设），即：**同一个任务在干净会话里能成，在被污染的长期 REPL 会话里连续 24 次失败。**

**这条改变了优先级**：Priority 0 的**B（Fresh vs Polluted 对照）是解开整个吸引子的钥匙**，应排在最前；而"T4 阈值是否太敏感"排在因果归因之后（ChatGPT §15"不要先修阈值"的判断，本窗实验予以支持）。

**附带观察（待查）**：两次 completed 探针的日志中均出现 1 次 `give_up` 字样却仍 `Task completed`（6 步、calls=2）。可能只是报告文本命中，也可能是一次内部 give_up 被拦截/降级——**建议列入 P0-C 一并核实**（若确为"give_up 出现但终态 completed"且无痕迹，则 F9/拦截链的可观测性有缺口）。

---

# 4. 对两份 AI 报告的逐条复核（含纠正）

## 4.1 Claude §1.1 裸 ERROR 泄漏 —— 现象确认，**定性纠正**
- ✅ 现象为真：日志确有 `ERROR agent_core::r#loop: do_plan_inner failed error=stalled: …` 直达终端。
- ⚠️ **但"反驳 Projection F0=0"这句说过头了**：CLOSURE §7 那一行写的是「**错误输出投影成功 = 零**」（即"把错误伪装成成功"）。裸 ERROR 泄漏是它的**反面**——吵闹而非伪装，**不构成 F0-6 的反例**。
- ✅ 正确归类：属 **Projection isolation（内部 tracing 未收敛到投影层）+ Projection completeness** 缺陷，与既有 **RC38 家族（内部/脚手架文本外泄到用户可见层）** 同源。
- 📌 裁决建议：不作为 F0，但**必须作为 P0 复现项**（§6-A），并同步修订 CLOSURE §7 表述（"投影成功失真=零"与"投影隔离/完整性"是三件不同的事，此前被合并成一行，是**指标合并导致的可观测性缺口**——与 RC51 同类）。

## 4.2 Claude §1.2 元诊断死循环 —— 现象确认，**机制纠正**
- ✅ 24 连败、T4 27 次、自激追问形态：全部经原始日志确认。
- ⚠️ **机制归因被本窗实验否证**：fresh session 2/2 成功 → 不是"这类任务天然易停滞"，而是**会话污染相关**。
- 📌 裁决建议：单独立项的必要性**成立**（后果严重：系统失效时用户连"问一下发生了什么"都走不通），但立项方向应从"任务类型弱点"改为「**会话状态污染下的持续性失败吸引子**」，并以 fresh/polluted 对照实验作为立项的因果前提。

## 4.3 Claude §1.3 / ChatGPT P0-C 内容截断（BUG-012）—— 确认，且触及定义缺口
- 系统内部完成分析，用户只见 `✓ Done (21 steps)`。**不是假完成**（产物落盘），但暴露一个此前没人定义过的口径：
  > **"Projection 正确" ≠ "用户可得"**。CLOSURE 只验了前者。
- 📌 建议在验收模型里新增一条判据（可编号）：**User-visible completion evidence**——完成的证据必须能完整到达用户，而不只在内部存在。

## 4.4 ChatGPT P0-B / §15 —— 支持，并已提供部分正面证据
- "不要先修 T4 阈值"：本窗实验支持（fresh 下阈值未造成失败，说明阈值不是主因）。
- Fresh vs Polluted 对照：本窗已给出 fresh 侧数据（2/2 completed），**polluted 侧可直接复用手工测试的 24 连败样本**，只需补一次"同任务在 polluted 会话中的对照跑"即可闭合因果。

## 4.5 两处 AI 均未提及、本窗补上的点
1. **CLOSURE §7 把三类不同的投影属性合并成一行**（成功伪装 / 隔离 / 完整性），导致"零"这个数字无法被证伪也无法被证实——这是与 RC51（坏测量方法）同型的**指标合并缺陷**；
2. **我的 CV-02 与盲测的表面冲突需要解释**：CV-02 中同版本 4 跑有 2 次 completed，盲测同版本 24 连败。二者不矛盾——**CV-02 是 fresh one-shot，盲测是 polluted REPL**，恰为 fresh/polluted 假设的又一组对照证据。

---

# 5. 当前状态：哪些仍成立，哪些转入复验

## 仍成立（未被盲测反证）
- **Structural**：447 passed / 0 failed / 1 ignored，FMT/CLIPPY/RT4/TEST 全 0（本窗 .133 亲手复现）
- **RC48 false stop**：有界、可解释、本窗复现 1/4 —— **ACCEPTED DEVIATION 地位不变**
- **RC49 规范链**：已闭环（本窗核实 `compacted.jsonl` 1,244,350 B @13:18 + session 归档 17,335 B）
- **RC50/RC51**：已登记（非交互审批抵消恢复 F1；行数测法缺陷 P2）
- **无假完成 / 无静默丢失 / 无沙箱绕过**（三个 F0 项暂无反证）

## 转入 revalidation（不得再作为"已验证"使用）
1. Projection isolation（裸 ERROR）
2. Projection completeness（内容截断）
3. Failure-state escape（24 连败 = 无法逃出失败态）
4. Diagnostic recovery（追问失败原因反而加深失败）
5. Long-session anchor continuity（后期 Plan 与当前输入脱节）
6. Context/session provenance（压缩摘要重复、Debug 结构外泄）

---

# 6. 下一步实验优先级（建议顶层照此派单）

## Priority 0（阻断项，必须完成后才谈 A）
- **A. 裸 ERROR 原样复现**：在干净环境复现 `do_plan_inner failed` 泄漏路径，定位是 tracing 未接管还是投影层缺分支。
- **B. Fresh vs Polluted 对照（最高价值）**：同一元诊断任务，fresh 会话（本窗已有 2/2 completed 基线）vs 复现盲测污染态的会话（可重放手工测试前缀）。判定因果是否为会话污染。
- **C. 用户可见完成证据测试**：区分"内部完成"与"用户可核实的完成"；顺带核实 §3 附带的 `give_up` 字样与 completed 共存现象。

## Priority 1（缺陷修复，可并入常规批次）
- **D. introspect 路由审计**（被当 bash 拼接，exit 127，5 次）
- **E. apply_patch 空白脆弱**（P1-8 复发）
- **F. compacted summary 投影审计**（重复打印 + `Text("…")` Debug 外泄）

## Priority 2（在 P0-B 因果明确后）
- **G. T4 阈值敏感性实验**（先归因后调参；当前**禁止**直接改阈值）
- **H. 会话级持续失败吸引子**的修复设计（若 B 证实污染因果）

## 原 A1/A2/A3 的处置
Interaction Surface Audit（A1）与 Decision-Terminal 锚点（A2）**仍然是欠账，但已不是升 A 的瓶颈**——优先级让位于 P0 三项。**A3（重跑 gate）在 P0 修复前无意义**（gate 只覆盖脚本化条件）。

---

# 7. 本轮新登记缺陷

| 编号 | 问题 | 级别 | 证据 |
|---|---|---|---|
| **RC52** | **会话级持续失败吸引子**：同一 polluted REPL 会话中连续 24 轮失败（run-013→036），T4 同图停滞 27 次；fresh 同任务 2/2 completed → 与会话状态强相关，机制稳定、未处置 | 🔴 **F0-clause（NOT READY 触发项）** | 手工测试逐轮重建 + 本窗 fresh 对照实验 |
| **RC53** | `introspect` 被当作 bash 命令拼接执行（exit 127），5 次 | 🟡 P1 | 手工测试（本窗计数） |
| **RC54** | `apply_patch` 空白/缩进脆弱匹配（P1-8 复发） | 🟡 P1 | 手工测试 run-013 |
| **RC38（扩展）** | 内部文本外泄家族新增两类：裸 tracing ERROR 行、compacted 摘要 `Text("…")` Debug 格式 | 🟡 P2 | 手工测试（6 处 / 16 行） |
| **RC51（沿用）** | 指标合并缺陷：CLOSURE §7 把"成功伪装/隔离/完整性"三属性合并为一行"零" | 🟡 P2 | CLOSURE §7 + 本窗复核 |

---

# 8. 引用文件清单（转发件溯源）

**本窗产物（独立取证）**
- `docs/hearth-core-freeze-closure-verification-砺.md` —— CV 独立验证报告（B 判定；含 §9 对两份 AI 回函的处置）
- `.133:~/li_cv01_run.log` —— §8 命令清单 + 4 项测试 + 全量 gate 原始输出（447/0/1）
- `.131:~/li_cv02/{provenance.txt,summary.txt,run1-4.log}` —— RC48 四次独立复现
- `.131:~/li_probe/{prov.txt,summary.txt,probe1-2.log}` —— fresh vs polluted 对照（fresh 侧）

**外部输入**
- `release/手工测试v0.2.18.txt` —— 用户真实 REPL 盲测（3,703 行，一手证据）
- `docs/hearth-v0.2.18-blind-test-report-v1-Claude.md` —— Claude 盲测报告
- `docs/Hearth v0.2.18 独立验证阶段评审报告-chatgpt.md` —— ChatGPT 阶段评审（P0-A/B/C）
- `docs/hearth-core-freeze-handoff-pack-v1.md` —— 四包整合 v2
- `docs/hearth-core-freeze-closure-01-final-report-v1.md`、`docs/core-freeze-review/freeze-decision.md` —— 执行窗口终裁与冻结文件

**总账**
- `docs/consolidated-remediation-ledger-v2.md`（RC48/49/50/51/52/53/54 均在表）

---

**最后一句**：v0.2.18 的代码没有变差，是我们对它的**证据边界**被一次真实交互重新划定了。暂缓确认不是倒退，而是把"脚本化条件下的可靠"与"真实交互中的可靠"这两件事，第一次分开记账。
