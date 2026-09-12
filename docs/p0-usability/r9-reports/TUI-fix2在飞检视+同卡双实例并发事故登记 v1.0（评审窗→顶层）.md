# TUI-fix2 在飞检视 + 同卡双实例并发事故登记 v1.0（评审窗→顶层）

- **日期**：2026-09-12 21:29–21:35 CST
- **窗口**：砺·评审（只读守门）
- **目标机**：`192.168.220.133`（ssh wutao@…，所有命令经 paramiko 短连接执行）
- **对象**：`/home/wutao/hearth-tui-new/` 的固定卡「hearth TUI 修复-2（体验三件）」
- **判决**：**本轮不产验收结论**。理由见 §5。产出一条 **P0 事故**（同卡双实例并发）与一份**在飞静态核验**。

---

## §0 执行纪律声明

| 项 | 状态 |
|---|---|
| 本机（Windows）执行 | **零**。全部命令经 `paramiko` 在 `.133` 上跑 |
| 对 `.133` 的写入 | **仅新增一份报告**（`/home/wutao/r9-reports/`）。未改任何源码/日志 |
| 日志纪律 | **只读**。`run-*`/`trace-*`/`evidence-*` 一个字节未动 |
| 进程纪律 | **未 kill、未启动任何进程** |
| LLM 调用 | **零**。本轮全程无 LLM 任务发起 |

---

## §1 基线第 5 次过期（硬约束再次生效）

任务书称「正在跑 step3（基线 17:00）」。实测：

| 任务书声称 | 实况（21:29 实测） | 差 |
|---|---|---|
| step3 在跑 | step3 **17:02 已终态**（`run-step3.log` = 20 轮上限未收敛），且 step4/4b/4c/step5 全已结 | 已过期 **4.5 小时**，且中间**已发生 3 张新卡** |
| step3 完成后准备 step4 卡 | step4 **17:30 已结束并已验收**（见 18:03 轮报告） | 指令已失效 |
| — | **真实在跑的是全新卡「TUI 修复-2」**（21:13 启动） | 任务书完全未覆盖 |

> **纪律结论（连续 5 轮）**：任务书"当前进度"段**不得作为行动依据**。每轮必须先实测（`date` + `pgrep` + 文件 `mtime/md5`）再决策。本轮已按此执行。

`run-step3.log` 原文（178 B）：
```
（达到 20 轮工具循环上限，任务未收敛——请缩小问题范围重试）
（达到 20 轮工具循环上限，任务未收敛——请缩小问题范围重试）
```

---

## §2 ⚠️ P0 事故：同一张卡被启动两次并并发运行

### 2.1 事实

| 实例 | PID（timeout/python） | PPID 宿主 | 启动时刻 | 已运行 | stdout 去向 | 日志登记 |
|---|---|---|---|---|---|---|
| **A（幽灵）** | 119049 / 119050 | 119044 = `bash /tmp/fix2-run.sh` | **21:13:13** | 16:27 | **`run-fix2.log (deleted)`** | **无** |
| **B（登记）** | 119143 / 119144 | 119138 = `bash /tmp/fix2-run.sh` | **21:14:01** | 15:39 | `run-fix2.log` | `[21:14:02] fix2 start` |

两实例命令体**逐字相同**（同一 `/tmp/fix2-run.sh`、同一卡文、`timeout 2700`），cwd 同为 `/home/wutao/hearth-tui-new`。

### 2.2 四项并发后果（均已实证）

1. **A 的输出永久丢失**：`/proc/119049/fd/1` → `run-fix2.log (deleted)`。
   B 在 21:14:01 以 `>>` 重建了同名文件（新 inode），A 仍指旧 inode → **A 的 stdout 已无任何路径可达**。
2. **stderr 两股交错**：两实例 `2>>` 同一 `trace-fix2.err`（5,674 B, 21:28）。
   该文件内同时出现 `hearth_test` / `hearth_test2`（A 的会话名）与 `hearth_fix2`（B 的会话名）→ 两实例活动可辨。
3. **tmux 命名空间互相踩**：`trace-fix2.err` 内实录
   ```
   tmux kill-session -t hearth_test 2>/dev/null; tmux kill-session -t hearth_fix2 2>/dev/null;
   tmux new-session -d -s hearth_fix2 -x 120 -y 30 ...
   ```
   → **一方把另一方的验证会话杀掉了**。实测 21:30:07 `hearth_fix2` 刚被重建。
4. **同文件双写者（最危险）**：两实例都编辑 `src/main.rs`。
   `src/main.rs` 21:29:49 最后落笔（md5 `b451f288…`）；仅有一份 `main.rs.bak`（21:24:44, 20,500 B）。
   → **last-writer-wins，任一实例的编辑可被静默覆盖，且两方的自检结论全部失效**（无法归因）。

