# codex-rust v20 — 收官轮：修 planner 毒债 + 经验遗忘部署 + 基线封存

> 定位：v19 找到了 T13/T19 的真凶——`loop.rs:967` 的 `all_done` 判定把"读了文件"当成"完成了任务"。v20 修掉这个横跨六版的毒债，把 deepseek 从 92.5% 推到 95%+，部署经验系统最后的遗忘机制，然后**封存季度基线**。
>
> v20 之后：自进化实验周期结束。系统进入稳态——每季度跑一次全量体检。

---

## §1 起点状态（v19 深水轮过闸，基线 92.5% 新高）

| 发现 | 证据 |
|---|---|
| **T13/T19 root cause** | planner all_done 把 Read 节点完成 == 任务完成 → agent 读完就走，从不写 |
| **六轮 0/8 全挂解释了** | 不是模型不会写，是 planner 说不用写 |
| deepseek 基线 92.5% | 历史新高（v15 90.0%） |
| 经验自适应开关 | wiring 12/12，强模型不注入、弱模型降级用 |
| T19 唯一盲区 | NO_GENERIC_FN——需要工具链升级（v20 先修 planner，看够不够） |

---

## §2 v20 目标：三线（两个 P0 + 一个维护）

### 线 A：修 planner all_done —— T13/T19 根治（P0）

**bug 位置**：`loop.rs:967`

```rust
// 当前逻辑（有缺陷）：
// all_done = 所有节点的 status ∈ {Completed, Skipped, Failed} → 真的 Done

// 修复后：
// all_done = 所有 Write 类节点已完成 + 至少 1 个 Write 节点被真实执行
// Read 类（grep/read）节点完成 ≠ 任务完成
```

| 修复 | 文件 | 行数估计 |
|---|---|---|
| task_graph 节点打 Type 标签（Read/Write） | `loop.rs` | ~10 行 |
| all_done 只认 Write 节点完成 + ≥1 个 Write 真实执行 | `loop.rs:967` | ~5 行 |
| Done 前兜底自检：无 Write 执行记录 → 强制 Replan | `loop.rs` | ~5 行 |
| wiring 新断言 `all-done-requires-write` | `wiring-v13.toml` | +1 条 |

### 线 B：退火 —— 遗忘机制部署（P1）

experience store 的 `prune()` 和 `upgrade_core()` 函数在 v11 就写好了，但从未接入定时调度。v20 做 30 行的接线工作：

| 步骤 | 文件 |
|---|---|
| 在 service 的 observer 每小时任务中加 prune 巡检 | `service/main.rs` |
| prune 阈值：effectiveness < 0.3 + 90 天未引用 → 归档/删除 | `experience/src/lib.rs`（函数已有） |
| upgrade_core：refs ≥ 3 + effectiveness ≥ 0.8 → Core 层 | `experience/src/lib.rs`（函数已有） |
| wiring 断言 `experience-prune-wired` | +1 条 |

### 线 C：季度基线封存（v20 定版后）

| 步骤 | 产出 |
|---|---|
| 修完 planner 后重跑 deepseek 20×2 固定序 | 新基线通过率 |
| 以 v20 定版为 Q3-2026 基线（FMT/CLIPPY/TEST/wiring 全绿 + 基准通过率 + 应力场 + 回放） | `docs/quarterly-baseline-q3-2026.md` |
| 后续版本：只有新功能加基准任务；修 bug 只跑受影响题 + 回放 + 应力场 | 维护期规程 |

---

## §3 阶段拆解

| 阶段 | 线 | 任务 | 预计 |
|---|---|---|---|
| **S1** | A | planner all_done 修复 + wiring 断言 | 0.5 天 |
| **S2** | A | 修复后 T13/T19 单题验证（各跑 3 次确认根治） | 0.5 天 |
| **S3** | B | 遗忘机制部署（prune + upgrade_core 接线） | 0.5 天 |
| **S4** | C | deepseek 20×2 固定序重跑（修复后基线） | 0.5 天（机器时间） |
| **S5** | C | zhipu 自适应验证补跑（S4 降级项） | 0.5 天（机器时间，可选） |
| **S6** | 全 | 守门审计 + tag v20.0 + 季度基线封存 | 0.5 天 |

---

## §4 执行窗口

```
窗口 A（修 bug）：S1+S2 — planner 修复 + T13/T19 验证
窗口 B（部署）：S3 — 遗忘机制 + wiring 断言
窗口 C（封存）：S4+S5+S6 — 基线重跑 + 季度体检基线发布
```

---

## §5 验收红线

| 级别 | 判据 |
|---|---|
| 🔴 | T13 修复后仍在 0/3（修复无效） |
| 🔴 | wiring 断言破裂 |
| 🔴 | 修复引入新回归（此前 PASS 的题变 FAIL） |
| 🟡 | T19 修复后仍 0/3（planner 修了但 NO_GENERIC_FN 确实需要工具链升级） |
| 🔵 | 新基线通过率数字（不做及格线，只记录） |

---

## §6 v20 之后：进入稳态

```
季度体检（不再每版跑全量）：
  deepseek 20×2 固定序 + 应力场 24 次 + 回放 31 条
  红线：通过率 < 85% 或 应力场 panic > 0

新版本规则：
  新功能 → 加 1+ 基准任务 + wiring 断言
  修 bug → 只跑受影响题 + 回放 + 应力受影响的场景
  季度才跑全量体检
```

---

*锻造教会我们"先测再修"——v13-v18 六轮对着 T13/T19 跑了 48 次基准，所有 provider、所有经验配置全挂。v19 拿出解剖刀看 agent 真做了什么，才发现它从来没写过。这不是模型笨——是 planner 说"读完就算完了"。修掉这一行，agent 才真正开始干活。*
