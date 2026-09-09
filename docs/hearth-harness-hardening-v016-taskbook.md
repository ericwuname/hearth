# Hearth 加固任务书 v0.1.6（正式签发版）

| 项 | 内容 |
|---|---|
| **版本** | v0.1.6 |
| **签发日期** | 2026-08-23 |
| **签发人** | 顶层守门员 |
| **接收方** | 执行窗口（施工） |
| **依据** | `release/手工测试v0.1.5.txt`（build `33f5b7b`）、`docs/hearth-harness-hardening-v014-taskbook.md`、`docs/hearth-harness-hardening-v013-taskbook.md` |
| **前置** | v0.1.3（R1–R5 快速修复）+ v0.1.5（含 v0.1.4 修复）**已完成且真机验证** |
| **本期目标** | 修掉 v0.1.5 真机暴露的 **2 个 P0 阻断 + 1 个 P1 交互增强**，让 deepseek-v4-flash 通道下大任务可稳定完成、澄清交互支持选项式 |
| **过闸口径** | 🔴 阻塞 / 🟡 遗留 / 实现率 ≥ 0.9；守门员**独立核验源码 + 真机复测**（不信报告信源码） |
| **下游门禁** | v0.1.6 是 **v0.2 V2-8 综合验收（贪吃蛇/象棋/小型 web 应用 VM 真机稳定）的前置闸门**——V2-8 跑不起来先回头看 v0.1.6 |

---

## §0 背景：v0.1.5 真机暴露了什么

build `33f5b7b`（deepseek-v4-flash）已修好 R1 终止闸、R2 预算 40、R5 REPL 提示符、B4 diff 编辑、R7 不伪装 Done。
**仍坏、且全是 harness 设计问题**（按用户口径：gemini 不可用，deepseek-v4-flash 够用，问题归 harness）：

1. **【P0·崩溃】字符边界 panic** — `context.rs` 按**字节**切中文目标，长会话 compaction 触发即整 REPL 死掉。
2. **【P0·大任务全败】provider 健壮性** — 象棋/8000字长文/俄罗斯方块 全部 `read body: error decoding response body` 或 `planner: empty LLM response`；deepseek-v4-flash 慢/偶发空响应时 harness 太脆（重试 1 次 + 60s 就放弃、空响应直接判致命、报错还让你切 gemini——而 gemini 现在用不了）。
3. **【P1·交互】澄清仅自由文本 + 远程丢文本** — 本地澄清能收任意文本（好），但不支持选项式（1/2/3、y/n）；远程侧澄清只收 y/n 且**丢弃文本**。

---

## §1 问题清单（精确 file:line）

| 编号 | 严重度 | 现象 | 根因位置 | 验收 |
|---|---|---|---|---|
| **R8** | 🔴 P0 | 长中文目标 → REPL panic `end byte index 60 is not a char boundary`，整个会话死 | `crates/agent-core/src/context.rs:157-158` `&goal[..60]` 字节切片 | 中文目标 >60 字节不再 panic |
| **R9** | 🔴 P0 | 大任务（象棋/长文）`read body`/`empty response` 全败，harness 60s 放弃 | `loop.rs:1428-1429`（重试 2/60s 过紧）、`llm-openai/src/lib.rs:338-346`（read body 仅重试 1 次 500ms）、`planner/src/lib.rs:295`（planner `max_tokens=2048` 太小）、`run_local.rs:332`（报错文案误导切 gemini）、planner 网络错直接降级单节点不重试 | deepseek-v4-flash 下大任务稳定完成；空响应触发重试而非静默/致命；报错无 gemini 误导 |
| **R10** | 🟡 P1 | 澄清只自由文本；远程澄清只 y/n 且丢文本 | `agent-types Gap` 无选项字段、`loop.rs:1184` payload 无 options/style、`run_local.rs:382-421` 无选项渲染、`lib.rs:917-946` 远程只 y/n | 含糊目标显示编号选项；输数字/ y/n /自由文本均生效；远程一致 |

