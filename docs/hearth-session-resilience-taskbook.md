# Hearth 会话韧性 + 长任务 + 对标 Codex 补充任务书

> 补 `hearth-harness-hardening-v016-taskbook.md`。
> 依据：手工测试 v0.1.5.txt（build 33f5b7b）+ codex CLI 最新架构（2026-08 联网核实）。
> 用户三痛点：① 对话窗口一废就全废 ② 长任务做不了 ③ 对标 codex CLI。
> v0.1.6 已修 R8（panic）/R9（provider 重试）/R10（澄清选项）——但那是"止血"，本文补的是"让对话窗口死不掉 + 能扛长任务"这一层。

---

## 一、三个痛点 → 手工日志铁证 → 根因定位

| 用户痛点 | 日志铁证 | 根因 |
|---|---|---|
| **① 窗口一废就全废** | `context.rs:158` 字节切片 panic → 整个 REPL 进程死，前面 20 轮对话全丢，只能重开 | 会话状态在内存，无持久化；panic 无隔离 |
| **② 长任务做不了** | 8000字长文/5000字武侠/3000字科幻 → 全部 `read body`/`empty response`；甘特图写了 SVG 方案又 replan 回去用 matplotlib，最后 give_up | 无 compaction + 无任务状态持久化 + 输出不分块 |
| **③ 历史要重说** | 问"我们刚刚做了哪些事" → agent 用 `glob **/*` 找文件，而不是读对话历史 | REPL 每轮 `create_session`，历史不喂入上下文 |

---

## 二、联网核实的 codex CLI 现状（2026-08）

codex CLI 已全面开源（`github.com/openai/codex`，Apache-2.0），核心 harness 架构：

| codex 能力 | 实现 | hearth 现状 | 差距 |
|---|---|---|---|
| **Thread 持久化** | SQLite/JSONL，进程重启不丢，resume/fork/archive/rollback | 内存态，`resume` 命令有但 REPL 没用 | 🔴 用户痛点①③ |
| **Turn/Item 三层** | Thread→Turn→Item 原子事件，稳定 API | 有 EnvelopedEvent 但无持久化 | 🟡 |
| **Prompt Caching** | Responses API 缓存，二次方→线性 | 无 | 🟡 长任务成本 |
| **Compaction** | 超阈值压缩早期上下文，模型参与总结 | 无（v0.2 WS4 排了但未做） | 🔴 用户痛点② |
| **崩溃隔离** | turn 失败不影响 thread，可重连 | panic 直接杀进程 | 🔴 用户痛点① |
| **分块长输出** | 流式 delta 累积 + 多 tool_call | 单轮超长输出截断 | 🟡 |

**核心结论**：用户三个痛点，全部对应 codex 的 **Thread 持久化 + Compaction + 崩溃隔离** 三件套。这不是"能力对标"，是"会话能不能活下来"的基础设施。

---

## 三、新增工作流（在 v0.1.6 之后，v0.2 之前）

### WS-X1（🔴 最高优先）：会话持久化 + 崩溃隔离 —— 治痛点①③

**这是用户最痛、最该先做的。** 手工测试里 `context.rs` panic 直接杀掉整个 REPL，20 轮对话灰飞烟灭。R8 修了 panic 本身，但没修"panic 后会话还能不能活"。

| 项 | 内容 |
|---|---|
| **X1-1 会话落盘** | REPL/chat 每轮结束，把 transcript 追加落盘 JSONL（`~/.config/hearth/sessions/<session_id>.jsonl`）。已有 `transcript.rs` 逐步记录，补"持久化 + 启动加载" |
| **X1-2 REPL 复用 Thread** | REPL **不再每轮 create_session**——单 session 复用，历史 Turn 喂入上下文（对齐 codex 的 Thread 模型）。这就是"我们刚刚做了哪些事"能答对的根 |
| **X1-3 崩溃隔离** | agent 循环包一层 `catch_unwind`/`JoinHandle` 隔离——单轮 panic 不杀进程，转成"本轮失败 + 可继续"；panic 前后 transcript 已落盘，重开 `resume` 无缝续接 |
| **X1-4 resume 打通** | `resume <session_id>` 从落盘 JSONL 重建上下文（历史消息 + 已写文件清单），让"窗口废了重开"变成"窗口废了 resume 接着干" |

