# Hearth P5-FOUNDATION-01 骨架地基加固施工总包 v1.0（顶层定版）

> **日期**：2026-09-01　**派工方**：顶层规划窗口　**执行方**：执行窗口（段 1 纸面 / 段 2 施工）
> **依据**：`docs/Hearth 骨架层对标成熟 harness：十轮深挖与地基裁决（砺·评审）.md`（下称"十轮深挖"）§11 加固清单 + `docs/Hearth 三层摸底：现状 · 可达 · 理想（砺·评审）.md`（下称"三层摸底"）桶 B 骨架项
> **用户裁决（2026-09-01）**："我的想法是修，而且是修完——骨架层是发动机，现在发动机还不稳。"
> **基线**：本机 HEAD `7c21805`（P4 Node 02 已落）｜Cargo.toml v0.2.19｜464 tests / 0 fail
> **supersedes**：无（新增）。与 P4-REVALIDATION-01 并行存在，**排期互斥关系见 §2**。

---

## 0. 一页纸总纲

| 项 | 内容 |
|---|---|
| **目标** | 把地基三档中的 🔴 崩溃安全（C-）修到 B，🟡 对抗安全（C+）修到 B-，顺手补三个骨架校验空洞 |
| **范围** | **必做 S1-S5**（十轮深挖 §11）+ **A3 提必做**（run_local 集成测试，作为 S1-S3 的验收载体）+ **T 级三项**（三层摸底 C-1/C-2/C-3，骨架校验空洞） |
| **不做** | D1-D5（追工具数/MCP/拆 loop.rs/landlock 全树/bwrap 三层）；A1（bash 进程复用）/A2（fuzz）后置 v0.3 backlog |
| **排期** | **两段式**：段 1 = 零代码准备（现在可下发，与 P4 并行）；段 2 = 真施工（**P4 Node 15 CANDIDATE + 双窗裁决通过后**开工） |
| **版本** | 施工产出 v0.2.22（S1-S4 批）→ v0.2.23（S5+T 批）。**v0.3.0 保留给能力线**（砺阶段盘点 §3.3），本包不占用 |
| **红线** | 段 2 开工前零 crates/ 代码改动；N12-N18 每步先红后绿；涉及冻结区的项必须走 N10 冻结修订 RFC |

## 1. 为什么现在不并行施工（顶层裁决记录）

1. **冻结区约束**：S4/S5 动 `sandbox/lib.rs`、C-1/C-12 动终态语义——全部在 Core Freeze 边界内（事实模型含沙箱）。P4 §17 禁令期间改它们 = 违反自家纪律。
2. **热点文件冲突**：`loop.rs` 是 P4 Node 05/06/13 的主战场；`run_local.rs` 与 P4 Node 03/12 交集。并行施工必产生合并冲突与归因污染。
3. **验收尺子还没造完**：S1-S3 的判据（崩溃注入、恢复语义）需要 SimUser/集成测试承载，而 SimUser Stage 1 是 P4 交付物。用 P4 造的尺子验收 P5，正好衔接。
4. 用户判断"估计时间还很长"——这段时间正好把段 1 做扎实，段 2 开工即冲刺。

## 2. 排期模型（两段式）

```
现在 ──────────────► P4 Node 15 CANDIDATE ──► 人工+外部AI双窗裁决 ──► CONDITIONAL FREEZE（v0.2.21）
  │                                                              │
  ├─ 段1：N00 规格冻结 + N01 裁决材料（零代码，可立即下发）        │
  │                                                              ▼
  └──────────────────────────────────────────── 段2：N10 → N11 → N12(S1) → N13(S3) → N14(S2)
                                                    → N15(S4) → [v0.2.22] → N16(S5) → N17(T级) → [v0.2.23] → N18 回归终验
```

**段 2 开工前置条件（三条全满足）**：
- P4 Node 15 产出 CORE FREEZE CANDIDATE；
- 人工窗 + 外部 AI 窗双签通过（签发 CORE FREEZE v2 · CONDITIONAL，含 DG-1/DG-2）；
- P4 campaign 数据落盘（N16 的 S5 裁决要用）。

若 P4 双签不通过：先处置 P4 暴露项，本包段 2 顺延（段 1 成果不作废）。

## 3. 冻结修订流程（CONDITIONAL FREEZE 下的合规路径）