---

## §2 工作流（执行窗口逐项实现）

### WS-A · R8 字符边界 panic（P0，先做，1 行级）

**动机**：compaction 只在上下文涨到阈值才触发，所以"小任务正常、长会话才崩"——这是随时会炸的雷，必须先排。

**改法**（`crates/agent-core/src/context.rs:157-161`）：

```rust
// 改前（BUG：goal.len() 是字节数，&goal[..60] 按字节切，多字节字符中间切即 panic）
let goal_short: String = if goal.len() > 60 {
    format!("{}…", &goal[..60])
} else {
    goal
};

// 改后（按字符计数 + 按字符截取，语义才是"60 字符"）
let goal_short: String = if goal.chars().count() > 60 {
    format!("{}…", goal.chars().take(60).collect::<String>())
} else {
    goal
};
```

**验收（R8）**：`cargo test -p agent-core` 不回归；手测目标 >60 字节中文（如"帮我写一个完整的、带注释的、可运行的 Rust 实现快速排序并附带单元测试的程序"）长会话不崩。

---

### WS-B · R9 provider 健壮性（P0，deepseek-v4-flash 当"会抖的可恢复通道"）

**动机**：deepseek-v4-flash 慢且偶发空响应/超时。harness 应把它当**可恢复瞬时故障**而非致命故障；重试要给够机会，且**不要误导用户切 gemini**（现在 gemini 用不了）。

**B1 主循环重试放宽**（`crates/agent-core/src/loop.rs:1428-1429`）：

```rust
// 改前
const MAX_TRANSIENT_RETRIES: u32 = 2;
const TOTAL_RETRY_CAP: std::time::Duration = std::time::Duration::from_secs(60);
// 改后（deepseek-v4-flash 慢，给 4 次 + 120s；退避 2/4/8/16s 仍 < 120s）
const MAX_TRANSIENT_RETRIES: u32 = 4;
const TOTAL_RETRY_CAP: std::time::Duration = std::time::Duration::from_secs(120);
```

**B2 read body 重试**（`crates/llm-openai/src/lib.rs:338-346` 与 `crates/llm-cn/src/lib.rs:430` 同构）：当前 `read body failed — retrying once` 仅重试 1 次 500ms。改为**有限次退避重试**（如 3 次，间隔 500ms/1s/2s），仍失败才返回错误。注意：此处 `attempt` 已在日志中使用，套一层 `for attempt in 1..=3` 即可。

**B3 planner 容量 + 重试**（`crates/planner/src/lib.rs:295`）：
- `max_tokens: Some(2048)` → `Some(8192)`（与 `loop.rs:1441` 主循环对齐；象棋/8000字长文需大输出）。
- planner 网络错误当前 `do_plan` 直接 `return Ok(single_node_graph(goal))`（line 304-316）——对 deepseek 抖动太早放弃。改为：网络/瞬时错**先有界重试（≤3 次）**，仍失败再降级单节点（已在 R7 验证降级路径正确，保留）。空响应已 `Err(Transient)` 上抛（line 326），确保主循环 `loop.rs:1428` 重试能接住。

**B4 报错文案去 gemini 误导**（`crates/codex-cli/src/run_local.rs:332`）：

```rust
// 改前
"provider 故障——请重试，或 --provider gemini 切换通道（hearth config set provider gemini 可持久化）"
// 改后（gemini 当前不可用，去掉误导；给可操作提示）
"provider 通道瞬时故障——请重试；若持续失败检查网络/API key/限流，或 hearth config set provider <其他可用通道>"
```

**验收（R9）**：deepseek-v4-flash 下跑象棋 / 8000 字长文 / 俄罗斯方块，**不再**出现 `read body`/`empty response` 致命失败；偶发空响应后 harness 自动重试并成功；报错文案不含 gemini；`cargo test` 全绿。

---

