# Review Index — CORE FREEZE REVIEW-01

> 独立评审入口：按序核验，每项附命令 + 预期输出。命令在 `.133:~/codex_t`（gate/锚点）与 `.131`（真机日志）执行。
> 纪律：任何一项与预期不符 = 打回；全部通过 = 支持 Final Report 的 Freeze 建议。

## 0. 环境预检

| 看什么 | 命令 | 预期 |
|---|---|---|
| 版本三方一致 | `.133`: `/usr/local/bin/hearth --version`；`grep -m1 ^version ~/codex_t/Cargo.toml` | 双双 0.2.18 |
| gate 终态 | `.133`: `grep _RC= ~/t_gate_cfr_final.log` | 四 RC=0 |
| 磁盘 | `df -h /`（双 VM） | >10G free |

## 1. Authority Matrix（文件：authority-matrix.md）

| 看什么 | 命令 | 预期 |
|---|---|---|
| give_up 决策权/否决权分离 | `grep -n "GIVE_UP_ROUTED_TO_DONE\|GIVE_UP_OVERRIDDEN\|GIVE_UP_INTERCEPTED" crates/agent-core/src/loop.rs` | 三路拦截锚点全在 |
| RC47 路由 | `grep -n "criteria 空 + 已实质完成" crates/agent-core/src/loop.rs` | 命中（仅 criteria 空形态） |
| verification 唯一生产者 | `grep -n "fn verify_acceptance_criteria" crates/agent-core/src/loop.rs` | 唯一定义 |

## 2. Decision-Terminal Map（文件：decision-terminal-map.md）

| 看什么 | 命令 | 预期 |
|---|---|---|
| RC47 正反测试 | `cargo test -p agent-core --lib test_rc47` | 1 passed |
| QA 反例（无产物 failed 保留） | `cargo test -p agent-core --lib test_node05_negative` | 1 passed |

## 3. False Stop / RC48（文件：false-stop-lifecycle.md）

| 看什么 | 命令 | 预期 |
|---|---|---|
| n12r1 拦截证据 | `.131`: `grep -ac VERIFICATION_RESERVE ~/fa/lr_n12r1.log` | 1 |
| n12r3 反证（有 Reserve 时全链路正确） | `.131`: `grep -ac GIVE_UP_OVERRIDDEN ~/fa/lr_n12r3.log` + `cargo test --manifest-path /tmp/lr_n12_r3/sortlib/Cargo.toml` | 1 + 1 passed |
| 复现跑可比性 | 三跑同任务同 budget（criteria-frozen.md） | 层归表见 false-stop-lifecycle.md |

## 4. History Slice（文件：history-slice-lifecycle.md）

| 看什么 | 命令 | 预期 |
|---|---|---|
| 标记存在 | `grep -n "history-slice-note" crates/agent-core/src/loop.rs` | 命中 |
| marker 测试 | `cargo test -p agent-core --lib test_history_slice_marker` | 1 passed |
| lost-to-LLM 判级 | 读 history-slice-lifecycle.md 对照表 | 零恢复通道成立 |

## 5. Archive Recoverability（文件：memory-lifecycle.md）

| 看什么 | 命令 | 预期 |
|---|---|---|
| 阈值优先级修正 | `grep -n "env 测试仪器必须压过注入值" crates/agent-core/src/context.rs` | 命中 |
| 阈值矩阵测试 | `cargo test -p agent-core --lib test_compact_threshold_env_override` | 1 passed |
| C-probe 三版迭代记录 | 读 memory-lifecycle.md C-probe 表 | C 未证明（诚实判级）+ 防污染设计要点 |

## 6. Fact Survival（文件：fact-lifecycle.md）

| 看什么 | 命令 | 预期 |
|---|---|---|
| INV-M01 双 fixture | `cargo test -p agent-core --lib inv_m01` | 2 passed |
| state 层 preserved 断言 | `grep -n "original_goal preserved" crates/agent-core/src/context.rs` | 命中 |

## 7. Long-run Evidence（文件：long-run-evidence.md）

| 看什么 | 命令（.131） | 预期 |
|---|---|---|
| Node 08 单跑链 | `cargo test --manifest-path /tmp/cfr_n08/chainfree/Cargo.toml` + `cat /tmp/cfr_n08/FREEZE_RESULT.txt` | 1 passed + CHAIN_OK |
| Node 09 双产物 | `cargo test --manifest-path /tmp/cfr_n09/todoapi/Cargo.toml` + `/tmp/cfr_n09b/units/...` | 1 passed + 3 passed |
| Node 10 双拓扑 | `/tmp/cfr_n10a/configlib`（3 passed）+ `cat /tmp/cfr_n10b/legacy_app/FINAL.txt` | 3 passed + ADAPTED_OK |
| Node 11 QA 产物 | `cat /tmp/cfr_n11/notes/todo.txt /tmp/cfr_n11/notes/done.txt` | buy milk + really all set |

## 8. Exit Code 五态（P2-LR 承接）

| 看什么 | 命令 | 预期 |
|---|---|---|
| 类型化载体 | `grep -n "pub struct BashExitError" crates/tool-runtime/src/dispatcher.rs` | 命中 |
| 五态矩阵测试 | `cargo test -p agent-core --lib test_five_state` + `test_bash_exit_code` | 双 passed |

## 9. gate 与版本

| 看什么 | 命令（.133） | 预期 |
|---|---|---|
| Final Gate | `bash ~/run_gate_r2c.sh` | 四 RC=0，447/0（v0.2.18） |
| tag | `git tag`（本机） | 含 v0.2.17/v0.2.18 |

## 已知限制（委托方需知，非脚注）

1. **archive C 未证明**——A+B 已证，C 两版探针未达成防污染前提；委托方需知：archive 是"已保存"而非"模型可自动找回"。
2. **40 切片零恢复通道**——被裁事实对 LLM 永久不可见（state 存活）；Task Continuity 投影缓解关键事实。
3. **QA 轮 T4 stall 高频**（n11 12/16）——已完成"继续"形态的 model+decision 已知限制。
4. **false stop（RC48）**——Reserve 零和 × GiveUp 时点；有界，修复待顶层批准。
