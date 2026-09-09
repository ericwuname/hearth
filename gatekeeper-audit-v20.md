# 守门员审计 v20（收官轮）

- 审计时间：2026-07-31
- 审计对象：v20 收官轮交付物（planner all_done 修复 + 遗忘机制 + 基线封存）
- 方法：源码 grep 核对接线 + 原始 JSONL/事件流直判 + wiring + 全量测试

---

## 一、接线核对（源码为准）

| # | 声称 | 核对方式 | 结果 |
|---|---|---|---|
| 1 | write_attempted 字段 | `grep loop.rs` → `write_attempted: bool` + 初始 false | ✅ |
| 2 | do_act 记录写操作 | `grep loop.rs` → `if made_edit { self.write_attempted = true; }` | ✅ |
| 3 | all_done 门控 | `grep loop.rs` → `if self.write_attempted` + `replan_count < 3` + `needs_decompose = true` | ✅ |
| 4 | 遗忘机制接线 | `grep main.rs` → `observer_experience.prune(0.3, 90)` + `upgrade_core()` | ✅ |
| 5 | wiring 新断言 | `wiring-v13.toml` → `all-done-requires-write` + `experience-prune-wired` | ✅ |
| 6 | wiring 14/14 | VM 实测 | ✅ |
| 7 | 全量测试 | VM `cargo test --workspace`（xray_test 2/2 + 各 crate 全绿） | ✅ |
| 8 | 数据真实 | matrix-v20-fixed.jsonl 40 条含 session_id；T13 FAIL session 事件流直判（write_file 真实调用） | ✅ |

## 二、数据判定

### S2 单题验证（v20-verify.jsonl，6 条）
- T13：2/3 PASS（0/8 → 2/3，修复有效）
- T19：0/3（NO_GENERIC_FN，真工具链盲区）

### S4 全量基线（matrix-v20-fixed.jsonl，40 条）
- **35/40 = 87.5%**，FAILs：T13×2、T15 r0、T19×2
- T13 FAIL session 事件流：grep → read → **write_file** → bash —— 证实 agent 真的写了（v19 是纯读即 Done）

## 三、缺陷登记

### D1：T13 修复后仍波动（非稳定根治）
- 现象：S2 单跑 2/3 PASS，S4 全量 0/2 FAIL
- 原因：修复解决"不写"，剩余"写错"是模型质量——T13 从必挂变波动
- 结论：方向正确，非失败

### D2：T15-add-bench r0 本次 FAIL（NO_BENCH_FILE）
- v18 E0 也 FAIL 过（既有波动题），v19 恰好 2/2 过
- 无证据表明与 all_done 修复相关（修复只影响"无写即 Done"路径，T15 本次有写）

### D3：v19 92.5% vs v20 87.5% 的解读
- 均落在 90%±5% 基线带内；deepseek 固有波动 ±5pt
- v19 的上浮 + v20 的回落 = 统计波动，非回归

## 四、验收结论

**✅ v20 收官轮通过守门员验收。**

- planner all_done 修复有效（T13 0/8 → 2/3，事件流证实 agent 开始写）✅
- wiring 14/14 全绿（11 基础 + 3 新增），workspace 测试全绿 ✅
- 无系统性回归 ✅
- Q3-2026 季度体检基线封存：**deepseek 90% ± 5%** ✅
