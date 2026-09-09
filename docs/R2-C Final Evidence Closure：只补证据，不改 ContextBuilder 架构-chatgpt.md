按照你们现在的节奏，我不会再让执行窗口碰 ContextBuilder 主体。下一单直接限定成：

R2-C Final Evidence Closure：只补证据，不改 ContextBuilder 架构。

① 用 list_tools() 排序修复后的最新二进制，在 DeepSeek 同口径下重采 tools_hash，确认 unique=1；
② 调用独立 Claude 评审窗口执行 docs/data/rubric-blind-pack/ 的 4 样本盲评，回传逐项 0-2 分及与 EC-03 baseline 的差异，不由执行窗口自行评分；
③ 将上述结果追加至 R2-C Closure 报告；
④ T3 活性判据问题只归档为独立 OPEN，不得在本窗口修复；
⑤ ContextBuilder / D 类禁改清单继续全部生效；
⑥ 若①②无结构性反证，则提交 R2-C Final Acceptance，不得再扩大施工范围。

---

# 守门员批注（2026-08-29 · Closure Window 交付独立复核后补充，与正文同效力）

> 复核实证：VM `~/t_gate_r2c.log` 四 RC 全 0、**TOTAL_PASSED=387** ✅；`list_tools` 按名排序修复落地（`dispatcher.rs:151`）；commit 链至 `38f2665` ✅。T3 根因声明与我的独立 grep 一致（loop.rs 中 `TaskNode.status` 赋值仅存在于测试代码）。以下 6 条随 ①~⑥ 执行。

## 补充 1：MISS-B 分类必须修订——"顺序漂移"是实锤污染源，不是 unlikely

`list_tools` 修复前 tools 数组顺序**每请求随机**（HashMap 迭代）——这直接推翻 EC-02 表里 "MISS-B tool schema 变化 = unlikely" 的判断：**内容 unlikely 成立，但顺序 confirmed 污染源（本轮已修）**。两个连锁动作：①EC-02 分类表加此勘误；②修复前 DeepSeek 54 请求的命中率数字含此噪声——**①的重采（新二进制）才是干净数字**，预期命中率上升；若 tools_hash unique=1 而命中率仍不升，则 MISS-E（provider 粒度）权重进一步增大——这本身就是有价值的证据。

## 补充 2：T3 活性判据发现 = RC1 家族新实证，且直接波及 R2-D 的价值兑现

`TaskNode.status` 生产路径无更新（我的独立 grep 与 trace 证据一致：6/6 工具成功但 reflect 看到 0/2）——**波及面不止 reflect prompt：Continuity 块注入给模型的 completed/remaining 同样失真**（模型一直看着"0 完成"的假状态工作）。因此 T3 修复（活性判据）不是普通 OPEN：
- **优先级建议升至 D 类四件之前**（它使 R2-D TaskGoal 的价值在真实任务里打折）；
- 修复设计必须回答"**node status 的生产者是谁**"——推荐对齐 WS13 三重校验（写盘 verify 通过后标记，而非 LLM 自报——RC1 教训：活性判据不能靠自报）；
- reflect prompt 的 `Goal:` 字段显示图首节点而非 original_goal——小项一并修；
- 触碰 TaskGraph 事实模型 → 独立施工单 + 顶层批准（Closure 禁改边界遵守，本批注只定优先级）。

## 补充 3：①重采一石二鸟

tools_hash unique=1 确认 + **同任务集干净命中率对照表**（修复前 54 请求 vs 修复后 N 请求，pre/post 两列并排）——一张表同时关闭 tools_hash OPEN 项和 MISS-B 修订的实证。

## 补充 4：DeepSeek 余额留意

通道验活时余额曾报 ¥4.4——本轮 4 任务后仍有余量，①重采（4 任务）规模相同应够；跑前瞄一眼余额，不足则充值或换通道并标注口径。

## 补充 5：replan 换图重建 = 设计内代价——追认 + 一个 QUARANTINE 级备忘

"topology 以图结构为单位稳定，replan 换图即重建"语义正确（结构变了重建是诚实行为）。备忘一个未来优化候选（**本轮与后续施工单均不做**）：replan 常见"同结构微调"（加/改一节点）也触发整块重建——增量拓扑注入（拓扑 diff）可进一步保前缀，收益待实测，先进 QUARANTINE 备忘录。

## 补充 6：gate 387 实证确认

隔离门禁四 RC 全 0 + 387 passed（381+6 新测试，算术吻合）——本轮交付的门禁证据链闭合。⑥ 的 Final Acceptance 裁决输入已齐：本报告 §3-§8 + ①②回传 + 盲评回传。