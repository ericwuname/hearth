已审阅《Hearth 问题总账 & 整改申请》。

该文档作为**审计总账与整改候选清单通过**，但不得直接把 W1–W10 当成一个连续施工任务执行。

当前正式裁决：

# 一、先把总账转换为“当前有效施工批次”

当前进入第一批：

```text
W1：基线/环境同步
W3：活性与完成语义
W4：事实投影与结构化完成报告
```

其余：

```text
W5：Context Compaction
W6：Sandbox / Approval
W7：Scope / Tooling
W8：Planning Quality / Goal Preservation / resume exactly-once
W10：Resource observation / bash truncation
```

保持独立待后续派工。

W9 R2-C 门禁收尾属于历史项，**R2-C v0.2.9 已 Final Accepted，不得重新施工**。如工作树存在遗留测试修正，仅做当前状态核实，不重新打开 R2-C 架构。

---

# 二、P0 操作安全：密钥泄漏必须特殊处理

当前总账及相关日志材料中包含明文 API credentials。

执行窗口必须遵守：

```text
禁止重新输出 secret
禁止复制 secret
禁止将 secret 写入新的 docs/reports/logs
禁止将 secret 打包进 release
禁止把 secret 带入 commit
```

后续所有报告只允许使用：

```text
<REDACTED>
```

或：

```text
key fingerprint / suffix
```

不要在施工日志中回显完整凭据。

**credential rotation 与 git history rewrite 是两个独立动作。**

历史清理需要顶层批准；
credential rotation 不应因为 history rewrite 尚未批准而无限期拖延。

目前 `git push` 冻结继续有效，任何窗口不得自行解除。

---

# 三、W1：只做基线统一，不改业务逻辑

先核实：

```text
HEAD commit
Cargo version
.131 binary version
.131 source HEAD
.133 binary version
.133 source HEAD
```

目标：

```text
两 VM binary >= v0.2.9
两 VM source = 当前 HEAD
```

尤其要区分：

```text
binary 已更新
≠
source tree 已同步
```

完成后建立：

```text
docs/vm-version-sync.md
```

记录：

* 当前 HEAD
* 两 VM commit/version
* binary path
* source path
* 后续发版同步规程

这一批不要修改 crates。

---

# 四、W3：活性与完成语义——本轮最高优先级代码任务

### 已确认根因

当前：

```text
root agent
→ node.status 未正确进入 Completed
→ steps_without_progress / v22 gate 误判
→ replan
→ give_up
```

该问题已有决定性实机证据：

即使：

* 工具全部成功
* 产物正确
* 0 errors
* 三步实际完成

系统仍可能最终 `give_up / failed`。

因此 W3 正式采用：

> **D3 = C：显式 Completed + 事实级 Progress Signal，两者都做。**

### 要求

1. 找到 root agent 正常完成路径。
2. 完成任务节点时显式进入 `Completed`。
3. 活性判据不能只依赖 `node.status == Completed`。
4. 将“产生新事实”纳入 progress signal，但必须定义清楚：

   * successful tool result
   * meaningful artifact creation/modification
   * valid conclusion / task-state transition
     哪些属于 progress。
5. 不得把普通 `pwd / ls / 重复定位` 等无意义命令当成 progress。
6. replan / Reflect 路径与正常 task completion 必须保持不同语义。
7. 正常 all_done / v20 gate 重入不得重新累计 stall。
8. Reflect → Replan 的真实停滞仍必须能够触发 stall。

### RC31 顺带约束

按守门员增 4：

> `original_goal` 必须成为 W3 完成判定/验收锚之一。

但这不要求本轮实现完整 Goal Drift 系统。

只要求：

```text
Original Goal
+
Current Task Result
+
Completion Decision
```

之间具有可审计关系。

RC31 完整目标替换护栏留在 W8。

### 负面测试必须先红后绿

至少覆盖：

```text
TC-1 纯读任务
TC-2 纯读 + 写
TC-7 root agent progress
```

再补：

```text
正常 tool success
→
不会因 node.status 未置位而 give_up
```

以及：

```text
真正无进展
→
仍能触发 give_up/stall
```

### 必须做实机验证

Tier3 三步真实任务：

```text
create
→ run
→ verify
→ record
```

要求：

```text
真实完成
+
正确终态
+
无假失败
```

并至少再跑一组纯读任务。

---

# 五、W4：事实投影 / Completion Report

W4 独立于 W3，禁止混成一个大型重构。

目标：

> **后端产生什么事实，CLI 必须如实投影什么事实。**

至少解决：

### RC8

`Done.status` 不再被 CLI 丢掉。

### RC9

`give_up` 原因必须进入结构化完成结果，而不是只留 tracing。

### RC20

禁止继续使用：

```text
output.contains("error")
output.contains("failed")
```

作为最终成败判定。

优先使用工具层已有结构化退出码/错误状态。

特别要求：

```text
exit code != 0
→
结构化 error
```

而不是猜字符串。

### 结构化报告

扩展现有报告 schema，至少能表达：

```text
goal
status
terminal state
reason
steps
tool summary
artifacts
verification scope
remaining work
```

CLI 是 projection，不能自行重新猜一次事实。

### 终态必须区分至少：

```text
completed
verify_failed
give_up
error
timeout
cancelled
```

用户必须能够一眼知道：

> “它是完成了、失败了、主动放弃了、还是超时了。”

