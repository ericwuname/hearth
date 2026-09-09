# Hearth Harness 加固任务书 v0.1.4 · 正式签发版（v0.1.3 测试复盘 + 新 P0）

> ┌─ 签发栏 ───────────────────────────────────────────────────────────
> │ 版本：v0.1.4（正式签发版 · 快速修复 · 含新 P0）
> │ 签发日期：2026-08-23
> │ 签发人：顶层守门员（全局架构 / 评审 / 验收 / 任务书）
> │ 接收方：执行窗口
> │ 下发时序：接在 v0.1.3 之后；**R7 必须先于一切端到端测试**（含 v0.2 V2-8），
> │   因为 provider 不稳定时任何真机任务都无意义
> │ 依据：release/手工测试v0.1.3.txt 真机测试（build 47cb0e4）+ 守门员源码核验
> │   关键源码：codex-cli/src/lib.rs:527 / repl.rs:9,96,241,320；
> │   llm-openai/src/lib.rs:316-343；agent-core/src/loop.rs:1371-1419,2238；
> │   planner/src/lib.rs:155,402；llm-gateway/src/types.rs:197
> │ 过闸口径：🔴阻塞 / 🟡遗留 / 实现率≥0.9；守门员独立核验（不信报告信源码）
> └────────────────────────────────────────────────────────────────────

---

## 0. v0.1.3 测试复盘（build 47cb0e4，REPL 真机）

| R | 判据 | 结果 | 证据 / 源码 |
|---|---|---|---|
| R1 (B1 终止) | 纯问答≤3步、无 replan 逼写、无 tool not found | **部分达成** | `tool not found:bash` 已消失✅；无逼写文件（以文本结束）✅；但首轮"你是谁"仍 10 步（模型自决探索，非强制 replan）；后期纯问答有 2 步达标✅。B1 强制写文件闸已移除，残留过度探索归 WS3。 |
| R2 (B4 预算) | chat+REPL 默认 40、杜绝 replan 烧预算 | **部分达成（REPL 漏改）** | chat 默认 40（lib.rs:61）✅；但 REPL 仍硬编码 20：`lib.rs:527` `run_local_repl(&resolved, 20)` ❌；`repl.rs:9` 的 `REPL_BUDGET=40` 常量未被用于 agent 循环（仅用于 create_session 元数据）。"Giving up after 16 steps"+ 重复 give-up 仍出现（planner 15% 阈值早退，lib.rs:402）。 |
| R3 (B5 路径) | 家目录绝对路径放行、/etc 拒绝 | **未显式验证，无回归** | 日志无 path traversal denied；`read /home/wutao/...`、`bash cd /home/wutao` 均正常（绝对路径未拒）✅；但 grep 绝对路径专项测试未出现，建议补。 |
| R4 (B6 沙箱) | node --check / tail 返回真实退出码 | **未验证** | 日志无 node --check / tail 测试，无法判。 |
| R5 (B7 REPL 分流) | WARN 不污染提示符 | **未达成** | 日志 271-282 行：WARN 与 `hearth> 〉` 提示符同行/交错（"hearth> 〉2026-...WARN..."），REPL 渲染未分流。 |

**结论**：v0.1.3 的 B1 核心症状（tool not found / 逼写文件）已修；但 R2 的 REPL 预算漏改、R5 的 REPL 日志分流未做。更重要的是——日志暴露出一个 **v0.1.3 完全没覆盖、且当前最致命的新故障（见 R7）**。

---

## R7（🔴 P0 · 新）：deepseek 通道持续超时/空响应 —— `read body failed`

