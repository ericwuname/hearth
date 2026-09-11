# 提交顶层验收 · 契约手术 7 项 + EMBER 五期（M0-M4）

- **提交人**：执行窗（本报告由「砺·评审」代拟）　**日期**：2026-09-12 00:40
- **范围**：2026-09-11 全天 + 跨夜（提交链 e4dafa3 ← 3eb8b29 ← 440e25b ← 37cc9a6 ← 9cae37a ← 908dbaa ← 3530d69 ← ecb82fa）
- **一句话**：用户交办的两条主线（契约手术、EMBER M4）全部完成；**但本批"施工与验收同主体"，证据是实测记录而非独立复核——建议顶层安排独立复核（第二节给了照跑命令）。**

---

## 〇、性质申报（必读，先于结论）

1. **施工/验收同主体**：用户 9-11 显式授权"全权交给你，你一直做到 M4 完成"——本批 7 项手术与 M4 的施工、验收、报告均由同一主体完成。**存在利益冲突，不能自证**。
2. **砺·评审边界申报**：砺的常规边界是"不改 `crates/`，只核验/出总账"。本批**越界施工**（授权范围内，但顶层应知悉）。
3. **本报告的证据等级**：所有"✅"均为**实测记录**（有红绿输出可查）；**无一条经过第三方独立复现**。

---

## 一、交付清单（10 件，全部已入库）

### A. 契约手术 7 项（hearth 引擎侧根因修复）

| 项 | 病灶（实测锚点） | 修复 | 红样本（先红复现的失败） | commit |
|---|---|---|---|---|
| **K-1** 交付语义 2.0 | M0 v1：任务要"实现 CLI"，产物只有探测残留 `probe.json` → `✓ Done`，核心交付零行 | `deliverable_type_mismatch`（任务-产物类型核对，缺→Replan≤2 轮，用尽则诚实交付） | `K-1: 要求实现 CLI 而产物只有 probe.json → 必须拦` | 3eb8b29 |
| **K-2** headless 输出契约 | `-p` stdout **800+ 字节全污染**（badge/进度/收尾/六段总结），答案无独立输出；**S18 当时声称"stdout 仅正文"实际未实现** | `HEADLESS` 标记 + `outln!/out!` 分流宏（进度→stderr 不丢）+ 答案独立输出 | 红=实测 stdout 800+B → 绿=**2 字节（"7"）**，stderr 1679B | 3530d69 |
| **K-3** 参数归一契约 | `bash.rs` 裸取 `args.get("cmd")`（read/grep/glob/edit/patch 已有容错，**bash 遗漏**） | 改用 `extract_str_arg`（Object/raw String/截断前缀三形态） | `②raw-String: missing 'cmd' argument` | ecb82fa |
| **K-4** 状态落盘契约 | `save_run_state_mirror` best-effort 但**三个吞错点全静默**；缺"无内存依赖"断言 | 三处补 `tracing::warn`；新增无内存依赖往返契约测试 | 篡改断言 → `FAILED` | 908dbaa |
| **K-5** 退避预算感知 | M1 实测 `deadline exceeded 1114s>=900s`——S7 长退避（30s/1m/2m）吃光墙钟预算被自己的超时杀死 | 退避前算剩余任务时间；**>30% → 不硬等，转暂停（可 resume）** | `K-5: 不得硬等 25s，实际 25.002374706s` | 9cae37a |
| **K-6** 审批语义 workspace 化 | **M1 审批拒绝真因**：`edit` 分支 `p.has_root()` 让 **workspace 内的绝对路径**也需审批 → 非交互拒绝 → 续跑中断 | 判"是否逃逸 workspace"（逻辑规范化，不触盘）；bash 分支同补参数兼容 | `K-6: workspace 内绝对路径不得触发审批` | 37cc9a6 |
| **K-7** provider 错误分类 | .133 IPv6 无出口致全链路瞬退，错误一律 transient → **S7 耐心退避掩盖根因空转 1h** | 分类标签（DNS/连接拒绝/地址族/连接失败(查DNS)/TLS/超时/限流/鉴权/上游5xx/未分类）进投影 | `K-7: M1 实测串必须可辨` | 440e25b |

**回归**：`cargo test -p agent-core --lib` → **146 passed / 0 failed**（K-1 零误伤）；Linux 全量 131+ 通过。

### B. EMBER 五期（立项书 M0-M4 全达成）

