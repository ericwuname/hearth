# TUI-polish4 闭环（N6 弹点条件化）+ 任务书指定验收 v1.0（评审窗→顶层）

- **日期**：2026-09-13 06:38–06:50（GMT+8）
- **执行窗口**：砺·评审（`192.168.220.133`，评审/跑测主场）
- **本机零执行**：全部动作在 `.133`；本机只做读文件、SFTP 编排、写报告
- **本轮真 LLM 调用**：**3 次**（v23 验收 1 + polish4 施工自测 1 + v24 复验 1），**全程串行、无并发**

---

## 0. 一句话结论

任务书给的「step3 在跑（17:00）」**第 12 次过期**——实测 `step3` 早在 09-12 17:02 终态。
本轮按任务书第 2 条做**指定验收**（得到 pre-fix **RED**），并**自拟自跑一张单点微卡 polish4**，
把 TUI 车道最后一个**用户可见外观缺陷 N6** 关闭：**RED 1 → GREEN 0，同一仪器、正反成对**。

---

## 1. 基线核对（先实测，再决策）

| 项 | 实测值（2026-09-13 06:38–06:40） | 与任务书声称 |
|---|---|---|
| 进程 | `pgrep -af '[e]mber\.py\|[h]earth chat'` → **NONE** | 声称「step3 在跑」❌ |
| `src/main.rs` | md5 `d39899bc71093cef30c89940e53a8828` / 25,711 B / 634 行 / mtime **04:24:19** | — |
| 二进制 | `target/release/hearth-tui` md5 `7bb296ae43c16d065196db781dd918d9` / 1,042,216 B / 04:24:23 | — |
| 真终态 | step1–6 / fix1–6 / polish3(+3c) **全部已终态**；`main.rs` 自 04:24 逐字节未变 | — |

⇒ **无在飞件**（本轮开工前车道是空的）。任务书描述的「step3 在跑」与实际相差 **13.5 小时**。

---

## 2. A. 任务书指定验收（pre-fix，评审窗自建 `/tmp/tui-accept-v23.sh`，v1–v22 未动）

| 步 | 实测 |
|---|---|
| build | `cargo build --release` → rc=0；二进制 md5 **编译前后不变**（`7bb296ae…`）= 同源 |
| 启动 | `tmux new-session -d -s rv23 -x 100 -y 24 -c <dir> <binary>`（二进制直接当会话命令，**Ctrl+Q 判定才有效**） |
| 交互 | 06:40:15 send-keys `只回答一个数字：3+4=?` + Enter → **6 秒** `done (5s)` |
| 答案 | `7  ◂ span 耗时 2734ms` / `7✓ Done (1 steps)`，session `6fac6e9c`，`turns: 1` |
| 退出 | Ctrl+Q → `tmux has-session` **rc=1**（会话销毁）✅ |
| 既有会话 | `ht3` / `t` **未动** ✅ |

### 四阶段判定（pre-fix）

| 阶段 | 判定 | 依据 |
|---|---|---|
| ① 功能有没有 | ✅ | 骨架 + 输入 + 真接 hearth 进程 + 多轮会话 + 折叠 + 滚动全在 |
| ② 能不能用 | ✅ | 一问一答端到端闭环，6 秒，无卡死、无 running 残留 |
| ③ 好不好用 | ✅ | **默认 `fold:off` 状态下答案就在视口第 5–6 行**（不按任何键），D14 保持关闭 |
| ④ 好不好看 | **⚠️** | **`│· ═══════════ 任务总结 ═══════════`（分隔线带弹点，N6）**；`7✓` 之间缺空格；`── 收尾` 块 stdout 整块重复（N2a） |

**⭐ 本轮验收的最大副产物**：这次抓屏同时就是 **N6 的 RED 基线**（见 §3），
所以这张验收不是重复劳动——它给本轮微卡提供了**真·可失败的对照**。

---

## 3. B. 发卡 polish4（评审窗自拟自跑，单点微卡）

- **卡文**：`/home/wutao/tui-polish4-task.txt`（5,891 B）；**≤10 步**
- **发射**：06:41:44 `setsid nohup bash /tmp/tui-polish4-launch.sh >/tmp/tui-polish4-driver.log 2>&1 </dev/null &`
  - 日志**按实例分名**（H10）：`run-tui-polish4.log` / `trace-tui-polish4.err`
  - 形式即任务书要求的 `timeout 2400 python3 /home/wutao/ember/ember.py "$(cat 卡文件)" >> run-xxx.log 2>> trace-xxx.err`
- **前置备份**：**评审窗先做**（避 H9 的 0 字节陷阱）：`src/main.rs.bak-before-polish4_20260913-064144`
  25,711 B，md5 `d39899bc…` **与源逐字节一致** ✅
- **完工**：**06:45:19 `polish4 end exit=0`**（**3 分 35 秒**）；`evidence-polish4.log` **真存在**（6,889 B），末行 `POLISH4 DONE`

