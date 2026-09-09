# CHANGELOG v18.0 — 精炼轮

经验回路正交实验轮（E0-E3），产出**否定性发现**：经验回路对强模型（deepseek）无效。

## Added
- **S1 embedding 注入**：`service/main.rs` `zhipu_embed_fn()`（zhipu embedding-3，256 维，`EMBEDDING_ENABLED=0` 可关）+ `experience/lib.rs` search 空结果回退 keyword + set_path 加载时 embedding 回填。
- **S2 LLM 精炼**：`bench/refine_experiences_v18.py`（glm-4.7 精炼 FAIL → 结构化经验；glm-4.5-flash 空 content 不可用——已实测声明）。
- **S3 随机化**：`runner.py --shuffle` + `bench/self-evolve-v18.py`（E0-E3 驱动，显式 store 清理/注入 + EMBEDDING 开关）。
- **实验报告** `docs/self-evolution-v18-report.md` + 审计 `gatekeeper-audit-v18.md`。

## Experiment Results（deepseek v4-flash，20×2 随机序，成本 ¥1.6）

| 实验 | 配置 | 通过率 | delta |
|---|---|---|---|
| E0 | baseline（空 store） | **35/40 = 87.5%** | — |
| E1 | keyword + 规则经验 | **35/40 = 87.5%** | +0.0 |
| E2 | keyword + LLM 精炼 | **34/40 = 85.0%** | -2.5 |
| E3 | embedding + LLM 精炼 | **33/40 = 82.5%** | -5.0 |

## Key Findings（否定性）

1. **经验回路是 provider 相关的**：zhipu（弱 65%）+20pt 有效；deepseek（强 87.5%）-5pt 无效/有害。v17 结论不适用于强模型。
2. **embedding 无增量**（82.5% < 85.0%）→ v19 停止该线投入。
3. **LLM 精炼无增量**（85.0% < 87.5%）→ v19 停止该线投入。
4. **无顺序效应**（E0 随机 87.5% ≈ v15 固定 90%）→ v17 的 65% 是 zhipu 波动。
5. **稳定题**：15 题 8/8 全 PASS；T13/T19 四轮全 FAIL（能力墙，经验救不了）。

## Changed
- service 新增 `EMBEDDING_ENABLED` 环境开关（默认开）。
- experience search 空结果回退 keyword（迁移期兼容旧经验）。
- runner 新增 `--shuffle`。

## Known Debt（v19）
1. 经验注入改为**自适应开关**：检测连续失败才注入（避免强模型被干扰）。
2. T13/T19 能力墙深挖（deepseek 也挂——比经验回路更高优先级的瓶颈）。
3. deepseek 基准回归 20×2 固定序作为季度体检基线。
4. deepseek API 余额 ¥8.2（充值 ¥10，本轮消耗 ¥1.8）。
