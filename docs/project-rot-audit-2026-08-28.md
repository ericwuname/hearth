# Hearth（hearth-rs）项目腐化审计报告

> **审计日期**：2026-08-28
> **审计目标**：截至今日的全项目健康度 / 腐化迹象（代码、版本、文档、归档、技能）
> **审计方法**：守门员源码审计法（不信报告信源码、硬指标实算、生产路径必接线）
> **基线**：commit `f0b4a54`（ux-polish-01），225 commits，27 crates

---

## 0. 结论速览（分层级）

| 维度 | 状态 | 结论 |
|---|---|---|
| 源码整洁度 | ✅ 健康 | 无 TODO/FIXME 堆积、无空壳、无 `assert!(true)` 假测试、crate 目录与 manifest 完全对齐（27/27） |
| 编译健康 | ⚠️ 无法本地核验 | 本沙箱 cargo 仅是 rustup 代理（无 RUSTUP_HOME/rustc），**未能本地编译**；以既有 VM 门禁为准（v0.2.3 VM 339 passed 全绿） |
| 版本控制 | 🔴 漂移风险 | 14 文件 modified + **62 文件 untracked（39 md / 10 gz / 9 txt）** 从未提交；master 分支滞后当前 78 个提交 |
| 归档堆积 | 🔴 未管制 | `.workbuddy/` 下 **21 个无标注 tarball（codex_*.tgz，共 14.3 MB）**，难追溯、难审计 |
| 文档/记忆漂移 | 🟡 代谢滞后 | 多处版本值过时（如"24 crates"实 27、"constitution 981 字符"实 3488）；已发现并纠正 |
| 技能库 | ✅ 健康 | 5 个用户级技能，职责边界清晰、无重复/冲突 |

**总体判断**：代码本体腐化度极低（这是项目最稳的资产）；最大风险在**版本控制漂移**与**归档堆积**——属"知识/数据丢失风险"，非代码腐烂。

---

## 1. 审计方法与局限（诚实声明）

- 实测手段：`git log/tag/diff`、`grep` 源码锚点、`find`/`wc` 实算指标、`cargo check` 尝试。
- **关键局限**：本沙箱的 `cargo`（`~/.workbuddy/cargo/bin/cargo.exe`）与 `rustc` 仅是 rustup 代理，`RUSTUP_HOME`/`CARGO_HOME` 未配置、无实际工具链，**无法在本机编译**。因此"编译警告堆积""未使用代码（cargo 告警）"维度无法本地核验。
- 替代证据：项目权威编译健康以 **Linux VM 门禁**为准——v0.2.3 发布记录载 VM fmt/clippy/`cargo test` 全绿、339 passed。对该证据的独立性仍需用户在 VM 开机后复核（详见 §5）。

---

## 2. 源码整洁度（✅ 无腐化）

| 检查项 | 结果 | 证据 |
|---|---|---|
| TODO / FIXME / XXX / HACK | **0 处真实待办**（grep 命中 3 行，均为 `error[EXXXX]` 文档误命中与 `read_lints` 注释） | `grep -rn 'TODO\|FIXME\|XXX' crates` |
| 空壳 / 超小文件 | 无（最小非空文件 `codex-cli/src/main.rs` 5 行、`service/src/session.rs` 3 行，均为正常委托/重导出） | `find -cve '^\s*$'` |
| 假测试（`assert!(true)`） | **1 处，且为 fixtures 字符串**（`project-xray/src/facts.rs:170` 测试输入样例），非真测试 | `grep 'assert!(true)'` |
| crate 孤儿 / 缺失 | 无。`Cargo.toml` members 与 `crates/` 目录 **27/27 完全对齐**，无漏测/无孤儿 | members vs `ls crates/*/` 比对 |
| 分支命名 | ✅ 当前 `ux-polish-01` 无斜杠（符合铁律；无 loose-ref 损坏风险） | `git branch` |

**结论**：代码层面的"腐烂"信号（TODO 堆积、死代码、接线当功能、零断言测试）在本项目基本不存在。这是项目护城河。

