# P5-FOUNDATION-01 · 设计规格冻结（N00）

**日期**：2026-09-01｜**执行**：执行窗（段 1 零代码）｜**状态**：待顶层签发冻结
**依据**：`Hearth P5-FOUNDATION-01 骨架地基加固施工总包 v1.0（顶层定版）.md` §4 N00
**纪律**：所有行号锚点**已重新 grep 实测**（不采信旧文档行号）；与 P4 Node 14 campaign 零交集（本文件为纸面产出，未动 `crates/`）。

---

## 0. 锚点实测结果（含两处旧文档行号更正）

| 总包给的行号 | 实测 | 判定 |
|---|---|---|
| `edit.rs:141-162` | ✅ 141、162 均为 `tokio::fs::write` | 一致 |
| `patch.rs:113/151` | ✅ 均为 `tokio::fs::write` | 一致 |
| `run_local.rs:501-505` save_snapshot | ✅ 调用点在 505（定义 `session_store.rs:44`） | 一致 |
| `lib.rs:1078-1085` 生产沙箱 | ✅ 1078 `StdCommand::new`、1081 `.envs()` | 一致 |
| dev `NoopSandbox:185` env_clear | ✅ 182-185 有 `.env_clear()` | 一致 |
| `constitution.rs:38` FALLBACK | ⚠️ **实际 `const FALLBACK` 在 :24，使用点在 :36** | **行号漂移 2 行，本规格以 :24/:36 为准** |
| `bash.rs:89` 610s | ✅ `Duration::from_secs(610)` | 一致 |
| S5 的 egress-allowlist | ✅ `web.rs` 消费 `HEARTH_EGRESS_ALLOWLIST`（由 `run_local.rs:223-228` 注入） | 一致 |

---

## 1. S1 — 工具层原子写（冻结区外）

**问题实证**：项目**已有** tmp+rename 原子写，但只用在**自己的状态文件**上：
`config.rs:78`、`session_store.rs:58/82/124`、`memory/src/lib.rs:147`、`project-sync/registry.rs:32`。
工具层（用户代码路径）仍是裸 `tokio::fs::write`：`edit.rs:141/162`、`patch.rs:113/151`——`edit.rs:141` **先 truncate 原文件再分块 append**，崩溃即毁原文件。

**规格**

```rust
// crates/tools-builtin/src/atomic.rs（新文件）
pub fn write_atomic(path: &Path, content: &str) -> std::io::Result<()>
/// 语义：同目录 tmp（.<basename>.hearth-tmp-<pid>-<rand>）
///      → write_all → sync_all → rename
/// 错误语义：任一步失败 → 返回 Err，**原文件字节不变**（rename 是原子最后一步）
/// 清理：rename 失败必须 unlink tmp（不留垃圾）；sync_all 失败视为硬失败（不允许"写了但没落盘"）
```

**收口点**：`edit.rs:141` 分块逻辑重写为「分块写 tmp、最后 rename」；`edit.rs:162`、`patch.rs:113/151` 短路径同样收口。

**先红后绿 fixture 设计表**

| # | 红测试输入 | 预期失败断言 |
|---|---|---|
| S1-R1 | 目标文件已存在 10KB；`write_atomic` 在 tmp 写入后、rename 前被注入 IO 错误 | 原文件 md5 与调用前**一致** |
| S1-R2 | 分块写（>64KB，跨 3 块），中途 kill -9 | 原文件完好（无半截内容） |
| S1-R3 | tmp 目录不可写 | 返回 Err 且原文件完好，且**无残留 tmp** |
| S1-G1 | 正常写入 | 内容正确 + 无 tmp 残留 + inode 变更（证明确经 rename） |

---

## 2. S3 — 增量 checkpoint（冻结区外，但触 loop.rs 调用点 → 走 FZ-RFC-4）

**问题实证**：`save_snapshot` 唯一调用点在 `run_local.rs:505`，且位于 `Ok` 分支内 → kill / 失败路径**丢整轮**。

**规格**

