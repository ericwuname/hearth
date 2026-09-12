# hearth TUI step3/step4 验收与热键缺陷定案（评审窗 → 顶层）

- **日期**：2026-09-12 17:54–18:00 CST
- **口径**：评审窗独立实测（`.133`），本机零执行；证据链 `/tmp/tui-accept-v3.log`、`/tmp/tui-accept-v4.log`
- **目标机**：`.133`；工作副本 `/home/wutao/hearth-tui-new/`；施工者 EMBER

---

## 0. 一句话结论

**TUI 的"真交互"（step3）已实测打通、可用；step4 的三项"好用"能力亦实测生效。**
但交付过程有两处硬伤：① 源码一度**编译不过**（EMBER 未做自测）；② **`q` 键在空输入时退出程序**，导致用户打任何以 q 开头的词就直接掉线。评审窗已完成全部修复的独立复验，并**另定位出一处 step4b 未修干净的残留缺陷（1 行）**。

> **收口（18:03 更新）**：D1/D2/D3 三处**已全部修复并经评审窗独立复验关闭**。终态：`cargo build --release` 0 error；空输入打 `question` → 输入框完整显示且会话存活；`Ctrl+Q` 才退出；`3+4=?` → `hearth: 7`。详见 §9。
>
> **⚠️ 评审窗自我更正**：本报告早期版本曾记一条「H5：ember 被静默杀死」——**该结论错误，已撤回**（step4b 实为正常 `exit=0`，是我用单次 `ps` 探针误判）。更正与新纪律 H5' 见 §6。

---

## 1. 基线更正（收到的口径已过期约 55 分钟）

| 收到的基线（17:00） | 实测（17:54） |
|---|---|
| step3 正在跑 | step3 **已于 17:02 结束**：`run-step3.log` 两行均为「达到 20 轮工具循环上限，任务未收敛」 |
| — | step4 **已于 17:30 结束**：同样两行撞上限；`main.rs` 停在**编译不过**的半成品态 |
| step1/step2 已验收 | 属实。但 step3/step4 **均未产出 `evidence-step3.log` / `evidence-step4.log`**（两卡都明确要求自测落盘） |

**过程性事实**：ember 两轮各撞 20 轮工具循环上限，主要轮次消耗在反复试 `cargo build`——**`.cargo/bin` 不在非登录 shell 的 PATH 里**，每轮都要重新发现一次（`trace-step3.err` 可见「cargo 返回 42 字符报错 → 转去 ls ~/.cargo/bin → 才加 export」的循环）。

---

## 2. 决定性证据

### 2.1 真交互打通（评审窗 v3，17:55:44）

```
tmux send-keys -t v3 "只回答一个数字：3+4=?"   →   [t1] 输入回显：│> 只回答一个数字：3+4=?▌
tmux send-keys -t v3 Enter
[t2 = +5s ]  │[fold:off hist:1 scroll:0/0] - running... (4s)      ← 运行态+
[t3 = +25s]  │  hearth: 7                                          ← 正确答案
             │[fold:off hist:1 scroll:0/15] done (18s)             ← 终态
```
`·` 前缀过程行（执行报告路径、任务总结六项）完整流式上屏；**界面全程未卡死**。

### 2.2 输入回显不丢字符（评审窗 v5，17:59:21）

```
[A] │> team quest▌      ✅ 完整
[B] │> tell a joke▌     ✅ 完整（step4b 修复后）
[C] "question quality equal" → tmux: can't find pane → SESSION=DEAD  ❌ 见 D2
```

### 2.3 `q` 首字母隔离取证（评审窗 v5，17:59:42）——**决定性**

```
源码锚点: main.rs:168   KeyCode::Char('q') if input.is_empty() => quit = true,
T1  空输入直接打 "question"           →  SESSION DEAD        ❌
T2  先打 "ab" 再打 q（"abq"）          →  │> abq▌  SESSION ALIVE ✅
```
**结论**：`input.is_empty()` 门控**只挡住了 q 出现在非首位的情况**；只要输入框为空、用户按下的第一个字母是 `q`，程序立即退出、已输入内容全丢。**修法必须用修饰键**（`Ctrl+Q`），与 `Ctrl+T` 折叠保持一致。

### 2.4 修复复验（评审窗 v4，17:58:20）

| 项 | 源码锚点 | 结果 |
|---|---|---|
| 编译 | `cargo build --release` | ✅ `Finished`，0 error |
| `q` 门控 | `main.rs:168` | ⚠️ 部分（见 D2） |
| `t` 改 Ctrl+T | `main.rs:238` `if key.modifiers.contains(KeyModifiers::CONTROL)` | ✅ 打字 `tell a joke` 完整 |
| `cap_scroll` | `main.rs:261` `fn cap_scroll(...)` | ✅ 编译通过 |
| title 文案 | `main.rs:407` `Ctrl+T=fold` | ✅ |
| Ctrl+T 行为 | 状态栏 | ✅ `fold:on` → 再按 → `fold:off` |
| 真交互回归 | — | ✅ `hearth: 7`，`done (12s)`，SESSION=ALIVE |

