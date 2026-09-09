# Hearth 问题总账 v3（合并版 · 统一状态口径）

> **日期**：2026-08-31　**窗口**：砺·评审　**基线**：v0.2.18（`41cc070` / tag v0.2.18）
> **合并来源**：`consolidated-remediation-ledger.md`（v1，卡片式，RC1–RC21 + E1–E7 + D1–D6 + W1–W5）＋ `consolidated-remediation-ledger-v2.md`（v2，表格式，RC32–RC54 + W/D 系列）。**v1/v2 原文件保留作审计轨迹，不再单独维护。**
> **用途**：下一轮**评审窗口审批 → 执行窗口执行**的唯一派工依据。
> **取证方式**：本表状态**不采信任何报告自述**；每条经源码 grep 复核（v0.2.18），无法源码判定的标 `NEEDS-RERUN` 并写明所需实验。

---

## 0. 状态口径（先冻结 —— 防 RC51 复发）

| 状态 | 含义 | 判定要求 |
|---|---|---|
| **CLOSED** | 机制已改且源码实证 | 必附源码锚点；行为类另附真机/测试证据 |
| **PARTIAL** | 机制已改但根因残留 | 必须写明"残留什么" |
| **OPEN-DIAGNOSED** | 已定位归因、未修 | 必附归因结论 + 修法方向（若需顶层批准须标注） |
| **OPEN-UNSTARTED** | **从未排进任何施工批** | 必附源码锚点（证明缺陷仍在） |
| **NEEDS-RERUN** | 改动已落，**真机从未复验** | 必写所需实验；**禁止**写成"待复测"（那是"已修待验"的语义） |
| **DEFERRED** | 顶层已裁决暂不做 | 必附裁决出处（哪份总包 §几） |
| **DECISION** | 待顶层拍板 | 必附选项 + 守门人倾向 |
| **ENV-DONE** | 环境治理已完成（非代码改动） | 必附验证结果 |

> ⚠️ 本版修正一处历史误写：v2 把 RC34 记为"待复测"，实为**从未实现**（render.rs 无去重代码）。此后"没复测"一律写 `NEEDS-RERUN`，"待复测"一词禁用。

---

## 1. 一页纸统计

| 状态 | 条数 | 说明 |
|---|---:|---|
| **CLOSED** | **17** | 其中 15 条为本轮源码复核坐实 |
| **PARTIAL** | **5** | 残留项均已写明 |
| **OPEN-DIAGNOSED** | **7** | 含 1 条 F0-clause（RC52，当前 Freeze 阻断项） |
| **OPEN-UNSTARTED** | **11** | ⚠️ 从未排进施工批，不是"漏做"而是"没立项" |
| **NEEDS-RERUN** | **6** | 改动已落、真机未验 |
| **DEFERRED** | **6** | 顶层已裁决 |
| **DECISION** | **11** | 顶层待拍板 |
| **ENV-DONE** | **4** | 环境治理 |

**一句话结论**：**没有全部实现。** 已闭环 17 条；真正的问题不在"做过没做完"，而在**11 条从未立项** + **6 条改了没验** + **7 条已定位未修**。

---

## 2. 总表

### A. CLOSED —— 已闭环（源码实证，v0.2.18）

