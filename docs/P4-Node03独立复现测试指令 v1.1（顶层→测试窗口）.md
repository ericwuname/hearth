# P4 Node 03 RC52 矩阵独立复现测试指令 v1.1（顶层 → 测试窗口）

> **日期**：2026-09-01　**派工**：顶层规划窗口　**执行**：测试窗口　**判读**：砺·评审（测试窗口只交数据，不自判）
> **v1.1 修订（07:10）**：①执行窗口宣布 Node 00-09 收工 → **附录 A 仪器修复单改由测试窗口实施**（diff 留痕、交砺复核后方可跑 C）；②补录 Node 09 caps 校准（Agnes 实测 128K）；③确认 Node 10-12 归测试窗口。
> **判据源**：`docs/core-revalidation/Node03-验收清单（砺·评审→测试窗口）.md`（砺出具，G/P/D 编号沿用该清单）
> **基线**：本机 HEAD `24ff85a`（Node 07-09 完成）｜tag v0.2.20｜Cargo.toml 0.2.20｜gate 455→459 tests
> **环境**：`.131`（driver 需真 PTY + 真 LLM + hearth binary，只能跑 .131）

---

## 0. 一句话任务

**用 SimUser PTY driver 独立复现执行窗口的 RC52 判定矩阵（A fresh ×5 / B contaminated ×5 / C resume ×3）**，验证其数据可复现性，交原始数据给砺判读。**当前 RC52 = CAUSE LIKELY 是执行窗口自跑自判的（4fdbda3），你的复现是它升格 CONFIRMED / 进 Node 13 修复批的独立前提。**

## 1. 背景 30 秒版

- 执行窗口已完成 Node 00-09：SimUser Stage 1（真 PTY driver + 13 检测器）→ Projection 审计 → RC52 因果矩阵（fresh 40% 失败 / polluted 100% / resume 67%）→ 归因 **decision 层 RC47 族主因 + 会话污染放大器** → Node 05/06 修复批（v0.2.20）→ Node 07-09 审计。**执行窗口已宣布收工（gate 455→459，双 VM 0.2.20 对齐）：Node 10-12（Stage 2 + Campaign）正式归你，Node 13 修复批回执行窗口（v0.2.21）。**
- **砺查出 🔴 阻断项**：C 条件的 session id 提取正则（`run_rc52_matrix.py:99`）恒失效，resume 的是空 id——**C 条件旧数据（67%）全部无效**。修复单已备好（附录 A）。
- **Node 09 caps 校准（判读注意，对矩阵无影响）**：COMPACT_DBG 实测 Agnes provider caps = **128K**（非宣传值 512K）→ 压缩触发 ≈ 实际窗口 15%，保守方向安全；caps 申报值待供应商核实。G-7 探明 planner dump 落盘位置时，注意 v0.2.20 起 tracing 已收敛到 `~/.config/hearth/diagnostics.log`。
- 你的任务 = 复现 A/B、实施附录 A 修复后补跑 C、探明三处仪器接线存疑项、交数据。

## 2. 铁律（五条，违反即数据作废）

1. **不改生产代码**：crates/ 一律不碰。tools/simuser/ 仪器**授权你按附录 A 实施**（v1.1 顶层裁量：执行窗口已收工，仪器属 dev 侧、不在冻结区，且被测对象是 crates/ 而非仪器，自证链不成立）。实施纪律：单独 commit、diff 全量贴进报告、**交砺复核通过后方可开跑 C 条件**；附录 A 之外的新问题只登记不修。
2. **不自判**：判定矩阵的归因结论由砺出具；你交付的是 run/capture/detect/archive + 现象记录。
3. **禁混表**：新数据落 `.131:~/fa/p4-rerun/`（新目录），与执行窗口的 `~/fa/p4/` 物理隔离；results.json 头部写明 tag、commit、binary sha256。
4. **原始终态优先**：每跑必须留全量 `.log` + `.events.json` 双件；分析层结论可被校正，原始 buffer 不可丢（D-5 终态标记不全的风险靠原始日志兜底）。
5. **失败即报**：任何单跑异常不吞进 error 字段了事——报现象、留日志、标记作废，由砺决定是否重跑。

