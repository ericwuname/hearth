# Phase D 真机验证报告 · 八实验（执行窗 → 砺判读）

> **执行**：执行窗口　**日期**：2026-09-02　**任务书**：《Hearth 可用性改造长程任务书 v1.0》§二 P2
> **环境**：.133（PD-C2 ✓，未占 .131）；hearth 0.2.23（md5 `34f6f807…`）；provider=agnes-2.5-flash（PD-C1 ✓，D6 偏差已声明）
> **原始数据**：.133 `~/phaseD/`（summary.json / summary_v2.json + 每实验 .log + session JSONL 路径）
> **纪律声明**：判读分离——本报告只交数据与初步归因，**终判归砺**；🔶→✅/❌ 定级由砺裁定。

## 一、总裁决速览

| 实验 | 候选 | 裁决（执行窗初步） | 关键数据 |
|---|---|---|---|
| D1 | L1/L1.5 写文件强制 | ❌ 未复现（本任务措辞下纯文字完成） | write 调用 0 次；输出含 give_up 字样但任务有文字交付——单样本，建议砺用 L1.5 误路由用例表复测 |
| D2 | L0.5 中段截断 | **✅ 复现** | 13260 字节 cat 回灌 = **5532 字符 = HEAD4000 + 截断标记 32 + TAIL1500 精确匹配**（loop.rs:5278-5291）；中段标记丢失 |
| D3 | L0.6/L5 | ⚠ 仪器受限，未定级 | session JSONL 不存 system prompt（每周期现场拼装）；planner dump 无特征串（dump 机制待核）——需砺指定更直接仪器 |
| D4 | L2.5 非 tty abort | **✅ 复现**（语义已进化） | `approval_denied_noninteractive` 结构化拒绝 + 补救指引逐字在案；非裸 abort（v0.2.23 有 🤖 模式横幅与"用 repl 或 --approve-within session"引导） |
| D5 | **L6 反应回路缺位** | **✅ 复现** | read→write 依赖两步：result 文件**未创建**（批量执行拿不到上步 read 输出，capture:None 机制证据） |
| D6 | F1 弱模型放大 | ✅ 方向性复现（偏差声明见下） | 同一任务：Agnes 11.2s 产出 3 文件全对（终态却 give_up——产物与终态背离，RC52 族再添一例）；qwen2.5:3b 62.8s **0 文件** + give_up |
| D7 | F2 kill→resume | **🔶 新缺陷**：one-shot chat 中途 kill = 会话从未落盘 | 存活 20s/45s 及 kill 后三个时点，session JSONL 均为 **0 个新文件** → 无从 resume；resume 机制本身对已落盘会话可用（fc3e4cff 两次 attach 成功） |
| D8 | L3.5 无 diff 事件 | ✅ 复现 | 事件流无任何 diff/patch 类事件 |

## 二、四项高价值发现（初步归因，待砺定级）

### 2.1 D5 = RC52 架构根因候选的机制实证（U-06 升格材料）

read→write 依赖任务在单 run 内失败：批量执行 capture:None，write 拿不到 read 输出。**这与 RC52/B 臂（"继续"型 3/3 失败）同根**——模型整批盲跑、无中间反馈。砺此前"架构层候选根因（L6 反应回路缺位）"条目（裁决书 T-3）由 🔶 获得机制级实证；**W1 设计小样优先级的判据（任务书 §二 P2 验收条款）已满足**。

### 2.2 D2 + D6 串成证据链：L0.5 截断存在，但强模型可自愈、弱模型被吞没

- 强模型（Agnes）被截断后**主动 grep 二次查询**补齐信息（3 次工具调用收敛）；
- 弱模型（qwen2.5:3b）同范式任务 0 产物。
- 初步归因：**L0.5 的实际伤害 ∝ 模型主动性 = F1 放大系数的另一面**。W2（回灌保真）的价值不止"看得见"，更是弱模型场景的生死线。

### 2.3 D7 = 新缺陷：one-shot chat 中途 kill 丢全会话（建议登记 U-44）

- 三个时点（存活 20s / 存活 45s / kill 后）session JSONL 均未出现；
- 初步归因：one-shot chat 路径疑未接 S3 checkpoint 回调（CLI 接线覆盖 repl，chat 路径待核——`run_local.rs` `attach_session_safety` 的构造点覆盖面）；kill（SIGKILL）阻止退出时落盘，会话整体丢失；
- 与 U-33（S3 已落地）**不矛盾但覆盖面有缺口**：repl 路径有 checkpoint，chat 路径待核；
- resume 机制正向证据：对已落盘会话（fc3e4cff）两次 attach 成功并运行。

### 2.4 附带发现（两条，交砺归档）

1. **工具间写域不对称**：`write_file` 写 /tmp 被读根域拒绝（workspace/home），而 bash 工具**可以**写 /tmp（D3/D6 的文件全是 bash 建的）——同一目标两种工具两种权限，用户可感不一致；
2. **resume 旧会话的 give_up 终止 ×2**：D7v3/v4 的 resume 运行均以 `决策 → Error（终止）… ✗ Done (N steps)` 失败——OD-1/B 臂模式在 resume 场景再添两例。

## 三、仪器修正记录（诚实披露，供砺核数据可信度）

| 轮 | 问题 | 修正 |
|---|---|---|
| v1 | D2 标记位置太浅（落在 HEAD 区） | v2 用 13260 字节文件 + 标记 line45（~8800 字符深居切割区） |
| v1/v2 | D2 "marker_in_stored=True" 假阴性 | 根因：模型后续 grep 输出含标记——改以**首个 Tool 消息长度精确匹配 5532** 定案 |
| v1 | D3 JSONL 无 prompt 可查 | v2 试 planner dump（HEARTH_DEBUG_PLANNER_INPUT=1）仍未获特征串——仪器待砺指导 |
| v1/v2/v3 | D7 杀不到中段 | 根因 = **L6 盲批使任务单周期完成**（本身即证据）；v5 改 sleep 90 设计后转向会话落盘发现 |
| v1 | D6 两 runner 撞车 | 杀孤儿实例后重跑，D6 数据来自单 runner |

## 四、待砺裁定项

1. D1/D3 是否需要第二口径复测（L1.5 误路由用例表 / 更直接的 planner dump 仪器）；
2. U-06（RC52 架构根因）是否凭 D5 升格定级；
3. U-44（chat 路径 session 落盘缺口）是否成立并排入段 2 前置修复；
4. U-07（F1 放大系数）以 D6 数据初步定级还是等 qiyuan-8b 口径补测。
