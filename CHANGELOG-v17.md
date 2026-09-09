# CHANGELOG v17.0 — 生长轮

锻造终结后的第一轮：经验回路闭合验证——证明 agent 能用自己的失败经验变聪明。

## Added
- **经验回路实验工装** `bench/self-evolve-v17.py`：全自动实验（baseline → 经验积累 → 注入 → 重跑 → 对比），含 `--resume` 断点续跑、逐题变化比对。
- **经验构造工装** `bench/gen_experiences_v17.py`：把 baseline FAIL 记录（goal + verify_tail）规则构造为结构化经验，SSH 注入 VM 的 `experience.jsonl` + 重启 service 加载。
- **自进化证明文档** `docs/self-evolution-proof-v17.md`：三轮实验完整记录 + 偏差声明 + 经验引用证据（reuse_rate）。

## Experiment Results（zhipu glm-4.5-air）

| 轮次 | 经验状态 | 通过率 |
|---|---|---|
| 第 1 轮 baseline | 空 store | **26/40 = 65.0%** |
| 第 2 轮 | 运行中自动积累 | **32/40 = 80.0%**（+15.0 pt） |
| 第 3 轮 | 预注入 14 条失败经验 | **34/40 = 85.0%**（+20.0 pt） |

- **三轮单调递增：65% → 80% → 85%，经验越完整通过率越高。**
- R3 vs R1：**improved 9 / regressed 1**（T13-fix-index0），improved 集中在第一轮失败的 L3-L5 难题（T09/T10/T13/T16/T17/T18）。
- R3 的 85% **超过 v14 历史最佳 77.5%**（+7.5pt），同 provider/任务集/工具链仅凭经验回路达成。
- store `reuse_rate = 0.49`——经验搜索在生产路径真实工作（被引用即证明命中）。

## Changed
- **实验 provider 临时切换**：deepseek API 402 欠费 → 本轮实验用 zhipu glm-4.5-air（v14 同款）。deepseek 恢复后可用 `V17_PROVIDER=deepseek` 重跑验证。

## Known Debt（v18）
1. 任务顺序未随机化（T00→T19 固定序），顺序效应未排除——v18 需随机化。
2. 经验内容为规则构造（非 LLM 精炼）——R3 vs R2 的 +5pt 证明内容质量有增量，v18 做 LLM 精炼 + embedding 向量化。
3. deepseek API 欠费待充值（402），恢复后重跑 deepseek 版实验对比。
4. 实验脚本隐式 clear store 风险（self-evolve-v17.py 的 clear 步骤应改为显式参数）。
