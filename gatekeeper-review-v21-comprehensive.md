# codex-rust v21 守门员综合审计（刀入鞘轮）

- 审计时间：2026-08-01 00:12
- 审计对象：v21 全部 6 份交付物（消债 + 设计决策 ×2 + 维护规程 + 报告 + CHANGELOG）
- 方法：源码 grep + VM 门禁 + wiring 14 条

---

## 一、源码级核实

| # | v21 声称 | grep 方式 | 结果 |
|---|---|---|---|
| 1 | embedding 死代码已删 | `grep main.rs` → `zhipu_embed_fn` 0 命中 | ✅ |
| 2 | reqwest 依赖已删 | `grep service/Cargo.toml` → 0 命中 | ✅ |
| 3 | **硬编码 key 清零（5 处）** | `grep -r "KEY_PATTERNS\|ark-" crates/` → 0 命中 | ✅ 比计划多清 4 处 |
| 4 | key 走 .env（dotenvy） | `grep main.rs` → `unwrap_or_default()` ×4 provider | ✅ |
| 5 | .gitignore 含 .env | `grep .gitignore` → `.env` | ✅ |
| 6 | wiring 14/14 | VM `codex-xray wiring` | ✅ |
| 7 | smoke 真实 | T00-smoke 42.3s PASS（deepseek 经 .env） | ✅ |
| 8 | 三份新文档 | grep 文件存在 + 内容核验 | ✅ |

---

## 二、关键发现

### 发现 1：超额消债——密钥清零

v21 计划删 1 处 ZhiPu key（embedding 函数内）。审计 grep 发现 provider fallback 还有 4 处硬编码——全部清除，统一走 `.env`。**仓库内零 hardcoded API key**。

### 发现 2：新运维依赖——VM .env

service 依赖 VM 上的 `.env` 包含 API key。丢失 = provider 401。**已文档化在 `maintenance-protocol.md` §6 和 gatekeeper D2**——运维约束明确但不阻塞。

### 发现 3：计划漏了 4 处 key，但最终清得比计划干净

审计 D1 记录了执行偏差，但偏差方向是正确的——发现了计划未覆盖的隐患并一并修复。

---

## 三、两份设计决策——证据链完整

| 决策 | 证据 | wiring 锚定 |
|---|---|---|
| 经验 = 降级通道 | v17 zhipu +20pt / v18 deepseek -5pt / v19 自适应开关 | `experience-adaptive-switch` |
| planner all_done → Write 门控 | v19 解剖（仅 grep+read→Done）+ v20 修复（0/8→2/3） | `all-done-requires-write` |

两份决策都含"维护须知"——明确列出**不许做的回退操作**和对应的 wiring 红线。后来的人读这份文档就知道"为什么不能动"。

---

## 四、维护规程——完整可操作

| 维度 | 覆盖 |
|---|---|
| 体检节奏 | 季度全量（基准+应力+回放+门禁）+ 按需增量 + 明确不跑 |
| 红线 | 4 条（通过率<85% / 应力>0 / wiring 断 / 造假）— 每条有阈值 |
| 红线响应 | 5 步（复现→分类→决策→验证→记录） |
| 新功能 checklist | 5 项（基准任务 + wiring + 单跑 + 测试 + CHANGELOG） |
| 年度回顾 | 趋势报告 5 维度（通过率/失败题/成本/债务/策略） |
| 环境/密钥 | VM 地址 + .env 位置 + 零硬编码纪律 |

**唯一缺口**：`docs/quarterly-baseline-q3-2026.md` 列在文档索引里但本交付物中未创建——不影响 v21 过闸，留维护期 Q3 体检时创建。

---

## 五、门禁数据

| 指标 | 数值 |
|---|---|
| 硬编码 key | **0**（v21 前 5 处） |
| 死代码 | embedding 整块已删（44 行 + reqwest + 开关） |
| cargo check | PASS |
| wiring | 14/14 ✅ |
| smoke | T00 PASS 42.3s |
| provider 注册 | 6/6（全部经 .env） |

---

## 六、闸门判定

```
🔴 = 0（key 清零 / check 过 / wiring 14/14 / 规程存在）
🟡 = 0（设计决策均含完整证据链，无空谈）
🟡 = 1（季度基线文件未创建——不阻塞，留维护期创建）
🔵 = 0
```

**✅ v21 刀入鞘轮通过守门员验收。**

计划承诺的消债 3 项实际完成 7 项（多清 4 处硬编码 key + reqwest 依赖 + .gitignore）。两份设计决策含完整证据链 + wiring 锚定。维护规程覆盖全部维护场景。

---

## 七、维护期启动确认

```yaml
状态: 已启动
生效版本: v21.0
Q3-2026 首次季度体检: 2026-10 月第一周
红线:
  - deepseek 20×2 < 85% → 问题分析
  - 应力场 panic > 0 → 修复或降级
  - wiring 断裂 → 恢复或重评
  - 数据造假 → 停线审查
按需增量:
  - 新功能 → 加基准题 + wiring
  - 修 bug → 受影响题 + 回放 + 应力
  - 模型升级 → 全量基线
```

---

*锻造九轮 v12-v20 证明了刀能砍。v21 把刀擦干净、编号入鞘、写了保养手册——从此每次出鞘都有体检数据等着，每次入鞘都有断言守着。*
