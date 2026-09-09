# 全局透视盘点 — v18 时点（2026-07-31）【已 supersede】

> ⚠️ **本文已被 `docs/global-panorama-v21.md` 取代**（v21.0 已提交+维护期启动，2026-08-01）。保留本文件仅作历史轨迹。
> 替代并 supersede `global-panorama-v11.5.md`（v11.5 写于 07-30，早于基因系统，已严重过时）。
> 作者：顶层架构角色 · 2026-07-31
> 判据延续 v10.1 铁律：**只认"生产路径可达"（grep 可证调用链），不采信提交信息/交付报告自报**。
> 本盘点基于**真实主干状态**：`git` HEAD = `fd5829e`（**v17.0**，已提交）+ **未提交 v18 WIP**（工作树改动）。

---

## 〇、版本状态（最重要的一句）

**主干停在 v17.0，但工作树已演进到 v18 阶段（未提交）。** 任何"全景"若仍以 v11.5 / v17.0 为基线，都漏了 v18 这次大改。

| 项 | 状态 |
|---|---|
| 最新 tag | `v17.0`（`fd5829e`） |
| 工作树 | **有未提交改动**：`experience`(+34)、`service`(+61 行+`reqwest` 依赖)、bench 一批 v18 脚本/结果 |
| v18 性质 | **精炼轮**：embedding 语义召回 + LLM 经验精炼 + 随机任务序（见 §5） |
| v18 提交状态 | **未提交、未 tag**。E0/E1/E2/E3 **四组全部跑完**，守门员审计已通过（见 `gatekeeper-audit-v18.md`） |

---

## 一、轮次谱系（v12 → v17 主干，v18 WIP）

| 版本 | 轮次名 | 主干实质贡献 |
|---|---|---|
| v12.x | 基准轮 | Rust 自动编程基准 0%→95%/100%；工具 rename（edit→write_file）；`ToolContext.cwd`→session workspace |
| v13.0 | 接线防火墙 | codex-xray + 接线断言（7 条 → 现 11 条）；constitution/civ 双 A 接 |
| v14.0 | 淬火轮 | `sub_budget` 毒债修复 + wiring 红线断言；应力场 24/24 零 panic；31 条回放素材归档 |
| v15.0 | 铸基轮 | 择脑（standard=deepseek 90%）；ReplayProvider 100% 零 token；应力场 22/24；天花板对照 |
| v16.0 | 封刀轮 | 清债 + 边界白皮书 + 锻造综述 |
| v17.0 | 生长轮 | **经验回路闭合验证（仅 zhipu 弱模型有效，+20pt，v17 自报）** |
| v18 WIP | 精炼轮 | embedding 向量召回 + LLM 经验精炼 + 随机序（**本轮重点，见 §5**） |

⚠️ **v17 的"经验回路有效"结论仅对弱模型(zhipu)成立，不可外推到强模型**——v18 用 deepseek 复现得到否定结果，见 §5。

---

## 二、基因层级系统（v11.5 全景不存在，v12.1 才确立）

出处 `docs/gene-expression-system-design.md:28-31`。所有 RFC/设计必须使用此定义：

| 层 | 名称 | 牙齿 | 表达器官 | 硬上限 |
|---|---|---|---|---|
| G0 | 结构基因 | 物理不可能违反 | sandbox/landlock/seccomp、审批门、进程隔离 | 无上限，每条=拓扑级 |
| G1 | 反射基因 | 代码分支必然执行 | 相位钩子（subconscious/nervous/CostGuard/交付三门） | 每条必挂 wiring 断言 |
| G2 | 检索基因 | 正确时机出现在眼前 | `experience` crate 判例卡+情境召回 | 无上限，受 KPI 淘汰 |
| G3 | 叙事基因 | **无牙齿（偏置）** | `constitution.md`→build_messages | **≤10 条，换血制** |

铁律：入籍三元组（层级/表达器官/验证方式）；**单向上升不许空降**；G3 想加第 11 条先废一条。

---

## 三、器官地图（v17.0 已验证 + v18 改动标注）

> 验证口径：grep 存在性 → 读控制流确认行为 → 确认表达力。下列"行为"结论均来自 fd5829e 源码核对。

