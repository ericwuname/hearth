# codex-rust v18 — 精炼轮：LLM 凝练 + embedding 向量化 + 消除顺序效应

> 定位：v17 证明了"经验回路有效"（65→80→85%）。v18 把经验的**质量**拉起来——从"规则构造的粗经验"到"LLM 精炼的语义经验"，从"keyword fallback"到"embedding 向量检索"。不是加新机制——是把已有的 self-evolution 管线从"能跑"推到"跑得好"。

---

## §1 起点状态（v17 经验回路证明有效）

| 发现 | 证据 |
|---|---|
| 经验回路有效 | 65% → 80% → 85% 三轮单调递增 |
| 经验精准命中 | improved 8/9 是之前失败题，仅 1 regression |
| 经验搜索真工作 | reuse_rate = 0.49（37 条经验中近一半被 search 命中） |
| 经验质量有增量 | 规则构造（R3 85%）> agent 自写空泛条目（R2 80%）> 空 store（R1 65%） |
| 当前局限 | keyword fallback 搜索（非语义）、经验 rule-constructed（非 LLM 精炼）、任务顺序未随机化、仅 zhipu 数据 |

---

## §2 v18 目标：三条真金

### 线 A：embedding 向量化 —— 把 keyword fallback 升级为语义搜索

**当前**：`ExperienceStore::search()` 没有 embed 函数 → 走 keyword 子串匹配（`lib.rs` keyword fallback）。v17 的 reuse_rate 0.49 是在 keyword 模式下取得的。

**v18**：注入真实的 embedding 函数 → `cosine_similarity` 的代码已经写好在 `experience/src/lib.rs`。

**embedding 来源（源码审查后定）**：~~OpenAI text-embedding-3-small~~ **zhipu `embedding-3`（256 维）**——实测 OpenAI 本机直连 20s 超时（VM 大概率被墙），zhipu embedding-3 实测 200 OK，且与 LLM 精炼同通道（zhipu）。

**实现坑（审查发现）**：`set_embed_fn(&mut self)` 但 service 中 ExperienceStore 已 `Arc::new` → 必须在 `Arc::new` 之前构造 embed_fn 并调用，main.rs 调换构造顺序。

| 指标 | v17（keyword） | v18（embedding） | 验证方式 |
|---|---|---|---|
| search 命中精度 | keyword 子串匹配 | cosine 语义相似 | 同一 query → 对比两类返回的 top-3 经验相关度 |
| reuse_rate | 0.49 | 预期上升 | metrics 端点对比 |

**验证**：`bench/gen_experiences_v17.py` 里加一个 `--eval-search` 模式——对每条 FAIL 跑 keyword search 和 embedding search，人工评分 top-3 相关度。embedding 相关度 > keyword → embedding 胜出。

### 线 B：LLM 经验精炼 —— 从"规则构造"到"完整语义复述"

**当前**：v17 的经验内容是规则构造的——`"task: T09, error: TEST_FAIL, fix: check enum variants"`。有效（+5pt vs R2），但极限有限。

**v18**：用 LLM（zhipu glm-4.7——glm-4.5-flash 实测返回空 content 不可用）读 FAIL session 的消息历史（`GET /api/v1/sessions/:id/messages` 端点已存在）→ 凝练为结构化经验：
```
{
  "problem": "自定义 Error 类型未实现所有必要的 trait bound",
  "solution": "为 Error 类型派生 Debug + Display，并为转换函数实现 From trait",
  "context": { "level": "L4", "category": "error-handling" }
}
```

**验证**：v18 的注入实验用 LLM 精炼经验替代规则构造经验 → 对比通过率（预期 ≥ v17 R3 的 85%，因为有更丰富的语义内容）。

### 线 C：随机任务顺序 —— 消除 v17 的唯一方法学漏洞

**当前**：T00→T19 固定顺序，第 2 轮"后半段受益于前面的经验积累"可能混入顺序效应。

**v18**：runner 加 `--shuffle` 参数，两轮使用不同随机序。如果随机化后仍是 80%+ → 经验回路与顺序效应无关。

---

## §3 核心实验（复用 v17 工装，加三个变量）

```
VARIABLE_A：搜索方式（keyword vs embedding）
VARIABLE_B：经验来源（规则构造 vs LLM 精炼）
VARIABLE_C：任务顺序（固定 vs 随机）

实验矩阵（zhipu glm-4.5-air，与 v17 同 provider 可比）：

  E0：baseline（空 store，随机顺序）              → 确认随机化的 baseline
  E1：keyword + 规则构造 + 随机（v17 复制+随机化）  → 消除顺序效应后复现 85%
  E2：keyword + LLM 精炼 + 随机                    → 仅改经验质量
  E3：embedding + LLM 精炼 + 随机                  → 改搜索+经验质量
```

