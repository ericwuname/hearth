# hearth-slim 手术包二·P0/P1 修复战报 v1.0（traecode → 执行窗）

- **日期**：2026-09-11　**依据**：《hearth-slim 手术包二·P0/P1 修复任务书 v1.0（执行窗→traecode）》（b49e8bf）+
  八卡真机验收报告（9033ac5，PC-1..4）
- **施工顺序**：修复1（S10，P0）→ 修复2（S8，P0）→ 修复3（S7，P1）——**按序执行，各自独立 commit**
- **结论先行**：**三处修复全部落地，先红后绿证据在案**；S9 de-scope 与口径断点已登记；
  真机复验（含 `kill -9 → resume`、`[retry]` 投影实测）属执行窗，本报告申报不代办。
- **环境**：本机 Windows MSVC（.133/.131 不可达——问题卡照旧，见第七节）

---

## 一、修复清单（三修复 · 三 commit）

| # | 级别 | 病灶（实测锚点） | commit | 一句话结果 |
|---|---|---|---|---|
| 1 | **P0** | S10 流式 tool_calls 参数丢失 → 工具全废 + 上游 400 | `5b7fdea` | 分片按 **index** 聚合；call_id 首片优先/缺则合成——工具恢复可用 |
| 2 | **P0** | S8 resume 只报状态不续跑（steps=0） | `a013c01` | current_goal 恢复 + steps 接着数 + `--budget` 追加 + 断点可发现 |
| 3 | **P1** | S7 连接拒绝/502 误判 unrecoverable 零重试 | `cad56c8` | 连接拒绝/超时/502/5xx **统一 Transient**（长退避） |

---

## 二、修复 1（P0）· S10 流式 tool_calls 拼装

**根因（比验收报告的一层更深）**：OpenAI 风格 SSE 的 `tool_calls` 分片**只有首片带 `id`**，
后续片只有 `index` + `function.arguments` 增量（`id` 为 `null`）。S10 的
`stream_model_call` 按 **`call_id`** find 聚合 —— 后续片（`call_id=""`）永远匹配不到首片，
**每一片都被当成一个新 call**：工具名空、参数被拆碎 → `missing 'query' argument`、
`tool not found:`、`missing field tool_call_id` 上游 400。

**修复**：
- `llm-gateway`：`StreamEvent::ToolCallDelta` 增 `index` 字段（分片聚合 key）；
- `llm-openai`（agnes/gemini/openai/deepseek 全部 OpenAI 兼容主通道）parser 透传 index；
  `llm-local`（vllm 透传 / ollama 整片 index=0）、`llm-cn`（透传）、`llm-replay`（整片 index=0）同步；
- `agent-core`：聚合 key 改 **index**；`call_id` 取首片非空 id，聚合结束仍空则合成
  `call-{index}-{seq}` —— 同时根治 tool 结果消息回填的 `missing field tool_call_id` 400。

**先红后绿（3 + 1 测试）**：旧实现下 3 个 agent-core 测试红，失败形态**逐字复现**实测病灶：

```
ToolCall { call_id: "call_abc123", name: "web_search", args: String("") },
ToolCall { call_id: "",          name: "",           args: Object{"query": "rust tokio"} }
```

①单工具分片拼装（name/args/call_id 完整）②并行工具调用（index 0/1 交错分片各自聚合）
③首片无 id → 合成 call_id（配对不缺）；另加 llm-openai parser 层分片行测试（index 透传）。
非流式对照不回退（`chat()` 路径测试全绿）。

---

## 三、修复 2（P0）· S8 resume 续跑

**如实归因（不夸大）**：实测"输出总结即退出 + steps=0"与 **PC-1 级联同源** ——
恢复后首个模型调用被上游 400 打死 → provider 错误收尾（steps=0）→ S14 总结打印后退出。
本 commit 修的是**与 PC-1 无关的四处真实缺陷**：

| # | 缺陷 | 修复 |
|---|---|---|
| 1 | 目标漂移：断点只存 `original_goal`，resume 时 `current_goal` 被字面 `"continue"` 覆盖 | `run_state_snapshot` 增 `current_goal` 落盘 + `restore_run_state` 回灌；CLI goal 缺省时用断点目标（显式传参时尊重用户） |
| 2 | steps 不接着数：`continue_turn` 清零（REPL 每轮重计语义） | `AgentLoop::set_resume_keep_steps` —— 下一次 `run()` 消费后接续旧 `steps_used` 并自动复位 |
| 3 | 预算不可追加：`--budget` 报 unexpected argument | `Resume` 命令增 `--budget N`（缺省 = 默认档）；总预算 = 断点已用 + N（`resume_budget` 纯函数，饱和加法防溢出回绕成小预算） |
| 4 | 断点不可发现：只落 config 区 | checkpoint 同点写 workspace 镜像 `<cwd>/.hearth/runs/<id>.json`；resume 投影真实路径（主存储 + 镜像） |

