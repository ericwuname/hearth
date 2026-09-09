# Hearth v0.2.3 手工测试 — 缺陷任务书（交执行窗口）

> **来源**：`release/手工测试v0.2.3.txt`（VM 真机 REPL，commit `5a0be40`）+ 用户复盘（2026-08-27）  
> **编写角色**：顶层（架构/守门员）｜ **状态**：待执行窗口认领  
> **目标读者**：执行窗口（照此施工、跑门禁、交证据）  
> **门禁铁律**：不信报告信源码；每个 patch 下笔前锚定真实源码行；修复后必须跑验证命令并贴证据。

---

## 0. 背景与优先级

用户用 Agnes 会员通道做真人驱动测试，结论：**agent 基础能力弱的根因不在模型，在地基与接线漏**——`introspect` 的上下文填充指标漏算系统提示词（误导）、provider 单通道死、沙箱误伤 git；压缩机制本身已接线（history≥32k 即触发），但油箱表不反映真实负载。同时 **API 成本异常高（14 元≈啥也没干）**，根因是 agentic loop 放大 + 每请求背负的固定 `system_text` + provider 重试。

**优先级**：

- **P0（阻塞 Agnes 测试 / 烧钱 / 联网全废）**：T1（provider+model 选择）、T2（provider 错误分类+fallback）、**T10（出网白名单 ctx.env 从未注入→联网全拒）**
- **P0/P1（成本+稳定性）**：T3（压缩度量修复）、T4（系统提示词瘦身）、T5（planner 调用降本）
- **P1（VM 自校验 / 交互治理）**：T6（沙箱 /dev/null）、T7（seccomp node/python3）、**T11（白名单缺 agent 提议+审批流）**
- **P2（体验/稳定性）**：T8（ambiguous_option 标签）、T9（provider 死时工具消失）

---

## 1. 任务总览表

| 编号 | 名称                           | 严重度      | 源码锚点                                                                                                    | 验收                                                 |
| -- | ---------------------------- | -------- | ------------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| T1 | provider/model 自定义选择         | 🔴 P0    | `run_local.rs:61-135`, `config.rs:119`                                                                  | `--model`/`HEARTH_MODEL` 生效；agnes/custom 可用        |
| T2 | provider 错误分类 + 多通道 fallback | 🔴 P0    | `llm-openai/lib.rs:364-436`, `llm-gateway/types.rs:88-121`, `loop.rs:1636-1649`, `run_local.rs:489-495` | 死通道判非 Transient+提示换通道；单通道死能降级                      |
| T3 | 上下文填充指标纠偏（纳入 system_text）    | 🔴 P0/P1 | `context.rs:92,97`, `loop.rs:610-622,944,956+`                                                          | `introspect` 暴露真实总负载（history+system_text），压缩判据据此校准 |
| T4 | 系统提示词瘦身                      | 🟡 P1    | `loop.rs:956-1042`, `run_local.rs:188-202`                                                              | 实测 system_text 下降 ≥30%、单请求总 token 下降               |
| T5 | planner 调用降本                 | 🟡 P1    | `loop.rs` `do_plan`/`do_reflect`                                                                        | 简单任务 step 数与 LLM 调用数下降                             |
| T6 | 沙箱放行 /dev/null               | 🔴 P1    | `crates/sandbox/src/lib.rs:316,369,374,763`                                                             | VM 上 `git`/`>/dev/null` 正常                         |
| T7 | seccomp 放行 node/python3      | 🔴 P1    | `crates/sandbox/src/lib.rs:535,659,701,714`                                                             | VM 上 `node --version`/`python3` 不 SIGSYS           |
| T8 | ambiguous_option 选项丢空格       | 🟡 P2    | `planner/src/lib.rs:157-185`(extract_options)                                                           | 多词选项标签完整可读                                         |
| T9 | provider 死时工具短暂消失            | 🟡 P2    | `tool-runtime/src/dispatcher.rs:129`                                                                    | provider 异常不导致 tool not found                      |
| T10 | 出网白名单形同虚设（ctx.env 从未注入）    | 🔴 P0    | `web.rs:93-97`, `run_local.rs:159,240`, `repl.rs:250`, `session.rs:267`, `context.rs:20`, `config.rs`    | config/env 配白名单后 web_fetch 真实放行；现有 deny 单测补"配了能放行"用例 |
| T11 | 出网白名单缺"agent 提议 + 用户审批"流 | 🟡 P1    | `web.rs:98-105`, `api/lib.rs:155`, `loop.rs:1865`                                                       | 未白名单域触发审批，approve 后放行并落盘持久化；deny 维持拒绝                  |