### 2.3 成因推断（有据）

| 线索 | 证据 |
|---|---|
| 第一实例用的是**旧卡文** | A 的 argv 中 `【修复④】` 段出现 1 次；B 的 argv 中该段**重复 2 次** |
| 卡文在两次启动之间被改 | `/home/wutao/tui-fix2-task.txt` mtime = **21:14**（晚于 A 的 21:13:13） |
| 结论 | 疑似「改了卡文 → 直接重发（`bash /tmp/fix2-run.sh`）→ **未杀旧实例**」。两次启动间隔 48 秒 |

### 2.4 证据分裂

| 文件 | 大小 | mtime | 推断归属 |
|---|---|---|---|
| `evidence-fix2.log` | 5,367 B | 21:26 | 实例之一 |
| `evidence-fix2b.log` | 5,673 B | 21:26 | 另一实例（自行加 `b` 后缀避让） |

→ 卡文只要求写 `evidence-fix2.log` 一份，实际产出两份 = **两实例各自留证的直接指纹**。

---

## §3 在飞静态核验：四项修复落地情况

**快照口径**：`src/main.rs` 546 行 / 20,228 B / md5 `b451f288ceacec53f536d004f00c9a00` / mtime 21:29:49。
**⚠️ 该文件仍被施工者实时编辑，以下是"此刻快照"，不是终态。**

| # | 卡文要求 | 源码锚点 | 判定 |
|---|---|---|---|
| **①** | 行前缀瘦身（去掉逐行 `hearth:`；答案成连续段落；用户行只留一个标记） | `:420` 注释 `// Fix 1: no per-line "hearth:" prefix; 2-space indent, default foreground`；`:421` `text.lines()` 块级渲染<br>`grep -n 'hearth: ' src/main.rs` → **0 命中** | ✅ **已落地** |
| **②** | 鼠标滚轮（`EnableMouseCapture`/`DisableMouseCapture` + `ScrollUp/ScrollDown` 复用 `scroll_offset`） | `:143` `EnterAlternateScreen, EnableMouseCapture`<br>`:229-243` `Event::Mouse` → `ScrollUp` `pinned_bottom=false; scroll_offset += 3` / `ScrollDown` `saturating_sub(3)` + 归零置 `pinned_bottom`<br>`:346-348` 退出路径 `DisableMouseCapture` | ✅ **主体落地**<br>⚠️ 见 G1/G2 |
| **③** | 配色极简（全屏≤2 色相；去 Yellow/Green/Magenta） | `:25-29` `// Fix 3: only 2 color hues` + `const C_STRUCT = DarkGray; const C_ACCENT = Cyan;`<br>`grep -c 'Magenta\|Yellow\|Green'` → **0 命中** | ✅ **已落地** |
| **④** | 等待可见性（`running... (Ns) ⚙ <当前动作>`，无动作则回退） | `:516-521` `last_tool_action(convo)` → `.map(\|a\| format!(" ⚙ {a}"))` → 拼入 `running... ({secs}s){action_str}`；`:508-510` 保留 `[fold: hist: scroll:]` | ✅ **已落地** |

**旁证（非施工者自述）**：
- `cargo build --release` 于 **21:28:07 成功** → `target/release/hearth-tui` = **1,033,832 B**（较 step1 的 908,304 B 增长）。此处**编译通过**。
- `evidence-fix2.log` 抓屏中 `input` 区块提示已变为 `(Up/Down=history PgUp/PgDn/wheel=scroll Ctrl+T=fold)` → 鼠标能力已被产品化暴露。
- 同一抓屏中答案行为 `│  hearth is a small agent harness that talks to LLMs.`（2 空格缩进、无 `hearth:` 前缀）→ 与 ① 的源码一致。

---

## §4 缺口（源码锚点，均在飞，可能随后补齐）

| 编号 | 级别 | 事实 | 锚点 |
|---|---|---|---|
| **G1** | **P1** | 鼠标/终端恢复**只走正常返回路径**，**无 panic 恢复**。卡文明确要求「panic/退出路径也要恢复，否则终端残留鼠标模式」。全文件 `grep -n 'panic\|set_hook\|catch_unwind'` → **0 命中** | `:346-348` |
| **G2** | P2 | `scroll_offset += 3` **未做上界钳制**，而同分支的 `ScrollDown` 用了 `saturating_sub`。PgUp 实现段本轮未抽到，**属未完全核实项，如实标注** | `:232` vs `:238` |
| G3 | 观察 | `Enter` 分支硬编 `--budget 40`（chat）/ `--budget 30`（resume）。与既有「hearth 单步 60–97 秒」的实测体感是否匹配，待专项 | `:277-294` |
| D12 | 遗留 | 启动预置 2 轮假对话仍在（`evidence-fix2.log` 里 `> what can hearth do?` / `> show me a hello world` 即为此）。本卡列为"不动历史"，属预期，仍是用户体感噪音源 | `:130-139` |
| **H1** | 再次实证 | `evidence-fix2b.log` 内同时出现 `✓ Task completed（1 步）` 与 `验证状态 = UNVERIFIED`、`【质量自检】未跑` | hearth 自报 |

