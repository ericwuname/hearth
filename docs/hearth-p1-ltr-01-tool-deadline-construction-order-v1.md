# Hearth P1-LTR-01 施工单 v1：Tool-level Deadline Enforcement（设计稿 · 待批准）

> **性质**：设计稿——**批准后施工，本文档不含代码变更**。
> **签发依据**：顶层《收到〈W3W4 回归验证 · 新基线复验报告 v1〉》§四（P1-LTR-01 立项）+ 守门员裁决 3（设计必答三问）。
> **实证锚点**：task deadline=15s + tool=sleep 300s 时工具继续执行至外部窗口 ≈360s 终止（W3W4 复验 V3_timeout 首轮，`rv2_V3_timeout.log`）——当前 deadline 是 **loop 级**而非**工具执行级**。
> **范围红线**：禁改 Budget 数值语义 / ApprovalPolicy / landlock（N1-SBX 刚定格）/ seccomp allowlist（O-3 另立）；禁在 loop 层另起 if（守门员明令）。

---

## 一、必答三问（守门员裁决 3）

### 问 1 · 落点层：effective timeout 沿 H1 declared_timeout 通道下发

- **现状链路**（v0.2.4 H1 已接通）：`bash.rs::execute` 读 `args.timeout_secs`（默认 180、cap 600）→ `sandbox.spawn(..., Duration)`；dispatcher 侧 `register_with_declared_timeout`（610s 窗口）豁免 30s 总闸。
- **落点决策**：effective timeout 在 **dispatcher 调工具前的单点**计算（`min(declared, remaining)`），经现有工具调用参数链下发——bash/sandbox 的 spawn duration 天然消费，**loop 层零改动**。禁在 run loop 加 if（明令）。
- 工具声明超时的默认值语义不变（未声明 → dispatcher 默认闸；声明 → min 后下发）。

### 问 2 · 剩余量传递机制：共享截止时刻戳（非跨层传参）

- **选型**：`ToolContext` 增 `task_deadline: Option<std::time::Instant>`（时刻戳，非剩余秒数）——**时刻戳天然免疫逐层传参漂移**（剩余秒数在传参/排队/调度延迟中会失真；时刻戳任何一层取 `saturating_now() - deadline` 都一致）。
- 传播链：`run()` 开头（H2 mark_run_start 处，已知 run_budget.max_time_secs）→ AgentLoop 持有 deadline 时刻 → 每次工具调用前注入 ToolContext（ctx 在 dispatcher.dispatch 路径已逐层传递——agent-core → tool-runtime → tools-builtin，全链已存在，只加一个字段）。
- `Option::None` = 行为完全不变（service/测试路径无 deadline 时零影响）。
- **跨 run 语义**：REPL 每轮 continue_turn 重置 deadline 起点（H2 既有）——字段在 run() 开头重写，跨轮不串。

### 问 3 · terminal reason 归属：task 侧 `deadline_exceeded`，防两类原因混淆

- 判定规则（工具被截断时）：
  ```text
  effective = min(declared, remaining)
  若 remaining ≤ 0 时工具被截断（工具错误信息含"task deadline"标记）：
      → loop 观察/收尾走既有 H2 deadline_exceeded 路径（复用，九态不变）
  若 declared < remaining 时工具被截断：
      → tool timeout（现状语义，W4 结构化错误）
  ```
- **投影语义钉死**：任务剩余期截断 = `deadline_exceeded`（可提高 HEARTH_TASK_TIMEOUT_SECS 后 resume 的既有提示保持）；工具自身超时 = tool timeout 错误帧。V-3 矩阵两类原因不混淆。
- 实现方式：工具截断错误输出携带结构化标记（如 stderr 前缀 `[task-deadline]`）→ loop 层 do_act 的 error 分类消费——不加新事件类型（契约只增不改）。

## 二、ChatGPT 六要素覆盖

| 要素 | 处理 |
|---|---|
| tool-runtime | `ToolContext.task_deadline: Option<Instant>` 新字段（Default None 兼容） |
| dispatcher timeout | effective = min(declared, remaining) 单点计算（dispatch 入口） |
| sandbox process lifetime | spawn duration = effective（bash 沙箱既有 Duration 消费点不变） |
| subprocess kill | 复用既有 timeout 路径（SIGKILL + cgroup cleanup，X1 fix 已含 reap——lib.rs:1108-1131） |
| cancellation | Ctrl-C 路径零改动（abort 优先于 deadline，两层独立） |
| cleanup | 复用 X1 reap；无新增清理路径 |

## 三、测试矩阵

| # | 场景 | 断言 |
|---|---|---|
| T-1 | 单测：effective 计算（declared=600, remaining=30 → 30；declared=10, remaining=30 → 10；deadline=None → declared） | 纯函数，本地可跑，先红后绿 |
| T-2 | 真机：`sleep 300` + `HEARTH_TASK_TIMEOUT_SECS=30` | 工具 ≈30s 被截断（对照本轮穿透 360s），终态 `deadline_exceeded` |
| T-3 | 真机：`sleep 10` + timeout 30s | 正常完成（不误伤——remaining > declared 时 effective=declared） |
| T-4 | 真机：`HEARTH_TASK_TIMEOUT_SECS=15` + sleep 60 | deadline_exceeded（复验本轮 V3_timeout2 构造仍绿） |
| T-5 | 无 deadline 路径（service/单测） | Option None 行为不变（回归） |

## 四、不变量与回滚

- **不动**：landlock（N1-SBX 刚定格）/ seccomp（O-3 另立）/ RT4 / ApprovalPolicy / Budget 数值语义 / 事件契约。
- **兼容**：deadline=None 全路径行为不变；bash 600s cap 保留（cap 与 min 的叠加顺序 = min(declared, remaining) 后仍受 600s 硬顶约束——工具自保上限不因任务 deadline 放宽）。
- **回滚**：单 commit revert；回滚后回到 loop 级 deadline 现状（穿透缺口复存，无新增风险）。
- **预估 diff**：tool-runtime context +1 字段、dispatcher 单点 min、bash.rs 消费 ~5 行、agent-core deadline 注入 ~10 行、单测——**预计 <60 行**。

## 五、验收门禁（增 6 约定：gate log 路径登记）

- 隔离门禁全绿（≥411 + 本单新增单测），gate log 完整路径写入报告（如 `.133:/home/wutao/t_gate_p1ltr01.log`）。
- 真机 T-2/T-3/T-4 三构造 + V-3 矩阵回归抽查（timeout/cancelled 两态）。
