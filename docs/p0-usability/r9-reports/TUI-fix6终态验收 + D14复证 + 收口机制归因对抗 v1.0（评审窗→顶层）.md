# TUI-fix6 终态验收 + D14 复证 + 收口机制归因对抗 · v1.0

- **窗口**：砺·评审（评审/守门人窗口）
- **时间**：2026-09-13 02:29–02:40 CST
- **目标机**：`.133`（192.168.220.133），**本机零执行**，全程只读 + 1 次真 LLM 探针
- **交付对象**：顶层
- **一句话结论**：任务书基线**第 9 次过期**（称 step3 在跑，实测早已终态）；TUI 车道 fix6 终态经独立复验**四阶段 ①✅②✅③⚠️④⚠️**，D14「答案不在视口」**复证成立**；本轮最高价值不是 TUI，而是**推翻了 `docs/hearth-issues.md` 对"简单任务几十轮"的归因**——真机制是 `[convergence]`（progress 仅认写类工具，阈值 3），不是它说的 `[budget-warn]/[budget-critical]`（实测触发 0 次）。

---

## 0. 基线判定（第 9 次过期，先实测再决策）

| 任务书声称 | 实测（2026-09-13 02:29:04 CST） |
|---|---|
| step3（接 hearth 真交互）正在跑 | **无 ember 进程**（`pgrep -f '[e]mber\.py'` = 0） |
| run-step3.log 未终结 | `run-step3.log` 早为终态：三行「达到 20 轮工具循环上限」（末次 09-12 17:02） |
| 工作副本待推进 | `src/main.rs` md5 **`620ee919af509efd9c4cb566ac982508`** / 23,152 B / mtime **01:19:40**，自 fix6 完工后**逐字节未变**；二进制 1,041,840 B / mtime 01:19:51 |

⇒ TUI 车道 **step1–step6 / fix1–fix6 全部终态且已验收**，本轮无「在跑件」。
⇒ 真实在飞的是**用户本人的真机使用会话**（§3.1）——这才是本轮该看的东西。

> 纪律留痕：基线已连续 9 轮过期。本轮仍按「先实测（日期 + 进程 + mtime/md5）→ 再决定动作」执行，未按任务书声称状态直接动手。

---

## 1. 本轮验收（任务书指定流程，逐条照做）

探针：`/tmp/tui-accept-v19.sh`（本窗自建，md5 `e39f872b5039e8237381a36edc125954`，4,954 B，LF；**v1–v18 一概未动**）
日志：`/tmp/tui-accept-v19.log`（243 行，追加制）　抓屏：`/tmp/v19-cap/`

| # | 任务书要求 | 实做 | 结果 |
|---|---|---|---|
| 1 | `cd /home/wutao/hearth-tui-new && cargo build --release` | 已做（前置 `export PATH=$HOME/.cargo/bin:$PATH`） | `Finished release profile in 0.09s`，**build_rc=0**，二进制 md5 **`79f0177b7a710b6cc84ecca4e59876f8`**、1,041,840 B ⇒ **无待编译改动，源码↔二进制同步** |
| 2 | tmux 启动 TUI | `tmux new-session -d -s rv19 -x 100 -y 24 -c <dir> <binary>` | 启动即 `[fold:off scroll:0/0] ready`，conversation 区**无预置假对话**（N4 修复保持有效）✔ |
| 3 | `send-keys` 输入 `只回答一个数字：3+4=?` + Enter | 已做，**0.6s 内分两次 send** | **Enter 后 6 秒内出现 `done (4s)`**，界面不卡、无 `running` 残留 |
| 4 | `capture-pane -p` 抓屏 | 100×24（真实视口）+ 200×50（全量）双抓 | 见 §2 |
| 5 | 判定四阶段 | 见 §2 | ①✅ ②✅ ③⚠️ ④⚠️ |

**真交互铁证（200×50 全量抓屏节选）**：

