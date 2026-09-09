# Hearth v0.2.18 独立验证阶段评审报告

## ——关于 CORE FREEZE 状态、盲测反证与下一阶段证据闭环的综合裁决

**评审对象**：Hearth v0.2.18
**基线**：tag `v0.2.18`
**此前裁决**：CORE FREEZE / CLOSURE-01
**本报告性质**：独立验证阶段的横向评审意见
**当前结论**：**FREEZE CONFIRMATION SUSPENDED；v0.2.18 保留为工作基线；不得继续宣称“独立验证已完成”**

---

# 0. Executive Summary

本轮出现了一个与此前所有评审不同性质的重要事件：

在 `v0.2.18` 已由 CLOSURE-01 宣布：

* F0 = 0
* false completion = 0
* silent loss = 0
* infinite loop = 0
* sandbox/approval bypass = 0
* Projection distortion = 0
* zero blocker
* CORE FREEZE

之后，出现了一次**非执行窗口、非脚本化、非预设 benchmark 的真实用户盲测**。

该盲测约 90 分钟、36 个 run，并发现至少三类值得重新核验的行为：

1. **用户终端出现裸 `ERROR agent_core::r#loop` tracing 输出**，与 CLOSURE-01 “Projection 失真 = 0”的声明存在直接潜在冲突；
2. **run-013 ~ run-036 连续 24 次失败**，表现出明显的 persistent failure attractor；其中大量失败发生在用户尝试询问“上一轮为什么失败/发生了什么”的诊断场景；
3. **内部已经生成的分析内容未完整到达用户可见输出**，用户最终只看到 `Done`/状态标记，形成明显的 completion evidence visibility 问题。

同时还观察到：

* `introspect` 偶发被当作 bash 命令执行；
* `apply_patch` 的空白/缩进匹配问题仍可复现；
* 长会话后期出现与当前用户输入不匹配的旧目标/旧 anchor；
* compacted summary 出现重复、内部 Debug 结构暴露；
* T4=2 的 stalled 阈值是否过于敏感值得实验，但目前**不应直接修改阈值**，应先完成根因归因。

因此，本轮的正确工程结论不是：

> “v0.2.18 已经被证明错误，应立即回滚。”

也不是：

> “这些只是新的 F1，不影响 Freeze。”

而是：

> **此前 CORE FREEZE 是一个有效的候选基线，但其“独立验证完成”这一证据状态已经被新的独立行为证据挑战。必须暂停 Freeze confirmation，保留 v0.2.18 作为工作基线，并针对冲突的 invariant 进行定向复现和归因。**

---

# 1. 首要程序性结论：代码状态与证据状态必须分离

此前最大的程序性问题之一，是容易把：

> “代码是否应该继续作为基线”

和：

> “我们是否已经拥有足够证据确认 Freeze”

混为一谈。

本轮明确建议正式建立两个独立状态：

## 1.1 Artifact State

当前：

**v0.2.18 = 保留**

理由：

* 当前没有证据证明整个 Core 必须回滚；
* 已有大量机制层测试通过；
* 本轮盲测发现的是部分行为路径和验证边界问题；
* 没有发现 sandbox bypass、authority violation 等必须立即撤回二进制的证据。

因此：

> **不得因为 Freeze confirmation 被暂停，就自动把 v0.2.18 判定为不可用或要求回滚。**

## 1.2 Evidence State

当前：

**FREEZE CONFIRMATION = SUSPENDED**

原因：

新的独立盲测产生了与 CLOSURE-01 F0 声明直接冲突的候选证据。

因此当前不得继续写：

> “CORE FREEZE 已经经过独立验证确认。”

正确表述应为：

> **v0.2.18 remains the Freeze candidate baseline; independent confirmation is suspended pending targeted reproduction and adjudication.**

这个状态区分应成为 Hearth 后续所有 Freeze 流程的正式规则。

---

# 2. 本轮最重要的方法论发现：脚本化验证覆盖范围并不等于真实交互可靠性

此前证据链主要是：

```text
源码/机制设计
    ↓
scripted tests
    ↓
long-run benchmark
    ↓
review package
    ↓
closure review
    ↓
CORE FREEZE
```

