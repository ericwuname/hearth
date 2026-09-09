# Hearth RC24 审批门施工单 v1：语义化判定 + 非交互结构化语义 + 会话级审批委托

> **性质**：独立施工单（顶层裁决稿，ChatGPT 施工顺序第①位）——**批准后施工**。
> **签发依据**：ChatGPT 顶层裁决（DEV-1 升最高优先、RC24 先行）+ 总账 RC24/RC29 卡片 + W3/W4 回归验证偏差报告（DEV-3）。
> **基线**：v0.2.9 / HEAD `26ad3be` 链；隔离门禁 **391 passed / 0 FAILED**（守门员已复核）。
> **目标**：一次施工闭合三件事——①审批判定语义化（`> /dev/null` 误杀）②非交互模式审批语义结构化（静默阻塞→秒级可行动失败）③会话级审批委托（RC29"授权最大权限"有人接收）。

---

## ⛔ 零、本单禁改（D 类冻结清单继续生效）

以下事项的 diff 中**禁止出现**（各自独立施工单）：bash stdout 截断、filesystem write accounting、**Goal Revision 三分类接入 `apply_turn_goal`（W8 下单，DEV-1 随后）**、resource 超限控制流、tool schema 相位裁剪、bridge、subagent、TUI、ContextBuilder 架构。

**本单授权触碰范围**：`bash_cmd_is_destructive`（`loop.rs:220-270` 一带）及其调用点、`run_local.rs` one-shot 审批循环（:702-720 一带）、`repl.rs` 审批等待（:171-188 一带）、`terminal.rs` normalize 映射（新增 reason）、REPL `trust` 命令、CLI `--approve-within` flag、相关单测。

---

## 一、根因锚点（已核，施工前须复核行号）

| 根因 | 锚点 | 现状 |
|---|---|---|
| `> /dev/` 一刀切判破坏性 | `loop.rs:230` `raw.contains("> /dev/")` | `/dev/null` 是位桶（写入即丢弃，无系统状态变更），且 T6 后 landlock 已放行其写入——**语义已变，判定没跟上**。V-1 实测：T-C 首探测被阻塞、B02/B04 blocked |
| 非交互审批无语义 | `run_local.rs:702-720`（one-shot y/N inline stdin）；headless 下 stdin EOF → 静默 deny | B02/B04 blocked 23s 后 failed，**无结构化原因、无可行动提示**——RC24 第③环 |
| 会话级委托缺失（RC29） | 无任何机制 | 用户"授权最大权限"后审批门仍弹（ALL L22011→L23011/L24615 双实证） |

## 二、施工内容

### A. 审批判定语义化（`bash_cmd_is_destructive`）

```text
判定表（显式化 + 逐行单测）：
仍判破坏性（保留）：
  fork bomb（:226）
  > /dev/sd*、/dev/mem 等真实设备块/字符设备（保留设备写语义）
  > /proc/sys/、> /sys/（内核接口）
  rm/rmdir/dd/mkfs/shutdown/reboot/…（:234 命令表不变）
移出破坏性（本单核心）：
  > /dev/null、2> /dev/null、&> /dev/null（位桶：丢弃输出，无系统状态变更）
  ——注意保留 > /dev/ 前缀对其他 /dev/* 节点的拦截（只精确豁免 /dev/null）
```

- 豁免实现建议：`:230` 拆为"先匹配重定向目标路径，`/dev/null` 放行、其余 `/dev/*` 仍拦"。
- 单测：`> /dev/null` / `2>/dev/null` / `cmd > /dev/null 2>&1` 不触发审批；`> /dev/sda`、`> /dev/mem` 仍触发（**总账 TC-8/TC-9 转绿**）。

### B. 非交互审批结构化语义

```text
检测：stdin 非 tty 或显式 --non-interactive
行为：审批请求产生时【立即结构化 deny】——不等待、不静默、不假装成功
  ①事件：审批事件照常进事件流（Observer 可见，approval_denied）
  ②终态：run 终止，reason = approval_denied_noninteractive
    （terminal.rs normalize 映射加一行 → 投影 ✗ + reason）
  ③可行动提示（LLM 与用户都可见）：
    "破坏性操作在非交互模式被拒绝——用 hearth repl（可交互批准）
     或 --approve-within session（显式委托）后重跑"
```

- V-1 B02/B04 的 23s 阻塞 → **秒级结构化失败**；对照旧基线"挂起 200s"——审批路径不再产生任何形式的无限等待。
- 单测：headless 构造审批请求 → 断言秒级返回 + reason=approval_denied_noninteractive + 事件流含 approval_denied。

### C. 会话级审批委托（RC29）

```text
入口（显式 opt-in，默认关闭）：
  one-shot：--approve-within session
  REPL：trust on / trust off（会话级，可随时撤销）
语义：
  委托开启 → 普通破坏性命令（rm/dd 等）自动放行，每条记审计日志
    （tracing + run report 审批字段标 delegated:true）
  永远不委托的硬红线（即使 trust on）：
    fork bomb / > /dev/sd* / > /proc/sys / > /sys/（物理级与内核接口）
  投影：委托放行的命令帧标注 [delegated]（用户可审计）
```

- 单测：trust on → rm 命令放行且审计行存在；trust off → 恢复逐条审批；fork bomb 在委托下仍触发审批。
- **设计说明（顶层已裁）**：委托范围 = 命令表级破坏性操作；物理级/内核接口不设委托——"授权最大权限"解决的是打扰问题，不是解除 G0。

## 三、验收（真机）

1. **TC-8/TC-9 转绿**（总账既有用例）：`> /dev/null` 写正常、无空格写法安全分级一致。
2. **V-1 B02/B04 场景复跑**：无委托 → 秒级 `approval_denied_noninteractive` 结构化失败（对照 23s 阻塞）；带 `--approve-within session` → 任务推进至 completed。
3. **REPL trust 场景**：trust on 后同会话破坏性命令不再弹窗且审计行存在；trust off 恢复。
4. **RC29 原话场景**：模拟"授权后离开"——委托开启后 0 次重复弹窗（对照 ALL L22011→L24615 的 2 次重复）。
5. 全量隔离门禁 ≥391（+本单新测试）全绿，`~/run_gate_r2c.sh`。

## 四、红线与纪律

- 审批事件流语义不变（InteractionRequested 消费方零改动——只改判定与模式语义）。
- 先红后绿：TC-8/9 与非交互用例须先在旧代码上红。
- 每步独立 commit；发现设计偏差 → 偏差报告停在该点。
- 本单完成 = DEV-3（RC24 阻塞）闭合 + DEV-4（cancelled headless 投影）可顺带核查（SIGINT → cancelled 投影链）。

## 五、后续排队（本单验收后另派，不在本单）

**W8 · Goal Revision / 任务类型路由（DEV-1，ChatGPT 定为紧随本单）**：`classify_user_input` 接入 `apply_turn_goal` + **论述型任务路由**（无产物任务短路 TaskGraph/reflect 循环——V-1 完成率 1/10 的结构性根因）。之后 DEV-2 Reflect 判定质量走独立证据窗（不塞规则）。
