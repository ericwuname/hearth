# codex-rust v16 — 封刀轮：清债 + 边界白皮书 + 锻造综述

> 定位：v12→v15 四轮锻造证明了 agent 能跑（90%）、不崩（24/24）、不回退（wiring 9/9）、可重放（31/31）。v16 是锻造的最后一轮——**不再加新维度，把拖了三版的最后三笔债清掉，出能力边界白皮书，写一份完整的锻造证明**。
>
> v16 之后：锻造周期结束。进入"维护期"——每个季度跑一次基准、应力、回放，拿三次锻造门的数据出趋势报告。

---

## §1 起点状态（v15 过闸，deepseek 90.0%）

| 维度 | 状态 |
|---|---|
| deepseek 20×2 稳定性 | **90.0%**（36/40） ✅ |
| L4 层（zhipu 的痛点） | **10/10**（deepseek 全部水过） ✅ |
| 回放 | **31/31 = 100%**（68s / 原 2,354s / 零 token） ✅ |
| 应力场 v15 | **22/24 = 91.7%** / 0 panic / ST7 隔离零泄漏 ✅ |
| wiring | 9/9 断言全红 ✅ |
| 能力墙 | T14/T09 证实模型强度墙，T19 所有模型全挂 |
| 价值雏形 | agent 侧数据就绪（40 次 36.2 分钟），人侧待填 |
| experience | 内存层，无持久化（拖了六版） |
| subconscious | 部分信号硬编码（拖了五版） |

---

## §2 v16 目标：三线

### 线 A：清两笔跨版本毒债

| 债务 | 当前状态 | v16 目标 | 工作量 |
|---|---|---|---|
| **experience 持久化** | 内存 `RwLock<Vec>` — 重启即失忆 | 文件存储（JSONL append-only）+ 启动时 load | ~50 行 |
| **subconscious 信号完成** | CostGuard 硬编码 `cost_ratio=0.0`（永不触发） | 从 nervous-system / cost meter 读取真实值 | ~30 行 |
| **wiring 锁定** | +2 条新断言（experience-persists / cost-guard-live） | wiring-v13.toml 扩至 11 条 | — |

**为什么这时候清**：拖了六版的债，每次审计都点出来、每次都延期。锻造就是为了证明"不是不能修，是不敢修（怕引入回归）"。现在 wiring 防火墙有了、回放有了——有足够的安全网修这两笔。

### 线 B：能力边界白皮书

T14/T09/T19 三题在 v13-v15 三轮跨 provider 基准中建立了足够的数据来画边界：

| 题 | 现象 | 结论 |
|---|---|---|
| T14-add-serde | deepseek-v4-pro PASS / zhipu FAIL | **模型强度墙**：derive 宏需要更强模型 |
| T09-add-error-type | deepseek-pro PASS / zhipu FAIL | **模型强度墙**：自定义 Error 类型需要更强模型 |
| T19-merge-duplicate | 所有模型全挂 | **工具链/夹具缺陷**：泛型合并对当前 agent 工具集超纲 |

**产出**：`docs/capability-boundaries.md` — 按难度层 + provider 列出已知边界，带证据锚点（v13-v15 基准 JSONL 引用）。

### 线 C：价值维度人侧 + 最终锻造证明

| 步骤 | 产出 |
|---|---|
| 3 题人机对照的人侧数据 | 人工完成 T02/T13/T09 并计时 |
| 价值系数 = 人耗时中位 / agent 耗时中位 | `value-coefficient`（>1 = agent 更快） |
| 锻造综述报告 | `docs/forge-final-report.md` — v12→v16 四轮锻造的完整证据链 |

---

## §3 阶段拆解（源码审查后优化版）

> 审查发现：S1 和 S2 都改 `loop.rs`（S1 改第 1832 行写文件、S2 改第 989 行 cost_ratio），两次编译=双倍等待。**合并为一次改动**。S3 断言文档可与代码改动并行。S5 人侧数据需真人操作，设降级方案。

