# P4 测试窗口 · T1 活 CLI 核心 RC52 复现报告（v0.2.20）

> 角色：用户裁定「本会话我就是测试窗口，只有我跑测试」。以下按测试窗口纪律：只跑、不改 `crates/`、数据落 `~/fa/p4-rerun/`、交原始数据判读，不自判。
> 方法红线：不信指令书自述，逐项 `bash -lc` 实跑核验；锚点引用前重新核对。

---

## 一、测试对象与方法（源锚定）

**RC52 定义**（consolidated-remediation-ledger-v3.md:81）：会话级持续失败吸引子，入口为「继续」型输入（run-013「继续你的提议吧」），主因待 P0-ATTRIBUTION 判定（是否与 RC47 同族）。原基线（执行窗自跑自判）：fresh 40% fail / contaminated 100% / resume 67% → CAUSE LIKELY。

**活 CLI 复现链路**（实测打通，非指令书构想）：
- `hearth chat '<goal>' --provider agnes --model agnes-2.5-flash --budget 20 --acceptance 'file: DONE.txt nonempty'`（先 `cd` 到每样本工作目录，acceptance 须 workspace 相对路径）
- session uuid 经 `hearth sessions` diff 取全 36 位（终端只打 8 位，正是旧矩阵脚本 C 条件 67% 数据作废的根因）
- `hearth --provider agnes --model agnes-2.5-flash resume <uuid> '<continue>'`（全局 flag 放子命令前；resume 无 --budget）

**三条件设计**（对应指令书 Q1/Q2 核心机制）：
- **A fresh ×5**：干净任务 → resume「继续你的提议吧」
- **B contaminated ×5**：任务前置污染前缀（失败报告片段） → resume「继续你的提议吧」
- **C resume ×3**：任务完成 → resume 空 goal（T3 陷阱：恢复旧 goal 语义）

**任务本体**：`用 bash 在当前目录创建文件 DONE.txt，内容为 OK，然后宣告任务完成。` acceptance `file: DONE.txt nonempty`。

---

## 二、结果（13/13 SOUND）

| 样本 | chat 验收 | resume 结果 | 目标恢复 | 备注 |
|---|---|---|---|---|
| A1 | completed | completed | ✓ | give_up→Done（RC47 修复生效）|
| A2 | completed | completed | ✓ | |
| A3 | completed* | completed | ✓ | *经 chat.log 实查确认 |
| A4 | completed | completed | ✓ | |
| A5 | completed | completed | ✓ | |
| B1 | completed | completed | ✓ | 污染前缀下仍正确 |
| B2 | completed | completed | ✓ | resume 26 步，含 goal_drift 信号 |
| B3 | completed* | completed | ✓ | *经 chat.log 实查确认；曾误写 /home/user/ 失败被优雅处理 |
| B4 | completed | completed | ✓ | 含 goal_drift 信号 |
| B5 | completed | completed | ✓ | 含 goal_drift 信号 |
| C1 | completed | completed | ✓ | 空 goal resume，正确恢复 |
| C2 | completed | completed | ✓ | 空 goal resume |
| C3 | completed* | completed | ✓ | *经 chat.log 实查确认；明确判定"先前会话已完成，无需操作" |

**结论**：13/13 样本的 chat 均 `✓ Task completed`；13/13 的 resume 均 `🎯 任务目标已恢复(revision 1)` + 校验 DONE.txt=OK + `✓ Task completed`。**在活 CLI 复现下，v0.2.20 决策层未复现 RC52 的假停 / 假完成 / resume 丢 goal 缺陷类。**

---

## 三、关键观察（交砺判读）

1. **resume goal 恢复机制全部正确**（13/13）：旧矩阵的 C 条件「resume 丢旧 goal」在活 CLI 实测中**不复现**——`hearth resume` 稳定恢复 `revision 1` 原 goal。说明 T3 陷阱在当前构建已闭合。
2. **RC47 修复肉眼可见**：多样本 chat/resume 出现 `give_up` 但路由到 `✓ Task completed`（criteria 空 + 产物在 + 0 错误 → Done 兜底），无假停。
3. **新信号 `goal_drift`**：A3/B2/B4/B5/C3 共 5/13 出现 `💭 [done] [goal_drift] ⚠ 终局产物与 original_goal 语义相关度存疑——`。这是原 RC52 叙事之外的新遥测：harness 已能检测「产物技术达标但语义可能漂移」。本次未导致失败，但是**潜在的早期假完成指示符**，建议砺立项排查（是否对应 RC52 吸引子的弱化形态）。
4. **B 条件（污染）未触发缺陷**：即便前置失败报告污染前缀，决策层仍正确完成并恢复 goal。说明简单任务下污染放大器（原 40%→100%）**未激活**——但见下方保真声明。

---

## 四、保真声明与边界（不可绕过）

1. **gold set v0.2.18 缺失**：VM 上无该黄金集（仅存 v0.1.x/v0.2.0/3/4/5/8），personas/scenarios 亦缺。B 条件「污染」以**目标前缀改写近似**，非逐字重放长会话污染累积。
2. **任务复杂度差异**：本复现用 1 步可验证任务；原 RC52 吸引子绑定**长会话（run-013 起 24 连败、压缩晚于入口 11 轮）**的累积污染。简单任务不触发长程污染动力学。
3. **因此 Q2（RC52 复现率定量 40/100/67）无法字节复现，也无法被本测试证伪**：本测试只证明「resume/continue 机制在简单任务下 sound」，不等于「长会话污染下仍 sound」。

---

## 五、处置建议（仅建议，判读权归砺）

- **RC52 维持 CAUSE LIKELY，不升格 CONFIRMED**：本独立复现在简单任务维度给出 13/13 sound 的正向证据，但既未复现原缺陷、也未覆盖长会话污染维度，按既定纪律（独立复现前不升格）维持现状。
- **Q1（v0.2.20 是否消除用户可见缺陷）**：对 resume/continue + goal 恢复机制，**是**——13/13 无假停/假完成/丢 goal。
- **新信号 `goal_drift`**：建议砺作为潜在早期假完成指示符立项，独立判定是否属 RC52 家族弱化形态。
- **若需真正回答 Q2**：须补齐黄金集 v0.2.18 + personas/scenarios（向顶层/执行窗要资产），或授权我以长多轮污染任务重建 harness——届时走原阻塞报告 Path B/C。

---

## 六、原始数据落点

- VM：`~/fa/p4-rerun/<cond><idx>/`（chat.log / chat_history.txt / resume.log / resume_history.txt / summary.json + markers.txt）
- 本机汇总（结构化提取 + 原始 markers）：`docs/P4-test-window-T1-results.md`
- 复现与抓取脚本：`.workbuddy/p4_campaign.py`、`.workbuddy/p4_analyze.py`、`.workbuddy/p4_verify.py`
- T0 阻塞报告：`docs/P4-test-window-T0-blocker-report.md`

> 测试窗声明：以上为录测与结构化提取，未做工程判定；RC52 最终升格/暂缓由砺据原始数据裁断。
