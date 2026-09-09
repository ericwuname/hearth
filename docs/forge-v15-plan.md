# codex-rust v15 — 择脑 + 真防线 + 价值（定版 v2-final）

> 定位：v14 淬火证明了 agent 的**身体**在暴风雨里不会崩（应力场 24/24）。v15 回答身体之外的三问——**换最好的脑子跑两遍还能 90%+ 吗（择脑）？已经做对过的事还会不会做对（真回放防线）？agent 比人快多少（价值）？**

---

## §0 定版评审修订（R1–R6）

对 v1 草案做源码级评审后落定的六项修订。**每项都先核实了真实源码/数据，不是纸面推演。**

| # | 级别 | 问题 | 裁定 |
|---|---|---|---|
| **R1** | 🔴 | **线 B 回放语义自相矛盾**：§2 写「回放用已有消息、不需要模型推理」，但步骤写「读 fixture → POST → 校对 outcome」。POST 到现有服务 = 真实 provider 重新推理，**非确定性**，测的是模型运气不是回归 —— 这样的"CI 第五门"是假门。 | 改为**确定性回放**：新增 `ReplayProvider`（`LlmProvider` 实现，注册名 `replay`），按序吐 fixture 录制的 `ChatResponse`，**LLM 被钉死、工具真实执行**。任何 harness 退化（工具执行/状态机/预算/审批）直接暴露。可行性已核实（见 §1.1）。 |
| **R2** | 🔴 | **线 C 依赖真人计时**，S3 标"0.5 天"但 agent 无法代劳，会卡死交付。 | 拆分：agent 侧耗时**矩阵 `wall_s` 已有，零成本**；人侧降级为**表格模板 + 计时规程**，交由用户择期填写，**不进 v15 红线**。 |
| **R3** | 🟡 | **能力边界白皮书证据不足**：现有证据只是"zhipu 挂 + deepseek v13 单遍也挂"，即"两个弱模型都挂"，不足以断言"题目/harness 无罪"。 | 加 **S1b gemini 天花板对照**（`gemini-3.6-flash`，T14/T19/T09 ×2 = 6 次，几分钟）。**gemini 过 → 确证模型能力边界，白皮书成立；gemini 也挂 → 说明夹具或工具链有坑，白皮书作废、转去查 harness**。这是白皮书能否立住的前提，不可省。 |
| **R4** | 🟡 | **红线漏三门**：§5 只留 wiring，v14 是四门（fmt/clippy/test/wiring）。 | 补齐四门为 🔴，并保留应力场回归重跑。 |
| **R5** | 🟡 | **v14 欠债③ ST7 判据拆分未进计划**（`ok` 把"任务完成"和"隔离不泄漏"绑一起，导致 1/3 的误报）。 | 塞进 S2b，拆为 `isolation_ok` / `task_ok` 两个独立字段。 |
| **R6** | 🔵 | **T09 未列入观察名单**：§1 表只提 T14/T19，但 v14 中 T09 是 FLAKY 50%。 | T09 纳入天花板对照与能力边界表。 |

### §1.1 R1 可行性核实（源码锚点）

| 事实 | 锚点 | 结论 |
|---|---|---|
| fixture 是完整事件流，含 `tool_call{name,args,call_id}` / `tool_result` / `token` | `bench/replay/fixtures/*.json` → `history.messages[]` | 足以重建每一步 LLM 输出 ✅ |
| provider 是 trait，可注入 | `crates/llm-gateway/src/provider.rs:11` `pub trait LlmProvider` | 可实现 stub ✅ |
| registry 支持按名注册 | `crates/llm-gateway/src/registry.rs:48` `pub fn register` | 可注册 `replay` ✅ |
| **agent-core 只调 `.chat()`（非流式）** | `crates/agent-core/src/loop.rs:1037` | stub 只需实现一个方法，`stream()` 可最小化 ✅ |
| 响应结构 | `ChatResponse { content, tool_calls, finish_reason, usage }`（types.rs:17） | 直接从 fixture 映射 ✅ |

---

## §2 起点状态（v14 有条件过闸）

| 维度 | 数值 |
|---|---|
| wiring 防火墙 | 8/8 PASS + 自证能变红 ✅ |
| 毒债修复 | sub_budget 不再砍半，T02/T13/T18 全稳 ✅ |
| 应力场 | 24/24 零 panic + 审批门/并发/隔离/宪法全实证 ✅ |
| zhipu 20×2 | 主表 77.5%，预算修正口径 **87.5%**（<90%） |
| 真实缺口 | **T14(derive宏) 0/2 + T19(泛型合并) 0/2 + T09 FLAKY 50%** |
| deepseek | v13 单遍 20×1 跑了 90%，**未跑稳定性** |
| 回放素材 | 31 条 PASS session 归档 ✅（全部 zhipu） |
| replay.py | ❌ 未实现（v14 欠债①） |
| 机器时间基线 | zhipu 40 次 = 47.5 min，中位 57.3s／单次最长 355.1s |

---

## §3 v15 三线

### 线 A：择脑 — deepseek 20×2 稳定性 + 天花板对照