**现象**（日志 19:01 起，几乎每一轮）：
```
WARN llm_openai: read body failed — retrying once provider=deepseek attempt=0/1 error=error decoding response body
ERROR agent_core::r#loop: plan chat failed — give up ... retries=1 class=Transient deadline_hit=true
✗ transient provider error: read body: error decoding response body
✗ Done (1 steps)   ← 失败却被标记为 Done，误导用户
```
- **节奏**：每次 attempt 间隔 ~31s（deepseek 挂起超时），2 次 ≈ 62s > `TOTAL_RETRY_CAP`(60s, loop.rs:1372) → `deadline_hit=true` → 放弃。
- **伴生**：planner `Failed to parse LLM task plan as JSON: EOF at line 1 column 0/167`（同一根因：deepseek 返回空/截断体 → 降级单节点）。
- **源码链路**：`llm-openai/src/lib.rs:316-343`（provider 层重试 1 次 500ms，失败上抛 Transient）→ `agent-core/src/loop.rs:1371-1419`（loop 层 ≤2 次退避 2s/4s，60s cap）→ `llm-gateway/src/types.rs:197`（decode 失败→Transient）。
- **判定**：这是当前**最致命的可用性阻断**（harness 在 19:01 后基本不可用），且不在 v0.1.3 范围。

**根因（待诊断，非 harness 单方面可控）**：deepseek 通道在该测试时段**持续超时/空响应**——可能限流（免费/低价通道累积调用后被限）、过载、网络/代理、或长上下文超限。harness 侧的问题是：① 重试后仍等满 60s 才放弃；② 失败后**伪装成 Done** 误导用户；③ 无清晰"provider 故障"提示与切换手段。

**修复意图（执行窗口细化；诊断先于修）**：
1. **诊断前置（必做，见 §4）**：先用 Gemini 通道（用户首选、key 可用）复跑同一组对话；抓一次失败原始 HTTP（`RUST_LOG=llm_openai=debug`）看 status/body，确认是 429 / 超时 / 空体。
2. **失败不伪装 Done**：provider 故障须输出可行动错误（"provider 故障，请重试 / 切换通道"），不得标 `Done`。
3. **请求超时 + 有界退避**：为 HTTP 请求设合理超时（~20-30s）失败快速返回；退避按故障类型区分（瞬时抖动 2/4/8s；持续故障不盲目死等 60s）。
4. **可切换 provider**：配置/运行期可在 deepseek↔gemini 间切换，避免单通道故障阻塞。
5. planner 空体：不要静默降级单节点，应区分"空响应=provider 故障"并上抛/提示。

**验收判据**：
- 连续 10 轮对话，在 deepseek 偶发抖动下**不静默失败**、不伪装 Done；明确提示 provider 状态。
- Gemini 通道可跑通中等任务（如贪吃蛇），证明 harness 不依赖单一通道。
- （若根因是 deepseek 限流）代码只做"优雅降级 + 清晰报错 + 可切通道"，真实修复=换通道/提额度——守门员据此判定 R7 完成，不要求"修好 deepseek"。

---

## R2（🟡 P1）：REPL 预算同步 40

**根因**：`codex-cli/src/lib.rs:527` `repl::run_local_repl(&resolved, 20)` 硬编码 20；`repl.rs:241/320` 用该 `budget` 设 `goal0.max_steps`。`repl.rs:9` 的 `REPL_BUDGET=40` 常量仅用于 `create_session`（repl.rs:96，会话元数据），**未用于 agent 循环**——典型的"常量定义了没接上"。

**修复意图**：`lib.rs:527` 改为 `repl::run_local_repl(&resolved, REPL_BUDGET).await?;`（或显式 40），让 agent 循环与 chat 一致为 40；消除 "Giving up after 16 steps"。

**验收判据**：`hearth repl` 跑中等任务（象棋/贪吃蛇）默认 40 步预算，不再 16-20 步早退；与 chat 一致。

---

## R5（🟡 P1）：REPL 日志分流，WARN 不污染提示符

**根因**：REPL（reedline）渲染与异步 WARN 日志未分流，WARN 直接混进 `hearth> 〉` 提示行（日志 271-282）。

**修复意图**：日志统一走 stderr 或独立渲染通道；REPL 提示符行只显示用户输入回显与 agent 输出；多行 WARN 块不插入用户输入行中间。

