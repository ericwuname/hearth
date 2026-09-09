# P4 Node 14 测试报告 v1.0（转各窗口）

> 测试窗口（砺·评审 代行）产出 · 供执行窗 + 各窗口转发同步
> 被测对象：hearth v0.2.21（commit `1366710` / tag `v0.2.21`，md5 `5e5ac1a327d806f27af77b2e8b89f14e`）
> 复测目标：RC52 修复（跨 run 产物事实回填）是否降低 fresh 失败率
> 环境：VM `.131`（192.168.220.131，wutao/123456），provider `agnes`/`agnes-2.5-flash`（挪威 VPN 出口）

---

## 〇 结论速览 / TL;DR

- **全矩阵 10 runs 跑通**：A×5（fresh 新开 + "继续"）+ B×5（13 行前缀重放 + "继续"）。
- **判定器结果（v0.2.21）**：
  - A 相：**5/5 CLEAN_PASS，0% 复现**。
  - B 相：B2–B5 全 `RC52_REPRO`（give_up → Task failed）；B1 因日志含 2 处 `send request failed` 字符串被判 `PROVIDER_UNREACHABLE` 出分母（但 B1 同时有 14 处 give_up，疑为 RC52 复现被瞬时 provider 噪声误判）。
- **失败率对照（分母锁定）**：

  | 相 | v0.2.20 复现率 | v0.2.21 复现率 | 修复效果 |
  |---|---|---|---|
  | A fresh | 100% (5/5) | **0% (0/5)** | ✅ 完全生效 |
  | B 前缀重放 | 100% (4/4 有效) | **100% (4/4 有效)** | ❌ 未生效 |
  | 总有效样本 | 100% (9/9) | **44% (4/9)** | 分相分化 |

- **预注册判据（派工单 §4：fresh 显著 <40%）：达标**（A 相 0% << 40%）。
- **关键字段（派工单 §7）**：hydration 标记全 0（含 A 相）、truncation 标记全 0、provider 不可达仅 B1 出现 2 次。⚠️ **hydration=0 是待解疑点**（见 §七、§十一）。
- **判读分离声明**：本报告只呈数据与判定器原始输出，**不自行裁定修复"有效/无效"**，效力交砺/顶层（见 §十一）。

---

## 一 派工单与开工三查

依据 `docs/P4-Node14测试派工单 v1.0（执行窗→测试窗口）.md`：

- **开工三查**（`.workbuddy/n14_precheck.py`）：
  - `hearth --version` → `0.2.21`
  - `md5sum /usr/local/bin/hearth` → `5e5ac1a327d806f27af77b2e8b89f14e`（与派工单 §1 锁定的 md5 一致）
- **五项预检**：`RESULT: ALL_GREEN_GO`
- **目录隔离**：`~/fa/p4-rerun2`（禁止写 `~/fa/p4` 与 `~/fa/p4-rerun`，避免与历史结果串扰）

---

## 二 实验设计

| 相 | 构造 | 触发锚 | 观测目标 |
|---|---|---|---|
| A（fresh） | 新会话 → 完成任务 → REPL 输入"继续你的提议吧" | RC52 = REPL "继续" 且**未显式恢复目标** → planner 重规划已完成任务 → give_up → Task failed | `continue_terminal` |
| B（前缀重放） | 13 行自测对话前缀重放 → 末行"继续你的提议吧" | 同上，但带历史上下文 | `continue_terminal`（B 臂字段名 `prefix_last_terminal` 为前缀终态，统一以 continue 终态为准） |

- **分母锁定原则**：provider 不可达 / 仪器缺陷（DRIVER_INDUCED）/ 跑批异常（RUNNER_ERROR）**不计入失败率分母**。
- **判定器**：`n14_judge.py`，纯确定性字符串/计数，模型"感觉"不作证据。

---

## 三 全矩阵原始结果

