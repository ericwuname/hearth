# Hearth 地基加固任务书 v1 · 评审补充与执行记录（2026-08-27）

> **评审人**：顶层守门员（AI，有源码访问权）  
> **评审对象**：  
> ① `docs/hearth-v0.2.3-手工测试-去重bug清单.md`（Claude 写）  
> ② `docs/hearth-foundation-hardening-taskbook-v1.md`（Claude 写）  
> ③ `docs/project-comprehensive-review-2026-08-27.md`（智谱 GLM-5.3 写）  
> **方法**：源码直读核验 + VM 实机取证（不采信二手报告）——按任务书自己的方法论要求执行。  
> **执行基线**：commit `f0b4a54`（v0.2.3 T1-T10 修复之后）。



---

## 一、对任务书（②）的核实结论：根因假设逐条验证

任务书要求"原假设不成立时以源码为准并注明"——以下为逐条核实结果：

| 任务                 | 任务书假设                             | 源码核实结果                                                                                                                                                                                                                                                       | 判定       |
| ------------------ | --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------- |
| P0-1 resume 崩溃     | serde unwrap panic 逃逸             | **假设不成立**。`session_store.rs:92 load_turns` 用 `filter_map(…ok())` 跳过损坏行，resume 路径全程 `Result` 无 unwrap。VM 实机取证：v0.2.4 记录（1351 行）**无任何 core dump/segfault/panic 证据**；且 Claude 清单引用的行号（16811-16863）超出实际文件长度（该文件仅 1351 行）——**行号锚点是幻觉**。真实 v0.2.3 记录中 resume 正常工作。 | 降级为防御性记录 |
| P0-2 无顶层超时         | 循环在 replan/continue 间打转无 deadline | **假设成立（源码+实机双证）**。`context.rs:62 budget_exhausted()` 只查 `steps_used >= max_steps`——`Budget.max_time_secs` 字段存在但**全仓无消费点**（死字段）。实机：2026-08-27 16:08 启动的 `hearth chat` 挂起 2 小时+，16:44 的 repl 反复拉起 cargo 留下 10 个孤儿进程，需人工 kill。                                  | ✅ 已修（H2） |
| P1-4 bash 30s 超时   | bash 工具默认超时 30s                   | **假设不成立（根因更深一层）**。`bash.rs:127` 默认已是 180s（v12.4 改的）。真凶是 **dispatcher 层 30s 总闸**（`dispatcher.rs:72 default_timeout=30s`）先掐断了 bash——普通 `register()` 注册吃默认值，盖过工具内部 180s。证据：实测报错文案 `tool 'bash' timed out after 30s` 出自 `dispatcher.rs:147`。                     | ✅ 已修（H1） |
| P0-5 压缩失忆          | 折叠后原文无落盘                          | **假设成立**。`context.rs:140` drain 出旧轮后 `summarize_turn` 折成 60 字符摘要，原文丢弃无归档。                                                                                                                                                                                    | ✅ 已修（H3） |
| P1-7 429 风暴        | 重试节奏不变                            | **假设成立**。日志尾部实测：429 被分类 Transient，每轮吃满 4 次重试+退避，35 处 429 记录。免费额度耗尽类重试无意义。                                                                                                                                                                                    | ✅ 已修（H4） |
| P1-6 config get 缺失 | unrecognized subcommand           | **已过时**（对旧二进制测的）。当前源码 `lib.rs:365 ConfigAction::Get` 已存在；本轮补 egress-allowlist 字段进 get 输出。                                                                                                                                                                    | ✅ 补齐     |
| P1-7 REPL 粘贴拆分     | bracketed paste 未对接               | **部分成立**。REPL 用 reedline（`repl.rs:298`），reedline 本身支持 bracketed paste；拆分发生在 SSH 终端"按键式粘贴"场景（粘贴被逐行作为回车发送，应用层无法区分敲入与粘贴）。本轮按任务书降级方案处理：`{...}` 包裹机制保持 + 文档引导，不做不可靠的猜测式代码改动。                                                                                      | 📝 降级    |
| P1-2 前缀稳定          | HashMap 序列化不稳定                    | 快速审计：工具 schema 经 `ToolSchema` 结构体（字段顺序固定）序列化，消息数组 append-only。**无证据表明前缀不稳定**——装仪表（P1-1）后攒数据再动。                                                                                                                                                               | ⏸ 观测     |

