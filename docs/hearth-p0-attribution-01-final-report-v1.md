# Hearth P0-ATTRIBUTION-01 Final Report v1

**日期**：2026-08-31　**执行窗口**：砺·执行　**基线**：v0.2.18（+env-gated 观测代码，零控制流改动）
**性质**：盲测反证定向复现与归因（零修复纪律 ✓——本轮生产代码仅 env-gated dump，见 §1.1）

## 1. Executive Summary

| 问 | 结论 | disposition |
|---|---|---|
| **Q-A** 裸 ERROR 泄漏路径 | **CONFIRMED**——完整路径定位（源码+真机复现）；未被投影为成功 → 非 NEW F0 | **NEW F1**（Projection isolation+completeness，RC51 拆分建议） |
| **Q-B** 24 连败因果 | **EVIDENCE GAP**——重放器本身失真（三缺陷），受控矩阵未达成；RC52 维持"observed attractor" | **EVIDENCE GAP**（按 STOP-① 停在证据边界） |
| **Q-C** 截断点 + give_up/completed 共存 | 截断点=**Projection 层终态分支**（定位）；give_up 字样=报告"反思轨迹"历史记录 → **FALSE POSITIVE**（反证已写）+ 可观测性缺口登记 | FALSE POSITIVE + **NEW F1**（completion projection completeness） |
| RC53/RC54 | RC53=**model 层 tool misroute 定位**；RC54=定位+fixture DEFERRED | 登记不修 ✓ |

**零生产代码修复** ✓（唯一改动 = env-gated 观测，§1.1）；**NEW F1 三项 + EVIDENCE GAP 一项**；无 F0。

## 1.1 唯一许可改动：env-gated 观测代码（总包 §1.2）

`HEARTH_DEBUG_PLANNER_INPUT=1` → dump 到 `~/.config/hearth/debug/<session_id>/`：
- `plan-<ms>-prompt.txt`：planner 实际输入逐消息快照（loop.rs do_plan build_messages 后）
- `graph-<ms>.txt`：TaskGraph sig + 前值 + 节点全文（T4 比较点）
- `compact-<ms>-summary.txt`：压缩摘要全文（maybe_compact）

零控制流影响；未开时零开销。双 VM 构建记录：.133 sha256 `bffe2dc0cfbbf73d…`、.131 `efc512fa2ee86653…`（0.2.18 + obs）。真机 dump 实证：session `5406164f` 目录 graph-*.txt 正常落盘。

## 2. Node 00 — Baseline（`node00-baseline.md`）

- 主机统一 .133（盲测 L4 实锤）；binary sha256 锁定；cwd=$HOME、`hearth repl`、无 budget flags（goal_tier 启发式，盲测 run-013 18 步 ≤ economy 20 吻合）；Agnes/agnes-2.5-flash。
- 重放剧本：run-001..013 逐字提取（41 条 `hearth>` 行 → 砺 13 逻辑 run 对齐，L265-267 三行=逻辑 run-004）；截止点定死 run-012 ✓；剧本 hash 归档。

## 3. Node 01 — Q-A：裸 ERROR 泄漏路径（CONFIRMED）

**完整路径（源码实测）**：
1. `do_plan_inner` Err（T4 stalled）→ `tracing::error!(error=%e, "do_plan_inner failed")`（**loop.rs:2381，生产路径**，level=ERROR，target=agent_core::r#loop）；
2. subscriber：`tracing_subscriber::fmt().with_env_filter("hearth=info,error,…").with_writer(std::io::stderr)`（**codex-cli/lib.rs:264-266**）→ **stderr 直写终端**；
3. **完全绕过 Projection 层**（render.rs 不经手）→ SSH 终端直接可见（盲测 L265/L2059 用户两次粘贴原文实锤；本轮 b1c 重放会话 3 处复现）。

**判级（口径 ✓）**：错误**未被投影为成功**（渲染层看不到它，无"错误→成功"投影路径）→ **非 NEW F0**。定性 = **Projection isolation 违约**（裸 telemetry 绕过投影层直达用户）+ **completeness 缺口**（错误内容不进任何结构化通道）。
**RC51 修订案**：CLOSURE §7 "Projection" 指标拆三行——**isolation**（终端零裸 telemetry）/ **completeness**（用户可见内容 ⊇ 内部终态事实）/ **correctness**（渲染与事实一致）。

## 4. Node 02 — Q-B：对照矩阵 = EVIDENCE GAP（STOP-① 触发）

**三版 harness 全部失真，受控矩阵未达成**：

| 版本 | 形态 | 失真 | 证据 |
|---|---|---|---|
| v1 管道→REPL | piped stdin | **DenyAllNonInteractive**（非 TTY 审批策略）→ 12/12 瞬失败；**EOF 后 REPL 忙等刷 prompt（56MB）** | p0_b1c.log（已归档作废） |
| v2 PTY→REPL | script -qec | 输入消费竞争 + 同类刷屏（191MB） | p0_b1c.log（作废） |
| v3 resume 链 | chat 建 session + resume 逐条 | **resume 恢复旧 goal**（"任务目标已恢复: 什么是Rust…revision 1"——新输入不入 goal）→ A 侧填充轮全部变成重复回答第一问 | p0_a1c.log |

**附带发现（NEW F1 登记）**：
1. **REPL 不可程序化驱动**（F1）：管道 EOF 忙等刷 prompt + 非 TTY 审批策略降级 + PTY 输入消费竞争——**直接阻塞拟真测试台 SimUser 对 repl 形态的驱动**（测试台需改用 PTY 交互驱动器或 file_issue 路线）。
2. **resume goal 语义**：`hearth resume <id> "<text>"` 恢复旧 goal（text 作为用户消息入队，goal 不更新）——设计语义如此，但与"逐字重放 REPL 输入序列"的直觉不符，harness 必须按此语义设计。

