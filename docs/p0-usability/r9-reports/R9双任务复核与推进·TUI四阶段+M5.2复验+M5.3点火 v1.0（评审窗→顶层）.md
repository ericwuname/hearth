# R9 双任务复核与推进报告 v1.0（评审窗 → 顶层）

- 复核时间：2026-09-12 15:46–15:52 CST
- 执行机：`192.168.220.133`（评审/跑测主场）；**本机零执行**，仅 SSH 编排
- 复核人：砺·评审（评审/守门人窗口）
- 覆盖：A = TUI 四阶段验收复核；B = EMBER-M5.2 独立复验 + M5.3 点火
- 铁律遵守：日志只追加不删；每步 `ls`/`md5sum` 确认产物；**结论均来自评审窗自建探针复跑，非引擎自述**

---

## 〇、一句话结论

| 项 | 状态 | 一句话 |
|---|---|---|
| A. TUI | ❌ **未收敛（源码逐字节未变）** | 四阶段 ①✅②⚠️③❌④⚠️；输入框被 Layout 压成 0 行内高 → **打字看不见**；两轮 ember 施工均因环境缺 tmux 烧完轮次 |
| B. M5.1 | ✅ 已通过（上一轮） | edit 接线三处齐 + 三形态实测 |
| B. M5.2 | ⚠️ **主体通过，附 1 潜伏缺陷 + 1 证据不可复现** | 真接线 + 真摘要成功；但兜底未转发预算、引擎证据的 `<=2000` PASS 依赖 env 时序 |
| B. M5.3 | ✅ **已点火并通过（本轮实测）** | 15:49:14 → 15:51:09 `exit=0`；断点落盘 + `-c` 注入 + 反向不注入，**评审窗真机复现全绿** |

---

## 一、A. TUI 四阶段验收复核

### 1.1 状态判定依据（先证"没在跑"）

| 检查 | 值 | 判定 |
|---|---|---|
| `run.log` | **0 字节**，无 `TUI_DONE` | 无成功标记 |
| `pgrep -f ember.py` | 空 | **进程已消失** |
| `run2.log` / `run3.log` | 14:19 / 14:32 | 两次撞 20 轮工具循环上限 |
| `src/main.rs` | 164 行，md5 `27001a9b65bb77ab5312132359a5aa8a`，mtime **14:32** | **自上一轮复核起逐字节未变** |
| `trace3.err` 尾 | `[retry]×3 → 错误: 重试 3 次后仍失败（read operation timed out）` | 末次为 provider 超时收尾 |

→ 满足"进程已消失"触发条件，执行四阶段验收。**注意：`/tmp/tui-accept.sh` 有已知缺陷（`D=/home/wutao/hearth-tui` 指向 9/1 旧 Web 项目、阶段④`| head -0` 丢弃全部输出），故本轮仍用缺陷已修正的 `/tmp/tui-accept-v2.sh`（`D=/home/wutao/hearth-tui-new`，阶段③升级为真按键交互）。**

### 1.2 抓屏实录（tmux 120×32，`/tmp/tui-accept-v2.log` 15:47:53 段）

```
┌ hearth TUI ──────────────────────────────────────────────────────────────┐
│© 2026 Sapiens AI · hearth is a research demo TUI — not a product         │
└──────────────────────────────────────────────────────────────────────────┘
┌ conversation (demo) ─────────────────────────────────────────────────────┐
│> you: what can hearth do?                                                │
│  hearth:  hearth is a small agent harness that talks to LLMs via a JSON… │
│> you: show me a hello world                                              │
│  hearth:  fn main() { println!("hello world"); } — run with `cargo run`. │
│  — end of demo conversation (next steps will stream real turns) —        │
│                       （以下 21 行全空）                                   │
└──────────────────────────────────────────────────────────────────────────┘
┌ input ───────────────────────────────────────────────────────────────────┐   ← ★ 只有上边框
┌ model: agnes-3.0-flash | session: demo | steps: 0 ───────────────────────┐   ← ★ input 无内容行、无下沿，直接被下一区块盖住
```