本包四项（S4/S5/C-1/C-12）触碰冻结区。按顶层已裁定原则："冻结不等于永不改，改走冻结修订流程"。N10 节执行：

1. **逐项申报 RFC**：改什么文件、动事实模型哪一层、为什么必须动（引用十轮深挖锚点）、回归影响面；
2. **双签局部重开**：人工窗 + 外部 AI 窗对每条 RFC 单独批准（不是整包批准）；
3. **回归证据随附**：每个 RFC 项的修复必须带"先红后绿"测试 + 464 存量全绿证据；
4. **台账留痕**：每条 RFC 编号入总账 v3（FZ-RFC-xx），冻结文件附"修订记录"节。

## 4. Node 清单

### 段 1 · 零代码准备（现在可下发，与 P4 并行，不动 crates/）

#### N00 · 设计规格冻结（纸面）

产出 `docs/p5-foundation/specs.md`，对 S1-S5 + T 级逐项写：

| 项 | 规格要求 |
|---|---|
| **S1 工具层原子写** | 统一 `write_atomic(path, content)`：同目录 tmp + write + `sync_all` + rename；`edit.rs:141-162` 分块逻辑重写为"分块写 tmp、最后 rename"，原文件在成功 rename 前永不打开写句柄；`patch.rs:113/151`、`edit.rs:162` 短路径同样收口。**接口签名、错误语义（rename 失败→报错且原文件完好）、白名单测试用例清单** |
| **S3 增量 checkpoint** | 每 turn 结束（或每 N=5 步）append 事件到 session JSONL（对标 Claude Code "write as events occur"）；`save_snapshot` 从 `run_local.rs:501-505` 的 `Ok` 分支迁出为循环内例行调用；kill 后 resume 恢复到最近 checkpoint。**N 值、事件 schema、恢复语义规格** |
| **S2 改前快照+回滚** | 写操作（edit/patch/write_file）执行前 copy 原文件 → `~/.config/hearth/snapshots/<sid>/<seq>_<basename>`；新增 `hearth rollback <sid> [seq]` CLI。**快照保留策略（上限/清理）、rollback 交互语义** |
| **S4 env_clear** | `lib.rs:1078-1085` 加 `.env_clear()` + `.envs(最小白名单)`：PATH/HOME/LANG/TMPDIR + 既有 env_vars_clone。**白名单枚举表**（对标 dev 路径 `NoopSandbox:185` 语义） |
| **S5 出网收口** | **两步走**（顶层裁决，见 §5） |
| **C-3 宪法回退告警** | `constitution.rs:38` FALLBACK 路径加 `warn!` + 结构化事件；`sanitize()` 截断处加标记（顺带修三层摸底 A4 隐患） |
| **C-2 代价对账** | 工具实际耗时 > 声明 `declared_timeout` × 1.2 → 产生 `timeout_declaration_drift` 事件（观测级，不拦截）；bash.rs:89 的 610s 与实测 869s 的差值成因一并核查 |
| **C-1+C-12 终态验证标注** | ⚠️ **顶层设计裁定：Terminal 九态枚举不动**（冻结区最小侵入）。TaskResult/执行报告增加 `verification: VERIFIED\|UNVERIFIED` 字段：run 全程存在 ≥1 条"实际验证命令"（读/列目录不算，须实测目标行为）→ VERIFIED，否则 UNVERIFIED；投影层对 UNVERIFIED 的 completed 渲染为"目标达成（未验证）"，禁止裸"目标达成"。与三层摸底 C-12 的 `completed_unverified` 判据等效，但不动状态机 |

同时产出：**每项"先红后绿"fixture 设计表**（红测试输入 → 预期失败断言）。C-1 的 fixture 直接用 `release/手工测试v0.2.19.txt` run-001（37 步零验证）作语料（三层摸底桶 A-3）。

#### N01 · S5 方案裁决材料（纸面 + 只读实测）

产出 S5 决策备忘录，供顶层在 N16 开工前定案：

