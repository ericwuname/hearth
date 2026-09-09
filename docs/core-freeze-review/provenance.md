# Provenance — CORE FREEZE REVIEW-01

## 三查（Node 00 实修后终态）

| VM | source | binary（系统 PATH） | gate |
|---|---|---|---|
| .133 | `/home/wutao/codex_t` = **0.2.18** | **0.2.18**（0.2.16 滞后 → 0.2.17 重建 → 0.2.18 重建，三查一致） | `~/t_gate_cfr_final.log` **447/0 四 RC=0**（v0.2.18） |
| .131 | `/home/wutao/codex` = **0.2.18** | **0.2.18**（含 COMPACT_DBG 诊断 eprintln——Final Report 披露，正式版需移除） | 真机日志 `~/fa/cfr_*.log` |

## 版本链

v0.2.16（P2-MC）→ v0.2.17（P2-LR）→ **v0.2.18**（P2-CFR：阈值优先级修正）。
CHANGELOG：v0.2.16/17 独立文件 + v0.2.18 附于 CHANGELOG-v0.2.17.md。
git：tag v0.2.17 → v0.2.18（gate 后打）；历史连续性以 CHANGELOG + docs 为准（git 历史自 463b315 起可信——P2-MC 事故声明延续）。

## 资源

.133：磁盘 44%（64G）/ .131：磁盘 69%（36G）——df 保险丝全程未触发。
target：两 VM 均真实目录（无共享 symlink 污染）。
备份：`/home/wutao/hearth-v0.2.16-backup-20260831.tar.gz`。

## 诚实披露

- .131 binary 含本轮诊断 eprintln（COMPACT_DBG）——仅 stderr 输出，无行为影响；正式发布前需一次无诊断重建。
- .133 provenance 异常（0.2.16 滞后）在本轮 Node 00 实修关闭（重建+安装+冒烟 test_rc47 passed）。