| 器官 | 状态 | 备注 |
|---|---|---|
| `agent-core`（loop.rs） | 🟢 | `LoopPhase{Init,Plan,Act,Observe,Reflect,Done,Error}`；钩子 do_plan:851 / do_act:1134 / do_observe:1316 / do_reflect:1509 |
| `constitution` | 🟢 修好 | `constitution_prompt()` 运行时读 `constitution.md`（:35），超 6000 字符静默截断。**（注：v11.5 全景记录的 #2 宪法回归，v13+ 已修）** |
| `experience` | 🟢→🟡(v18 改) | G2 层。v18 注入 embedding（见 §5）；search 现 cosine+keyword fallback |
| `project-xray` | 🟢 | 接线断言 **11 条**，CLI spec **硬编码** `docs/xray/wiring-v13.toml`（main.rs:36）。**新建 wiring-vNN.toml CI 不读** |
| `nervous-system` | 🟡 | `query()`→NerveAction；唯一调用点 loop.rs:1589（do_reflect 内）→ 无会话即零感知 |
| `subconscious` | 🟡 | `SubconsciousGate::check()` 接 loop.rs:991（do_plan 前） |
| `sandbox` | 🟢 | landlock+seccomp，直接 exec+pre_exec 套 NO_NEW_PRIVS（VM 禁 unshare） |
| `service` | 🟢→🟡(v18 改) | v18 加 `zhipu_embed_fn` + `reqwest` 依赖（见 §5） |
| `resource-monitor` | 🟢 | `is_critical()`=mem>80%||disk<1GB |
| `llm-gateway` | 🟢 | trait `LlmProvider`，方法 `chat()`（**无 `generate()`**） |

---

## 四、已知债务台账（v17 既有 + v18 新增）

**v17 既有（来自 MEMORY.md 债务台账）：**
- ❌ `/readyz` 假实现（routes.rs:255-259）
- ❌ 硬编码 key **6 处**（main.rs:40/254/271/305/329/344 + :156 placeholder）
- 🔴 loop.rs:1590 丢弃 `drain_civ_alerts()` 返回值；loop.rs:417 `drain_nervous_alerts()` 零调用者（中断日志生成后被扔）
- 🟡 `is_critical=Abandon|DeliverAndQuit` → loop.rs:1595 的 `Simplify` 分支**不可达**

**v18 新增（本轮发现）：**
- 🔴 **`service/src/main.rs` `zhipu_embed_fn()` 硬编码 ZhiPu key**（`<redacted-key>`）——与既有 6 处硬编码同源反模式，应抽环境变量/配置。

---

## 五、v18 WIP 深度盘点（本轮重点）

### 5.1 代码实现（已确认，工作树未提交）

**`crates/experience/src/lib.rs`（线A embedding）：**
- `set_path()` 启动时**回填 legacy 条目 embedding**（无 embedding 的条目调 `embed_fn` 补算）。
- `search()`：cosine 模式下若全部条目为 legacy（cosine=0→空集），**回退 keyword**——迁移窗口安全网。

**`crates/service/src/main.rs`（线A 接线）：**
- 新增 `zhipu_embed_fn()`：调 ZhiPu `embedding-3`（256 维）via `reqwest`，`ZHIPU_API_KEY` 环境变量（**含硬编码 fallback key**）。
- 启动在 `Arc::new` **之前**注入 embed_fn（`set_embed_fn` 取 `&mut self`）。
- `EMBEDDING_ENABLED` 环境变量（默认开；`=0` 关 → keyword 控制臂，对应 v18 E1/E2）。
- `Cargo.toml` 加 `reqwest.workspace = true`。

> 线B（LLM 经验精炼脚本 `refine_experiences_v18.py`）与线C（`runner --shuffle`）已写脚本，但 E3（embedding+refined 臂）实验未完成。

### 5.2 实验结果（bench/v18-e0..e3.log，provider=**deepseek**）

| 实验 | 变量 | 结果 |
|---|---|---|
| **E0** | 空 store + 随机序 | **35/40 = 87.5%** |
| **E1** | keyword + 规则构造经验 + 随机序 | **35/40 = 87.5%** |
| **E2** | keyword + LLM 精炼经验 + 随机序 | **34/40 = 85.0%** |
| **E3** | embedding + LLM 精炼经验 + 随机序 | **33/40 = 82.5%**（四组全部完成） |

### 5.3 🔴 红色发现：经验回路是 **provider/模型强度相关** 的（非顺序效应）

> ⚠️ **更正（2026-07-31 晚，数据出来后）**：本盘点初稿曾把 v17→v18 落差归因为"任务顺序效应"，**已被 v18 完整四组实验推翻**——E0 随机序 87.5% ≈ v15 固定序 90%，**无顺序效应**。v17 的 65% 是 zhipu 模型波动，不是顺序偏差。真实机制如下：