**★ 即根因的肉眼证据**：`input` 区块**只有上边框一行**，既无内容行也无 `└──┘` 下沿；紧随其后的 status 区块同样只有上边框。终端总高 32 行，抓屏 32 行全被占满，说明不是"没渲染"，而是**被分配了 0 行内高**。

### 1.3 逐阶段判定

| 阶段 | 判定 | 证据 |
|---|---|---|
| ① 功能有没有 | ✅ **有** | `src/main.rs` 164 行；`Block::bordered` ×4、`Borders::ALL` ×3、`Layout::vertical` ×1、`Constraint::` ×4；四区块命名齐（top / convo / input / status）；`fn main` 在 L18 |
| ② 能不能用 | ⚠️ **能起、能画、能退，但输入失效** | `cargo build --release` → `Finished in 0.09s`；二进制 908,304 B；`SESSION_ALIVE=yes`；抓屏 5,244 B / 32 非空行；真按 `q` → `QUIT=yes(已退出)`（**事件循环通**） |
| ③ 好不好用 | ❌ **失败** | 真按键 `send-keys -l "hello hearth"` → 屏上命中 **0** 行；`Backspace`×6 后输入行仍是 demo 文案；`Esc` 后 `hello` 残留 2 行（属 demo 正文，非输入框）。**输入内容物理上不可见** |
| ④ 好不好看 | ⚠️ **部分** | ANSI 颜色转义 **69** 个；边框字符 `┌`×4 `┐`×4 `└`×2 `┘`×2 `│`×52 `─`×617。但 **`└`/`┘` 只有 2 对**——正常应 4 对 → 印证两个区块被压掉下沿，**视觉塌陷** |

### 1.4 根因（源码锚点）

`/home/wutao/hearth-tui-new/src/main.rs:64-70`：

```rust
let chunks = Layout::vertical([
    Constraint::Length(3),   // top bar      ← 3 行，边框2+内容1，正常
    Constraint::Min(8),      // conversation ← 独占剩余
    Constraint::Length(1),   // input line   ← ★ 只给 1 行
    Constraint::Length(1),   // status bar   ← ★ 只给 1 行
])
```

`input` 与 `status` 两块都用了 `Block::bordered()`（上下各 1 行边框），却只被分到 `Length(1)`：

> **可用内高 = 1 − 2(边框) = 0** → 输入文本无处渲染，`└──┘` 下沿也画不出。

这解释了全部现象：按键事件到达并被存进 `input` 状态（`q` 能退出证明事件通），但**渲染层没有行可以画**。

**修法（1 行）**：`[Constraint::Length(3), Constraint::Min(6), Constraint::Length(3), Constraint::Length(3)]`（input/status 至少 3 行：边框 2 + 内容 1）。

### 1.5 为什么两轮都失败（环境根因，已修）

- `.133` **原本完全没有终端复用器**（tmux/screen 均缺）→ ember 末次工具调用 `tmux capture-pane` 报"未找到命令" → **无法自验 → 烧完 20 轮**。
- 已在 14:38 于 `.133` 装入 `tmux 3.4-1ubuntu0.1`（+ libevent-core-2.1-7t64、libutempter0）。
- ⚠️ **登记**：环境已变，**本轮之前/之后的 ember 成功率不可直接跨时点比较**。

---

## 二、B. EMBER-M5.2 独立复验（上下文智能压缩）

### 2.1 复验方式

**不采信引擎自述**。评审窗自建探针 `/home/wutao/m52_probe.py`（本机写、SFTP 强制 LF 上传），对 `ember.py` 做静态接线核验 + 行为复跑 + 真摘要联调 + `solve()` 真路径验证。

`tools.py` 未被动（md5 `5dc4983d0622cee2984a004c6e0aa674`，与上一轮一致）✅

### 2.2 结论：**主体通过**（功能成立、接线真实、真摘要成功）

