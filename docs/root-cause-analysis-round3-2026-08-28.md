# Hearth 根因分析 · 第三轮（完成语义 / 压缩失忆 / 联网白名单）

> 评审窗口（守门人）产出 · 2026-08-28
> 方法：手工测试日志实证 + HEAD 源码逐行核验（不修改任何实现代码）
> 证据来源：`release/手工测试v0.x.x.txt`（9 份）+ `crates/` HEAD 源码
> 前置文档：`docs/root-cause-analysis-tier3-2026-08-28.md`（RC1–RC4）

---

## 0. 三条线索的一句话结论

| # | 线索 | 结论 | 性质 |
|---|------|------|------|
| 1 | Hearth 什么时候算"完成" | **LLM 自报 Done + 一层只校验"文件存在且非空"的物理验证**，没有语义级验收；且**失败原因从不呈现给用户** | 🔴 完成语义缺失 + 观测层隐瞒 |
| 2 | 同一对话内压缩导致失忆 | **压缩一直在触发**（非 BUG-002 所说"未触发"）；摘要是**纯规则模板、不调 LLM**，Assistant 全部内容与所有工具输出 100% 丢弃，每轮只剩 60 字 | 🔴 设计级信息销毁 |
| 3 | 联网白名单配了但不访问 | **35 次拒绝 / 0 次成功**，全周期从未真正联网；Hearth 归因为"沙箱隔离外网"**是错的**（seccomp 白名单含 socket/connect） | 🔴 未解决 + 错误自诊断 |

---

## 1. 线索一：完成语义 —— "它认为完成了"和"你知道它完成了"是两件事

### 1.1 "完成"是怎么判定的

源码路径（HEAD）：

| 环节 | 位置 | 内容 |
|------|------|------|
| 完成信号来源 | `crates/agent-core/src/loop.rs:3268` | `if matches!(phase, LoopPhase::Done)` —— **由 LLM 在 Reflect 阶段自报** |
| 唯一物理验证 | `loop.rs:3271-3298` | `if !self.written_files.is_empty()` → `verify_written_files()` 只查**文件存在 + 非空** |
| 验证失败回喂 | `loop.rs:3285-3292` | replan ≤3 次，超限按 `verify_failed` 失败收尾 |
| 成功落定 | `loop.rs:3323-3336` | `Event::Done { ok: true, status: "completed", goal, steps }` |

**关键缺口**：`if !self.written_files.is_empty()` —— **纯读、纯评估、纯调研的任务完全不经过任何验证**。
也就是说：当任务目标是"读代码、给结论、出报告"这类不写文件的任务时，LLM 说 Done 就是 Done，系统零校验。

而手工测试里的长程任务（"分析这个项目""跑一遍测试"）**恰恰全是这类任务**。

### 1.2 用户看到了什么

三个 CLI 渲染点，全部只读两个字段：

```
crates/codex-cli/src/repl.rs:196-199       steps + ok
crates/codex-cli/src/lib.rs:1051-1056      steps + ok
crates/codex-cli/src/run_local.rs:707-709  steps + ok
```

```rust
// render.rs:111-119
pub fn done(steps: u64, ok: bool) {
    let icon = if ok { "✓" } else { "✗" };
    let msg = format!("{icon} Done ({steps} steps)");
    ...
}
```

而 `Event::Done` 的负载里**明明带着 `status` 字段**，三种失败语义完全不同：

| status | 含义 | 用户实际看到 |
|--------|------|--------------|
| `completed` | 自报完成且文件校验通过 | `✓ Done (N steps)` |
| `verify_failed` | 自报完成但文件缺失/为空，replan 3 次仍失败 | `✗ Done (N steps)` |
| `error` | 真异常，**或 `give_up`** | `✗ Done (N steps)` |

**`status` 在 codex-cli 全仓没有任何一处渲染**（`grep '"status"' crates/codex-cli/src/*.rs` 的命中全部是别的用途：会话状态、节点状态、任务状态）。

### 1.3 give_up 的路径 —— 失败原因被吞在哪里