---

## 2. 逐任务规格

### T1 — provider / model 自定义选择（P0）

- **现象**：用户想换 Agnes，CLI 报"未知 provider: Agnes——可用: deepseek / openai / ollama / vllm"；且无 `--model` 参数、无 `HEARTH_MODEL`，`model` 只能写 `config.toml`。
- **根因（已核）**：
  - `crates/codex-cli/src/run_local.rs:61-135`：provider 仅 `deepseek/gemini/openai/ollama/vllm` 硬编码 match，无 `agnes` 也无通用 `custom`/`openai-compatible` 分支。
  - `crates/codex-cli/src/config.rs:119`：`model` 仅来自 `self.model`（config.toml），`resolve()` 未读 `--model`/`HEARTH_MODEL`。
  - `lib.rs` Chat/Repl 参数表无 `--model`（已核：仅 `--provider`/`--url`/`--api-key`）。
- **修复方案**：
  1. `config.rs` 的 `resolve()` 给 `model` 增加 `cli_model`/`HEARTH_MODEL` 覆盖（保持 arg>env>file 三级）。
  2. `run_local.rs` 增加 `agnes`（或通用 `custom`/`openai-compatible`）分支：`OpenAiProvider::new("agnes", model, Some(url), key)`，url 缺省 `https://api.agnes-ai.cn/v1`，model 缺省 `agnes-2.5-flash`。
  3. 更新 `--help` 文本与 `other =>` 报错里的可选 provider 列表（含 gemini/agnes）。
- **验收**：`hearth --provider agnes --model agnes-2.5-flash repl` 或 `HEARTH_MODEL=agnes-2.5-flash hearth repl` 能直连 Agnes；help/报错列表正确。
- **验证命令**：
  ```bash
  cargo build -p codex-cli
  ./target/debug/hearth --help | grep -i model
  # VM 真机：HEARTH_MODEL=agnes-2.5-flash HEARTH_API_KEY=<key> hearth --provider openai --url https://api.agnes-ai.cn/v1 repl
  ```
- **临时绕过（已验证可用，无需改码）**：`hearth config set provider openai` + `config set url https://api.agnes-ai.cn/v1` + `config set model agnes-2.5-flash` + `config set api-key <key>`。

### T2 — provider 错误分类 + 多通道 fallback（P0）

- **现象**：末段 deepseek 经 Agnes 分发器的通道下线，harness 反复提示"瞬时故障，请重试"，planning 全死；日志里记的 `HTTP 503 model_not_found ... AgnesAI_error` 是用户/agent 的转述。
- **根因（已核源码，推翻原"503 被误判 transient"判断）**：
  - `llm-openai/src/lib.rs:364-376`：HTTP 状态码映射**已是正确**——`429→Transient`、`≥500→Fatal`、`4xx(含 model_not_found)→Param(不重试)`。即干净的 503 与 4xx **不会被误判 transient**。
  - **真正的缺陷在连接级/传输错误**：当通道离线（分发器不可达、connection reset、DNS 失败）时，请求走 `llm-openai/src/lib.rs:419-436` 的 `Err(e)` 分支 → 一律包成 `LlmError::Transient`；`classify_anyhow`（`llm-gateway/src/types.rs:95-108`）启发式也会把含 "connection/reset/network/send request" 的错误判 Transient。`loop.rs:1636-1649` 据此重试（≤4 次/120s），`run_local.rs:489-495` 只要错误串含 "transient" 就打印"provider 通道瞬时故障——请重试"。于是**死的通道被当成瞬断无限重试 + 误导用户重试**，与用户观察到的症状一致。
  - 多通道：VM 上 `provider=deepseek` 实际只走 Agnes 分发器单点（`run_local.rs:62 build_provider` 无 fallback），分发器下线即全脑停摆。
