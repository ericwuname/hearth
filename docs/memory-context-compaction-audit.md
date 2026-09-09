# Compaction Trigger Audit — Node 02（P2-MEMORY-CONTEXT-01）

日期：2026-08-30　基线：v0.2.15　证据等级：全部锚点实测（confirmed）

## 1. 触发规则（真实控制流）

```text
build_messages（loop.rs:2035-2039，每次 LLM 调用构建前——唯一生产调用点）
  → ctx_mgr.maybe_compact()                        （context.rs:189-250）
     → estimate_chars() ≥ 32_000 ?                 （context.rs:169/190）
        否 → return false
        是 → 保留最近 COMPACT_KEEP_TURNS=2 轮        （context.rs:193-197）
             旧轮 drain（**仅 state.history**，:201）
             archive_compacted_turns 先落盘（best-effort，:205-207）
             summarize_turn 逐轮规则式摘要（不调 LLM，:209-211）
             摘要+剩余轮合并回 history（:213-224）
             首条摘要注入归档检索提示（:228-243）
```

## 2. 四问直答

| 问 | 答 |
|---|---|
| 什么时候触发 | 每次 build_messages 前（即每步 Plan/Act 的 LLM 调用前） |
| 按什么单位 | **字符**——但口径是 `format!("{:?}", content).len()` = Debug 包装 + UTF-8 **字节**（context.rs:179），非 naive chars、非 token、非 provider usage |
| 估算 token？ | 无。注释自承 "≈32k chars ≈ 8k tokens"（context.rs:168）；Node 01 实测：est 口径 32k ≈ **9,009 tokens**（combined workload），且实际只代表 22,940 naive chars |
| provider usage？ | 完全未接入——compaction 判定与 API 返回的 usage.prompt_tokens 零关联 |

## 3. 阈值是否 provider-aware？——**否（confirmed）**

- `COMPACT_CHAR_THRESHOLD = 32_000` 是编译期常量（context.rs:169），无 config/env 覆盖、无 provider 查询。
- provider 能力表：`llm-cn/src/lib.rs:412` HunyuanProvider `max_context_tokens: Some(32_000)` 写死；**全表无 Agnes 条目**（Agnes 走 config.toml 通用 openai 兼容通道，capabilities `max_context_tokens` 未注入）。DeepSeek/local 各自的窗口信息同样未进入 compaction 判定。
- `/v1/models` 实测（Agnes）：模型列表**不含 context_length 字段**——自动发现窗口不可行，批-2 的"已知表 + 手工覆盖 + 比例"最小切片路线被证实为唯一可行路径。
- 32,000 定性：**历史遗留值**（WS4 时代 "≈8k tokens" 假设的产物），与当前 provider capability 无推导关系。对照 Agnes 512K：触发点 ≈ 容量 1.8%。

## 4.（v1.1 必答）RC40：context_fill_pct 语义错位单列

**锚点**：`loop.rs:1338-1346`（update_body_state）：

```rust
let fill_pct = if total_chars >= COMPACT_CHAR_THRESHOLD { 100 } else {
    (total_chars / COMPACT_CHAR_THRESHOLD * 100) as u64 };
... "context_fill_pct": fill_pct,   // 注入 LLM 可见 scratch["body"]
```

- **真实含义**：`(history+system 的 est 口径字节) / 32,000` = **距压缩阈值的接近程度**（compact pressure）。
- **名义含义**："上下文填充率"——模型（与用户）会把它读成"上下文窗口用了多少"。用户实测 83%→65% 回落当场质疑（压缩后自然回落），即为语义错位的野外证据。
- **误导性**：模型据"填充率 46%"判断窗口还很空，实际窗口用量不到 2%；反之若真按 512k 用量计，46% 的"填充率"应是健康的。**疲劳判断被系统性误导**。
- **改阈值治不了它**：阈值改多大，该字段的分母永远是 32k 常量（或新常量），语义仍是"距阈值距离"而非"窗口占用"。
- **处置建议（Node 11/12 执行）**：改名 `compact_pressure_pct` + 语义注释（距压缩阈值 %）——S-1 连带项：`tools-builtin/src/status.rs:91`（示例 JSON）与 `status.rs:101`（断言 `assert_eq!(v2["context_fill_pct"], 62)`）必须同步改，否则单测与真机 telemetry 字段名分裂。
- 可见性归类（S-8）：改名后**保留 LLM-visible**——"距压缩阈值 % "对模型判断自身疲劳有真实价值（这正是 T3 油箱表的本意），但必须用不撒谎的名字。

## 5. 历史 trimming / summary / archive / restore

- **trimming**：只 drain `state.history`（context.rs:201）——goal/TaskGraph/acceptance_*/scratch 均在 state 层，**不在压缩路径**（S-3 前提确认）。
- **summary**：`summarize_turn`（:312-378）纯规则模板——goal（首条 user，60 字截断）+ 工具清单 + 写盘文件；**Assistant 全文与 tool_result 100% 丢弃**（RC5/RC37 机制本体确认）。
- **archive**：`archive_compacted_turns`（:258-274）先于折叠落盘 `archive/<session_id>.jsonl`（G3-03 会话隔离；`HEARTH_ARCHIVE_FILE` env 可覆盖；session 空回落共享文件）。**best-effort：失败仅 warn 不阻塞**——S-2 警告成立："代码里有 archive 调用"≠"Fact 已持久化"，Node 04 须实测落盘。
- **restore**：无自动 restore——恢复通道 = 摘要头部注入的 grep 检索提示（:228-243，模型自主 bash grep）。检索提示存在 ≠ 检索成功（S-6 B 档口径）。

## 6.（Node 02 执行中重大发现）单 run 下压缩是死代码（confirmed）