| # | 检查项 | 结果 | 证据 |
|---|---|---|---|
| 1 | `compress_history` 定义 | ✅ | L275，全文件唯一 |
| 2 | **接在 `solve()` 真路径** | ✅ | L405 `messages = compress_history(cfg, messages)`；旧 `messages = trim_history(` **已消失** |
| 3 | **接线真的生效**（非死代码） | ✅ | mock 掉 `call_model` 跑 `solve()`：history 19 条 + 追问 1 条 = 20 条 → **首轮喂给模型的仅 8 条** |
| 4 | 兜底路径（坏 `base_url`） | ✅ | 不抛异常；`[compact]` **恰好一行**；首条 user 目标保留；未注入摘要；4961 → 1805 字，≤2000 |
| 5 | **真摘要 happy-path**（真调 Agnes，1 次小请求） | ✅ | `[compact] 压缩 7 条 → 摘要 241 字（预算 3000，压缩前 5541 字）`；`[历史摘要]` 已注入；20 → 14 条；目标保留；摘要正文为**真模型产出**（对连接池日志做了归纳） |
| 6 | `python3 -c "import ember"` | ✅ | `IMPORT_OK` |
| 7 | 证据文件存在 | ✅ | `m5-2-evidence.log` 3,447 B |

**判据**：M5.2 的核心诉求（超预算→首条目标 + 最近 12 条原文 + 中间摘要；摘要失败退回丢最旧不中断；`[compact]` 可观测）**全部成立且真机可复现**。

### 2.3 发现 F1（潜伏缺陷，建议 P0-微）

**位置**：`ember.py:290` 与 `ember.py:302` —— 两处兜底调用写作 `trim_history(messages)`，**未转发预算**。

```python
308: def trim_history(messages, budget=HISTORY_CHAR_BUDGET):   # ← 默认值在 def 时求值并绑定
290:     compacted = trim_history(messages)                    # ← 不传 budget，吃上面的默认值
302:     compacted = trim_history(messages)                    # ← 同上
282: if total <= HISTORY_CHAR_BUDGET:                        # ← 却用"运行时"模块常量判阈值
292/300/304: 投影里打印 HISTORY_CHAR_BUDGET                 # ← 也用"运行时"常量
```

**机理**：Python 函数默认参数**在 def 时求值一次**。`compress_history` 用的是**运行时**的模块常量 `HISTORY_CHAR_BUDGET`，而 `trim_history` 吃的是 **import 时**绑定的默认值。两者可脱节 → 兜底实际用**旧预算**裁剪，而 `[compact]` 投影仍打印**新预算** → **投影误报**。

**现网影响**：**不阻塞**。生产形态 `python3 ember.py` 只在 import 时读一次 env，两者恒等。**触发条件**是"运行时改模块常量/按模块属性注入预算"。

**修法（1 行 ×2）**：`compacted = trim_history(messages, HISTORY_CHAR_BUDGET)`。

### 2.4 发现 F2（证据不可复现，建议 P0-方法）

引擎 `m5-2-evidence.log` 记录：

```
[PASS] 兜底后总长 <= 2000（当前 1798）
兜底后消息数: 8
```

**我用同一份 `t_m52.py`、同一份 `ember.py`（md5 未变）复跑，得到相反的 `[FAIL]`。** 决定性红绿对照（15:51，二者仅差 env 时序）：

| 运行形态 | import 时 budget | `trim_history` 默认参数 | 兜底后 | 判定 |
|---|---|---|---|---|
| `env -u EMBER_HISTORY_BUDGET python3 t_m52.py` | 64000 | `(64000,)` | 4161 字 / **18 条** | ❌ **FAIL** |
| `EMBER_HISTORY_BUDGET=2000 python3 t_m52.py` | 2000 | `(2000,)` | 1798 字 / **8 条** | ✅ PASS |

→ **引擎那次 PASS 的数字（1798 / 8）只在 env 于 `import` 之前注入时才复现**。`t_m52.py` 是在 `import ember` **之后**才设 env，再只改 `ember.HISTORY_CHAR_BUDGET`（模块属性）——改不动 `trim_history` 已绑定的默认参数。

**意义（本轮最有价值的发现）**：
> 该条 `[PASS]` **不是"断言成立"的证据，而是环境时序的产物**。这与既有的 H2（产物清单不校验文件存在）同属一类：**证据看似存在，实则不成立 / 不可复现**。
> 修掉 F1 之后，此条在两种环境下都会**真绿**——修 F1 不只是洁癖，它把一条"假绿"变成"真绿"。