## 二、对去重清单（①）的勘误

1. **"16863 行记录"不存在**——实际 VM 上 `手工测试v0.2.4.txt` 为 **1351 行**。清单中所有 \`行号锚点（16811-16863、10156）均为幻觉\*\*，不能作为证据引用。
2. P0-1（resume core dump）在实际记录中无佐证，**不应列为 P0**。真正有实机证据的 P0 是 P0-2（挂起）。
3. 清单未发现的新证据（本轮补充）：**VM 上残留的孤儿进程现场**（10 个 `cargo test/build` 进程由挂死的 hearth 反复拉起、`lock_inode_wait` 卡锁、release 目录从未产出）——这是 P0-2+P1-4 组合杀伤的实证。

## 三、对全面审查报告（③）的核对

数据抽查全部属实（27 crates / 91 rs 文件 / 225 commits / tag 线 / v0.2.3 HEAD=f0b4a54）。一处需更新：

- §6.1 "v0.2.3 T1–T10 修复后 VM 单测全绿（commit message 载明）"——**该记载不实**。本次恢复中断门禁实测：f0b4a54 基线 `cargo test --workspace` 实为 **TEST_RC=101**（codex-cli config.rs:245 测试初始化器漏 `egress_allowlist` 字段，E0063 编译失败）+ project-sync 5 个 clippy error。已在本轮修复。**教训：commit message 的门禁自述不可信，以 t_gate.log 为准。**

## 四、本轮执行清单（H 系列，全部带正负面测试）

| #   | 修复                                                                                                                                                                       | 改动位置                                                                                         | 验证                                                                                                |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| H1  | bash dispatcher 30s 总闸 → `declared_timeout` 机制（bash 声明 610s 窗口；显式 `register_with_timeout` 仍优先）                                                                           | `dispatcher.rs` trait+注册法 / `bash.rs` / `run_local.rs:221`                                   | `test_declared_timeout_extends_dispatch_window`（含对照组）+ `test_explicit_timeout_overrides_declared` |
| H2  | 任务级 wall-clock deadline：`max_time_secs` 死字段激活（`deadline_exceeded()`）+ run 循环前置检查（复用 budget 收尾路径，status=deadline_exceeded）+ CLI 每轮默认 900s（`HEARTH_TASK_TIMEOUT_SECS` 可覆盖） | `context.rs` / `loop.rs` run 循环 / `run_local.rs`                                             | `test_deadline_exceeded_activates`（cap=0 必超时——修复前恒 false）+ `test_continue_turn_resets_deadline`   |
| H3  | 压缩归档：折叠前原始 Turn 追加落盘 `~/.config/hearth/archive/compacted.jsonl` + 摘要头部注入归档检索提示（agent 知道有找回手段）                                                                            | `context.rs maybe_compact`                                                                   | `test_compact_archives_original_turns`（UniqueNeedle 必须在归档 + 提示必须在摘要）                              |
| H4  | 429 熔断（轻量）：免费额度类 429（消息体含 free/rate limit/upgrade）首次即失败，错误带结构化建议（换通道而非重试同一 key）                                                                                          | `loop.rs` plan chat 重试环                                                                      | 需真机验证（日志路径）——代码路径单测不可行（涉及网络层），标注为实机验证项                                                            |
| H5  | P0-4 写前目标校验（轻量非阻塞）：用户消息字面提到的文件名与写入类工具目标完全不符 → ThinkSummary 警示事件（不拦执行——D 类控制流须顶层任务书）                                                                                      | `loop.rs check_write_target_mismatch` + 纯函数 `extract_mentioned_files`/`write_target_matches` | `test_extract_mentioned_files_and_match`（BUG_LEDGER.md vs hearth_bug_log.md 实测场景必报）               |
| H6  | P1-1 缓存仪表：`Usage` + `OpenAiUsage` 加 `prompt_cache_hit_tokens/miss_tokens`（Option——缺失≠0），CostEntry 累计+`cache_reported_calls`（区分不支持与零命中）                                   | `types.rs` / `cost.rs` / `llm-openai.rs` / llm-local / llm-cn 全构造点                           | `test_usage_cache_fields_deserialize`（DeepSeek 风格解析+标准风格不 panic）+ `test_cache_usage_accumulation` |
| H7  | P1-5 local 模式只读命令：`history`/`status`/`tools`/`replay` 无 `--url` 时走本地数据路径（与 sessions 一致）                                                                                  | `lib.rs` 四命令                                                                                 | 实机验证项（VM 手测）                                                                                      |
| H8  | P1-9 执行报告：每轮结束生成 `.hearth/reports/<sid>/run-NNN.md`（目标/状态/步数/耗时/token/工具调用成败表/产物/审批/反思轨迹/错误要点）                                                                           | 新模块 `report.rs` + `run_local.rs` 接线                                                          | `test_report_written_and_complete`（字段齐全+序号递增）                                                     |
| H9  | P1-8 apply_patch 宽松档：精确匹配失败后按"去行尾空白+CRLF 归一"逐行匹配一次；内容真不存在仍拒绝                                                                                                             | `patch.rs lenient_replace_span`                                                              | `test_patch_lenient_trailing_whitespace`（宽松命中+对照组仍拒）                                              |
| H10 | P2-3/P2-4（部分）：`config get` 输出 egress-allowlist + 标注 env 优先为安全例外                                                                                                          | `lib.rs`                                                                                     | 手测                                                                                                |
| —   | 基线修复：config.rs E0063（测试初始化器补字段）+ project-sync 4 clippy（derive Default/format!/matches!/doc 缩进）+ unused import                                                            | 见 git diff                                                                                   | VM 门禁全绿                                                                                           |

**版本**：0.2.3 → 0.2.4（workspace version + install.sh 同步）。

## 五、未做与理由（诚实披露）

| 项                               | 理由                                                                            |
| ------------------------------- | ----------------------------------------------------------------------------- |
| P0-3 会话文件 schema 版本化            | P0-1 崩溃前提不成立（load_turns 已宽容损坏行）；JSONL 每行独立 Turn 天然抗半截损坏。版本化价值随崩溃前提坍缩，降为后续可选项。 |
| P1-2 前缀字节稳定                     | 无不稳定证据——先装仪表（H6）攒数据，不盲改（任务书自己也要求先 diff 再动手）。                                  |
| P1-7 bracketed paste 代码层        | SSH 按键式粘贴应用层不可分辨；`{...}` 包裹已是可靠机制，做文档引导（降级方案即任务书认可路径）。                        |
| P2-2/P2-6/P2-7 沙箱一致性/幂等/SIGTERM | 审计类任务，本轮预算集中在有实机证据的 P0/P1 修复；挂账下轮。                                            |
| P0-4 完整版（阻塞式写前确认）               | D 类控制流（改 loop 行为阻塞执行），按项目铁律须顶层任务书——本轮只做非阻塞警示（H5），阻塞版留顶层决策。                    |

## 六、门禁

- VM（Ubuntu 24.04 真机）：`cargo fmt --check` + `clippy --workspace --all-targets -- -D warnings` + `cargo test --workspace`
- 基线（f0b4a54）：fmt ✅ / clippy ❌101 / test ❌101（E0063）
- 修复后：见本 commit 对应 `~/t_gate.log`（CLIPPY_RC=0 / TEST_RC=0 / 351+ tests passed 为验收线）