本轮第一次增加：

```text
v0.2.18
    ↓
真实用户
    ↓
无预设剧本
    ↓
长会话
    ↓
自然追问
    ↓
失败后诊断
    ↓
真实 terminal projection
```

结果发现了此前脚本化 benchmark 没有暴露的问题。

这说明：

> **scripted benchmark 可以证明测试覆盖范围内的性质，但不能单独证明真实交互状态空间不存在新的失败吸引态、上下文污染和 projection escape。**

因此以后 Freeze 前应正式加入：

## Independent Blind Interaction

其性质不是普通新增测试 case，而是独立验证层。

建议最终 Freeze 验证链为：

```text
Structural Verification
        ↓
Scripted Behavioral Verification
        ↓
Long-run Verification
        ↓
Independent Review
        ↓
Independent Blind Interaction
        ↓
Targeted Reproduction of Anomalies
        ↓
Freeze Confirmation
```

---

# 3. Finding P0-A：裸 tracing ERROR 与 Projection F0=0 的潜在直接冲突

盲测报告记录了多个用户可见的：

```text
ERROR agent_core::r#loop:
do_plan_inner failed
error=stalled: 2 consecutive replans produced identical TaskGraph ...
```

随后又出现格式化的：

```text
✗ stalled: ...
✗ Done (...)
```

如果原始终端证据确认该 `ERROR` 确实直接出现在用户-facing terminal，则它与 CLOSURE-01：

> “错误输出投影成功 = 0”

存在直接冲突。

这里不应把问题降级成：

> “日志不够漂亮。”

这是一个 observable behavior property。

此前模型可能只验证了：

```text
structured terminal state
        ↓
formatted projection
```

但真实用户面对的是：

```text
ALL runtime output
        ↓
user-facing terminal
```

因此必须区分：

### Projection Correctness

系统投影的结构化事实是否正确。

### Projection Isolation

内部 runtime diagnostics 是否会绕过 projection boundary 直接进入用户界面。

### Projection Completeness

系统决定应该呈现给用户的信息是否完整到达。

本轮盲测至少同时提出了后二者。

---

# 4. P0-A 的验证要求

首先不要直接修改代码。

应在 `v0.2.18` 原样环境中完成：

1. 找到产生：
   `do_plan_inner failed`
   的 tracing 调用点；
2. 确认其 level；
3. 确认输出 destination；
4. 确认 stdout/stderr；
5. 确认正常 SSH terminal 是否可见；
6. 用真实 T4 stalled 场景原样复现；
7. 保存原始 terminal output。

如果复现成立：

> **CLOSURE-01 的 “Projection distortion = 0” 不能继续保持原判级。**

但在没有源码/运行时复现前，不应提前声称根因一定是 tracing 默认 stderr。

---

# 5. P0-B：Persistent Failure Attractor

盲测最值得关注的行为不是单次 false stop，而是：

```text
run-013 → failed
run-014 → failed
...
run-036 → failed
```

共 24 个连续失败。

这不能直接证明：

> “系统永久无法恢复。”

因为当前证据只有一个长 session。

更准确的术语应是：

> **Observed Persistent Failure Attractor**

即：

> 在该 session 的状态条件下，系统进入一个连续失败状态，并未自行逃逸。

这比“永久回不来”更严谨。

---

# 6. 为什么它比 RC48 更重要

RC48 的基本结构是：

```text
GiveUp
  ↓
Reserve 已耗尽
  ↓
没有第二次 verification opportunity
  ↓
false stop
```

这是一个**有界决策资源问题**。

而本轮发现的行为可能是：

```text
failure
  ↓
user asks for diagnosis
  ↓
planner produces diagnostic TaskGraph
  ↓
replan similarity
  ↓
T4
  ↓
GiveUp
  ↓
user asks again
  ↓
similar diagnostic TaskGraph
  ↓
T4
  ↓
...
```

系统每轮都可能正常终止，因此：

> “没有无限循环”

仍可能成立。

但：

> “failure recovery 正常”

则受到严重挑战。

这揭示了一个此前 Reliability Matrix 中需要进一步拆开的维度：