---

## 3. 版本控制漂移（🔴 主要风险）

### 3.1 工作区未提交物（数据丢失风险）

`git status` 当前：**14 个 modified + 62 个 untracked**。untracked 构成：

- **39 个 `.md`**（大量审计/验收/规划文档，含本次前一日我生成的 `project-comprehensive-review-2026-08-27.md`）
- **10 个 `.gz`**（bench tarball 等）
- **9 个 `.txt`**
- 另含 `target/` 与 `bench/syscall_probe/`

**风险**：这些文件均在项目目录（`Desktop/codex-rust-v1.0-final`），绝大多数未纳入版本控制。**一旦磁盘故障或误删，知识/审计证据将不可逆丢失。** 它们本身是成果，却被"游离"在 git 之外。

### 3.2 master 分支僵尸化

- `master` 停在 `0b159c5`（demo-v21j-frozen，2026-08 早期），`ux-polish-01` 领先 **78 个提交**且从未合并回 master。
- 后果：若任何 CI / 协作者 / 发布脚本基于 `master`，会拿到过时代码；`master` 已成为"冻结幽灵"，无人维护。

**建议**：
- 对 62 个 untracked 中已成熟的文档，**选择性提交**入库（先备份 review，再 commit，不进 trash）；
- 将 `ux-polish- 01` 的成果合并回 `master`（或改用 `main` 作为真实开发线），避免长期分叉。

> ⚠️ 我**不会**对这些文件执行任何移动/删除/清理动作，除非你显式确认具体文件清单。如需我帮你分批提交，请明确目标目录与标准。

---

## 4. 归档堆积（🔴 未管制）

`.workbuddy/` 目录（**项目数据、非临时缓存，禁止删除**）下发现：

- **21 个无标注 tarball**：`codex61.tgz` / `codex61b.tgz` / `codex61c.tgz` / `codex_6a.tgz`…`codex_v41.tgz` 等，命名无规律、无 manifest，**共 14.3 MB**。
- 疑似：VM 传输快照 / 备份 / 技能导入的中间产物，但**无用途标注、无时间说明、无索引**。

**风险评估**：这类"未管制归档"是典型腐化源——难以追溯用途、难以审计、易混入旧/损坏版本。但 `.workbuddy` 受保护，**绝不能 `rm`/通配删除**。

**建议**（需你确认后才动作）：
1. 先告诉我这批 tarball 的来历（VM 快照？备份？）；
2. 如是备份，统一重命名为 `<日期>-<用途>.tgz` 并建 manifest；
3. 确认哪些可回收；我会用 trash/备份机制，**绝不直接删除**。

---

## 5. 文档与记忆漂移（🟡 代谢滞后）

- 既有文档多处版本值过时（非代码腐化，但会误导后续读者）：
  - 记忆中"**24 crates**" → 实际 **27**（新增 agent-runtime / project-sync / observer）；
  - "**constitution 981 字符**" → 实际 **3488 字节**（已扩充）；
  - "**v23 基线 1255498**" 与 git 实际 commit 不符（git 实为 `5ec25e9` / `f0b4a54`）。
- `docs/requirements-ledger.md` 仍标注 observer "无消费者" 为已知历史债务（可接受，已标注）。
- docs/ 目录偏大（185 篇，命名前缀聚类：hearth×32 / acceptance×22 / self×8 / gemini×8 / audit×8…），存在重复感，但命名约定尚可，尚未到需重构程度。

**建议**：每次 localStorage 文档变更时顺手把文档里的"版本数/字符数/commit"等硬编码事实改为"见 git tag / 实算"，避免复述过时值。

---

## 6. 技能库（✅ 健康，1 处可改进）

用户级技能（`~/.workbuddy/skills/`）：`aihot`、`code-audit-gatekeeper`、`codex-vm-test`、`multi-advisor-sequential-review`、`proposal-stress-test` —— 5 个，职责边界清晰、无重复/冲突/重叠。

