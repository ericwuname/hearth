# R8 包D 卫生清理战报 v1.0（执行窗→顶层）

> **执行**：traecode 执行窗　**日期**：2026-09-07　**依据**：《R8双线收口·执行委托书》§一（签 3 已授权）+ 顶层签发件增 3
> **性质**：卫生清理，分钟级，零功能语义改动。

---

## 一、fmt 收编（顶层裁决 5：收编不还原）

- commit **e78f27b** `style: cargo fmt`——`crates/agent-core/src/context.rs`（44 行）+ `crates/codex-cli/src/render.rs`（5 行），均为验收窗已核实的纯 cargo fmt 格式化残留，零语义。
- 改动清单：`+33 / -16`，全部为换行/缩进重排，无标识符、逻辑、字符串变更。

## 二、双机引擎身份修复（委托书 §一-2，实际双机执行）

### .131（委托书点名机）

| 项 | 修复前 | 修复后 |
|---|---|---|
| `/usr/local/bin/hearth` | `hearth 0.2.23 (unknown)`，md5 `34f6f807` | md5 **`9ec07d9d`**（ebd2f52 构建） |
| `~/.bashrc:127` alias | `hearth→codex-r4 旧构建` | `hearth→codex-r6` |
| `bash -lc 'hearth --version'` | 0.2.23 (unknown) | **`hearth 0.2.25 (ebd2f52)`** ✅ 判据逐字达成 |

### .133（执行窗补充——包A 跑测机，同款病灶一并修复）

`bash -lc 'hearth --version'` 在 .133 同样解析到 `0.2.23 (unknown)`（/usr/local/bin/hearth md5 `34f6f807` + `~/.bashrc:128` alias 指 codex-r4）。包A 全部跑测在 .133，引擎身份留档纪律（R7 委托书沿用条款）要求跑测机环境健康，故按同款操作修复：

| 项 | 修复后 |
|---|---|
| `/usr/local/bin/hearth` | md5 **`9ec07d9d`**（ebd2f52 构建） |
| `~/.bashrc:128` alias | `hearth→codex-r6` |
| `bash -lc 'hearth --version'` | **`hearth 0.2.25 (ebd2f52)`** ✅ |

**偏差申报**：委托书 §一-2 仅点名 .131；.133 修复为执行窗依据"跑测机引擎身份健康"纪律的增量动作，操作与 .131 完全同款，如实呈报。

## 三、根目录清扫（顶层增 3）

- **移前零引用确认**：`git grep -l "_r7_\|_QUARANTINE"` 全部入库文件 → 命中 5 处，逐一核查：
  - `crates/observer/src/lib.rs:265` = 测试函数名 `test_r7_rebuttal_persists_without_mutating_rules`（`_r7_` 系函数名内下划线，误匹配，与脚本无关）；
  - `docs/hearth-cli-acceptance-record.md:19` = 引用同一测试函数名（同误匹配）；
  - 3 份 p0-usability 文档 = 签发件/委托书/验收报告对脚本清点的**历史记录性提及**，非 runbook 执行依赖。
  - **结论：零执行依赖** ✅
- **移动清单**：
  - `_r7_*.sh` **30 个**（签发件记 33，执行时实存 30，差额 3 个无从追溯——R7 期间可能已被个别脚本自清理，如实报数）→ `docs/data/r7-20260907/scripts/`（commit 394708e）；
  - `_QUARANTINE_undesired_209d/` **76 文件（0.7 MB）原样移入** `docs/data/r7-20260907/_QUARANTINE_undesired_209d/`，**零删除**。归属与最终处置**随本战报呈顶层裁**：目录内容为 v0.2.x 评审期材料（yaml 用例、rs 源码切片、历史 log、评审报告 doc），无入库主档对应物。
- **gitignore 陷阱处置**：21 个 `.log` 被全局 `*.log` 规则静默排除 → commit 432bca7 `git add -f` 补齐，**现 quarantine 76/76 全部入库**（"保留现场"要求优先于 gitignore 默认）。
- **清扫后根目录**：`_r7_*.sh` 0、`_QUARANTINE_*` 0 ✅

## 四、防再犯（签发件原文，即刻生效）

一次性脚本今后直接落 `.133`/`.131` 或 `.workbuddy/tmp/`，**不落仓库根目录**。本战报执行期间的全部通道脚本已按此落位 `.workbuddy/tmp/`（vm.py / fix131.py / fix133.py / diag133.py / probe133.py，不入库）。

## 五、commit 清单

| commit | 内容 |
|---|---|
| e78f27b | `style: cargo fmt`（fmt 收编，独立 commit 符合顶层裁决 5） |
| 394708e | 根目录清扫主体（30 scripts + 55 quarantine 文件） |
| 432bca7 | quarantine 21 个 `.log` force-add 补齐（76/76） |

包D 战报本体 commit 见本文件入库提交。

## 六、遗留与移交

1. quarantine 归属裁决权在顶层（本包只做原样保留）；
2. `.131`/`.133` 的 `~/.local/bin/hearth`（alias `hearth-cli` 指向物）未在本包范围内核查——如顶层认为 `hearth-cli` 也须入引擎身份纪律，另开小包；
3. 包A（R7-5）随即开工，.133 环境已就绪（engine = ebd2f52 构建）。

---
*执行窗（traecode）· 2026-09-07 · 依据：R8 委托书 §一 + 顶层签发件裁决 5/增 3 · 双机验证输出逐条实测*