| 项 | 规格 |
|---|---|
| 触发 | 每 turn 结束（N=1 起步，压测后可调至 5 步）；对齐 Claude Code "write as events occur" |
| 事件 schema | `{"ts","sid","seq","kind":"turn","turn":{...}}`，append-only JSONL，与既有 session JSONL 同目录不同文件（`checkpoints.jsonl`） |
| 写入方式 | **复用 S1 的 append_atomic**（append 语义：读-拼接-原子替换，或 O_APPEND + 单行 ≤PIPE_BUF 保证） |
| 恢复语义 | `hearth resume` 时：若 checkpoint 序列 > 会话主文件 → 以最近**完整** checkpoint 为准，并在终端显式提示"恢复到第 N 轮（checkpoint）" |
| 幂等 | seq 单调；重放不得产生重复 seq |

**fixture**：kill -9 于 turn 3 → resume 后 history 含 turn 1-2（红：修复前为 0 轮）；checkpoint 文件损坏（末行半截）→ 回退到上一个完整 checkpoint，不 panic。

---

## 3. S2 — 改前快照 + 回滚（冻结区外）

**问题实证**：全仓 `rollback` 相关生产代码 **grep 为空** → 能力不存在。

**规格**

| 项 | 规格 |
|---|---|
| 触发 | `edit` / `patch` / `write_file` 执行**前** copy 原文件 |
| 落点 | `~/.config/hearth/snapshots/<sid>/<seq>_<basename>` |
| 保留策略 | 单会话上限 **200 个**或 **50MB**（先到先删，删最旧）；会话结束保留 7 天 |
| CLI | `hearth rollback <sid> [seq]`：无 seq = 回滚最后一个快照；有 seq = 回滚到该序号**之后**的状态 |
| 交互语义 | 回滚前打印「将回滚 N 个文件 + 文件清单」，需确认（非 tty 时结构化拒绝，复用 `approval_denied_noninteractive` 同款） |
| 错误语义 | 快照缺失 → 明确报错（不得静默跳过）；回滚写盘**走 S1 原子写** |

**fixture**：改 3 个文件 → rollback → 三个文件 md5 与改前一致（红：修复前无此能力）；快照超限 → 最旧被清且新快照成功。

---

## 4. S4 — env_clear（**冻结区**，走 FZ-RFC-1）

**⚠️ 本轮实测更正（推翻此前记录的风险定性）**：

- 生产路径确实**没有** `env_clear()`（`lib.rs:1078-1085`）；
- **但**传入的 `env` 不是父进程全量环境：`run_local.rs:219-236` 的 `tool_env(cfg)` **只注入两个变量**——`HEARTH_EGRESS_ALLOWLIST`、`HEARTH_READ_ROOTS`；`ToolContext::default()` 的 `env` 是空 HashMap；全仓无 `std::env::vars()` 喂给工具上下文。
- 另有 `bash.rs:163-178` 注入的 `PATH`（cargo bin + `/usr/local/bin:/usr/bin:/bin`）。

**结论**：**"API key 直透 LLM 子进程"不成立**（key 根本不在 ctx.env 里）。真实风险是**纵深防御缺口**——没有 `env_clear()` 兜底，未来任何人往 `ctx.env` 加东西就会无条件直达子进程；且沙箱内 bash 缺 `HOME`/`LANG`/`TMPDIR` 等常规变量，属"缺功能"而非"泄密"。
→ 风险等级由 🔴 降为 🟡，但 **S4 仍建议做**（作为兜底，成本低）。

**规格**

```rust
child
    .env_clear()
    .envs(最小白名单)   // PATH / HOME / LANG / TMPDIR + 既有 env_vars_clone
```

白名单枚举表（对标 dev `NoopSandbox:185` 语义）：

| 变量 | 必要性 | 来源 |
|---|---|---|
| `PATH` | 必需（bash 自身定位命令） | `bash.rs:163-178` 注入值 |
| `HOME` | 必需（cargo/git 等读 HOME） | 进程 env |
| `LANG` / `LC_ALL` | 中文输出正确投影 | 进程 env，默认 `C.UTF-8` |
| `TMPDIR` | 临时文件 | 进程 env，默认 `/tmp` |
| `HEARTH_EGRESS_ALLOWLIST` | web_fetch 白名单 | `tool_env(cfg)` |
| `HEARTH_READ_ROOTS` | 读范围限域 | `tool_env(cfg)` |
| **禁止透传** | 任何 `*KEY*`/`*TOKEN*`/`*SECRET*` 及父进程其余全部变量 | — |