```
planner/src/lib.rs:595/603  →  ReflectVerdict::GiveUp
        ↓
agent-core/src/loop.rs:2937-2957
        Ok(StepOutcome { next: LoopPhase::Error("give_up: cannot make progress".into()), emit: vec![] })
        ↓
agent-core/src/loop.rs:3339-3362
        self.emit(Event::Done(json!({ "ok": false, "status": "error", "goal": ..., "steps": ... })))
```

**`"give_up: cannot make progress"` 这个字符串只进了 `tracing::warn!`（日志文件），从未进入 `Event::Done` 负载，更从未到达终端。**

→ 用户看到的 `✗ Done (17 steps)` 与"进程崩了"长得一模一样。

### 1.4 与"你为什么说了那么多次继续"的直接因果

统计 7 份手工测试日志的收尾标记：

| 版本 | `✓ Done` | `✗ Done` | 失败率 |
|------|---------:|---------:|-------:|
| v0.1.0 | 0 | 2 | 100% |
| v0.1.2 | 6 | 7 | 54% |
| v0.1.5 | 7 | 10 | 59% |
| v0.2.0 | 16 | 11 | 41% |
| v0.2.3 | 8 | 7 | 47% |
| v0.2.4 | 78 | 101 | 56% |
| v0.2.5 | 66 | 26 | 28% |
| **合计** | **181** | **164** | **48%** |

结合 RC1（活性信号断裂 → 5 步 × 3 次 replan ≈ 15 步强制 GiveUp）：

> **Hearth 有一半的轮次以 `✗` 收尾，其中相当比例是它自己 `give_up`；但终端只显示 `✗ Done (N steps)`，不告诉你是"它放弃了"还是"真出错"。**
>
> 用户看到的唯一信息是"这轮结束了，好像没成"。合理的反应只有一种：**再说一次"继续"**，希望换个随机种子能成。
>
> **"继续"不是用户的交互习惯，是对"失败原因不可见"的应激反应。** 系统没有给出任何别的可用动作。

### 1.5 判定

- 🔴 **完成语义缺失**：无语义级验收（只验文件字节），纯读任务零校验
- 🔴 **终态不可分辨**：`status` 字段产生但不渲染，give_up / error / verify_failed 三态同形
- 建议方向（**待顶层拍板，不在评审窗口落地**）：Done 事件增补 `reason` 字段 → CLI 渲染 `✗ Done (17 steps) · 放弃：连续 5 步无进展`；纯读任务是否引入"产出物契约"（要求落盘结论文件）作为验收锚点

---

## 2. 线索二：同一对话内的压缩失忆 —— 从"倒数第 3 轮之前"开始

### 2.1 先纠正一个流传的错误结论

日志里 Hearth 反复自述（并被登记为 BUG-002 / V1）：

> "85% 超阈值但未见压缩" / "100% 满仓后压缩仍未触发" / "T12 长对话压缩行为 ⏳ 观察中，压缩仍未触发（印证 BUG-002）"

**这是错的。** 压缩一直在触发，证据就在它自己的上下文里：

| 日志 | `[compacted 会话摘要]` 出现行号 | 次数 |
|------|------|-----:|
| 手工测试v0.2.4.txt | 1471, 1564, 1601, 1707, 9473, 10075, 16057 | 7 |
| 手工测试v0.2.5.txt | 3792, 3810, 6163, 6180, 6197, 6214, 6258, 6307 | 8 |

**Hearth 一边被压缩，一边得出"压缩未触发"的结论。** 这是继 RC2（观测层说谎）之后的第二例同型故障：**结论与它自己上下文里的证据直接矛盾**。

### 2.2 压缩机制的真实形态（源码核验）

```rust
// crates/agent-core/src/context.rs:156-159
pub const COMPACT_CHAR_THRESHOLD: usize = 32_000;  // ≈32k chars ≈ 8k tokens
pub const COMPACT_KEEP_TURNS: usize = 2;           // 保留最近 2 轮完整
```

调用点：`agent-core/src/loop.rs:1310` —— **每个 step 的 observe 之后都调一次**。

