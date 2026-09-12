# 【草稿 · 未发射 · 待顶层裁】TUI step6 打磨小卡（≤12 步）

> 起草：评审窗 砺·评审 ｜ 2026-09-12 20:27 ｜ **不发射，等顶层批复后才可启动**
> 依据：`TUI-step5多轮会话验收·证据方法缺陷定案 v1.0（评审窗→顶层）.md` 的 D9 / D10 / D11

---

## 任务：hearth TUI 第 6 步：三处打磨（只做这三件，**≤12 步**）

【工作目录】`/home/wutao/hearth-tui-new/`

【环境硬约束 · 不遵守必白烧轮次】
- 每条 cargo 命令前必须：`export PATH=$HOME/.cargo/bin:$PATH`
- 启动 TUI 用：`tmux new-session -d -s v6 -x 100 -y 24 -c /home/wutao/hearth-tui-new '/home/wutao/hearth-tui-new/target/release/hearth-tui'`
  （**二进制直接当会话命令**；用 shell 中转会导致 Ctrl+Q 判定无效）
- 判"本轮结束"**只能**看状态栏出现 `done (`，**不许**用 `steps:` 当结束信号（它按 Enter 就自增）
- **先写证据再收尾**：证据文件必须是你改动后的**第 1 个动作**写入，不许留到最后

【本步三件（三件必须都做）】

**① D9「不知道就说不知道」**（当前行为：全新会话被问"我叫什么名字？"会抓 `uid`/`hostname` 猜一个名字）
- 在系统提示/任务指令里加一条硬约束：**涉及"用户的个人身份信息"（姓名、年龄、住址、单位等），若上下文中没有明确来源，必须回答"我不知道"，禁止从环境元数据（uid/hostname/os 用户名/路径）推断或声明**。
- 定位点：TUI 侧无此逻辑，属 **hearth harness** 的提示注入——**只改 `/home/wutao/hearth-tui-new/` 内能改的东西**；若判定必须改 hearth 源码，则**停下来在证据里写明"越界，需顶层授权"**，不要自行改 `/home/wutao/hearth-slim/`。
  （评审窗注：本条建议以"最小可落点"为准——若 TUI 侧无法承载，则本件降级为"登记 + 上报"，其余两件照做。）

**② D10 `steps:` 语义**（`main.rs:470` 现为 `history.len()`，显示"已发消息数"却标成 `steps`）
- 二选一并说明理由：(a) 改为**真值**——从 session json 读 `steps_used`；或 (b) **改名**为 `msgs:`/`turns:` 并保持真实。
- 注意：`hist:` 已在同栏显示消息数，若选 (b) 请避免同栏两个同义字段。

**③ D11 状态栏滞后**（回答已渲染完，状态栏仍 `running... (Ns)` 直到子进程退出）
- 目标：**子进程最后一行输出到达后 ≤1s 内**，状态栏不再是 `running`。
- 允许的最小改法示例：在 stdout/stderr 读取线程 EOF（`for line in reader.lines()` 循环退出）时补发一个"收尾中"信号，UI 收到即结束 `running`（`last_secs` 用当前 elapsed 近似），不必等 `child.wait()`。

【自测（存 `/home/wutao/hearth-tui-new/evidence-step6.log`）——**证据必须先落盘**】
- a) `cargo build --release` 通过（贴末尾 2 行）
- b) **D9 反例测试**：全新会话（首条消息）问 `我叫什么名字？` → 抓屏**必须**出现"不知道/无法得知"类表述，且**不得**出现 `wutao`/`uid`/`hostname`
- c) **D11 正例测试**：问 `只回答一个数字：3+4=?`，**同一秒**抓屏 → 文件里能看到 `hearth: 7` 与状态栏**已不是** `running`
- d) **D10**：抓屏状态栏一行贴进证据，说明改了哪个字段、依据的源值是多少

【约束】
- 只改 `/home/wutao/hearth-tui-new/` 内文件；**不许动** step1–5 已完成的布局/回显/历史/滚动/折叠/多轮会话
- bash 只用简单命令（cargo/tmux/ls/cat/grep/python3）
- **日志一律追加（`>>`），禁止 `cat >` 覆盖已有证据文件**
- key 从 `/home/wutao/ember/config.json` 动态读，**禁止在脚本里内联明文 key**

【交付】
- `evidence-step6.log`（含 a/b/c/d 四段实测）
- 一段 ≤5 行的改动摘要（改了哪几行、为什么）