**自评口径**：F2 的"引擎那次为何是 PASS"属**推断**（我无法回溯其 shell 上下文），但红绿两侧数字**与证据文件逐位吻合**，且机理可由 `trim_history.__defaults__` 直接观测——故列为**已证**的可复现性缺陷，而非对引擎的动机指控。

---

## 三、B. M5.3 点火记录

| 项 | 值 |
|---|---|
| 触发前提 | TUI 无进程（不并发）✅；M5.2 主体通过 ✅；已确认无并发 LLM 任务 ✅ |
| 任务卡 | `/home/wutao/m5-3-task.txt`（1,581 B，mtime 15:34）——"给 ember.py 加断点可见（M5 最后一件）" |
| 启动脚本 | `/tmp/m5-3-launch.sh`（沿用既有模式：`timeout 2400` 硬上限、`HEARTH_TASK_TIMEOUT_SECS=2100`、`--budget 30`、**日志追加不删**） |
| 不死通道 | `setsid nohup bash /tmp/m5-3-launch.sh >/tmp/m5-3-driver.log 2>&1 </dev/null &` |
| 启动时刻 | **15:49:14**（`[15:49:14] M5.3 启动（task_len=890）`） |
| 会话 | `8e30317c`；`HEARTH_ALLOW_NO_CGROUP=1`（真机跑批必需） |
| 前置基线 | `ember.py` md5 `c74ede0904c3824ca7ebde4fca73bc30`（504 行） |
| 结果 | **15:49:14 → 15:51:09，运行 115 秒，`exit=0`**；产出 `ember.py`（md5 变为 `610e6dd0ae1733441f059853d078af3f`）+ `m5-3-evidence.log` |

### 3.1 独立复验（评审窗实跑，非引擎自述）—— **全绿**

| # | 检查项 | 结果 | 证据 |
|---|---|---|---|
| 1 | `python3 -c "import ember"` | ✅ | `IMPORT_OK` |
| 2 | 断点记录逻辑接线 | ✅ | L158 `_write_last_action(messages)` 定义；**L192 由 `save_session()` 调用**；取最后一条 `role=="tool"` 消息、`[:300]` 截断、`makedirs(exist_ok=True)`、`except OSError → 仅告警不中断` |
| 3 | 恢复注入逻辑接线 | ✅ | L492-502 `-c` 分支：读 `.ember/last_action.txt` → 非空则追加 `[断点提示] 上次中断时的最后动作: …（请据此接着做）` + stderr `[resume] 已注入断点提示（N 字）` |
| 4 | evidence 文件三形态 | ✅ | `m5-3-evidence.log`（671 B）含 a/b/c 三段真实输出 |
| 5 | 正向实测（**我跑**） | ✅ | 文件存在时 `python3 ember.py -c …` → stderr 打出 **`[resume] 已注入断点提示（5 字）`** |
| 6 | **反向实测（我跑）** | ✅ | 把 `last_action.txt` 移走后再 `-c` → `[resume]` 行数 **0** → **证明非无条件注入** |
| 7 | 产物落地 | ✅ | `s1.txt`（5 B，内容 `hello`）、`.ember/last_action.txt`、`.ember/last_session`、`.ember/sessions/` 均在 |

**结论：M5.3 通过。** 至此 EMBER M5 三件（edit 接线 / 上下文压缩 / 断点可见）**齐件**。

### 3.2 M5.3 口径备注（非缺陷）

- `_write_last_action` 实现的是 **截断式摘要**（取最后一条工具结果原文 `[:300]`），**不是模型摘要**。卡内措辞"摘要（≤300 字符）"两种读法都满足；列为**口径确认项**，若期望"模型归纳式摘要"则需另立小卡。
- 另注意：`last_action.txt` 会被**每次** `save_session` 覆盖，故其内容随最近一次运行而变（本轮实测中由 `已写入 s1.txt：5 字符 / 5 字节。`(23 字) 变为 `hello`(5 字)）。这是设计使然，但**验收截图需与运行时刻对齐**，否则会误判不一致。

