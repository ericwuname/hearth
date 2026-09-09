# Node 00 — Environment / Provenance Lock（P2-LONG-RUN-ACCEPTANCE-01）

日期：2026-08-31　总包：v1.1（砺批-0~8 + 复-1~4 已内联）　执行窗口：砺·执行

## 版本与对齐（S-1 落实）

| VM | source | source version | binary（对齐前） | binary（对齐后） | 磁盘 |
|---|---|---|---|---|---|
| .133（评审） | `/home/wutao/codex_t` | 0.2.16 | **0.2.15（脱节，S-1 实测）** | **0.2.16 ✓**（重建+安装，build 1m56s） | 71G free（38%） |
| .131（执行） | `/home/wutao/codex` | 0.2.16 | 0.2.16 ✓ | 0.2.16 ✓ | 36G free（69%） |

内存：两 VM available ≈ 6.7G。df 保险丝（<10G 中止）登记。

## target symlink 检查（总包 §5 要求）

- .133：`~/codex_t/target` 曾是指向 `~/codex/target` 的 symlink——P2-MC 期间已清理（76G 弃用树构建产物删除）并重建为**真实目录**（P2 Final Report §18-1）；本轮 `ls -la` 确认为目录，无共享 target 污染。
- .131：`~/codex/target` 为真实目录（独立缓存）。

## .git 重建口径声明（砺批-7 / v1.1 必办①）

> **历史连续性以 CHANGELOG + docs 为准；git 历史自 `463b315` 起可信**（P2-MC 期间本地 .git 损坏重建）。tag v0.2.16 在库，HEAD `40f607c` 链一致（砺窗已核，本窗复核 tag → 463b315 祖先关系成立）。

## 源码备份（v1.1 必办③）

`/home/wutao/hearth-v0.2.16-backup-20260831.tar.gz`（2.1MB，crates/docs/Cargo.toml，防 .git 损坏类事故再丢历史）。

## gate 基线

`.133:~/t_gate_mc_final.log` = **443/0 四 RC=0**（v0.2.16）——登记为本轮 Final Report **增量记账基线**。

## 结论

- 双 VM 二进制 0.2.16 对齐完成（S-1 关闭）；环境无异常，真机实验可开跑。
- 环境红线：每实验前后 df 检查（§25）；异常结果标 invalid。
