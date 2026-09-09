# Hearth RC24 审批门施工验收报告 v1

> **性质**：施工完成 + 验收证据汇报（执行窗口 → 顶层）。
> **施工单**：`docs/hearth-rc24-approval-gate-construction-order-v1.md`（基线 v0.2.9 / HEAD `66705aa`）。
> **结果**：**三项全部施工完成，隔离门禁 399 passed / 0 failed / 1 ignored（全绿，+8 新测试），真机验收 5 项执行完毕；2 项新发现 + 2 项残留如实披露**。

---

## 一、施工内容与实测锚点

施工单锚点复核发现两处行号漂移（逻辑一致，不影响施工）：

| 项 | 施工单锚点 | 实测锚点 | 结论 |
|---|---|---|---|
| 判定函数 | loop.rs:220-270 | loop.rs:220-272（施工前） | ✅ 一致 |
| one-shot 审批 | run_local.rs:702-720 | run_local.rs:**788-896**（NeedApproval 分支） | ⚠️ 漂移 |
| REPL 审批 | repl.rs:171-188 | repl.rs:176-200（SSE 版）；直跑 REPL 审批实际在 run_local.rs | ⚠️ 漂移 |
| terminal normalize | terminal.rs:42-52 | terminal.rs:42-52 | ✅ 一致 |

### A. 审批判定语义化（TC-8/TC-9）——commit 1
- `bash_cmd_is_destructive` 重构为三级分类 `BashRisk::{Benign, CommandTable, HardRedline}`（`classify_bash_cmd`，loop.rs）。
- 新增 `redirect_targets()` 重定向目标扫描器：`>`/`>>`/`2>`/`&>` 全形态，**无空格写法与带空格同权**（TC-9 根因修复），引号内 `>` 不解析、fd 复制（`2>&1`）不算路径。
- `/dev/null` 精确豁免（位桶，Benign）；其余 `/dev/*`、`/proc/`、`/sys/` → HardRedline（物理级/内核接口，永不委托）。
- 命令表（rm/dd/…）→ CommandTable（可委托）；fork bomb → HardRedline；pipe-to-shell → CommandTable。
- 新增单测 4 个：`test_rc24_dev_null_bitbucket_not_destructive`（10 形态）/ `test_rc24_other_dev_nodes_still_blocked`（7 形态）/ `test_rc24_kernel_interfaces_still_blocked`（4 形态）/ `test_rc24_risk_tiers`。

### B. 非交互审批结构化语义——commit 2
- 新增 `ApprovalPolicy::{Interactive(默认), DenyAllNonInteractive, DelegateSession}`（agent-core 导出）+ `set_approval_policy()`。
- `do_act` 审批块三分支：非交互模式下审批请求**照常进事件流**（InteractionRequested 同形，payload 仅追加 `denied:"noninteractive"` 标记——消费方零改动红线遵守）→ 立即结构化拒绝 → run 终止。
- 新增 `run_abort` 结构化终止通道：`summary.reason=approval_denied_noninteractive` + `summary.hint`（可行动提示）。
- `terminal.rs` normalize 显式映射行（`approval_denied_noninteractive → failed`，九态封闭集不变）+ 封闭集测试补 reason。
- CLI `run_local`：stdin 非 tty → 自动 DenyAllNonInteractive；渲染层非 tty 分支不读 stdin、打印 `⛔ … approval_denied_noninteractive` + `↳ 下一步: hearth repl / --approve-within session`。
- 新增单测：`test_rc24_noninteractive_approval_structured_deny`（e2e：reason/hint/事件/normalize 四断言）。

### C. 会话级审批委托（RC29）——commit 3
- one-shot：`--approve-within session`（lib.rs clap 参数，作用域校验仅支持 `session`）。
- REPL：`trust on` / `trust off`（repl.rs 命令区，/help 已更新；委托状态在 AgentLoop 上跨轮保持）。
- 委托语义：CommandTable 级破坏性自动放行；**硬红线永不委托**（fork bomb/设备写//proc//sys——顶层裁决"委托解决打扰，不解除 G0"）。
- 审计三件套：①`tracing::info!(audit=true, delegated=true, cmd=…)` ②CLI 投影 `[delegated] 会话委托放行（审计已记）: …`（ThinkSummary 帧）③run report `summary.approval_delegated` + `approval_delegated_cmds` 字段（completed/budget/abort/error 四条终态路径均落）。
- 新增单测 3 个：`test_rc24_delegate_session_auto_approves_command_table`（真实 rm 执行+零弹窗+审计字段）/ `test_rc24_delegate_does_not_bypass_hard_redline`（fork bomb 委托下仍弹审批）/ `test_rc24_interactive_policy_unchanged`（默认策略回归保护）。

