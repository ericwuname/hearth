# 【草稿·未发射】引擎卡 A/B —— `hearth-slim` 输出通道分流与答案换行

> **状态：未发射。** 发射前置条件：**顶层明确授权触碰 `hearth-slim`**（评审窗无此权限，不得自行施工）。
> 立卡人：评审窗（砺·评审）　立卡时间：2026-09-13 07:5x
> 依据报告：`TUI-任务书指定验收（r14）+ 答案粘连与样板占比定案 v1.0（评审窗→顶层）.md` §3.3 / §3.4

---

## 一、共同背景（实测事实，勿再探索）

用 TUI 实际调用的同一 binary 直连取原始 stdout：

```
cd /tmp/r25w && /home/wutao/hearth-slim/target/release/hearth chat '只回答一个数字：3+4=?' \
  > /tmp/r25-stdout.txt 2>/tmp/r25-stdout.err
```

| 观测 | 数值 |
|---|---|
| stdout 行数 | **31**（含锁提示、目标锚定、span、收尾×2 份、总结块） |
| stderr 行数 | **1**（`diagnostics → …`） |
| 答案所在 | L10 `7  ◂ span 耗时 14866ms` / L11 `7✓ Done (1 steps)` —— **无一行仅含答案** |
| 纯重复 | L17–19 ≡ L12–14（`── 收尾` 三连块**打印两次**） |
| 答案占比 | 2 / 31 行 = **6.5%** |

关键源码锚点（**均已核过，勿再 grep 探索**）：

| 锚点 | 事实 |
|---|---|
| `crates/codex-cli/src/render.rs:41-45` | `outln!` → `println!`（带换行） |
| `crates/codex-cli/src/render.rs:48-53` | `out!` → `print!`（**无换行**） |
| `crates/codex-cli/src/render.rs:132-145` | `token(delta)` = `ensure_result_separator(); out!("{delta}")` ⇒ **答案流无收尾换行** |
| `crates/codex-cli/src/render.rs:200-217` | `done()` → `outln!("{icon} Done ({steps} steps)")` |
| `crates/codex-cli/src/render.rs:262-273` | `span_close()` → `outln!("…◂ span 耗时 {duration_ms}ms")` |
| `crates/codex-cli/src/lib.rs:374-388` | `set_headless(true)` **仅**在 `-p`/`--print` 路径调用 |
| `crates/codex-cli/src/run_local.rs:1033/1041/1049` | 收尾三行（one-shot 路径） |
| `crates/codex-cli/src/run_local.rs:1124-1186` | 收尾三行**副本**（REPL 路径）← 重复块疑似来源，**需你确认调用关系** |

---

## 二、卡 A（推荐，优先）—— K-2 分流下沉到 `chat`

**目标**：`hearth chat "<goal>"` 在 **stdout 非 tty** 时自动进入 headless 语义 ⇒ 答案走 stdout、进度/收尾/总结走 stderr。

**理由（三句）**：① `outln!`/`out!` 宏**已**按 `headless()` 自动分派，机制现成；② TUI 已把 stderr 行映射为 `Kind::Proc`（可折叠），stdout 行映射为 `Kind::Answer` ⇒ **TUI 零改动**即得到"结果通道只剩答案"；③ 一处改动同解 **O9 答案粘连 + N2a 样板重复 + O7 次序不一致 + D14 答案被埋**。

**改动约束**：
1. 只动分流决策点（`lib.rs:374-388` 附近），**不改** `render.rs` 的渲染内容；
2. 触发条件必须窄：`stdout` 非 tty **且** 非交互（TUI 的 `stdin=null` + piped 正是此形）。**交互式 REPL 行为必须逐字节不变**；
3. 正文（`token()`）**必须留在 stdout**，不得随之进 stderr；
4. 禁止顺手重构其它模块。

**验收（自测必须能失败，先红后绿）**：
| # | 断言 | 期望（新） | 期望（旧，红） |
|---|---|---|---|
| A1 | `hearth chat '只回答一个数字：3+4=?' >o 2>e` 后 `grep -c '收尾' o` | **0** | 6 |
| A2 | 同上 `grep -c '收尾' e` | **6** | 0 |
| A3 | stdout 中 `grep -c '^7$'`（**纯答案行**） | **≥1** | 0 |
| A4 | 交互式 `hearth repl` 手工跑一题，输出与术前逐字节一致 | 一致 | — |

**证据落盘顺序（H8：证据先落盘再收尾）**：先写 `evidence-engineA.log`（含 A1–A4 四条的真跑输出），**再**写总结。

---

## 三、卡 B（后备，1 行级）—— 答案流补收尾换行

若卡 A 未获批，最小修法：答案流结束时补一次 `outln!()`，使**答案独占一行**。

**改动约束**：
- 位置：`render.rs` 中答案流结束处（`token()` 的调用方收尾 / 或 `done()` 前），**1 行级**；
- 不得引入"答案重复打印"（现 stdout 已出现两次 `7`，需你确认是设计还是缺陷后一并处置）；
- 交互式行为不变。

**验收（能失败）**：直连跑同题 → `grep -cE '^7\s*$' stdout` **≥1**（旧值 **0**）；`grep -cF '7✓' stdout` = **0**（旧值 1）。

---

## 四、硬纪律（写死，违反即无效）

- `export PATH=$HOME/.cargo/bin:$PATH`（非登录 shell 无 cargo，已烧过多轮）。
- 日志**只追加**（`>>`），**按实例分名**：`run-engineA-<实例>.log` / `trace-engineA-<实例>.err`（H10）。
- 动源码前**先做时间戳备份**并**校验非零**（H9：`cp` 曾产出 0 字节档）。
- 判"进程已死"须**双条件**：进程表无命中 **＋** 日志有 `end` 哨兵行。
- **禁止探索式侦察**：本卡已给全部行号锚点；只读所列行，不做全仓 grep 漫游（H11：探索膨胀 → 单请求超 180s → 重试吃光墙钟 → 被 `timeout` 强杀在半成品态）。
- **禁改** `hearth-tui-new/src/main.rs`（本卡 TUI 侧零改动；动了即破坏实验效度）。
- **不得**在 `.131` 跑（需在 `.133` `/home/wutao/hearth-slim` 原地构建）。
- 墙钟留余量：建议 `timeout 1800`；自保动作写死（被 kill 时无法自保，兜底归评审窗）。