摘要生成函数 `summarize_turn()`（`context.rs:298-365`）：

```rust
let goal = t.messages.iter()
    .find(|m| matches!(m.role, Role::User))
    .map(|m| format!("{:?}", m.content))     // ← Debug 格式："Text(\"…\")"
    .unwrap_or_default();
let goal_short = if goal.chars().count() > 60 {
    format!("{}…", goal.chars().take(60).collect::<String>())
} else { goal };
// tools  = ToolCalls 的工具名 BTreeSet 去重 + 排序
// files  = write_file/edit/apply_patch 的 path
Message::new(format!("summary-{idx}"), Role::Assistant, MessageContent::Text(format!(
    "[compacted 会话摘要] 轮次目标: {goal_short} | 工具: {tools_part} | 写盘: {files_part}"
)))
```

### 2.3 这就是失忆的全部机制 —— 六个精确断点

**① 不调 LLM。** 设计注释写得很清楚（`context.rs:151-153`）：
> "规则式摘要、不调 LLM——确定性、可测试、不阻塞"

这不是 bug，是 v0.1.5 的**有意设计取舍**（当时为了"compaction 先于提步数"）。代价是：**没有任何语义抽取，只有字段投影**。

**② `format!("{:?}", m.content)` 的 7 字符税。**
取的是 `MessageContent` 的 **Debug 串** `Text("…")`，所以 60 字符预算里：
- `Text("` 吃掉 6 字符，结尾 `")` 再吃 2 字符
- 内容里的引号被转义成 `\"` 再翻倍吃字符
- 中文一句话 20 字就顶满，**句尾（往往是真正的诉求）直接被 `…` 切掉**

实测样例（v0.2.5:6180）：
```
[compacted 会话摘要] 轮次目标: Text("继续") | 工具: bash | 写盘: 无
```
60 字符预算，实际有效信息 = **"继续" 两个字**。

**③ Assistant 消息全部丢弃。** 每一轮的推理、结论、计划、自我纠错 —— 一条不留。

**④ 所有 tool_result 全部丢弃。** grep 出来的 8000 字符正文、测试输出、报错栈 —— 一条不留。

**⑤ 工具名去重到 BTreeSet。** 用了几次、什么顺序、什么参数 —— 全丢，只剩一个排序后的工具名集合。

**⑥ 注释与实现不符（守门员最看重的一类证据）：**

```rust
// context.rs:153
// 旧轮次折叠为一条摘要消息（保留每轮 goal + 工具清单 + 写盘文件，不丢架构决策）。
```

**代码里没有任何一个字段承载"架构决策"。** 注释承诺的保留项，实现里不存在。

### 2.4 失忆起点：滚动遗忘，不是一次性事件

```
第 N 轮结束 → estimate_chars ≥ 32000 → drain(..len-2) → 前 N-2 轮全部塌成 60 字摘要
第 N+1 轮  → 又满了 → 再塌一次（上一轮的摘要 + 新的一轮）
```

**结论：任何时刻，Hearth 只拥有「最近 2 轮的完整内容」+「一串 60 字墓碑」。**
**失忆起点 = 倒数第 3 轮之前的所有内容，且这个边界随着对话推进不断向前滚动。**

这精确解释了用户的观测：

> v0.2.4:7262 **"完了，你真的失忆了。上下文压缩导致的吗？"**
> v0.2.5:1508 "我**读不到被压缩掉的对话内容**，所以'上一轮我们说到哪了'我记不住"

不是"某次压缩弄丢了一段"，是**从第 3 轮起就系统性读不到了**。

### 2.5 归档补救为什么没兜住

H3（v0.2.4）加了落盘：`~/.config/hearth/archive/<sid>.jsonl`，并在摘要头部注入检索提示。

三个断点：