| 阶段 | 线 | 任务 | 预计 |
|---|---|---|---|
| **S1+S2** | A | **合并**：experience 文件持久化 + CostGuard 真值接地 | **0.5 天** |
| **S3** | A | wiring +2 条新断言（文档改动，与 S1+S2 并行） | 0.2 天 |
| **S4** | B | 能力边界白皮书撰写（与 VM 编译并行） | 0.5 天 |
| **S5** | C | 3 题人侧数据 + 价值系数计算（⚠️ 需真人 30-60min） | 0.5 天 |
| **S6** | C | 锻造综述报告 v12→v16 | 0.5 天 |
| **S7** | 全 | 守门审计 + tag v16.0 | 0.5 天 |

### 降级条款（S5 无真人时）

若用户无 30-60 分钟连续操作，S5 降级为：
- **只做 T02-change-timeout（L2）** 一题（~5 分钟），余两题标记"待补"
- 或用 ollama 本地模型跑一个"模拟人" baseline 做最低保底
- 降级不影响 🔴 验收红线（S5 是 🟡/🔵 级别）

### 关键技术决策

**experience JSONL 并发安全**：多 session 同时 append 会交错行。
```rust
pub struct ExperienceStore {
    entries: tokio::sync::RwLock<Vec<Experience>>,
    file_mutex: tokio::sync::Mutex<()>,   // 保护文件写，不阻塞读
    file_path: Option<PathBuf>,
    embed_fn: Option<EmbedFn>,
}
```
`append()` 先写内存（`RwLock` 读锁即可），然后抢 `file_mutex` 追加 JSONL。

**NervousSystem → CostGuard 数据流**：NervousSystem 有 `cost_accumulated_usd` 和 `cost_budget_usd` 但没有归一化输出。加 ~5 行：
```rust
pub fn cost_ratio(&self) -> f32 {
    match self.cost_budget_usd {
        Some(b) if b > 0.0 => (self.cost_accumulated_usd / b).min(1.0) as f32,
        _ => 0.0,
    }
}
```
`loop.rs:989` 硬编码 `0.0` → `self.nervous.cost_ratio()`。~10 行净改动。

---

## §4 执行窗口（源码审查优化版）

```
窗口 A（清债）：S1+S2（一次改完）→ cargo build → deploy → wiring 11 条
                 S3（并行：wiring 断言文档与 VM 编译同时做）
窗口 B（写稿，与 A 并行）：S4 — 能力边界白皮书
窗口 C（算数）：S5 — 人侧数据 + 价值系数（或用降级方案）
窗口 D（收尾）：S6 锻造综述 + S7 审计 + tag v16.0 + 进入维护期声明
```

> 净代码改动 ~80 行（S1+S2），加上编译等待与验证，窗口 A 约 1-1.5h。窗口 B 可完全与 A 并行。

---

## §5 验收红线

| 级别 | 判据 |
|---|---|
| 🔴 | experience 持久化后重启 service，`GET /api/v1/experience/metrics` 保留重启前的 count |
| 🔴 | subconscious CostGuard 的 `cost_ratio` 从 nervous-system 真实读取（非硬编码 0.0） |
| 🔴 | wiring 11 条全绿（新增 2 条不得断裂） |
| 🔴 | 锻造综述报告 v12→v16 含四轮基准的趋势数据 |
| 🟡 | 能力边界白皮书格式 |
| 🔵 | 价值系数数字（不做及格线） |

---

## §6 v16 之后：进入维护期

```
季度体检（不再每版跑全量 60 次）：
  每季度 1 次：基准 20×2（deepseek）+ 应力场 24 次 + 回放 31 条
  产出：季度趋势报告（成功率/成本/时长 三条曲线）
  红线：成功率 < 85% 或 应力场有 panic

维护期的新版本：
  只有新功能才加基准任务（新 fixture + wiring 断言）
  修 bug 不跑全量基准（只跑受影响的题 + 回放 + 应力场的受影响场景）
```

---

*锻造断语：四轮七天的锻造接近尾声。从 0% 到 90%，从 grep 死循环到 wiring 防火墙自证能变红，从一碰就崩到 24/24 应力场零 panic。最后一轮不是加新的——是把忘了六版的债清了、把边界画好、把证明写完。然后刀入鞘，季度体检。*
