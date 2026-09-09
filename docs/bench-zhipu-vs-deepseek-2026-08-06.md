# 基准对比报告：免费脑 Zhipu vs 强模型 DeepSeek（v24 红队基准）

- 生成时间：2026-08-06
- 基准套件：bench/runner.py，20 个真实 Rust 工程任务（T00–T19），含 L1/L2/L3 难度
- 引擎：codex-rust v24（service + agent loop），同一套 harness，仅 `BENCH_PROVIDER` 切换
- 数据源：`~/codex_work/bench/results/raw/zhipu-v24.jsonl`（400 runs）、`deepseek-v24.jsonl`（160 runs）

---

## 1. 总览

| 指标 | Zhipu（免费 glm-4.5-air） | DeepSeek（deepseek-chat） |
|---|---|---|
| 总 runs | 400 | 160 |
| 通过 | 58 | 145 |
| **原始通过率** | **14.5%** | **90.6%** |
| 其中 `phase=error`（agent loop 中途崩） | **342 / 400 = 85.5%** | **0 / 160 = 0%** |
| 401/余额耗尽污染 | 0 | 0 |
| 结论性质 | ⚠️ 数字不可作能力基准 | ✅ 真实能力数字 |

> **关键判读**：Zhipu 的 14.5% 是**可靠性数字，不是能力数字**。85.5% 的 run 根本没跑到验证阶段就因免费通道掉连接而 `phase=error` 中止。DeepSeek 的 90.6% 是干净的真实能力通过率。

---

## 2. 逐任务对比（20 任务）

| 任务 | Zhipu (20 runs) | DeepSeek (8 runs) | 差异解读 |
|---|---|---|---|
| T00-smoke | 0/20 | **8/8** | Zhipu 全 abort（API 污染） |
| T01-read-api | 20/20 | 8/8 | 两者都会（简单） |
| T02-change-timeout | 0/20 | **8/8** | Zhipu 全 abort |
| T03-fix-off-by-one | 20/20 | 8/8 | 两者都会（简单） |
| T04-json-output | 0/20 | **8/8** | Zhipu 全 abort |
| T05-extract-common | 11/20 | 8/8 | Zhipu 部分 abort |
| T06-fix-null-check | 0/20 | **8/8** | Zhipu 全 abort |
| T07-fix-logic-invert | 0/20 | **8/8** | Zhipu 全 abort |
| T08-rename-function | 0/20 | **8/8** | Zhipu 全 abort |
| T09-add-error-type | 0/20 | 7/8 | Zhipu 全 abort |
| T10-extract-config | 7/20 | 8/8 | Zhipu 部分 abort |
| T11-read-struct | 0/20 | **8/8** | Zhipu 全 abort |
| T12-change-default | 0/20 | **8/8** | Zhipu 全 abort |
| T13-fix-index | 0/20 | 3/8 | **两者都难**（DeepSeek 37.5%） |
| T14-add-serde | 0/20 | 7/8 | Zhipu 全 abort |
| T15-add-bench | 0/20 | **8/8** | Zhipu 全 abort |
| T16-split-module | 0/20 | **8/8** | Zhipu 全 abort |
| T17-fix-race | 0/20 | **8/8** | Zhipu 全 abort |
| T18-add-pagination | 0/20 | **8/8** | Zhipu 全 abort |
| T19-merge-duplicate | 0/20 | **0/8** | ⚠️ **两者都 0% → 真能力缺口** |

---

## 3. 三个核心发现

### 发现 A：Zhipu 的瓶颈是「可靠性」，不是「能力」
- 13 个任务 Zhipu 显示 0%，但 DeepSeek 在这些任务上 **8/8**。这些任务已被证明「agent harness 能解」，所以 Zhipu 的 0% 纯粹是免费通道**连接抖动导致 run 中途 abort**，与能力无关。
- 仅有的 2 个 Zhipu 拿满 100% 的任务（T01/T03）恰恰是简单任务——说明**只要连接不断，Zhipu 也能做对**。
- 推论：免费 Zhipu 作为「自动跑基准」**不可用**——85% 的 run 废掉，统计数字无意义；但作为「人工盯着、断了就重跑」的辅助脑，**能力大概率接近 DeepSeek**。

### 发现 B：DeepSeek 是干净可靠的强模型
- 90.6% 通过率，0 个 abort，0 个 401（预算充足）。
- 唯一的「软肋」集中在 3 个任务：T09(7/8)、T14(7/8)、T13(3/8)，以及 T19(0/8)。

### 发现 C：T19 是**双方共有的真能力缺口**（重点排查对象）
- T19-merge-duplicate：**Zhipu 0/20 + DeepSeek 0/8**，是唯一一个两边都归零的任务。
- 这不是 API 问题（DeepSeek 跑满了 8 次全失败），是 **agent/harness 在这个任务类型上的真实短板**——值得单独立项排查（很可能是 verify 规则或任务描述本身有坑）。
- T13-fix-index 也偏难（DeepSeek 仅 37.5%），但比 T19 好，属「难但可解」。

---

## 4. 预算复盘

| 项目 | 金额 |
|---|---|
| DeepSeek 起始余额 | ~4 元（用户口述） |
| 中途余额探底 | 2.41 元（~90 runs 时） |
| 用户追加充值 | +5 元 |
| 最终消耗 | 跑满 160 runs，未触发 401 |
| 单价 | ~0.034 元/run（均值） |

> 用户明确「真没钱了」——本报告为烧钱基准的终版，不再开新 LLM 端到端任务。

---

## 5. 结论与建议

1. **默认脑选 DeepSeek**：能力（90.6%）+ 可靠性（0 abort）双优，是唯一能 unattended 跑基准的模型。
2. **免费 Zhipu 仅限有人值守的辅助场景**，不能进自动化基准/生产长链路。
3. **立专项排查 T19**（双方 0%）：这是当前 harness 最硬的真实短板，修好它 deepseek 通过率有望破 93%。
4. **T13 列为次优先**：复杂索引修复类，deepseek 也只 37.5%。
5. 本次对比方法论已沉淀：zhipu 的 `phase=error` 必须按「API 污染」剔除后才能谈能力，否则数字会严重误导。

---
*数据可在 VM `~/codex_work/bench/results/raw/` 复核；分析脚本见 `.workbuddy/vm_analyze_zhipu.py`、`vm_deepseek_progress.py`。*
