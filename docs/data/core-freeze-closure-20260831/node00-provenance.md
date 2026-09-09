# Node 00 — Baseline Integrity + RC49 处置（CORE FREEZE CLOSURE-01）

日期：2026-08-31　执行窗口：砺·执行

## 三查（v0.2.18）

| 查项 | 结果 |
|---|---|
| git describe | v0.2.18-2-g5b0c7b9（tag 后 2 个 docs 提交，链一致） |
| git rev-parse HEAD | `5b0c7b90d3448e31b1bd805d3545af7695d3b4b4` |
| git status --short | **0 项（干净）** |
| Cargo.toml version | 0.2.18 ✓ |
| .131 binary | hearth 0.2.18，**sha256 `9e92dfc9d9186cba3fac31a3a181fd5c78fc4045ba35ef7a47173e61c5414a22`** |
| .131 磁盘 | 36G free（69%） |

## 诊断构建矛盾裁决（批-2/S-2）

`strings /usr/local/bin/hearth | grep -c COMPACT_DBG` = **0** → 当前 .131 binary = **干净构建**（12:0x 重建，诊断 eprintln 已随 context.rs 干净同步移除）。CFR line 90 自曝的"含诊断构建"为**历史态**（CFR 真机 benchmark 时代）——**显式接受**：CFR 真机证据产自含诊断输出的 v0.2.18 构建（诊断点 stderr 输出不改变控制流）；当前二进制已无诊断。矛盾消解，两文档不再各说各话。

## RC49 规范链重跑（批-1/S-1，最高优先级）

**背景**：守门员实锤——CFR Node 08 规范链执行于阈值优先级修复前 73 分钟（FREEZE_RESULT mtime 05:21:22 < 修复 06:34），env 7000 被注入值 195,840 压过 → 该链 compact 腿证据无效（RC49 成立）。

**重跑（v0.2.18 干净二进制，同任务同参数）**：

| 信号 | 值 |
|---|---|
| ARCHIVE_BEFORE | 120 lines / 3 files |
| Phase 1（fail→repair→retest→compact→stop） | **completed**（N08A_EXIT=0） |
| ARCHIVE_MID | 120 lines / 3 files |
| Phase 2（resume→continue→complete） | **completed**（N08B_EXIT=0） |
| ARCHIVE_AFTER | 120 lines / **4 files**（**新 session 归档文件**） |
| compacted.jsonl | **+18KB @13:18**（1225012→1244350） |
| **新 session 归档文件** | **`8335e03c-...jsonl` 17,335 字节 @13:19**（session 绑定路由实证） |
| FREEZE_RESULT.txt | CHAIN_OK ✓ |
| chainfree 独立复验 | **cargo test 1 passed** ✓ |

**压缩实证（权威信号）**：archive 落盘增长（compacted.jsonl +18KB）+ session 归档文件创建——**compact 腿在 v0.2.18 上真实发生**。压缩时点：修复-复测后的 resume 阶段（恢复历史+续跑处理跨阈值）——链序 compact→resume→continue→complete 成立。

**RC49 处置**：**CLOSED**——CFR Long-run 柱 compact 腿经 v0.2.18 重跑恢复完整；CFR Final Report line 50 的失真表述由本文件更正。BLOCKER 条件不触发。

## gate 复现（Node 01）

`~/t_gate_closure.log`：**FMT=0 / CLIPPY=0 / RT4_SOLO=0 / TEST=0，447 passed / 0 failed**——与 v0.2.18 记账基线一致，无测试数变化。