| 属性                   | 含义            |
| -------------------- | ------------- |
| bounded termination  | 是否会最终停止       |
| recovery correctness | 是否能从失败状态恢复    |
| failure-state escape | 是否能离开持续失败吸引态  |
| diagnosis usability  | 系统失败后能否解释自身失败 |

**bounded termination ≠ successful recovery。**

---

# 7. P0-B 与旧目标/anchor 脱节问题应合并实验

本轮不建议先独立调查：

> “诊断任务为什么容易 T4？”

再调查：

> “为什么当前输入与旧 anchor 脱节？”

因为两者可能是同一个上游问题。

建议直接做一个 2×N 对照：

## Condition A：Fresh Session

全新 session：

```text
diagnostic request
→ capture TaskGraph
→ capture anchor
→ capture context
→ capture replan fingerprint
→ capture terminal state
```

## Condition B：Polluted Session

先制造：

```text
failure
→ replan
→ compact
→ stalled
```

再提出完全相同的 diagnostic request。

然后比较：

* TaskGraph fingerprint
* anchor
* session id
* compacted summary
* current user turn
* state
* planner input
* replan count
* T4 trigger
* Reserve
* terminal decision

---

# 8. 该实验的关键判定

如果：

```text
Fresh Session
→ diagnostic request
→ normal
```

而：

```text
Polluted Session
→ same diagnostic request
→ stale TaskGraph
→ repeated T4
```

则高度支持：

> **context / anchor / session continuity contamination**

而不是：

> diagnostic tasks inherently trigger T4.

这两个假设的修复方向完全不同。

因此：

**目前不建议直接把 T4 阈值 2 改成 3。**

先归因，再决定阈值。

否则可能只是：

```text
真正的根因
    ↓
T4 提前暴露
    ↓
把 T4 调宽
    ↓
症状晚一轮出现
```

这不是修复。

---

# 9. P0-C：内部完成证据与用户可见完成不是同一个性质

盲测发现：

> 实际已经执行分析、读取文件、生成报告，但最终用户只看到 `Done` 或状态标记。

技术上，这不是严格意义的：

> false completion。

因为事实可能真的完成了。

但从用户角度：

> “完成了，但我看不到任何依据”

与：

> “系统声称完成但实际上没做”

在即时交互体验中非常接近。

因此建议将 Completion 分为四层：

| 指标                           | 定义                 |
| ---------------------------- | ------------------ |
| `false_completion`           | 声称完成但事实未完成         |
| `completion_evidence`        | 内部存在完成证据           |
| `completion_projection`      | 完成证据被正确投影          |
| `user_verifiable_completion` | 用户无需访问内部文件即可理解完成依据 |

此前 F0=0 主要覆盖第一层。

本轮说明后面三层仍需明确测试。

---

# 10. P0-C 不应被降成普通 UI 问题

这是因为 Hearth 的核心原则之一是：

> 事实必须能够被投影。

如果内部事实存在：

```text
analysis result
artifact
verification
```

但用户最后只得到：

```text
✓ Done
```

那么：

> Core 内部事实正确 ≠ 用户能够验证 Core 的行为。

因此建议把：

**Projection Completeness / User-verifiable Completion**

作为独立 reliability property。

---

# 11. 2.1 introspect → bash

盲测发现：

```text
bash → cmd: introspect
```

导致：

```text
bash: introspect: command not found
exit code 127
```

而另一路又正确出现：

```text
⚙ introspect →
```

这说明可能存在 tool selection / tool routing 分歧。

应调查：

```text
user intent
 ↓
planner
 ↓
tool selection
 ↓
structured tool invocation
```

是否存在某条路径把：

```text
introspect
```

错误序列化成：

```text
bash("introspect")
```

目前不能直接升格成 F0。

除非进一步证明：

> structured tool 被错误降级到 shell 后绕过了 sandbox / approval / authority boundary。

当前证据尚不足以得出这一结论。

---

# 12. 2.2 apply_patch whitespace failure

这是一个真实的工具鲁棒性问题。

它已经在更早测试中出现，本轮 v0.2.18 仍然能够复现。

因此：

> 此项不能再被描述为“已经解决”。

但目前也没有证据表明它违反 Core hard invariant。

建议：