- **修复方案**：
  1. 区分"端点不可达/通道下线"与"瞬时网络抖动"：连接级失败（connection refused / unreachable / DNS / 通道持续 5xx）→ 判为 **Fatal/切换通道** 类（建议 `hearth config set provider <其他通道>`），而非 Transient 重试；真正的瞬时（超时/429）才重试。
  2. `run_local.rs:489-495` 错误呈现区分"重试（瞬断）"与"换通道（端点死）"，Fatal 文案不得含"请重试"。
  3. `build_provider`（`run_local.rs:62`）为 deepseek 增加备用通道或多 provider 链，单点死可降级。
- **验收**：构造"端点不可达"错误，断言分类为非 Transient（Fatal/切换）且重试次数=0、文案提示换通道；构造"超时/429"仍判 Transient 重试。主通道死时给出可行动切换提示。
- **验证命令**：
  ```bash
  cargo test -p llm-gateway classify_anyhow   # 已有 test_error_classification_table，补"connection refused→非 Transient"用例
  # 单测：注入 connection refused / DNS 失败响应，断言非 Transient
  ```
- **锚点（已钉）**：`llm-openai/src/lib.rs:364-376`（HTTP 状态映射）、`:419-436`（传输错误→Transient）、`llm-gateway/src/types.rs:88-121`（classify_anyhow）、`loop.rs:1636-1649`（重试）、`run_local.rs:489-495`（"请重试"呈现）。

### T3 — 上下文填充指标纠偏（纳入 system_text）（P0/P1，地基/可观测性）

- **现象**：`introspect` 显示 `context_fill_pct` 85%→100%，但会话在 100% 后仍能继续对话（用户已现场证伪"满仓即崩"）。
- **根因（已核源码，推翻原"压缩失效"判断）**：
  - `crates/agent-core/src/context.rs:92` `COMPACT_CHAR_THRESHOLD=32000`；`:97` `estimate_chars()` **只统计 `state.history`**（对话+工具返回），**完全不含** `build_messages` 拼接的 `system_text`。
  - `loop.rs:610-622` 的 `context_fill_pct = estimate_chars()/32000`（封顶 100）＋ 同值 `context_chars`。即填充表**只反映 history，彻底漏掉 system_text**。
  - `loop.rs:944` `build_messages` 每次前调 `maybe_compact()`，触发条件即 `estimate_chars() >= 32000`——**压缩机制本身已接线且应正常工作**。日志 session 实测 `context_chars` 到 42751（≥32k）且后续仍持续多轮对话 ≈ 压缩已静默触发、腾出空间，与"满仓还能聊"一致。**因此"压缩从未触发/形同虚设"的结论是错的**（那是基于日志 agent 误把 27398 当成"含系统预置"的误判）。
  - **真正缺陷 = 可观测性误导**：系统提示词（宪法+目标+规则+8 工具描述+天赋调度）是每请求附加的大块固定开销，对模型真实负载举足轻重，却完全不进 `introspect` 的油箱表 → 用户/agent 看到的"85%"严重低估真实上下文，误判余量。
- **修复方案**：
  1. `update_body_state`（`loop.rs:609`）在算 `context_chars/fill_pct` 时，**额外统计 `system_text` 字符数**（在 `build_messages` 算完 `system_text` 后存一份长度到 ctx_mgr，或在此处重建估计），让 `introspect` 同时暴露 `history_chars` / `system_chars` / `total_chars` 与 `total_fill_pct`（分母可设为模型真实上限或 32k+system 的校准值）。
  2. 告警/预算阈值基于 `total` 而非仅 history，避免"表绿实则满"。
  3. 压缩判据维持 history-based（system 是固定块、不增长，history 才是增长项），**不要**为凑触发把 system 塞进 `estimate_chars`（否则新会话一上来就误触发）。
- **验收**：`introspect` 输出含 `system_chars`/`total_chars`；真机新会话即能看到 system_text 真实体量；在 tracing 中 grep `context compacted (WS4)` 确认压缩确实发生（非空口假设）。
- **验证命令**：
  ```bash
  cargo test -p agent-core test_maybe_compact_folds_old_turns
  # 真机：开新会话 → introspect 记录 history/system/total；另开终端 grep 运行日志 "context compacted"
  ```