- 方案 ① seccomp 硬拒（`connect/bind/sendto/sendmsg` 拒绝，AF_UNIX 豁免，对标 Codex）：**注意须推翻 P3 Node 02"防线 B=不加位"裁定**，总账留痕；实测影响面 = 沙箱内 bash 联网任务（pip/cargo 拉依赖等）全灭清单；
- 方案 ② bash 工具层命令级预检 + 审批门：网络命令（curl/wget/nc/ssh/…）+ egress-allowlist 域名校验，命中非白名单 → `InteractionRequested`（复用现有审批机制，`loop.rs:3293-3343` 同款 emit）；
- 方案 ③ 本地代理注入（对标 codex-network-proxy）：需新建代理组件，工作量大，**顶层初步倾向不做**；
- **顶层初步倾向：② 必做（N16a），① 视 ② 上线后 P5 campaign 实测漏过率再定（N16b 可选）**。N01 需实测：当前 bash 在沙箱内 curl 是否真的出网（校验十轮深挖 R4.5 与早期"DNS 被拦"旧记录的矛盾——以最新实测为准，陈旧记录作废）。

### 段 2 · 真施工（前置条件满足后，按序执行）

#### N10 · 冻结修订 RFC + 双签（§3 流程）

申报四条：FZ-RFC-1（S4 沙箱 env）/ FZ-RFC-2（S5 沙箱网络 + bash 预检）/ FZ-RFC-3（C-1+C-12 终态 verification 字段）/ FZ-RFC-4（S3 涉 loop.rs checkpoint 调用点）。S1/S2/C-2/C-3 在冻结区外（工具层/CLI/观测层），免申报但留登记。

#### N11 · Before 取证 + 验收尺子落地

1. **崩溃注入红证据**（在 .133 独立 workspace，避开 P4 在制品目录）：
   - E-1：长文件写入中途 `kill -9` → 取证"原文件被 truncate + 新内容半截"（S1 的 before）；
   - E-2：任务执行中途 kill → resume → 取证"该轮历史零落盘"（S3 的 before）；
   - E-3：并发两 session 写同一文件 → 取证"后写覆盖"（S2 的 before）；
   - E-4：子进程 env dump → 取证 `HEARTH_API_KEY` 等直透（S4 的 before）；
   - E-5：沙箱内 `curl https://example.com` → 取证出网成功（S5 的 before，与 N01 结论对账）。
   - 全部留档 `docs/p5-foundation/before-evidence.md`（before/after 对照是终验 A-4 的依据）。
2. **A3 run_local.rs 集成测试骨架**：CLI 主路径（create → run → kill → resume → 落盘断言）测试文件落 `crates/agent-runtime/tests/`，先只覆盖**当前正确行为**（绿基线回归锁），崩溃注入用例在 N12+ 逐个红转绿。

#### N12 · S1 工具层原子写（冻结区外，最先做）

- 按规格实现 `write_atomic`；edit/patch 全部写路径收口；
- 红测试：E-1 场景自动化（写入中注入失败 → 断言原文件逐字节不变）；
- 全量回归：464 + 新增全绿；tag 点见 §6。

#### N13 · S3 增量 checkpoint

- save_snapshot 迁出 Ok 分支；每 turn/每 N 步落盘；
- 红测试：E-2 场景自动化（中途 kill → resume → 断言历史恢复至最近 checkpoint，丢失 ≤ N 步）。

#### N14 · S2 改前快照 + 回滚

- 快照钩子接入 N12 的统一写入口（这就是 S1 先行的原因）；`hearth rollback` CLI；
- 红测试：E-3 场景自动化（写后 rollback → 逐字节恢复）。

#### N15 · S4 env_clear —— **[v0.2.22 tag 点]**

- 白名单最小集 + 红测试：E-4 自动化（子进程 env 断言无密钥）；
- S1+S3+S2+S4 批全绿 → **tag v0.2.22**，CHANGELOG 记录（对照 Claude Code 改前快照、Codex env 隔离语义）。

#### N16 · S5 出网收口（按 N01 裁决执行 N16a 必做 / N16b 视裁决）

- N16a：bash 命令级预检 + 审批门；红测试：非白名单 curl → 触发 `InteractionRequested`；白名单内 → 放行；
- N16b（可选，须顶层另批）：seccomp 硬拒 + AF_UNIX 豁免 + 总账防线 B 裁定推翻记录。

#### N17 · T 级批（C-3 → C-2 → C-1+C-12）

- 顺序即风险顺序：C-3（constitution.rs，1-3 行 + 单测）→ C-2（dispatcher 观测事件）→ C-1+C-12（终态 verification 字段，冻结区 RFC-3 已批项；fixture 用 run-001 语料）；
- 顺带：wiring 门禁给 `constitution-reads-file` 加一条运行时行为断言（十轮深挖 A4 提必做项并入）。

#### N18 · 回归门 + 终验 —— **[v0.2.23 tag 点]**

