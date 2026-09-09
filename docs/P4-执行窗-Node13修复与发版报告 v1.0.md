# P4 执行窗 Node 13 修复与发版报告 v1.0

**窗口**：执行窗（Node 00-09 施工者续任）｜**日期**：2026-09-01｜**基线**：v0.2.20 → **本次发版**：v0.2.21
**触发**：测试窗《P4测试窗-汇总报告 v1.0》判读移交（RC52 fresh 40%/污染 100%，decision 层 RC47 族主因 + 污染放大器，CAUSE LIKELY 待升格）
**纪律声明**：本窗只做机制级修复与仪器修复，**未改动任何 benchmark 场景/判定脚本**；α/β/γ 终审不归本窗；先红后绿 + 独立复跑全程执行。

---

## 1. 修复批总览

| # | 项 | 性质 | 状态 |
|---|---|---|---|
| N13-1 | RC52 跨 run 产物事实回填（机制级修复） | 代码 | ✅ 先红后绿，独立复跑绿 |
| N13-2 | R-1 仪器修复（simuser session 提取） | 测试仪器 | ✅ 双落点 md5 留痕，**C 条件仍 BLOCKED 待砺复核** |
| N13-3 | 静默截断标记（事实投影侧 4 加 + 2 豁免） | 代码 | ✅ 含公共 helper + 单测 |
| N13-4 | v0.2.21 发版（三门 gate + 部署 + tag） | 流程 | ✅（详见 §5，含两处如实披露） |

## 2. N13-1：RC52 机制级修复

### 2.1 根因链（证据核验 5/5 通过，Task #457）

测试窗 A1 场景真机证据（.131 `~/fa/p4/p4_A1.log` 42/77/80/82 行）：

```
轮 A：写 p4_smoke.txt → verify → completed          ✅ 事实产生
轮 B：「继续」→ 无事可做 → give_up → failed          ❌ 假阴性
resume：🎯 任务目标已恢复…(revision 1) → completed   （结构性绕过，非同一轮）
```

**机制**：`loop.rs` run() 重置块 `self.written_files.clear()`（~4499 行区）在 REPL 每轮新 run 时**销毁跨轮产物事实** → 新轮 give_up 时 RC47 路由条件（`fa01_criteria_empty && !written_files.is_empty() && consecutive_errors == 0`）不命中 → Error → 假阴性 failed。这是「压缩=信息销毁」家族的兄弟形态：**run 边界 = 完成事实销毁**。

### 2.2 最小 diff（give_up 消费端扩展）

- 新增 session 级字段 `session_written_files: Vec<WrittenFile>`（loop.rs ~951）：跨 run 不清空；写盘时同路径去重留最新（~3513）；上限 64 条（防无界增长）。
- give_up 臂 RC47 路由前回填（~4299），gate = **本轮零写盘 + 零错误**：
  ```rust
  if self.written_files.is_empty()
      && !self.session_written_files.is_empty()
      && self.consecutive_errors == 0
  { /* tracing::warn! RC52_ARTIFACT_HYDRATION */ self.written_files = self.session_written_files.clone(); }
  ```
- **安全网不变**：回填只恢复「路由资格」，产物存活性仍由 Done 相位盲区C 确定性校验裁决（文件被删/为空照常打回），INV-LR03 不变。
- **不做**：不动 planner schema（STOP-1/2 防线维持）；不引入 authority duplication；T4 stall 臂不改（其已有同款拦截，criteria 空落 Err 是正确有界停止）。

### 2.3 先红后绿

| 阶段 | 证据 |
|---|---|
| 红 | fixture 真红：`give_up: cannot make progress (no acceptance criteria — giveup unverified)`——与真机 A1 终态**逐字一致**（plumbing 先行、消费端后落，保证编译过的真红） |
| 绿 | `test_rc52_cross_turn_artifact_hydration_on_giveup`（session 记录 + written_files 空 + GiveUp → 断言 `LoopPhase::Done`）；agent-core lib **126/126 独立复跑绿** |
| 负向门 | `test_rc52_hydration_gated_on_zero_errors`：注入真实 bash error ToolResult → 断言 `LoopPhase::Error(_)`（有错不回填） |

**过程教训（如实登记）**：负向测试不能直接预设 `consecutive_errors = 1`——do_reflect 入口（~4013-4014）对零错误轮会重置为 0，必须注入真实 error 结果让累加逻辑自然生效。首版假绿已翻红修正。

## 3. N13-2：R-1 仪器修复（Node03 指令 v1.1 附录 A 授权内）