| 条件 | task/prefix 终态 | continue 终态 | session_ids | 规模 |
|---|---|---|---|---|
| A1 | completed | completed | 219a8560 | 7.1 KB |
| A2 | completed | completed | 5fc94d25 | 7.1 KB |
| A3 | completed | completed | 2682c624 | 8.6 KB |
| A4 | completed | completed | 979b0161 | 5.4 KB |
| A5 | completed | completed | e25b7dd4 | 5.4 KB |
| B1 | failed | failed | fb47e3de, 1c67dd2b | 65.9 KB |
| B2 | failed | failed | 24dc8bf8 | 95.7 KB |
| B3 | failed | failed | 152b50cd, 03d6209b | 51.2 KB |
| B4 | failed | failed | 15a49c88 | 68.1 KB |
| B5 | failed | failed | faceb654, e980f065, a5050aea | 56.0 KB |

**逐日志标记计数**（本地复算，re 正则与判定器同口径）：

| 条件 | provider_fail | give_up | hydration | truncation | 日志字节 |
|---|---|---|---|---|---|
| A1 | 0 | 1 | 0 | 0 | 7144 |
| A2 | 0 | 1 | 0 | 0 | 7141 |
| A3 | 0 | 0 | 0 | 0 | 8561 |
| A4 | 0 | 1 | 0 | 0 | 5363 |
| A5 | 0 | 2 | 0 | 0 | 5372 |
| B1 | **2** | 14 | 0 | 0 | 65911 |
| B2 | 0 | 21 | 0 | 0 | 95663 |
| B3 | 0 | 17 | 0 | 0 | 51202 |
| B4 | 0 | 17 | 0 | 0 | 68080 |
| B5 | 0 | 13 | 0 | 0 | 55968 |

> 注：`give_up` 标记数 ≠ 终态失败（A 相内部规划也会打 give_up 后恢复，终态仍为 completed）。判定器以 `continue_terminal` 字段为终态真相；`give_up` 仅作佐证。

---

## 四 判定器输出（verbatim）

```
==============================================================================
Node 14 判定结果 —— results.json
==============================================================================
  A1     CLEAN_PASS           task=completed              cont=completed
  A2     CLEAN_PASS           task=completed              cont=completed
  A3     CLEAN_PASS           task=completed              cont=completed
  A4     CLEAN_PASS           task=completed              cont=completed
  A5     CLEAN_PASS           task=completed              cont=completed
  B1     PROVIDER_UNREACHABLE task=failed                 cont=failed
  B2     RC52_REPRO           task=failed                 cont=failed
  B3     RC52_REPRO           task=failed                 cont=failed
  B4     RC52_REPRO           task=failed                 cont=failed
  B5     RC52_REPRO           task=failed                 cont=failed
------------------------------------------------------------------------------
  CLEAN_PASS           5
  RC52_REPRO           4
  PROVIDER_UNREACHABLE 1
------------------------------------------------------------------------------
  有效样本 9（剔除 1：provider 不可达/仪器缺陷/跑批异常）
    RC52_REPRO           4/9 = 44%
    CLEAN_PASS           5/9 = 56%
  >>> RC52 复现率 = 44%
  对照：v0.2.20 执行窗自报 fresh = 40%；Node 14 预注册预期 = 显著低于 40%
==============================================================================
```

---

## 五 失败率对照表 v0.2.20 ↔ v0.2.21

口径统一为 **test-window 同矩阵复测**（v0.2.20 = 此前 Step3 复测；v0.2.21 = 本 Node 14）。

### v0.2.20（基线）

| 相 | 有效 n | RC52_REPRO | CLEAN_PASS | 剔除 | 有效复现率 |
|---|---|---|---|---|---|
| A fresh | 5 | 5 | 0 | 0 | 100% (5/5) |
| B prefix | 4 | 4 | 0 | 1（B4 DRIVER_TIMEOUT） | 100% (4/4) |
| 合计 | 9 | 9 | 0 | 1 | **100% (9/9)** |

