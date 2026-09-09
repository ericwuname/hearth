收到《Hearth W8 验收报告 v1》及 N-1 诊断报告。

顶层复核结论：

# W8 正式验收通过，关闭施工窗口。

## 一、W8 结论

W8 主项达到验收标准：

* A1 任务类型路由：通过
* A2 Goal Revision 三分类：通过
* A3 RC26 计划块去重：通过
* A4 goal_drift observe-only：通过
* A5 resume exactly-once 补测：通过
* N-2 one-shot cancelled projection：通过
* 门禁 407 passed / 0 failed：通过
* V-1 十论述任务 10/10 completed：通过

特别确认：

> DEV-1 的核心问题已经获得足够强的行为证据支持：
> 论述/问答型任务不应进入当前 TaskGraph → Reflect → Replan 闭环。

因此 W8 主项正式 CLOSED。

---

## 二、OPEN-W8-1 保留，但不重开 W8

当前发现：

> “不要修改任何现有文件” + 真实产物意图
> → 被负向短路误判为 QA。

这是一个真实边界缺口，但不是 W8 主项回归。

原因：

* 分类结果与旧版本一致；
* 新版本并未制造该问题；
* T-A 的主要任务路由能力已经通过；
* 问题属于“constraint signal 与 task type classification 混淆”。

正式记录：

`OPEN-W8-1`

下一步原则：

> **负向约束不能单独决定 task type。**

未来设计分类器时必须区分：

```text
Task Type
+
Constraints
```

例如：

```text
“写 README，但不要修改现有文件”
=
Product Task
+
Negative Constraint
```

不要继续用关键词堆叠无限修规则。

当前不施工。

---

## 三、N-1 正式进入独立 G0 Sandbox 修复

诊断结论认可：

> Landlock `EINVAL` 的根因不是 char device，而是 `FS_RW` 混入 directory-only rights。

`/dev/null + FS_FILE_ONLY = OK`
以及
`regular file + FS_RW = EINVAL`
已经构成决定性矩阵证据。

因此批准：

### N1-SBX 独立施工单

目标：

```text
FS_RW
=
目录可用权限集合

FS_FILE_ONLY
=
非目录可用权限集合
```

非目录路径不得使用 directory-only rights。

至少覆盖：

```text
/dev/null
regular file
directory
```

并锁定非法 rights/object type 组合的负面测试。

### 真机验收必须证明两层：

1. `landlock_add_rule()` 成功；
2. 实际 `/dev/null` 写入成功。

只证明“不再 EINVAL”不够。

同时验证：

* 普通文件规则仍正常；
* 目录规则不回归；
* G0 fail-closed 语义不改变。

**禁止重新修改 RC24 approval semantics。**

---

## 四、N-2 正式 CLOSED

one-shot：

```text
Ctrl-C
↓
⏹ 本轮已取消
```

真机已验证。

不再进入 W8，也不另开长期 backlog。

---

## 五、N-3 DEFER

Markdown report 缺 `approval_delegated` 消费仍保留，但不阻塞主线。

后续 report/projection 整理时统一处理。

---

## 六、DEV-2 保持独立

当前已有重复证据：

```text
工具成功
+
0 errors
+
progress fact 存在
+
artifact 成立
↓
Reflect LLM
↓
give_up
```

这一问题暂不修。

不要继续往 ContextBuilder 里塞 prompt。

后续独立研究：

> **Fact / Verification / Reflect 冲突时，谁具有最终否决权？**

候选方向先观察：

```text
Fact
+
Verifier
+
Reflect
=
冲突时再验证
```

但本轮禁止改变此控制流。

---

## 七、当前正式状态

```text
R2-C ContextBuilder       ✅ CLOSED
W3/W4                    ✅ CLOSED
RC24                     ✅ CLOSED
W8 Goal Routing           ✅ CLOSED

OPEN:
N-1 Sandbox Landlock      🔴 G0
TaskGraph 活性/事实       🔴 高优先
DEV-2 Reflect quality     🟠 独立研究
OPEN-W8-1                 🟡 分类边界
N-3                       ⚪ DEFER

DEFER:
Subagent
TUI
MCP
Bridge
```

---

## 八、下一步

请不要继续施工 W8。

下一任务：

**先创建《N1-SBX Landlock File Rights 修复施工单 v1》并停在设计阶段。**

施工单至少包含：

1. 当前 `FS_RW/FS_RO` 真实 rights 集
2. Landlock object type / rights 适用关系
3. `FS_FILE_ONLY` 定义
4. `add_landlock_rule()` 的目标类型处理方式
5. `/dev/null`、普通文件、目录测试矩阵
6. 负面测试
7. 真机 kernel 7.0.0-30 验证
8. fail-closed 不变量
9. 不影响 RC24
10. 回滚方案

**暂不写代码。**

同时不要修改：

```text
ContextBuilder
TaskGraph
Goal Revision
Reflect decision
ApprovalPolicy
```

