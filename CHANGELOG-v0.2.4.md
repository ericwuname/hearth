# CHANGELOG v0.2.4 — 地基加固（2026-08-27）

> 来源：`hearth-foundation-hardening-taskbook-v1.md`（Claude 评审任务书）经源码级核实修正后执行。
> 评审与执行记录：`docs/hearth-v0.2.4-hardening-review-and-execution.md`。
> 触发证据：手工测试 `手工测试v0.2.4.txt`（VM 实机 1351 行）+ VM 挂死进程现场取证。

## P0 修复（阻断"可信委托"承诺）

### 任务级 wall-clock deadline（原任务书 P0-2，代号 H2）
- **根因**：`Budget.max_time_secs` 是死字段——`budget_exhausted()` 只查步数，时间预算全仓无消费点。实机：`hearth chat` 挂起 2 小时+需人工 kill。
- **修复**：`ContextManager` 增加 `deadline_exceeded()`/`run_elapsed_secs()`/`mark_run_start()`；run 循环步数检查前置 deadline 检查，复用 budget 收尾干净路径（`status=deadline_exceeded`）；CLI 每轮默认 900s（`HEARTH_TASK_TIMEOUT_SECS` 环境变量可覆盖）。
- **测试**：`test_deadline_exceeded_activates`（cap=0 必超时——修复前恒 false）、`test_continue_turn_resets_deadline`。

### 上下文压缩归档（原 P0-5，代号 H3）
- **根因**：`maybe_compact` 折叠旧轮为 60 字符摘要后原文直接丢弃，无归档落盘——"压缩"等价"永久丢失"。
- **修复**：折叠前原始 Turn 追加落盘 `~/.config/hearth/archive/compacted.jsonl`（best-effort 不阻塞）；摘要头部注入归档 grep 检索提示（agent 知道有找回手段）。
- **测试**：`test_compact_archives_original_turns`（UniqueNeedle 必须在归档 + 提示必须在摘要）。

### 写前目标校验·非阻塞警示（原 P0-4 轻量版，代号 H5）
- **根因**：用户指定写 `BUG_LEDGER.md`，agent 却新建 `hearth_bug_log.md`，全程零信号。
- **修复**：写入类工具（write_file/edit/apply_patch）目标路径与用户消息字面提到的文件名完全不符时产出 ThinkSummary 警示事件（不拦执行——阻塞式确认涉 D 类控制流须顶层任务书）。
- **测试**：`test_extract_mentioned_files_and_match`（实测场景 BUG_LEDGER.md vs hearth_bug_log.md 必报）。

## P1 修复

### bash 工具 30s 总闸（原 P1-4，代号 H1）
- **根因（比任务书假设更深一层）**：bash 工具内部默认已是 180s（v12.4），真凶是 **dispatcher 层 30s 默认总闸**——普通 `register()` 注册吃 `default_timeout=30s`，盖过工具内部 180s。
- **修复**：`Tool` trait 新增 `declared_timeout()`（默认 30s）；bash 声明 610s 窗口；新注册法 `register_with_declared_timeout`（显式 `register_with_timeout` 仍优先）。
- **测试**：`test_declared_timeout_extends_dispatch_window`（含对照组）、`test_explicit_timeout_overrides_declared`。

### 429 免费额度快速失败（原 P1-3 轻量版，代号 H4）
- **根因**：429 被分类 Transient，每轮吃满 4 次重试+退避（实测 35 处 429 记录）；免费额度耗尽重试同一 key 无意义。
- **修复**：消息体含 free/rate limit/upgrade 的 429 首次即失败，错误带结构化建议（换 provider/等窗口，而非重试）。

### 缓存命中率仪表（原 P1-1，代号 H6）
- **修复**：`Usage`/`OpenAiUsage` 增加 `prompt_cache_hit_tokens`/`prompt_cache_miss_tokens`（Option——缺失≠0）；`CostEntry` 累计缓存命中并记录 `cache_reported_calls`（区分"通道不支持"与"真零命中"）。为 P1-2 前缀稳定性诊断装表。
- **测试**：`test_usage_cache_fields_deserialize`、`test_cache_usage_accumulation`。

### local(auto) 模式只读命令（原 P1-5，代号 H7）
- **修复**：`history`/`status`/`tools`/`replay` 无 `--url` 时走本地数据路径（会话文件/dispatcher），与 `sessions` 行为一致——不再误报"需要 --url"。

### 结构化执行报告（原 P1-9，代号 H8）
- **修复**：每轮 run 结束生成 `.hearth/reports/<session_id>/run-NNN.md`：目标/状态/步数/耗时/token/工具调用成败表/产物文件/审批/反思轨迹/错误要点。不翻原始终端 log 即可复盘。
- **测试**：`test_report_written_and_complete`。

### apply_patch 宽松匹配档（原 P1-8，代号 H9）
- **根因**：SEARCH 块要求逐字节精确匹配，行尾空白/CRLF 差异即拒（高频摩擦）。
- **修复**：精确匹配失败后按行规范化（去行尾空白+CRLF→LF）逐行匹配一次；命中时用文件真实字节区间替换；内容真不存在仍正确拒绝（宽松≠放水）。行偏移用字节级 `\n` 扫描（`lines()` 剥 `\r` 会错位）。
- **测试**：`test_patch_lenient_trailing_whitespace`（宽松命中+对照组仍拒）。

## P2 修复

- **config get 补全（P2-3/P2-4 部分，H10）**：`config get` 输出 egress-allowlist 字段；标注 env 优先合并为安全例外（环境变量可强制收紧白名单不被配置文件放宽）。
- **P1-6 说明**：`config get` 已在 v0.2.3 后存在（手工测试对旧二进制），本轮补齐 egress 字段与未知字段错误提示。

## 基线修复（上轮遗留）

- `codex-cli/config.rs:245` 测试初始化器补 `egress_allowlist` 字段（E0063——上轮 commit 自述"VM 单测全绿"不实，实测 TEST_RC=101）。
- project-sync 4 处 clippy（derive Default 替代手写 impl / useless format! / matches! 宏 / doc list 缩进）+ unused import。

## 评审勘误（对任务书的核实结论）

- **P0-1 resume 崩溃：原假设不成立**——`load_turns` 用 `filter_map` 跳过损坏行全程无 panic；实测记录（1351 行）无任何 core dump 证据；任务书引用行号（16811-16863）超出文件长度，为幻觉锚点。P0-3 版本化价值随崩溃前提坍缩，降为可选。
- **P1-7 REPL 粘贴拆分：降级处理**——SSH 按键式粘贴应用层不可分辨，`{...}` 包裹是可靠机制，文档引导（任务书认可的降级路径）。
- **P1-2 前缀稳定：先观测**——无不稳定证据，H6 仪表装好后攒数据再动。

## 未做（挂账下轮）

P0-3 会话版本化（前提坍缩）、P1-2 前缀字节稳定（待数据）、P2-1 裸日志审计、P2-2 沙箱一致性审计、P2-5 文档数值核对、P2-6 幂等审计、P2-7 SIGTERM 对等、P0-4 阻塞式写前确认（D 类须顶层任务书）。

## 门禁

VM（Ubuntu 24.04 真机）：`cargo fmt --check` RC=0 ✅ / `clippy --workspace --all-targets -- -D warnings` RC=0 ✅ / `cargo test --workspace` RC=0，**351 passed / 0 failed**（61 suite 全 ok，`~/t_gate.log` 可查）。