- **E0（空 store，随机）= 87.5%** = E1（注入规则经验）= 87.5% → **对强模型(deepseek)经验零增量**。
- **E2（LLM 精炼）= 85.0%**、**E3（embedding+精炼）= 82.5%** → 经验注入**反而略降**（噪声内；机制假设"无关经验干扰强模型推理"，审计 D2 未完全证实）。
- **跨 provider 对照**：zhipu(弱，65% baseline) +20pt 有效 🟢；deepseek(强，87.5%) −5pt 无效/有害 🔴。
- **结论**：经验回路的适用域 = **模型能力低于任务难度的场景**。对生产标准脑(deepseek)它是噪音/干扰，应作为**可选降级通道（连续失败才注入）**，非常驻增强。v17 的"+20pt 有效"是真实的**弱模型**结论，错在**外推**。**`--shuffle` 仍应作为基准默认（消除任何残留顺序偏差），但顺序不再是 v17 落差的主因。**

### 5.4 方法论偏离（计划 vs 实际）

| 计划（self-evolution-v18-plan.md） | 实际 |
|---|---|
| 矩阵用 zhipu glm-4.5-air（与 v17 可比） | **实际全用 deepseek**（E0-E3 日志均 `provider=deepseek`） |
| E0-E3 各 20×2=40 全跑完 | E3 未完成 |
| 经验来源含规则构造(37条级) | 实际仅 5 条（refined/rule 各 5） |

---

## 六、对 G2 / 基因系统的含义（实证更新）

- v18 把 G2 检索从纯 keyword 升级为 **cosine + keyword fallback**（线A 已落代码 + 守门员审计接线核对 ✅），是 G2 表达器官的能力增强。
- 🔴 **但对强模型(deepseek) G2 召回经实证无正向贡献、甚至略负**（E0=E1 零增量，E2/E3 略降）。G2 的"正确时机出现在眼前"**对强模型是噪音假设**，已被 v18 证伪（在强模型域）。
- **架构含义**：G2 不应作为常驻机制，应改为**降级通道**——仅当 agent 连续失败（模型能力<任务难度）时注入经验。这同时简化了系统（常态不注入 = 不加载 embedding、不调用精炼、不污染 prompt）。
- ⚠️ 弱模型域 G2 仍有价值（v17 zhipu +20pt），但本项目标准脑是 deepseek，故默认形态 = 关闭常驻注入。

---

## 七、下一步（经 v18 结论收敛后的待办，待用户裁决）

> 经 v18 否定性结论，v15→v18 在经验回路上的四轮投入已闭环：**对生产模型零收益**。这恰好替我们做了一个"停止"决策——embedding / LLM 精炼 / 自主凝练闭环 整条线从"有益待证"降为"停止"。v19 已据此收口并向前走了关键一步。

1. **收口 v18/v19（必要·已做）**：v18.0(`9d6a2c4` 否定性发现轮) + v19.0(`76cf11e` 深水轮) **均已提交+tag**，悬空 WIP 态已结束（对应 telos 锚点 D2）。v19 交付：T13/T19 根因解剖 + 经验自适应开关(`consecutive_errors>=3` 门控，wiring 12/12) + deepseek 固定序 92.5% 新高基线。我上轮路线图建议#3「经验=降级通道」已落地。
2. **回退 embedding 死代码（必要·减债·仍开）**：E3 证无增量，且 `zhipu_embed_fn` 含硬编码 key（债务台账 🔴）→ v19 **未回退**，该代码 + reqwest 依赖 + 硬编码 key 仍在 `service/main.rs`（:28/:543）。建议 v20 顺手回退，或单独清理。**回退属施工活，顶层出任务书、不写代码。**
3. **写"经验回路适用域"设计决策（必要·顶层文档·已部分落地）**：实证结论已落成 `loop.rs` 自适应开关代码（连续失败≥3 才注入）；可补一份短设计决策文档固化口径。非业务代码。
4. **T13/T19 真凶 = planner 判定缺陷（非能力墙）**：v19 解剖证明 `loop.rs:967` `all_done` 把 Read 节点 Completion 当任务完成 → agent 读完不写就自报 Done。这是六轮 0/8 全挂、经验注入无效的真正原因。v20 修复**已落工作树（未提交）**：`all_done` 须 `write_attempted` 才放行，否则强制 replan（loop.rs:973-996）。属"必要·修真 bug"，非"能力/边界"之争。
5. **对外 RFC（CN-001 / RFC-004 v3 / RFC-005 替代路径）**：维持暂停，待 telos 锚点 D1-D4 拍板。

---

*本盘点所有代码事实均在 `fd5829e` + 当前工作树现场 grep/读源码复核（2026-07-31）。v18 实验结果来自 `bench/v18-e{0,1,2,3}.log` 原始日志。*
