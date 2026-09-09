# Long-run Evidence — CORE FREEZE REVIEW-01 Node 08/09/11

## Node 08 — Canonical Compact → Resume → Continue（单跑成链，S-4）

chainfree（rev 反转 bug → 修复 → 复测 → 阈值 7000 强制压缩 → deadline 中断 → resume → 完成）：
- phase 1 **completed**（10 步）+ phase 2 resume **completed**（含 CHAIN_OK ✓）。
- 独立复验：chainfree cargo test **1 passed** ✓。
- **re-teach=0 / 零重复破坏性执行 / 零 goal mutation / 零假完成 / 零假 give_up** ✓。
- 压缩触发：env 7000（S-3 声明）——v0.2.18 修复后仪器真实生效。
- **P2-MC Deviation① 由本 Node 单跑闭环**（P2-LR 分证合链 → 本轮单链完整）。

## Node 09 — Long-run Product Benchmark ×2

| | Task A todoapi | Task B units |
|---|---|---|
| 终态 | completed | completed |
| 独立复验 | **1 passed** ✓（多模块 store/model） | **3 passed** ✓（含 round-trip 测试） |
| 产物 | NOTES.md ✓ | CHANGELOG.md ✓ |

## Node 11 — QA / Intent Boundary（16 轮）

- 产物侧：notes/todo.txt="buy milk"、notes/done.txt="really all set" ✓（任务轮全部达成）。
- "继续"两形态：进行中形态正常续跑 ✓；已完成形态 = **RC48 第二形态暴露**（planner 对已完成轮再 decompose 同图 → T4 stall——12/16 轮高频，model+decision 层，有界）。
- "查看状态"：TaskControl 正常（零 revision++）✓。
- "情绪+任务"复合：任务部分完成（todo.txt 写入）✓；情绪部分被混合处理（任务优先）——无机械切句复发 ✓。
- 用户纠正（"我自己就是 hearth"→"我是用户"）：无 GoalMutation 异常 ✓。

## 与 P2-LR 基准的对照

| 维度 | P2-LR | CFR（本轮） |
|---|---|---|
| Product completed | 2/2（geoutil/units 系） | 2/2（todoapi/units）+ configlib/chainfree/mathnotes 独立复验全 passed |
| QA 边界 | 0 failed | **12/16 T4 stall（RC48 第二形态暴露）** |
| Compact+resume 单跑链 | ✓（chainlib） | ✓（chainfree 复证） |

**如实声明**：QA 轮 T4 stall 高频是本轮**新暴露**的 model+decision 层已知限制（修复须顶层批准），已入 known-deviations F1。
