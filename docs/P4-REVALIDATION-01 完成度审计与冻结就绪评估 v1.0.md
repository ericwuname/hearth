# P4-REVALIDATION-01 完成度审计与冻结就绪评估 v1.0

**审计方**：执行窗（P4 施工者，对本人交付部分作自审）｜**日期**：2026-09-01 18:35
**审计对象**：`docs/Hearth P4-REVALIDATION-01 长程施工总包 v1.0（合并定版）.md`（Node 00–15 + §20 Final Acceptance）
**方法**：逐项对照总包条款 → 现场取证据（git tag / docs / .131 文件系统 / 在跑进程），不采信任何"我记得"。

---

## 0. TL;DR

| 问题 | 结论 |
|---|---|
| 总包完成度 | **Node 级 9/16 完成、3 部分完成、4 未启动（含 1 进行中）** |
| **能出最终冻结报告吗** | **不能。** 且缺口不在"报告没写"，而在**支撑数据不存在** |
| 最大缺口 | **Node 10（SimUser Stage 2，personas/scenarios 为空目录）/ Node 11（≥30 runs 冻结 tag campaign）/ Node 12（七场景 × 三样本）——三项均未执行** |
| Strong 十项 | 可判 ✅ 仅 **3 项**，🟡 **5 项**，🔴 **2 项** → **不具备 Freeze Review 资格**（§20 明定 Strong 是唯一资格） |
| 好消息 | Node 14 A 条件首跑 **5/5 干净通过**（v0.2.20 是 5/5 复现）——修复方向对，但**判读不归本窗** |

> 按总包 §17：Node 15 只能产出 **CORE FREEZE CANDIDATE**，且最终裁决 = 人工评审窗 + 外部 AI 评审窗 + 至少一次独立命令/证据复核。
> 当前连 CANDIDATE 的**输入数据**都缺，故本报告结论 = **NOT READY（继续施工）**。

---

## 1. Node 级完成度总表

| Node | 总包要求（摘） | 状态 | 证据锚点 |
|---|---|---|---|
| **00** Baseline Lock | 双 VM provenance、gate 基线 455、建证据总目录、状态改 SUSPENDED | ✅ | `docs/core-revalidation/baseline.md`、`decision-status.md` 在位 |
| **01** SimUser Stage 1 | PTY driver 规避三版失真陷阱 / 12 检测器 / metrics.yaml 冻结 / S1 驱动模型登记 / 测试台自检 | 🟡 **部分** | driver.py（真实 PTY、定步调、resume 语义已注明）✅；检测器 **13 个**（≥12）✅；`metrics.yaml` v1.0 冻结 ✅；**S1 异源驱动模型登记未做**（campaign 全走 agnes 同源，无法分离"模型共模"与"系统缺陷"）🔴；测试台自检仅做过 pilot 🟡 |
| **02** Projection Reality Audit | A/B/C/D/E 五维（E=截断族 ≥8 处三问） | ✅ | `docs/core-revalidation/projection-audit.md` 五节齐全 |
| **03** RC52 因果实验 | A×5 / B×5 / C×3 | 🟡 **部分** | A/B 已由测试窗跑（Step3：9/10 真实复现，B4=DRIVER_INDUCED）；**C 条件仍 BLOCKED**（R-1 仪器已修、干跑 8/8，待砺解封） |
| **04** RC52 Attribution Decision | 六层归因、五档输出 | ✅ | 已出 `CAUSE LIKELY`（decision 层 RC47 族 + 污染放大器）；升格 CONFIRMED 归顶层（α 议题） |
| **05** Projection Fix（RC51-A/B） | 修后 tag v0.2.20 | ✅ | tag `v0.2.20`；诊断日志不再直灌终端 + 产物清单投影 |
| **06** RC53 / RC54 | 最小修复 + tag | ✅ | RC53 bash 内部工具名结构化提示；RC54 确定性 whitespace fixture（判 CLOSED） |
| **07** Intent Adversarial Audit | 八类输入 + 四看（分类/revision/TaskGraph/Approval） | ✅ | `docs/core-revalidation/node07-09-audits.md` §Node 07 |
| **08** Decision/Terminal Mapping | 五 Decision × Terminal 映射 + 五组语义区分 | ✅ | 同上 §Node 08 |
| **09** Memory/Context Reality | 切片可恢复 / archive 三件事 / compaction / 压缩校准 | ✅ | 同上 §Node 09；Agnes caps 实测 128K（非宣传 512K） |
| **10** SimUser Stage 2 | B1 相关矩阵+联合分布采样器 / B2 persona 向量+10 场景 / B3 补 3 检测器（共 15）/ B4 四维验收 | 🔴 **未启动** | `.131:~/fa/simuser/personas` = **0 文件**、`scenarios` = **0 文件**；检测器 13（未到 15） |
| **11** Controlled Campaign | 版本冻结 + **≥30 runs** + 执行窗停止改码 + Q11 声明 | 🔴 **未达标** | T1 = 13 runs（v0.2.20）；本次 Node 14 = 10 runs（v0.2.21，A 完成 5、B 跑中）；**跨 tag、非单一冻结版本的 30 runs 尚不存在** |
| **12** Failure/Attractor Campaign | 七场景 × 三样本（success+failure+counterexample） | 🔴 **未启动** | 无对应数据目录 |
| **13** Minimal Fix Batch | fixture→red→minimal diff→green→独立复跑 → **tag v0.2.21** | ✅ | RC52 机制级修复 + 截断标记 4/2 + R-1 仪器；tag `v0.2.21` / commit `1366710`；gate 460/0/61 |
| **14** Final Independent Campaign | 版本冻结 + 独立测试窗执行 SimUser + 回放 + 盲测 | 🟡 **进行中** | `.131:~/fa/p4-rerun2` 在跑：**A1–A5 全部 completed/completed**，B 序列跑中（进程 410616 存活） |
| **15** Core Freeze Candidate Package | `docs/core-freeze-candidate/` 八文件 + 三方裁决 | 🔴 **未启动** | 目录不存在 |