| 期 | 判据 | 结果 | 施工主体 |
|---|---|---|---|
| M0 单轮 | 四门槛 | 全过 | hearth（v2 卡明确后一次成功） |
| M1 工具循环 | L1-01 原卷 5 判据 + 换卷 | **全对**（换卷数字我保密，逐项吻合） | tools.py=hearth；接线=执行窗 |
| M2 记性 | 跨进程回忆 + 续干 | 四步全过 | 执行窗补全（hearth 两轮未成） |
| M3 自诊 | 先红后绿 + 零回归 | 全过 | 执行窗（hearth 窗口内未完成） |
| **M4 自举** | **六判据 + 零人工** | **全绿** | **EMBER 自己** ⭐ |

M4 关键证据：分页正确（第 101-105 行 + 行号标注）/ **与 sed 逐行一致** / 边界（498-500 共 3 行）/ 向后兼容（24499 字符）/ schema 更新 / 整体问答 `6+7=13` ✓。
产物演进：5.4KB（单轮）→ 11KB（工具）→ 15.2KB（记性）→ **15.9KB + tools 8.9KB（自举）**。

---

## 二、独立复现指引（顶层/第三方窗口照此跑，≈15 分钟）

```bash
# 0) 登录评审机（.133），工程在 ~/hearth-slim
ssh wutao@192.168.220.133   # 密码 123456
cd ~/hearth-slim && export PATH=~/.cargo/bin:$PATH

# 1) 【必做】源码级验证——先证明代码到位，再谈测试（K-4 的教训：曾因同步未达跑出假绿）
grep -c "fn deliverable_type_mismatch" crates/agent-core/src/loop.rs      # 期望 1
grep -c "fn test_k1_deliverable"       crates/agent-core/src/loop.rs      # 期望 1
grep -c "fn test_k5_backoff"           crates/agent-core/src/loop.rs      # 期望 1
grep -c "fn test_k6_workspace"         crates/agent-core/src/loop.rs      # 期望 1
grep -c "fn test_k7_provider"          crates/agent-core/src/loop.rs      # 期望 1
grep -c "extract_str_arg(&args, \"cmd\")" crates/tools-builtin/src/bash.rs # 期望 1

# 2) 全量回归（期望 146 passed）
cargo test -p agent-core --lib

# 3) 逐项契约测试（期望各自 ok）
cargo test -p agent-core --lib test_k1   # 交付语义
cargo test -p agent-core --lib test_k5   # 退避预算
cargo test -p agent-core --lib test_k6   # 审批语义
cargo test -p agent-core --lib test_k7   # 错误分类
cargo test -p codex-cli  --lib session_store   # K-4 落盘契约
cargo test -p tools-builtin --lib test_k3      # K-3 参数归一

# 4) 【对抗验证·建议做】红样本复现（证明测试真能失败）
#    任选一例：把 K-6 的 path_escapes_workspace 调用换成旧的 has_root() 表达式
#    → 期望 test_k6 FAILED（说明断言真的在把关，不是空转）

# 5) 【端到端·建议做】K-2 管道语义真机验证
cd ~/ember-exam/m4/ember
ember 等价物：HOME=... ./target/release/hearth -p "只回答一个数字：3+4=?"
#    期望：stdout 仅 "7"（2 字节）；进度全在 stderr
```

**K-2 复现脚本、M4 验收脚本**：`/tmp/k6-verify.sh`、`/tmp/k7-verify.sh`、`/tmp/k1-verify.sh`、`/tmp/m4-acceptance.sh`（.133 上可重跑）。

---

## 三、验收勾选表（顶层逐项签）

| # | 项 | 证据位置 | 签 |
|---|---|---|---|
| 1 | K-1 交付语义 2.0 | commit 3eb8b29 + `test_k1` | ☐ |
| 2 | K-2 headless 输出 | commit 3530d69 + 真机 `-p` stdout=2B | ☐ |
| 3 | K-3 参数归一 | commit ecb82fa + `test_k3` | ☐ |
| 4 | K-4 落盘契约 | commit 908dbaa + `session_store` 测试 | ☐ |
| 5 | K-5 退避预算 | commit 9cae37a + `test_k5` | ☐ |
| 6 | K-6 审批语义 | commit 37cc9a6 + `test_k6` | ☐ |
| 7 | K-7 错误分类 | commit 440e25b + `test_k7` | ☐ |
| 8 | 全量回归 146 passed | 第二节命令 2 | ☐ |
| 9 | EMBER M0-M4 五期 | `docs/p0-usability/r9-reports/EMBER-M*验收报告*.md` | ☐ |
| 10 | 报告与产物入库 | `ember/`（23 文件）+ 7 份报告 | ☐ |

---

## 四、异见栏（反方意见必答 · 项目强制栏目）

### 4.1 我自己对本次交付的异见（4 条，都不轻）