```
> 只回答一个数字：3+4=?
·   ▸ span [plan] 2026-09-12T18:31:00.232136104+00:00
· ───── 结果 ─────
  7  ◂ span 耗时 2601ms
  7✓ Done (1 steps)
·   ── 收尾 | 改了什么: 本轮无产物落盘
·   ── 收尾 | 还剩什么: 账本无未完成项
·   ── 收尾 | 依据: 完成决策 = accepted: all_done gate + verify passed；验证状态 = UNVERIFIED
·   📄 执行报告: /home/wutao/hearth-tui-new/.hearth/reports/cea3a9a8-…/run-001.md
·   ✓ Task completed（1 步）——目标达成（未验证），产物见报告
·   ── 收尾 | 改了什么: 本轮无产物落盘          ← 重复第 1 处
·   ── 收尾 | 还剩什么: 账本无未完成项          ← 重复第 2 处
·   ── 收尾 | 依据: … 验证状态 = UNVERIFIED     ← 重复第 3 处
· ═══════════ 任务总结 ═══════════
```

- 会话产物：`.hearth/reports/cea3a9a8-5cdf-4a77-a2d8-8351d90bff55/run-001.md`（报告目录数 **73 → 74**，实体存在）
- `Ctrl+Q` → 会话销毁 OK；收尾无残留 `hearth chat`；`ht3` / `t` 两个既有 tmux 会话**未动**
- 本轮真 LLM 调用 **1 次**，全程串行

---

## 2. 四阶段判定

| 阶段 | 判定 | 依据 |
|---|---|---|
| ① 功能有没有 | **✅** | TUI 启动 / 输入 / 生成子进程 / 流式回显 / spinner / 折叠 / 滚动 / 历史 / 退出，全部具备 |
| ② 能不能用 | **✅** | 问句 → `7`，4 秒；无卡死、无残留 `running`；会话与报告落盘正常 |
| ③ 好不好用 | **⚠️** | **D14：真实视口下答案看不见**（§2.1） |
| ④ 好不好看 | **⚠️** | 样板重复 2 遍 + `7✓` 缺空格 + 折叠态弹点（§2.2） |

### 2.1 ⭐ D14 复证（本轮最强 TUI 证据）：100×24 真实视口，答案不在框内

100×24 抓屏（`/tmp/v19-cap/10-viewport.txt`）的 conversation 区**共 16 行，逐行都是样板**：

```
│·   ── 收尾 | 依据: 完成决策 = accepted: all_done gate + verify passed；验证状态 = UNVERIFIED
│· ═══════════ 任务总结 ═══════════
│· 【一句话】已完成3+4的计算，答案为7。
│· 【产物】无
│· 【过程要点】
│· - 接收输入目标：只回答一个数字：3+4=?
│· - 执行计算：3+4=7
│· - 输出结果：7
│· 【问题与处理】无
│· 【剩余/建议】无，任务闭环
│· 【质量自检】未跑
│· （完整过程: .hearth/reports/<session>/ 下本轮 run 报告）
│· ══════════════════════════════
```

结构解析（探针内联 Python，锚**控件边框行**而非前缀——沿 A6 / v12 教训）：

```
total_lines=24  box_rows=16  viewport_rows=24
box spans screen rows 2..23
ANSWER_CANDIDATE: none found inside the conversation box
question_echo_rows=[6, 9, 10]      running_residue_rows=[]
```

⇒ 状态栏 `[fold:off scroll:0/17]`：**答案两行（`7  ◂ span 耗时 2601ms` / `7✓ Done (1 steps)`）在视口上方第 17 行处**，被整块样板顶出屏幕。
⇒ **用户每次提问都要额外按 Ctrl+T 或 PageUp 才看得到答案**——这是 TUI 车道现存唯一的 P0 体感问题。

**对照（本轮同时实测）**：`Ctrl+T` 折叠后，视口只剩 8 行，**答案进入可见区且被保护**：

| 特征串 | fold OFF | fold ON |
|---|---|---|
| `任务总结` | 1 | **0** |
| `【一句话】` | 1 | **0** |
| `【质量自检】` | 1 | **0** |
| `收尾` | 6 | **0** |
| `◂ span`（答案本体） | 1 | **1** ← 永不折叠 ✔ |
| `✓ Done`（答案本体） | 1 | **1** ← 永不折叠 ✔ |