| ID | 问题 | 源码锚点 | 验收证据 |
|---|---|---|---|
| RC1 | 活性信号接线断裂（长程必死 ~15 步） | `loop.rs:3871` `mark_nodes_completed_on_accept` + 调用 `:4884` | 生产者 = Done 相位 verify 通过，**非 LLM 自报** |
| RC3 | bash 失败对 agent 不可见（非零退出码仍 Ok） | `scheduler.rs:45-48` is_error 由 Result 分支**结构化**决定（废除 `starts_with("error:")`） | 五态接线后 `ExitNonZero/ExitSignal/ToolTimeout` 可区分 |
| RC6 | 摘要 Debug 包装吃字符 | `context.rs:182`（naive）/ `:202`（legacy）/ `:212`（effective_estimate） | honest counting 测试（naive=7 vs legacy=29） |
| RC8 | `Event::Done.status` 三处 CLI 全不渲染 | `codex-cli/lib.rs:646 / 816 / 998 / 1024` | 原"三处全不读"→ 现四处读 status |
| RC9 | `give_up` 原因被吞 | `loop.rs:4840-4841` `error_detail` + `giveup_unverified` ×3 处（2576/4173/4487） | 失败报告不再折叠为 "loop error" |
| RC10 | 纯读/评估任务零完成校验 | `loop.rs:926 / 2046 / 2173` 盲区C 产物校验 | 与 acceptance 双证据门并存 |
| RC12 | `HEARTH_EGRESS_ALLOWLIST` env 到不了进程 | `codex-cli/config.rs:151/160` `merge_allowlist(env_val, cfg)` | env 与 config 双通道均已接入 |
| RC20 | 中文/非英文错误渲染成绿色 ✓ | `scheduler.rs:26-29/45-48` 结构化 is_error | 野外铁证：同一错误 v0.2.8 画绿 8 次 → v0.2.10 画红 9 次 |
| RC39 | **α 主根**：输入分类无疑问句类别 | `loop.rs:376-385` / `381` `QUESTION_WORDS` / `406` `？` 判定 | 修 B；正反测试（Product+疑问尾缀不误吞） |
| RC40 | `context_fill_pct` 语义错位 | 改名 `compact_pressure_pct`（`loop.rs:1333/1349` + `status.rs:91/101`），旧名全库零命中 | 动态分母，语义如实 |
| RC42 | 压缩频率体感失真 | provider-aware 阈值 `loop.rs:4352-4370`（env > caps×0.6×2.55 > 常量） | + `COMPACTION_MODE=legacy` 回滚通道 |
| RC44 | acceptance failed 被折叠成 pending | `loop.rs:1677-1695` 反折叠（object.status=="failed" → "failed"） | 三态完整（none/pending/passed/failed） |
| RC46 | bash exit code 未接 ToolResult | `BashExitError`（`dispatcher.rs:20-30`）+ scheduler 双 downcast | 五态测试通过 |
| RC47 | "继续"续轮 planner give_up | `loop.rs:4223` `GIVE_UP_ROUTED_TO_DONE` + 正反测试 | criteria 空 + 产物在 + 0 errors → 路由 Done |
| RC49 | CFR 规范链 compact 腿证据时效 | v0.2.18 干净二进制重跑；本窗核实 `.131`：`compacted.jsonl` 1,244,350 B @13:18 + session 归档 17,335 B @13:19 | 压缩确已发生（遗留文档指标缺陷 = RC51） |
| E7 | 源码树与二进制版本不同步 | 双 VM 已 0.2.18 对齐；`vm-version-sync.md` 分窗登记纪律已建立 | 本轮实测 binary 0.2.18（双方） |

### B. PARTIAL —— 部分处置（残留已写明）

| ID | 问题 | 已改 | **残留** |
|---|---|---|---|
| RC5/RC30 | 压缩失忆 + 迷航重跑 | turn 粒度修复（单 run 压缩复活）+ provider-aware 阈值 + archive 先行 + INV-M01 fixture | **60 字墓碑摘要仍规则式**（`context.rs:430-431`）；B 档模型检索未证明 |
| RC37 | 压缩后假完成 | INV-M01 锁定事实层不丢 | 墓碑摘要仍规则式；新形态 = RC52（会话级吸引子） |
| RC43 | 活性记分与任务构成错配 | give_up 消费端三路拦截 + Reserve 先决（真机实证） | **progress 口径维持不变**（FA01 §5 顶层裁决），扩展 DEFER |
| RC35 | 静默截断族（5+2 处） | render/RAG 两处定性 SAFE；web 8000 补 `[N chars truncated]`（`web.rs:130`）；bash 6000 原有标记 | **constitution 6000 无标记**（defer）；新增裸 tracing 泄漏（RC38） |
| RC15 | 自测口径失真（web_fetch "假 PASS"） | web 截断已补标记 | "全域名被拒却标 PASS"的判定口径**未专项复验** |

### C. OPEN-DIAGNOSED —— 已定位归因、未修