**"重新进入消息循环继续执行"** 由既有 `run_local_continue` 承载（restore_history +
restore_run_state 后 `run_take` → 消息循环）——**未新建第二套循环**（防机制分叉）。

**先红后绿**：`test_pc2_resume_keeps_steps_and_continues` 旧实现下红（期望
`steps_used == 3 + 本轮`，实得 `1` = 清零重数）；修复后绿，同测断言 current_goal 恢复
+ resume 实际执行（`report.steps ≥ 1`）。另加 `test_pc2_resume_budget_adds_extra`
（追加/零已用/零追加/饱和）与 session_store 断点镜像落盘断言。

---

## 四、修复 3（P1）· S7 错误分类统一

**根因（比验收报告的一层更深）**：`classify_anyhow` 的**启发式表里这些早已是 Transient**
（`connection refused`/5xx 都在表内），但 `llm-openai` provider 层把它们包成
`LlmError::Fatal` —— **结构化路径 downcast 命中后短路了整张表**。两套口径就此分叉：
超时走 `else` 分支侥幸正确（Transient），连接拒绝/502 走 `if` 分支判"端点死"（Fatal）。

**修复**（分类口径集中一处、可审计、可单测）：
- `llm-openai` 新增纯函数 `classify_http_status`（429/5xx 含 502/503/504 → Transient；
  400/401/403/模型不存在 → Param 不重试）与 `classify_transport_error`（连接拒绝/不可达/
  DNS/超时/TLS 统一 Transient）；两个 call site 改走它们；删除"通道不可达（端点死）"Fatal 口径；
- `llm-gateway` 启发式表补齐全部 5xx 码（`502/503/504` + bad gateway/service unavailable/
  gateway timeout）—— 旧规则只认 `"500"`/`"upstream"`，非结构化通道（如 `llm-cn` 的
  anyhow 文本）返回 502 会落到 Fatal 兜底（跨通道一致性缺口）；
- `codex-cli` 失败提示同步：provider 瞬时故障提示"已长退避重试、窗口耗尽会暂停可 resume"。

**保留项（显式申报）**：**连续 2 次 read-body 畸形流**仍为 Fatal（同一畸形流重试不自愈，
T3 既有设计）—— 这是唯一有意保留的 Fatal 传输类，未纳入本次统一。

**先红后绿（4 测试）**：
- `test_pc3_real_502_response_is_transient`（本地 TCP 回真实 502）—— 修复前红，错误**逐字复现**
  实测锚点 `provider unrecoverable error: HTTP 502: ...`；修复后绿；
- `test_pc3_real_connection_refused_is_transient`（真实连接拒绝）—— Transient，且错误不得再含
  "端点死/unrecoverable"；
- `test_pc3_http_status_classification` / `test_pc3_transport_error_uniformly_transient` ——
  纯函数口径锁定；llm-gateway `test_error_classification_table` 增 502/503/504 用例。

**回归更新（如实）**：`agent-core::test_provider_failure_run_terminal_state_failed` 的 mock
从 `Fatal("HTTP 503 down")` 改为 `Fatal("content policy violation")` —— 503 已（正确地）
是 Transient，该测试要的非 transient 路径需用真正不可恢复的类别。

---

## 五、门禁

- 每修复：`cargo fmt --all` 干净 → `cargo clippy --all-targets` 零警告 → 新测试先红后绿 → 独立 commit。
- 收官整包（`cargo test --workspace --no-fail-fast`，本机 MSVC）：
  - **绿**：`agent-core` **142/142**、`codex-cli` **34/34**、`llm-openai` **14/14**、
    `llm-gateway` / `llm-local` / `llm-cn` / `llm-replay` 全绿、`service`（含 integration **9/9**）、
    `planner` / `agent-types` / `agent-runtime` / `api` / `bridge` / `tool-runtime` /
    `project-sync` / `observer` / `memory` / `nervous-system` 等全绿。
  - **红（3 个 target，全部环境/既有，与本批零交集）**：`tools-builtin` 11 项、
    `sandbox` 1 项、`project-xray` 1 项 —— 明细见第八节第 2 条。
  - **对比上一批**：失败 target 由 4 → 3（`service` integration 全绿），**无新增失败**。

---

## 六、红线自查