⇒ **`fold` 默认 ON = 1 行改动即解 D14，且本轮已实测有效**。这属**默认呈现策略**，评审窗**未代手改**，待顶层定调。

### 2.2 ④ 好看：重复与标点

1. **`── 收尾` 三连在 fold OFF 下出现 6 次（2×3）** ⇒ 引擎侧重复输出 **N2a 第 4 次复现**（前 3 次：22:45 直跑 stdout 31 行、23:48 D14 归属实验、本轮 canonical 抓屏）。TUI 只是忠实转发。
2. `7✓` —— 答案 `7` 与 `✓ Done` 之间**缺空格**（答案未换行）。
3. `· ═══════════ 任务总结 ═══════════` —— 折叠/未折叠都带 `· ` 弹点（**N6**）。

---

## 3. ⭐ 本轮最高价值：两条「报告 vs 源码」对抗结论

### 3.1 用户真机使用：一场 33 轮会话（01:29–02:26），产物由 hearth 自己写

`.hearth/runs/f852f409-409f-473a-a698-379606afbcde.json`：

```
original_goal   : 你告诉我你能不能生成图片啊
current_goal    : 好的，那么我们今天的谈话先到这里吧
goal_revision   : 33            budget.max_steps: 200   tier: standard
session_written_files: notes/svg_example.svg(2111B) / egress_allowlist.py(1969B) /
                       tools/__init__.py / tools/bash.py(711B) / …
```

- `run-001.md`（01:29:47）→ `run-033.md`（02:26:50），**33 轮、57 分钟**，一场从"你能生成图片吗"聊到"今天先到这里"的连续会话。
- `.hearth/reports/f852f409-…/` 与 `tools/`、`docs/`、`egress_allowlist.py` **同一时段**生成。
- `~/.config/hearth/diagnostics.log`（UTC，+8 即 CST）逐条钉死归属：
  - `18:22:59Z` `tool=write_file target=docs/research-codex-claude-code.md`
  - `18:23:11Z` `tool=write_file target=docs/next-gen-ember-plan.md`
  - `18:26:07Z` `tool=write_file target=docs/hearth-issues.md`
  - 全部带 `WARN agent_core::r#loop` / `ERROR agent_core::scheduler` ⇒ **是 hearth（引擎）在写，不是人工，也不是 ember**
- 同段日志显示它在自建 ember v2 的 `tools/` 包时反复自纠：
  `apply_patch rejected: search block not found`、`AssertionError: tools not callable`、`AttributeError: 'function' object has no attribute 'run'`。

⇒ 结论：**`tools/` 包 + 三份计划文档 = hearth 在用户会话内的自产**。这是一次**真实用户任务**（结果导向第一原则里唯一有意义的样本），而不是任务书里的 step3。

### 3.2 ⭐⭐ `docs/hearth-issues.md` 把「简单任务几十轮」**归错了因**

该文档 §2.1（hearth 自己写的问题总结）把根因归到：

> `[budget-warn] 预算已用 83%…请评估剩余工作能否在预算内完成`
> `[budget-critical] …立即收口：停止开启新子任务…`

并据此给出 P0 修法：「把 `budget-critical` 从"禁止新任务"改为"优先收敛"」。

**实测（`~/.config/hearth/diagnostics.log` 2.5 MB 全量 grep）**：

| 特征串 | 命中次数 |
|---|---|
| `budget-warn` | **0** |
| `budget-critical` | **0** |
| `convergence directive` | **86** |
| `steps_without_progress` | **86** |
| `give_up` | 512 |

**真机制（源码锚点 `crates/agent-core/src/loop.rs:3760-3772`）**：

```rust
if self.steps_without_progress >= 3 && !self.progress_nudged {
    self.progress_nudged = true;
    self.ctx_mgr.add_user_message(format!(
        "[convergence] 已连续 {} 步无新事实且无产物变更。强制收口：\
         要么用纯文本向用户提出具体问题（结束本轮等待回答），\
         要么立即交付现有成果并明确说明缺口；禁止继续同类探索。",
        self.steps_without_progress));
    tracing::warn!(…, "R7-5 A-2-1: convergence directive injected on A-arm");
}
```