### v0.2.21（本测）

| 相 | 有效 n | RC52_REPRO | CLEAN_PASS | 剔除 | 有效复现率 |
|---|---|---|---|---|---|
| A fresh | 5 | 0 | 5 | 0 | **0% (0/5)** |
| B prefix | 4 | 4 | 0 | 1（B1 PROVIDER_UNREACHABLE） | **100% (4/4)** |
| 合计 | 9 | 4 | 5 | 1 | **44% (4/9)** |

### Delta（核心结论）

| 维度 | v0.2.20 | v0.2.21 | 变化 |
|---|---|---|---|
| A fresh 复现率 | 100% | 0% | **↓ 完全消除** |
| B prefix 复现率 | 100% | 100% | **持平（修复未触及）** |
| 总有效复现率 | 100% | 44% | ↓ 但被 B 相拉高 |

**结论：修复对 RC52 是"选择性生效"——清除了 fresh 续作路径的复现，但前缀重放续作路径的复现率纹丝不动（100%）。** 总复现率从 100% 降到 44%，完全是 A 相贡献；B 相是 unchanged failure。

> 敏感性：若把 B1 改判为 RC52_REPRO（见 §六 歧义），则 v0.2.21 有效样本=10、RC52=5、复现率=50%，B 相仍=100%（5/5）。两种口径下"A 修好、B 没修好"的结论不变。

---

## 六 B1 判读歧义（判读分离，交砺/顶层）

判定器把 B1 标 `PROVIDER_UNREACHABLE`，**但**其日志同时含 14 处 `give_up` 与仅 2 处 `provider_fail` 字符串。判定器优先级规则（`provider_fail` 先于 `cont.startswith("failed")` 判定）导致一旦日志出现 provider 失败字样即出分母——即使终态失败由 give_up 主导。

- **两种口径**：
  1. 判定器口径：B1 出分母 → 有效样本 9，B 相 4/4 复现。
  2. 实质口径（give_up 主导）：B1 实为 RC52 复现 → 有效样本 10，B 相 5/5 复现。
- **不影响主结论**：无论取哪种，B 相复现率均 ≥80%，修复对 B 相无效。

此即派工单 §9"判读分离"要交砺/顶层裁定的典型样本：**环境噪声（瞬时 provider 失败）与任务失败（RC52 give_up）在同一日志共存时，分母归属需人工裁决**。

---

## 七 三项关键字段采集（派工单 §7）

| 字段 | 期望 | 实测 | 说明 |
|---|---|---|---|
| hydration 计数（`RC52_ARTIFACT_HYDRATION`） | 修复路径应触发 | **全 10 跑 = 0** | ⚠️ 见下 |
| 截断标记（`[... N chars truncated]`） | 无 | **全 10 跑 = 0** | 本轮无上下文截断 |
| provider 不可达 | 作分母剔除依据 | **仅 B1 = 2 次** | 其余 9 跑 provider 稳定 |

### ⚠️ hydration=0 疑点

修复名是"跨 run 产物事实回填"，其插桩标记 `RC52_ARTIFACT_HYDRATION` 在**全部 10 个日志（含已通过的 A 相）均为 0**。这意味着：

- 要么回填逻辑走了但未打该标记（插桩缺失/标记字符串不符）；
- 要么 A 相 0% 复现由**其他机制**带来（非预期回填路径）。

这与"断言不能失败=断言不存在"红线同源：**行为改善了，但修复宣称的机制路径未被观测证实**。建议砺/顶层补一道插桩或单测验证，确认修复真走了"跨 run 产物事实回填"，而非碰巧绕过。

---

## 八 预注册判据对照（派工单 §4）

| 判据 | 阈值 | 实测 | 结论 |
|---|---|---|---|
| fresh 失败率 | 显著 <40%，否则判修复无效回炉 | A 相 0/5 = **0%** | **达标** |