---

# 六、W5/W6/W7/W8/W10 本轮不得进入 diff

尤其：

```text
compaction
approval delegation
read scope
PATH collision
Goal Revision
resume exactly-once
filesystem accounting
bash stdout cap
resource hard control
```

本轮全部留在 backlog。

不要“顺便修”。

---

# 七、关于测试矩阵

守门员已经修正：

> 12/18 真 LLM 矩阵不得进入普通 CI。

普通 CI：

```text
llm-replay
```

真 LLM：

```text
手动触发
```

W3 可以把 replay 版固定进 CI，但不允许每次 gate 消耗 Agnes 配额。

---

# 八、关于“先红后绿”

每一个新测试必须：

```text
旧代码
→ 测试先失败
↓
修复
↓
测试转绿
```

禁止只写一个永远不会失败的“漂亮测试”。

门禁继续：

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

并以正确 VM / 正确 source tree 的实际 gate log 为准。

---

# 九、最终执行顺序

严格：

```text
W1 基线统一
      ↓
W3 活性/完成语义
      ↓
W3 真机验证
      ↓
W4 事实投影/结构化报告
      ↓
W4 真机验证
```

每个代码批次独立 commit、独立 gate。

禁止把 W3/W4/W5/W6/W7/W8/W10 混成一个 commit。

---

# 十、完成后只报告

```text
1. 当前 HEAD / commit
2. 两 VM source/binary provenance
3. W3 diff
4. W3 先红后绿证据
5. W3 unit/integration/VM gate
6. Tier3 长程实机结果
7. W4 diff
8. W4 tests
9. 终态/错误投影实机结果
10. 新发现
11. OPEN / UNKNOWN / DEFER
12. 是否建议进入下一批
```

最后重申：

> **本轮不是“把问题总账全部修完”。**

本轮只解决第一根因链的核心事实生产问题，以及第二根因链中最直接的事实投影问题。

目标是先把：

```text
Agent 做了什么
→
系统实际知道什么
→
用户最终看到什么
```

这条链打通。

完成后停止，等待下一批批准。

---

# 守门员批注（2026-08-29 · 首批切分同意，附 6 条执行细则）

> 切批裁决同意：W1/W3/W4 首批（链1+链2 核心）、W5-W8/W10 进 backlog、W9 历史化、密钥纪律、先红后绿——全部正确。以下 6 条为执行细则。

## 补充 1：W9 实证关闭（"当前状态核实"的结果 = 已闭环）

评审窗口定位的 4 处测试修正在 Closure Window **已落工作区（等价形式）**：①编译修复（TaskResult 结构体构造，门禁可编译）；②`do_plan_inner` 前置（`loop.rs:5490` 等）；③`original_goal` 补设（`:5489` "cb status"）；④错断言以等价方式解决（`:5509` 断言在 **387 全绿门禁**中通过——Continuity 输出含该短语的合法匹配，非原判定的"根本不在格式里"状态）。**W9 从待办划掉，历史项关闭**；执行窗口无需再做 R2-C 相关动作。

## 补充 2：W3 最易做错的点——置位生产者纪律（ChatGPT 未点破）

要求 2"完成任务节点时显式进入 Completed"——**生产者不能是 LLM 自报**（RC1 老教训：活性判据靠自报=没有判据）。推荐对齐 WS13：**写盘 verify 三重校验通过后标记**。且注意：planner 产出的节点与实际工具调用**没有天然映射**（哪个工具调用完成对应哪个节点是不存在的信息）——v1 落法建议：**run 级 verify 通过 → 相关节点统一置位**（粗粒度但诚实），节点级精细归因留后续专项，防止 W3 膨胀成"节点级归因研究"。

## 补充 3：W3 负面测试矩阵加一条更尖锐的反例

v0.2.8 L765-767 实测案例：**系统刚报 `✓ Task completed（2 步）——目标达成`，2 行后用户质问目标完全偏离**——"假完成 + 目标替换同帧出现"比 Tier3 三步案例更尖锐。列入 W3 负面测试（2 步假完成场景：产物与 original_goal 无关时不得 completed）。

## 补充 4：W4 的 RC20 修复锚点

RC20（英文子串匹配画错误状态）的确切位置 = `render.rs:86-92`——is_error 口径统一到工具层退出码后，渲染层直接消费结构化字段，顺带修掉"中文错误画绿✓"问题。W4 验收加一条：中文错误输出必须正确投影为失败。

## 补充 5：W1 的 vm-version-sync.md 写入已知坑

同步规程必须包含：**tar 打包排除 `target/`、`.git/`、`.workbuddy/`、`release/`**（保 VM 35G 增量编译缓存——历史血泪：全量覆盖会毁缓存重来）。

## 补充 6：密钥提交面澄清（防执行窗口误判扩大 D7 范围）

`d81087b` 入库的 `release/*.txt` 提交前做过**全量密钥扫描**（完整 key 0 命中，仅 8 字符前缀审计引用）——#0 泄漏主体是更早的 `059f160`（未推）。结论：①T-#0d 的 gitignore 仍建议做（防未来日志带 key 入库）；②已入库的 release/*.txt **不需从 HEAD 移除**（扫描为干净）；③是否纳入 D7 filter-repo 全量擦除，届时按其实际内容再定。执行窗口不要自行"预防性删除"已入库文件。