### T4 — 系统提示词瘦身（P1，成本）

- **现象**：每请求 `build_messages` 都会拼接一份固定 `system_text`（角色+目标+规则+8 工具描述），其真实体量未在 `introspect` 中体现，但实打实占 token、花钱。真机体量需实测（见修复方案），此前日志里"27,398 字符里系统预置占大头"的归因是**错的**——源码证明 `context_chars` 只计 history，根本不含 system_text。
- **根因（已核源码）**：`loop.rs:956-1042` `build_messages` 拼 `system_text`，构成 = 大段 coding-agent 指令块（含 8 工具 prose 描述，`:982-996`）+ Trust-but-Verify(`:1021-1026`)+ Gene Constitution(`constitution_prompt()`, `:1030-1031`)+ 天赋调度注入(`:1039`)+ 项目记忆 Hearth.md(`:1044+`)。另每次请求还随附 8 个工具的 function-calling JSON schema（来自 `scheduler.tool_schemas()`）。`run_local.rs:188-202` `build_dispatcher` 实际只注册 **8 个工具**（Bash/Read/Edit/Patch/Glob/Grep/Introspect/Web——日志 agent 说的"11 个"是误数）。这是每请求固定开销，与对话长度无关。
- **修复方案**（任一或组合）：
  1. 工具 schema 懒加载/精简：仅当前阶段相关工具的描述进提示，其余按需注入。
  2. constitution / 规则文本压成要点（去掉示例性长文）。
  3. 让 `context_fill` 计入 `system_text`，使填充表反映真实负载（与 T3 协同）。
- **验收**：实测 `system_text` 字符数较基线下降（目标 ≥30%），且单请求总 token 下降；真机对话 transcript 比对确认不损功能。
- **验证命令**：
  ```bash
  # VM 真机：新会话首条消息后 introspect，记录 context_chars / context_fill_pct
  ```

### T5 — planner 调用降本（P1，成本/性能）

- **现象**：规划相位单次 10s–200s（日志 `耗时 189018ms` 一条 plan），简单 HTML 游戏拖数小时；reflect 每步必调 LLM。
- **根因（已核源码）**：`loop.rs:1278` `do_plan_inner`→`planner.decompose(LLM)`（自 v12.6 受 `needs_decompose` 门控，仅重规划时重跑）；`loop.rs:2311` `do_reflect`→`planner.reflect(LLM)` **每步必调**。每步 reflect 一次 LLM + 随每请求背负的大 `system_text` + provider 重试，叠成高成本/高延迟；上下文膨胀→重规划频发进一步放大。
- **修复方案**：
  1. 评估 planner 调用频次：纯问答/一次性生成类走轻量路径（已在 `loop.rs:950` 有 `goal_requires_product` 轻量提示，可扩展）。
  2. plan 结果短期缓存，避免同目标重复长延迟规划。
  3. 重规划设冷却/去重，避免上下文轻微变化就整轮重排。
- **验收**：简单生成任务（如写单文件）step 数与 LLM 调用数较基线下降 ≥30%；真机计时对比。
- **验证命令**：
  ```bash
  cargo build -p agent-core
  # VM 真机：同一简单任务前后计时 + 统计 span [plan] 次数
  ```

### T6 — 沙箱放行 /dev/null（P1）

- **现象**：VM 上 `bash: /dev/null: 权限不够`、`git ... fatal: could not open '/dev/null'` → git 与任何 `>/dev/null` 重定向不可用。
- **根因（已核源码）**：沙箱 child 进程经 landlock 限制 FS 访问——`apply_landlock_in_child`（`crates/sandbox/src/lib.rs:316`）只对 `writable_paths`/`read_only_paths` 调 `add_landlock_rule`（`lib.rs:763`）放行；landlock 对未显式授予的路径**全部拒绝**。`/dev/null` 不在授予集合内 → 打开即 EPERM/EROFS，破坏 git 与所有重定向。（与 seccomp 无关，seccomp 管 syscall 不管路径。）
- **修复方案**：在 landlock 授予集合里显式加入 `/dev/null`（只读，FS_RO）。位置：`apply_landlock_in_child` 组装 `read_only_paths` 处，或直接在 `:369/:374` 之外补 `add_landlock_rule(ruleset_fd, "/dev/null", FS_RO)`。
- **验收**：VM 沙箱内 `: > /dev/null && echo ok` 与 `git rev-parse --is-inside-work-tree` 不报权限错误。
- **验证命令**：
  ```bash
  cargo test -p sandbox test_p5_landlock_allows_workspace_write  # 现有 landlock 单测基线
  # VM 真机：hearth repl 内 bash → `git status` 与 `echo x > /dev/null` 正常
  ```