| ID | 问题 | 级别 | 归因 / 修法方向 | 状态 |
|---|---|---|---|---|
| **RC52** | **会话级持续失败吸引子**（run-013 起 24 连败、T4 27 次；fresh 同任务 2/2 completed） | 🔴 **F0-clause（当前 Freeze 阻断项）** | 与会话状态强相关；**入口为「继续」型输入**（run-013 "继续你的提议吧"），压缩晚于入口 11 轮（非起因）→ 主因待 P0-ATTRIBUTION 判定（是否与 RC47 同族） | **P0-ATTRIBUTION-01 已立（零修复归因）** |
| RC48 | false stop（Reserve 零和 × GiveUp 时点） | 🔴 F1 | 机制有界 + model 时点；修法（Reserve 分账 / acceptance-passed 消费扩展）**须顶层批准** | ACCEPTED DEVIATION，待批 |
| RC45 | `verify_replan_count` 双上限跨相位零和 | 🟡 P1 | Act<1 与 Done<3 共用计数器；预案在 failure-recovery-model §2 | 独立单待立 |
| RC50 | 非交互审批拒绝抵消恢复 | 🔴 F1 | RC24 设计行为（非缺陷）；**禁放宽审批**（触碰 STOP-3/5）→ 写入 does-not-guarantee | 待写冻结文件（本窗已核实：freeze-decision 的 does-not-guarantee 段**零覆盖**） |
| RC51 | 测法/指标缺陷（行数作证 + 三属性合并成一行） | 🟡 P2 | 压缩发生性证据只允许字节+mtime+文件数；CLOSURE §7 拆 isolation/completeness/correctness 三行 | 纪律待写入后续包 |
| RC53 | `introspect` 被当 bash 拼接（exit 127，5 次） | 🟡 P1 | 工具路由/序列化分支 | 待定位（只定位不修） |
| RC54 | `apply_patch` 空白/缩进脆弱（P1-8 复发） | 🟡 P1 | 精确匹配失败后降级重试 | 最小 fixture 待建 |

### D. OPEN-UNSTARTED —— ⚠️ 从未排进任何施工批

| ID | 问题 | 源码锚点（证明仍在） | 备注 |
|---|---|---|---|
| RC16 | `HEARTH_URL` 双语义（静默切 service 模式） | `codex-cli/config.rs:138/218`、`lib.rs:273` | **D4 已给方案**（拆 `HEARTH_SERVICE_URL` + `HEARTH_LLM_URL`），未做 |
| RC18 | `--provider` 不联动 url（Agnes key 送 deepseek → 401） | `run_local.rs:95/110` `cfg.url` 覆盖 | **D5 已给止血方案**（config set provider 时联动提示），未做 |
| RC13 | service 路径白名单不读 config.toml | （v1 锚点 `service/src/main.rs:411-416`） | 与 RC16/18 同批（W2） |
| RC14 | `merge_allowlist` 实为并集去重（与注释/推断矛盾） | `config.rs:160` | 语义需顶层裁定后再改 |
| RC25/RC27 | 读范围=全盘 `/`；PATH 同名无护栏 | 未找到任何读范围限域代码 | W7 |
| 防线B | seccomp 缺 SENDTO（bash DNS 死） | `sandbox/lib.rs:475/621` 有 `SYS_SOCKET(41)`，**SENDTO 未命中** | W7 决策项 |
| RC34 | ×2 渲染重复 | `codex-cli/render.rs` **无任何去重/折叠代码**（grep 零命中） | ⚠️ 此前误记为"待复测" |
| RC32 | stalled 同图提前放弃 | `loop.rs:2641` 阈值仍为 **2**（v0.2.18 实测触发 27 次） | W10；**禁止先改阈值**（归因前） |
| RC7 | 注释与实现不符（承诺"不丢架构决策"无承载字段） | `context.rs:153` | 低优先 |
| RC4 | 规划不执行 + replan 塌缩 | 旧日志实证（v0.2.4） | 需现版本复现后再定 |
| RC41 | 模型/供应商身份自述错误 | 存疑未查 | W2 附带 |
| RC38 | 脚手架空转 + **裸 tracing 泄漏**（新增两类） | 手工测试：裸 ERROR 6 处、compacted Debug 外泄 16 行 | P0-A 复现项（P0-ATTRIBUTION Node 01） |

