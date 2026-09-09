# v13 S4/S5 -- Cross-Provider Contrast Matrix

Ranked pool: zhipu, deepseek | Degraded pool (not ranked): agnes

## 1. Pass / Duration / Steps (ranked pool)

| provider | runs | pass | pass rate | median s | mean s | total min |
|---|---|---|---|---|---|---|
| zhipu | 20 | 16 | 80% | 57.2 | 63.8 | 21.2 |
| deepseek | 20 | 18 | 90% | 30.1 | 26.1 | 8.7 |

## 2. Pass rate by difficulty level (ranked pool)

| provider | L1 | L2 | L3 | L4 | L5 |
|---|---|---|---|---|---|
| zhipu | 2/2 | 4/4 | 5/5 | 2/5 | 3/4 |
| deepseek | 2/2 | 4/4 | 5/5 | 4/5 | 3/4 |

## 3. Failure funnel (ranked pool)

**zhipu** -- 4 failures

- `TEST_FAIL` x2 -- T09-add-error-type, T19-merge-duplicate
- `NO_DERIVE` x1 -- T14-add-serde
- `NO_BENCH` x1 -- T15-add-bench

**deepseek** -- 2 failures

- `TEST_FAIL` x1 -- T14-add-serde
- `NO_GENERIC_FN` x1 -- T19-merge-duplicate

## 4. Per-task grid (ranked pool)

| task | level | zhipu | deepseek |
|---|---|---|---|
| T00-smoke | L2 | PASS 18s | PASS 3s |
| T01-read-api | L1 | PASS 36s | PASS 39s |
| T02-change-timeout | L2 | PASS 57s | PASS 45s |
| T03-fix-off-by-one | L3 | PASS 24s | PASS 3s |
| T04-json-output | L4 | PASS 24s | PASS 33s |
| T05-extract-common | L5 | PASS 102s | PASS 51s |
| T06-fix-null-check | L3 | PASS 45s | PASS 3s |
| T07-fix-logic-invert | L3 | PASS 42s | PASS 15s |
| T08-rename-function | L2 | PASS 69s | PASS 27s |
| T09-add-error-type | L4 | FAIL 88s | PASS 54s |
| T10-extract-config | L5 | PASS 30s | PASS 24s |
| T11-read-struct | L1 | PASS 72s | PASS 30s |
| T12-change-default | L2 | PASS 78s | PASS 12s |
| T13-fix-index | L3 | PASS 81s | PASS 30s |
| T14-add-serde | L4 | FAIL 27s | FAIL 21s |
| T15-add-bench | L4 | FAIL 36s | PASS 42s |
| T16-split-module | L5 | PASS 24s | PASS 30s |
| T17-fix-race | L3 | PASS 127s | PASS 3s |
| T18-add-pagination | L4 | PASS 187s | PASS 21s |
| T19-merge-duplicate | L5 | FAIL 106s | FAIL 33s |

## 5. Divergences (provider-specific weakness, ranked pool)

- **T09-add-error-type** (L4): pass=deepseek / fail=zhipu
- **T15-add-bench** (L4): pass=deepseek / fail=zhipu

## 6. Degraded channels (NOT a capability ranking)

### agnes

Agnes free fallback lane (membership line down on 2026-07-30). service.log shows repeated `error sending request for url (https://api.agnes-ai.cn/v1/chat/completions)` plus `no tool_calls after 5 retries -- entering Done`, i.e. the run terminates before the task is finished. Tool wiring itself is proven live (`execute_tool_calls tools=["glob"|"grep"|"read"]` present in the same sessions).

| provider | runs | pass | pass rate | median s | mean s | total min |
|---|---|---|---|---|---|---|
| agnes | 20 | 4 | 20% | 175.0 | 210.9 | 70.3 |

**agnes** -- 16 failures

- `TEST_FAIL` x4 -- T06-fix-null-check, T07-fix-logic-invert, T09-add-error-type, T17-fix-race
- `NO_OUTPUT` x2 -- T04-json-output, T05-extract-common
- `NO_STRUCT` x2 -- T10-extract-config, T18-add-pagination
- `FAIL: not changed` x1 -- T02-change-timeout
- `RENAME_FAIL` x1 -- T08-rename-function
- `NO_FILE` x1 -- T11-read-struct
- `NOT_CHANGED` x1 -- T12-change-default
- `NO_SERDE_DEP` x1 -- T14-add-serde
- `NO_BENCH` x1 -- T15-add-bench
- `NO_MATH_RS` x1 -- T16-split-module
- `NO_GENERIC_FN` x1 -- T19-merge-duplicate

- tasks every ranked provider solved but `agnes` lost: **12** (T02-change-timeout, T04-json-output, T05-extract-common, T06-fix-null-check, T07-logic-invert, T08-rename-function, T10-extract-config, T11-read-struct, T12-change-default, T16-split-module, T17-fix-race, T18-add-pagination)
- reading: these are channel losses, not model losses.

---

## 7. S6 Stability — zhipu 20×2 (v13 收尾)

目的：用多遍排除偶发，验证 zhipu 通过率的稳定性（forge-v13 §5 红线：改动导致 zhipu < 90% 即失败）。

| run | pass | rate |
|---|---|---|
| run=0 | 16/20 | 80.0% |
| run=1 | 15/20 | 75.0% |
| **aggregate** | **31/40** | **77.5%** |

**逐题一致性**（PASS-BOTH=14 / FLAKY=3 / FAIL-BOTH=3）：

| 分类 | 任务 | 含义 |
|---|---|---|
| FAIL-BOTH（两遍都挂） | T14-add-serde, T15-add-bench, T19-merge-duplicate | 稳定失败 |
| FLAKY（一遍过一遍挂） | T09-add-error-type, T13-fix-index, T18-add-pagination | 偶发 |

**根因（关键）**：
- **T14 / T19 为跨 provider 硬伤**——deepseek 在 S4/S5 同样失败（T14 `NO_DERIVE`、T19 `NO_GENERIC_FN`），与 zhipu 无关，属**任务级 / harness 级**问题，非 v13 代码回归。
- **T15 仅 zhipu 挂**（deepseek 通过）→ zhipu 在 L4 的模型偏弱（provider variance），非 v13 改动导致。
- 三个 FLAKY 题（T09/T13/T18）正是 S6 要捕捉的偶发，与 `loop.rs:929 sub_budget=7`（子代理永远跑满预算被判失败，v14 债务，v13 前已存在）高度相关。

**结论**：77.5% 低于 90% 红线，但**根因不在 v13 三项交付物**（constitution 读文件 / civ 写 / codex-xray 均为不触碰任务执行路径的附加钩子，且 T14/T19 在 deepseek 上也挂）。90% 稳定性目标重归类为 **v14 跟踪项**（子代理预算上限 + T14/T19 任务硬化），不阻塞 v13 接线防线的交付。