| 步骤 | 产出 |
|---|---|
| S1 deepseek 20×2 稳定性（复用 runner + 已统一的 20 步 meta 口径） | `matrix-v15-deepseek.jsonl` |
| **S1b gemini 天花板对照（T14/T19/T09 ×2）** | `matrix-v15-gemini-ceiling.jsonl` |
| deepseek ≥90% → 定为 standard brain（默认 provider） | 写入 `forge-report-v15.md` |
| deepseek <90% **且** gemini 也挂同样题 → 出能力边界白皮书 | `docs/capability-boundaries.md` |
| deepseek <90% **但** gemini 过 → **不写白皮书**，转查夹具/工具链 | v16 立项 |
| zhipu 降级为 "L1–L3 可用，L4+ 建议 deepseek" | provider 能力分级表 |

> **口径声明**：v15 全程使用统一的 `max_steps=20`（v14 S4b 后已补齐全部 L4/L5 的 meta.json）。与 v14 主表 77.5%（混合 15/20 步口径）**不可直接对比**，应对比 v14 修正口径 87.5%。

### 线 B：真回放防线 — 补 v14 欠债①（31 条素材已备）

| 步骤 | 产出 |
|---|---|
| `crates/llm-replay/`（或 llm-gateway 内模块）实现 `ReplayProvider` | 确定性 stub provider |
| `bench/replay.py`：加载 fixture → 指定 provider=replay → 比对**工具调用序列 + 终态 + verify** | 回放引擎 |
| 31 条素材全量回放 → `replay-pass-rate` | `bench/results/replay-v15.md` |
| 通过 → 归档"回放黄金集"，挂 CI 第五门 | 定版前必过 |

**回放的独特价值**：基准是开放式（测"新任务能不能做"），回放是封闭式（测"做对过的任务现在还做不做得对"）。后者对回归远更敏感，且因为 LLM 被打桩，**又快又确定**。

**边界声明**：31 条素材全部来自 zhipu；ReplayProvider 打桩后 provider 身份不再影响回放结果，故本轮回放与 v15 主 brain 选型**解耦**。跨 provider 的"同 prompt 不同模型"对比留 v16。

### 线 C：价值雏形 — agent vs 人（降级版，不进红线）

| 步骤 | 产出 |
|---|---|
| agent 侧：从矩阵 `wall_s` 出耗时分布（按 L1–L5 分层） | `bench/results/value-pilot-v15.md` |
| 人侧：提供 3 题（L2/L3/L4 各一）计时表模板 + 规程 | 同上，待用户填 |

**如实声明**：3 题是雏形不是完整 V 维度——完整实验需独立第三人、随机化顺序、排除学习效应。v15 只做"跑通方法"。

---

## §4 阶段拆解与依存（关键：service 独占）

> ⚠️ **硬依存**：跑分（S1/S1b）用**当前 service binary**；S2 的 ReplayProvider 需要**重编+重启 service**。二者不能并行，否则跑分中断。

| 阶段 | 线 | 任务 | 依存 |
|---|---|---|---|
| **S1** | A | deepseek 20×2（`chain_v15.py`，含 gemini 试水 fail-fast） | 现役 binary |
| **S1b** | A | gemini 天花板对照 T14/T19/T09 ×2 | S1 之后（同 binary，串行） |
| **S2** | B | ReplayProvider + replay.py + 31 条回放 | **必须等 S1/S1b 全部结束**（要重启 service） |
| **S2b** | — | ST7 判据拆分（v14 欠债③） | 无 |
| **S3** | C | 价值雏形（agent 侧自动 + 人侧模板） | S1 数据 |
| **S4** | A | 择脑判定：standard brain 或能力边界白皮书 | S1 + S1b |
| **S5** | 全 | 四门回归 + 应力场重跑 + 审计 + report + tag | 全部 |

---

## §5 验收红线（R4 已补齐）

| 级别 | 判据 |
|---|---|
| 🔴 | VM 四门任一不过：`fmt --check` / `clippy -D warnings` / `test --all` / `project-xray wiring`（8 条断言） |
| 🔴 | 回放成功率 < 80%（大量已成功 session 退化） |
| 🔴 | 应力场重跑出现 panic（回归） |
| 🟡 | deepseek 20×2 < 85%（两个 provider 都不够稳——如实记账，择脑失败但不阻塞交付） |
| 🟡 | 报告仍含 "replay.py 未实现"（v14 欠债不得跨到 v16） |
| 🔵 | 价值雏形报告格式 / 人侧数据缺失（如实标注即可） |

---

## §6 v16 预留（本轮不做）

- 跨 provider 回放（deepseek 素材 → deepseek 回放）
- 完整价值实验（随机化、第三人、排除学习效应）
- experience 持久化（当前仍是内存层）
- subconscious 完全动态化
- 应力场扩展到 12 场景（网络断／SSE 中断恢复／长 session 不泄漏）
- 若 S1b 显示 gemini 能过 T14/T19 → 夹具/工具链专项排查

---

*锻造断语：v14 证明了身体不崩。v15 回答三问——最好的脑子两遍还能 90%+ 吗？做对过的会不会再做对？agent 比人到底快多少？其中第二问必须用打桩的确定性回放来答，否则只是把基准又跑了一遍，自己骗自己。*