---

## 3. 四阶段判定（step3 + step4b 合并后）

| 阶段 | 判定 | 依据 |
|---|---|---|
| ① 功能有没有 | ✅ | 四区块齐（顶栏/对话区/输入/状态栏）+ 真子进程 + 流式 + 运行态计时 + 历史/滚动/折叠 + 状态栏 `[fold/hist/scroll]`；472 行 |
| ② 能不能用 | ✅ | `3+4=?` → `hearth: 7`（18s / 回归 12s），全程不卡；沙箱行、`·` 过程行、执行报告路径均上屏 |
| ③ 好不好用 | ⚠️ | ↑历史召回 ✅、PgUp `0/51→5/51` ✅、Ctrl+T 折叠 ✅、不回滚 ✅；**但 `q` 掉线缺陷未除干净（D2）**，且状态栏 `session: demo`/`steps: 0` 为硬编码（D5） |
| ④ 好不好看 | ✅ | 4 对边框齐全（step2 的「只有上边框」缺陷已消失）、对齐/缩进整齐、`·` 暗色区分；⚠️ 长行按宽度**硬截断**（`run-001.m│`）而非省略号，观感像数据损坏 |

---

## 4. 缺陷清单与定案

| # | 级别 | 缺陷 | 状态 |
|---|---|---|---|
| D1 | P0 | `cap_scroll` 调用 vs `visible_line_count` 定义名不一致 → 2×E0425，**源码编译不过** | ✅ 已修（L261 改名），复验编译绿 |
| D2 | **P0** | **`q` 键退出吞首字母**：空输入下打 `question` 直接退出（L168 `is_empty` 门控不充分） | ✅ **已修（step4c）**：改 `Ctrl+Q`；T1 原 DEAD → 今 `> question▌`+ALIVE |
| D3 | P1 | `t` 键吞首字母（`tell a joke`→`ell a joke`） | ✅ 已修（Ctrl+T），复验绿 |
| D4 | P1 | `evidence-step3.log` / `evidence-step4.log` **两卡均未产出**（卡内明确要求） | ⚠️ 部分：step4c 已交 `evidence-step4c.log`（6747B，证据真实）；step3/4 两份永久缺失 |
| D5 | P2 | 状态栏 `session: demo` / `steps: 0` 硬编码，不反映真实 session（如 `e0d54e68`）与 steps（实际 1） | ⏸ 未开卡（低优先，仅影响观感/自检） |
| D6 | P2 | step4 的折叠自测断言「`·` 行数应为 0」**不可失败**（本就无 proc 行时也是 0）→ 假绿风险 | ⏸ 记录，后续卡统一要求"先有 proc 行再折叠" |

---

## 5. 已采取的动作

1. **step4b 卡**（≤15 步，只修编译 + `q` + `t` 三处）已发并启动（17:57:37）。
   - **中途误判与更正（评审窗自身失误）**：17:58/17:59 两次 `pgrep/ps` 未命中，我据此判定「进程静默消失、被硬杀」。**该判定错误**——终态显示 step4b **18:01:10 撞 20 轮上限后正常 `exit=0`**，`run-step4b.log` 有完整「结束」行，并留下 `evidence-step4b.log`。撤回见 §6 H5'。
   - 后果：因误判"已死"，我在 18:00:38 启动了 step4c，与 step4b 尾部**重叠约 32 秒**（18:00:38–18:01:10）。已核查：step4b 在 17:58 之后**未再改 `main.rs`**（仅 tmux 自测与写证据），**无编辑碰撞**；终态源码编译且功能全绿。
   - 评审窗据「不信报告信源码」**独立复验其编辑结果**：编译 ✅、功能 ✅（见 §2.4），**未代 EMBER 手改一行**。
2. **step4c 卡**（≤8 步，只改 1 行 + 补自测）已发并启动（18:00:38 启动 / **18:01:50 `exit=0`**，72 秒），评审窗 v6 独立复验全绿（见 §9）。
3. 本报告落盘：本机 `docs/p0-usability/r9-reports/` + `.133:/home/wutao/r9-reports/`。
4. 日志一律**追加制**，未删改任何既有文件；验收脚本新增 `/tmp/tui-accept-v3.sh`、`/tmp/tui-accept-v4.sh`、`/tmp/tui-accept-v6.sh`（不改动 `tui-accept.sh` / `v2` 原脚本）。

---

## 6. harness 层发现（累积）

- **H5（当场撤回，替换为 H5'）**：~~ember 可被静默杀死且不留 exit 记录~~ —— **该判定是错的，当场认错撤回**。事实：step4b 17:57:37 启动 → **18:01:10 撞 20 轮上限后正常 `exit=0`**，日志有完整「结束」行 + `evidence-step4b.log`。错误来源：评审窗在 17:58/17:59 两次 `ps/pgrep` 未命中，**用单次探针就断言"已死"**（成因未定，疑似长 cmdline 下 `ps` 输出截断，未复现）。
  → **新纪律 H5'（评审窗自查）**：**判定"进程已死"必须同时满足两条——进程表无命中 ＋ 日志有「结束」哨兵行**；只有前者不足为凭。这是评审窗自身违反"不信报告信源码也要信实测"的方法论事故，已记入 memory。