**fixture**：沙箱内 `env | sort` → 仅含白名单 6 项（红：修复前仅 2-3 项且无固定兜底）；注入 `HEARTH_TEST_SECRET=1` 到父进程 → 沙箱内**不可见**。

---

## 5. S5 — 出网收口（**冻结区**，走 FZ-RFC-2；方案待 N01 裁决）

⚠️ **本轮实测出现与历史记录冲突的关键事实**（详见 `N01-S5-决策备忘录.md`）：
沙箱内 `curl` 对 example.com 与 baidu **均返回 000（连接失败）**，而同机沙箱**外** curl 正常（Agnes 401 / baidu 200）。
→ 即**"bash curl 出网零管控"这一旧结论在本环境不复现**，S5 的方案选择必须先辨明根因（DNS / CA 证书 / 网络三者之一），再决定 N16a 是否必需。

规格（待裁决后定稿，此处只锁接口形状）：
- 方案 ②（命令级预检 + 审批门）：网络命令词表（curl/wget/nc/ssh/scp/rsync/git+远程/…）+ egress-allowlist 域名校验；命中非白名单 → `InteractionRequested`（复用 `loop.rs:3293-3343` 同款 emit 路径）。
- 方案 ①（seccomp 硬拒 connect/bind/sendto/sendmsg，AF_UNIX 豁免）：须推翻 P3 Node 02「防线 B=不加位」裁定并留痕。
- 方案 ③（本地代理）：顶层倾向不做。

---

## 6. C-3 — 宪法回退告警（冻结区外）

**锚点实测**：`constitution.rs:24` `const FALLBACK`（编译内置旧版宪法），使用点 `constitution.rs:36` `load_constitution_file().unwrap_or_else(|| FALLBACK.to_string())`。

**规格**：FALLBACK 命中时 `tracing::warn!` + 结构化事件（`constitution_fallback_used`，含原因：文件缺失/读取失败）；`sanitize()` 截断处标记 —— **已于 v0.2.21 完成**（`constitution.rs:79-86`，带 `\n[... N chars truncated]`）。

**fixture**：`CODEX_CONSTITUTION_PATH` 指向不存在路径 → 事件出现且 warn 落 diagnostics.log（红：修复前静默）。

---

## 7. C-2 — 代价对账（观测层）

**锚点实测**：`declared_timeout()` trait 方法在 `tool-runtime/dispatcher.rs:66`；bash 声明 610s（`bash.rs:89`）；注册于 `run_local.rs:242`；effective 语义 `context.rs:22`（min(declared, remaining_task_time)）。

**规格**：工具实际耗时 > `declared_timeout × 1.2` → 产生 `timeout_declaration_drift` 事件（**观测级，不拦截**），字段 = `{tool, declared_ms, actual_ms, ratio}`。**顺带核查**：`bash.rs:89` 的 610s 与实测 869s 差值成因。

**fixture**：mock 工具声明 100ms、实际 200ms → 事件产生且 ratio=2.0（红：无事件）。

---

## 8. C-1 + C-12 — 终态验证标注（**冻结区**，走 FZ-RFC-3）

**顶层已裁定**：Terminal 九态枚举**不动**。

**规格**：`TaskResult` / 执行报告新增 `verification: VERIFIED | UNVERIFIED`；
判定 = run 全程存在 ≥1 条**实际验证命令**（读文件/列目录**不算**，须实测目标行为，如跑测试、执行二进制、请求接口）→ `VERIFIED`，否则 `UNVERIFIED`；
投影层对 `UNVERIFIED` 的 completed 渲染为「**目标达成（未验证）**」，禁止裸「目标达成」。

**fixture 语料**：`release/手工测试v0.2.19.txt` 的 run-001（37 步零验证）——红：标注为 VERIFIED（错误）；绿：标注 UNVERIFIED 且终端显示「（未验证）」。

---

## 9. 段 1 完成度

| 项 | 状态 |
|---|---|
| N00 规格冻结（本文） | ✅ 完成，待顶层签发 |
| N01 S5 裁决材料 | ✅ 完成（见 `N01-S5-决策备忘录.md`），**含推翻旧结论的实测** |
| 段 2（N10-N18 真施工） | ⛔ **未开工**——前置条件未满足（详见总包 §2）+ Node 14 版本冻结约束 |