- **锚点（已钉）**：`crates/sandbox/src/lib.rs:316`(apply_landlock_in_child)、`:369/:374`(add_landlock_rule 调用)、`:763`(add_landlock_rule)。

### T7 — seccomp 放行 node/python3（P1）

- **现象**：VM 上 `which node`/`which python3` 偶发 `错误的系统调用（核心已转储）`（SIGSYS），间歇（同会话后续 `node --version` 又正常）。
- **根因（已核源码）**：沙箱 seccomp 为**白名单 + 默认 KILL**（`crates/sandbox/src/lib.rs:535` `SECCOMP_ALLOWLIST` 121 项；`:671/:718` 未命中→`SECCOMP_RET_KILL_THREAD`）。源码注释 `lib.rs:514` 已自承"**白名单缺 libuv 事件循环与 v8 运行时 syscall**"——node（libuv+v8）与 python3 用到的部分 syscall 不在 121 项内 → 被 SIGSYS 杀。间歇是因部分执行路径恰好不触发缺失 syscall。
- **修复方案**：用 VM `strace -f -c` 抓 node/python3 实际 syscall 集，把白名单缺失的 syscall 补进 `SECCOMP_ALLOWLIST`（`:535`）。参考 `docs/seccomp-allowlist-v1.md`、`:667` 注释。可先用 `HEARTH_SECCOMP_MODE=errno`（`:714`）临时降为 ERRNO 便于 strace 定位被拦项。
- **验收**：VM 沙箱内 `node --version`、`python3 --version` 稳定不崩（连续 10 次）。
- **验证命令**：
  ```bash
  # VM 真机：hearth repl 内 bash → `for i in $(seq 1 10); do node --version; done` 无 SIGSYS
  # strace -f -e trace=%net,%desc hearth ... 对照 SECCOMP_ALLOWLIST 找缺失项
  ```
- **锚点（已钉）**：`crates/sandbox/src/lib.rs:535`(SECCOMP_ALLOWLIST)、`:659`(apply_seccomp_deny_list)、`:701`(遍历白名单)、`:714`(HEARTH_SECCOMP_MODE 回退)、`:514`(缺 libuv/v8 注释)。

### T8 — ambiguous_option 选项丢空格（P2）

- **现象**：规划澄清选项变成 `recalltherulesofchin`（应为 "recall the rules of china"），用户被迫手选。
- **根因（已核源码，锚点已钉）**：`crates/planner/src/lib.rs:157-185` 的 `extract_options` 在切分"或"两侧候选时，用 `.chars().filter(|c| !c.is_whitespace()).collect()`（`:165-169`）**删除所有空格而非 trim**；随后 `:173` 各取 `take(20)`。→ "recall the rules of china" 去空格得 "recalltherulesofchina"(21 字符) → 截 20 → "recalltherulesofchin"，与日志现象**完全一致**。
- **修复方案**：两侧候选改用 `.trim()` 保留词间空格（如需限长则按词截断而非删空格）；单测覆盖多词选项（同文件 `test_derive_gaps_ambiguous_option_blocking` 可扩展）。
- **验收**：含多词描述的澄清选项标签完整可读（如 "recall the rules of china"）。
- **验证命令**：`cargo test -p planner extract_options`（新增/扩展）。
- **锚点（已钉）**：`crates/planner/src/lib.rs:157-185`（extract_options），关键 `:165-169`。

### T9 — provider 死时工具短暂消失（P2）

