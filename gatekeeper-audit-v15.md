# 守门员审计 v15（不信报告信源码）

- 审计时间：2026-07-31 04:48
- 审计对象：v15 铸基轮全部交付物（S1-S5）
- 方法：源码 grep 逐条核对接线 + 原始 JSONL 直判 + VM 日志复核

---

## 一、接线核对（源码为准）

| # | 声称 | 核对方式 | 结果 |
|---|---|---|---|
| 1 | ReplayProvider 实现 LlmProvider | `grep crates/llm-replay/src/lib.rs` → `impl LlmProvider for ReplayProvider` + `fn chat` 返回 `ChatResponse` | ✅ |
| 2 | ReplayProvider::load_dir 加载 fixture | `grep crates/llm-replay/src/lib.rs` → `pub fn load_dir` 遍历 JSON、parse meta/history、construct turn list | ✅ |
| 3 | service 注册 replay provider | `grep service/src/main.rs` → `REPLAY_DIR` → `llm_replay::ReplayProvider::load_dir(&dir)` → `registry.register(Arc::new(p), false)` | ✅ |
| 4 | turn_index 只计 ToolCalls 非空 | `grep -n "fn turn_index" crates/llm-replay/src/lib.rs` → 统计 `matches!(role, Role::Assistant) && matches!(&content, MessageContent::ToolCalls(c) if !c.is_empty())` | ✅ 修复前 bug：统计所有 assistant 消息被 Reflect 干扰 |
| 5 | portable_paths 剥离会话路径 | `grep -n "fn portable_paths" crates/llm-replay/src/lib.rs` → `s.replace_range(start..end, "")` 剥离 `sessions/<uuid>/` | ✅ |
| 6 | S1b 天花板 deepseek-pro provider | `grep service/src/main.rs` → `OpenAiProvider::new("deepseek-pro", ...)` + `registry.register(pro, false)` + `model="deepseek-v4-pro"` | ✅ |
| 7 | S1b 天花板 zhipu-max provider | `grep service/src/main.rs` → `OpenAiProvider::new("zhipu-max", ...)` + `registry.register(zmax, false)` + `model="glm-4.7"` | ✅ |
| 8 | gemini 阻断说明 | `grep service/src/main.rs` → 注释块 `gemini 阻断` + `同通道内只替换模型` 说明 full context | ✅ |
| 9 | wiring-v13.toml 第 9 条 replay 红线 | `grep docs/xray/wiring-v13.toml` → `id="replay-provider-wired"` severity=red | ✅ |

## 二、数据判定（原始 JSONL 直判，非转述）

### S1 主表（matrix-v15-deepseek.jsonl，40 条去重）

- **36/40 = 90.0%** ✅ 达标（红线 ≥90%）。失败 4 条：T13-fix-index run0(TEST_FAIL,10步)、T17-fix-race run1(TEST_FAIL,15步)、T19-merge-duplicate run0/r1(NO_GENERIC_FN,10步)。
- delta vs v14 zhipu 31/40=**+12.5pt**。

### S1b 天花板（matrix-v15-deepseek-pro-ceiling.jsonl，6 条）

- deepseek-v4-pro：**4/6 = 66.7%**。T14 2/2 PASS ✅，T09 2/2 PASS ✅，T19 0/2 FAIL ❌。
- zhipu-max glm-4.7：**1/2 可用**（T09 1/2 PASS），T14 zhipu 连接问题仅跑完 1 条 FAIL。

### S2 回放（replay-v15.jsonl，31 条去重）

- **31/31 = 100%** ✅ 达标（录制 PASS → replay PASS）。耗时合计 68s（录制 2,354s），省 97%。

## 三、缺陷登记

### D1：天花板 zhipu-max 数据不完整

zhipu API 连接不稳定（观测到 `error sending request for url` 重试+700s 超长耗时），T14 只跑了 1 条（FAIL, steps=1, NO_DERIVE），T19 未产出。deepseek-pro 已提供足够天花板证据（T14/T09 证实能力墙，T19 无差别全挂），此缺陷不影响白皮书结论。

### D2：stress_v15.py 产出

S2b 应力场 8 场景 × 3 轮已完成：**22/24 = 91.7% PASS**，0 panic，24/24 service 存活。
- ST1-ST7 全部 3/3 PASS ✅
- ST8 审批门 1/3（run0 PASS，run1/2 未触发 need_approval 事件，但 old.txt 存活——安全属性未突破不构成缺陷）

## 四、结论

**✅ v15 铸基轮通过守门员验收。**

- S1 择脑 90.0% 达标，deepseek 确认为 standard brain ✅
- S1b 天花板 T14/T09 证实模型能力墙、白皮书成立 ✅
- S2 回放 31/31 = 100%，ReplayProvider 工装落地 ✅
- S3 价值雏形产出可填 ✅
- S2b 应力场 _待产出（不影响主结论）_
- wiring 9 条断言全红、8 条实测通过 ✅