**小计**：✅ 9 项｜🟡 3 项（01 部分 / 03 部分 / 14 进行中）｜🔴 4 项（10 / 11 / 12 / 15）

---

## 2. §20 Final Acceptance 逐项评估

**Weak**（结构完整）：✅ **已达成** —— Node 执行 + gate 全绿（460/0/61）+ 工具正常。
> 但总包明定：**Weak 只证明结构完整，Strong 才是进入 Freeze Review 的唯一资格。**

**Strong 十项**：

| # | 判据 | 判定 | 依据 |
|---|---|---|---|
| 1 | Projection internal/user-visible 一致 | 🟡 | RC51-A/B 已修；截断族 4 处已加显式标记、2 处豁免，**剩余若干处未标记**（详见 projection-audit §E） |
| 2 | RC52 在至少一个真实场景被因果解释 | ✅ | 机制闭合：run 边界清 `written_files` 销毁跨轮产物事实 → RC47 路由失效；修复后 A 条件 **5/5 干净**（疗效判读归测试窗/顶层） |
| 3 | 无新的无限失败吸引子 | 🟡 | 无反向证据，但**未经 30 runs 系统性验证**（缺口即 Node 11/12） |
| 4 | Intent/QA/Task 路由稳定 | 🟡 | Node 07/08 为 v0.2.20 抽查级审计，非 campaign 规模 |
| 5 | Resume reteach = 0 | 🟡 | T1 13/13 正确恢复 goal（revision 1）；但 **C 条件未闭环**，不能算强证据 |
| 6 | Compaction 不静默吞事实 | 🟡 | 40 切片标记 + 本次 4 处截断标记；"压缩=信息销毁"家族仍有未处置项（见 open deviations） |
| 7 | Verification 不被 self-report 替代 | ✅ | 盲区 C 产物确定性校验 + O-4 判定复验在位 |
| 8 | 无 sandbox/approval 绕过 | 🟡 **（v1.1 更正：原判 🔴 过重）** | **源码实证**：生产沙箱确无 `env_clear()`（`lib.rs:1078-1085`），但传入 env 由 `run_local.rs:219-236` 的 `tool_env(cfg)` 构造——**只注入 `HEARTH_EGRESS_ALLOWLIST` / `HEARTH_READ_ROOTS` 两项**，且 `ToolContext::default().env` 为空、全仓无 `std::env::vars()` 喂给工具上下文。→ **"key 直透子进程"不成立**；真实风险是**纵深防御缺口**（无兜底）+ 沙箱内缺 HOME/LANG/TMPDIR（缺功能非泄密）。另实测：沙箱内 **DNS 被拦**（curl 一律 000、`getent` 触发被拒 syscall 而 core dump），出网默认不可行 → 外泄面比先前估计更小 |
| 9 | SimUser 30+ run campaign 指标稳定 | 🔴 | personas/scenarios 空目录；无单一冻结 tag 的 ≥30 runs |
| 10 | 真人盲测不出现已知 F0 缺陷 | 🔴 | 无 v0.2.21 上的盲测记录 |

