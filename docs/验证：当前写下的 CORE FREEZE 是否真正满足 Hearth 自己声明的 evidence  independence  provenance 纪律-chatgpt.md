GLM，收到上一轮 CORE FREEZE 整合报告，以及外部评审对该报告的进一步审查。

现在不要继续论证“为什么可以 Freeze”，也不要修改生产代码。

我们进入一个非常窄的最终 Closure Verification Pass。

这次任务的目标不是重新审 Hearth，而是验证：**当前写下的 CORE FREEZE 是否真正满足 Hearth 自己声明的 evidence / independence / provenance 纪律。**

请严格遵守：

1. 不修改生产代码。
2. 不为了让结果变好而修改测试。
3. 不把报告中的“已验证”“独立复验”“可重复”当作事实前提。
4. 不采信 commit message、自述、报告结论。
5. 每一个结论必须标明 evidence 来源。
6. 如果某项无法实际执行，必须明确写 UNKNOWN / NOT EXECUTED，不得用已有报告间接替代。
7. 不要为了通过 Freeze 而主动修复发现的问题；本轮只做验证和分类。
8. 严格区分：

   * paper review / narrative review
   * source inspection
   * local command execution
   * independent execution
   * behavioral reproduction
9. 本轮结束时，不预设 CORE FREEZE 一定成立，也不预设一定撤销。由证据决定。

---

# Closure Verification Pass

请只处理下面三个问题。

## CV-01：真正独立执行 §8 验证命令

这是最高优先级。

当前报告 §8 给出了完整的 grep / cargo test / gate / 真机命令清单，但目前外部评审窗口主要完成的是“报告与论证审查”，并没有亲手执行这些命令。

因此请确认：

### A. 评审 VM `.133`

在实际仓库：

`~/codex_t`

亲自执行报告 §8 中与 `.133` 对应的命令。

至少包括：

```bash
grep -n "pub enum FailureKind" crates/agent-core/src/terminal.rs
grep -n "GIVE_UP_ROUTED_TO_DONE\|GIVE_UP_OVERRIDDEN\|GIVE_UP_INTERCEPTED\|giveup_unverified" crates/agent-core/src/loop.rs | head

grep -n "一次工具交换 = 一个新 Turn" crates/agent-core/src/loop.rs
grep -n "HEARTH_COMPACTION_MODE\|CHARS_PER_TOKEN" crates/agent-core/src/context.rs crates/agent-core/src/loop.rs
grep -rn "compact_pressure_pct" crates/ | head -3
grep -rn "context_fill_pct" crates/ || echo "旧名清零 ✓"

grep -n "pub struct BashExitError" crates/tool-runtime/src/dispatcher.rs
grep -n "history-slice-note" crates/agent-core/src/loop.rs
grep -n "set_session_id" crates/codex-cli/src/run_local.rs

grep -n "env 测试仪器必须压过注入值" crates/agent-core/src/context.rs

cargo test -p agent-core --lib inv_m01
cargo test -p agent-core --lib test_rc47
cargo test -p agent-core --lib test_five_state
cargo test -p agent-core --lib test_history_slice_marker

bash ~/run_gate_r2c.sh
```

### B. 对每条命令记录：

* 实际命令
* 原始 stdout/stderr
* exit code
* 执行环境
* binary/source provenance
* 是否与报告 §8 的预期一致

不要只输出“全部通过”。

如果某条命令无法执行，明确：

`NOT EXECUTED`

并说明原因。

### C. `.131` 真机命令

如果当前执行窗口具备 `.131` 访问能力，也执行：

```bash
cargo test --manifest-path /tmp/cfr_n09/todoapi/Cargo.toml
cargo test --manifest-path /tmp/cfr_n09b/units/Cargo.toml
cargo test --manifest-path /tmp/cfr_n10a/configlib/Cargo.toml
cat /tmp/cfr_n08/FREEZE_RESULT.txt
grep -ac VERIFICATION_RESERVE ~/fa/lr_n12r1.log
python3 /home/wutao/fa/collect_lr.py
```

同样必须保留原始结果。

如果 `.131` 无法从当前窗口访问，不得用已经归档的日志伪装成“本次独立执行”。

明确写：

`.131 independent execution = NOT AVAILABLE`

然后将历史归档证据与本次实际执行严格分开。

---

# CV-02：RC48 n12r3 可重复性验证

上一轮报告及评审中使用了：

* n12r1：Reserve 已耗尽 → failed
* n12r3：Reserve 尚存在 → GiveUp intercepted → verification passed → completed

这个对照足以支持“因果假设 A”，但目前不足以支持强措辞：

> “可重复”

因为目前实际样本仍然主要是 n=1 vs n=1。

因此：

## 不要再使用“可重复”作为既成事实。

请把问题改写成：

> 在相同任务、相同参数、相同机制条件下，当 GiveUp 时点存在 Verification Reserve 时，是否能够稳定复现 GiveUp interception → verification → completion？

如果环境允许，在不修改生产代码的情况下，对 n12r3 的决定性条件进行 **3–5 次独立重复**。

每次至少记录：

| run | GiveUp | Reserve at GiveUp | Intercept | Verification | Terminal | artifact | independent recheck |
| --- | ------ | ----------------: | --------- | ------------ | -------- | -------- | ------------------- |

重点观察：

