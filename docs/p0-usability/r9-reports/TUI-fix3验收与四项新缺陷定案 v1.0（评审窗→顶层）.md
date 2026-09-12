# TUI-fix3 独立验收 + 四项新缺陷定案 v1.0（评审窗 → 顶层）

- 评审窗（砺·评审） | 2026-09-12 22:33–22:45 | 目标机 `.133`
- 被测对象：`/home/wutao/hearth-tui-new`（施工者 EMBER）
- 结论一句话：**fix3（输入丢字符 P0）独立复现通过 ✅**；但同一轮里抓到 **4 处新缺陷**（1×P0 / 2×P1 / 1×P2，含 1 处引擎侧），已就地立卡 **TUI 修复-4** 并启动。

---

## 0. 先纠正基线（第 6 次过期）

| 任务书声称（17:00 基线） | 22:33 实测 |
|---|---|
| step3「正在跑」 | step3 早已结束；此后 step4/4b/4c/step5/fix1/fix2/**fix3** 全部已终态 |
| `pgrep -f ember.py` 有进程 | **无**（`rc=1`，零 ember 进程） |
| — | 真实最新事件：**fix3 22:06:13 → 22:15:52 exit=0**；`main.rs` 自 **22:07:35** 起逐字节未变（md5 `fc1b8dac…`） |

→ 本轮按纪律「**先实测再动作**」，不按任务书声称的状态行动；实际做的是 **fix3 独立验收 + 新缺陷定案 + 发下一张小卡**（任务书 step3 分支的等价推进）。

---

## 1. fix3 验收：输入丢字符 P0 — ✅ 通过（评审窗独立复现）

### 1.1 源码核验（锚点）
- `src/main.rs:231` 全文件 **只有 1 处** `event::read()?`（`grep -c` 实证）：`match event::read()? { Event::Mouse(m) => …, Event::Key(key) => …, _ => {} }` —— 三 arm 合并成立。
- 原缺陷（一个 poll 周期两次 `read()` → 后一事件被吞 ≈50% 丢键）在结构上已不可能复现。
- 二进制 `target/release/hearth-tui` 1,040,688 B / 22:07:59 / md5 `b60a9cae…`，比源码晚 24s → 构建顺序正确。

### 1.2 行为复现（自建 `tui-accept-v12.sh`，v1–v11 未动）
零 LLM 探针（无 Enter，不触发模型；可在他人 LLM 任务旁安全跑）：

| 探针 | 输入 | 输入框实测 | 判定 |
|---|---|---|---|
| A 单次突发 | `AbCdEf0123456789`（16 字符） | `> AbCdEf0123456789▌` | ✅ 16/16 全在，**零丢失** |
| A2 逐键 30ms | 同上，一键一发 | `> AbCdEf0123456789▌` | ✅ 16/16 全在 |
| B Esc 清空 | `Escape` | 占位符复位 | ✅ |
| C 中文+符号 | `只回答一个数字：3+4=?`（13 字符，fix3 原题面） | `> 只回答一个数字：3+4=?▌` | ✅ 13/13 全在 |
| D ↑ 历史 | `Up` | `> 只回答一个数字：3+4=?▌` | ✅ 完整召回 |
| E ↓ 清召回 | `Down` | 占位符复位 | ✅ |

真 LLM 一轮（仅 1 次调用）：`只回答一个数字：3+4=?` → **2s 内 `done`**，产物级铁证 `.hearth/reports/45b86a9d-…/run-001.md`：`【一句话】计算 3+4 的结果并返回数字 7`、`【输出结果】7`、`耗时 2s | 步数 1` → ✅ **答 7**。

### 1.3 四阶段判定
| 阶段 | 判定 | 依据 |
|---|---|---|
| ① 功能有没有 | ✅ | 真交互通（答 7）；Esc/↑/↓/PgUp/PgDn/Ctrl+T/Ctrl+Q 全部生效；Ctrl+Q 会话即销毁（会话数 1→0） |
| ② 能不能用 | ✅ | 输入 100% 回显（P0 已闭）；不卡死；单轮 2–8s |
| ③ 好不好用 | ⚠️ | 见 §2：**红字误报**、**折叠名不符实**、**长行读不全**、**启动假对话** 四处拖后腿 |
| ④ 好不好看 | ✅（基本） | 四边框对齐（4 对 `└┘`）、状态栏接真值（`session: 45b86a9d`、`steps: 1`、`[fold:off hist:1 scroll:0/16]`）——D5 已闭 |

---

## 2. 本轮新缺陷定案（4 项，全部源码锚点级）

### N3 — 「需要人工确认」红字每次都误报 · P0 · TUI 侧
- **现象**：每个会话都弹 `⚠ 该请求需要人工确认，当前 TUI 不支持交互回复 —— 请在终端直接运行 hearth chat 处理`，但任务其实正常完成。
- **根因（铁证）**：`main.rs:182-189` —— `let is_confirm = l.contains("需要你确认") || l.contains("❓") || l.contains("approve");`。而每次必打印的委托行是 `🔓 会话级审批委托已开启（--approve-within session）——…`，其中 **`--approve-within` 含子串 `approve`** → 条件必中。
- **捕获证据**：抓屏中红字**紧跟**在委托行之后（`…--approve-within session…` → 下一行即 `⚠ …`），顺序与源码逻辑一致。
- **修法**：删掉 `"approve"` 这一支（1 行）。

### N1 — Ctrl+T 折叠名不符实 · P1 · TUI 侧
- **现象**：Ctrl+T 前后状态栏 `scroll:0/16` → `0/15`，**只少 1 行**；`── 收尾 |` `📄 执行报告` `✓ Task completed` 一行都不折叠。
- **根因（铁证）**：`main.rs:403` 只跳过 `Kind::Proc`；而 `Kind::Proc` **仅赋给 stderr 行**（`main.rs:194`），stdout 行一律成 `Kind::Answer`（`:184`）。hearth 的全部叙事都走 **stdout** → 永不被折叠。该轮会话 stderr **只有 1 行**（`diagnostics → …`），所以折叠效果恰为「少 1 行」——数字与源码完全对上。
- **修法**：推入前按内容重分类（`──`/`📄 `/`✓ `/`─────`/`· `/`▸ span` → Proc）。

### N2 — 答案被硬截断 + 引擎重复输出 · P1 · 分两半
- **N2b（TUI 侧）**：`main.rs:466` `Paragraph::new(visible_iter.to_vec())` **未启用 `wrap`**（全文件 `grep Wrap` 0 命中）→ 超宽行在边框处齐边砍断，无换行无省略号，用户读不全。修法：追加 `.wrap(Wrap { trim: false })`。
- **N2a（引擎侧，非 TUI）**：绕开 TUI，直接捕获 `hearth chat "请用大约三句话介绍你自己。" --budget 40 --approve-within session` 的原始 stdout：**41 行 stdout**，同一段答案出现在 **第 10 行与第 15 行（逐字相同）**，且 `── 收尾` 三连 + `📄 执行报告` + `✓ Task completed` 也**各打印两遍**（首份以 `◂ span 耗时 1464ms` 收尾，次份以 `✓ Done (1 steps)` 收尾）。
  → **TUI 只是忠实转发**，重复的账应记在 **hearth（hearth-slim）** 上。**建议单独向顶层报引擎卡**，不要把它塞进 TUI 卡。

### ~~N2c（观察项）— 长答案的第 2/3 段未在任一窗口出现~~ → **本轮已撤回（我自己的取样误差）**
当时看到答案只有第 1 段的**两份拷贝**，怀疑 TUI 丢行。后续（v14）同题复跑：答案经 `wrap` 后逐行完整、续行无字符丢失（`…运行测试验证、定位` ／ `bug，并能查阅资料核对事实。…`），且**那一次模型本来就只答了一段**。
→ **撤回**：TUI 未丢行，是「滚动窗口取样 + 模型输出长度差异」造成的误判。**不立案。**（与 v12 提取器错抓行同族——本轮第二次被我自己的观测方法坑。）

### N4（= 旧 D12）— 启动凭空预置 2 轮假对话 · P2 · TUI 侧
- `main.rs:158-167` 硬编码 `what can hearth do?` / `show me a hello world` 两轮问答；启动抓屏实证（conversation 区直接显示这两轮 + `(type a message below and press Enter)`）。新用户会误以为是自己问过。

---

## 3. 遗留未闭（承接前几轮，未变）
| 编号 | 内容 | 级别 | 状态 |
|---|---|---|---|
| G1 | 鼠标捕获仅在**正常返回路径**恢复（`main.rs:350`）；全文件 `panic`/`set_hook`/`catch_unwind` **0 命中** → panic 时终端会留在鼠标捕获态。注释自称 "always disabled on exit"，**属过度声称** | P1 | 未修 |
| G2 | `main.rs:236` 鼠标滚轮 `scroll_offset += 3`、`:325` PageUp `+= 5` **上界未钳制** | P2 | 未修 |
| H1 | `验证状态 = UNVERIFIED` 与 `✓ Task completed` 并存（本轮再次实证，第 4 次） | 方法 | 待顶层裁 |
| H2 | 产物清单不校验文件是否存在（本轮 fix3 自报 `evidence-fix3.log` 实际存在，属诚实自报；但门禁仍不校验） | P0-方法 | 待顶层裁 |

---

## 4. 已发卡：TUI 修复-4（小而准，只改 `src/main.rs`）
- 卡文件：`/home/wutao/tui-fix4-task.txt`（4,143 B）；启动器 `/tmp/tui-fix4-launch.sh`。
- 内容：N3（1 行）/ N1（分类）/ N2b（wrap）/ N4（清预置）。**N2a 未入卡**（属引擎侧）。
- **口径更正（我自己的笔误）**：本报告初稿此处曾写「+ G1（panic 恢复，卡文明确要求）」——**不实**：卡文四条并未包含 G1。G1 仍是**未修的遗留项**（`grep panic/set_hook/catch_unwind` = 0）。已改正。
- 启动：**22:37:58** `setsid nohup bash /tmp/tui-fix4-launch.sh`（ppid 脱离），日志 **`run-tui-fix4.log` / `trace-tui-fix4.err`（按实例区分文件名，遵 H10）**。
- 前置：`pgrep ember.py` = 0（无并发）；`src/main.rs` 时间戳备份 `src/main.rs.bak-before-fix4_20260912-223754`（20,730 B 非零，md5 = `fc1b8dac…` 与源一致）。

---

## 5. 归档异常（不掩盖）

1. **`run-fix3.log` 出现 2 条 end 行（1 start / 2 end）**：`[22:15:24] fix3 end exit=0` 与 `[22:15:52] fix3 end exit=0`，相隔 28s；而 `/tmp/fix3-run.sh` 内**只有 1 条 start + 1 条 end echo** → 说明**启动器被执行了两次，且其中一次的 start 行丢失**（H10 家族的「stdout 丢失」签名）。
   **但未造成双写**：`main.rs` 自 **22:07:35** 起 md5 未变（早于两条 end 行 8 分钟），二进制 22:07:59 —— 无可疑的后写覆盖。**判为「疑似同卡二启，无实害」，登记待顶层关注，不建议追查。**
2. **施工者自报微瑕**：fix3 自述「`只回答一个数字：3+4=?` **14 字符**逐字在」，实际为 **13 字符**（评审窗逐字计数）。不影响结论，仅记录其自报精度。
3. **`ht3` tmux 会话残留**（22:09:58 建，其 `hearth-tui` 空闲至今，屏上停在 `done (3s)`）。**未 kill、未打扰**（属他人会话）。另有 `t` 会话（16:35 建）长期空闲。

---

## 6. 方法论自曝（本轮我自己的错）

**v12 第一版提取器抓错了行**：我用「首个以 `│>` 开头的行」当输入框，结果 A/A2/B/C 四个探针全部误判为 `EXACT=NO`（抓到的是**预置假对话**那行 `> what can hearth do?`），一度看似「fix3 没修好」。
**纠正方式**：改按 `┌ input` 上边框定位、取其下一行为输入内容（`v12_chk2.py`），**重跑同一批已存抓屏**（零新增 LLM 调用）→ 结论翻转为全绿。
→ 教训并入卡模板：**抓屏断言必须锚定"控件边框"而非"某个前缀"，否则会被同形文本污染。**（与 A6「steps: N 误当完成信号」同族。）

---

## 7. 证据清单
`docs/p0-usability/r9-reports/data/`（本机）与 `/home/wutao/r9-reports/`（.133）双侧：
- `tui-accept-v12.log` / `tui-accept-v13.log`（本轮两个自建验收脚本全文）
- `fix3-v12-A/A2/B/C/D/E`、`fix3-v12-turn1.txt`、`fix3-v13-up/fold/bottom.txt`（各阶段抓屏）
- `fix3-n2-stdout.txt`（**引擎重复输出的原始 stdout**，41 行）/ `fix3-n2-stderr.txt`（1 行）
- `run-fix3.log`（含双 end 行异常）/ `evidence-fix3.log`
- 脚本：`.workbuddy/tmp/tui-accept-v12.sh`、`tui-accept-v13.sh`、`v12_chk2.py`、`tui-fix4-task.txt`、`tui-fix4-launch.sh`

## 8. 口径
- `main.rs`：md5 `fc1b8dac…`（fix3 终态，548 行 / 20,730 B / 22:07:35）；备份 `src/main.rs.bak-before-fix4_20260912-223754` 同 md5。
- 二进制 md5 `b60a9cae…`（1,040,688 B / 22:07:59）。
- `ht3`=残留会话名（他人）；`h3`/`ht3`/`hearth3`=fix3 期间用过的会话名（见 trace）。
- 本轮 **LLM 调用总计 3 次**（v12 一轮 / v13 一轮 / N2 引擎直跑一轮），无并发。

---

## 9. 补记：TUI 修复-4 已完工并通过独立复验（22:45–22:50）

**施工**：22:37:58 启动 → **22:45:03 `fix4 end exit=0`**（约 7 分钟）。`main.rs` **548 行 / 20,879 B / md5 `59024235…`**，二进制 1,038,008 B / 22:45:39。

**源码核验（逐条对上，diff 无任何附带改动）**
| 项 | 源码实证 |
|---|---|
| N3 | `:180-181` `is_confirm = l.contains("需要你确认") \|\| l.contains("❓");` —— `"approve"` 支**已删** |
| N1 | `:174-179` 新增 `is_proc`（`trim_start` 后 `──`/`📄 `/`✓ `/`─────`/`· `/或含 `▸ span`）；`:182` `let kind = if is_proc { Kind::Proc } else { Kind::Answer }` |
| N2b | `:18` use 补 `Wrap`；`:466` `.wrap(Wrap { trim: false })` |
| N4 | `:158` `let mut convo: Vec<(Kind, String)> = Vec::new();` |
| G1 | **未动**（`grep panic/set_hook/catch_unwind` = **0**）→ 遗留 |

**行为复验（自建 `tui-accept-v14.sh`，1 次真 LLM，22:45:37→22:46:07）**
| 项 | 结果 | 铁证 |
|---|---|---|
| N4 | ✅ | 启动 conversation 区 `nonempty_rows=1`（仅提示语），`preset_fake_convo=False`（不再出现 what can hearth do? / hello world） |
| N3 | ✅ | fold:on 抓屏**含触发行** `🔓 …（--approve-within session）…` 而**无** `⚠ 需要人工确认` 行 → 触发条件在、误报消失 |
| N1 | ✅ | `[fold:off scroll:0/10]`（总内容 29 行）→ `[fold:on scroll:0/0]`（≤19 行）：`── 收尾`×3 + `📄` + `✓` + `── 收尾`×3 共 **8 行整块消失**（≥3 达标）；**折叠后答案首次进入可视区**（折叠的价值闭环成立） |
| N2b | ✅ | 同题答案 `hardcut_rows=0`（修前为 2 行齐边截断）；续行可读：`…阅读与修改代码、运行测试验证、定位` ／ `bug，并能查阅资料核对事实。…`；委托长行亦正确折成 2 行 |
| 回归 | ✅ | Ctrl+Q 会话销毁（`YES→NO`）；提问/答案/总结完整 |

**新观察（不立案，仅备忘）**
- **N2a 再次复现**：v14 抓屏中答案同为两份（`◂ span 耗时 1482ms` 版 + `✓ Done (1 steps)` 版），再次确认**重复输出在引擎侧**。
- **O2 外观小瑕**：折叠用的 `Kind::Proc` 行统一加 `· ` 前缀，与引擎自带的 2 空格缩进叠加成 `·   ── 收尾 | …`（三个空格），略显毛糙。**建议并入后续「打磨卡」，不单独立卡。**
- **自曝第二次**：我在 §2 N2c 里一度怀疑 TUI 丢行，实为我的 `cut -c1-125` 截断 + 该轮答案本就单段所致 → 已在 §2 撤回。

**四阶段终判（fix4 后）**：①✅ ②✅ ③✅（除 O2 外观小瑕与引擎侧 N2a） ④✅ —— **TUI 车道 D5/D7/D8/N1/N2b/N3/N4 全部关闭**；遗留仅 **G1（panic 恢复）/ G2（滚轮上界未钳制）/ O2**。

**证据**：`data/tui-accept-v14.log`、`data/fix4-v14-init.txt`、`fix4-v14-turn1.txt`、`fix4-v14-fold.txt`、`fix4-v14-up.txt`；脚本 `tui-accept-v14.sh`、`v14_chk.py`。

