# FREEZE CANDIDATE · simuser-status（Node 15 八文件之五）

> **执行窗 2026-09-03**｜基线 tag `v0.2.23`（commit `117a947`）｜**COND-2 口径：检测器 16（13+3），顶层裁定不凑 15**
> **Q11（强制）**：本测试台全部语料来自**单一人类主体**（corpus_seed_dataset.json v2-deep10，n=4681 脱敏用户话语）。一切成功率指标只能表述为"该单主体语料模拟下的成功率"，禁止"真实用户成功率"。

## 1. 资产清单（`tools/simuser/`，双 VM 已同步）

| 资产 | 状态 | 说明 |
|---|---|---|
| 相关矩阵 | ✅ `correlation_matrix.json`（`corr-rules-v1.1`） | 273 条种子话语、7 维、Cramér's V；**G1 CONDITIONAL PASS + 修正回执（0.065→0.1047）待砺复核（G1-C3）** |
| 联合分布采样器 | ✅ `sampler.py` | 分层采样（模态→类别→逐字种子）；拒绝采样稀释缺陷已修（B4 自测抓出）；稀有类强制放大 3×3pt |
| persona ×12 / 场景 ×10 | ✅ `personas/` `scenarios/` | `gen_assets.py` 从语料+矩阵派生（非手写）；Node 12 七类 = s01-s07 + 3 战略类 |
| analyzer | ✅ 16 检测器 | 13 + 3（multi_intent_drop / paste_error_probe_quality / longpaste_truncation）；B3 自检 6/6；真实日志回归命中（v0.2.18 3 处探针质量差） |
| B4 验收 | ✅ `validate_assets.py` | 12 PASS / 0 FAIL / 2 MANUAL（砺 §5.4-5 已裁定） |
| PTY driver | ✅ `driver.py` | 真实 PTY（三版失真陷阱规避在档）；**已知交付缺口：scenario 驱动接线未做**（campaign 报告 §6.3.1） |
| 矩阵 runner | ✅ `run_rc52_matrix.py` | OUT=`~/fa/n14-final`（COND-1 已修正） |

## 2. 数据产出（全部已归档，索引见 evidence-index.md）

| 数据 | 规模 | 去向 |
|---|---|---|
| RC52 三臂 campaign（改道后定位=L1 pre-fix 基线） | 40 runs + keystone 系列 + 转移矩阵 n=227 | 砺 v1.2 终版报告 |
| 顶层熟悉版盲测 | 42 轮 4304 行 | behavioral-status §2 |
| A/B 首正式轮 | A 4✗1✓ / B 3✓2✗ | behavioral-status §2.1 |
| **A/B 正式轮（逐字包，BP-2 全新 session）** | **A 2✓8✗ / B 4✓1✗** | behavioral-status §2.2 |
| Phase D 八实验 + 补测 | D1-D8 + P-D5/P-D1/P-D6/P-D3 | 两份报告（执行窗→砺判读） |

## 3. 复测判据（供 L1 决胜局 post-fix 对照，campaign §6.1）

| 指标 | pre-fix | 修复目标 |
|---|---|---|
| give_up (0,0,0) 占比 | 72.8% | 显著下降 |
| 续作型 P(✓\|上轮✗) | 7.0% | 显著上升（最灵敏单指标） |
| 续作型 P(✓\|上轮✓) | 85.0% | 不下降（防一刀切放宽） |
| B 臂 ✓率 / C 续作 ✓率 | 27% / 10% | 上升 / >0 |
| A 臂 ✓率 | 100%（伪通过） | 与 B/C 趋同（非维持 100%） |
| L1_B3_NONPRODUCT_RESCUE 命中 | —（新增信号） | >0 且终态带 UNVERIFIED |

## 4. 开放项

- G1-C3 复核（砺）→ 通过后 OD-3 闭环；
- driver scenario 驱动接线（§6.3.1，交付缺口）；
- 检测器误报治理：`truncated_output` 在长日志上 268 命中偏噪（v0.3 调阈值）。