1. GiveUp 是否发生；
2. GiveUp 时 Reserve 是否 > 0；
3. `GIVE_UP_OVERRIDDEN` 是否出现；
4. verification 是否实际执行；
5. verification 是否通过；
6. 最终是否 completed；
7. 是否存在任何 false stop；
8. 是否出现其他影响因素。

### 如果只能跑 1 次

不要补写成“可重复”。

改为：

> `single reproduction observed; repeatability not established`

### 如果 3–5 次全部符合

才允许使用：

> `reproduced N/N under the tested condition`

注意：

**不要从 3–5 次样本推导“机制普遍可靠”。**

它只能证明：

> 在该具体条件下观察到 N/N 的复现。

RC48 的 F1 disposition 不因此自动升级或降级。

---

# CV-03：Interaction Surface Audit / Decision-Terminal Mapping Audit

上一轮评审曾要求增加两个验证面：

1. Interaction Surface Audit
2. Decision-Terminal Mapping Audit

当前 CLOSURE-01 报告虽然出现了：

* Authority Matrix
* Decision→Terminal Map
* Node 02
* Node 03/04
* Node 11 QA

但这不等价于已经证明：

> “上一轮要求增加的 audit 已经作为独立验证项正式完成。”

因此请做一次严格 provenance check。

## 对 Interaction Surface Audit：

确认：

* 是否存在明确的 audit artifact；
* 文件路径；
* audit scope；
* 输入 surface；
* 检查项；
* 实际结果；
* 是否有真实执行证据；
* 是否只是报告中的文字总结。

如果没有完整独立 artifact：

明确标记：

`Interaction Surface Audit = NOT ESTABLISHED AS INDEPENDENT EVIDENCE`

不要因为 Node 11 存在就自动视为 audit 已完成。

## 对 Decision-Terminal Mapping Audit：

同样确认：

* artifact 路径；
* scope；
* decision 枚举；
* terminal 枚举；
* mapping；
* authority；
* downgrade / rejection path；
* 实测 evidence。

尤其确认：

`Decision → Terminal`

是否真正逐项有证据，而不是只有一张人工整理的 mapping table。

如果 Node 02 已经满足该要求，可以明确：

`Decision-Terminal Mapping Audit = SATISFIED`

并引用具体 artifact / command / evidence。

如果只是部分满足：

`PARTIAL`

不要强行 PASS。

---

# 最后增加一个非常重要的“程序状态修正”

请正式承认下面这个事实：

当前报告标题和 CLOSURE-01 已经写：

> CORE FREEZE

但这份 Freeze 原本是执行窗口自身写入的最终裁决。

而 Hearth 自己又要求：

> reviewer cannot self-certify / execution window cannot be sole authority.

因此：

**在 CV-01～CV-03 完成之前，不得把“报告中已有的 CORE FREEZE”当作独立裁决已经成立。**

请在 Closure Verification 结论中明确区分：

### State A

`Execution-window declared CORE FREEZE`

与

### State B

`CORE FREEZE independently verified`

这两个状态不能混为一谈。

如果本轮 CV-01～CV-03 完成后证据足够，则可以正式说：

> CLOSURE-01 的执行窗口自我声明经过独立验证后获得确认。

如果仍不足，则：

> CORE FREEZE remains execution-window declaration, not independently verified.

这不是要求撤销 Freeze，而是要求把**程序事实说准确**。

---

# 最终输出格式

请不要重新写一篇泛泛的 Hearth 总结。

只提交一份：

# CLOSURE VERIFICATION REPORT

包含以下章节：

## 1. Execution Provenance

* VM
* source
* binary
* commit/tag
* execution identity
* commands actually executed

## 2. CV-01 Independent §8 Execution

表格：

| Command | Executed? | Exit | Actual Result | Matches Report? |
| ------- | --------- | ---: | ------------- | --------------- |

附关键原始输出。

## 3. CV-02 RC48 Repeatability

| Run | Reserve | Intercept | Verification | Terminal | Result |
| --- | ------: | --------- | ------------ | -------- | ------ |

明确：

`repeatability established / not established`

## 4. CV-03 Audit Closure

### Interaction Surface Audit

* artifact
* evidence
* status

### Decision-Terminal Mapping Audit

* artifact
* evidence
* status

## 5. Independence Status

明确回答：

* 哪些是 paper review；
* 哪些是 source inspection；
* 哪些是真实 command execution；
* 哪些是独立 behavioral reproduction；
* 哪些仍然只是 execution-window evidence。

## 6. Freeze Procedural Status

严格选择：

### A. INDEPENDENTLY VERIFIED CORE FREEZE

或

### B. CORE FREEZE DECLARED, INDEPENDENT VERIFICATION INCOMPLETE

或

### C. CORE FREEZE BLOCKED

不要为了符合原报告而选择 A。

## 7. F0 / F1 / F2

重新列出：

* F0
* F1
* F2
* 新发现
* 是否产生新的 blocker

但不要因为证据缺失自动把原来的 F1 变成 F0。

---

最后一句必须非常克制：

**本轮不是为了证明 Hearth 已经完美，而是为了证明 Hearth 是否已经达到它自己定义的“可以冻结”的证据门槛。**

如果证据不够，就老实说不够。

如果证据够，就给出证据。

不要用措辞替代实验。