* 记录为 tool robustness defect；
* 独立修复；
* 增加 whitespace / indentation / newline variation fixture；
* 不要为了修复它扩大 Core scope。

---

# 13. 2.3 长会话后的目标锚点脱节

本轮观察：

用户输入：

```text
你能回复我一下情况吗
```

但 planner 仍然产生类似：

```text
Locate the ledger file
Verify step 1 is marked Done
...
```

这不是普通的“模型回答不好”。

必须追踪：

```text
current user message
        ↓
session state
        ↓
compact summary
        ↓
anchor
        ↓
planner input
        ↓
TaskGraph
```

尤其需要确认：

> **过期目标到底在哪一层重新成为 planner 的输入。**

不能仅凭最终输出把责任归给 model。

---

# 14. 2.4 compacted summary 重复打印

本轮观察：

```text
[compacted session summary]
轮次目标: Text("...")
```

在多个 Plan cycle 中重复出现，并且暴露了内部 Debug 结构。

这里至少包含两个性质：

1. summary 是否被正确更新；
2. internal representation 是否错误进入 projection。

因此建议与 P0-A / P0-C 一起归入：

> **Projection Boundary Regression**

专项测试。

---

# 15. 关于 T4：不要先修阈值

系统在 run-030 自己提出：

> stalled threshold 2 → 3

这个建议可以作为 hypothesis。

但不能因为它来自系统自身，就作为解决方案。

正确实验是：

```text
v0.2.18 / T4=2
        ↓
重放
        ↓
记录 failure trajectory

临时实验版本 / T4=3
        ↓
相同条件重放
        ↓
比较
```

必须同时回答：

1. 连续诊断失败是否减少；
2. 正常任务的 false stop 是否增加；
3. 真正死循环的 detection latency 增加多少；
4. TaskGraph fingerprint 是否仍然不变；
5. 是否只是把失败推迟一轮。

只有在这些数据成立之后，才讨论是否进入 RFC。

---

# 16. 当前不能直接推翻的内容

为了避免过度反应，本轮盲测目前**不能直接证明**以下内容错误：

* false completion 必然 > 0；
* sandbox/approval bypass；
* authority violation；
* silent persistent state loss；
* resume 必然 reteach；
* 所有环境下都会出现 24 连败；
* T4 本身设计错误；
* v0.2.18 整体架构需要回滚。

因此不要从：

> “发现三个严重问题”

跳跃到：

> “整个 Core 设计失败”。

这同样违反证据纪律。

---

# 17. 当前 Reliability Matrix 应暂时修改为“待重新核验”

建议至少临时把以下项目标成：

| Capability       | CLOSURE-01 | Blind-test status                                                     |
| ---------------- | ---------- | --------------------------------------------------------------------- |
| Failure Recovery | PASS*      | **REOPEN / targeted reproduction**                                    |
| Decision         | PASS*      | **REOPEN / targeted reproduction**                                    |
| Projection       | PASS       | **REOPEN / direct contradiction candidate**                           |
| Memory           | PASS*      | **REOPEN / context-contamination investigation**                      |
| Completion       | PASS       | **PASS on false-completion definition; projection completeness OPEN** |
| Terminal         | PASS       | **Projection boundary OPEN**                                          |

其他没有被本轮证据触及的项目可以保持原状态。

---

# 18. 关于原 CLOSURE-01 的评价

CLOSURE-01 不应被简单判定为：

> “错误报告。”

更准确的评价是：

> **CLOSURE-01 在其测试覆盖范围内形成了一个相当完整的证据包，但真实非脚本化交互随后证明，该 evidence envelope 尚不足以支持“全部用户可见行为已独立确认”的更强陈述。**

尤其应该保留它已经证明的东西：

* 447/0；
* FA01；
* P2-MC；
* P2-LR；
* RC47；
* 五态；
* Reserve 机制；
* archive-first；
* authority matrix；
* 多个真机 benchmark。

不能因为新的盲测，就把这些已有证据全部清零。

真正需要做的是：

> **把新增观察映射回已有 invariant，然后重新判断哪些 invariant 被直接反证，哪些只是新增 F1。**

---

# 19. 当前 Freeze 状态