**Strong 结果：✅ 3 / 🟡 5 / 🔴 2 → 不满足"进入 Freeze Review 的唯一资格"。**
（v1.1 更正：第 8 项由 🔴 降为 🟡——原判基于"key 直透"的旧记录，源码实证不成立；详见 §3.1。**结论不变：仍不具备资格**，因为 🔴 的第 9/10 项未动。）

---

## 3. 关键新数据（本轮实测，供测试窗与顶层引用）

| 项 | v0.2.20（基线） | v0.2.21（本次） |
|---|---|---|
| A 条件（fresh，REPL「继续」） | **5/5 RC52 复现**（判定器复算，与砺 Step3 人工结论一致） | **矩阵 A1–A5 全部 completed/completed（0/5 复现）** |
| ⚠️ 分母口径 | — | **pilot A1（通路检查）实测 = failed，矩阵 A1 = completed**——同码同输入异果。主判据以矩阵 10 跑为准，pilot 作为**非确定性披露项**单列（已写入派工单 v1.1 预注册表）。若把 pilot 计入，比率为 1/6 而非 0/5——**两个数字都必须出现在报告里** |
| 判定器 | — | `~/fa/n14/n14_judge.py`，确定性分桶；口径已用旧数据自检对齐人工结论 |
| B 条件 | 4/5 复现 + 1 DRIVER_INDUCED | **跑批进行中**（进程 410616） |
| C 条件 | 数据作废（R-1 正则恒 None） | **BLOCKED**，待砺复核后解封 |

⚠️ 本窗只报数据，**不作疗效结论**（判读分离）：这是测试窗跑出来的、在我方代码上的结果，天然带自证风险，须由测试窗出报告、砺判读、顶层终审。

**本窗自陈缺陷一处**：派工单 v1.0 给的 C 行删除 sed 会连带删掉行尾 `])` → `SyntaxError`，测试窗已现场修复；v1.1 已更正为带 `ast.parse` 校验的写法（见派工单 §3）。

---

### 3.1 v1.1 更正：两条旧结论被实测推翻（自纠）

| 旧记录 | 实测 | 更正 |
|---|---|---|
| 生产沙箱缺 `env_clear()` → **key 直透 LLM 子进程** | `tool_env(cfg)` 只注入 2 个 HEARTH_* 变量；`ToolContext::default().env` 为空；全仓无 `std::env::vars()` | **不成立**。真实风险 = 纵深防御缺口（无兜底）+ 沙箱缺常规环境变量。🔴→🟡 |
| seccomp 放行 socket/connect → **bash curl 出网零管控** | 沙箱内 curl 对 example.com/baidu **均返回 000**；`curl -v` = `Resolving timed out after 15002ms`；`getent` = **被拒 syscall → core dump（rc=159）**；CA 证书可读 | **净效果不成立**：出不去网，**卡在 DNS**。旧记录作废 |

