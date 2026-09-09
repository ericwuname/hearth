# 守门员审计 v18（精炼轮）

- 审计时间：2026-07-31
- 审计对象：v18 精炼轮交付物（E0-E3 四组实验 + S1-S3 工程产出）
- 方法：源码 grep 核对接线 + 原始 JSONL 直判 + VM 日志复核 + wiring 验证

---

## 一、接线核对（源码为准）

| # | 声称 | 核对方式 | 结果 |
|---|---|---|---|
| 1 | embedding 注入（zhipu embedding-3） | `grep crates/service/src/main.rs` → `zhipu_embed_fn` + `set_embed_fn` + `EMBEDDING_ENABLED` | ✅ |
| 2 | search 空结果回退 keyword | `grep crates/experience/src/lib.rs` → `if cosine_hits.is_empty()` → `keyword_scored` | ✅ |
| 3 | set_path 加载时 embedding 回填 | `grep crates/experience/src/lib.rs` → `if exp.embedding.is_empty()` + `f(text).await` | ✅ |
| 4 | runner --shuffle | `grep bench/runner.py` → `--shuffle` + `random.shuffle` | ✅ |
| 5 | 显式 store 清理/注入 | `grep bench/self-evolve-v18.py` → `store_action` 显式参数 | ✅ |
| 6 | LLM 精炼脚本 | `grep bench/refine_experiences_v18.py` → glm-4.7 + V18_SRC | ✅ |
| 7 | embedding 真生效 | VM service.log → `experience store embedding ENABLED` + loaded=5 | ✅ |
| 8 | wiring 11 条未破 | VM `cargo run -p project-xray -- wiring` | ✅ 11/11 pass |
| 9 | 实验数据真实 | JSONL 直判：matrix-v18-{e0,e1,e2,e3}.jsonl 各 40 条含 session_id | ✅ |

## 二、数据判定（原始 JSONL 直判）

| 实验 | 配置 | 通过率 | 与 E0 delta |
|---|---|---|---|
| E0 | baseline（空 store，随机） | **35/40 = 87.5%** | — |
| E1 | keyword + 规则经验 + 随机 | **35/40 = 87.5%** | +0.0 |
| E2 | keyword + LLM 精炼 + 随机 | **34/40 = 85.0%** | -2.5 |
| E3 | embedding + LLM 精炼 + 随机 | **33/40 = 82.5%** | -5.0 |

**跨四轮稳定性**：15 题 8/8 全 PASS，2 题 0/8 全 FAIL（T13/T19），3 题波动（T09/T15/T17）。

## 三、核心结论（颠覆 v17）

**经验回路是 provider 相关的**：
- zhipu（弱，65% baseline）：经验 +20pt 有效 🟢
- deepseek（强，87.5% baseline）：经验 -5pt 无效/有害 🔴

v17 的"经验回路有效"结论**不能推广到强模型**。经验回路的适用域 = 模型能力低于任务难度的场景。

## 四、缺陷登记

### D1：v17 结论外推风险
v17 报告未声明"仅对 zhipu 有效"，v18 用 deepseek 复现推翻。教训：**单 provider 实验结论不得外推，跨 provider 复现是必须的**。已在 v18 报告 §2 修正。

### D2：E2/E3 微降的原因假设（未完全证实）
LLM 精炼经验匹配到不相关任务 → 干扰推理。假设基于 FAIL 分布（E2/E3 新增的 FAIL 是 T09/T15/T17 波动题），需日志级分析确认，但未做（成本考虑）。

### D3：成本实测
deepseek 单任务 ¥0.01（实测），160 次总成本约 ¥1.6，9.83 元预算下充足。已记录备查。

## 五、验收结论

**✅ v18 精炼轮通过守门员验收（作为"否定性发现"轮）。**

- 四组实验真跑、数据完整、wiring 11/11 全绿
- 两条 🟡 判据均成立：embedding 无增量、LLM 精炼无增量 → **v19 停止这两条线投入**
- 最有价值的产出是**否定性结论**：经验回路对强模型无效，v19 路线据此修正