N1-SBX 施工单提交后停止，等待顶层批准。

---

## 守门员补充（2026-08-29 23:55 · 复核实证后追加，与正文同效力）

> 复核记录：W8 三 commit（`6ca3938`/`db15d37`/`d5ecebb`）在库；W8 四函数锚点实锤（`goal_requires_product` loop.rs:167、`classify_user_input` :303、`check_goal_drift` :974、`apply_turn_goal` :1297）；**门禁三份全部找到并实证**——407（`t_gate_w8ta.log`）/399（`t_gate_w8tb.log`）/399（`t_gate_rc24.log`），failed 全 0，但都落在 **.133**（.131 的 `t_gate_r2c.log` 停在 391）。诊断矩阵与内核 man page 原文一致，`FS_RW` 定义（lib.rs:255-268 含 REMOVE_DIR/MAKE_*/REFER/TRUNCATE）与"目录专有位混入"结论吻合。**W8 验收与 N-1 根因均确认**。以下 8 条为设计单约束。

### 增 1（最重要）：FS_RO 同病，菜单 0 只修 FS_RW 是修一半
`FS_RO = READ_FILE | READ_DIR | EXECUTE`（lib.rs:251）——**`READ_DIR` 同样是目录专有位**。`:382` 的 read_only_paths 循环（`add_landlock_rule(…, FS_RO)`）遇到普通文件目标时**同样静默 EINVAL**。当前默认配置 read_only_paths 全是目录（`/` 等）无实际炸点，但机制上与 writable_paths 同族。设计单必须同时给 FS_RO 一个文件变体（`FS_RO & FILE_MASK = READ_FILE | EXECUTE`），不能只做 FS_RW 侧。

### 增 2：FILE_MASK 动态掩出，禁止手写常量集
`FILE_MASK = EXECUTE | WRITE_FILE | READ_FILE | TRUNCATE`（恰为内核"适用于文件"的四位全集；**REFER 也是目录专有**，诊断的 FS_FILE_ONLY 恰好不含它，但报告没点名——设计单必须把 REFER 列入 DIR_ONLY 清单防未来误留）。实现推荐 `FS_FILE_ONLY = FS_RW & FILE_MASK`（const 数学 + 静态断言 `FS_FILE_ONLY & DIR_ONLY_MASK == 0`），未来 ABI v6+ 新位进 FS_RW 时自动正确；手写四常量会在下一次权限集演进时静默漏位。

### 增 3：条款 4 钉死为 fstat 类型感知，调用点零改动
`add_landlock_rule()` 内部 `fstat(parent_fd)` → `S_ISDIR ? 全量集 : 文件集`，调用点 `:377/:382/:391` **一行不改**。比"每个调用点记得选对集合"稳健——顺带根治诊断指出的同族隐患（writable_paths 未来含常规文件的静默 EINVAL），不需要 `/dev/null` 特判。

### 增 4：纯函数单测先红后绿（平台无关，本地可跑）
权限集数学断言（`FS_FILE_ONLY & DIR_ONLY_MASK == 0`、`/dev/null` 实际用的集 & DIR_ONLY == 0、FS_RO 文件变体同理）**不依赖内核，Windows 本地即可先红后绿**；内核矩阵 C1-C5 转 VM 真机脚本。**`/tmp/ll_diag.c` 在 VM /tmp 重启即丢——先归档进 `docs/data/n1-landlock/`**（诊断报告附录补 C 源码），证据资产化，复跑不重写。

### 增 5：真机两层验收之外，回填总账两处
① TC-8/TC-9 转绿后回填 consolidated-remediation-ledger；② **总账 T6 条目勘误**——"T6 已落地/已验收"的历史记载必须改写为"T6 自 v0.2.3 起从未生效，被旧审批门遮挡至 RC24 才暴露"（与 f0b4a54 假绿同族：历史"已验收"记载与实证不符，必须留痕）。

### 增 6：门禁日志登记约定（本次教训）
本轮守门员花了 4 次探测才在 .133 找到 407/399/399——.131 的 r2c log 停在 391，差点误判假绿。从本单起：**执行报告的"门禁"节必须写明每份 gate log 的完整 VM 路径**（host + path），守门员按路径直取。批次名入文件名（`t_gate_w8ta.log` 这个格式很好，保持）。

### 增 7：fail-closed 语义显式声明
修复**不改变** add 失败时的 warn+skip 行为（skip 后 restrict_self 全拒 = fail-closed 保持）。设计单要写明："本单只是让 add 成功，失败语义分毫不动"——防止执行窗口顺手"加固"成硬失败（那是行为变更，需另行批准）。

### 增 8：验收后顺路两件
① `.131` 源码树同步（vm-version-sync，W8 报告 OPEN 表已列——N1-SBX 合入后一次做掉，避免孤儿状态）；② VM config 现为 agnes（RC24 验收时恢复），本单真机验证用不上 LLM，不烧配额。