**可改进（供你决定是否整理）**：`code-audit-gatekeeper` 未声明"cargo 在部分沙箱仅 rustup 代理、可能不可用"——本次审计被迫改用 VM 门禁证据才完成。若将来复用该技能，应在 SKILL.md 补一条 fallback：「本地 cargo 不可用时，以项目 VM 门禁日志 + 源码静态检查作为编译健康代理」。

---

## 7. 整改清单（按优先级）

| 优先级 | 项 | 动作 | 前提 |
|---|---|---|---|
| **P0（数据风险）** | 62 个未跟踪文件 | 备份后可选择性 commit；绝不 `rm`（OS trash / 备份机制） | 需你确认哪些入库 |
| **P0（数据风险）** | 21 个 `.workbuddy/*.tgz` | 先确认来历，再命名/归档/回收（trash 非删） | 需你确认 |
| **P1（协作）** | master 僵尸分支 | 合并 ux-polish-01 或改 main 为开发线 | 你拍板策略 |
| **P2（整洁）** | 文档硬编码事实过时 | 顺手改为引用 git tag / 实算 | 随时 |
| **P2（技能）** | code-audit-gatekeeper 补 fallback | 若你要求则修 | 你确认 |

---

## 8. 最终判断

1. **代码本体：干净**。无 TODO 堆积、无空壳、无假测试、crate 接线完整——审计方法论要求的"空壳/未接线/接线当功能"三类 rot 在源码层均未命中。
2. **最大的真实风险是"知识资产游离"**：62 份未提交文档 + 21 份无标注归档 + master 落后 78 提交，构成不可逆丢失隐患，而非代码腐烂。
3. **编译健康无法本地闭环**：本沙箱工具链缺失，标量于 VM 门禁（339 passed）。**该证据需用户在 VM 开机后独立复核**，不可视为已验证事实（遵循"地基稳"量化门槛须真机复跑的原则）。
4. **下一步**：建议先处理 P0（备份 + 提交 + 归档确认），再谈架构级工作。我不会在你未明确确认前对任何文件动手。

---

*本报告全部结论基于 2026-08-28 源码直读与 git 实测，编译告警维度受工具链限制已如实披露。*

---

## 9. 处置结果（2026-08-28 守门员复核 + 用户拍板"1办 2办 3按B"）

> 复核勘误两处：①本报告头注"基线 f0b4a54"与自身实测矛盾——"78 commits 滞后"仅对当日 HEAD（`334739b`，v0.2.6）成立；②编译健康引用的 v0.2.3 339 passed 已过时——同日守门员 VM 直读 `~/t_gate.log` 实证 **v0.2.6 = CLIPPY_RC=0 / TEST_RC=0 / 363 passed 全绿**，"待复核"项闭环。

| 项 | 用户决定 | 处置 |
|---|---|---|
| 1. 未跟踪文件入库 | ✅ 办 | docs/ 全部 md + release/ 手工测试记录 txt + 综合评审 md + bench 资产 → 分批 commit；`target/`、`release/*.tar.gz`（11 个共 72.2 MB）补进 .gitignore 留盘不入库；**9 个 crates/*.rs modified 为执行窗口 R2-D 在制品（含批示 6 next_action_deterministic 实现），不代提交，留执行窗口随 R2-D 交付** |
| 2. 21 个 tgz 归档 | ✅ 办（登记不删） | 已建 `.workbuddy/archive-manifest.md`（文件/大小/日期全登记，最早 2026-07-29 = 基准期产物）；回收须用户逐项确认 + trash |
| 3. master 僵尸分支 | ✅ 按方案 B | **`ux-polish-01` 定为开发主线**，master 不做日常合并、仅在发版时打 release tag 指向主线 commit；本决定随本节入库存证 |

密钥安全：入库前全库扫描，新 Agnes key（cpk-）0 命中；命中的均为审计文档中的旧 key **前缀**引用（8 字符，不可利用，key 本身已弃用）。
