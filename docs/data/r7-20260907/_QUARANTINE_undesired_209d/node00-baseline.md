# Node 00 — Baseline / Provenance / Health Lock（P1-FAILURE-ADAPTATION-01）

日期：2026-08-30　总包：v1.1　执行窗口：砺·执行（本会话）

## 本机（host）

| 项 | 值 |
|---|---|
| source | `C:\Users\87465\Desktop\codex-rust-v1.0-final` |
| HEAD | `e0703d429fe9335467774491bc1478256a365ca9`（Final Report 提交；代码终态 `2fe0688`） |
| tag | v0.2.14（最新） |
| Cargo.toml version | 0.2.14 |
| working tree | 历轮未跟踪 docs/release 文件若干（不影响源码树）；源码 crates/ 无未提交改动 |

## .133（评审 VM，gate 主战场）

| 项 | 值 |
|---|---|
| source path | `/home/wutao/codex_t`（tar 同步树，**无 .git**——以 Cargo.toml version 为准） |
| version | 0.2.14 |
| binary | `/usr/local/bin/hearth` = hearth 0.2.14；`bash -lc hearth --version` = 0.2.14（PATH 分叉已消除） |
| disk `df -h /home` | `/dev/sda2 118G 102G 12G 91%`（12G 余量，够增量 gate） |
| baseline gate | `bash ~/run_gate_r2c.sh` → `/home/wutao/t_gate_fa_baseline.log` |
| gate 结果 | FMT_CHECK_RC=0 / CLIPPY_RC=0 / RT4_SOLO_RC=0 / TEST_RC=0；63 条 "test result: ok"，**429 passed / 0 failed / 0 ignored**（RT4 单测在 rt4_solo.log 单独跑，属预期） |

## .131（执行 VM）

| 项 | 值 |
|---|---|
| source path | `/home/wutao/codex`（tar 同步树，**无 .git**） |
| version | 0.2.14 |
| binary | `/usr/local/bin/hearth` = 0.2.14；`bash -lc` = 0.2.14 |
| disk `df -h /home` | `/dev/sda2 118G 77G 37G 68%` |

## 结论

- 双 VM + 本机三方版本一致 = **v0.2.14 / 基线 429 tests 全绿**。
- 磁盘余量充足（.133 12G / .131 37G）。
- 分窗登记沿用 `vm-version-sync.md`：.133 = 评审/gate；.131 = 执行真机。未修改历史口径。
- 健康锁完成，开始 Node 01。
