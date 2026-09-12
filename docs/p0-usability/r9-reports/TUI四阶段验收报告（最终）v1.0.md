# hearth TUI 四阶段验收报告（最终）v1.0

- **日期**：2026-09-12 18:15　**施工**：**EMBER**（step1→step4 全部由 EMBER 完成，执行窗只出卡与验收）
- **产物**：`/home/wutao/hearth-tui-new/`（Rust + ratatui；main.rs ~17KB；release 二进制）
- **结论**：**用户定的四阶段判定，全部通过** ✅✅✅✅

---

## 一、四阶段判定（逐项实测证据）

| 阶段 | 判定 | 实测证据 |
|---|---|---|
| **① 功能有没有** | ✅ **全有** | 顶栏（标题+版权行）、对话区（you/hearth 双色）、工具/过程流（`·` 前缀）、输入框（提示+光标）、状态栏（`model/session/steps` + `[fold hist scroll]`）——**五区块齐备** |
| **② 能不能用** | ✅ **真能用** | `cargo build --release` ✓；tmux 启动即出界面；**Enter 真触发 hearth 子进程**（`hearth -p`，session 0b3f2646，答案 **7 / 4 / 10** 实测三题全对）；45 秒后界面正常**不卡**、输入框复位可继续 |
| **③ 好不好用** | ✅ **好用** | **↑ 历史**（回填上一条输入 ✓）；**PgUp/Dn 滚动**（`scroll:0/44 → 5/44` ✓）；**Ctrl+T 折叠过程行**（`dot_lines 18 → 0`，答案保留，状态栏 `fold:off→on` ✓）；输入框内**自带快捷键提示**（`Up/Down=history PgUp/PgDn=scroll Ctrl+T=fold`） |
| **④ 好不好看** | ✅ **好看** | 完整边框（圆角/直角色块分区）、列对齐、块状布局、过程行 `·` 前缀 + 暗色与答案区分、输入行光标 `▌`、状态栏信息密度适中 |

## 二、最终界面（tmux 实抓，30 行存档）

```
┌ hearth TUI ────────────────────────────────────────────────────────────────┐
│© 2026 Sapiens AI - hearth is a research demo TUI                           │
└─────────────────────────────────────────────────────────────────────────────┘
┌ conversation ───────────────────────────────────────────────────────────────┐
│> you: what can hearth do?                                                   │
│  hearth: hearth is a small agent harness that talks to LLMs.                │
│> you: show me a hello world                                                 │
│  hearth: fn main() { println!("hello world"); }                             │
│> you: 只回答一个数字：5+5=?                                                 │
│  hearth: 10                                                                 │
└─────────────────────────────────────────────────────────────────────────────┘
┌ input  (Up/Down=history PgUp/PgDn=scroll Ctrl+T=fold) ──────────────────────┐
│> type a message... (Ctrl+Q=quit, Esc=clear, Enter=send) ▌                  │
└─────────────────────────────────────────────────────────────────────────────┘
┌ model: agnes-3.0-flash | session: demo | steps: 0 ──────────────────────────┐
│[fold:off hist:1 scroll:0/16] done (5s)                                      │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 三、施工溯源（诚实）

| 步骤 | 内容 | 结果 |
|---|---|---|
| step1 | 最小骨架（四区块 + 示例对话） | ✅ 908KB 二进制；底部布局有 bug |
| step2 | 修底部布局 + 输入回显 | ✅ 底部完整、回显/Esc/Enter ✓ |
| step3 | 接 hearth 进程（后台线程 + channel + 非阻塞 + `·` 过程行） | ✅ 真交互打通（答案上屏） |
| step4 | 历史 / 滚动 / 折叠 / 状态栏 | ✅ 三项全过（Ctrl+T 折叠 18→0） |

**注**：每步都撞过 EMBER 的 **20 轮工具循环上限**（step3/step4 的代码写完后无余量跑自测）——执行窗接手做编译与实测验收，**功能本身均为 EMBER 实现**。

## 四、遗留与建议

1. **hearth 调用用的是 `-p`（headless 单次）**：问答/单任务够用；**多轮会话**（保持同一 hearth session）未接 → 建议 step5 接 `hearth chat --session` 或 resume；
2. **spinner**：本次以"`running... (Ns)` + 状态栏"替代动画（够用）；纯动画可后补；
3. EMBER 的 **20 轮上限**已成为复杂任务的主要瓶颈——建议把"每卡 ≤15 步"作为后续任务卡的硬约束（已在 M5 拆卡中验证有效）。