### 3.1 N6 是什么（RED 实测抓屏原样）

```
│· ═══════════ 任务总结 ═══════════     ← 分隔线被当成普通 Proc 行加了 `· ` 弹点
│· 【一句话】成功计算并回答了数字 7。
```

### 3.2 唯一改动（diff 全量，**只有这一处**）

```diff
@@ -513,7 +513,13 @@
                 let lines_iter = text.lines();
                 for (i, ln) in lines_iter.enumerate() {
-                    if i == 0 {
+                    if ln.starts_with('\u{2550}') {
+                        // N6: section rule lines (═══) get no "· " bullet; indent like answer lines
+                        convo_lines.push(Line::from(Span::styled(
+                            format!("  {ln}"),
+                            Style::default().fg(C_STRUCT),
+                        )));
+                    } else if i == 0 {
                         convo_lines.push(Line::from(vec![
                             Span::styled("· ", Style::default().fg(C_STRUCT)),
                             Span::styled(ln, Style::default().fg(C_STRUCT)),
```

- `634 行 → 640 行`；`md5 d39899bc… → 1aa4a2583a94f511ed1adb01221d634a`
- **越界检查**：`Cargo.toml` / `Cargo.lock` md5 与 mtime（09-12 14:16）**均未变**；`hearth-slim/` 未触碰 ✅
- **日志只追加**：全部既有 `evidence-*` / `run-*` mtime 保持原值；`evidence-polish4.log` 为**新建**文件 ✅

---

## 4. C. 独立复验（评审窗自建 `/tmp/tui-accept-v24.sh`，**与 v23 同一仪器**）

> 复验**不接受施工方自述**。同一命令、同一口径，只换被测二进制。

| 指标 | RED（pre-fix，v23） | GREEN（post-fix，v24） | 判定 |
|---|---|---|---|
| `grep -cF $'\u2502\u00b7 \u2550'`（`│· ═`） | **1** | **0** | ✅ |
| `grep -cF $'\u2502  \u2550'`（`│  ═`） | **0** | **1** | ✅ |
| 独立第二口径（Python3 UTF-8 `count`） | — | `eq_lines=1 dotted=0 plain=1` | ✅ 与上表一致 |
| `═══` 行原文（无损，回滚缓冲 -S -400） | `16:│· ═══════════ 任务总结 ═══════════` | `16:│  ═══════════ 任务总结 ═══════════` | ✅ |
| build | rc=0 | rc=0，`warn_count=0` / `error_count=0` | ✅ |
| 二进制 | `7bb296ae…` / 1,042,216 B | `c2b5f0e3c9e6397cb8e2dd987483f5a1` / 1,042,576 B / 06:42:22 | ✅ 比源码晚 14 秒构建 |
| 回归：真交互 | 6s `done (5s)`、答案 `7` | **5s `done (5s)`**、答案 `7`、`turns: 1`、`running` 残留 **0** | ✅ |
| 回归：Ctrl+Q | rc=1 | rc=1（会话销毁） | ✅ |
| 既有会话 | `ht3`/`t` 未动 | `ht3`/`t` 未动 | ✅ |

**四阶段终判（post-fix）**：**①✅ ②✅ ③✅ ④⚠️**（④ 的两条残留均在**引擎侧**，见 §5）

---

## 5. D. 本轮新发现 / 登记

| 编号 | 级别 | 内容 | 归属 |
|---|---|---|---|
| **H12（新）** | 工具链 | **断言脚本必须显式声明解释器**：同一行 `grep -F $'\u2502\u00b7 \u2550'` 在评审窗 `bash -lc` 下正确返回 1/0，在施工方 shell 下却展开成字面 ASCII → **假阴性 0**。施工方**主动怀疑并换 Python3 口径交叉验证**，方向正确。 | 卡模板 |
| **O8（新）** | P3 | 风格不统一：`═══` 行已无弹点（`│  ═══`），紧邻的 `【一句话】` 仍带弹点（`│· 【一句话】`）。是否统一待定（**本轮有意不做**，避免顺手扩大范围）。 | TUI 打磨 |
| **O4（复现）** | P2 | 状态栏 `scroll:9/17` **pre/post 完全一致** ⇒ O4 未解是对的（本轮判定**非 1 行改动能解**：`main.rs:547` 的 `start` 用作**逻辑行**切片索引，而 `max_off` 若改显示行则**单位与索引耦合错位**，需显示行→逻辑行映射，属独立小工程）。 | TUI 打磨 |
| **N2a（第 6 次）** | P1 | `── 收尾` 三连整块在 stdout **重复两次**（pre/post 抓屏均如此，确定性）。`run-001.md` 内无 `收尾` 行 ⇒ 重复在**引擎 stdout 通道**。 | **引擎/需授权** |
| `7✓` 缺空格 | P2 | 答案 `7` 与 `✓ Done` 之间无空格 ⇒ 引擎输出。 | **引擎/需授权** |