---

## 二、先红后绿取证

旧逻辑（loop.rs@66705aa）Python 等价复刻实测（2026-08-29 本机）：

```
TC-8 [echo hi > /dev/null]          old=True  -> RED(误判破坏性)
TC-8 [curl -s url 2> /dev/null]     old=True  -> RED(误判破坏性)
TC-8 [cmd > /dev/null 2>&1]         old=True  -> RED(误判破坏性)
TC-9 [echo x >/dev/sda]             old=False -> RED(无空格漏判)
TC-9 [dd … of=/dev/sda 2>/dev/null] old=True  -> green（旧代码靠 dd 命令表接住）
TC-9 [echo x > /dev/mem]            old=True  -> green（旧代码靠带空格匹配接住）
RED CONFIRMED: 4 reds —— 先红后绿取证完成
```

---

## 三、隔离门禁（~/run_gate_r2c.sh @ .133）

| 项 | 结果 |
|---|---|
| cargo fmt --check | RC=0 |
| cargo clippy -D warnings | RC=0 |
| RT4_SOLO（无 env fail-closed） | RC=0 |
| 全量 workspace 测试 | **RC=0，399 passed / 0 failed / 1 ignored**（基线 391 + 本单 8） |

8 个 RC24 测试全部通过（证据：`docs/data/rc24-20260829/gate-rc24.log`）。

---

## 四、真机验收（.133，release 二进制，Agnes 通道）

### 验收① TC-8：`>/dev/null` 位桶写
**判定层 ✅ 转绿**：headless one-shot 下 `echo rc24-tc8-ok >/dev/null && echo TC8_SURFACE_OK` 不再触发审批门（旧行为=阻塞），命令直接进入沙箱执行层。
**⚠️ 新发现 N-1（沙箱层，非本单范围）**：执行被 **landlock** 拒绝——
```
WARN sandbox::linux_impl: landlock_add_rule failed for '/dev/null': Invalid argument (os error 22). Rule skipped.
bash: 行 1: /dev/null: 权限不够
```
即施工单前提"T6 后 landlock 已放行其写入"在真机不成立：T6 的放行规则本身添加失败（EINVAL），`/dev/null` 在沙箱内**仍不可写**。此前该问题被审批门遮挡（命令根本走不到沙箱），RC24-A 放行后暴露。**归 G0/sandbox 独立修复单**（本单授权范围不含 sandbox crate）。TC-8 完整转绿依赖该修复。

### 验收② TC-9 + B 项：设备写 headless 结构化拒绝 ✅
`echo rc24-tc9 >/dev/sda 2>&1`（无空格写法）：
```
⚙ bash → cmd: echo rc24-tc9 >/dev/sda 2>&1
✗ ⛔ approval — 非交互模式审批被拒（approval_denied_noninteractive）
  ↳ 下一步: 用 hearth repl（可交互批准）或 --approve-within session（显式委托）后重跑
✗   ✗ Task failed — approval_denied_noninteractive（3 步）……
  ↳ execute bash tool —— 破坏性操作在非交互模式被拒绝……
```
act span **0ms**（总耗时 1m54s 全部是 Agnes 网络抖动重试，与审批无关）。对照旧基线 B02/B04 的 23s 阻塞后无 reason 失败：**秒级结构化失败达成**。run report reason 字段落盘（`tc9-denied-run-report.md`）。

### 验收③ B02/B04 复跑 + RC29：委托推进任务 ✅
`--approve-within session` + 场景（write_file 创建 /tmp/rc24_victim.txt → bash rm 删除）：
```
⚙ bash → cmd: rm /tmp/rc24_victim.txt
💭 [act] [delegated] 会话委托放行（审计已记）: bash: rm /tmp/rc24_victim.txt   ← act span 22ms
✓
```
- **0 次审批弹窗**（对照 ALL L22011→L24615 的 2 次重复弹窗）——RC29"授权后离开"达成；
- rm **真实执行**：事后 `ls` 确认文件已删；报告 md 工具表 `bash … rm … ✓`（`rc29-delegated-run-report.md`）；
- 委托审计链：CLI `[delegated]` 帧 + summary 审计字段 + tracing 日志。
- **如实披露**：该 run 最终 give_up（8 steps, 0 errors, rm 已成功）——这是 W3/W4 回归验证轮已报告的 **DEV-2（reflect LLM 判定质量）**残余，非本单回归；DEV-1/W8 路由后论述与产物任务的完成判定将分别治理。

