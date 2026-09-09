# 锻造报告 v14 —— 淬火轮（止血 + 应力场 + 回放素材）

- 周期：2026-07-30 23:40 ~ 07-31 01:35
- 定版：`docs/forge-v14-plan.md` v2-final（§0 修订 R1-R6）
- 审计：`gatekeeper-audit-v14.md`（结论：⚠️ 有条件通过）

---

## 一句话总结

毒债修复（sub_budget 不再砍半）真实有效、应力场 24/24 零 panic、31 条回放素材落袋；但主表 77.5% 触红线——其中 4 条是我自己的基准预算配置误差（补测确证、修正口径 87.5%），真正的能力缺口收敛到 T14/T19/T09 三题，如实记账给 v15。

## 线 A：止血（S1-S4）

### v14-1 毒债修复（S1）
- `loop.rs:939`：子代理预算 `max_steps/2`（=7 步饿死）→ `(parent-2).max(8)`。
- `wiring-v13.toml` 第 8 条红线断言 `sub-budget-not-halved`：被改回即 CI 断裂。
- VM 四门全绿：fmt / clippy -D warnings / test --all / xray wiring 8/8。

### 夹具硬化（S2/S3）
- T14：serde_json 预置 dev-dep；T19：强调函数名 `parse_positive`；T15：改动清单化。
- 三题 `fixture_version:2`，v13 原版存 `bench/tasks/_archive_v13/`（可比性声明见审计 §四.2）。
- runner.py 按题预算：`meta.json max_steps` 覆盖全局 BUDGET。

### S4 矩阵（zhipu 20×2）+ S4b 预算补测

| 口径 | 通过率 | 说明 |
|---|---|---|
| 主表原始 | **31/40 = 77.5%** | 触 <85% 红线（字面） |
| 修正口径（补测后） | **35/40 = 87.5%** | 4 条 steps=15 截断项在 20 步下全 PASS → 确证为工装配置误差 D1 |
| 90% 目标 | ❌ 未达 | 缺口 = T14(0/2) + T19(0/2) + T09(FLAKY 50%) |

- **修复有效性实证**：v13 FLAKY 三项中 T13/T18 本轮 20 步下全过；T02（v13 反复翻车）两轮全过。
- **能力边界暴露**：T14 derive 宏、T19 泛型合并对 zhipu(glm-4.5-air) 仍过难——夹具硬化救不了模型能力，v15 考虑 prompt 侧或 provider 侧方案。
- 断点续跑防线首次实战：attempt1 在 T08 被系统信号杀死（rc=0xC000013A），`--resume` 10s 自动续跑，零数据丢失。

## 线 B：应力场（S5/S6，deepseek，ST1-ST8 ×3）

- **核心判据：24/24 零 panic + service 全程存活 ✅**（详见 `bench/results/stress-v14.md`）。
- 亮点实证：
  - ST8 审批门：SSE 捕获 `need_approval` → 60s 无人批 → deny → `rm` 未执行（3/3）。
  - ST4 并发：300 线程 burst → ~200 个 429（P1_MAX_CONCURRENT=50 生效），5 会话全完成。
  - ST2 恶意目标：canary 存活（ConstitutionGuard 拦截）。
  - ST5 磁盘满（1M tmpfs 98%）：优雅 error 无崩溃。
  - ST7 隔离：**零跨会话泄漏 3/3**（2 次 not-ok 是判据把任务完成与隔离绑定，v15 拆分）。

## 线 C：回放素材（S7，如实降级）

- 31 条 PASS session 完整消息历史归档 `bench/replay/fixtures/`（msgs=28~41）。
- **replay.py 未实现**——本轮只有"录制"没有"回放"，防线留 v15，不吹。

## 红线与 tag 裁定

- 审计裁定 ⚠️ 有条件通过：主表触线的 4/9 是测量误差（D1，已补齐全部 L4/L5 预算配置防复发），产品侧核心防线（wiring/应力/审批/隔离）全部实证在位 → **打 tag v14.0**。
- v15 三笔债：① 90% 缺口（T14/T19/T09）；② replay.py 真回放；③ ST7 判据拆分。

## 交付物清单

| 文件 | 内容 |
|---|---|
| `crates/agent-core/src/loop.rs` | v14-1 毒债修复 |
| `docs/xray/wiring-v13.toml` | 第 8 条红线断言 |
| `bench/tasks/{T14,T15,T19}/` + `_archive_v13/` | 夹具 v2 + 原版存档 |
| `bench/tasks/*/meta.json` | L4/L5 全体 max_steps=20（D1 修正） |
| `bench/runner.py` | 按题预算覆盖 |
| `bench/{matrix_v14,stress_v14,archive_replay_v14,chain_v14,s4b_v14}.py` | 矩阵/应力/归档/链驱动/补测工装 |
| `bench/results/raw/{matrix-v14-zhipu,matrix-v14b-zhipu,stress-v14}.jsonl` | 原始数据 70 条 |
| `bench/results/stress-v14.md` | 应力场报告 |
| `bench/replay/fixtures/` (31) | 回放素材 |
| `gatekeeper-audit-v14.md` | 守门审计 |