### E. NEEDS-RERUN —— 改动已落、真机从未复验

| ID | 问题 | 现状 | 所需实验 |
|---|---|---|---|
| RC36 | 追问被 Goal Revision 吞掉（8 问 0 答） | 机制已随修 B 落地（`loop.rs:381/406`），**真机未复测** | QA 复现跑：连问同一问题 3 次，检查 revision 是否仍 ++ |
| RC33 | 目标修订爆炸（revision 71） | 同上；v0.2.18 盲测 revision 10（量级下降但**无对照基线**） | 同 RC36 实验 + 对照版本 |
| RC24-B | landlock `/dev/null` EINVAL（连带打挂 introspect） | 修复已落，未复测 | TC-8b 真机复跑 |
| RC29 | `trust on` 显式验证（不弹窗 + 审计留痕） | 待测试窗口验证 | W9 |
| 复测包 | TC-8b / ×2 渲染 / A-5·A-36 连续性 / 12·18 跑矩阵 | 待执行 | 测试窗口 v0.2.18 复跑 |
| RC2 | 观测层子串匹配（render.rs:86-92 对人用 contains） | 需确认是否随 RC20 一并解决 | 渲染层专项回归（P0-C 一并进行） |

### F. DEFERRED —— 顶层已裁决暂不做

| 项 | 裁决出处 |
|---|---|
| progress 语义扩展（四类 progress） | FA01 §5（须顶层批准；二阶效应清单在案） |
| 常量标定（BUDGET_LOW_THRESHOLD 等） | 修 A 备而未用（复-3 裁决 2） |
| MAX_HISTORY_MSGS=40 入 archive | CLOSURE Node 07（已打标记，冻结窗口不动） |
| Memory / Compaction 架构重建 | Core Freeze 红线 |
| constitution 6000 截断标记 | P2 Node 10（defer） |
| interaction 8 类样本补全 | P2-LR Node 03（2 类仅单测，DEFER） |

### G. DECISION —— 顶层待拍板

| ID | 事项 | 守门人倾向 |
|---|---|---|
| D4 | `HEARTH_URL` 拆分方案（A 拆双名 / B 保留旧名） | **A**（需兼容旧名一段时间） |
| D5 | config 是否联动提示 url 随动 | 至少做"联动提示"止血 |
| D6 | 两 VM 源码同步时机 | **A 发版后同步**（B 易漏，E7 已致结论失真） |
| D7 | #0 密钥擦历史（不可逆） | 顶层裁决 |
| D8 | W5 是否升 P0（依据：用户独立诊断 + 53–60% 失败紧邻压缩点） | 建议升 |
| D9 | 第 2/3 层评估栈是否立为正式工作项 | 建议立（落点见 D11） |
| D10 | 「任何系统不得对自身产出做终局判定」是否升宪法级原则 | 建议升（一条收敛六项） |
| D11 | 评估栈是否落 Observer OS（第三权·零执行权） | 建议落（对内补 L2/L3、对外成差异点） |
| W13 | 常量体检（子 agent 4096 = 512K 的 0.8% 等） | 建议优先（低成本高杠杆） |
| W12 | 摩擦遥测 + 轨迹 grader 进 CI + 变异测试 + persona 包 | 建议分阶段 |
| 指标 | 引入**正确弃权率**作北极星之一 | 建议采纳 |

### H. ENV-DONE —— 环境治理（非代码改动，已完成）

| ID | 动作 | 验证 |
|---|---|---|
| E1 | 删 `~/.cargo/bin/hearth`（v0.2.3 陈旧副本） | 两种 shell 均 0.2.8+，`which hearth` = `/usr/local/bin/hearth` |
| E2 | `~/.hearth_env` 在 `.bashrc` 早退前 + `.profile` 双点加载 | 登录/非登录 shell env 一致 |
| E3 | config.toml 补 egress-allowlist | `hearth config get egress-allowlist` 正确返回 |
| E4 | provider 切 Agnes 四项 | 冒烟通过（2 步 completed） |
| E6 | 死 alias `hearth-cli`（`.bashrc:126`） | ⚠️ **未清理**（低优先，随 E 类下一并做） |