- **现象**：provider 死亡时调度器报 `tool not found: write_file`/`tool not found: bash`，工具短暂"消失"。
- **根因（已核源码，锚点修正）**："tool not found" 字符串出自 `crates/tool-runtime/src/dispatcher.rs:129`（`self.tools.get(name).ok_or_else(|| anyhow!("tool not found: {}", name))`——注册表查找未命中）；`crates/agent-core/src/scheduler.rs:36` 仅是 `tracing::error!(... "tool dispatch failed")` 的日志点，并非根因。需执行窗口用 tracing 确认：provider 死亡时 dispatcher 的 `tools` 注册表为何会 miss（是否 loop 在 provider 异常路径重建/替换了 dispatcher，或 LLM 在 provider 错误下返回了畸形 tool_call 名）。
- **修复方案**：provider 异常**不得**清空/重置工具注册表；dispatch 失败时若根因是 provider 错误，应上抛 provider 错误而非伪装成 "tool not found"。
- **验收**：注入 provider 错误，工具调用仍报具体 provider/tool 错误而非 "not found"；会话不丢工具。
- **验证命令**：
  ```bash
  cargo test -p tool-runtime   # dispatcher 现有单测（dispatcher.rs:359 已断言 "tool not found"）
  # 真机/单测：provider 异常路径下 dispatcher.tools 不为空；provider 异常时 dispatch 仍命中已注册工具
  ```

### T10 — 出网白名单形同虚设（ctx.env 从未注入）（P0）

- **现象**：用户已在 shell 导出 `HEARTH_EGRESS_ALLOWLIST` 或（以为）在 config 配了出网白名单，但 `web_fetch` 仍一律"出网被拒"，任何域名都访问不了。
- **根因（已核源码，铁证）**：
  - Web 工具 `crates/tools-builtin/src/web.rs:93-97` 从 `ctx.env["HEARTH_EGRESS_ALLOWLIST"]` 读白名单；`egress_allowed`（`web.rs:35-45`）在空表时 `return false` → deny-by-default。
  - 但生产路径构造 `ToolContext` 时**全部为空 env**：`run_local.rs:159,240`、`repl.rs:250`、`session.rs:267` 均用 `..Default::default()`（或 `..self.ctx.clone()`，起点亦空）；`context.rs:20` 的 `Default` 实现 `env: HashMap::new()`。
  - **全代码库无任何位置把进程环境或 config 灌入 `ctx.env`**（已 grep 确认：无 `std::env::vars()` / `.env.insert` 注入；仅有的 `env::var` 命中是 `lib.rs:243` 的 `RUST_LOG` 过滤与 `main.rs:113`，均无关）。
  - `config.rs` 也**无 `egress_allowlist` 字段**（grep 确认）。即白名单既无 config 字段、env 又被 `.env = {}` 覆盖为空 → 用户无论怎么"加白名单"都无效。`web.rs` 唯一能读的名字 `HEARTH_EGRESS_ALLOWLIST` 从未被任何代码注入 `ctx.env`，结构性不可达。
  - **测试误导（假绿）**：`web.rs:173 test_web_fetch_denied_without_allowlist` 用 `env: HashMap::new()` 断言 deny——它只证明"空 env→拒绝"，从不证明"配了能放行"；而生产里"配了白名单的路径"因 `ctx.env` 永远空而**结构性不可达**。该测试给出错误的安全感。
- **修复方案**：
  1. `config.rs` 增加 `egress_allowlist: Vec<String>`（支持逗号字符串解析），`resolve()` 读取。
  2. 三处构造 `ToolContext`（`run_local.rs:159,240` / `repl.rs:250` / `session.rs:267`）注入 `ctx.env["HEARTH_EGRESS_ALLOWLIST"]` = `cfg.egress_allowlist` 与进程 env `HEARTH_EGRESS_ALLOWLIST` 的合并（config 优先，env 兜底）。
  3. Web 工具后缀匹配逻辑（`web.rs:35-45`）保持不动，已正确。
