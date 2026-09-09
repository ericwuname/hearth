# 宪法条文 ↔ 架构执行点映射表 v1.0（R5-12）

> **依据**：《Hearth智能性根治长程任务包 v1.0（顶层）》R5-12——"团队不缺正确认知，
> 缺的是把认知落进机制"。本表把 Codex Gene Constitution（`constitution.md`，八条）
> 的关键条文与 agent-core 的架构执行位一一对应，**可抽查触发**（每行有机器断言或
> 日志锚点；`test_r512_constitution_execution_points_wired` 锁死本表存在与锚点在位）。
> **纪律**：条文文本不进代码（`constitution.md` 运行时读取，v13 S3-a 契约）；
> 代码只实现条文的**可执行语义**。

| 条文 | 可执行语义 | 执行位（锚点） | 触发时机 | 抽查方式 |
|---|---|---|---|---|
| **第 1 条 · 真实第一**（Never pretend / Verify, don't guess） | 验证证据看退出码：失败命令不点亮 VERIFIED | `loop.rs` record_tool_exchange 的 R5-8 块（`R5_8: verification command failed` warn） | 每次 bash 验证命令执行 | 单测 `test_r58_failed_verification_command_does_not_light_verified`；tracing 日志 `R5_8` |
| 第 1 条 | 终态验证标注封闭集（VERIFIED/UNVERIFIED） | `AgentLoop::verification_state()` | 每次终态投影 | CLI 投影 + R1-1 三臂测试族 |
| 第 1 条 | sticky VERIFIED 防回退（证据是 turn 级事实） | `run()` 起始 `verification_evidence = false` | 每 run 起始 | 单测 `test_verification_evidence_reset_per_run` |
| 第 1 条 | 验证命令分类（bash -c 解包/只读表/深度上限） | `is_verification_command()`（R4.1） | 每条 bash 命令 | R4.1 测试族 |
| 第 1 条 | 验收标准确定性核验（exit code / 文件内容） | `verify_acceptance_criteria`（Node 03 acceptance） | criteria 存在的 completed 前 | Node 03 测试族 |
| **第 5 条 · 局外人视角**（What would a skeptic say?） | Reflect 怀疑者检测：事实显示真进展而判弃 → 冲突记录（不吞声） | `classify_reflect_fact_conflict` + scratch `reflect_fact_conflict` + `REFLECT_FACT_CONFLICT` warn | 每次 GiveUp 判定臂 | Node 03/04 测试族；scratch 投影可读 |
| 第 5 条 | Reflect 决策依据显式投影（决策 → X；依据：错误/写盘/产物事实） | run 循环 Reflect 臂 `why` 行（emit_think_summary） | 每次 reflect 后 | 事件流/CLI 投影可读（RC52 纪律） |
| 第 5 条 | 失败事实回喂决策者（证据到达，非叙事自报） | R5-1 `Last Failure Facts` 注入块 + R5-3 强制换策略 | 上轮失败 → 下轮 messages | 单测 `test_r51_*` / `test_r53_*` |
| **全部条文** | 宪法全文进系统提示（运行时读文件，禁硬编码摘要） | `constitution::constitution_prompt()` ← build_messages 尾部注入 | 每轮 system prompt | wiring assertion `constitution-reads-file`（v13 S3-a）+ 本表锁 |

## 未落执行位的条文（诚实边界）

- **第 2 条（忠实意图）/ 第 3 条（资源守护）/ 第 4 条（认知未知）/ 第 6 条（连续存在）/
  第 7 条（trust-but-verify）/ 第 8 条（求真）**：部分语义已由既有机制间接承载
  （预算系统=第 3；[UNKNOWN] gap 注入=第 4；Hearth.md/WS7 provenance 提示=第 7），
  但**尚未逐条配独立执行位**——不冒充已配齐。后续按 R5-12 同款流程逐条立项。
- **更新纪律**：宪法文本变更（需顶层批）时，本表必须同轮复核；执行位锚点重命名时，
  `test_r512_constitution_execution_points_wired` 会红——红即表与代码漂移，先修表。