## 3. 前置检查（P-1 ~ P-5，全绿才许开跑）

| # | 检查 | 方法 | 过线标准 |
|---|---|---|---|
| **P-1** | binary 对版本（E7 已三次复发） | `.131` 上 `bash -lc hearth --version` + sha256 比对本机 Cargo.toml | = 0.2.20 且 sha 与执行窗口记录（fc39c27c）一致；否则重建后再跑 |
| **P-2** | 测试环境模板 | `unset HEARTH_URL` + `export HEARTH_CGROUP_BASE=/sys/fs/cgroup/hearth` + `source ~/.cargo/env` | REPL 启动徽章正常（🔒 landlock+seccomp fail-closed） |
| **P-3** | 结果目录隔离 | 确认 `~/fa/p4-rerun/` 为空目录 | 无旧 results.json 残留 |
| **P-4** | **cgroup 绕过确认**（砺 P-4） | `run_rc52_matrix.py:38` 现设 `HEARTH_ALLOW_NO_CGROUP=1` | 先实测：在 P-2 环境下去掉该开关跑 1 个 A 样本——若 cgroup 正常工作（bash/glob/grep 可用、无 fail-closed 打死）→ **正式跑不带此开关**；确不可用才保留，且必须在 results.json 元数据与报告头显式标注"无 cgroup 环境" |
| **P-5** | driver 冒烟 | 1 次最小 REPL 会话（单轮任务） | 终态标记命中、session id 正常捕获、`expect` 无早发 |

## 4. 批次 1：A/B 条件复现（10 跑）+ 仪器接线探明

### 4.1 A 条件 ×5（fresh：全新 REPL，小任务完成后发继续型输入）

- 按执行窗口同构条件跑；逐跑记录：continue_terminal、步数、findings、耗时。
- **G-3 判据**：fresh 失败率与自测 40%（2/5）同向，允许 ±1 样本波动。

### 4.2 B 条件 ×5（contaminated：逐字重放 run-001..012 污染前缀后发继续型输入）

- **G-4 判据**：失败率 100%（5/5）；若 <5/5，如实记录——污染放大器结论将重估，**不得凑数重跑**。
- **G-10 附带**：核对硬编码 PREFIX 13 行与 `release/手工测试v0.2.18.txt` 原始输入的对应性，差异逐行登记（改写前缀来源属执行窗口修复单可选项，你只登记）。

### 4.3 仪器接线探明（G-7/G-8/G-9，只探不修）

| # | 任务 | 方法 | 交付 |
|---|---|---|---|
| **G-7** | planner prompt dump 是否真落盘、落哪 | `HEARTH_DEBUG_PLANNER_INPUT=1` 已在矩阵 env 开启；跑 1 个样本后全盘查 `plan-prompt.txt` / `graph.txt` / `compact-summary.txt`（cwd 与 HEARTH 状态目录） | 落盘路径或"未落盘"结论 + 文件样本 1 份 |
| **G-8** | TaskGraph fingerprint 采集能力 | analyzer 0 命中已证——**以原始数据代偿**：从 events.json/log 提取每轮 TaskGraph 结构摘要（节点数/签名变化） | 每跑的 graph 演化简表 |
| **G-9** | terminal 字段未聚合 | driver 原始 terminal 事件在 events.json——从原始数据补出每跑终态分布 | 三条件终态计数表（completed/failed/timeout 均计数） |

> G-8/G-9 的仪器缺口登记给执行窗口 Node 10 仪器增强批；本次以原始数据代偿，不影响复现有效性。

## 5. C 条件：BLOCKED（旧数据已作废）

- 旧 resume 67% **不得引用、不得混表**（G-5）。
- 触发条件 = **你按附录 A 实施 R-1/R-2/R-3（R-4/R-5 顺手），diff 单独 commit 并交砺复核通过**。落地前**不跑 C**——跑了也是废数据。
- 实施顺序建议：先跑红测试（构造 8 位短 id 终端输出 → 旧正则必 None）留红证据 → 修复 → 红转绿 → 交砺复核 → 开跑 C。