**关键口径（`loop.rs:3741-3746`）**：事实级 progress **只认 `write_file` / `apply_patch` 成功**——

```rust
// A-2-1: 事实级 progress 计数（A 臂口径同 W3 双轨——write 类成功才算
// 事实进展；read/glob/grep/bash 不算）。
.filter(|tc| tc.name == "write_file" || tc.name == "apply_patch")
```

⇒ **调研 / 分析 / 规划类任务**（正是用户在做的事）绝大多数步是 `read` / `grep` / `bash`，
⇒ **连续 3 步就注入"禁止继续同类探索"** ⇒ 用户体感 = "简单问题也要几十轮"。
⇒ **该文档建议的 P0 修法即使照做也无效**——`budget-critical` 那条路径**根本没被触发**。
⇒ 真正的杠杆是 **progress 的定义（仅写类）+ 阈值 3**。

> 方法论注记：这是「**静态处方清单 vs 实测证据**」的又一起正面案例（D-9 第一案同族）。
> 且它是**引擎自己给自己开的药方**——"被测对象自述"再次不可信，与铁律 #1 一致。

### 3.3 出网白名单：是环境变量，不是引擎缺陷

- `.bashrc:126`：`export HEARTH_EGRESS_ALLOWLIST=rust-lang.org,crates.io,docs.rs,doc.rust-lang.org,github.com`
- hearth 的 `web_search` 走 duckduckgo / bing ⇒ **不在白名单 ⇒ 必拒**（文案锚点 `loop.rs:9773`：「出网被拒：域名 X 不在 HEARTH_EGRESS_ALLOWLIST 白名单——联网工具是唯一受控出网口」）。
- ⇒ `docs/research-codex-claude-code.md` **一条可点链接都没有，不是 bug，是白名单把搜索源关在门外**；该文档正文亦自认「DuckDuckGo/Bing 回退源返回的命中**通常包含**…」（= 模型先验，非检索所得）。
- ⇒ **建议把这份"对标资料"降级标注为「未检索/未得链接」**，否则下游（`next-gen-ember-plan.md` 整份计划）会把先验当事实用——而 `next-gen-ember-plan.md` 第 5 节正是据此提出 Hearth 修正项。
- 修法（一行）：把 `html.duckduckgo.com,duckduckgo.com,bing.com` 加进白名单。**改的是用户的 `.bashrc`，属跨边界，本轮未执行，待确认。**

### 3.4 两个 hearth 二进制并存：用户命令行走的是 6 天前旧件

| 二进制 | 版本 | mtime | 大小 | md5 |
|---|---|---|---|---|
| `/usr/local/bin/hearth`（root） | **0.2.25 (ebd2f52)** | 2026-09-07 12:58 | 8,917,976 B | `9ec07d9d…` |
| `/home/wutao/hearth-slim/target/release/hearth` | 0.2.27 树 | 2026-09-12 02:20 | 8,768,952 B | `cee2f0f5…` |

- TUI 走**绝对路径**（`src/main.rs:24` `const HEARTH_BIN = "/home/wutao/hearth-slim/target/release/hearth"`）⇒ **TUI 车道测的是 0.2.27 ✔**（此前一轮若按 PATH 推断会误判，此处更正）。
- 新鲜度核验：`find crates -name '*.rs' -newermt "2026-09-12 02:21"` = **空** ⇒ 该树二进制不落后于源码。
- 但**用户命令行的 `hearth`（PATH）命中 `/usr/local/bin/hearth` = 0.2.25**。`f852f409` 的 `max_steps=200`，而 TUI 硬编码 `--budget 40`（新会话）/ `30`（resume）（`src/main.rs:312-330`）⇒ **该会话非 TUI 发起**，应为 `hearth repl`（bash_history 中 `hearth repl` 反复出现）⇒ **很可能跑的是 0.2.25 旧件**。
  - 状态：**推断，未定案**（`repl` 默认预算源码锚点**未取到**，故不作强结论）。
- ⇒ 影响：用户"简单任务几十轮"的体感，若来自 `repl`，则测的是 **09-07 旧构建**，与 R9 手术后的 0.2.27 不同尺。**建议同题双通道各跑一次做对照实验**——本轮**未做**（避免与用户在场会话并发）。