- **验收**：`config.toml` 配 `egress_allowlist = ["rust-lang.org"]` 后 `web_fetch https://rust-lang.org` 放行；未配则仍 deny；`docs.example.com` 随 `example.com` 放行。
- **验证命令**：
  ```bash
  cargo test -p tools-builtin test_egress_allowlist            # 已有，保持
  # 新增：构造含 allowlist 的 ToolContext，断言命中放行（端到端证明注入生效）
  # VM 真机：config.toml 配 egress_allowlist 后 web_fetch 真实出网
  ```

### T11 — 出网白名单缺"agent 提议 + 用户审批"流（P1）

- **现象**：访问未白名单域名时 Web 工具只能硬报错"出网被拒"，用户得手工一个一个改 env/config；用户明确希望"提交我审批即可，为啥我要手工一个一个设置"——即 agent 检测到需要某域，提请用户一次性审批落盘，而非人工逐条维护。
- **根因（已核源码）**：`web.rs:98-105` 命中 deny 直接 `Err(出网被拒...)`，无提请用户审批的出口。WP-0 `InteractionRequest`/`InteractionRequested` 已在代码落地（`api/src/lib.rs:155` 定义；`agent-core/src/loop.rs:1865` 有 `kind:"approval"` 的 blocking 用法范例），但白名单场景未接入——deny 路径与交互原语之间断链。
- **修复方案**：
  1. Web 工具（或其 deny 处置）在 `egress_allowed==false` 时，发起 `InteractionRequested { kind: "egress_allowlist_request", blocking: true, payload: {host, url, suggested: host} }`（**复用 WP-0，内核绝不 match kind**——`api/lib.rs:151-153` 约束）。
  2. Human OS 侧渲染为"是否将 `<host>` 加入出网白名单？"一键 approve/deny。
  3. approve（`resolved:true`）→ 将 `host` 写入持久化 `egress_allowlist`（config.toml）+ 并入当前会话内存白名单，`web_fetch` 重试成功；deny → 维持拒绝，不落盘。
  4. 可选：同一会话多次提议在 UI 聚合为一次批量审批，减少打扰。
- **验收**：未白名单域触发审批请求（事件流可见 `egress_allowlist_request`）；approve 后本次及后续同域访问放行且落盘持久化；deny 维持拒绝、不写盘。单测：mock 交互响应 approve → 断言 host 进入 allowlist 且 fetch 成功。
- **验证命令**：
  ```bash
  cargo test -p tools-builtin      # 扩展 web_fetch：注入 InteractionRequest approve → 命中放行
  cargo test -p agent-core         # 交互响应落盘到 egress_allowlist 的路径
  # VM 真机：REPL 内 web_fetch 未白名单域 → 出现审批提示 → approve 后出网
  ```

---

## 3. 门禁（执行窗口交付前必过）

| 项            | 标准                                     |
| ------------ | -------------------------------------- |
| 实现率          | ≥ 0.9（11/11 任务有可验证交付）                   |
| 🔴 数         | = 0（T1/T2/T3/T6/T7/T10 必须真修并贴证据）        |
| fmt / clippy | `cargo fmt --check` 0、`cargo clippy` 0 |
| 单测           | 新增/修改处补单测，全量 `cargo test` 通过           |
| 真机证据         | T1/T3/T4/T5/T6/T7/T10/T11 须附 VM 真机复测输出（非仅单测） |
| 成本对照         | T3/T4/T5 交付后附"同任务前后 API 调用数/费用"对比      |

---

## 4. 给执行窗口的备注

- **成本是头号诉求**：用户明确"V4 flash 烧不起、14 元测个寂寞"。T1/T2/T3/T4/T5 直接关联降本，优先排期。
- **Agnes 直连已被验证可作为临时通道**（config.toml 四行），但 T1 是正式解。
- **联网全废是高优先级 bug**：T10 证明 `ctx.env` 从未注入 → 所有 `web_fetch` 结构性全拒（与"加了白名单仍访问不了"完全吻合）。T10 修好后，T11 的"agent 提议 + 用户审批"流才能真正闭环，免去手工逐条维护。
- **核实锚点原则**：本任务书已为 T1–T11 钉死源码锚点，但执行窗口下笔前仍须先 `grep` 复核真实行号（源码会演进），禁止凭记忆改。
- **不要动冻结区**：本任务书不含 UI 冻结区（demo-v21j-frozen）与 genes-next.md（仍 PAUSED）。
