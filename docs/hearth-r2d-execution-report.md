# R2-D TaskGoal 执行报告（批示落地 · 2026-08-28）

> **执行**：v0.2.7 / commit `46ed8f6`
> **依据**：《R2-B TaskGoal Goal Persistence 顶层评审批示》（批示 1-10）+ 守门员补充 1-6
> **VM 门禁**：fmt/clippy/test 全 RC=0，**370 passed / 0 failed**（~/t_gate.log）

---

## 一、批示逐条落地（10/10 + 补充 6/6）

| 批示 | 落地 | 锚点/测试 |
|---|---|---|
| 1. original immutable | RunState 4 新字段（serde default 兼容旧文件）；`apply_turn_goal` 两步法（重建前检测旧 goal——修复 continue_turn 先覆盖致比较恒 false 的顺序 bug）；`GoalChanged` 事件三层接线（api/agent-runtime/CLI） | T1 |
| 2. state_revision 一致性 | `save_taskgoal`/`save_graph_with_revision` 同值；不一致 → recovery path（stale 标注+warning，不阻断不静默）；旧格式裸 graph 兼容 rev=0 | T2 |
| 3. 初始化条件 | original absent → 写（生命周期条件，spawn 前持久化——首次副作用前） | T7 |
| 4. constraints provenance | 第一版仅 Hearth.md/CLI 显式/InteractionRequest；写入通道=恢复值或空 | — |
| 5. verification scope | report 拆 artifact/acceptance 两字段；criteria 空恒 "none" 绝不冒充 passed；非空未确认="pending"；`normalize_terminal_state` 纯函数不动（补充 4 红线） | T6 |
| 6. next_action deterministic | `(拓扑深度, node id)` 稳定排序 + completed/remaining 派生函数 | T5 |
| 7. Task Continuity 进 Runtime Context | **history 尾部 Role::System 标签块**（dynamic suffix 区，禁入 stable system_text——补充 1 与 R2-A 根因调和）；resume 恢复链（restore_taskgoal+一致性校验+CLI 🎯 提示） | T3/T4 |
| 8. 不建第四套模型 | taskgoal.json 仅持久化载体；Plan 语义唯一真相=TaskGraph | 结构性保证 |
| 9. 施工范围 | 7 项施工全做；normalized_goal/node-level criteria/ContextBuilder/GD 强制暂停均未动 | — |
| 10. 负面测试 | **T1-T7 全落地**（MockLlm 零预算——补充 6 口径） | 7/7 PASS |

补充 5（R2-E 追认）：ToolInvocation 派生结构（内存态）挂下轮与 Observer 消费侧一起落。补充 6（采集器 v2 钩子下沉 gateway 层）：挂 R2-C 前置（57.6% 数据引用时已标注"plan 相位有偏子集/下界"）。

## 二、真机验证（v0.2.7 release，Agnes 通道）

| 项 | 结果 | 证据 |
|---|---|---|
| fresh run | ✅ `🎯 任务目标已锚定（original_goal 持久化）` + `✓ Task completed（6 步）`；taskgoal.json 落盘（original 全文无截断 + goal_revision=1 + state_revision=2） | t_fresh.log |
| resume | ✅ `🎯 任务目标已恢复: ...（revision 1）` + Task completed（10 步）+ **产物 hello_r2d.txt 实际含 RESUME_OK**——恢复的任务语义驱动了真实续做 | t_resume.log |
| goal revision | ✅ 第二轮换目标 → `🔁 目标已修订（revision 2）——原目标保留为锚点` + 第二轮 Task completed；"查看状态"类新指令正确识别为修订（rev 3） | t_revision.log |
| compact | ✅ 单测 T4 锁定（compact 后 original/continuity 存活）；真机 compact 挂下次长会话 | — |
| crash recovery | ⚠️ 部分：T2 单测锁定数据层 + 恢复链真机已验（rev 2 正常恢复）；**mutation 直验**（手工造成 mismatch 看 stale 标注）脚本 $HOME 展开问题未做成——挂账 | — |

## 三、意外发现（P0 级，已处置）

**采集任务失控写盘 57G 塞满 VM 磁盘**（Task 1 的 21 步均 2.7G/步）——副作用失控的活体实证。已清理（磁盘 100%→49%，释放 59G；telemetry 数据保全）。**升级建议**：产物大小审计（单文件/累计写盘上限）进入 R2 候选项——现有 G0 沙箱限制路径不限制体积。

## 四、挂账

1. verification scope 的 criteria 写入通道（planner 扩展单）
2. 采集器 v2（gateway 层钩子，覆盖 planner 直连 chat 的绕过路径）
3. ToolInvocation 派生结构（内存态，补充 5 口径）
4. crash recovery mutation 直验
5. R2-C ContextBuilder 设计（数据已备，Task Continuity 注入路径须复用）

## 五、分支说明

ux-polish-01 期间有其他窗口的 commit 叠加（`d81087b` 腐化审计整改 / `0ccbe26` syscall_probe——非本窗口产物，无冲突）。