1. **同主体自证**（最重）：本批"我修→我验→我写报告"。按项目铁律（不信报告信源码），**本报告本身属于"报告"**，顶层不应直接采信——**第二节命令就是为此准备的**。
2. **K-1 是启发式，不是语义理解**：`deliverable_type_mismatch` 靠**关键词表**（实现/脚本/CLI + 扩展名）判"任务要不要源码产物"。**必有两类错**：①任务说"分析日志"没提"工具"，实际要交付脚本 → **漏拦**；②任务要"一段文字说明"却含"报告"字样 → **误拦**。当前只做到"明显病灶场景能拦"（M0 v1 那种）。
3. **K-5 的"30% 阈值"是我拍的，没有数据**：为什么是 30% 不是 50%？没有实验支撑。**可能的后果**：阈值过保守 → 抖动时就暂停（用户体验变差）；过激进 → 仍可能撞 deadline。**这是待标定的参数，不是结论。**
4. **M4 的"零人工"要说清边界**：我做了三件"人工"准备——①从 m3 复制干净目录 ②传任务卡 ③准备验收脚本。**"零人工"指的是"施工过程零干预"**（我没在 EMBER 干活时插过手），**不含环境准备**。

### 4.2 预判顶层会问的四个问题 + 我的回答

| 质疑 | 回答 |
|---|---|
| "七项手术有没有引入新缺陷？" | 全量回归 146 passed 零误伤，但**这只覆盖现有测试**。K-1 改的是"完成判定"，K-6 改的是"审批语义"——两者都是**行为面改动**，测试覆盖不到的真实任务行为需②的实跑才能暴露 |
| "为什么 K-6 的病灶和任务书写的（只读命令被拦）不一样？" | 任务书里写"diff 被拦"是**从现象推的假设**；执行时核实源码发现 `diff` 判 Benign，真因是 `edit` 的 `has_root()`。**任务书的病灶假设有误，我改对了真因**——这类"假设 vs 实测"偏差本批共 3 处（K-2/S18 声称、K-6 病灶、K-4 的 mirror 语义），建议顶层注意机构性的"文档滞后于代码" |
| "EMBER 现在能干什么真活？" | 单轮问答、多轮工具循环（读/写/跑/验）、跨进程记忆、自诊、自举。**L1-01 级任务（日志分析类）已验证可用**。更复杂的（多文件重构/长链任务）**未验证** |
| "为什么不让 trae 做？" | 用户 9-11 明确说 trae 积分耗尽、授权执行窗做。**若 trae 恢复，建议由它做第二节的独立复核**（换主体=真独立） |

---

## 五、不在本批范围（挂账，请顶层排期）

| 挂账 | 状态 |
|---|---|
| D2 历史 40 条 + service 集成重建（第一批） | 未动 |
| cognition MVP（包 B 数据合流） | 未动 |
| 归档线两签（`v0.2.23-final`：用户签 + T-6 外部签位） | 未动 |
| 版本 bump `0.2.26 → 0.2.27` + CHANGELOG | **未做**（跨边界，等你裁） |
| GitHub 同步（`ericwuname/hearth`；remote 已配） | **未做**——你定的"手术完成+bump 后同步"条件已达成，但需先做安全清理审查 |
| 仓库卫生：`.git.corrupt-20260911/`（32M 事故残留，建议项目外归档不删）、release/ 下 24 个未入库 md、`.playwright-cli/` 加 ignore | 未做 |
| **用户终审：3 个真实小任务** | **只有用户能做** |

---

## 六、请顶层裁示（4 个决策点）

1. **是否采信本批交付**（采信 / 要求独立复核后再定 / 驳回）？
2. **独立复核安排**：交 trae（若积分恢复）/ 另起一个评审窗口 / 或指定我方按第二节命令做"复现式验证（声明为复现非独立）"？
3. **收口是否执行**：bump 0.2.27 + CHANGELOG + 仓库卫生 + GitHub 同步（含安全清理审查）——**一次性批还是分步**？
4. **EMBER 线走向**：M0-M4 立项书已完结——延展（M5）还是转向"用 EMBER 干真活"（我建议后者：北极星是你真实完成任务数）？

---

**附：产物位置**
- 报告：`docs/p0-usability/r9-reports/`（EMBER M0-M4 验收报告 ×5、问题卡 ×3、契约手术任务书、本报告）
- 代码：`ember/`（ember.py 15.9KB / tools.py 8.9KB / 三代版本 / 各期验收日志）
- 记忆：`.workbuddy/memory/2026-09-11.md`（全天记录，含环境纪律 4 条）