### ⭐ 值得表扬的正面样本

- **施工方首次主动质疑自己的仪器**：自测 c 项 `grep` 得 0（与预期一致但**理由是错的**），它没有拿这个"0"当通过，
  而是查出 `$'...'` 未展开的原因、改用 Python3 UTF-8 计数，并在证据里**如实标注**（`evidence-polish4.log` 中 Note 段）。
  → 对照 fix5 的「不可失败断言」（F2/H2 家族），这是**口径自省的第一次**。
- **本轮自测正反成对且可失败**：卡文强制"必须先取 RED"，且 RED 由**评审窗独立提供**（§2），
  施工方无法用"恒真断言"糊过去。**卡模板级改进，建议固化**。

---

## 6. E. 纪律与收尾

| 项 | 状态 |
|---|---|
| 本机零执行 | ✅ 全部实测在 `.133` |
| 日志只追加 | ✅ 既有 `evidence-*`/`run-*` mtime 全部保持原值 |
| 未 kill 他人会话 | ✅ `ht3`(PID 123147) / `t` 保留未动 |
| 残留进程 | ✅ 无 ember / hearth chat 残留 |
| 并发 | ✅ 开工前探空；全程串行；本卡与任何他人 LLM 任务无重叠 |
| 备份 | ✅ 评审窗先做，非零且与源 md5 一致 |

---

## 7. F. 待顶层裁决（评审窗**未启动**，等批复）

| 优先级 | 事项 | 为何需要顶层 |
|---|---|---|
| **⭐ P0** | **引擎卡「stdout 收尾去重 + TL;DR 后置」** | 一处改动同解 **N2a（答案重复）+ O7（报告 TL;DR 置顶 vs stdout 置底）+ `7✓` 缺空格**；**动 `hearth-slim` 源码越界，需授权** |
| **⭐ P0** | **D17 版本面收敛**（用户手工入口 = 0.2.25 ≠ 被测件 0.2.27） | **用户体感数据与被测件不同源**，会污染一切真机结论 |
| P1 | 引擎卡「progress 口径（仅 write 类）+ 阈值 3」 | 解"简单任务几十轮"真因，动 `hearth-slim` 需授权 |
| P2 | O4（`max_off` 单位对齐）／ O8（弹点风格统一） | 独立小工程，但**现在无阻塞缺陷**，优先级低于上面三条 |
| P3 | `ht3` 陈旧实例（PID 123147，自 09-12 22:09）是否清理 | 陈旧渲染易被误读为现状 |
| — | 存量：H1/H2/H3/H4/H7/H8/H9/H10/H11/**H12** + M5.2 的 F1/F2 + D13 + 密钥轮换 | 卡模板级，另立 |

> **评审窗的建议**：TUI 车道（A）到此**用户可见缺陷清零**（除引擎侧转嫁项）。
> 再往下的 O4/O8 属"锦上添花"，**不建议**在没有顶层指令的情况下继续自转——应把预算转向 §7 的 P0 引擎卡。

---

## 8. 本轮产物

| 类型 | 路径 | 说明 |
|---|---|---|
| 报告（本件） | `docs/p0-usability/r9-reports/TUI-polish4闭环（N6弹点条件化）+ 任务书指定验收 v1.0（评审窗→顶层）.md` | 本机 + `.133:/home/wutao/r9-reports/` 双侧 |
| 卡文 | `/home/wutao/tui-polish4-task.txt` | 5,891 B，已发射并完工 |
| 证据（14 件） | `docs/p0-usability/r9-reports/data/r13-*` | 见下 |

证据清单：`tui-accept-v23.sh` / `r23-accept.log` / `r23-screen.txt` / `r23-scroll.txt` /
`tui-accept-v24.sh` / `r24-accept.log` / `r24-screen.txt` / `r24-scroll.txt` / `r24-build.txt` /
`evidence-polish4.log` / `run-tui-polish4.log` / `trace-tui-polish4.err` /
`main.rs.polish4`（终态全文）/ `tui-polish4-task.txt`

---

## 9. 口径分树（**禁混标**）

| 代号 | 含义 |
|---|---|
| `d39899bc…` | polish4 **前** `main.rs`（= polish3c 终态，634 行） |
| `1aa4a258…` | polish4 **后** `main.rs`（640 行） |
| `7bb296ae…` | polish4 前二进制（1,042,216 B） |
| `c2b5f0e3…` | polish4 后二进制（1,042,576 B） |
| `/tmp/tui-accept-v23.sh` | 评审窗 **pre-fix RED** 验收脚本（v1–v22 未动） |
| `/tmp/tui-accept-v24.sh` | 评审窗 **post-fix GREEN** 复验脚本（同一仪器） |