**验收判据**：REPL 中 WARN 出现在独立区域，提示符 `hearth> 〉` 行干净无夹杂。

---

## R4（🟡 P1）：修复沙箱杀验证命令（exit -1）—— 补测

**说明**：v0.1.3 任务书已列 R4，但本测试日志未含 node --check / tail 用例，故仍未验证。沿用 v0.1.3 R4 修复意图：
- 复现定位：沙箱内 `node --check x.js` / `tail -n 5 f` 用 strace/bpf 看是被 KILL 还是 bash 返回 -1。
- seccomp 缺 syscall（node 用 `prlimit64`/`membarrier` 等）→ 补白名单；bash 探针误判 → 修正探针。
- 确保验证类命令返回**真实退出码**。

**验收判据**：`bash cmd: node --check game.js` → 0 或真实非 0，不再 -1；`tail -5 f` 正常输出。

---

## R3（🔵 P2）：补 grep 绝对路径专项测试

**说明**：本测试未显式跑 `grep path: /home/wutao/...`。R3 修复意图（放宽 allowed_root 至项目/家目录）已落地且未见回归，但须补专项验收。

**验收判据**：`grep pattern: xxx path: /home/wutao/codex_6d` → 正常搜索；`grep path: /etc/shadow` → 仍 denied。

---

## R1 收尾（🔵 P2）：纯问答首轮过度探索

**说明**：首轮"你是谁"仍 10 步（模型自决 glob/read 探索）。B1 强制写文件闸已移除（核心已修），残留属"模型对纯聊天/问答仍进工具探索"。完整收敛归 v0.2 WS3（planner 瘦身）。本版可做轻量缓解：意图启发式（聊天/问答类不进工具探索，直接文本回答）。

**验收判据**：纯问答/聊天类首轮 ≤3 步结束，不调工具。

---

## §4. 诊断前置（R7 必做，先于修）

1. **切 Gemini 复测**：`hearth --provider gemini ...`（用用户首选通道、OpenAI 兼容端点、可用 key）复跑同一组对话。Gemini 正常 → 确认 deepseek 侧问题（限流/超时）；Gemini 也失败 → harness 解码/网络 bug，转深挖。
2. **抓原包**：`RUST_LOG=llm_openai=debug hearth repl` 抓一次失败，看 `chat NON-success` 的 status 与 body，确认 429/超时/空体。
3. **查额度**：确认 deepseek key 限额/限速（免费或低价通道累积调用后易被限）。

---

## §5. 下发时序与依赖

- **R7 最高优先**：必须先于一切端到端测试（含 v0.1.4 其余项与 v0.2 V2-8）。provider 不稳定时任何真机任务都无意义。建议用户**现在就切 Gemini 复测**，边诊断边等 R7 落地。
- **R2 / R5 并行**：REPL 预算 + 日志分流互不依赖，可并行。
- **R4 / R3 / R1 收尾**：R4 补测沙箱、R3 补 grep 专项、R1 轻量缓解，可在 R7 落地后收尾。
- **v0.2 在其后**：v0.2 WS1-WS6 能力对标，且 V2-8 综合验收（贪吃蛇/象棋/web 应用真机稳定）**前置依赖 R7 解决**。

---

## 签收回执

| 项 | 内容 |
|---|---|
| 签发人 | 顶层守门员 |
| 签发日期 | 2026-08-23 |
| 接收窗口 | 执行窗口 |
| 接收确认 | ________________（执行窗口签收后回填） |
| 开工前置 | 完成 §4 诊断（Gemini 复测 + RUST_LOG 抓原包），明确 deepseek 根因后再定 R7 代码范围 |
| 过闸提交 | R7 + R2 + R5 全过 + 守门员独立核验；R4/R3/R1 收尾补测 |

> 本任务书为顶层正式签发件。R7 是 v0.1.3 测试暴露的新 P0，优先级高于其余快速修复；若诊断确认根因为 deepseek 限流，R7 以"优雅降级 + 清晰报错 + 可切通道"为完成判据，不要求修 deepseek 本身。