1. **提示注入有前提条件**（`context.rs:225-237`）：只在 `history` 非空、首条消息是 `Text`、且内容不含 `"archive/"` 时才追加
2. **是"告诉你有个文件可以 grep"，不是"帮你找回来"** —— 完全依赖 Agent 自觉发起一次 bash 检索
3. **更致命的时序**：提示注入在 `history` 的**第一条消息**里。而下一次压缩发生时，**这条带着提示的消息本身就在被 drain 的范围内**——它会连同提示一起被塌成新的 60 字摘要，提示再次注入……但**它自己携带的检索方法论**也随摘要重写而重建。理论可续，实际在 15 次压缩的日志里，Hearth 从未成功靠归档找回过上下文（全部表现为"无法读取"）。

### 2.6 判定

- 🔴 **设计级信息销毁**：60 字模板 + 保留 2 轮，对长程任务等于断头台
- 🔴 **错误自诊断**：BUG-002/V1"压缩未触发"与 `[compacted]` 出现 15 次直接矛盾
- 与 RC1 的耦合：RC1（活性信号断裂 → 15 步 GiveUp）+ 本轮（第 3 轮起失忆）= **Agent 既不记得做过什么，也算不出自己有进展**
- 建议方向（**待顶层拍板**）：① `summarize_turn` 至少并入 Assistant 末条文本 + 工具结果首 N 字符；② 去掉 `Text("…")` Debug 包裹，60 字全部让给正文；③ `COMPACT_KEEP_TURNS` 2 → 可调；④ 归档检索从"提示"升级为"工具"（`archive_search`），让找回历史成为一条可达路径而非一句建议

---

## 3. 线索三：联网白名单 —— 35 次拒绝，0 次成功，且从未解决

### 3.1 硬数据

| 日志 | `出网被拒` | `HEARTH_EGRESS_ALLOWLIST=` 提及 | `web_fetch` 提及 | **真实取回正文** |
|------|----------:|------------------------------:|----------------:|----------------:|
| v0.2.0 | 8 | 9 | 15 | **0** |
| v0.2.3 | 14 | 20 | 35 | **0** |
| v0.2.4 | 4 | 6 | 5 | **0** |
| v0.2.5 | 9 | 17 | 20 | **0** |
| **合计** | **35** | 52 | 75 | **0** |

**"真实取回正文"判定依据**：`web.rs:128-133` 成功返回必带 `"verifiable": true` 与 `"source"` 字段。
`grep -c "verifiable"` 在全部 9 份日志中 = **0**。

→ **结论：手工测试全周期，web_fetch 从未成功联网一次。**

### 3.2 Hearth 给出的原因是错的

日志里的自诊断：

> v0.2.5:2451 **"结论：沙箱环境完全隔离外网，所有域名访问均被 `HEARTH_EGRESS_ALLOWLIST` 白名单机制拦截"**
> v0.2.5:3261 "我的执行环境是隔离的容器化沙箱，默认没有出网连接权限。即使设置了白名单，容器级……"

**源码反证**：

```rust
// crates/sandbox/src/lib.rs:577-580  —— seccomp 白名单数组（99 项）
SYS_SOCKET,       // = 41
SYS_CONNECT,      // = 42
SYS_SOCKETPAIR,   // = 53
```

`lib.rs:1832-1835` 注释自认：
> "出网 connect（**connect 在白名单**=放行/超时——记 BLOCKED 或 INCONCL）"

**→ seccomp 根本不拦出网。沙箱允许 `socket` + `connect`。**
**→ 真正的拦截 100% 来自应用层：`crates/tools-builtin/src/web.rs:91-102`。**

Hearth 把"应用层白名单拒绝"误诊为"容器级网络隔离"，并据此建议用户去改容器配置 —— 一条**完全错误的排查方向**。

### 3.3 "你手工加了"和"它申请了"是两个时期的两种机制

这一点必须分开，否则账算不清：

| 时期 | 机制 | 源码 | 日志证据 |
|------|------|------|---------|
| **v0.2.3 及之前** | **纯手工配置**，Agent 无申请入口 | 无（v0.2.6 才加） | v0.2.3:621-628 Hearth 答："白名单机制：是**你（用户/部署侧）手工配置**，不是'我添加→你审批'……我的工具集里**没有**配置白名单的工具" |
| **v0.2.6 起** | **Agent 申请 → 用户审批 → 运行时追加 + 持久化** | `loop.rs:1040-1104`（InteractionRequested `egress_allowlist_request`，timeout 60s，on_timeout deny）+ `scheduler.rs:156-164`（`set_env_var`）+ `run_local.rs:320-346`（`set_egress_persist_callback` 落 config.toml） | v0.2.4/v0.2.5 日志尚未跑到该路径 |