**控制流实测**：

- `record_turn` 生产调用点**仅一处**：`loop.rs:4379`——run() 启动时 `Turn::new(0)`。
- `record_tool_exchange`（loop.rs:2307，每个 Act 步后调用 :3464）把 assistant/tool
  消息 push 进 `history.last_mut()`——**永远是同一个 Turn 0**；
  `add_user_message`（context.rs:133）同样只进 last turn。
- → 单个 `hearth chat` run 全程 `history.len() == 1` → `maybe_compact` 的
  `keep_from = len-2 = 0` → `return false`（context.rs:198-199 "轮数太少"）。
  **无论 est_chars 多大，压缩从不触发。**

**影响面**：
1. `hearth chat` 主路径（单 run）**没有压缩**——历史在内存中无限增长，
   实际约束 prompt 的是另一层：`MAX_HISTORY_MSGS = 40`（loop.rs:2200，
   build_messages 切片——**静默**丢弃最旧消息，无标记、无归档；消息留在
   state.history 未销毁，但 LLM 永远看不到）。"失忆"在单 run 的真凶是它，
   不是 32k 压缩。
2. REPL/多轮会话（每次用户输入新 run() → 新 Turn）下压缩**能**触发——
   RC40 野外证据（83%→65% 回落）即来自 REPL 手工测试，与本题自洽。
3. `context_fill_pct`（分母 32k）在单 run 随 history 无限增长到 100 并停住——
   "疲劳信号"完全失真（est 口径 + 死压缩 + 无限增长三重叠加）。

**真机证据**：Node 05 试跑 A1（8×1500 行 seq 输出，历史 est ≈ 44k > 32k）：
`grep -c "context compacted" = 0`，任务 completed 17 步、RESULT.txt 8 数字
全对——压缩从未触发，40 消息切片承担了全部窗口约束。

**对总包的影响（重组 Node 顺序的理由）**：
- Q2 答案修正：单 run 的连续性断裂主因 = **40 消息静默切片**（无标记无归档）
  + 压缩死代码使"归档找回"通道在该路径不存在；REPL 则为压缩触发。
- Node 05/06 的 A/B 前提失效（两组都不会触发压缩）→ **最小修复（Node 12 的
  turn 粒度对齐 + env 阈值覆盖）必须前置**，A/B 在修复后的代码上执行才有意义。
  修复属 Memory 层（turn = 一次交换的语义对齐），不触 STOP-2/6（不动
  TaskGraph/TaskGoal/Completion/Terminal 语义）。
- Node 12 施工清单据此更新：①turn 粒度对齐（每次工具交换 = 新 Turn，
  先红后绿）②provider-aware 阈值（批-2 最小切片 I-6）③estimate_chars
  诚实计数（Debug+bytes → naive chars，Node 01 系数 2.55 chars/token）
  ④RC40 改名 compact_pressure_pct（S-1 连带 status.rs）⑤env 阈值覆盖（已落）。

**红样本锚点（修复前）**：`grep -c "context compacted" ~/fa/ab_A1.log = 0`
（44k est 历史，任务 completed——死代码实锤）。

---

## 7.（Node 10）静默截断清单 v1（第一阶段只分类，不改动）

| # | 常量 | 位置（v0.2.15） | 截断对象 | 标记？ | 分类 | 备注 |
|---|---|---|---|---|---|---|
| 1 | 200 | codex-cli/render.rs:82 | 工具结果**显示**预览 | 无（显示层） | SAFE | 仅投影；事实在 state/history 完整 |
| 2 | 200 | loop.rs:3672 | RAG 检索 query 拼接 | n/a | SAFE | 检索启发式，非事实存储 |
| 3 | 60 | context.rs:322 | 压缩摘要 goal_short | 摘要语义即缩略 | SAFE | run goal 全文在 state.goal（INV-M01 fixture 已锁） |
| 4 | 8000 | web.rs:17/126 | 网页正文入 tool result | **无截断标记**（note 仅未验证警告） | NEEDS_REVIEW | 补"[N chars truncated]"标记即可（最小改） |
| 5 | 6000 | constitution.rs:20 | constitution 注入 system | 无标记 | NEEDS_REVIEW | 规则完整性问题，非 Fact 路径 |
| 6 | 6000（4000+1500） | loop.rs:4892 | bash 工具输出入 history | **有标记** ✓ | NEEDS_REVIEW | 注意：archive 存的是**截后**文本 → B 档恢复上限受此钳制（登记不修） |
| 7 | 4096 | code-index:351 + loop 测试 mock | max_context_tokens 能力声明 | n/a | SAFE | **非截断路径**（砺批-5 猜想证实：常量名自我描述不可信，grep 后确认无消费） |
| 8 | 40 msgs | loop.rs:2200 MAX_HISTORY_MSGS | prompt 历史切片 | **无标记、无归档** | **FACT_RISK** | 单 run 真正的窗口约束；被切消息仍在 state.history（未销毁）但 LLM 永不可见；与压缩/归档零联动 |
| 9 | — | context.rs:179 estimate_chars | 测量仪器虚增（Debug+bytes） | n/a | **FACT_RISK** | Node 01 定量 1.39-2.49×；提前触发压缩（Node 12 修） |
| 10 | — | loop.rs:1346 RC40 | context_fill_pct 语义错位 | n/a | **FACT_RISK** | Node 12 改名 compact_pressure_pct（S-1 连带 status.rs:91/101） |

**结论**：FACT_RISK 三处（#8/9/10）全部进入 Node 12 施工清单；#4/#5 补标记属最小改（Node 12 顺手）；#6 的 archive-存截后文本问题只登记（修复=归档存原文，涉及 archive schema 变更，超本轮最小改动原则）。