1. **沙箱 crate 语义零触碰**：`crates/sandbox` 本批零改动；
2. **`a_arm_act_tally` 9 处原位**：未触碰；
3. **key 永不入 git**：三 commit 仅含 `.rs` 源码，零凭据；
4. **S5 已拆机制禁止复活**：未新建相位机/强制 decompose；resume 复用既有消息循环
   （未建第二套循环）。

---

## 七、De-scope 登记与口径断点（按任务书）

### 7.1 S9 provider 降级链 —— **de-scope（用户拍板暂不做）**
- `zhipu` 未注册问题**随卡关闭**；
- **已工作的部分保留不删**：`[fallback]` 切换 + 通道标注投影（实测工作）；本批**未触碰**
  `fallback.rs` —— 单通道主路径零影响；
- 配置面 `providers` 数组维持现状；重新启用条件：另行拍板。
- **一致性说明**：PC-3 修复后连接拒绝/502 为 Transient → 按 S9 既有边界设计
  （transient 交回 S7 长退避，仅 fatal/param 切通道）**不会**触发通道切换 —— 与
  "防瞬时抖动污染同尺对照"的既有裁决一致（无需改代码）。

### 7.2 口径断点登记（模型层切换）
- **agnes-2.5-flash → agnes-3.0-flash**（用户拍板 2026-09-11）：换模型 = 换模型层，
  **此前全部基准数据（0.2.25-base / 0.2.26 术前 / slim 复测）与新数据不可比** ——
  凡 3.0-flash 跑出的数据必须标注"3.0"，**禁与 2.5 时代数据混标对照**；
- 本机 config 已由执行窗切换并实测（1 步 1.2s 直答 ✓）；`.131/.133` config 待 VM 可达时同步（执行窗侧）。

---

## 八、申报与待裁定（不静默）

1. **环境问题卡（照旧）**：`.133/.131` 不可达（ssh 免密与 paramiko 空密码均失败）——
   本批按既有裁定走本机 MSVC；真机复验留执行窗。
2. **工作区既有红（与本批零交集）**：`tools-builtin` 11 项（POSIX bash + Unix 绝对路径语义，
   含 1 项负载抖动）、`sandbox::test_noop_sandbox_echo`（需 POSIX echo）、
   `project-xray::real_workspace_wiring_all_green`（读工作区既存删除项 `docs/xray/wiring-v13.toml`）
   —— 未"修绿"（防把环境/既有问题伪装成代码问题）。
3. **⚠ 新发现的模型层风险（建议顶层裁定，本批未擅改）**：`agnes-3.0-flash` 的
   **thinking 与正文共享输出预算**（本仓既有文档《agnes-3.0-flash 评估报告 v1.0》
   实测：`max_tokens=2048` → `finish=length` 且**正文 0 字**；该报告对 EMBER 的硬性约束
   是 **max_tokens=65536**）。而 Hearth 当前 `max_tokens` 硬编码为 **8192**（`agent-core` 2 处 +
   `planner` 1 处，无 env 覆盖）。**影响**：3.0-flash 在重推理轮可能把预算耗尽 → 空正文轮
   （R7-5 A-1 会重试一次，但会降级体验）。**建议**：单独立卡把 `max_tokens` env 化
   （`HEARTH_MAX_TOKENS`，thinking 模型默认拉高），**不在本修复单范围内擅自改**
   （会动 token 成本口径，执行窗正在做分账统计）。
4. **本批未做（属执行窗）**：真机复验（PC-1 流式工具任务无 400 / PC-2 `kill -9 → resume`
   续到交付 / PC-3 三类不可达均出 `[retry]` 且 30s 起）。

---

## 九、执行窗复验清单（对照验收日志）

| # | 复验项 | 判据 |
|---|---|---|
| 1 | **PC-1** 真机流式工具任务 | 含工具（写文件 + `web_search`）任务工具调用成功、**无 400**；与 `--no-stream`（非流式对照）结果一致 |
| 2 | **PC-2** `kill -9 → resume` | 任务跑到一半 `taskkill /F` → `hearth resume <id>` → **从断点续跑到交付**（产物完整、历史连续、**steps 接着数**）；`--budget N` 可追加；`.hearth/runs/<id>.json` 可发现 |
| 3 | **PC-3** 三类不可达 | 拒绝 / 超时 / 502 均出 `[retry]` 且**退避 30s 起**（不再零重试暂停） |
| 4 | 回归 | 非流式工具路径不回退（9-10 时代对照仍在） |
| 5 | 后续（复验过后） | S12 判别实验（魂斗罗质量版重跑 → 用户评分）+ S13 真机搜索取证 + S11 目测（Linux）+ 吴涛 3 真实任务 |