## 6. 批次 2：C 条件补跑 ×3（修复落地后）

- **G-0**：三个样本 session_ids 均为 8 位非空 hex，且 resume 后确认"会话已恢复"；
- **G-0b**：人为构造一次空 id → 断言抛错作废而非静默执行；
- 补跑后出完整 13 跑 results.json；**G-6**：终态分布不得全 timeout（全 timeout → 疑似 DRIVER-INDUCED，如实记录交砺）。

## 7. 交付物（交砺判读 + 抄送顶层）

1. `.131:~/fa/p4-rerun/`：13 跑 `.log` + `.events.json` + `results.json`（头部含 tag/commit/sha/时间/cgroup 状态声明）；
2. `G-0~G-10 逐条打勾表`（判据源 = 砺验收清单 §2，红绿如实，不解释不辩护）；
3. 差异报告：任何与执行窗口自测值不同向的结果，附原始日志路径（**G-3/G-4 不同向 = RC52 暂缓升格**，这是正常科学结果，不是事故）；
4. G-7 dump 路径结论 + G-8/G-9 原始数据代偿表；
5. 新发现问题登记单（只登记，不修）。

## 8. 禁止事项

- 禁改 crates/；tools/simuser/ 仅限附录 A 范围且须 diff 留痕交砺复核；
- 禁因结果"不好看"而重跑凑数（同构条件重跑须在报告声明次数与原因）；
- 禁引用 `~/fa/p4/` 旧数据参与判读；
- 禁在报告里使用"已验证/确认修复"等自证表述——你交数据，砺出验收。

---

## 附录 A · 仪器修复单（v1.1 起：由测试窗口实施，diff 留痕交砺复核）

> 来源：砺·评审验收清单 §1.4 + §4。原定执行窗口实施；因执行窗口已宣布 Node 00-09 收工，顶层裁量改由测试窗口按单实施（若用户选择仍转执行窗口，本单同样适用）。

| # | 位置 | 修复 | 级别 |
|---|---|---|---|
| R-1 | `run_rc52_matrix.py` run_c 96-102 行 | `uuid = d0.run.session_ids[-1] if d0.run.session_ids else ""`；`if not uuid: raise RuntimeError("C-condition: session id 提取失败，本样本作废（不得静默 resume 空 id）")`；删除失效正则 | 🔴 阻断 |
| R-2 | `driver.py:28` | `TURN_TERMINAL_MARKERS` 补全 give_up / deadline_exceeded / cancelled 对应终端文本（先实测 .131 上三种终态的真实输出再写标记）；映射逻辑 `:147-149` 同步扩展 | 🟠 高（防 DRIVER-INDUCED 归因污染） |
| R-3 | `run_rc52_matrix.py:106` | `task_terminal` 硬编码 `"chat"` 改为从 d0（chat 轮）实际终态读取 | 🟡 中 |
| R-4（可选） | `run_rc52_matrix.py:17-31` | PREFIX 改为从 `release/手工测试v0.2.18.txt` 或既有 log 读取，保逐字重放可证 | 🟡 中（G-10） |
| R-5 | `driver.py:117-118` 死代码 / `:119` import 位置 / `:75` `preexec_fn` → `start_new_session=True` | 顺手清 | ⚪ 低 |

**修复纪律**：只动 tools/simuser/ 仪器，不碰 crates/；每处修复带红测试（R-1 的红 = 构造 8 位短 id 终端输出，旧正则必 None）。

---
*判读与 RC52 升格裁决归砺·评审。执行窗口已于 Node 09 收工（Node 00-09 全落，gate 455→459）。复现完成且砺验收通过后，测试窗口按总包顺序接 **Node 10（SimUser Stage 2：personas/scenarios 空目录落地 + 检测器补至 15 + G1 相关系数门禁）→ Node 11-12 Campaign（30 runs，另行派工细则由砺按判据出具）**；Node 13 修复批回执行窗口（v0.2.21）；Node 15 Freeze Candidate 人工+外部 AI 双窗终审。*
