收到 RC24《审批门施工验收报告 v1》及 W3/W4 回归、R2-C Closure 全部证据。顶层复核完成，现正式裁决如下。

## 一、RC24 正式验收

**RC24 APPROVED，施工收官，不回退，不追加施工。**

本单授权范围内：

* BashRisk 三级语义分类：通过
* redirect_targets 全形态识别：通过
* `/dev/null` 与其他 `/dev/*` / `/proc` / `/sys` 边界：通过
* 非交互审批结构化拒绝：通过
* `--approve-within session`：通过
* REPL `trust on/off`：通过
* CommandTable 可委托 / HardRedline 永不委托：通过
* 审计链：通过
* 399 passed / 0 failed / 1 ignored：通过

特别确认：**N-1 不构成 RC24 失败。**

`>/dev/null` 已经在审批判定层正确放行，随后才暴露 Landlock `EINVAL`。这是新的 G0/Sandbox 问题，而不是 RC24 regression。

## 二、立即停住 RC24

从现在开始，**不要再扩大 RC24 的测试面，也不要为了消化 OPEN 项而继续修改 RC24。**

禁止借 RC24 顺手修改：

* ContextBuilder
* TaskGraph 事实模型
* Goal Revision
* bash stdout / fs accounting
* Resource 控制流
* phase pruning
* bridge / subagent
* TUI
* 其他 D 类冻结区域

RC24 到此形成稳定基线。

## 三、遗留项正式分流

### N-1：G0 / Sandbox，独立排单

`/dev/null` 在 sandbox 内不可写，T6 Landlock rule 添加出现 `EINVAL`。

处理原则：

**只查 Sandbox/Landlock。不要重新动 RC24 approval gate。**

---

### N-2：P1 / 小型 hotfix

one-shot `hearth chat` 在 Ctrl-C 后缺少用户可见的 cancelled 终态投影，而 REPL 已正常。

这是明确的 projection correctness 缺口。

允许单独做一个极小 hotfix：

> one-shot cancelled 分支补齐终态投影。

不得借此扩展 cancellation architecture。

---

### N-3：P3 / Deferred

Markdown report 尚未消费 `approval_delegated` 字段。

JSON summary、CLI `[delegated]`、tracing audit 均已存在，因此这是 report projection 完整性问题，不阻塞主线。

暂不施工。

---

### DEV-2：保持独立 OPEN

`write 成功 / 0 errors / 事实 progress 已存在 → reflect LLM 仍 give_up`

这是 Reflect decision-quality 问题，不属于 RC24。

**不要并入 RC24 修。**

---

### DEV-1：进入 W8

论述/问答型任务进入 TaskGraph → Reflect → Replan 循环的问题，正式归入：

**W8 Goal Revision / Task Type Routing**

这已经有足够证据，不需要继续用 RC24 做实验。

## 四、下一主线

正式结束 RC24 后：

> **下一主施工单：W8 Goal Revision / Task Type Routing。**

目标不是继续给 Reflect 打补丁，而是解决更上游的问题：

**什么任务应该进入 TaskGraph 闭环，什么任务应该走直接回答/不同执行路径。**

尤其关注：

* 论述/问答类无 artifact 任务
* 有明确产物的执行类任务
* goal_requires_product 的正向分类
* planner 是否应该对不同任务类型采用不同路由
* Reflect 的输入事实与最终决策之间的边界

但 W8 施工前，先按既定流程出施工单，不要直接改代码。

## 五、关于 N-2

N-2 不必等待 W8。

如果你认为维护成本最低，可以先作为独立 hotfix 处理；但必须保持单独 commit、单独 gate、单独证据，不得混入 W8。

## 六、最终状态

请将本窗口状态记录为：

**RC24 = FINAL ACCEPTED / CLOSED**

遗留项：

* N-1 → G0 Sandbox
* N-2 → P1 Hotfix
* N-3 → Deferred
* DEV-2 → 独立 D 类问题
* DEV-1 → W8

**本窗口停止施工。下一步等待 W8 施工单。**