### 验收④ REPL trust 场景 ✅
管道驱动（reedline 受限终端 fallback 路径）：
```
hearth>   🔓 trust on——会话级审批委托已开启：命令表级破坏性操作（rm/dd 等）自动放行，
          逐条记审计日志；fork bomb/设备写/内核接口（硬红线）仍需审批
hearth>   🔒 trust off——已撤销委托，恢复逐条审批
hearth> bye 👋
```
trust on 后的**真实任务委托执行**路径与 one-shot 共享 loop 层（已由单测 + 验收③真机覆盖）；REPL 端验证的是命令接线与提示渲染。

### 验收⑤ DEV-4 顺带核查（SIGINT → cancelled 投影链）
- 事件流 ✅：`INFO hearth::run_local: run cancelled by Ctrl-C` + 执行报告落盘；
- REPL 路径 ✅：`⏹ 本轮已取消`（repl.rs:432）；
- **⚠️ 新发现 N-2：one-shot（`hearth chat`）Ctrl-C 后无终态投影行**——run_local.rs:660 cancelled 分支注释"submit/repl 已渲染 ⏹，此处不重复"对 one-shot 路径不成立（one-shot 无 submit()）。用户可见的只有 INFO trace（非交互下通常不可见）。一行级修复（cancelled 分支补投影），**按纪律未越权改动**，交顶层裁决（可并入下一单或 hotfix）。

---

## 五、发现与偏差汇总（交顶层裁决）

| 编号 | 内容 | 建议 |
|---|---|---|
| N-1 | `/dev/null` 沙箱内不可写：landlock 放行规则添加 EINVAL（T6 真机失效，被旧审批门遮挡）。TC-8 判定层已绿、执行层待修 | 归 G0/sandbox 独立单（查 landlock rule flags 对 /dev/null 的处理） |
| N-2 | one-shot Ctrl-C 无 cancelled 终态投影（REPL 有） | 一行修复，可并入 W8 单或 hotfix |
| N-3 | run report **md 渲染**未输出 `approval_delegated` 字段（JSON summary 层已落、CLI [delegated] 帧已投影、tracing 已记）——md 模板未消费该字段 | 低优先，投影层增强随下一单 |
| 残余 | 验收③ run 以 give_up 收尾（0 errors、产物已达成） | 既有 DEV-2（reflect LLM 质量，D 类独立证据窗），非本单回归 |

**红线遵守确认**：InteractionRequested 事件同形（消费方零改动）；D 类冻结清单零触碰；bash stdout 截断/Goal Revision/resource 控流/ContextBuilder 等均未出现在 diff 中。

## 六、变更清单（4 commits）

- `crates/agent-core/src/loop.rs`：BashRisk 分类 + redirect_targets 扫描器 + ApprovalPolicy + do_act 审批三分支 + run_abort 结构化终止 + 委托审计 + 8 个 RC24 单测
- `crates/agent-core/src/terminal.rs`：normalize 映射行 + 测试补 reason
- `crates/agent-core/src/lib.rs`：导出 ApprovalPolicy
- `crates/codex-cli/src/lib.rs`：`--approve-within` 参数 + 作用域校验
- `crates/codex-cli/src/run_local.rs`：策略注入 + 非 tty 结构化拒绝渲染 + hint 投影
- `crates/codex-cli/src/repl.rs`：trust on/off + /help
- 证据归档：`docs/data/rc24-20260829/`（6 文件）

## 七、待办移交

1. 顶层复核本报告 → 处置 N-1/N-2/N-3；
2. W8 · Goal Revision / 任务类型路由（DEV-1，ChatGPT 排定紧随本单）；
3. .131 执行 VM binary 同步（当前仍为 0.2.9+W3/W4，未含 RC24——待验收批复后随 vm-version-sync 一并升级）；
4. 源码备份：`Desktop/hearth-src-backup-20260829-rc24-pre.tar.gz`（施工前，9.6MB）。
