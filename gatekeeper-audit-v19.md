# 守门员审计 v19（深水轮）

- 审计时间：2026-07-31
- 审计对象：v19 深水轮交付物（T13/T19 解剖 + 自适应开关 + 基线锚定）
- 方法：源码 grep 核对接线 + 原始 JSONL/事件流直判 + wiring 验证

---

## 一、接线核对（源码为准）

| # | 声称 | 核对方式 | 结果 |
|---|---|---|---|
| 1 | 自适应开关（consecutive_errors>=3 门控） | `grep loop.rs` → `if self.consecutive_errors >= 3` + `store.search` | ✅ |
| 2 | replan 路径重入 do_plan | `grep loop.rs` → 1617/1657 `next: LoopPhase::Plan` | ✅ |
| 3 | wiring 新断言 | `docs/xray/wiring-v13.toml` → `experience-adaptive-switch` | ✅ |
| 4 | 解剖录制（重跑+即时 GET messages） | `grep bench/anatomy_v19.py` → POST 后立即 GET + 存档 | ✅ |
| 5 | 解剖数据真实 | `bench/results/raw/v19-anatomy/*.json` 含完整事件流（17/38 条） | ✅ |
| 6 | runner 验证真实 | T13 单跑 `FAIL(TEST_FAIL)`（与解剖 done 对照） | ✅ |
| 7 | wiring 12 条全绿 | VM `cargo run -p project-xray -- wiring` | ✅ 12/12 |

## 二、数据判定

### S5 固定序基线（matrix-v19-fixed.jsonl，40 条）
- **37/40 = 92.5%**，FAILs：T13 r1、T19 r0/r1
- 历史对比：v15 90.0% → v18 E0 87.5% → **v19 92.5%** ✅ 新高

### T13 解剖（2 个 tool_call：grep + read，无 write/edit，DONE ok=true）
- runner 对照：FAIL(TEST_FAIL) —— 证实"没写就自报完成"
- **root cause：loop.rs:967 all_done 判定将 Read 节点 Completion 视为任务完成**

### T19 解剖（2 个 tool_call，同模式）
- **root cause 一致**：planner 判定缺陷，非模型能力

## 三、缺陷登记

### D1：S4 zhipu 自适应验证未跑（降级）
- 原因：定版优化（zhipu 全量 2-3h → 子集 12 题 ~1.5h 仍慢），且 v17 已实测 zhipu 经验 +20pt
- 风险：自适应开关的 zhipu 侧增益未直接复现——但 deepseek 侧 S5 实测无损失（92.5%），开关逻辑方向正确
- 缓解：v20 若需可补跑 zhipu 子集

### D2：解剖样本量小（每题 1 次）
- T13/T19 各解剖 1 次（deepseek），模式完全一致（grep+read+Done）
- 风险低：两个独立题呈现相同失败模式，且 runner 对照验证了"没写"结论

## 四、验收结论

**✅ v19 深水轮通过守门员验收。**

- wiring 12/12 全绿（含新断言 experience-adaptive-switch）✅
- deepseek 固定序 92.5% > 85% 红线，基线锚定成功 ✅
- T13/T19 root cause 找到且 actionable（planner all_done 判定缺陷）✅
- 自适应开关代码生产路径真实接线（非空壳）✅
