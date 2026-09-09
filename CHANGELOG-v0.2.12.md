# CHANGELOG v0.2.11 → v0.2.12 — P1-CONSOLIDATION-01 执行边界/任务语义/稳定性收敛（2026-08-30）

> 证据链：`docs/Hearth P1-CONSOLIDATION-01 长程施工总包 v1`（Node 00-11）+ 归档 `docs/data/n1-landlock/`（mkdirat_probe.c）、`docs/data/p1ltr-20260830/`、`docs/data/n1sbx-o1-20260830/`。
> 门禁：415 → **416+ passed / 0 failed**（`.133:/home/wutao/t_gate_consol_node02.log` 四 RC 全 0）。

## Node 01 — G0 Sandbox Capability Closure（commit 7b53800）

- **🔴 seccomp 全表审计（ausyscall 权威反查 122 号）**：`SYS_FCHDIR` 历史错位——原值 133 在 x86_64 实际是 **mknod**（创建设备节点被误放行，可与 writable 目录组合触碰设备）；真 fchdir=81 反而缺位（mkdir -p 失败根因链）。修正为 81——**收紧性修复**（关闭 mknod 误放行 + 补齐 fchdir）。
- **O-3 闭环（修 3 证据驱动）**：无缓冲 probe（`docs/data/n1-landlock/mkdirat_probe.c`）实锤沙箱内 mkdir(83) 放行、mkdirat(258) SIGSYS exit 159 → 加 `SYS_MKDIRAT=258`（最小集，与 mkdir 语义配对）。
- **umask(95) 加白**：strace 全量差集实锤 coreutils（mkdir/touch）启动必调。
- **真机三测**：positive `mkdir -p → MKDIR_OK`；negative `mknod → 159 拒绝`（收紧实证）；regression `write/cat → WRITE_OK`。
- **Q-3 结论**：维持 ABI v4 rights 集（当前实现所需全集 = 15 位）；kernel ≥5 新位（IOCTL_DEV 族）不纳入 handled = kernel 设计语义的未限制边界（**记档不扩**，非 fail-closed 缺口）；现有 `landlock_create_ruleset` probe 即 feature detection（复用，不建第二套 detector）。

## Node 02/03 — Task Type × Constraint 分离 + QA 跳过 decompose（commit 2c0c1b9）

- **OPEN-W8-1 收口**：`goal_requires_product` 负向短语**剥离**机制——residual 上判定 Task Intent（"写 README 但不要修改现有文件" = Product + Constraint）。A-G 测试矩阵全绿；真机：C/F/TC1 completed + 既有文件 hash 保全；T3 原 prompt 重跑 completed 14 步（README 生成，budget 25 + 干净目录）。
- **QA 跳过 decompose（Node 03 Q1 最小诊断修复，不改终态语义）**：QA/论述任务跳过 planner decompose——论述 goal 拆图 → all_done 恒 false → reflect replan → T4 stall（真机实证 9 步）→ 修复后同任务 completed 2 步。

## Node 03 — REFLECT_FACT_CONFLICT 观察标记（commit 6d1d66a，修 4 落法）

- Reflect give_up 时若事实层显示真实进展（artifacts 非空 + 0 errors）→ `REFLECT_FACT_CONFLICT` 标记：tracing + run report `summary.reflect_fact_conflict` 字段——**observe-only**：不做新 Event 变体、不改终态、不 force continue（Fact/Reflect 决策权问题另立研究）。
- **Q1 初答**：冲突样本归因 = task classification 类（论述任务被拆图，Node 02/03 修复闭合）+ model judgment 类（写盘后仍 give_up，DEV-2 残余继续观察）。
- **Q3**：现有 verifier = `verify_written_files`（artifact 级）+ `completion_fact_check`（original_goal 对齐）——已存在，无需造第三套。

## Node 04 — verify_failed 端到端 fixture（commit 117cf02）

- 确定性构造（零 production 改动）：MockLlm 写产物→空内容覆盖→DONE ×3 → Done 相位 verify 命中"缺失或为空" → replan(≤3) → 超限 → **verify_failed 终态**。断言 ok=false + status=verify_failed + verify.missing 明细 + terminal 九态映射保持。
- fixture v2 说明：rm 方案撞审批门被拒（RC24 语义正确工作）——改用空覆盖路径。

## Node 05 — N-3 审批委托 Markdown 投影（commit 6d1d66a）

- `RunReportInput.approval_delegated (bool, Vec<String>)` + report「## 审批委托」节——false/true+1 条/true+多条/空清单四象限一致投影。ApprovalPolicy/delegation 语义零改动。

## Node 07/08 — 长程验收 + telemetry 审计

- 长程任务（20-30+ steps：建项目→受控失败→修复→复测→产物，telemetry 开）+ 10+ turn discussion（W8 回归）——结果见 Final Report。
- telemetry 审计结论：`phase` 字段（main/aux）已实现主链/辅助调用分桶（cache_telemetry.rs:147）——细分 role 延后（记录不扩）。

## 安全边界声明

- seccomp：+3（mkdirat/umask/fchdir 修正）全部有真实 syscall 证据 + 最小权限原则 + 正/负/回归三测；**FCHDIR 错位修正为收紧**（关闭 mknod 误放行）。
- landlock：零改动（N1-SBX 语义保持）。
- approval/RT4/事件契约：零改动。