**如果 E3 > E2 > E1**：三个变量各产生增量 → 全投入 v19。
**如果 E2 ≈ E1**：LLM 精炼对 zhipu 无增量（规则构造已够好）→ embedding 是 v19 唯一投入。
**如果 E3 ≈ E2**：embedding 无增量（keyword 已够好，或 embedding 维度太低）→ 仅 LLM 精炼投入 v19。

### 审查补充：E0 的方法学意义 + 显式清理

1. **E0 是方法学验证**：若 E0（随机化 baseline）明显高于 v17 的 65%（如 ≥75%），说明 v17 的 65% 受顺序偏差影响（后半段任务积累经验抬高了某些轮次）——**随机化后 baseline 与注入组的差距才是经验回路净效应**。
2. **每轮 store 清理/注入改为显式参数**（v17 隐式 clear 坑）：实验脚本不接受隐式 `ssh_clear_and_restart`，清理/注入必须由调用方显式指定。
3. **--shuffle 随机 seed**：每次启动随机（不固定 seed），与"消除顺序效应"目标一致。

---

## §4 阶段拆解

| 阶段 | 线 | 任务 | 预计 |
|---|---|---|---|
| **S1** | A | embedding 函数注入 + `--eval-search` 对比工具 | 0.5 天 |
| **S2** | B | LLM 经验精炼脚本（读 FAIL session → 凝练 → append） | 0.5 天 |
| **S3** | C | runner `--shuffle` + runner `--provider` 参数化 | 0.5 天 |
| **S4** | 全 | 跑 E0-E3 四组实验（每组 20×2，共 160 次） | 1 天（机器时间） |
| **S5** | 全 | 实验报告 + embedding vs keyword 对比 + 结论 | 0.5 天 |
| **S6** | 全 | 守门审计 + tag v18.0 | 0.5 天 |

---

## §5 执行窗口

```
窗口 A（工具）：S1+S2 — embedding 注入 + LLM 精炼脚本
窗口 B（工装）：S3 — runner 随机化 + provider 参数化
窗口 C（跑分）：S4 — 160 次实验（后台 VM，可分批）
窗口 D（收尾）：S5 报告 + S6 审计
```

---

## §6 验收红线

| 级别 | 判据 |
|---|---|
| 🔴 | wiring 11 条断言破裂 |
| ��� | 实验数据造假（非真跑 VM） |
| 🟡 | embedding search 相关度 < keyword（不需要做 embedding 了） |
| 🟡 | LLM 精炼 +5pt 不成立（LLM 精炼无增量，v19 省掉这一步） |
| 🔵 | 通过率提升幅度（不做及格线） |

### 定版修订记录（源码审查后）
1. embedding 来源：OpenAI → **zhipu embedding-3**（OpenAI 本机超时被墙，zhipu 实测 200 OK）。
2. LLM 精炼模型：deepseek/zhipu → **zhipu glm-4.7**（glm-4.5-flash 实测空 content 不可用）。
3. `set_embed_fn` 需在 `Arc::new` 前调用（&mut self 约束）。
4. 实验矩阵补充 E0 方法学判读 + 显式 store 清理。

---

## §7 v19 预留

- **自主闭环**（v10.5 Phase 5）：v17 的 self-evolve 是人工驱动的"跑分→构造经验→注入→重跑"。v19 把这条管线写进 AgentLoop 本身——`do_reflect` 结束自动凝练经验、下次 `do_plan` 自动搜索注入、全程无人干预。
- **遗忘机制**（v10.5 Phase 4）：经验持续积累后需要 prune + upgrade_core——两个函数已写好，只需接入定时任务。
- **deepseek 版复现**：API 恢复后跑 deepseek 版 E0-E3，确认跨 provider 稳健。
- **沉积岩已写好在 experience/src/lib.rs**：`upgrade_core()` 和 `prune()` 的代码 v11 就建好了——只差一个定时调度触发器。

---

*v17 证明了经验有用。v18 回答——什么样的经验更有效？哪个搜索通路更准？随机化后结论还成立吗？三个变量、四组实验、一个清晰的路线图——每一项投入（embedding/LLM 精炼/随机化）都有对应的验证判据，做完就知道下一步投什么。*