- 注：exec-window 自报 fresh=40% 与 test-window 同口径 v0.2.20 A=100% 存在矛盾（议题β：基线效力撤销）。无论取哪个基线，0% 均显著低于 40%，判据达标不受基线选择影响。

---

## 九 派工单偏离说明（必要偏差，已记录）

派工单原指令：
```
sed -i '/("C", i) for i in range(1, 4)/d' run_rc52_matrix.py
```
意图删 C 行，但 C 行与承载 plan 列表闭合 `])` 的**同一行**被一并删除，导致：
```
SyntaxError: '(' was never closed
```

**修复（必要偏差）**：
```
sed -i "s#(\"B\", i) for i in range(1, 6)\]#(\"B\", i) for i in range(1, 6)])#" run_rc52_matrix.py
```
补回 `)`，确认 plan 列表仅剩 A×5 + B×5（C 仍 BLOCKED，符合派工单 §6）。验证：`python3 run_rc52_matrix.py` pilot A1 跑通，全矩阵 10/10 完成。

> 此偏差不影响数据有效性（仅修复了派工单脚本自身的语法错误），但须在交付物中显式披露（派工单 §9 红线）。

---

## 十 三项议题联动

- **议题α（RC52 升格 CONFIRMED）**：v0.2.21 A 相 0 复现但 B 相 100% 复现，证实 RC52 是**概率性/分相吸引子**而非偶发——升格为 confirmed，与"非确定性"记忆一致。
- **议题β（40/100/67 撤销精确基线效力）**：v0.2.20 同口径 A=100% 与 exec 自报 40% 矛盾；本报告对照表一律以 test-window 同矩阵口径为准，避免拿错基线。
- **议题γ（goal_drift 立为 RC52 弱化形态排查项）**：B 相前缀重放中观察到 `stalled: 2 consecutive replans produced identical TaskGraph ... giving up early`（事件流 line 4 即出现），属 goal_drift / stall 家族，建议纳入 RC52 弱化形态排查——它与 RC52 共享"give_up 终态"但触发更早（规划阶段即停滞）。

---

## 十一 判读分离声明 + 待裁定开放点

本报告严格遵守派工单 §9"判读分离"：**只呈数据、判定器原始输出与对照，不自行裁定修复效力**。请砺/顶层就以下开放点裁定：

1. **B 相 100% 复现 = 修复对前缀重放路径无效**。是否需要针对性二次修复？还是 B 相属另一家族（γ/stall）不在本修复 scope？
2. **B1 分类**：判定器口径（PROVIDER_UNREACHABLE）vs 实质口径（RC52 复现）。两种口径均不改"A 修好、B 没修好"结论。
3. **hydration=0**：修复行为改善但回填标记未触发。需确认修复机制是否真走"跨 run 产物事实回填"路径（建议插桩/单测验证），避免"行为好但机制不明"。
4. **发版裁决**：A 相稳定 0 复现是否足以支持 v0.2.21 按派工单预注册口径发版？或需先补 B 相修复再发？

---

## 附：资产清单

- 全矩阵原始日志：`~/fa/p4-rerun2/p4_{A1..A5,B1..B5}.log` + `.events.json`（已下载至本地 `.workbuddy/n14_final/`）
- 判定器：`~/fa/p4-rerun2/n14_judge.py`（本地 `.workbuddy/n14_final/n14_judge.py`）
- 结果：`~/fa/p4-rerun2/results.json`（本地 `.workbuddy/n14_final/results.json`）
- 实验驱动/预检：` .workbuddy/n14_run_rc52_matrix.src.py`、`n14_precheck.py`、`n14_setup2.py`、`n14_fix_plan.py`
- 背景 finalizer：`JAkXyl`（等 B3–B5 收口后自动下载+判定，与本次手动合成互为冗余校验）