**验收**：
- REPL 连续对话 5 轮，"我们刚才做了什么"能引用前 4 轮（而非 glob 文件系统）
- 人工触发一次 panic（如长中文 goal），进程不死，`resume` 后能引用 panic 前的对话
- 重启进程后 `hearth resume` 恢复同一会话，历史不丢

---

### WS-X2（🔴）：轻量 Compaction —— 治痛点②（长任务）

v0.2 WS4 排了 compaction，但**手工测试证明它是当前硬伤**（8000字长文全挂），应提前。

| 项 | 内容 |
|---|---|
| **X2-1 触发阈值** | 上下文 token 达上限 ~70% 时触发（对齐 v0.2 任务书 WS4 定义） |
| **X2-2 摘要策略** | 旧轮次摘要保关键信息：架构决策 / 已写文件清单 / 验证状态 / 开放 TODO。**用一次 LLM 调用做摘要**（不是纯截断） |
| **X2-3 长输出分块** | write_file 单次超长 content 自动拆多次（手工日志里 agent 已经在手动"分 4 段追加 game.js"，说明这是刚需——harness 应该帮它做，而不是让它自己拆） |

**验收**：8000 字长文在 deepseek 下稳定完成（分块写出 + compaction 不丢已写内容）；不再 `read body`/`empty response` 致命失败（配合 R9 重试放宽）。

---

### WS-X3（🟡）：Prompt Caching + 输出 token 对齐 —— 对标 codex 降本

codex 用 Responses API prompt caching 把二次方变线性。hearth 用 OpenAI 兼容 `/chat/completions`，deepseek 支持 context caching（前缀缓存）。**先做最廉价的**：

| 项 | 内容 |
|---|---|
| **X3-1 max_tokens 对齐** | planner `max_tokens=2048` → `8192`（v0.1.6 R9-B3 已列，确认落地）；主循环对齐 |
| **X3-2 上下文前缀稳定** | 历史消息顺序确定性（不随机打乱），让 deepseek 前缀缓存命中。这是"cost 线性化"的前提 |

**验收**：长会话 token 成本不再随轮数二次方增长（对比 compaction 前后单轮输入 token）。

---

## 四、顺序与依赖

```
WS-X1（会话持久化+崩溃隔离）→ 先做，治最痛的"窗口一废全废"
WS-X2（compaction+分块）    → 紧随，治"长任务做不了"
WS-X3（caching+token）      → 收尾，降本（可后置）
```

- **X1 是 X2/X3 的前提**：没有会话持久化，compaction 压缩完的摘要也无处安放；没有崩溃隔离，长任务中途 panic 一样全丢。
- 与 v0.1.6 关系：R8/R9/R10 先落地（止血），X1-X3 在其上补（韧性）。

---

## 五、验收红线

| 编号 | 判据 | 阻断级 |
|---|---|---|
| VX-1 | 连续对话 5 轮，"刚才做了什么"引用前文而非 glob | 🔴 |
| VX-2 | 触发 panic 后进程不死 + resume 恢复历史 | 🔴 |
| VX-3 | 8000 字长文稳定完成（分块 + compaction） | 🔴 |
| VX-4 | REPL 不再每轮 create_session | 🔴 |
| VX-5 | cargo test 全绿 + 无新增 panic | 🔴 |

---

## 六、为什么这是"对标 codex"而非"抄 codex"

codex 的 Thread/Turn/Item + SQLite + prompt caching 是**会话基础设施**，不是"能力镀金"。用户三个痛点的本质是：**hearth 的会话是"一次性内存态"，codex 的会话是"可恢复的持久态"**。

hearth 不需要照搬 codex 的 60+ crate / App Server / JSON-RPC 协议——那确实是平台化镀金（v0.1.x 阶段不需要）。但**Thread 持久化 + Compaction + 崩溃隔离**这三件是"单人稳定使用"的地基，没有它们，"简单对话没问题、一复杂就废"的状态就永远改不掉。

*"对标 top3"最该先对的，不是多智能体、不是 MCP、不是 App Server——是"对话窗口死不掉、长任务扛得住"。这三个，手工日志已经用 20 轮丢数据 + 8000 字全挂的代价，证明了它们就是当前最痛的洞。*
