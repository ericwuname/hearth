# P4 测试窗口 · T0 环境锁定与阻塞报告（砺·评审 代行测试窗口，2026-09-01）

> 角色：用户已裁定「本会话里我就是测试窗口，只有我跑测试」。以下按测试窗口纪律执行：只跑、不改 `crates/`、数据落 `~/fa/p4-rerun/`、交原始数据判读。
> 方法：不信指令书自述，逐项 `bash -lc` 实跑核验（红线：锚点引用前必须重新核对）。

---

## 一、T0 环境锁定结果

| 检查 | 指令书要求 | 实测结果 | 结论 |
|---|---|---|---|
| **P-1 版本** | `hearth --version` = 0.2.20 | `hearth 0.2.20 (unknown)`，binary 09-01 06:30 重建 | ✅ 通过 |
| **P-1 sha** | 前 16 位 = `fc39c27c` | `fc39c27c96a95af5cbf0…` | ✅ 精确吻合（E7 二进制落后 source 未复发） |
| **P-3 目标目录** | `~/fa/p4-rerun/` 为空 | 目录不存在 | ⚠️ 待创建（非阻断） |
| **cgroup** | v2 可用则不带 `ALLOW_NO_CGROUP` | `/sys/fs/cgroup/hearth` 存在，controllers: cpuset cpu io memory pids | ✅ v2 可用，P-4 走「不带开关」路线 |

**结论：binary 基线可信，T0 硬门禁通过。**

---

## 二、🔴 阻塞：指令书工具链与数据资产在 `.131` 整体缺失

指令书（v1.1 §9 附录 A + campaign v1 §4）假设一套 PTY 工具链，但 VM 实测**全部不存在**：

| 指令书声称 | 实测 |
|---|---|
| `tools/simuser/driver.py`（`HearthDriver`/`send_and_wait_turn`） | 全 VM `find` 无 `driver.py`；`~/codex/tools/simuser` 不存在 |
| `run_rc52_matrix.py:99` C 条件正则 bug | 全 VM 无 `run_rc52_matrix.py`；仅 `bench/matrix_v13/14/15.py`（v15 = deepseek 稳定性 benchmark，非 RC52 矩阵） |
| `metrics.yaml`（口径冻结 v1.0） | VM 上无此文件 |
| 黄金集 `release/手工测试v0.2.18.txt`（41 条输入行） | VM 上不存在（指令书自承「release/ 不随源码同步，VM 上没有」） |
| `personas/` + `scenarios/`（块 C 采样） | 全 VM 无此目录 |
| `~/codex` 是 git 仓库（附录 A「单独 commit」） | `~/codex` **非 git**；VM 上唯一 git 在 `~/住户反馈/projects/codex-rust-v1.0-final/.git`（Windows 同步侧） |

**真实部署的测试台（执行窗口实际用的）**：
- `hearth chat '<prompt>' --budget N --acceptance '…'` 直接驱动（见 `~/fa/run_ab.sh`、`run_rc49.sh`）；
- `~/fa/analyzer.py`（13 检测器）；
- `~/codex/bench/matrix_v15.py` 等（稳定性 benchmark，非 RC52 矩阵）；
- Node 00-09 全部日志在 `~/fa/`（`ab_A1..A5.log`、`ab_B1..B5.log`、`p4_n07a.log` 等）。

**VM 上有 6 套并行 checkout**：`~/codex`、`~/codex_work`、`~/codex_t`、`~/codex_new`、`~/codex_new5`、`~/住户反馈/projects/codex-rust-v1.0-final`——存在改错树风险。

### 影响
1. **T1 附录 A 仪器修复无法按指令书执行**：目标文件 `run_rc52_matrix.py`/`driver.py` 不存在 → 所谓「C 条件正则 bug」是规划文档里的构想，不在部署物中。无需「修仪器」，因为仪器未部署。
2. **T2 块 B 黄金集回放无法执行**：黄金集不在 VM，需用户/顶层提供 `手工测试v0.2.18.txt`。
3. **T2 块 C persona 分布对齐无法执行**：`personas/`/`scenarios/` 为空/缺失。
4. **「单独 commit」纪律无法满足**：VM 工作树非 git。

---

## 三、仍可执行的「核心 RC52 复现」（不依赖缺失资产）

`hearth` CLI 本身工作正常，`hearth chat --budget --acceptance` 可直接驱动。因此 **RC52 的核心现象（fresh vs contaminated vs resume）可用活 CLI 直接复现**，无需任何缺失文件：

- **A fresh ×N**：全新 REPL 会话，小任务完成后发继续型输入 → 统计 decision 层是否误判「已完成」。
- **B contaminated ×N**：单会话先喂干扰前缀再发继续型输入 → 统计失败率。
- **C resume 行为**：`hearth resume <id>` 直接观察是否恢复旧 goal（即指令书 T3 陷阱）。

这部分能直接回答指令书的 **Q1（v0.2.20 是否消除用户可见缺陷）** 与 **Q2（RC52 复现率定量）**，是最高价值、最低资产依赖的部分。

---

## 四、待用户/顶层裁决的范围问题

指令书不可按字面执行（工具链与数据资产缺失）。请定夺以下路径之一：

- **A（推荐·低成本）**：我现在就用活 `hearth chat` CLI 跑**核心 RC52 复现**（A/B/C 三条件，各 5/5/3 跑），直接回答 Q1/Q2；黄金集回放与 persona 分布待你提供资产后再补。成本远低于 ≥30 runs 全 campaign。
- **B（先对齐再烧钱）**：我先出「指令书 vs VM 实况」差距清单交顶层，由顶层补黄金集 / 确认 harness 意图后，再决定全 campaign 怎么跑。
- **C（重建工具链）**：我按指令书 spec 从零重建 `driver.py` + `run_rc52_matrix.py` + `metrics.yaml` 部署到 `~/fa/`，再严格按 T1-T3 跑——工程量最大，但让指令书真正可执行。

> 预算提示：你此前说「真没钱了」。A 路径最省；全 campaign（≥30+13+21 样本）是多小时多美元量级。建议先 A 出核心结论。
