# v18 精炼轮实验报告 —— 经验回路是 provider 相关的

> 实验日期：2026-07-31
> Provider：deepseek v4-flash（v15 定版 standard brain；用户充值 ¥10 恢复）
> 方法：E0-E3 四组正交实验（VARIABLE_A 搜索方式 / VARIABLE_B 经验来源 / VARIABLE_C 任务顺序），全部 20×2 随机序
> 对比对象：v17 经验回路实验（zhipu glm-4.5-air，65%→85%）

---

## 一、四组实验结果

| 实验 | 配置 | 通过率 | delta vs E0 |
|---|---|---|---|
| **E0** | baseline（空 store，随机序） | **35/40 = 87.5%** | — |
| **E1** | keyword + 规则构造经验 + 随机 | **35/40 = 87.5%** | +0.0 pt |
| **E2** | keyword + LLM 精炼经验（glm-4.7）+ 随机 | **34/40 = 85.0%** | **-2.5 pt** |
| **E3** | embedding（zhipu embedding-3）+ LLM 精炼 + 随机 | **33/40 = 82.5%** | **-5.0 pt** |

原始数据：`bench/results/raw/matrix-v18-{e0,e1,e2,e3}.jsonl`

## 二、核心发现：经验回路是 provider 相关的

| | v17（zhipu glm-4.5-air） | v18（deepseek v4-flash） |
|---|---|---|
| baseline 通过率 | 65.0% | 87.5% |
| 注入失败经验后 | **85.0%（+20.0 pt）** | **82.5%（-5.0 pt）** |
| 结论 | 经验有效 🟢 | **经验无效/有害 🔴** |

**模型基线强度是决定性变量**：
- **弱模型（zhipu 65%）**：agent 不会做 → 经验里的具体修复步骤给方向 → 大幅提升
- **强模型（deepseek 87.5%）**：agent 本来就会做 → 经验是噪音 → 规则经验泛泛建议可能污染 prompt，干扰推理

**推论**：v17 的"经验回路有效（+20pt）"结论**不能推广到强模型**。经验回路的适用域 = 模型能力低于任务难度的场景。

## 三、VARIABLE 判定

### VARIABLE_A：搜索方式（keyword vs embedding）
- E2（keyword）85.0% > E3（embedding）82.5% → **embedding 无增量，keyword 已够** ❌
- 注：embedding 注入验证成功（service log `embedding ENABLED` + set_path 回填 loaded=5），是真实的语义搜索对照。

### VARIABLE_B：经验来源（规则 vs LLM 精炼）
- E1（规则）87.5% > E2（精炼）85.0% → **LLM 精炼无增量，甚至略降** ❌
- glm-4.7 精炼质量验证 OK（T13 指出 UTF-8 索引、T15 指出 benches/ 目录、T19 指出泛型参数），但高质量经验对 deepseek 仍是干扰。

### VARIABLE_C：任务顺序（固定 vs 随机）
- E0（随机）87.5% ≈ v15（固定序）90.0% → **无顺序效应** ✅
- 随机化后的 deepseek 基线稳定，v17 zhipu 的 65% 是 zhipu 波动，非顺序效应。

## 四、跨四轮稳定性

- **8/8 全 PASS（15 题）**：T00/T01/T02/T03/T04/T05/T06/T07/T08/T10/T11/T12/T14/T16/T18 —— deepseek 对这些题极其稳定
- **0/8 全 FAIL（2 题）**：T13-fix-index、T19-merge-duplicate —— 能力墙，任何经验配置都救不了
- 波动题：T09/T15/T17 —— 经验注入前后在 PASS/FAIL 间波动（噪音）

## 五、工程产出（S1-S3）

1. **S1 embedding 注入**：`service/main.rs` 新增 `zhipu_embed_fn()`（zhipu embedding-3，256 维，EMBEDDING_ENABLED 开关）+ `experience/src/lib.rs` search 空结果回退 keyword + set_path 加载时 embedding 回填。
2. **S2 LLM 精炼**：`bench/refine_experiences_v18.py`（glm-4.7 精炼 FAIL → 结构化经验；glm-4.5-flash 实测空 content 不可用）。
3. **S3 随机化**：`runner.py` 新增 `--shuffle` + `bench/self-evolve-v18.py` 显式 store 清理/注入 + EMBEDDING 开关。
4. **wiring 11/11 保持全绿**（VM 实测）。

## 六、v19 路线修正

**原规划（v19 全投入经验管线）需要重新评估**：

| 原计划 | 修正后 |
|---|---|
| embedding + LLM 精炼全投入 | ❌ 无增量，停止投入 |
| 自主闭环（do_reflect 自动凝练） | 🟡 仅对弱模型有意义——deepseek 不需要 |
| 遗忘机制 prune/upgrade_core | ✅ 仍值得（防 store 膨胀） |
| **新方向：经验回路适用域边界** | ✅ 明确"模型强度决定经验价值"——经验系统应作为**可选降级通道**（模型弱时启用），而非常驻机制 |

**v19 建议**：
1. 把经验注入改为**自适应开关**：检测 agent 连续 N 次失败 → 才注入经验（避免强模型被干扰）。
2. 深挖 T13/T19 能力墙（deepseek 也挂）——这是比经验回路更高优先级的瓶颈。
3. deepseek 版基准回归到 20×2 固定序作为季度体检基线（90% 稳定性确认）。

## 七、验收红线核对

| 判据 | 结果 |
|---|---|
| 🔴 wiring 11 条断言破裂 | ✅ 未破（11/11 pass） |
| 🔴 实验数据造假 | ✅ 真跑 VM（四组 JSONL 含 session_id/wall_s） |
| 🟡 embedding 相关度 < keyword | ✅ **成立**——embedding 82.5% < keyword 85.0%，不需要做 embedding |
| 🟡 LLM 精炼 +5pt 不成立 | ✅ **成立**——精炼 85.0% < 规则 87.5%，v19 省掉 |
| 🔵 通过率提升幅度 | 无提升（-5pt），如实记录 |
