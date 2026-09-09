# Hearth W3/W4 回归验证 · 新基线复验报告 v1（RC24+W8 后）

> **性质**：验证轮交付（执行窗口 → 顶层）。**零代码改动**（V-4 红线遵守，本轮 diff = 0）。
> **口径说明**：原指令（hearth-w3w4-regression-verification-order-v1.md）已于旧基线（HEAD `26ad3be`，v0.2.9+W3/W4）执行完毕（commit `66705aa`：V-2 3/3、V-1 偏差 1/10 停点）。本轮 = **新基线复验**（HEAD `900d199`，含 RC24 审批门 + W8 路由/Goal Revision）——上轮偏差 DEV-1/DEV-3 已分别被 W8/RC24 修复，且 W8 的 A2 改动（"继续"输入分类为 TaskControl）对 resume 场景构成真实回归风险，必须重证。
> **运行口径**：.133 / release binary（8539952B，含 RC24+W8）/ Agnes / `HEARTH_ALLOW_NO_CGROUP=1` / headless（stdin=/dev/null）。.131 未同步新代码，故本轮以 .133 为准（与旧轮 .131 口径差异如实注明）。
> **证据归档**：`docs/data/w3w4-reverify-20260829/`（15 文件）。

---

## V-1 · 旧故障基线回归（证明 ①②）——**达成 ✅**

| 场景 | 旧基线 | 上轮（W3/W4） | **本轮（RC24+W8）** |
|---|---|---|---|
| 十论述 prompt（B01-B10 逐字） | 0/10 | 1/10（偏差停点） | **10/10 completed**（W8 验收当轮同基线实测；B05 抽查：单轮 calls=1、completion 2597 tokens 真论述） |
| PRODUCT_regress（write_file 任务） | — | — | ✅ completed（21s）——product 不被误路由 |
| **T-A 三步**（create→run→verify→record） | ❌ failed（假失败） | ✅ completed | ✅ **completed**（22.7s，hello_w8.py+w8_result.txt 产物 seen）。原 prompt 归档不可得，用等价三步构造（如实注明） |
| **T-B 纯读问询**（TC-1 逐字） | ❌ stalled | ✅ completed | ✅ **completed**（37.6s）——负向短路+QA 直答 |

- **证明 ①**（工具成功不因 node.status 假 give_up）：T-A completed + 十论述 0 次 give_up ✅
- **证明 ②**（纯读不被判不工作）：T-B completed ✅

## V-2 · resume 连续性（证明 ④）——**3/3 全过 ✅（A2 交互重点排查通过）**

场景（prompt 逐字复用旧轮）：三步任务（step1 创建 → 必然失败命令 → step2 创建）+ `--budget 6` 中途失败 → `hearth resume <sid> "继续"`。

| 轮 | 首轮终态 | resume 终态 | 复述进度 | 增量产物 step2.txt | 重教次数 |
|---|---|---|---|---|---|
| R1 | failed(status=failed) | failed（二次预算耗尽——真实原因） | ✅ | ✅ STEP2_DONE | **0** |
| R2 | failed(budget_exhausted) | ✅ **completed**（6 步） | ✅ | ✅ STEP2_DONE | **0** |
| R3 | failed(budget_exhausted) | ✅ **completed** | ✅ | ✅ STEP2_DONE | **0** |

- 验收四条：①复述 3/3（R2 首轮原文："任务目标已恢复（revision 1）… step1.txt 已创建内容 STEP1_DONE / py 失败如预期 / step2.txt 已创建"）②增量 3/3 ③fact-check 不误拒（R2/R3 accepted）④终态正确（无 stalled/误 give_up）。
- **A2 回归排查 ✅**："继续"被 classify 为 TaskControl → effective_goal=original（"任务目标已恢复（revision 1）"）→ 无 GoalChanged、无 revision 膨胀——resume 语义与 A2 正交成立。
- 对照 v0.2.3"继续什么？"历史症状：0 例 ✓（RC5/RC30 家族维持闭合）。

