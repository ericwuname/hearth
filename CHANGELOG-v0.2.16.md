# CHANGELOG v0.2.15 → v0.2.16 — P2-MEMORY-CONTEXT-01（2026-08-30）

> 使命：Truth survives time——压缩之后，Hearth 仍然知道真正发生过什么。
> 证据：docs/memory-context-compaction-audit.md + docs/data/memory-context-20260830/ + Final Report。

## 重大发现与修复：单 run 压缩死代码（Node 02/12）

- **发现**：`record_tool_exchange` 把每次工具交换 push 进 run 启动时的唯一
  Turn 0 → `history.len()` 恒 1 → `maybe_compact` 的 keep_from==0 守门恒 false
  → **`hearth chat` 主路径压缩从不触发**（真机 A1：44k est 历史 0 压缩）。
  历史上压缩只在 REPL 多轮会话生效。
- **修复**：turn 粒度对齐——一次工具交换 = 一个新 Turn（语义与 REPL 对齐）。
  红：单元红 + A1 真机；绿：单元 + e2e（`test_single_run_compaction_e2e`）。

## Compaction 现代化（Node 01/02/12，批-2/批-3 落地）

- **estimate 诚实计数**：`estimate_chars` 从 Debug 包装 + UTF-8 字节（虚增
  1.39-2.49×，Node 01 实测）改为 content 真实字符计数；legacy 口径保留为
  `legacy_estimate_chars`（回滚通道专用）。
- **provider-aware 阈值**（批-2 最小切片 I-6）：阈值 = 窗口 ×
  `HEARTH_COMPACT_WINDOW_RATIO`（默认 0.6）× 2.55 chars/token（Node 01 实测
  combined 系数）；窗口 = `HEARTH_CONTEXT_TOKENS` env 覆盖 > provider caps
  `max_context_tokens`；未知窗口回落遗留 32k 常量。
- **回滚通道**（§30）：`HEARTH_COMPACTION_MODE=legacy` 整体回滚旧口径
  （Debug+bytes est + 32k 常量），保留至少一个 release cycle。
- **RC40 修正**：`context_fill_pct` → `compact_pressure_pct`——语义如实
  （距压缩阈值 %，非"窗口填充率"），分母动态化（当前生效阈值）；
  status.rs 示例与断言同步（S-1）。

## INV-M01 确定性 fixture（§21 S-7）

- `test_inv_m01_fact_survives_compaction`：压缩后 state 层
  （goal/original_goal/constraints/acceptance_criteria/scratch）全 preserved +
  archive 原文可恢复（B 档）+ 摘要重构（goal/工具/写盘）+ 检索提示在场。
- `test_inv_m01_loss_mode_without_archive`：archive 缺席时原文事实确实消失
  ——fixture 能失败的确定性证明（无 archive 即无 B 档通道）。

## 静默截断（Node 10）

- 10 处清单分类完成（3 FACT_RISK / 2 NEEDS_REVIEW / 5 SAFE）——
  见 `memory-context-compaction-audit.md` §7。本轮修复：RC40 改名 + web.rs
  8000 截断补标记。MAX_HISTORY_MSGS=40（prompt 切片，静默无标记）登记为
  FACT_RISK，修复涉 prompt 结构（超本轮边界，移交）。

## 测试

新增 7：INV-M01 双 fixture、turn 粒度 e2e、env/注入阈值矩阵、estimate 诚实
计数。Final Gate v0.2.16：四 RC 全 0（`.133:~/t_gate_mc_final.log`）。