### WS-C · R10 交互选项原语（P1，零内核改动，符合 WP-0）

**动机**：用户要"选 1/2/3、y/n、还能手输其他"。利用已有 WP-0 交互原语（内核只透传 `kind/blocking/id`，不解析内容），**只改数据模型 + UI 渲染**，内核一行不动。

**C1 Gap 加选项字段**（`crates/agent-types/src/lib.rs:118-149`）：

```rust
pub struct Gap {
    pub from: String,
    pub why: String,
    pub blocking: bool,
    pub auto_assumed: bool,
    /// 可选：澄清选项（选项式交互用）。空 = 自由文本。
    #[serde(default)]
    pub options: Vec<String>,
    /// 交互样式："free_text" | "single_select" | "confirm"。默认 free_text。
    #[serde(default = "default_style")]
    pub style: String,
}
fn default_style() -> String { "free_text".into() }
```
构造器 `assumed`/`blocking` 补 `options: vec![]` + `style: "free_text".into()`；新增：
```rust
pub fn blocking_with_options(from: impl Into<String>, why: impl Into<String>, options: Vec<String>) -> Self {
    Self { from: from.into(), why: why.into(), blocking: true, auto_assumed: false,
            options, style: "single_select".into() }
}
```

**C2 planner 抽选项**（`crates/planner/src/lib.rs:67-73` ambiguous_option 处）：从目标里抽"或/或者/or/either"两侧候选，调用 `Gap::blocking_with_options`。best-effort：

```rust
// goal_ambiguous 命中时，抽出二选一候选
let opts = extract_options(&goal_lower); // 按首个 or-sep 切左右片段，各 trim 到 ≤20 字；失败则默认 ["按默认实现","我另有指定"]
gaps.push(Gap::blocking_with_options(
    "ambiguous_option",
    format!("目标含未澄清技术选项（'{}'），请选择或补充说明", snippet.trim()),
    opts,
));
```
（新增 `fn extract_options(s:&str)->Vec<String>`，按 `或|或者| or |either` 切分，取前两个非空片段；兜底默认两选项。）

**C3 内核 payload 透传**（无需改逻辑，`crates/agent-core/src/loop.rs:1184`）：

```rust
payload: serde_json::json!({
    "from": gap.from,
    "why": gap.why,
    "options": gap.options,   // 新增
    "style": gap.style,       // 新增
}),
```

**C4 本地 REPL 渲染 + 解析**（`crates/codex-cli/src/run_local.rs:382-421`）：clarification 分支改为——
- `style=="confirm"`：显示 `批准? [y/N]`，收 y/n；
- `style=="single_select"` 且 `options` 非空：列出 `1) A  2) B  3) C`，收数字；
- **任何输入不是合法数字 / 不属 y/n → 原样当自由答案**（`resolved=true, payload.answer=文本`）——这就是"还能手工输入其他"；
- `style=="free_text"`（默认）：同现状整行自由文本。
渲染时若有 options，先打印选项再收输入。

**C5 远程侧一致性修复**（`crates/codex-cli/src/lib.rs:917-946`）：当前 `if action=="clarification"` 只走 y/n 分支且 `submit_approval(sid,aid,approved)` 把文本丢了。改为与本地**同构**：打印 `from/why` + 选项（若有），`read_line` 收整行 → 解析为 answer → `submit_approval` 时把 answer 一并带回（参考本地 `resolve_interaction(sid, id, true, json!({"answer":...}))` 语义；远程若 API 仅支持 bool，至少把 answer 记日志且不在 UI 上谎称只收 y/n）。

**验收（R10）**：含糊目标（含"用 Rust 或 Go 实现"）REPL 显示编号选项；输入 `1`/`2` 选中、输入 `y`/`n` 走确认、输入任意其他文字当自由答案均生效；远程 HTTP 通道行为一致（选项可见、文本不丢）。

---

## §3 总验收门禁