---

## 4. 存证与口径分树（**禁混标**）

| 口径 | 值 |
|---|---|
| `src/main.rs` md5 | **`620ee919af509efd9c4cb566ac982508`**（fix6 终态，01:19:40 后未变） |
| TUI 二进制 md5 / 大小 | **`79f0177b7a710b6cc84ecca4e59876f8`** / 1,041,840 B |
| 引擎二进制 md5 | `cee2f0f5…`（0.2.27 树） vs `/usr/local/bin` `9ec07d9d…`（0.2.25） |
| 本轮探针 | `/tmp/tui-accept-v19.sh`（md5 `e39f872b…`，4,954 B，LF） |
| 本轮会话 | `cea3a9a8-5cdf-4a77-a2d8-8351d90bff55`（reports 73→74） |
| 用户会话 | `f852f409-409f-473a-a698-379606afbcde`（33 轮，01:29:47→02:26:50） |
| 收敛指令源码锚点 | `agent-core/src/loop.rs:3760-3772`（触发）/ `3741-3746`（progress 口径） |
| 日志计数（全量） | `budget-warn`=0 / `budget-critical`=0 / `convergence directive`=86 / `give_up`=512 |

**证据归档**（本机 + `.133:/home/wutao/r9-reports/` 双侧）：
`docs/p0-usability/r9-reports/data/` 下 11 件 `v19-*` / `hearth-issues-hearth-authored.md` / `research-codex-claude-code-hearth-authored.md` / `next-gen-ember-plan-hearth-authored.md` / `f852f409-user-session-run.json` / `v19-run001-report.md`。

**收尾纪律**：本机零执行；日志只追加（未删未覆盖任何旧档）；未 kill 任何他人进程；`ht3` / `t` 会话保留未动；本轮 LLM 调用 1 次、无并发。

---

## 5. 待顶层裁决

| 优先级 | 事项 | 性质 | 评审窗动作 |
|---|---|---|---|
| **P0** | **`fold` 默认 ON（1 行）**——本轮已实测"折叠后答案进视口"，直接解 D14 | 呈现策略定调 | **未改，待定调** |
| **P0** | **引擎卡「progress 口径（仅 write 类）+ 阈值 3」**——解"简单任务几十轮"**真因**（非 budget-critical） | 动 hearth-slim 需授权 | 已定位锚点，未动 |
| **P1** | 出网白名单加搜索源（改 `.bashrc:126`） | 跨边界（改用户环境） | **未执行** |
| **P1** | 两个 hearth 二进制是否统一（`/usr/local/bin` 0.2.25 vs 0.2.27） | 运维/环境，影响可比性 | **未执行** |
| **P1** | 打磨卡（N6 弹点 / O4 折行几何 / N7 折叠状态机） | 小卡草稿已备 | **未发射** |
| **P2** | 把 `research-codex-claude-code.md` 降级标注「未检索」 | 事实标注 | 未动 |
| **P2** | 引擎卡·收尾三连去重 + 答案后置（N2a 第 4 次复现） | 动 hearth-slim 需授权 | 未动 |
| 存量 | H1/H2/H3/H4/H7/H8/H9/H10 + M5.2 的 F1/F2 + D13 + M5/M6 收官 | 待裁 | — |

---

## 6. 下一张卡（草稿，**未发射**）

见同目录 `tui-polish2-draft-card.md`。**未发射理由（显式声明，非遗漏）**：

1. 用户 02:26 前仍在同一台机上跑真机会话，02:29 才转入空闲——**并发铁律优先**，不与其争 LLM 预算与测量窗口；
2. 卡内首选项（`fold` 默认 ON / 答案自动滚入视口）属**呈现策略**，须顶层定调后再落码；
3. TUI 车道已 step1–6 / fix1–6 全绿，**无阻塞性缺陷**需当夜解堵（对比 fix5/fix6 的自跑情形）。

> 若顶层批复"照发"，按「小卡 + `setsid nohup bash -c` + 日志按实例命名（H10）」三件套启动，启动前必探 `pgrep -af '[e]mber\.py|[h]earth chat'`。