建议正式写成：

## CORE FREEZE CANDIDATE — CONFIRMATION SUSPENDED

### Guarantees already evidenced

保留此前已验证且没有被本轮直接反证的机制性质。

### Claims currently under revalidation

至少：

1. Projection isolation；
2. Projection completeness；
3. Failure-state escape；
4. Diagnostic recovery；
5. Long-session anchor continuity；
6. Context/session provenance。

### Explicitly not concluded

目前不得宣称：

> “independent verification complete”。

---

# 20. 下一阶段实验优先级

建议严格按以下顺序：

## Priority 0

### A. 原样复现裸 ERROR

目标：

> 确认 Projection F0 是否直接失效。

---

### B. Fresh vs Polluted Session 对照

目标：

> 判断 persistent failure attractor 与 context/anchor contamination 是否存在因果关系。

---

### C. 用户可见完成证据测试

目标：

> 区分内部 completion evidence 与 user-verifiable completion。

---

## Priority 1

### D. introspect routing audit

### E. apply_patch robustness fixture

### F. compact summary projection audit

---

## Priority 2

### G. T4=2 vs T4=3 controlled probe

只有 A/B/C 的归因完成之后再进行。

---

# 21. 新增建议的证据对象

建议以后每次 Freeze 都保存：

```text
freeze-artifact-state
freeze-evidence-state
independent-blind-test
targeted-reproduction
known-unknowns
accepted-deviations
```

尤其是：

```text
independent-blind-test
```

必须成为正式证据包的一部分。

---

# 22. 最终裁决

**当前不建议继续推进“CORE FREEZE 已独立确认”的宣称。**

但同样：

**当前也不建议撤回 v0.2.18 或重新施工整个 Core。**

正确状态为：

> **v0.2.18 保留为 Freeze Candidate / Working Baseline；Freeze Confirmation 暂停；新增独立盲测证据进入正式证据链；针对 Projection、Failure Recovery、Context/Anchor Continuity 做定向复现。**

最重要的程序原则是：

> **不要因为代码暂时没有被证明需要回滚，就把证据不足说成证据充分。**

反过来也一样：

> **不要因为发现新的行为缺陷，就把已经被大量证据支持的机制性质全部推翻。**

目前真正需要做的是：

**把“什么已经被证明”“什么被反证”“什么只是新增未知”重新分层。**

---

# 23. 对下一轮评审窗口的明确要求

下一轮执行/验证窗口不要直接修复问题后再回来报告。

必须先提交：

### ① 原始复现证据

包括命令、环境、原始 stdout/stderr、时间戳、session/run ID。

### ② 源码定位

说明对应行为来自：

```text
mechanism
decision
model
provider
environment
```

中的哪一层。

### ③ 反事实对照

尤其是：

```text
Fresh Session
vs
Polluted Session
```

以及必要时：

```text
T4=2
vs
T4=3
```

### ④ invariant mapping

明确写：

```text
原 CLOSURE claim
        ↓
新观察
        ↓
是否直接冲突
        ↓
是否需要 reopen F0
        ↓
还是新增 F1
```

### ⑤ 最终 disposition

每个 finding 只能进入：

```text
CONFIRMED CONTRADICTION
NEW F0
NEW F1
EVIDENCE GAP
FALSE POSITIVE
DEFERRED
```

不得使用模糊的：

> “基本没问题。”

---

# 24. 总结

这次盲测最重要的成果，不是发现了多少 bug。

真正重要的是：

> **它证明了 Hearth 已有的 Freeze 验证体系仍然存在一个从 scripted evidence 到真实 interaction evidence 的 coverage gap。**

而这恰恰是一个成熟系统在冻结之前应该暴露的问题。

因此当前最合理的态度不是恐慌，也不是护住 Freeze。

而是：

> **Freeze 候选基线保留。**
>
> **Freeze 确认暂停。**
>
> **独立盲测证据进入正式证据链。**
>
> **先复现、再归因、后判级、最后决定是否修复。**
>
> **不因为漂亮的报告而相信，也不因为一次异常而过度否定。**

这才与 Hearth 自己一直要求的：

> **“不采信自述，只采信可复核证据。”**

保持一致。
