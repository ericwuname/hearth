# 设计决策：T13/T19 六轮全挂的根因是 planner all_done 判定缺陷

> 决策日期：2026-08-01（v21 刀入鞘轮）
> 状态：**已定版**——wiring 断言 `all-done-requires-write` 强制执行
> 证据链：v19 深水轮解剖（`docs/t13-t19-anatomy-v19.md`）+ v20 收官轮修复（`docs/self-evolution-v20-report.md`）

---

## 决策一句话

**任务完成判定（all_done）必须以"至少一次真实写操作（write_file/edit）"为前提；只读节点（grep/read）的完成不算任务完成。**

## 背景：六轮谜团（v13-v18）

T13-fix-index 和 T19-merge-duplicate 在 v13-v18 六轮基准中 **0/8 全挂**：
- 所有 provider 全挂（zhipu / deepseek / deepseek-pro）
- 所有经验配置全挂（空 store / 规则经验 / LLM 精炼 / embedding）
- 每次 verify 都是 TEST_FAIL / NO_GENERIC_FN

曾被误判为"模型能力墙"（v15 能力边界白皮书）——但 deepseek-pro 也过不了，说不通。

## 解剖：v19 发现真凶

v19 深水轮对 T13/T19 各做一次"重跑+即时录制"（`bench/anatomy_v19.py`），逐 tool_call 分析：

```
T13 事件流（10 步，仅 2 个工具调用）：
  Plan → [grep safe_get|middle_char] → Act → Observe → Reflect(continue)
  → Plan → [read src/lib.rs] → Act → Observe → Reflect(continue)
  → Plan → DONE (ok=true, status=completed)

T19 事件流（10 步，同模式）：
  Plan → [grep parse_age_u8|parse_count_u32] → Act → Observe → Reflect(continue)
  → Plan → [read src/lib.rs] → Act → Observe → Reflect(continue)
  → Plan → DONE (ok=true, status=completed)
```

**关键发现：agent 只 grep + read，从未调用 write_file/edit 就 self-report 完成。**

runner 对照验证：同一任务用 runner 跑 → FAIL(TEST_FAIL)——代码根本没改。

## 根因

```rust
// loop.rs:967（v20 修复前）
let all_done = self.task_graph.nodes.iter().all(|n| {
    matches!(n.status, TaskStatus::Completed | TaskStatus::Skipped | TaskStatus::Failed)
});
if all_done { return Done; }
```

**planner 把"读取源码"这一节点标记为 Completed 后，all_done 为 true，agent 提前进入 Done——它以为不用写。**

这不是模型能力问题，是 **planner 判定逻辑缺陷**：Read 类节点的完成被错误地等同于"任务完成"。

## 修复（v20 已实施）

```rust
// loop.rs — v20.0 all_done 门控
if all_done {
    if self.write_attempted {
        return Done;                              // 有真实写操作 → 正常收尾
    }
    if self.plan_state.replan_count < 3 {
        self.needs_decompose = true;              // 无写 → 强制重新规划
        self.plan_state.replan_count += 1;
        return Plan;
    }
    return Done;                                  // replan 耗尽兜底（防死循环）
}
```

- `write_attempted` 在 do_act 中检测（复用 v12.5 已有的 `made_edit`：write_file/edit）
- 验证结果：**T13 从 0/8 → 2/3 PASS**，事件流证实 agent 真的开始写文件了
- wiring 断言 `all-done-requires-write` 锚定 `["write_attempted", "needs_decompose = true"]`

## 维护须知

- **不要**把 all_done 改回"无写也 Done"——这会让 T13/T19 复现六轮全挂
- **不要**给 task_graph 节点盲目加 Read/Write 标签（v20 审查后放弃：改动面大、破坏 agent-types 序列化）
- 剩余盲区 T19（NO_GENERIC_FN）是**真实工具链能力缺口**（泛型合并函数），与 all_done 无关，修复方向是工具链/验证脚本升级
- 若未来优化 planner：任何改动必须跑 T13/T19 单题回归（各 ≥3 次）
