# hearth-slim 手术包二·八卡真机验收报告 v1.0（执行窗→用户/顶层）

- **日期**：2026-09-11 03:00　**验收人**：执行窗　**环境**：本机 Windows MSVC release（0.2.26 ae7b02e）+ Agnes 会员通道（.133 不可达，按既有裁定走本机）
- **环境差异声明**：Windows 无 cgroup（`⚠️ noop·仅开发模式` 降级提示，符合设计）、无 landlock/seccomp——**沙箱语义类判据以 .133/Linux 为准，本报告只判韧性/渲染/语义层**
- **结论先行**：**八卡不全放行**——5 卡过/部分过，**2 项 P0 必修复**（S10 流式工具参数回归、S8 resume 不续跑）+ 2 项 P1

---

## 一、验收结果矩阵

| 卡 | 判据 | 实测证据 | 判定 |
|---|---|---|---|
| **S7** 长退避 | 序列+投影+窗口+fatal 立即 | ✅ gemini 连接失败：`[retry] 第 1 次重试，等 30s（窗口剩余 1778s）`→`第 2 次，等 60s（剩余 1726s）`；✅ 暂停语义 | ⚠️ **部分过**——agnes 连接拒绝/502 被归为 `unrecoverable` 直接暂停（无重试），错误分类不一致 |
| **S8** 断点续跑 | kill-9→resume 续到交付 | ✅ 暂停语义+`断点已存（15c317a6…）修复后 hearth resume … 继续`投影；✅ resume 读到 paused 状态与原目标 | ❌ **未达标**——resume 只报状态不执行（steps=0，"系统未启动"），**续跑未实现**；`.hearth/runs/` 目录不存在（落盘位置待核） |
| **S9** 降级链 | 主通道挂→自动切+分账标注 | ✅ `🔁 [fallback] provider: agnes → gemini（降级链——数据按通道分账，基准对照不可比）`——**切换+通道标注纪律双双落地** | ⚠️ **部分过**——**zhipu 通道未注册**（可用列表：deepseek/openai/gemini/agnes/ollama/vllm），用户第二通道不可用 |
| **S10** 流式+三层 | 打字机+三通道分明 | ✅ `───── 结果 ─────` 分隔线渲染；❌ **流式下 tool_calls 参数解析丢失**——`⚙ web_search →`（空参数）→`missing 'query' argument`→ 全部工具调用失败 → 工具名为空 →`tool not found`→ 历史配对错乱 → 上游 400 | ❌ **P0 未达标**——回归：非流式时代工具调用正常（9-10 魂斗罗/L1-01 实证），S10 流式 delta 拼装引入 |
| **S11** 中断保留 | 打断→追问→续 | 代码级：`interrupt_handle`/`interrupt_notify`/`finalize_interrupted` + 单测 `test_s11_step_boundary_interrupt_pauses_run` 在案 | ⚠️ **待目测**（Windows 无 Ctrl-C 信号语义环境；机制与单测证据在案，留用户/Linux 目测） |
| **S12** 质量自检 | 自检→修复→诚实交付 | 代码级：selfcheck 接线（loop.rs/report.rs/run_local.rs）；✅ 实测中诚实交付特性生效（见 S14 总结块与暂停投影） | ⚠️ **待判别实验**——被 S10 回归阻断（工具全废，产物写不出，魂斗罗质量版无法重跑） |
| **S13** web_search+审计 | 工具可用+五要素 | ✅ 工具注册（10 工具列表含 web_search/web_fetch）；✅ 五要素审计落地（bash 15 处/patch 12/search 12/…） | ⚠️ **部分**——真机调用被 S10 阻断（注册面过，功能面待 S10 修复后复测） |
| **S14** 总结 TL;DR | 六段模板+不阻断+不编造 | ✅ **实测输出六段总结**（一句话/产物/要点/问题/剩余/自检）；✅ **诚实无编造**："因工具调用参数缺失（query/pattern 为空）导致搜索失败，任务未完成"——准确复述实测故障；✅ 降级生效：模型调用失败时模板填充+标"生成失败" | ✅ **过**——本批最亮的一卡 |

## 二、P0/P1 问题卡（退 traecode）

**PC-1（P0）S10 流式 tool_calls 参数解析回归**
- 现象：流式模式下工具调用参数丢失（`web_search →` 空参）、工具名空、消息历史出现 `missing field tool_call_id` 的上游 400
- 影响：**一切工具使用被阻断**（写文件/搜索/bash 全废）——比 S10 本身重要得多
- 定位建议：SSE delta 拼装（`tool_calls[].function.arguments` 增量 concat 与 index 处理）+ 工具结果消息回填的 tool_call_id 配对
- 验收：非流式对照（同任务 `--no-stream` 或降级路径应正常）+ 流式修复后同一任务工具调用成功

**PC-2（P0）S8 resume 不续跑**
- 现象：resume 读取断点状态成功（paused/原目标/断点 id），但**不实际继续执行**——输出总结即退出，steps=0
- 影响：韧性承诺（kill-9/断网/重启后续到交付）未兑现——当前只是"能报状态"
- 附带：`.hearth/runs/` 未落盘（实际落盘位置需自查，S8 卡要求 runs/<run-id>.json）；`hearth resume` 不接受 `--budget`
- 验收：kill -9 后 resume 续跑到交付（真机）

**PC-3（P1）S7 错误分类不一致**
- 现象：agnes 连接拒绝/502 → `unrecoverable` 直接暂停（零重试）；gemini 连接失败 → transient 长退避（正确）
- 影响：主通道抖动仍可能不重试就暂停（虽有暂停语义兜底）
- 验收：不可达 endpoint 三类错误（连接拒绝/超时/502）均走长退避

**PC-4（P1）S9 zhipu 未注册**
- 现象：`未知 provider: zhipu——可用: deepseek/openai/gemini/agnes/ollama/vllm`
- 影响：用户拍板的 agnes→zhipu→gemini 链第二环缺失（暂可用 gemini 代位）
- 验收：注册 zhipu（或改用户链为 agnes→gemini）

**P2 观察项**：config 路径三处不一致（`%APPDATA%/hearth/` 为代码路径、APPDATA 空时 fallback 到 `HOME/hearth/` 非标准位置）；`[fallback]` 投影每次重试重复打印；`hearth config show` 子命令不存在。

## 三、验收通过面小结（已实测工作）

- ✅ **S7 长退避序列与投影**（30s/1m/窗口倒计时，gemini 路径）
- ✅ **S9 降级切换+通道标注**（顶层"降级轮次标通道"纪律落地）
- ✅ **S8 暂停语义+断点投影**（"不再有不可恢复的 failed"达成——400 也走暂停而非 failed）
- ✅ **S14 总结 TL;DR 全项**（模板/诚实/降级/不阻断）
- ✅ **S10 结果通道分隔线渲染**
- ✅ 诚实交付链：失败时总结块如实描写故障原因（无编造）

## 四、待办

1. **trae 修 PC-1/PC-2（P0）+ PC-3/PC-4（P1）** → 执行窗复验
2. 修复后：S12 判别实验（魂斗罗质量版重跑→用户评分）+ S13 真机搜索取证 + S11 目测（Linux）
3. 终局：吴涛 3 真实小任务
4. 待用户：docs 已全恢复（1461 项）确认收到

**原始日志**：`C:/Users/87465/AppData/Local/Temp/hearth-accept/`（accept-A/B/C/D2 全量）