- **H7（连带教训）**：`run-*.log` 的「结束」哨兵行是唯一可靠的终态信号；建议所有启动器把「结束 exit=N」写成**带时间戳的独立行**（本轮 step4b/4c 均已具备，恰好暴露了探针误判）。
- **H1 再现（跨任务实证）**：TUI 抓屏同时出现 `✓ Task completed` 与 `验证状态 = UNVERIFIED`、`【质量自检】未跑`——done 门在 UNVERIFIED 下仍放行，已在 M5.1/M5.2 记录，此处第三次实证。
- **H6（新）**：**单字母热键在常驻输入框里是设计陷阱**——`q`/`t` 用「输入为空」做门控，必然吞掉首字母。凡常驻输入框，热键一律走修饰键（`Ctrl+X`）。此条建议升为 UI 卡通用约束。

---

## 7. 口径与环境登记

- 二进制口径：`main.rs` md5 `7170bd80…`（step4 半成品，17:30，编译不过）→ `7f24ff3e…`（step4b 修复后，18:00 编译 0 error）。
- 构建产物：`target/release/hearth-tui` 17:26（1,014,960 B，可用）→ 17:58:10（1,014,656 B，修复版）。
- 关键环境纪律：**每条 cargo 命令前必须 `export PATH=$HOME/.cargo/bin:$PATH`**——否则白烧轮次。
- 并存会话：tmux `ev4`（ember step4 遗留，17:26 起）、`t`（16:35 起）——**未动**；评审窗只起 `v3/v4/v5/q1/q2` 并在用后自行 kill。
- 并发：启动前实探 `pgrep -f ember.py` 为空，未见并发 LLM 任务。

## 8. 给顶层的建议

1. **收 D2 为"必过项"**：step4c 若不通过，TUI 不可交付——`q` 掉线对任何真实使用都是致命伤。
2. **补 D4 证据缺口**：step3/step4 两轮的 `evidence-*.log` 永久缺失，建议在 step4c 一并补齐（卡内已含）。
3. **立一条 UI 通用约束（H6）**：常驻输入框的单字母热键一律修饰键化，写进后续所有 TUI 卡的前置约束。
4. 其余（D5/D6）低优先，建议并入下一张"打磨卡"，不单独立项。

---

## 9. step4c 收口复验（评审窗 v6，18:02:31）

**step4c 卡**（≤8 步，只改 2 行：`main.rs:168` 改 `Ctrl+Q`、L413 title 文案）于 18:00:38 以 `setsid --fork` 启动，**18:01:50 `exit=0`**，全程 72 秒，产出 `evidence-step4c.log`（6747 B）。ember 自报与评审窗独立实测**一致**（自报诚实）。

评审窗独立复验（`/tmp/tui-accept-v6.log`）：

| 项 | 实测 | 判定 |
|---|---|---|
| 编译 | `Finished`，0 error | ✅ |
| 源码锚点 | L168 `Char('q') if key.modifiers.contains(CONTROL)`；L238 `Char('t') if ...CONTROL`；L261 `fn cap_scroll` | ✅ 三处全落地 |
| **T1 空输入打 `question`** | `│> question▌` + **SESSION ALIVE**（修复前为 **DEAD**） | ✅ **D2 关闭** |
| T2 打 `abq` | `│> abq▌` + ALIVE | ✅ |
| T3 `Ctrl+Q` | **SESSION DEAD**（退出键迁移成功） | ✅ |
| 真交互回归 | `hearth: 7`，`done (14s)`，`hist:1 scroll:0/16` | ✅ |
| 会话卫生 | `ev4`(17:26)、`t`(16:35) **未被扰动** | ✅ |

**D1 / D2 / D3 全部关闭。四阶段最终判定：①✅ ②✅ ③✅（热键缺陷清零）④✅（仅余长行硬截断的观感小瑕）。**

**残留观察（不阻塞）**：
- `main.rs` md5 演进：`7170bd80…`（step4 半成品，编译不过）→ `7f24ff3e…`（4b）→ **`f89d8d41…`（4c 终态）**。
- tmux 遗留两个**空闲** TUI 进程：`ev4`（PID 98008，step4 遗留）、`ev4b2`（PID 102403，**step4b** 自测遗留——step4b 已 kill 掉 `ev4b` 又新建 `ev4b2`，撞上限前未清理）。二者均无 LLM 调用，未动。
- `evidence-step4b.log`（162 B）质量备注：EMBER 用 `cat >> ... << 'EOF'`（**引号 heredoc**）写入，导致 `date: $(date)` **未展开、按字面落盘**；且只贴到 (a) 段就撞上限。→ 后续卡应要求"用 `$(...)` 不加引号的 heredoc，或分步 echo"。
- `evidence-step3.log` / `evidence-step4.log` 两份**永久缺失**，仅 step4c 补齐了证据习惯。