卡内自测要求（下一轮据此验收）：
a) 跑一个写文件任务 → 贴 `.ember/last_action.txt`；b) `-c` 续跑 → 贴 `[resume] 已注入断点提示（N 字）` 且回答引用上次动作；c) **反向**：删掉 last_action.txt 再 `-c` → **不应**出现 `[resume]`（证明非无条件注入）。

---

## 四、待顶层裁决批次（攒批，勿零散派活）

| # | 级别 | 事项 | 建议 |
|---|---|---|---|
| B1 | **P0-微** | M5.2 F1：`trim_history(messages)` 未转发预算（L290/L302） | 1 行 ×2 微补 + 重跑 `t_m52.py` 双环境（应双双全绿）。**不与 M5.3 串件**（M5.3 卡已明令不动压缩逻辑） |
| B2 | **P0-方法** | M5.2 F2：证据不可复现（env 时序依赖） | 并入 B1 一并销账；后续所有"预算/开关类"自测卡**统一要求 env 预置或显式传参**，禁止"改模块常量"式注入 |
| A1 | **P0** | TUI 未收敛：`main.rs:64-70` Layout 压死输入框 | 需**新任务卡**（TUI-2），卡内直接给出 `[3, Min(6), 3, 3]` 作为约束条件；**tmux 环境已修**，本轮起 ember 具备自验能力 |
| A2 | 方法 | 是否由我手改这一行？ | **建议否**。TUI 实验的测量对象是"ember 能否独立造出 TUI"；我手动补刀会**污染实验效度**，且 `main.rs` 是 ember 施工产物，非我方代码。修法应由 ember 在新卡下完成 |
| — | 挂账 | H1–H4（上一轮 harness 层发现）尚未立卡 | 其中 **H2（产物清单不校验存在）** 与**本轮 F2（证据不可复现）同源**，建议合并为一张"证据可信度"卡 |
| B3 | **待裁决** | **EMBER M5 三件已齐**（edit 接线 ✅ / 压缩 ⚠️主体通过 / 断点可见 ✅） | 是否就此**宣告 M5 收官**并转入 M6（或 vs-hearth 同题对照第二轮），由顶层定；M5.2 微补可并入收官批 |

---

## 五、异见栏 / 已知局限（写标题，不写脚注）

1. **我未能回溯 M5.2 引擎那次的 shell 上下文**，F2 的"env 预置"成因是**当次唯一自洽解释**，非直接观测。
2. **`.133` 环境已在一日中变更**（14:38 装 tmux）→ A 项前后两次 ember 运行**不可比**；本报告已就此标注。
3. 阶段②"能不能用"我判 ⚠️ 而非 ✅：**能启动/能渲染/能退出**属实，但"能不能用"的用户含义是"能对话"，在输入不可见的前提下**不具备可用性**。判分口径以用户体感为准，故与阶段③共担 ❌/⚠️。
4. M5.2 我未测 **KEEP_RECENT_MSGS=12 的边界**（恰好 12 条 / 13 条时走向不同分支）；该边界非卡内要求，列为**已知未覆盖**。
5. 本轮**未修改任何被测源码**——只做只读核验 + 上传独立探针 + 点火。

---

## 附：本轮产物清单

| 路径 | 说明 |
|---|---|
| `/home/wutao/m52_probe.py` | 评审窗自建 M5.2 复验探针（可复跑，无密钥） |
| `/tmp/tui-accept-v2.log` | TUI 四阶段验收日志（**追加**：129 行 → 本轮新增 15:47:53 段） |
| `/tmp/tui-screen.txt` / `/tmp/tui-screen-ansi.txt` | 最终屏纯文本 / 含 ANSI 留档 |
| `/home/wutao/ember-m5/ember/run-m5-3.log` | M5.3 运行日志（**新建，追加制**，271 行） |
| `/home/wutao/ember-m5/ember/m5-3-evidence.log` | M5.3 自测证据（a/b/c 三形态） |
| 本文件 | 本机 `docs/p0-usability/r9-reports/` + `.133` 双侧 |
