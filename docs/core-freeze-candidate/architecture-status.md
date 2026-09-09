# FREEZE CANDIDATE · architecture-status（先行件）

> **Node 15 八文件之一（先行起草）**｜执行窗 2026-09-02｜基线 tag `v0.2.23`（commit `117a947`）
> **状态**：DRAFT——Step 2 campaign / Step 3 正式盲测数据回填后定版
> **证据规则**：全部结论锚定源码或可复跑命令；"自述已验证"不计证据。

## 1. 规模与结构

- crates = **27**（2026-09-01 十轮深挖实测，纠正旧记录 23）；tests = **486/0**（v0.2.23 门禁拆分口径：agent-core 130 + workspace --exclude agent-core 356/0/59）。
- 生产热区 `agent-core/src/loop.rs` 约 5070 行，1 unwrap / 0 unsafe——技术债是"没拆"（D3），不抢排期。
- 三端口架构维持：Human OS / AI OS（唯一事实源）/ Observer OS（第三权零执行权），crate 隔离可审计。

## 2. v0.2.21→v0.2.23 收口内容（代码级）

| 项 | 内容 | 冻结区程序 |
|---|---|---|
| RC52 三臂统一 | `rc52_route_done_if_session_artifacts()` 接线 Reflect GiveUp / T4 stall / budget exhausted 三出口；`completion_fact_check()` 前移防无关产物误判完成 | FZ-RFC-2b（双签待追认，见 open-deviations） |
| S1 原子写 | `tools-builtin/src/atomic.rs` 同目录 tmp+rename；edit/patch 收口 | 冻结区外 |
| S2 改前快照 | `codex-cli/src/snapshot_store.rs` manifest 绝对路径 + `hearth rollback` | 冻结区外 |
| S3 turn checkpoint | `on_turn_checkpoint` 回调 + CLI 三构造点接线 | 冻结区外（触 loop.rs 调用点，FZ-RFC-4 备案） |
| S4 env_clear | 生产 LinuxSandbox `env_clear()` + 最小白名单（FZ-RFC-1） | **冻结区，双签待追认** |
| C-1/C-12 | `verification: VERIFIED/UNVERIFIED` + 投影「目标达成（未验证）」 | FZ-RFC-3 |
| C-2 | `timeout_drift_ratio` drift warn（观测级） | 冻结区外 |
| C-3 | 宪法 FALLBACK 告警 | 冻结区外 |
| 推理投影 | reflect 决策依据 + reasoning_content 投影（HEARTH_SHOW_REASONING=0 可关） | 冻结区外 |
| bash 修复 | extract_hosts 位置感知 + FILE_EXTS 排除表（误报连坐修复） | 冻结区外 |

## 3. 地基三档（对标裁决留存）

- 承重 🟢B+：状态机/九态/F1-F10 六策略/CI 五门禁。
- 崩溃安全 🔴→🟡：S1/S2/S3 落地后，"超长写先毁原文件 / kill 丢整轮 / 零快照回滚"三空已补；workspace 全量测试挂起（RT4-OOM 探针）仍开放。
- 对抗安全 🟡：S4 env_clear + egress 白名单收口后，剩余偏差见 open-deviations。

## 4. SimUser 测试基建（冻结区外，本窗新增）

`tools/simuser/`：相关矩阵（`corr-rules-v1.1`，n=273 种子，V 矩阵存档）+ 联合分布采样器（分层采样，逐字语料）+ persona ×12 / scenario ×10（G1-PENDING）+ 检测器 16（13+3，COND-2 口径）+ B4 验收 12 PASS/0 FAIL/2 MANUAL。G1 评审材料已送砺（`docs/core-revalidation/G1-相关矩阵评审材料（执行窗→砺）.md`）。
