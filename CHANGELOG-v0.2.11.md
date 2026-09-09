# CHANGELOG v0.2.10 → v0.2.11 — Tool-level Deadline Enforcement（2026-08-30）

> 证据链：`docs/hearth-n1sbx-o1-execution-report-v1.md`（同窗口 N1-SBX/O-1 执行）+ 本版 P1-LTR-01 真机矩阵（`/tmp/p1ltr_*.log` @ .133）+ gate `.133:/home/wutao/t_gate_p1ltr_full.log`（415 passed / 0 failed）与 `t_gate_p1ltr_final.log`。
> 总包：`Hearth P1-LTR-01 长程施工总包 v1`（Phase 0-15 全自主执行）。

## P1-LTR-01 — Tool-level Deadline Enforcement（commit b3229d5）

- **核心语义**：`effective_timeout = min(declared_timeout, remaining_task_time)`——任务 deadline 从"loop 层软提示"升级为"全执行链硬截止"。对照旧实证：`sleep 300` + task deadline 15s 曾穿透至外部窗口 ≈360s；修复后 ≈15-30s 结构化终止。
- **共享绝对时刻戳**：`ToolContext.task_deadline: Option<Instant>`——H2 `run_started_at + max_time_secs` 同源派生（不建第二套 deadline 类型）；时刻戳免疫逐层传参漂移；每轮 `continue_turn` 重置（不跨轮复用）；`Instant` 不持久化（resume 按新预算重建）。
- **dispatcher 单点计算**：`dispatch()` 内 `min` + `remaining ≤ 0` 时**不启动工具进程**直接类型化 `TaskDeadlineExceeded` 错误；`ctx.effective_timeout` 单点写入传给工具。
- **结构化超时语义分离**（守门员修 2）：`deadline_clamped` → 类型化 `TaskDeadlineExceeded`（downcast 判定，禁 stderr 字符串解析——RC20 反模式禁令）；declared 更早 → 普通 tool timeout（W4 语义）。情况 A/B/C 三分。
- **bash 消费**：`ctx.effective_timeout` 优先于 `args.timeout_secs` → `sandbox.spawn` 按真实剩余时间终止；复用既有 SIGKILL + cgroup cleanup（X1 reap 路径，T11 日志实证 `killed child and removed cgroup`）。
- **cancellation 优先**：Ctrl-C 路径独立不变——deadline 接近时取消仍归 `cancelled`（T14 实证，零泄漏）。
- **loop 层零改动**：deadline 经 ctx 通道下发，未在 run loop 加任何 deadline if（红线）。
- **单测**：T1（clamp）/T5（tool timeout 优先）/T2（None 回归）/T4（expired 不启动）/T3（continue_turn 重置）+ 415 passed 全量 gate。
- **真机矩阵**：T9 核心验收（sleep300+deadline30 → 40.4s deadline_exceeded，旧 360s）/ T12（15.3s 精确）/ T14（cancel 优先）/ 长程任务（sleep25+write+cat，deadline 120 → completed）/ T11 语义分离（sandbox 10s SIGKILL + tool timeout 语义，终态 deadline_exceeded 为时间真用尽）。

## 同窗口随附

- N1-SBX / O-1 施工与真机验收（commit `f8f549a`/`1d5bd86`，详见 `docs/hearth-n1sbx-o1-execution-report-v1.md`）。
- 总账回填：T6 条目勘误 + TC-8/TC-9 闭环批注。
- 双 VM provenance 对齐（系统 PATH 口径：`/usr/local/bin/hearth` = 0.2.10→0.2.11）。

## 已知缺口（OPEN，记录不修）

- **O-3**：seccomp allowlist 缺 `SYS_MKDIRAT(258)`（mkdir 类 SIGSYS/159 家族）——P1-LTR 验收后 **G0-SBX/SECCOMP 独立单顶上**（守门员修 5：优先于 DEV-2 证据窗）。
- Q-3：landlock ABI≥5 未 handled 位（observe-only）。
- OPEN-W8-1 / DEV-2 / verify_failed fixture / N-3：维持既定处置路径。