## V-3 · 六终态投影矩阵（证明 ③）——**5/6 实证，1 项如实标注 ⏸**

| 终态 | 构造 | 投影结果 |
|---|---|---|
| completed | T-A / PRODUCT_regress | ✅ `✓ Task completed` |
| failed | V-2 R1-R3 首轮 | ✅ `✗ Task failed — status=failed/budget_exhausted` + reason 可见 |
| give_up | "改进当前目录里的代码文件。"（T4 stall 旧场景逐字） | ✅ 13 步，`✗ stalled: 2 consecutive replans produced identical TaskGraph (4 nodes) — Tier3 T4` 投影 failed + stalled 原因（与旧轮一致，give_up/failed 区分度问题维持旧轮结论） |
| verify_failed | 真机构造不可行（需 run 中途篡改产物）——**如实标注 ⏸**；机制层由 verify 单测覆盖（407 门禁内） | ⏸ |
| timeout | `HEARTH_TASK_TIMEOUT_SECS=30` + `sleep 60` | ✅ **failed(deadline exceeded)**（首轮用 sleep 300 未触发——见观察 O-2） |
| cancelled | SIGINT | ✅ **`⏹ 本轮已取消` 投影**（N-2 修复后由上轮 OPEN 转绿） |

**RC20 回归锚**：中文错误命令（`cat /tmp/不存在文件_中文测试_zz.txt`）→ `✗ 工具出错` 结构化投影，**无绿✓误标**（err_proj=True, green_mis=False）✅

## 四证明状态汇总（对照旧轮）

| 证明 | 旧轮（W3/W4 基线） | **本轮（RC24+W8 基线）** |
|---|---|---|
| ① 工具成功不假 give_up | 部分 | **达成**（十论述 10/10 + T-A） |
| ② 纯读不被判不工作 | 达成 | **达成**（T-B） |
| ③ 六终态投影 | 部分 | **5/6 实证**（verify_failed 构造 ⏸ 如实标注） |
| ④ resume 连续性 | 完全达成 | **完全达成**（3/3 + A2 正交重证） |

**结论：四证明在新基线维持/转绿闭合；上轮偏差 DEV-1（路由）与 DEV-3（审批阻塞）修复后无回归。W3/W4 正式收口条件达成。**

## V-4 · 红线与偏差

- **零代码改动遵守**（本轮 commit 无 diff，验证脚本均为采集工具不入库）。
- 无 W3/W4 行为回归；无新增偏差需停点。
- D 类禁改清单零触碰。

## V-5 · 观察项（记录不修，交顶层）

| 编号 | 内容 | 建议 |
|---|---|---|
| O-1 | `budget_reassess` 交互分支未做 is_terminal 保护（RC24-B 只盖 approval）——headless 下 stdin 未关闭时会阻塞至 EOF/外部超时（V2R2 首采卡 200s 实证） | 与 approval 分支同款一行保护；可并入下一施工单 |
| O-2 | deadline 只在步间检查——单步长工具调用穿透（`sleep 300` + timeout 15s → 工具跑满 360s 窗口，进程被外部杀） | H2 已知局限的首次真机实证；建议工具层 deadline 联动（工具 declared timeout 上限截断到剩余 deadline） |

## V-5 · 12 项交付对照

1. commit：本轮零 diff（验证轮）✅
2. V-1 对照表 ✅（上表）
3. V-2 三轮 resume 证据 ✅（15 文件归档含 6 份 resume/fail 日志）
4. V-3 六终态矩阵 + RC20 中文错误证据 ✅
5. telemetry jsonl：长任务样本见 W8 验收归档（同基线）✅
6. Delegation Friction：本轮 0 次人工干预（resume 全程仅"继续"）✅
7. OPEN/UNKNOWN 更新：O-1/O-2 新增 ✅
8. 偏差清单：无新增偏差 ✅
