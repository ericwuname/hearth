# Node 00 — Provenance Freeze Audit（CORE FREEZE REVIEW-01）

日期：2026-08-31　执行窗口：砺·执行

## 三查结果（双 VM）

| VM | 查项 | 对齐前 | 对齐后 |
|---|---|---|---|
| .133 | 系统 PATH binary | **0.2.16（真滞后，守门员复-1 + 本窗复核确认）** | **0.2.17 ✓**（重建 48s + sudo cp 安装） |
| .133 | source Cargo.toml | 0.2.17 | 0.2.17 ✓ |
| .133 | gate 编译版本 | 0.2.17（`t_gate_lr_final.log` 447/0） | 0.2.17 ✓ |
| .131 | 系统 PATH binary | 0.2.17 ✓ | 0.2.17 ✓ |
| .131 | source Cargo.toml | 0.2.17 | 0.2.17 ✓ |

## 异常定性（批-2）

整合报告 §7 的 ".133 binary=0.2.16" **= 真滞后（情况 b），非笔误**——P2-LR 收尾时 .133 只跑了 gate（编译 0.2.17 产物在 target），系统 PATH binary 未同步安装（P2-MC 轮曾装 0.2.16）。v0.2.3 PATH 分叉家族复发形态。

**处置**：.133 重建 0.2.17 + 安装系统 PATH + 最小冒烟 `cargo test -p agent-core --lib test_rc47` = **1 passed**（证据链活着）。**LR 真机证据链不受影响**（真机在 .131，binary=0.2.17）——守门员预判成立，未重跑真机。

## 资源与树

- .133：磁盘 44%（64G free）、内存 6.3G avail；`~/codex_t/target` = 真实目录（P2-MC 事故已闭环，无共享 symlink）。
- .131：磁盘 69%（36G free）；`~/codex/target` = 真实目录。
- df 保险丝（<10G 中止）登记。

## 版本记账

- gate 增量记账基线 = **447**（v0.2.17，`t_gate_lr_final.log`）。
- 本轮零代码改动则 gate 复跑 447/0 即可（S-4）；若有 RC48 修复 → v0.2.18 bump 先于 gate。

## 结论

三方一致（source=binary=gate=0.2.17）双 VM 达成；provenance 证据链完整；STOP-6 不触发。