**部分信号（不作结论，仅登记）**：resume 链下 polluted 继续型 3/3 failed；A 侧因 goal 失真无效。**RC52 维持 "observed attractor, causal attribution incomplete"**——判定矩阵（fresh 继续型 × polluted 继续型）需正确的驱动器（PTY 交互式 driver，SimUSER-01 依赖）才能闭合。

## 5. Node 03 — 污染载体定位

按 STOP-① 未获得 fresh/polluted 对照数据 → 污染载体判级 **UNKNOWN**。既有时序事实维持：摘要链假说已否证为起因（首压 run-024 晚于入口 run-013 十一轮）；T4 必要非充分（run-003 T4 而成功）；RC47 族决策层路径为最高优先假设（待正确驱动器验证）。资源优先级维持：RC47 族 > 会话状态残留 > 摘要放大器。

## 6. Node 04 — Q-C：截断点 + give_up/completed 共存

**BUG-012 截断点定位**：Projection 层终态分支只渲染 `✓ Done (N steps)` + 报告路径指针；**分析正文只存在于 run-XXX.md 文件**（盲测 run-010 用户原话："我需要猜你的结果"）→ **completion projection completeness 缺口**（NEW F1，并入 RC51 completeness 行）。

**give_up 字样与 Task completed 共存（砺 probe 观察）= FALSE POSITIVE**：
- 反证：`give_up` 字样出现在 run 报告的**"反思轨迹"区**（reflect 判定的历史记录）；终端 `✓ Done` = RC47 路由（criteria 空+产物在→盲区C 产物校验）后的**最终终态**。两者是不同层级的记录（判定轨迹 vs 终态投影），非同一状态的矛盾。
- 反证核心：RC47 路由必经盲区C 确定性产物校验——completed 携带产物证据，非"give_up 被投影为成功"。
- **仍登记可观测性缺口**：`GIVE_UP_OVERRIDDEN` 标记在 scratch（internal-only），终端不显示"为何放弃意图后仍完成"→ 用户困惑合理。归入 RC51 completeness/completeness 行。

**completion 四层测法建议**：false_completion（既有）/ completion_evidence（产物+验证，已有）/ **completion_projection**（双通道 diff：报告正文 vs 终端渲染——SimUser 落地后可测）/ **user_verifiable_completion**（用户能否仅凭终端判断结果——双通道 diff 的用户侧指标）。

## 7. Node 05 — RC53/RC54 定位（只定位不修 ✓）

- **RC53**：盲测 L2602 `bash: 行 1: introspect: 未找到命令`（exit 127）——模型把 introspect 写进 bash 命令串（工具存在且曾正常调用 L112）。**层归 = model（tool selection）**；贡献因子 = introspect 输出的 shell 风格渲染（"=== 当前会话状态 ==="）诱导 bash 复现。修复方向：bash 工具对已知内部工具名返回结构化提示（"introspect 是内部工具，请用工具调用"）。
- **RC54**：apply_patch 空白/缩进脆弱匹配——源码定位 apply_patch 匹配逻辑；最小复现 fixture **DEFERRED**（本轮时间盒尽，登记 EVIDENCE GAP，下批立项首件）。

## 8. Node 06 — CLOSURE claim 冲突核查（invariant mapping）

| CLOSURE claim | 新观察 | 冲突? | 处置 |
|---|---|---|---|
| F0=0 | Q-A 泄漏未投影为成功；Q-C FALSE POSITIVE | 否 | 维持 |
| Resume reteach=0 | resume 恢复旧 goal（目标持久） | 否（一致） | 维持 |
| RC47 ACCEPTED | RC47 族 3/3 failed（resume 链污染侧） | 否（支持既有 ACCEPTED） | 维持 |
| Projection=PASS | 裸 ERROR 绕过投影 + BUG-012 completeness | **冲突 → reopen** | **NEW F1（RC51 拆分）** |
| REPL/interactive 边界 | REPL 不可程序化驱动 | 否（设计如此）但可用性缺口 | **NEW F1** |

## 9. Dispositions（最终）

| 项 | disposition |
|---|---|
| Q-A 泄漏路径 | CONFIRMED → **NEW F1**（Projection isolation+completeness，RC51） |
| Q-B 24 连败因果 | **EVIDENCE GAP**（重放器失真，STOP-①；RC52 归因待正确驱动器） |
| Q-C 截断点 | 定位 → **NEW F1**（completion projection completeness，RC51） |
| Q-C give_up/completed | **FALSE POSITIVE**（反证已写）+ 可观测性缺口并入 RC51 |
| RC53 | 定位完成（model 层）→ FIX-01 候选 |
| RC54 | 定位完成，fixture DEFERRED |
| REPL 程序化驱动 | **NEW F1**（SimUser 阻塞项） |
| resume goal 语义 | 登记为设计语义（harness 约束） |

## 10. Provenance

- 全部实验在 .133（盲测主机）；dump 构建 sha256 `bffe2dc0…`；重放剧本 + 12 会话日志 + dump 目录已归档 `.133:~/fa/` 与本仓库 `docs/data/p0-attribution-20260831/`。
- 生产代码改动 = env-gated dump（总包 §1.2 唯一许可）；**零修复** ✓；零 STOP-②③ 触发；STOP-① 触发一次（Q-B，按纪律停在证据边界）。
