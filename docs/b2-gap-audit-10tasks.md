# B2-A 真实任务 gap 质量审计（10 任务）

> 日期：2026-08-22 | 执行：hearth chat 直跑（VM Linux，deepseek，真 LLM）
> 方法：每任务独立工作目录跑 `hearth chat <goal> --budget 6`；审计读 `~/hearth/observer/<sid>.jsonl` 的 `plan_draft` 事件原始 JSON（非渲染输出）
> 脚本：`bench/b2_gap_audit.py`（benchmark 目录，可重跑）

## 1. 结论

- **10/10 任务 gap 100% 带 `from`+`why`**（可追溯来源 + 具体原因）
- **10/10 非阻塞 gap 100% 带 `assume=true`**（构造器强制）
- **0 通用问题**（无"你有什么要求吗"类为问而问）
- **0 误触发 blocking**（三轮收紧后——见 §4 演进）
- 每任务默认产出 `missing_goal_source` 非阻塞假设（目标无来源标注→按用户直接指令执行）——合理且带 why

## 2. 审计表（第三轮，收紧后）

| # | 任务类型 | 目标（简） | gaps | 待问 | 自动假设 | from 完整 | why 具体 | assume | 通用问题 |
|---|---|---|---|---|---|---|---|---|---|
| T01 | 写代码 | Rust add 函数+测试 | 1 | 0 | 1 | ✅ | ✅ | ✅ | 无 |
| T02 | 改bug | 修复 max 函数 bug | 1 | 0 | 1 | ✅ | ✅ | ✅ | 无 |
| T03 | 查资料 | 解释 Rust ownership | 1 | 0 | 1 | ✅ | ✅ | ✅ | 无 |
| T04 | 重构 | 重复代码重构 | 1 | 0 | 1 | ✅ | ✅ | ✅ | 无 |
| T05 | 配置 | Cargo.toml 依赖 | 1 | 0 | 1 | ✅ | ✅ | ✅ | 无 |
| T06 | 测试 | divide 边界测试 | 1 | 0 | 1 | ✅ | ✅ | ✅ | 无 |
| T07 | 解释概念 | async/await 解释 | 1 | 0 | 1 | ✅ | ✅ | ✅ | 无 |
| T08 | 多文件 | lib+main 项目结构 | 1 | 0 | 1 | ✅ | ✅ | ✅ | 无 |
| T09 | 模糊意图 | "优化这个项目" | 1 | 0 | 1 | ✅ | ✅ | ✅ | 无 |
| T10 | 跨语言 | Rust 实现 Python 推导式 | 1 | 0 | 1 | ✅ | ✅ | ✅ | 无 |

**全部硬闸门通过**（from+why 100%、assume 100%、无通用问题、无误触发）。

## 3. 样例（原始事件 JSON 节选）

```json
{
  "type": "plan_draft",
  "gaps_found": 1,
  "gaps_to_ask": 0,
  "auto_assumed": [
    {"from": "missing_goal_source", "why": "目标未标注来源，按用户直接指令执行（已假设）", "assume": true}
  ],
  "gaps_to_ask_details": []
}
```

## 4. 审计演进（三轮，发现并修复真问题）

| 轮次 | 发现 | 修复 |
|---|---|---|
| 一轮（基线 99 白名单） | 10/10 PASS 但 **5/10 触发 ambiguous_option blocking**（T02/T06/T07/T08/T10）——**误触发率 50%**，违反"AI 不得问废话" | 收紧检测（去 `or `/`either` 泛匹配，改"二选一实现"语义） |
| 二轮（收紧后） | 仍 5/10 触发——**根因：LLM 分解的任务描述是英文**（"inspect the repo with glob/read/grep to locate..."），`or `/`either` 匹配误伤 | 只保留强"未拍板"信号（待定/可选/tbd）+ 中英"二选一实现"模式 |
| 三轮（最终） | **0/10 误触发**，且**真歧义不丢**（B3-A 用"用 Rust 或 Go 实现"实测触发） | 补 goal（用户原话）优先检测——LLM 分解会消化"或"，goal 保留用户意图 |

## 5. 关键源码修复（derive_gaps）

`crates/planner/src/lib.rs`：
1. **goal 优先检测**（用户原话保留歧义——LLM 分解可能把"或"消化成单语言）
2. **task 描述兜底**（中英"二选一实现"模式：用/使用/采用/选 + 或 + 实现/写/语言/框架/库；use/implement/choose + or/either + implement/language/framework/library）
3. **why 带选项片段**（可追溯：用户看到"什么或什么"+ 返工面提示）
4. 仅强"未拍板"信号触发（待定/可选/tbd）——英文泛 `or `/`either` 不再误伤并列动作

## 6. 证据文件

- 审计脚本：`bench/b2_gap_audit.py`
- 原始结果：`bench/audit_results.jsonl`（一轮）/ `bench/audit_results2.jsonl`（二轮）/ `bench/audit_results3.jsonl`（三轮，最终）
- 每任务运行日志：VM `/tmp/hearth_audit3/works/<Txx>/run.log`
