# T13/T19 能力墙解剖报告（v19 S1/S2）

> 方法：重跑单题 + 跑完立即 GET /messages 录制完整事件流（session 在内存时）
> 关键前提修正：旧 session 历史已丢失（service 重启清空内存，磁盘只存 meta+done），必须重跑即时录制
> 原始数据：`bench/results/raw/v19-anatomy/*.json`

---

## 一、T13-fix-index 解剖（deepseek，10 步）

### 事件流

```
PHASE Init → Plan → [grep safe_get|middle_char] → Act → Observe → Reflect(continue)
→ Plan → [read src/lib.rs] → Act → Observe → Reflect(continue)
→ Plan → **DONE (ok=true, status=completed)**
```

### 工具调用序列（仅 2 个）

| # | 工具 | 参数 |
|---|---|---|
| 1 | grep | pattern="safe_get\|middle_char" |
| 2 | read | path=.../src/lib.rs |

### 关键发现

1. **agent 只读取了源码，从未调用 write/edit 就 self-report 完成**（ok=true）。
2. 10 步中 2 个工具调用、8 步空转（Plan/Act/Observe/Reflect 循环），第三次 Plan 后直接 Done。
3. runner 真实验证：**FAIL (TEST_FAIL)** —— 代码根本没改，cargo test 当然挂。
4. **root cause**：`loop.rs:967` 的 `all_done` 判定将 `TaskStatus::Completed | Skipped | Failed` 都视为完成——planner 在 Plan 阶段把"读取源码"这一动作标记为 Completed 后，agent 就提前进入 Done。**这是 planner 判定缺陷，不是模型能力墙。**

## 二、T19-merge-duplicate 解剖（deepseek，10 步）

### 事件流

```
PHASE Init → Plan → [grep parse_age_u8|parse_count_u32] → Act → Observe → Reflect(continue)
→ Plan → [read src/lib.rs] → Act → Observe → Reflect(continue)
→ Plan → **DONE (ok=true, status=completed)**
```

### 工具调用序列（仅 2 个）

| # | 工具 | 参数 |
|---|---|---|
| 1 | grep | pattern="parse_age_u8\|parse_count_u32" |
| 2 | read | path=src/lib.rs |

### 关键发现

1. 与 T13 **完全相同**的模式：read 后不 write，直接 Done。
2. verify 预期 NO_GENERIC_FN —— 因为 `parse_positive` 泛型函数从未被写入。
3. **root cause 与 T13 一致**：planner 判定缺陷（读取即完成）。

## 三、两个解剖的共同结论

```
T13/T19 并非"模型写不出修复"，而是"agent 根本没写"：
  planner 把 read/检查 动作标记为任务 Completed → all_done 触发 → agent 提前 Done

→ 这不是能力墙（换模型/加经验都无效的原因在此），
  是 planner 完成判定的逻辑缺陷（可达性/行为类 bug）
```

**这解释了 v13-v18 六轮中 T13/T19 0/8 全挂、所有 provider 全挂、经验注入无效的谜团**——因为问题根本不在"会不会写"，在"planner 让 agent 以为不用写"。

## 四、修复方向（v20 候选）

1. **all_done 判定加约束**：task_graph 中必须有 ≥1 个 `ToolType::Write`（write/edit/apply_patch）节点被真实执行，否则不允许 Done。读取类节点不得单独构成"完成"。
2. **planner 节点分类**：将 task_graph 节点分为 Read 类（信息收集）和 Write 类（变更产出），`all_done` 只认 Write 节点完成。
3. **兜底：Done 前自检**：若 task_graph 无 Write 节点或无写操作执行记录，强制 replan 而非 Done。

> 注：此为诊断报告，修复属施工活（v20 任务书），本报告只给出 root cause 与方向。