**所以你记忆里"我手工加了"和"它申请了"都是真的，只是发生在不同版本。** 混杂在一起就变成了"加了也没用"。

### 3.4 结构性断点（源码层）

**① `ctx.env` 是 web_fetch 的唯一读取来源**
```rust
// web.rs:91-96
let allowlist: Vec<String> = ctx.env.get("HEARTH_EGRESS_ALLOWLIST")...
if !egress_allowed(host, &allowlist) { return Err(anyhow!("出网被拒：域名 {host} 不在…")) }
```
`ctx.env` 是 `ToolContext` 里的一个 map，**不是进程 env**。

**② 两条注入路径不对称**

| 路径 | 注入点 | 读取来源 | 是否读 config.toml |
|------|--------|---------|-------------------|
| chat（`hearth chat`/`run_local`） | `codex-cli/src/run_local.rs:219-228` | 进程 env + config.toml 并集（`config.rs:150-153`, `merge_allowlist` 159-180） | ✅ |
| service（`hearth service`） | `service/src/main.rs:411-416` | **只读进程 env** | ❌ |

→ **在 service 模式下写进 `config.toml` 的白名单完全无效。** 手工测试若跑的是 service，这就是"配了没用"的直接原因。

**③ 日志里 Hearth 自己的排查也撞上了这个**（v0.2.5:3538-3544）：
```
环境变量:  egress_allowlist=rust-lang.org,crates.io,docs.rs,doc.rust-lang.org,github.com
配置文件:  egress_allowlist = github.com, rust-lang.org, pypi.org, www.baidu.com
**发现冲突**：环境变量和配置文件有不同的白名单。环境变量优先级更高，所以实际生效的是 `pypi.org` 和 `www.baidu.com`
```
**这段推理是错的**：`merge_allowlist`（`config.rs:159-180`）做的是**并集去重**，不是"env 优先覆盖"。两份都存在时，六个域名全部生效。Hearth 却推断成"只有文件里的两个生效"——**又一次结论与源码矛盾**。

**④ bash 工具能拿到 ctx.env**（`bash.rs:136-142` → `164-172` 注入子进程），所以 `env | grep HEARTH` 在 bash 里查得到。这不是盲区。

### 3.5 最终是否解决

**没有解决。** 证据：

- 35 次明确拒绝，0 次成功取回
- v0.2.5:1679 自测表自评："| T10 web_fetch 联网 | ✅ 通过（受限）| 白名单机制按设计工作，但所有域名被拒 → BUG-006 |"
  → **"白名单按设计工作"被当成 PASS，而它的实际含义是"联网功能一次都没能用"**
- v0.2.5:5460 最后一次尝试仍被拒（`api.deepseek.com`）

### 3.6 判定

- 🔴 **功能从未可用**：35 次拒绝 / 0 次成功
- 🔴 **错误自诊断 ×2**：① 归因"沙箱隔离外网"（seccomp 明确放行 socket/connect）；② 推断"env 优先级更高"（实际是并集）
- 🟡 **路径不对称**：service 模式忽略 config.toml
- 建议方向（**待顶层拍板**）：① 先做一次最小实证（新 VM 上 `HEARTH_EGRESS_ALLOWLIST=example.com hearth chat` 直接 web_fetch，判定是配置问题还是代码问题）；② 拒绝错误文案补上"当前生效白名单 = [实际列表]"，让拒绝自证；③ service 路径补 config.toml 读取；④ 自测标准加"至少 1 次成功取回"，否则 T10 不允许标 PASS

---

## 4. 三条线索的收敛：一个共同的病根