- **旧仪器缺陷**：`hearth sessions` 终端正则 `reports/<36>/` 在 8 位短 id 下恒 None → C 条件误记 timeout 口径（DRIVER-INDUCED）。
- **修法**：`tools/simuser/run_rc52_matrix.py` 改 **sessions 目录文件系统 diff**（`~/.config/hearth/sessions/<uuid>.jsonl` chat 前后 mtime 快照，新增/变化即会话 uuid）+ 旧正则 fallback + **session id 非空断言**（空即 fail，禁记 timeout）。
- 双落点 md5 一致：`0002871267aefa1effc7d3a1e1af74b7`（`~/codex/tools/simuser/` + `~/fa/simuser/`）。
- **状态**：C 条件重跑仍 **BLOCKED**——待砺·评审复核仪器 diff 后方可执行（判读分离）。

## 4. N13-3：静默截断标记（4 加 2 豁免）

**裁决原则**：事实产生/投影侧截断必须显式标记（信息销毁可见）；内部构造与纯显示豁免。

| 处 | 位置 | 裁决 |
|---|---|---|
| 1 | constitution.rs sanitize（宪法注入 LLM 上下文） | **加标**——静默砍尾可能吞掉第八条（求真）等后置条款 |
| 2 | loop.rs goal_drift 检测输入（final_statement→2000） | **加标**——事实投影 |
| 3 | loop.rs failed_nodes 摘要（output→80） | **加标**——事实投影 |
| 4 | codex-cli/client.rs 诊断错误消息（→160） | **加标**——事实投影 |
| 豁免1 | loop.rs:3781 检索 query 内部构造 | 豁免——内部 query，非投影 |
| 豁免2 | codex-cli/lib.rs:679 🎯 显示摘要 | 豁免——纯 UI |

公共 helper：agent-types `truncate_marked(s, max)`（chars 计数、超限追加 `\n[... N chars truncated]`，与既有分块标记同格式）+ 单测（未超限不加标/超限标数/CJK）。

## 5. N13-4：v0.2.21 发版记录

**两处如实披露（本窗遗漏，已修）**：

1. **fmt 存量漂移 17 文件**：GATE1 `cargo fmt --check` 报红，漂移覆盖本批 5 文件之外的历史手改存量（scheduler/terminal/config/repl/report/run_local/planner/tool-runtime/tools-builtin 等，E7 家族形态）。处置：.131 `cargo fmt` 统一 → **17 文件回传本机**，双树 md5 逐一对齐（抽查 3 文件 hash 与 fmt 变更清单吻合）。根因（历次窗口改码后未跑 fmt）挂账 §7。
2. **`test_sanitize_caps_length` 漏更**：#462 加截断标记后正文封顶+尾注使总长 > MAX，旧断言 `总长 == MAX` 未同步更新（前次「126/126 绿」为编辑前状态口径，本窗不掩饰）。处置：测试更新为新契约（正文封顶 MAX + 显式标记 500 chars + 总长 = MAX + 标记长），单测绿后全量重跑。

3. **`.131` 树 docs 资产 CRLF 污染（测试锁失效）**：GATE3 在 `project-xray` 集成测试红——`wiring spec hash changed actual=0x8426d94077542ad0`。双树实测：本机 `docs/xray/wiring-v13.toml`=9713B / FNV `0x0444c6bfbbbb0ea2`（**正是锁值**），.131 副本 9953B（+240B ≈ 240 行）/ FNV `0x8426d940…` → **行尾污染（CRLF）**，且全树扫描确认为**系统性**：`docs/` 下 .md/.toml 普遍 CRLF，`.rs` 源码全 LF 干净（故 fmt/clippy 不受影响，只有部分测试锁与资产读取受影响）。处置：以本机为准二进制回灌（9713B / 0 CRLF / FNV 复锁值）→ xray 2/2 绿。根因挂账 §7（同步管线走了文本模式）。

**发版序列**：Cargo.toml bump `0.2.21`（版本注释前置登记 Node 13 内容）→ .131 同步（md5 `3d8e4e96…` 双侧一致）→ 三门 gate → release build → 部署 → `hearth --version` 双 VM 复查 → tag。

| Gate | 结果 |
|---|---|
| `cargo fmt --check` | ✅ FMT_OK（统一后） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ CLIPPY_OK |
| `cargo test --workspace`（模板：unset HEARTH_URL + HEARTH_CGROUP_BASE + HEARTH_ALLOW_NO_CGROUP） | ✅ 全绿（明细见附录 A） |

## 6. E7 家族复发实锤（第三次）

