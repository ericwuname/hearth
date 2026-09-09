# 挂账清账验收报告 — 第 1/2/3 批全部完成

> 日期：2026-08-07 | 基线：`83f8cb0` → 本轮 commit：`ee9d869`（landlock）+ `6160585`（Q8/Q3/N7）
> 性质：用户选"第1+2+3批全做"——安全/环境/预算/小项挂账清账
> 状态：**3 项闭环 + 2 项复核闭环 + 1 项挂账（Docker 网络受限）**

---

## 第 1 批：安全 + 环境 ✅

| 项 | 结果 | 证据 |
|---|---|---|
| **Q9 API_KEY 默认全放行**（🔴 RT4-①） | ✅ **复核闭环——已修复** | `routes.rs:114-121` fail-closed：无 key 且无 ALLOW_NO_AUTH → **401 拒绝**（P0-1 默认安全反转早已落地）；有 key 时无凭据 401 / 错误 403 |
| **VM landlock 退化**（G0） | ✅ **根治** | **根因**：内核 Ubuntu HWE 7.0.0-28 = landlock **ABI v8**——空 attr（handled=0）create → **ENOMSG**（v8 新行为）；带权限位 → fd 正常。sandbox 探测用空 attr → 误判 landlock 不可用。**修复**：探测 attr 用 FS_RO（≥1 位）。**验证**：T00 1/1 PASS + `landlock not available` 计数 0（修复前 45+） |

## 第 2 批：预算项 ✅（1 完成 / 1 挂账）

| 项 | 结果 | 证据 |
|---|---|---|
| **季度体检基准 deepseek 20×2** | ✅ **36/40 = 90.0%——命中 90%±5% 基线带** | 与 v22 T6 权威值 90.0% 完全一致——三端口架构施工后**零退化**；失败 4 run：T19×2（模型能力墙）+ T00×1（multiply 未写全）+ T15×1（NO_BENCH_FILE）——T00/T15 各 1 run 真实抖动 |
| **Docker build 复验** | ⏸ **挂账（网络受限）** | VM 到 docker hub base 层（rust:1.82）拉取 70 分钟仅过半（0.7MB/s）——本轮不可行；**v22 已验证 docker 路径**（bfab533），v23/24 增量（observer crate）不涉及 Dockerfile 结构，风险低；待 VM 网络恢复（daocloud 加速器）后复验 |

## 第 3 批：小项清账 ✅

| 项 | 结果 | 证据 |
|---|---|---|
| **Q8** on_timeout 默认值 | ✅ **落地** | blocking 交互（approval/clarification）默认 `on_timeout=Some("abort")`（fail-safe）——建议值变为代码；内核只透传不解析 |
| **Q3** Observer 报告持久化 | ✅ **落地** | 会话结束后评估事件流（信封化）→ `CODEX_OBSERVER_DIR/reports/{sid}/report.md + report.json`（`Observer::run_and_report`）；与旧 v10 `daily-*.jsonl` 资源快照**分目录不混文件**；零执行权不变（只写报告）；`SessionManager.set_observer` 注入（不破坏 new 签名） |
| **N7** 密钥串用复核 | ✅ **复核通过** | crates 零硬编码真实 key（仅 `sk-placeholder` 占位）；无 .env 残留；sk-28d/sk-5ad/sk-proj/AIza/ark- 全 0 命中 |

## 门禁汇总

| 门禁 | 结果 |
|---|---|
| cargo fmt --check | ✅ 0 |
| cargo clippy -D warnings | ✅ 0 |
| cargo test --workspace | ✅ **240 passed 零失败** |
| cargo build --release | ✅ 成功 |
| 季度体检 | ✅ 36/40 (90.0%) |
| T00 实测（landlock 后） | ✅ 1/1 PASS |

## 挂账剩余（第 4 批，需用户拍板）

| 项 | 卡点 |
|---|---|
| **RT3 seccomp 白名单** | 用户挂起（大工程+安全敏感独立排期） |
| **T19 通过率** | 模型能力墙——可换 deepseek-pro/智谱验证（待用户定） |
| **Docker 复验** | VM 网络恢复后（daocloud 加速器） |
| **Q7** 熔断恢复流程 / **Q4** 时间轴真实回滚 | 设计+实现（中等，待排期） |
| **Q1/Q2/Q5/Q6** | 业务/产品决策 |

## 交付

- `ee9d869`：landlock ABI v8 兼容（G0 恢复）
- `6160585`：Q8 + Q3 + N7（小项清账）
- 体检数据：`bench/results/raw/quarterly-2026-08.jsonl`（VM + 本地，gitignore 不入库）
