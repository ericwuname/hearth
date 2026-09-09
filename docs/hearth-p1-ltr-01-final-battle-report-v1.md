# Hearth P1-LTR-01 总战报 v1：Tool-level Deadline Enforcement（2026-08-30）

> **性质**：长程施工总包（Phase 0-15 全自主执行）总战报——一次提交。
> **总包**：`Hearth P1-LTR-01 长程施工总包 v1`（含守门员开工前修正 5 条）+ 设计单 `docs/hearth-p1-ltr-01-tool-deadline-construction-order-v1.md`。
> **结论**：**PASS**（14 条验收标准全满足；无偏差上报事项）。

---

## A. Commit / 版本 / tag

```text
b3229d5  feat(p1-ltr-01): Tool-level Deadline Enforcement 全链路（6 文件，+261/-6）
c8b6532  style(n1-sbx): VM rustfmt 收敛
fb93f48  chore(v0.2.10): 版本资产 + 总账回填（T6 勘误 / TC-8·9 闭环）
e03565f  chore(v0.2.11): 版本 bump + CHANGELOG
最终 HEAD: e03565f   tag: v0.2.11   version: 0.2.11
```

## B. Implementation（核心逻辑 / 传播路径）

```text
ToolContext（tool-runtime/context.rs）
  + task_deadline: Option<Instant>     ← 绝对时刻戳（H2 run_started_at + max_time_secs 同源派生）
  + effective_timeout: Option<Duration> ← dispatcher 单点计算结果
  + deadline_clamped: bool              ← 结构化收紧标记（修 2）
  三字段 serde(skip)（不持久化，6.3）；None 路径行为与 v0.2.10 一致（T2）

AgentLoop run()（loop.rs）
  开头注入 scheduler.set_task_deadline(ctx_mgr.task_deadline())
  continue_turn 每轮重置 run_started_at → deadline 每轮重建（6.2）；跨轮零复用

ToolDispatcher::dispatch()（tool-runtime/dispatcher.rs）★唯一权威计算点
  effective = min(declared, remaining)；remaining ≤ 0 → 不启动工具，
  直接类型化 TaskDeadlineExceeded（Phase 6）；ctx 副本写 effective 下发

bash.rs（tools-builtin）
  ctx.effective_timeout 优先消费 → sandbox.spawn(Duration)（Phase 4）
  既有 SIGKILL + cgroup cleanup（X1 reap）复用（Phase 7，T11 日志实证）

loop 层：零改动（未加任何 deadline if——红线）；cancellation 路径独立不变
```

## C. Tests（先红后绿）

- **红**：旧代码无 task_deadline 通道（编译级红，位运算/结构缺位取证）。
- **单测（tool-runtime p1ltr_tests，全绿）**：T1 clamp（600s 声明被 1s remaining 截断 → 类型化 TaskDeadlineExceeded，≤5s 实测）；T5 tool timeout 优先（per-tool 1s < remaining 30s → 普通超时，**非** TaskDeadlineExceeded）；T2 None 回归；T4 expired 不启动（exec_count=0）；T3 continue_turn 重置。
- **负面对策**：clippy field_reassign_with_default 3 处修（字面量化）。

## D. Real-machine（.133 / kernel 7.0.0-30 / 系统 PATH /usr/local/bin/hearth 0.2.11）

| 场景 | 实际耗时 | 终态 | 判定 |
|---|---|---|---|
| **T9** deadline30 + sleep300 | **40.4s** | deadline_exceeded | ✅ 核心——对照旧基线穿透 ≈360s |
| T10 deadline30 + sleep10 | 29.8s | failed（**T4 停滞**，非 deadline——工具成功执行、effective=30 正确下发） | ✅ 短工具未被误杀；终态残余属 DEV-2/T4 家族 |
| T11 deadline30 + tool timeout10 + sleep60 | 40.7s | 首跳 **tool timeout**（sandbox 10s SIGKILL + cgroup 清理实证）→ 终态 deadline_exceeded（30s 真用尽） | ✅ 语义分离（§9 A/B 不混淆） |
| T12 deadline15 + sleep60 | **15.3s** | deadline_exceeded | ✅ 精确 |
| T14 Ctrl-C + deadline 25s | — | **⏹ cancelled**（零 deadline_exceeded 泄漏） | ✅ cancellation 优先 |
| 长程任务（sleep25 长工具 + write_file + cat，deadline 120） | 43.4s | **completed**（产物 LONGRUN_OK seen） | ✅ Phase 12 |
| T13 无 deadline 回归 | — | 全量 gate 415/0（绝大多数无 deadline 路径） | ✅（修 5 口径：gate 全绿即证据） |

日志归档：`/tmp/p1ltr_*.log` @ .133（T9-T14 + LTSK 全套）。

## E. Gate（增 6 约定：完整路径）

```text
host: .133 (192.168.220.133)
script: /home/wutao/run_gate_r2c.sh（唯一隔离门禁）
Phase 0 基线: .133:/home/wutao/t_gate_p1ltr_phase0.log   → fmt=0 clippy=0 RT4=0，411/0
全量:         .133:/home/wutao/t_gate_p1ltr_full.log     → fmt=0 RT4=0，415/0（clippy 3 处风格修，单包复验绿）
最终:         .133:/home/wutao/t_gate_p1ltr_final.log    → clippy=0 RT4=0，**415 passed / 0 failed**（fmt=1 为版本 bump 前的Cargo.toml 快照差，已收敛复验 FMT_NOW_OK）
```

## F. Regression（零回归证明）

N1-SBX（411 基线内含其 4 单测）/ O-1（headless guard 行为在 415 内）/ R2-C / W3/W4（四证明，上轮 CLOSED）/ RC24（TC-8/9 判定表测试原样通过）/ W8（路由单测原样通过）——全量 gate 415/0 覆盖，无回归。

## G. Deviations / OPEN / DEFER

- **OPEN**：O-3（seccomp 缺 SYS_MKDIRAT——P1-LTR 测试不依赖 mkdir，probe 通过；验收后 **G0-SBX/SECCOMP 独立单顶上**，守门员修 5）/ Q-3（landlock ABI≥5）/ OPEN-W8-1 / DEV-2 / verify_failed fixture / N-3。
- **O-3 与本单关系**：T3 目录规则验证改用无 mkdir 方式完成（可写目录内文件写/读 + 只读路径读），未阻塞；未顺手修 seccomp（红线）。
- **UNKNOWN**：无。
- **取舍说明**：ToolResult 不加 timeout_kind 字段（24 构造点成本 > 边际收益）——语义分离判定已由类型化错误 `TaskDeadlineExceeded` downcast 完成（修 2 达标），Display 文本仅透传展示不参与判定。

## H. Final

# **P1-LTR-01 = PASS**

14 条验收标准 ①-⑭ 全满足；双 VM provenance（系统 PATH 口径）：`.133 /usr/local/bin/hearth` = **0.2.11**、`.131 /usr/local/bin/hearth` = **0.2.11**，source = git archive HEAD `e03565f`。本轮停止，等顶层验收。
