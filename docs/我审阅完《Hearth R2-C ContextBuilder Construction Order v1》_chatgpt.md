我审阅完《Hearth R2-C ContextBuilder Construction Order v1》。

总体结论：**方向批准进入最终修订，但暂不签发施工批准。无需重新调查，也不要施工代码。只修订以下 6 点，然后重新提交 v1.1，提交后停在这里。**

### 1. 修正 Experience 空位逻辑——P0

当前 §六：

> `injected_experience: None 时 L2 留空位或以空段占位——保证字节偏移稳定`

这个逻辑不足以保证 system 字节稳定。

必须改成：

> **Experience 必须在 session 第一次 `build_messages()` 前完成确定。ContextBuilder 不允许在 system 已进入 cache 后从 empty → populated。**

明确规定：

* 首次 build 前完成 experience retrieval；
* 或明确该 session 永不注入；
* 禁止先空后填。

不要用“空槽位”把 system stability 问题掩盖掉。

### 2. 把 cache stability 拆成三层——P0

不要把：

> system_text 恒定

直接等同于：

> full prompt stable prefix 恒定。

施工单新增明确区分：

```text
A. System Stability
   system_hash unique == 1 / session

B. Message Prefix Stability
   后续 request 应保持既有 message 序列作为前缀，
   新增 history/tool result/observe/continuity 只出现在 suffix

C. Provider Cache Outcome
   token-level hit rate
   request-level hit rate
```

验收分别测量。

目标不是单纯“system_hash=1”，而是证明 ContextBuilder 确实改善了可复用 prefix。

### 3. 核实 `last_graph_sig` 的语义后才能复用——P0

当前 §四直接规定：

> `last_graph_sig` 复用为 topology 重建触发器。

必须补充 signature 输入字段契约。

只有当 `last_graph_sig` **仅由 topology identity 相关字段构成**（至少 id/description/deps，以及其他真正属于 topology 的字段）时，才允许直接复用。

如果当前 signature 混入：

* status
* result
* runtime state
* 其他动态字段

则不得复用，必须使用独立 `topology_sig`。

不要凭名称推断语义。

### 4. 修正“只移位置”的表述

§十：

> “语义内容不变（只移位置）”

改为：

> **执行所需事实集合不丢失，改变事实的分层表达与位置。**

因为 TaskGraph 已经发生：
`full Task Plan → topology + Continuity state`
这属于结构重组，不是单纯 relocation。

同时明确 `[DONE]/[FAILED]` 等逐节点状态如果被 completed/remaining/next_action 吸收，必须证明没有丢失执行所需事实。

### 5. 固化 L4 顺序的设计理由

保持：

```text
history
→ retrieval_context
→ LSP diagnostics
→ Task Continuity
```

但补充架构理由：

> **Task Continuity 是当前任务状态的最终语义锚点，因此位于本轮所有 observe/dynamic evidence 之后。**

这样避免未来施工时无依据调整顺序。

### 6. 收紧 rollback 条件

§十四改成分级：

**结构性 rollback：**

* gate failure
* system_hash 稳定性失败
* message prefix 结构违反设计
* 明确 Intent / completion 行为回归

**效果性 rollback candidate：**

* 同任务、同通道、同运行姿势下 cache 指标持续/显著退化

禁止把一次 cache 百分比波动直接作为 rollback。

---

除此之外，本施工单的总体架构保持不变：

* TaskGraph：方案 B
* topology → stable/task-stable
* state → Task Continuity
* Experience → L2
* L4 = history + retrieval + LSP + Continuity
* tool schema 全量稳定
* Continuity 唯一路径
* D 类事项全部隔离
* compact 不删除 Continuity
* v1 fallback 保留

**不要新增研究项，不要扩大范围，不要修改代码。**

修订为 `ContextBuilder Construction Order v1.1` 后停在这里，等待顶层最终批准。

---

# 守门员批注（2026-08-29 · 六点全同意；点 3/点 1 已源码裁决，v1.1 照抄即可）

## 裁决 1（点 3 当场落地）：`last_graph_sig` 实测**混入了 status——不得复用**

源码实测（`loop.rs:1762-1768`）：sig 的 hash 输入 = `id` + `description` + `deps` + **`format!("{:?}", n.status)`** ——ChatGPT 的担心完全命中：status 每步变化 → 复用为 topology 触发器 = topology 块每步重建 = MISS-A 复辟。v1.1 修法（两个 sig 各司其职，不冲突）：

```text
topology_sig（新增）：只 hash id/description/deps（即去掉 :1766 的 status 行）
  → 专用作 L2 Topology 块重建触发器（结构签名——结构不变就不重建）
last_graph_sig（保留现状，含 status）：继续作 T4 停滞检测
  → 语义签名就该含 status（replan 产出同图且同状态才算"原地踏步"）
```

## 裁决 2（点 1 落地位置确认）："先空后填"风险实测为真，重置点要一并处理

`injected_experience` 生命周期实测：字段 `loop.rs:516`、init None `:787`、填充 `:1997`、**重置 None `:1983` 与 `:3089`**——后两个重置点意味着"已填充→变回 None→再填充"的路径真实存在（不止首轮空槽问题）。v1.1 落法：experience 检索提前到**session 生命周期首次 build_messages 之前**完成（或显式决定永不注入），且**检索完成后 session 内禁止再重置**——`:1983/:3089` 两处重置点须豁免 experience 字段或改写语义，否则 ChatGPT 点 1 的修复会被这两个点绕过。

## 裁决 3（点 4 的具体检查项）：[FAILED] 节点事实保全

Task Plan 的逐节点状态标记含 `[FAILED]`，而 Continuity 块的 completed/remaining/next_action 三段**不必然表达"哪个节点失败 + 失败结果摘要"**——这正是点 4"证明没有丢失执行所需事实"的具体检查项。v1.1 必须明确：FAILED 节点在 Continuity 块的表达方式（建议增加一行 failed 节点列表或吸收其 result 摘要），并加单测锁定（构造含失败节点的 graph，断言迁移前后 failed 事实均对模型可见）。

## 裁决 4（点 2 的采集器落点合并）

B 层（Message Prefix Stability）验收需要**逐消息 hash 链**——正是 R2-A 采集器 v2 改进项里已列的"相邻请求公共前缀长度"。本施工单第 12 条验收时一并落：一个采集器同时服务 A（system_hash，已有）与 B（prefix 链，新增）两层测量，不另起炉灶。

## 其余

点 1 方向、点 2 三层拆分、点 4 表述修正、点 5 顺序理由、点 6 分级 rollback——全部同意，无补充。v1.1 按六点修订 + 本批注四条裁决产出后，直接进顶层最终批准。
