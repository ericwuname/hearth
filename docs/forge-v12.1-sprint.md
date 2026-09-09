# codex-rust v12.1 — 锻造漏斗执行图纸

**日期**: 2026-07-30 05:50（修订：ZhiPu ✅ / Doubao 多模型）
**前置**: forge-report-v12.md（秤已校准）
**铁律**: 不动 crates/，不动 bench/ 的 fixture 结构，只补 bench/ + docs/

---

## §0 起点状态（更新至 05:50）

### Provider 矩阵（战后）
| Provider | 模型 | Plan | Tool dispatch | 备注 |
|---|---|---|---|---|
| Doubao | deepseek-v4-flash | ✅ | ✅ bash/grep/edit | 每模型 50 万 token |
| Doubao | deepseek-v4-pro | 🔜 | 🔜 | 同 key，切模型名即可 |
| Doubao | Doubao-Seed-2.0-mini | 🔜 | 🔜 | 50 万额度 |
| Doubao | Doubao-Seed-2.0-Code | 🔜 | 🔜 | 专码模型 |
| Doubao | Doubao-Seed-1.8 | 🔜 | 🔜 | 50 万额度 |
| ZhiPu | glm-4.5-air | ✅ | ✅ bash 222bytes×2 | 1214→已修复（Planner 加 user 消息） |
| ZhiPu | glm-4.5v / glm-4.7 | 🔜 | 🔜 | 同修复覆盖 |
| ZhiPu | GLM-5.2 | 🔜 | 🔜 | 最新旗舰 |
| Agnes | agnes-2.0-flash | ⚠️ | — | 503，不提 |
| Ollama | qiyuan-8b | 🔑 | — | VM 不可达 |
| OpenAI | gpt-4o | 🔑 | — | 需 key |

### 关键发现到执行窗口
```
v12       : 秤校准 → Plan ✅ / Tool ✅ → 0% 成功率 (grep 但 no edit)
v12.1 fix : ZhiPu 1214 → Planner 加 user 消息 → GLM-4.5 ✅
v12.1 now : 2 providers 可用 → Doubao 多模型轮转 → 正式 60 跑分
```

---

## §1 目标：第一个完整漏斗

**产出**: 60 次运行 × 2 providers × 6 fixtures = 至少 60 组三元组，画漏斗分布图。

```
         60 次运行（10 任务 × 3 遍 × 2 providers）
              │
              ▼
    ┌─ infra（环境故障，不计分母）───┐
    │  plan-fail（LLM 调用失败）     │
    │  retrieval-miss（没找到文件）  │
    │  wrong-edit（找到了改错）      │
    │  compile-fail（编译红修不好）  │
    │  gave-up（主动放弃）           │
    │  budget-out（步数耗尽）        │
    │  timeout（30min 超限）         │
    └────────────────────────────────┘
              │
              ▼
        通过 verify.sh = 成功
```

---

## §2 拆解任务（B1-B5 + 新：Provider 轮转）

### B0 — Provider 轮转策略（新增）
**目标**: 一个模型额度跑完自动切下一个，不中断跑分。

策略：
1. 启动时 `curl VolcArk/models` 拉所有可用模型
2. 取第一个 → 跑分 → 401/429 时切下一个
3. 优先级：deepseek-v4-flash → v4-pro → Seed-2.0-Code → Seed-2.0-mini → Seed-1.8
4. ZhiPu 同样：glm-4.5-air → glm-4.7 → GLM-5.2 → glm-4.5v

**产出**: `bench/providers.json` — 所有可用模型清单 + 状态

### B1 — 补 14 fixtures（T06-T19）
与 sprint.md §3 一致，无需修改。优先完成 5 个高频类型：

| # | 类型 | 层级 | 示例 |
|---|---|---|---|
| T08 | rename-function | L2 | 改函数名+调用点 |
| T09 | add-error-type | L4 | 自定义 Error |
| T13 | fix-index | L3 | 数组越界 |
| T16 | split-module | L5 | 拆模块 |
| T18 | add-pagination | L4 | 加分页 |

### B2 — runner.py 串行化
```
runner.py run-all [--runs 3]           # 串行跑全部 task
runner.py run-all --tasks T01-T05      # 指定子集
runner.py run-all --provider doubao    # 指定 provider
runner.py status                       # 显示已完成/剩余/ETA
```

### B3 — report.py
```
report.py baseline.jsonl → forge-baseline.md
  三元组总表 | L1-L5 分层 | 漏斗图 | per-provider 对比
```

### B4 — 正式跑分
串行 60 次（10 任务 × 3 遍 × 2 providers），预计 3-4 小时。

### B5 — 出报告 → 决定 v12.2 方向

---

## §3 执行顺序（本次会话）

```
[现在]  → B0 Provider 发现 + 轮转注册
        → B1 补 5 个高频 fixture (T13/T08/T09/T16/T18)
        → B2 runner.py 单任务修复 (加入 send_message 步 + fixture 上传)
        → B2 report.py 骨架
        → 守门 180/0
        → 提交 B0+B1+B2
```

---

## §4 验收

| 级别 | 判据 |
|---|---|
| 🔴 | fixtures 的 verify.sh 无法独立执行 |
| 🔴 | 密钥入仓 |
| 🔴 | `git diff -- crates/` 非空 |
| 🔴 | runner.py 无法逐个跑完所有 fixture |
| 🟡 | 失败分类超过 8 类（太细）或少于 3 类（太粗） |
| 🔵 | 成功率数字不设及格线 |

---

*锻造断语: 不是修到 100%，是看清每一种失败长什么样。*