| 编号 | 判据 | 阻断级 |
|---|---|---|
| **V1-6-1** | 中文目标 >60 字节，长会话不 panic（R8 修法生效） | 🔴 |
| **V1-6-2** | deepseek-v4-flash 下象棋/8000字长文/俄罗斯方块稳定完成，无 `read body`/`empty response` 致命失败；报错无 gemini 误导（R9） | 🔴 |
| **V1-6-3** | 含糊目标显示编号选项；数字/y/n/自由文本三种输入均生效（R10） | 🟡 |
| **V1-6-4** | 远程 HTTP 通道澄清选项可见、文本不丢（R10 远程） | 🟡 |
| **V1-6-5** | `cargo test`（workspace）全绿；`cargo clippy`/`cargo fmt --check` 干净；无新增 `.unwrap()` 在交互热路径 | 🔴 |

**发版门槛**：🔴 项 100% 通过方可 commit/发布；🟡 项允许遗留但须登记原因与回退方案。

---

## §4 范围外（本期不做）

- 子代理（subagents）/ 并发 agent —— 能力对标维度 consciously 延后（Aider 证明非阻塞）。
- repo-map 第二阶段（按需上下文两阶段中的第二阶段）——归 v0.2 WS3。
- 整体重抄上游 codex-thread-store / Lark apply_patch —— 已决定轻量重实现，不在本期。
- MCP / app-server / 跨平台 / fork 回滚 / 换模型架构。
- G0 fail-closed 安全边界（landlock+seccomp）——**禁止触碰**。

---

## §5 顺序与依赖

1. **WS-A（R8）** → 先做，1 行级，解除随时崩溃风险，独立可提。
2. **WS-B（R9）** → 紧随，依赖 WS-A 解除崩溃后才能真机长跑大任务验证。
3. **WS-C（R10）** → 独立，可与 WS-B 并行，但建议 WS-B 先合以避免交互日志干扰复测。
4. 全部过 §3 门禁后 → 出 v0.1.6 release，作为 **v0.2 V2-8 综合验收的前置闸门**。

---

## §6 给执行窗口的说明

- **gemini 误区**：本机 gemini 需 VPN、暂不可用，**不要**在报错/文案里再引导切 gemini；deepseek-v4-flash 是既定主通道，把它当"会抖的可恢复通道"设计。
- **零内核改动原则（R10）**：交互形态变更只动 `Gap` 数据模型 + UI 渲染 + payload 透传；`loop.rs` 内核交互逻辑（只认 `blocking`/`id`）**一行不许改**。
- **守门员核验**：实现后我会按"不信报告信源码"独立核验——grep 确认 `&goal[..60]` 已消失、`MAX_TRANSIENT_RETRIES` 已放宽、Gap 字段已加、远程不再只 y/n；并真机复测 V1-6-1~V1-6-5。报告与源码不符一律打回。
- **提交纪律**：每项独立小提交（R8 / R9-B1..B4 / R10-C1..C5），便于逐条核验与回退。
- 若实现中发现顶层决策不可行（如 planner 重试与现有 fail_cache 冲突），**停止并回顶层出修订**，不得擅自改控制流或绕过门禁。

---

## §7 签收回执

| 字段 | 内容 |
|---|---|
| 接收方 | 执行窗口（施工） |
| 签收 | ________________（签名） / 日期 ________ |
| 约束确认 | ☐ 已读 §6 说明 ☐ 遵守零内核改动（R10） ☐ 不触碰 G0 ☐ 提交前过 V1-6-5 |
| 方向性决策 | 本期为**加固收尾**，不引入新能力维度；死守白盒四阶段 / Gap 三元组 / EnvelopedEvent / G0 fail-closed（详见 v0.2 任务书 §6）。不得擅自变更。 |

---
*本任务书为 v0.1.5 之后唯一待执行的加固任务，替代并闭合 v0.1.4（已含于 v0.1.5）。完成后即解锁 v0.2 V2-8 综合验收。*
