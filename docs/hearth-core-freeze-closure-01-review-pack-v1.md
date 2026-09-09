# Hearth CORE FREEZE CLOSURE-01 Review Pack v1

> **用途**：单文件核验材料——供砺·评审窗与外部 AI 对 Freeze Closure 做最终独立审查。命令在 `.133:~/codex_t`（gate/锚点）与 `.131`（真机）执行。

## 0. Verdict 预览

**CORE FREEZE**（F0=0；F1×4 ACCEPTED DEVIATION；零 BLOCKER；RC49 重跑闭环）

## 1. Baseline 三查

| 看什么 | 命令 | 预期 |
|---|---|---|
| tag/HEAD | `git describe --tags --always` | v0.2.18-2-g5b0c7b9 |
| 干净树 | `git status --short` | 空 |
| 版本 | `grep -m1 ^version Cargo.toml` + `.131: /usr/local/bin/hearth --version` | 双 0.2.18 |
| 诊断干净 | `.131: strings /usr/local/bin/hearth \| grep -c COMPACT_DBG` | 0 |
| sha256 | `.131: sha256sum /usr/local/bin/hearth` | `9e92dfc9…` |

## 2. RC49 规范链重跑（最高优先级核验）

| 看什么 | 命令（.131） | 预期 |
|---|---|---|
| 执行记录 | `grep -aE "ARCHIVE_BEFORE\|ARCHIVE_MID\|ARCHIVE_AFTER\|RC49_DONE" ~/fa/rc49.log` | BEFORE=120 / MID=120 / AFTER=120+4 files / RC49_DONE |
| 压缩落盘权威信号 | `ls -lat ~/.config/hearth/archive/` | compacted.jsonl 增至 1244350 @13:18 + session 文件 `8335e03c-…jsonl` 17KB |
| 链产物 | `cat /tmp/cfr_rc49/FREEZE_RESULT.txt` | CHAIN_OK |
| 独立复验 | `cargo test --manifest-path /tmp/cfr_rc49/chainfree/Cargo.toml` | 1 passed |
| 链终态 | `grep -aoE "Task (completed\|failed)" ~/fa/rc49.log` | completed ×2 |

## 3. Node 01 gate 复现

| 看什么 | 命令（.133） | 预期 |
|---|---|---|
| gate | `grep _RC= ~/t_gate_closure.log` | 四 RC=0 |
| 计数 | 447 passed / 0 failed | 与 v0.2.18 基线一致 |

## 4. F0 六项（四层复核明细见 closure-audit.md Node 03）

| 看什么 | 命令 | 预期 |
|---|---|---|
| F0-1 假完成 | `cargo test -p agent-core --lib test_rc47` | 1 passed |
| F0-2 静默丢失 | `cargo test -p agent-core --lib inv_m01` | 2 passed（loss-mode fixture 能失败） |
| F0-4 有界 | `grep -n "stall_count" crates/agent-core/src/loop.rs \| head -3` | T4 有界锚点 |
| F0-5 沙箱 | `.131: grep -ac "approval_denied" ~/fa/lr_n12r2.log` | ≥1（拒绝实证） |

## 5. F1 Disposition（四项全 ACCEPTED DEVIATION）

| 看什么 | 命令/文件 | 预期 |
|---|---|---|
| RC48 明细 | closure-audit.md Node 04 + `.131: grep -ac VERIFICATION_RESERVE ~/fa/lr_n12r1.log`（=1）+ `grep -ac GIVE_UP_OVERRIDDEN ~/fa/lr_n12r3.log`（=1） | 对照成立 |
| QA boundary | closure-audit.md Node 05 | duplication 定性成立 |
| Archive C=UNKNOWN | closure-audit.md Node 06 | 不升 PASS/FAIL |
| 40 切片判级 | `cargo test -p agent-core --lib test_history_slice_marker`（1 passed）+ closure-audit.md Node 07 | bounded declared ≠ silent |
| disposition 表 | closure-audit.md Node 12 | 四项 ACCEPTED，零 BLOCKER，零 OPEN |

## 6. freeze-decision.md

| 看什么 | 命令 | 预期 |
|---|---|---|
| 单一结论 | `grep -c "^# \*\*CORE FREEZE\*\*" docs/core-freeze-review/freeze-decision.md` | 1（唯一结论） |
| does-not-guarantee | `grep -n "does NOT guarantee" -A 6 docs/core-freeze-review/freeze-decision.md` | 含 archive C + 切片 + RC48 + variance |

## 7. No-Code Gate（Node 14）

| 看什么 | 命令 | 预期 |
|---|---|---|
| 生产代码零改动 | 本机 `git status --short`（crates/） | 空 |
| 版本保持 | `grep -m1 ^version Cargo.toml` | 0.2.18（无人为 bump） |
| 最终 gate | `.133: grep _RC= ~/t_gate_closure.log` | 四 RC=0，447/0 |

## 8. 防伪声明

本包所有结论可按上述清单逐条复跑；RC49 重跑以 archive 落盘（非日志 grep）为权威信号（批-1/S-1 纪律）；诊断构建矛盾以 strings 实测裁决（批-2/S-2）；false stop 2+12 构成明细落表（批-3）；四项 disposition 不以 OPEN 交付（批-8 认可的止损纪律）。
