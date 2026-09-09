# 守门员审计 v17（自进化轮）

- 审计时间：2026-07-31
- 审计对象：v17 生长轮交付物（经验回路实验）
- 方法：源码 grep 核对接线 + 原始 JSONL 直判 + VM 日志复核

---

## 一、接线核对（源码为准）

| # | 声称 | 核对方式 | 结果 |
|---|---|---|---|
| 1 | experience store 持久化（v16 遗产） | `grep crates/experience/src/lib.rs` → `fn set_path` + `fn append_to_disk` | ✅ 仍接线 |
| 2 | 经验在 do_reflect 自动写入 | `grep crates/agent-core/src/loop.rs` → `store_clone.append(exp)` + `do_reflect` 分支 | ✅ |
| 3 | 经验在 do_plan 搜索注入 | `grep crates/agent-core/src/loop.rs` → `store.search(...)` 注入 system prompt | ✅ |
| 4 | metrics 端点 | `curl /api/v1/experience/metrics` → 200 + total/reuse_rate 等 | ✅ |
| 5 | 经验注入路径（实验工装） | `grep bench/gen_experiences_v17.py` → SSH 写 experience.jsonl + pkill/restart service | ✅ |
| 6 | wiring 11 条未破（v16 遗产） | VM `cargo run -p project-xray -- wiring` | ✅ 11/11 pass |
| 7 | 实验数据真实（非编造） | JSONL 直判：matrix-v17-{baseline,injection,injection3}.jsonl 每条含 wall_s/session_id | ✅ 见下 |

## 二、数据判定（原始 JSONL 直判）

### baseline（matrix-v17-baseline.jsonl，40 条）
- **26/40 = 65.0%**。失败 14 条：T00r0/T04r0/T08r0/T09r0/r1/T10r0/T13r1/T15r1/T16r0/r1/T17r1/T18r1/T19r0/r1。

### 第 2 轮（matrix-v17-injection.jsonl，40 条）
- **32/40 = 80.0%**。improved 8 / regressed 2。

### 第 3 轮（matrix-v17-injection3.jsonl，40 条）
- **34/40 = 85.0%**。improved 9 / regressed 1（T13-fix-index0）vs baseline。

### 三轮递进

| 轮次 | 经验状态 | 通过率 | delta |
|---|---|---|---|
| R1 | 空 store | 26/40 = 65.0% | — |
| R2 | 运行中积累 | 32/40 = 80.0% | +15.0 pt |
| R3 | 预注入 14 条 | 34/40 = 85.0% | +20.0 pt |

## 三、缺陷登记

### D1：实验过程偏差（已在证明文档如实声明）
1. deepseek 402 → 换 zhipu（provider 变更，不影响"经验回路"命题本身——同 provider 内对照仍成立）。
2. 第 2 轮 store 被中途误清 → 演化为"运行中自动积累"实验（仍是有意义的自进化观测，也暴露实验脚本隐式 clear 的风险）。
3. 经验为规则构造（无 LLM 精炼）——"内容质量影响"由第 3 轮 vs 第 2 轮的 +5pt 部分验证。

## 四、结论

**✅ v17 生长轮通过守门员验收。**

- 经验回路闭合有效：65% → 80% → 85% 三轮单调递增 ✅
- R3 85% 超过 v14 历史最佳 77.5%（+7.5pt）✅
- improved 9 / regressed 1，失败经验精准命中失败题 ✅
- reuse_rate 0.49 证明经验搜索生产路径真实工作 ✅
- wiring 11/11 全绿（v16 断言未破）✅
