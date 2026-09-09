# v24 维护轮验收报告 — tag + 备份 + 文档 + 首次三端口季度体检

> 日期：2026-08-04 | 基线：`21429e1`（v23 封版）→ 验收后 `fed0c7e`
> 依据：`v24-maintenance-plan.md`（三件事：tag / 说明书 / 季度体检）
> 性质：维护轮——**不进新机制**，只做维护动作 + 季度反哺

---

## 〇、审查补充优化（用户问"有没有补充优化的地方"）

1. **tag v23.0 已提前达成**（上轮封版时已打）——计划第一步实际已完成，本轮核实。
2. **打包备份（用户新增要求）**：`git archive v23.0` → **`Desktop/codex-rust-v23.0-backup.tar.gz`（1.6M）**
   ——只含 git 跟踪文件（全部源码+文档），不含 target/.git，可复现、可备份。
3. **deepseek 20×2 基准烧 token**：遵循预算纪律（MEMORY：测试预算收紧）——本轮体检做
   **零成本项**（Observer mock 全触发 + wiring + replay + 全量回归），**AI OS 基准挂账待预算**
   （与 v22 收官口径一致）。计划 §二.3 的"4h 机器时间"降为"零 token 体检"。
4. **Observer 体检落地为可执行测试**：`test_v24_quarterly_all_rules_trigger`——构造阈值越界
   事件流 → **5 条规则全触发 + 全部 Finding 带 evidence**（维护合同判据可自动化）。
5. **说明书补 v23 段**（MEMORY 已知滞后项）：§二 v23.0 三端口架构轮 + §四基线事实刷新。

## 一、v24 三件事执行

| 事 | 状态 | 证据 |
|---|---|---|
| **tag v23.0** | ✅ 已达成（上轮） | `git tag v23.0` → `5ec25e9` |
| **打包备份** | ✅ 新增完成 | `Desktop/codex-rust-v23.0-backup.tar.gz`（1.6M，git archive） |
| **说明书补 v23 段** | ✅ 完成 | §二 v23.0 段（11 WP + 边界 + 测试状态）+ §四基线刷新 |
| **季度体检** | ✅ 完成（零 token 版） | 见下表 |

## 二、首次三端口季度体检（v24，零 token 预算版）

| 端口 | 体检项 | 判据 | 结果 |
|---|---|---|---|
| **AI OS** | 全量回归 | 239 tests 零回归 | ✅ **240 passed**（+1 v24 新测试） |
| | wiring 接线 | 15/15 全绿（v22 口径） | ✅ wiring 测试全过 |
| | replay 回放 | 31/31（v22 真值） | ✅ replay 测试全过 |
| | 基准 deepseek 20×2 | 90%±5% | 🟡 **挂账待预算**（烧 token） |
| **Observer OS** | 指标确定性（G3） | 两次计算字节级相同 | ✅ test_deterministic_metrics_same_twice |
| | 规则全触发 | 5 条规则全触发 | ✅ **test_v24_quarterly_all_rules_trigger（新增）** |
| | 熔断触发 | sandbox 违规 → CircuitBreak | ✅ test_circuit_sandbox_violation |
| | L2 fail-closed | seq 断档 → Err | ✅ test_l2_fail_closed_on_seq_gap |
| **Human OS** | 前端接真流 | /stream SSE + 信封渲染 | ✅ 上轮 VM 实测（/stream 200） |
| | 交互链路 | 审批 → POST /interaction | ✅ 上轮实测 + cli_contract_test 2/2 |
| | 断线重连 | Last-Event-ID 续传 | ✅ 上轮实测（G4） |

**Observer OS 13 测试全过**（确定性/5 规则/熔断/L2/evidence）——第三权体检首次闭环。

## 三、门禁汇总

| 门禁 | 结果 |
|---|---|
| cargo test --workspace | ✅ **240 passed 零失败** |
| wiring / replay | ✅ 全过 |
| Observer OS | ✅ 13/13（含新增 v24 全触发测试） |
| window-framework | ✅ 208/208（未改动） |

## 四、交付

- commit `fed0c7e`（说明书 v23 段 + Observer 体检测试）
- 备份：`Desktop/codex-rust-v23.0-backup.tar.gz`（1.6M）
- 维护合同更新（plan §三）：Observer OS 季度判据（G3 不重复 / 规则空触发 / 熔断漏报）

## 五、v24 之后（维护期）

- 唯一未决：RT3 seccomp（挂起）/ Q1~Q11 / **deepseek 20×2 基准（待预算批准）**
- 季度体检从三维变六维（AI OS 4 + Observer OS 4 + Human OS 3）——本轮零 token 版已覆盖 10/11 项
- 不再有"加机制"版本
