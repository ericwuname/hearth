# Failure / Recovery Lifecycle — CORE FREEZE REVIEW-01 Node 10

## FailureKind → RecoveryStrategy（源码实测：terminal.rs classify_failure 纯函数 + 策略矩阵）

| FailureKind（输入） | RecoveryStrategy | 有界性 |
|---|---|---|
| AssertionFailure（exit >0，测试失败族） | Verify→Repair→Retest（回喂 acceptance） | Reserve ≤1 + replan cap 3 |
| EnvironmentFailure（目录缺失/工具环境） | Adapt（探查→mkdir→重试） | 步数预算 |
| ToolTimeout | Escalate/Retry-with-awareness | deadline 权威截断 |
| DeadlineExceeded（任务级） | **不拦截**——Safety>Deadline（604s 权威实证） | — |
| Unknown（真无信号） | 不冒充（FA01 基线） | — |

## Node 10 双拓扑真机

- **Case A**（configlib 常量错误）：completed；独立复验 **3 passed**（MAX_RETRIES=3 修复在位）。
- **Case B**（legacy_app 目录缺失环境拓扑）：completed；ADAPTED_OK ✓（探查→适配路径→写入）。
- **INV-LR04**：same failure + same state + same strategy 无限循环——**未出现**（T4/Reserve/replan cap 三重有界）。

## 已知有界边界（如实声明）

- **RC48**：criteria 非空 + Reserve 零和 + GiveUp 时点 → false stop（n12r1；r3 反证：有 Reserve 时全链路正确 completed）。修复须顶层批准。
- **QA 轮 T4 2-node stall**（n11 首跑 12/16 高频）：planner 无完成事实感知 + model 对已完成"继续"轮再 decompose → T4 bounded stop（零假完成方向正确）。修复（planner 完成感知注入）须顶层批准。