---

## 3. 下一轮派工建议（评审窗口审批 → 执行窗口执行）

**排序原则**：先解阻断 → 再做低成本高收益 → 最后做需顶层先拍板的。

| 批次 | 内容 | 性质 | 依赖 | 验收方式 |
|---|---|---|---|---|
| **批次 0（已立，先行）** | **P0-ATTRIBUTION-01**：RC52 归因 + RC53/RC54 定位 + P0-A/C 复现 | **零修复** | 无 | 五件套（原始证据/源码定位/反事实对照/invariant mapping/disposition） |
| **批次 1（低成本高收益·从未立项）** | **W2 配置语义**：RC16（HEARTH_URL 拆分）、RC18（provider 联动 url）、RC13（service 读 config） | 小修 | D4 拍板 | 三处各加一条**能失败**的断言测试 |
| **批次 2（低成本高收益·从未立项）** | **W7 边界**：RC25 读范围限域、RC27 PATH 同名护栏、防线B SENDTO 决策 | 小修 + 1 决策 | D-防线B 拍板 | 限域后既有任务不回归；SENDTO 加位后沙箱内 DNS 可用（或明确记录"不加"的理由） |
| **批次 3（小修）** | RC34 渲染去重、RC45 Reserve 双上限、RC15 web 判定口径复验 | 小修 | 无 | 先红后绿；RC34 需渲染快照对比 |
| **批次 4（真机复验）** | RC36/RC33 追问与 revision 复现、RC24-B TC-8b、RC29 trust on、复测包 | **只测不改** | 批次 1-3 落地后 | 每条给原始日志 + 对照基线 |
| **批次 5（须顶层先拍板）** | W13 常量体检 / W11 α 治理收尾 / W5 压缩升级 / W12 评估栈 | 中修 | D8/D9/D10/D11 拍板 | 按各 D 项裁决执行 |

**每批通用门禁**：`fmt=0 / clippy=0 / tests pass`；**新增测试必须能失败**（先红后绿留证据）；真机证据须绑 binary 版本 + session id + 日志路径。

---

## 4. 给评审窗口的审批点（本轮需裁定的四件事）

1. **是否采纳 §0 的状态口径词汇表**（尤其"待复测"禁用、`NEEDS-RERUN` 与 `OPEN-UNSTARTED` 的区分）——这是防止总账再次漂移的前提；
2. **批次 1/2 是否立项**（RC16/18/13 与 RC25/27/防线B 这 6 条从未排期，且都是小修）；
3. **D4（HEARTH_URL 拆分方案）与防线B（SENDTO 加不加位）**——这两条是批次 1/2 的前置决策；
4. **DEFERRED 六项是否维持**（progress 扩展 / 常量标定 / 切片入 archive / Memory 架构 / constitution 标记 / interaction 补全）。

---

## 5. 维护纪律（防状态漂移）

1. **状态只允许 §0 八个词**，不得自造（"待复测""基本修好""应该可以"一律禁用）；
2. **每条 CLOSED 必须带源码锚点**（`file:line`），无锚点不得标 CLOSED；
3. **真机类结论必须绑版本 + session id + 日志路径**，跨版本不可比的要显式标注（如成功率只在 v0.2.8+ 可比）；
4. **本表每次发版后复核一次**，复核人与复核基线写入 §头部；
5. **v1/v2 冻结不再修改**——只维护 v3。

---

## 6. 溯源

- v1：`docs/consolidated-remediation-ledger.md`（P0/P1/P2 卡片 + E1–E7 + D1–D6 + W1–W5）
- v2：`docs/consolidated-remediation-ledger-v2.md`（RC32–RC54 表 + W/D 系列）
- 本轮复核证据：`docs/Hearth CORE FREEZE 顶层评审总览-砺.md`、`docs/hearth-core-freeze-closure-verification-砺.md`、`release/手工测试v0.2.18.txt`
- 下一轮派工件：`docs/Hearth P0-ATTRIBUTION-01 长程施工总包 v1-draft.md`、`docs/Hearth 拟真用户测试台与自反馈闭环 规划书 v1.1-砺.md`

*砺·评审零代码改动。本表所有状态可经上述文件复算。*