---

## §5 本轮**故意未做**的事与原因（显式声明，非遗漏）

任务书 #2 要求 `cargo build --release` + tmux + `send-keys` + Enter + 抓屏验收。**未执行**，四条理由：

| # | 理由 | 具体依据 |
|---|---|---|
| 1 | **会触发真实 LLM 调用，违反并发铁律** | `send-keys` 发消息 → hearth 真跑；而 `.133` 上**已有 2 个 LLM agent**（4 进程）在跑 |
| 2 | **被测对象是移动靶** | `src/main.rs` 21:29:49 仍在被写；此刻的抓屏不构成验收证据 |
| 3 | **会加剧 tmux 碰撞** | 施工者已占用 `hearth_fix2` 且**正在互相 kill 会话**；我再建会话可能杀掉施工者的验证会话 |
| 4 | **会抢 target 锁、吃施工者轮次预算** | `target/.cargo-build-lock` 存在；并发 `cargo build` 会阻塞施工者的构建 |

**验收延后的触发条件**：`pgrep -f ember.py` 为空 **且** `run-fix2.log` 出现 `fix2 end exit=` 哨兵行（**两条同时成立**，单次探针不足为凭——见既有纪律 H5'）。

---

## §6 请求顶层裁决（二选一）

| 选项 | 动作 | 代价 | 收益 |
|---|---|---|---|
| **A（本轮推荐）** | 不干预，等两实例自然退出（约 21:58–21:59，`timeout 2700`），随后由评审窗做**全量独立复评** | 两实例再并发 25 分钟，`main.rs` 可能被覆盖——**反正两方自检都不可信，终态本就必须重验** | 不触发"杀错实例"风险 |
| **B** | 立即终止幽灵实例 A，只留 B | 杀掉对方在飞的一步（磁盘已完成编辑不丢） | 立即消除 50% 写冲突面 |

若选 B，命令（**评审窗未执行，需授权**）：
```
kill 119049 119050   # timeout 壳 + python 子进程；宿主 bash 119044 随之结束
```
> 不做 `kill -9`；先 `TERM` 观察 `trace-fix2.err` 是否写下终止痕迹。

**无论 A/B，终态收口一律走**：编译 + 空输入探针（零 LLM）+ 真交互一轮 + 四项修复逐条抓屏 —— **不接受任一实例的自述**。

---

## §7 附录

**本轮只读命令口径**（全部经 paramiko 在 `.133` 执行）：
`date` / `pgrep -af` / `ps -o pid,ppid,lstart,etime,cmd` / `ls -l /proc/<pid>/fd/` / `cat /proc/<pid>/cwd` / `md5sum` / `wc -l` / `grep -n` / `sed -n` / `tail` / `head` / `tmux ls` / `ls target/release/`

**关键文件口径（21:29–21:33 快照）**

| 文件 | 值 |
|---|---|
| `src/main.rs` | 546 行 / 20,228 B / md5 **`b451f288ceacec53f536d004f00c9a00`** / 21:29:49 |
| `src/main.rs.bak` | 20,500 B / 21:24:44 |
| `target/release/hearth-tui` | 1,033,832 B / 21:28:07 |
| `run-fix2.log` | 22 B = `[21:14:02] fix2 start`（**无 end 行**） |
| `trace-fix2.err` | 5,674 B / 21:28 |
| `evidence-fix2.log` / `evidence-fix2b.log` | 5,367 B / 5,673 B（同 21:26） |
| tmux 会话 | `hearth_fix2`（21:30:07 新建）、`t`（16:35:02 空闲旧会话） |

**前序口径不变**：`ev4`/`ev4b2` 已不在会话列表（旧轮遗留，本窗未动）。

**环境登记**：`/tmp/fix2-run.sh` 从 `/home/wutao/ember/config.json` 读 key（无明文 `cpk-`，符合上一轮安全卫生要求）；但 `/tmp/fix1-run.sh`、`/tmp/tui-s4b-launch.sh`、`/tmp/tui-s4c-launch.sh` 的清理仍未做。
