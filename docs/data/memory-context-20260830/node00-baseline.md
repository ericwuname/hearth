# Node 00 — Baseline / Provenance（P2-MEMORY-CONTEXT-01）

日期：2026-08-30　总包：v1.1（批-0~8 + 复-1~5 已内联）　执行窗口：砺·执行

## 本机

| 项 | 值 |
|---|---|
| source | `C:\Users\87465\Desktop\codex-rust-v1.0-final` |
| HEAD | `0b6c680`（代码终态 = P2 基线 `ceb62b0` + docs 提交；crates/ 与 ceb62b0 一致） |
| tag / version | v0.2.15 / 0.2.15 |
| working tree | 17 个未提交 docs/release 项（历轮遗留，无 crates/ 改动） |

## .133（评审 VM）

| 项 | 值 |
|---|---|
| source / version | `/home/wutao/codex_t` / 0.2.15 |
| binary | `/usr/local/bin/hearth` = 0.2.15 |
| disk | `118G 108G 5.6G 96%` ⚠️ **磁盘风险项**：gate 需增量编译空间，Node 15 Final Gate 前须监控，必要时清理 target 增量 |
| memory | total 7889MB，available 6624MB |
| baseline gate | `/home/wutao/t_gate_fa_final.log`（v0.2.15，437/0，四 RC=0——代码自该 gate 后零改动，直接作为 P2 基线 gate 记录；Node 12 施工后另跑新 gate） |

## .131（执行 VM）

| 项 | 值 |
|---|---|
| source / version | `/home/wutao/codex` / 0.2.15 |
| binary | `/usr/local/bin/hearth` = 0.2.15 |
| disk / memory | 37G free / available 6698MB |
| provider | Agnes（config.toml，agnes-2.5-flash）；探测延迟 1.9-2.6s（健康） |

## 结论

- 三方版本一致 v0.2.15；上游 FA01 = CLOSED（PASS WITH DEVIATIONS，v0.2.15 `ceb62b0`）。
- 磁盘：.133 5.6G（风险登记）；.131 37G 充足——**A/B 长程实验与真机主战场放 .131**。
- 内存基线已记录（新要求）。健康锁完成，进 Node 01。