两条更正**都朝"风险比估计小"的方向走**，但都不改变冻结结论（第 9/10 项仍 🔴）。
证据与复现命令见 `docs/p5-foundation/N01-S5-决策备忘录.md`。

## 4. 还不能冻结的三条硬缺口

1. **SimUser Stage 2 未建**（Node 10）：personas/scenarios 空目录 → §3.5 强制的"异源驱动模型"与"分离模型共模"做不到，Node 11 的 30 runs 也缺采样器支撑。
2. **30 runs 冻结 campaign 与七场景三样本未跑**（Node 11/12）：现有 runs 跨 v0.2.20 / v0.2.21 两个 tag，按 §3.4「旧数据不得混表」与 S6「campaign 必须声明所测 tag」，不能拼数。
3. **Node 15 八文件未产 + 三方裁决未做**（Node 15）：即便前两项补齐，仍需生成 `docs/core-freeze-candidate/` 八文件并走人工评审窗 + 外部 AI + 独立复核。

另有 **§18 file_issue 条件未满足**：五类确定性事件的分类法尚未在检测器层用真实数据完整验证（检测器 13 个但未到 Node 10 要求的 15 个）。

---

## 5. 现在就能落的部分（不依赖 Node 14 数据）

| Node 15 八文件 | 可否先落 | 说明 |
|---|---|---|
| `architecture-status` | ✅ 可 | 三端口/九态/依赖 DAG 现状，可即时写 |
| `open-deviations` | ✅ 可 | 已有清单（env_clear、T4 阈值禁令、file_issue DEFERRED、Q11 单一主体、metrics.yaml 无阈值） |
| `evidence-index` | ✅ 可 | 全部证据目录已就位 |
| `freeze-candidate` | ⚠️ 半 | 可写骨架，结论段须留空待数据 |
| `behavioral-status` / `simuser-status` / `projection-status` | 🔴 待 | 依赖 campaign 与 Stage 2 数据 |
| `independent-review-request` | 🔴 待 | 依赖前七文件定稿 |

---

## 6. 建议路径（交顶层裁决）

**路径 A（按总包补齐，推荐）**：Node 14 收口 → 解 C 条件 → 建 SimUser Stage 2（persona/scenario 采样器）→ 单一冻结 tag 跑 ≥30 runs → 七场景三样本 → 真人盲测 → Node 15 八文件 → 三方裁决。
代价：周期最长，但符合总包，冻结结论可防御。

**路径 B（收窄范围申请豁免）**：承认 Node 10/11/12 未做，把冻结范围收窄为「**Core 决策层与投影层**」（不含 SimUser 能力线），并由顶层书面豁免 §20 Strong 第 9/10 项。
代价：冻结范围变小，且须在 `open-deviations` 显式登记豁免理由——**这是可辩护的，但必须是顶层书面裁决，不能由施工窗默认**。

**不建议**：现在就出"最终冻结报告"——数据的洞会被后续 campaign 撕开，反噬此前的可信度。

---

## 7. 开放偏差登记（拟入 `open-deviations`）

| ID | 偏差 | 处置 |
|---|---|---|
| OD-1 | 生产沙箱缺 `env_clear()`（dev 反有，倒挂）→ key 可直透 LLM 子进程 | 挂 P5 S4，未修；影响 Strong #8 |
| OD-2 | T4 阈值禁令（loop.rs ~2642，=2） | 总包特别禁令，维持不改 |
| OD-3 | file_issue = DEFERRED，条件未满足（检测器 13/15） | 待 Node 10 B3 补齐后复评 |
| OD-4 | Q11：persona 语料来自单一人类主体 | campaign 报告头部须声明，禁"真实用户成功率"表述 |
| OD-5 | `metrics.yaml` 有 forbidden 定义但**无数值阈值** | 只能判"稳定"不能判"达标"，Strong #9 受限 |
| OD-6 | 截断族剩余未标记项 | 本批 4 加 2 豁免，余数挂账 |
| OD-7 | C 条件 BLOCKED（R-1 解封待砺） | 影响 Strong #5 证据强度 |
