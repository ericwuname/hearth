# codex-rust v11.4 — 守门员综合审计

**审计日期**: 2026-07-30 00:33  
**基线交付物**: `final-audit-v11.4.md`  
**对比基线**: `gatekeeper-review-v10.5-comprehensive.md`（双红）  

---

## 〇、一句话结论

**Part A/B/C/D 全部落地——两个新 crate 货真价实，潜意识门禁在生产路径真接线，经验存储完整 API。是 v1.0 以来实现率最高的版本。唯一卡闸项：FMT 连续第 6 版失败（7 diff，全在新 crate）。**

---

## 一、真机三门（Linux VM）

| 门禁 | 状态 | 详情 |
|:---|---:|---|
| FMT | **❌ RC=1** | 7 处 diff（experience 4 + service 3） |
| CLIPPY | ✅ RC=0 | 0 警告 0 错误 |
| TEST | ✅ RC=0 | **181 实跑 / 0 失败**（声明数 = 实跑数 — 首次完全一致） |

**FMT 6 版连续失败历史**：

| 版本 | diff 数 | 根因 |
|---|---|---|
| v10.3 | 2 | 新代码 |
| v10.4 | 4 | 新代码（main.rs） |
| v10.5 | 8 | 新代码（loop+routes+main+registry） |
| **v11.4** | **7** | 新 crate experience(4) + service(3) |

**v11.4 FMT 7 处 diff**：

```
experience/src/lib.rs:174 — upgrade_core 方法格式化
experience/src/lib.rs:227 — metrics 方法格式化
experience/src/lib.rs:244 — 测试代码格式化
experience/src/lib.rs:398 — 测试代码格式化
service/src/routes.rs:584 — 新路由处理函数
service/src/session.rs:4   — 新 import
service/src/session.rs:10  — 新 import
```

---

## 二、v10.5 方案落地追溯

| Part | 内容 | 状态 | 证据 |
|---|---|---|---|
| A | v10.5 FMT+TEST 双红修复 | ✅ | TEST=181/0（v10.5 是 163 有 1 fail） |
| B | 潜意识层重构 | ✅ | `subconscious/src/lib.rs`（343行，5 guards，7 tests） |
| C | 五层记忆自进化 | ✅ | `experience/src/lib.rs`（~400行，8 methods，10 tests） |
| D | 执行方案 + 打包 | ✅ | FMT+TEST+CLIPPY 零阻塞（除 FMT） |

---

## 三、新增 crate 源码核实

### 3.1 subconscious（潜意识层）

**路径**: `crates/subconscious/src/lib.rs`（343 行）

| Guard | 逻辑 | 测试 |
|---|---|---|
| ConstitutionGuard | 检查 `rm -rf /`/`del /S /Q` 等危险命令 → Abandon | ✅ `constitution_guard_blocks_rm_rf` |
| CostGuard | cost_ratio > 0.95 / > 0.80 → Simplify | ✅ `cost_guard_triggers_at_96pct` |
| RepetitionDetector | 滑动窗口 10，失败率 > 70% → Simplify | ✅ `repetition_detector_at_8_of_10_fails` |
| LspGuard | 薄片：LSP 检查推迟到诊断注入 | — |
| ExperienceMatcher | 经验 → 短标签 `[直觉] 类似场景经验: X (置信度 N%)` | — |
| SubconsciousGate | 按优先级串联 5 个 guard | ✅ `gate_stops_at_first_guard` |

**AgentLoop 接线**：`loop.rs:754-776`，`apply_subconscious()` 在 `build_messages()` 之前执行，返回 `Abandon/Simplify` 直接跳 Phase。

### 3.2 experience（经验存储）

**路径**: `crates/experience/src/lib.rs`（~400 行）

| 方法 | 功能 |
|---|---|
| `append()` | 追加经验 + 生成 embedding |
| `search()` | 余弦相似度 + keyword fallback → Top-K |
| `evaluate()` | 环境匹配 → Reuse/Adapt/Create |
| `prune()` | 低评分 + 过期 → 删除 |
| `upgrade_core()` | effectiveness ≥ 0.8 + 引用 3+ → 核心经验层 |
| `reinforce()` | 评分增减 |
| `metrics()` | GrowthMetrics（复用率/新建率/纠正率） |
| `env_match_score()` | ContextRef 参考系校准 |

**AgentLoop 接线**：`loop.rs:344` 持有 `ExperienceStore`，`loop.rs:446` 注入，`run()` 末尾写入经验。

### 3.3 prompt 瘦身验证

```rust
// loop.rs:597-600 — v11.4 宪法瘦身
system_text.push_str(&format!(
    "\n\n## Rules:\n{}\n",
    "安全第一: 不删除用户文件, 不执行未经用户确认的外部命令, 修改前先读文件。"
));
// ~50 tokens vs 原 v10.5 的 ~500 tokens 宪法全文
```

---

## 四、全量指标

| 指标 | v10.5 | v11.4 | 变化 |
|---|---|---|---|
| 测试通过数 | 163（90 实跑） | 181（181 实跑） | +18，声明实跑首次一致 |
| crate 数 | ~21 | 23 | +2 |
| .rs 文件 | 58 | 58+? | +2 新 crate |
| TODO/FIXME | 0 | 0 | — |
| API 壳 | 0 | 0 | — |
| FMT | ❌(8) | ❌(7) | -1 diff |
| CLIPPY | ✅ | ✅ | — |
| prompt 宪法 tokens | ~500 | ~50 | -90% |

---

## 五、门禁判定

```
真机三门:
  FMT    = RC=1 (7 diff) → ❌ 连续第6版
  CLIPPY = RC=0          → ✅
  TEST   = RC=0 (181/0)  → ✅ 首次声明=实跑

源码接线:
  subconscious gate 在生产路径 ✅
  experience store 在生产路径 ✅
  9/9 债务全清 ✅
  0 TODO/FIXME ✅
  0 API 壳 ✅
```

**❌ 不过闸。唯一阻挡项是 FMT_RC=1（7 diff，全在新增 crate/service 修改）。**

其余所有指标均为历史最佳——TEST 声明实跑完全一致是第一次，subconscious + experience 两个新 crate 不是空壳是完整实现。

---

## 六、执行方案

### 立即过闸

```bash
cargo fmt --all
cargo fmt --all -- --check  # 确认 RC=0
# 重跑真机 FMT=0 → 过闸 → 打包
```

### 打包

```bash
tar czf codex-rust-v11.4-final.tar.gz \
  --exclude=target --exclude=.workbuddy --exclude=.git \
  --exclude="*.tar.gz" \
  Cargo.toml Cargo.lock crates/ docs/ constitution.md governance.md
```

### v11.5 远期

| 项目 | 优先级 |
|---|---|
| EmbedFn 接入真实 OpenAI embedding API | P2 |
| CIV session_complete 事件 | P3 |
| 覆盖率 / benchmark | P3 |

---

*审计签名*: Code Audit Gatekeeper（源码级 + 真机）  
*真机 RC*: `FMT_RC=1`（7 diff） / `CLIPPY_RC=0` / `TEST_RC=0`（181/0，实跑=声明）