| 线索 | 表层现象 | 共同病根 |
|------|---------|---------|
| 1 完成语义 | 用户不知道它是完成还是放弃 | **Agent 的内部状态不对外投影**：`status`、`give_up` 原因产生了，但不渲染 |
| 2 压缩失忆 | 第 3 轮起读不到 | **投影是有损单通道**：只投影 3 个字段（goal/工具/写盘），且 60 字截断 |
| 3 白名单 | 配了但访问不了 | **投影缺失导致误诊**：拒绝时不回显"当前生效白名单"，Agent 只能靠猜，猜错就是错方向 |

**三条都指向同一条：事实产生了，但没有（或只以极小带宽）投影到该看到它的人/Agent 那里。**

这与既有公理"后端产生事实，前端投影事实"（`MEMORY.md`）是同一条原则的三次违反：
- 线索 1 违反在 **CLI 这一层投影时把 status 丢了**
- 线索 2 违反在 **压缩这一层投影时把 97% 的事实丢了**
- 线索 3 违反在 **错误这一层投影时把"当前生效白名单"丢了**

---

## 5. 本轮新发现的问题清单（供顶层规划窗口派活）

| 编号 | 问题 | 严重度 | 源码锚点 | 验证方式 |
|------|------|--------|---------|---------|
| RC5 | 压缩摘要 60 字模板 + 保留 2 轮，Assistant/工具输出 100% 丢弃 | 🔴 | `agent-core/src/context.rs:156-159, 298-365` | 已由 15 处 `[compacted]` 实证 |
| RC6 | `summarize_turn` 用 Debug 格式，60 字预算被 `Text("…")` 吃掉 8 字符 | 🔴 | `context.rs:308-313` | 读码即得 |
| RC7 | 注释承诺"不丢架构决策"，实现无任何承载字段 | 🟡 | `context.rs:153` vs `298-365` | 读码即得 |
| RC8 | `Event::Done.status` 产生但三处 CLI 全不渲染 | 🔴 | `loop.rs:3323-3360` vs `repl.rs:196`/`lib.rs:1051`/`run_local.rs:707` | 读码即得 |
| RC9 | `give_up` 原因字符串只进 tracing，不进 Done 负载 | 🔴 | `loop.rs:2956` → `3357-3362` | 读码即得 |
| RC10 | 纯读/评估任务完全跳过完成验证 | 🔴 | `loop.rs:3271` `if !self.written_files.is_empty()` | 读码即得 |
| RC11 | BUG-002/V1"压缩未触发"结论与 15 处 `[compacted]` 矛盾 | 🔴 | 日志 v0.2.4/v0.2.5 | 已实证 |
| RC12 | web_fetch 全周期 0 次成功；"沙箱隔离外网"误诊 | 🔴 | `web.rs:91-102` vs `sandbox/lib.rs:577-580` | 已实证 |
| RC13 | service 路径白名单不读 config.toml | 🟡 | `service/src/main.rs:411-416` vs `run_local.rs:219-228` | 读码即得 |
| RC14 | "env 优先级更高"推断错误（实为并集） | 🟡 | `config.rs:159-180` | 读码即得 |
| RC15 | 自测把"白名单按设计工作（全拒）"标为 PASS | 🟡 | 日志 v0.2.5:1679 | 已实证 |

---

## 6. 证据边界声明

1. **版本边界**：VM 上无 `.git`，v0.2.4–v0.2.7 无 release 包。日志与 HEAD 源码的时间对齐以日志内自述版本号为准。
2. **日志编码**：三份日志内部存在 UTF-8 字节被按 GBK 解码后重新编码的乱码段（占比 0.5–1.1% 字符），影响少量长工具输出行的可读性，不影响本轮结论（关键行均已交叉验证）。
3. **未做的事**：本轮未修改任何 `crates/` 下代码；未执行新的 VM 实跑（三条线索均由日志实证 + HEAD 源码核验闭合）。
4. **RC12 的最后一公里未闭合**：35 次拒绝的**具体原因**（是 env 没传进 hearth 进程、还是 ctx.env 没注入、还是配的域名与请求的域名不匹配）需要一次受控复现实测才能定论。已备选项，待顶层拍板是否派活。