- .131 仓库树 `~/codex/tools/` **整目录缺失**（同步管线缺漏）→ `mkdir -p` 补建 + simuser 工具补传，双落点 md5 留痕。
- 根因挂账 v0.2.22/23：同步管线（tar 排除清单/sftp 覆盖/touch 三步）需把 `tools/` 纳入强制清单并加后验校验。
- 另发现：本机改动需谨慎覆盖 VM 树时，**SFTP 不展开 `~`**（paramiko 落点必须绝对路径）——已记入操作规程。
- **新变体（本次首发）**：**docs 资产 CRLF 污染**——同步管线某环节走文本模式，把 `docs/` 下 .md/.toml 批量转 CRLF（.rs 全 LF 干净，故 fmt/clippy 无感）。后果不是"文档排版不好看"，而是**以文件字节为输入的测试锁必然失配**（`wiring-v13.toml` FNV 锁即因此红）。判别法：双树 `stat` 字节差 ≈ 行数 → 即行尾污染。补法：二进制回灌 + 复算哈希对锁值。

## 7. 挂账与移交

| 项 | 去向 |
|---|---|
| α/β/γ 终审（fresh 失败率判读） | 顶层/外部 AI（判读分离，本窗不预支结论） |
| C 条件重跑 | 砺·评审复核 R-1 仪器 diff 后解 BLOCKED |
| E7 tools/ 同步管线根因 | v0.2.22/23（附 fmt 存量漂移根因：历次手改未跑 fmt，建议执行窗收尾门禁固定加 `cargo fmt --check`） |
| Agnes caps 申报 128K | 供应商侧待核（维持） |
| 静默截断 ×6 | **本批 4 加已修**，decision-status 相应滚动 |

## 8. Node 14 交接（测试窗独立复测）

- **版本绑定**：v0.2.21（`hearth --version` 双 VM 核验记录见附录 B）。
- **预注册预期**：RC52 修复后，fresh 失败率应**显著低于 40%**（A1 族「已验证完成 + 继续」假阴性链路已消除）；若 fresh 仍 ~40%，视为修复无效信号，回炉。
- 纪律不变：A/B 复现 + C 待解阻塞；不得为提高成功率修改 benchmark。

## 附录 A：workspace 测试明细（v0.2.21 gate 实测）

**模板（重要更正）**：`source ~/.cargo/env && unset HEARTH_URL && export HEARTH_CGROUP_BASE=/sys/fs/cgroup/hearth`，**不设 `HEARTH_ALLOW_NO_CGROUP`**。
- 更正说明：本窗早前曾判「3 个测试需 `ALLOW_NO_CGROUP=1`」——实测证伪：去掉该变量后 agent-core 126/126、sandbox 22/22 均绿；而 `test_rt4_cgroup_fail_closed` **前提要求该变量不存在**（设了必红）。早前 3 失败真因是并行污染，非环境变量缺失。

| 门 | 命令 | 结果 |
|---|---|---|
| fmt | `cargo fmt --check` | ✅（统一 17 文件存量漂移后） |
| clippy | `cargo clippy --workspace --all-targets -- -D warnings` | ✅ |
| test | `cargo test --workspace` | ✅ **460 passed / 0 failed / 61 targets** |

关键 crate 明细：agent-core 126、project-sync 45、llm-gateway 27、sandbox 22、hearth(lib) 22、planner 11、agent-types 11、experience 11、project-xray 11（含 wiring 哈希锁 2/2）、observer 14、tool-runtime 50 等（余见 gate 输出）。

## 附录 B：部署与版本核验

| 项 | 值 |
|---|---|
| 构建 | `cargo build --release -p codex-cli`（42.08s） |
| 产物 | `~/codex/target/release/hearth`，8,678,496 B，md5 `5e5ac1a327d806f27af77b2e8b89f14e` |
| 部署 | `.131`：`sudo cp → /usr/local/bin/hearth`；旧件备份 `/usr/local/bin/hearth.v0.2.20.bak`（md5 `33444d79…`） |
| 验证 | `.131`：`hearth --version` → **`hearth 0.2.21 (unknown)`**（unknown = VM 树非 git，无 commit 元数据） |
| `.133` | 仍 `hearth 0.2.20`——**有意不部署**：评审机源码树为 `~/codex_t`，拷入 .131 构建产物会形成「新二进制 + 旧源码」错配；测试窗环境按指令为 .131。评审侧若需 v0.2.21，请同步源码后在 .133 自行 rebuild。 |
| tag | `v0.2.21`（本机 git，master，前一 HEAD `ff6657f`） |