1. 全量 `cargo test --workspace` 全绿（464 存量 + 新增）；fmt/clippy `-D warnings`/xray 五门禁零告警；
2. **After 崩溃注入对照**：E-1~E-5 五场景全部重跑，after 证据与 before 对照表落 `docs/p5-foundation/final-report.md`；
3. E7 防复发三查：.133 binary 重建对齐 + sha256 + 冒烟（`bash -lc hearth --version`）；
4. 总账回填：RC24 → CLOSED（十轮深挖已证 `loop.rs:3293-3343` 已修）；crates=27 / tests=464 / provider-aware 阈值三条陈旧数据更正；P3 防线 B 裁定处置记录（若 N16b 执行）；
5. tag v0.2.23 + 独立窗（砺）终验审计。

## 5. S5 顶层初步裁决（N16 前最终确认）

| 方案 | 判定 | 理由 |
|---|---|---|
| ② 命令级预检 + 审批门 | **必做（N16a）** | 复用现有审批机制（`InteractionRequested`），工作量中，不破坏"bash 内合法联网"场景（白名单内放行），语义与 egress-allowlist 一致化 |
| ① seccomp 硬拒 | 可选（N16b，视实测） | 机制最硬（对标 Codex），但打死 bash 内全部合法联网（pip/cargo 拉包），需推翻 P3 防线 B 裁定；若 N16a 后 campaign 显示预检可绕过率高，再升 ① |
| ③ 本地代理 | 不做 | 需新建代理组件，工作量 S+，与单二进制战略冲突，收益与 ① 重叠 |

## 6. Node × 热点文件冲突矩阵（段 2 排序依据）

| Node | 主触文件 | P4 交集 | 冻结区 |
|---|---|---|---|
| N12 S1 | tools-builtin/edit.rs、patch.rs | 无 | 否 |
| N13 S3 | run_local.rs、agent-core/loop.rs（调用点） | Node 03/12 轻度（**已收口后施工，无实际冲突**） | 部分（RFC-4） |
| N14 S2 | tools-builtin + cli（rollback） | 无 | 否 |
| N15 S4 | sandbox/lib.rs | 无 | **是（RFC-1）** |
| N16 S5 | bash.rs、dispatcher.rs、sandbox/lib.rs | 无 | **是（RFC-2）** |
| N17 C-1/12 | agent-core（TaskResult/报告投影）、terminal.rs | 无 | **是（RFC-3，不动九态枚举）** |
| N17 C-3 | constitution.rs | Node 13 碰截断标记（已收口） | 否 |

## 7. 验收标准（顶层收货口径）

| # | 标准 |
|---|---|
| P5-A1 | 段 2 全部 Node 完成，每项"先红后绿"证据链完整（红测试在修复前 commit 先行且真实失败） |
| P5-A2 | E-1~E-5 before/after 对照表齐全，五场景行为符合 §4 规格 |
| P5-A3 | 464 存量 + 新增测试全绿，五门禁零告警 |
| P5-A4 | 冻结修订四条 RFC 双签留痕，九态枚举未动（仅加字段） |
| P5-A5 | E7 三查通过（.133 binary = source 同版本，sha256 一致，冒烟过） |
| P5-A6 | 总账回填完成（RC24 CLOSED + 三条陈旧数据更正 + 防线 B 处置记录） |
| P5-A7 | 砺·评审独立终验审计通过（抽验红测试真实性：checkout 修复前一版，红测试必须仍红） |

## 8. 明确不做（防范围膨胀）

| 项 | 去向 |
|---|---|
| D1 追工具数量 / D2 MCP·subagent·TUI | v0.3 功能层，本包不碰 |
| D3 拆 loop.rs | 技术债挂账，不与地基抢排期 |
| D4 landlock 全树可读 | ⚪ 行业共性，不动 |
| D5 bwrap 三层 | 不做（与单二进制战略冲突，S4/S5 收口后风险已降） |
| A1 bash 进程复用 / A2 fuzz | v0.3 backlog（A2 优先喂 bash 切段解析器 `lib.rs:591-647`） |
| 判断层 E1/E2/E3 | 留 v0.3 立项（三层摸底桶 B），本包只做其骨架侧前半 C-1（与 C-12 合并项） |

---
*段 1（N00/N01）可与本文件一并下发执行窗口；段 2 的下发令由顶层在 P4 双签通过后单独签发。*
