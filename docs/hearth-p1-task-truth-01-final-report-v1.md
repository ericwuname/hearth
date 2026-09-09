# Hearth P1-TASK-TRUTH-01 Final Report v1（2026-08-30）

## 1. Executive Summary

# **PASS**

十 Node 长程执行完毕；O-4（Deterministic Acceptance Verification）落地并完成真机全链验证；五项顶层修正 + 守门员 5 补充全部落实；全量 gate 四 RC 全 0（420/0/1）；双 VM provenance 对齐。唯一持续 OPEN = DEV-2 model judgment 残余（observe 中，见 §8）。

## 2. Commit Graph

```text
Node 00 Baseline        → （gate 补票基线 417/0，t_gate_tt_node00.log）
Producer Audit（修正 1）→ （审计结论：CLI 接线修复，非 STOP-8）
Node 03 O-4             → 1e22203（verifier + Reserve + pending 桥 + CLI 接线）
Node 04 fixture v2      → 117cf02（上总包）+ 本轮 missing 断言修正
lint 收敛               → 7ad56cc / c14fac0
v0.2.13 资产            → 4a68b8d（bump + CHANGELOG）
最终 HEAD: c14fac0   tag: v0.2.13（强制移至最终态）  version: 0.2.13
```

## 3. Implementation

| 修改点 | 内容 |
|---|---|
| ToolContext.criteria 解析 | `cmd:` / `file: p contains t` / `file: p nonempty` 三类结构化条目（自由文本不核验） |
| cmd verifier | dispatcher bash 通道（审批/sandbox/landlock 完整继承）+ 副作用禁名单粗滤 + **写盘快照确定性检测**（增 2：验证命令产生工作产物 = failed） |
| file verifier | workspace 白名单（绝对路径/越权 = invalid 不读——增 1）+ 64KB 上限 |
| Done 相位核验块 | passed → `acceptance_verification="passed"` **生产者补齐**；失败 → Verification Reserve（≤1，对齐源码额度，telemetry 记账——增 4）→ 耗尽 → verify_failed（acceptance_failures 明细） |
| pending 桥 | init_taskgoal ↔ run() 内 ContextManager::new 重建的 criteria 传递（否则 --acceptance 静默丢失——R2-D 时代无生产者未暴露的隐藏缺陷） |
| CLI 接线 | `--acceptance`（可重复）→ init_taskgoal（Producer Audit 修复） |

## 4. Test Evidence

- **先红后绿**："passed" 无生产者（现状）→ Node 03 核验块落地（生产者补齐）；cmd 禁名单/解析单测红→绿。
- **unit**：criteria 解析 5 例 / 禁名单 6 例 / passed 生产者端到端（MockLlm+EditTool+BashTool+criteria 双条目）——3/3 绿；node04 继承绿。
- **VM**：`.133:/home/wutao/t_gate_tasktruth_final.log` = **fmt=0 clippy=0 RT4=0，420 passed / 0 failed / 1 ignored**（ignored=既有 RT4 人工项）。

## 5. Long-run（Node 07 复跑——O-4 全链真机）

mathlib 同任务同 prompt（budget 50 + `--acceptance` cmd:/file: 双条目）：
- 模型过程中再次出现自述矛盾（自述 "PASSED" 而实际 test FAILED）→ **acceptance 核验以 L1 结构化证据判定**——artifact 自述与事实的矛盾被证据链约束（O-4 价值实证）；
- **外部独立复跑确证**：lib.rs 修复正确（a+b）、`cargo test` 真实 ok（1 passed）、RESULT.txt = LONGRUN_TEST_PASSED 与事实一致——语义正确性最终达成 ✅；
- 终态 44 步 failed：planner budget-low give_up 抢在任务实质完成后触发（Node 05 交互矩阵活样本，observe 不修——DEV-2 家族）；
- improvement 判定（预写口径）：②③⑤ 满足、① 终态偏差（failed 而非 completed——give_up 时序）、④ 步数 44 微超 42——**部分达成**，偏差根因 = Node 05 矩阵（非 provider 单因素，顶层 §二要求满足——机制嫌疑未排除前不归因模型）。

## 6. Regression

R2-C / W3/W4 / RC24 / W8 / P1-LTR / Sandbox（N1-SBX）/ Node 02 路由（A-G 继承）——全量 gate 420/0 覆盖，零回归。

## 7. Security

- seccomp：零改动（Node 01 上轮已收敛）。
- **本轮新增边界（O-4 verifier 三道防线）**：①副作用禁名单粗滤（修正 1）②写盘快照确定性检测（增 2）③cmd 走完整 bash 边界（审批/sandbox/landlock 继承）+ file 白名单/64KB（增 1）——**验证通道不产生新执行能力**（修正 1 红线满足）。
- approval/RT4/事件契约：零改动。

## 8. OPEN / UNKNOWN / DEFER

- **OPEN**：DEV-2 model judgment 残余（44 步 give_up 时序 + REFLECT_FACT_CONFLICT observe）；Q-3 kernel≥5 新位评估；O-4 边界声明（确定性核验 ≠ 语义理解完成）。
- **UNKNOWN**：无。
- **DEFER**：Bridge / Subagent / TUI / MCP（维持）。

## 9. VM Provenance（系统 PATH 口径 + 分窗登记）

| host | source path（分窗登记） | HEAD | binary path | version | gate log |
|---|---|---|---|---|---|
| .133（评审 VM） | ~/codex_t | c14fac0 | /usr/local/bin/hearth | **0.2.13** | .133:/home/wutao/t_gate_tasktruth_final.log |
| .131（执行 VM） | **~/codex**（自有施工树） | 同源 tarball | /usr/local/bin/hearth | **0.2.13** | （build 实证 52s） |

## 10. Final Recommendation

# **PASS**

（Node 07 终态偏差与 DEV-2 残余按顶层 §二/§三口径归 OPEN 持续观察，不构成本轮验收阻塞。）本轮停止，等顶层终验。
