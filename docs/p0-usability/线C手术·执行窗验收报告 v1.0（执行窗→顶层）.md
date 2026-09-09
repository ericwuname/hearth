# 线C手术·执行窗验收报告 v1.0（执行窗→顶层）

- **出品**：执行窗/验收窗
- **日期**：2026-09-08
- **验收对象**：`r9-reports/线C手术战报 v1.0（traecode→顶层·验收窗）.md`（9fc3bd8）
- **被验收方**：traecode
- **验收协议**：附录 A（不信转录、独立重算）+ R-1 口径（报数一律 wc -l 坐标）
- **判据设计**：本窗沙箱拦截 wsl.exe，bash 族测试在本窗环境性必红（失败模式=spawn 被拒，非 trae 窗的 UTF-16 乱码），绝对通过数两窗不可比。故本窗测试判定采用**同环境双跑 diff**：HEAD（9fc3bd8）与基线（93e7e57）同命令同机跑测试，**失败集合新增项必须为 ∅** 方判零新增回归。

---

## 判定摘要

**手术本体 VERIFIED**。十项 D 项落刀实证、wc 硬线达标、零生产残留、a_arm_act_tally 保护实证、门禁干净、四 crate 测试双跑 diff 零新增回归。验收 7 条中本窗可证 4 条全过（①②⑥⑦），③④⑤移交跑测窗真机。**建议：转跑测窗真机验收；三项全过后举行归档仪式。**

---

## 一、验收 7 条对表

| # | 条目 | 判定 | 本窗证据（独立重算） |
|---|------|------|---------------------|
| ① | wc 硬线 <10,500 | ✅ 过 | 实测 **8,114** 行（R-1 口径 wc -l）；trae 申报 8,115 恒偏 +1（见 §四-1），不影响达标 |
| ② | 门禁 fmt/clippy | ✅ 过 | fmt 复跑 rc=0；clippy `-p agent-core -p planner -p agent-types` 0 error；`--workspace` 挂 sandbox linux_impl 平台门控=Windows 固有（diff 实证 sandbox 零触碰，非手术引入，见 §四-3） |
| ③ | 真机盲测 | ⏳ 移交 | 0.2.26 部署 .131；C YES≥13 / G-A≥3；C-2 回滚触发器挂载验证 |
| ④ | 墙钟中位 | ⏳ 移交 | 双臂 ≥3 轮中位数降 ≥50%（A 臂 −73.6% 旧值不可复用，须 0.2.26 新跑） |
| ⑤ | SINGLE_LOOP=0 回退 | ⏳ 移交 | 真机实测（`HEARTH_ALLOW_NO_CGROUP=1` 口径） |
| ⑥ | 0.2.26 bump | ✅ 过 | 根 `Cargo.toml` `[workspace.package]` `version = "0.2.26"`（394fd7b，一行 diff 0.2.25→0.2.26） |
| ⑦ | commit 链合规 | ✅ 过 | 13 节点逐一 git 核验（§二.1），施工顺序与战报申报一致 |

---

## 二、独立核验明细（附录 A 口径，全部本窗实测）

### 2.1 commit 链（6abd609..9fc3bd8，13 节点）

| # | commit | 内容 | 核验 |
|---|--------|------|------|
| 0 | 6abd609 | 顶层三签（起点，已核验入库） | ✓ |
| 1 | 434efd8 | D-10 落刀 | ✓ |
| 2 | 4f054a0 | D-8（一） | ✓ |
| 3 | ba4869f | D-8（二） | ✓ |
| 4 | cf1e88c | D-1 落刀（**带红 gate**，事故申报） | ✓ |
| 5 | 283cec3 | D-1 修复（d1surgery.py 误吞 build_messages 自愈） | ✓ |
| 6 | 668d4a8 | D-2（C-2 回滚触发器挂载） | ✓ |
| 7 | 6440421 | D-7（ReflectVerdict 跨 crate 删） | ✓ |
| 8 | edfbda6 | D-5（LoopPhase 收窄） | ✓ |
| 9 | 86bed05 | D-6 | ✓ |
| 10 | 0c3f5f5 | D-3+D-4 合并（单独落盘将激活 B 臂空图短路，合并合规） | ✓ |
| 11 | 964d144 | D-9 | ✓ |
| 12 | 394fd7b | 0.2.26 bump + service 5 测试删 | ✓ |
| 13 | 9fc3bd8 | 收官战报（docs） | ✓ |

事故披露核验：D-1 红 gate（cf1e88c）与 gcc 丢 PATH 工具链事故均已在战报 §现实偏离申报，无隐瞒。

### 2.2 wc -l 轨迹（本窗 `git show` 逐 commit 重测，13,055→8,114）

| commit | loop.rs 行数 | Δ | 备注 |
|--------|-------------|---|------|
| 93e7e57 | 13,055 | — | 基线（pre-linec-baseline tag 实指核验 ✓） |
| 434efd8 | 13,029 | −26 | D-10 |
| 4f054a0 | 12,915 | −114 | D-8a |
| ba4869f | 12,640 | −275 | D-8b |
| cf1e88c | 12,140 | −500 | D-1（误吞 build_messages） |
| 283cec3 | 12,296 | **+156** | 修复回升（实证自愈真实性） |
| 668d4a8 | 12,022 | −274 | D-2 |
| 6440421 | 10,170 | −1,852 | D-7 大刀 |
| edfbda6 | 9,213 | −957 | D-5 |
| 86bed05 | 9,135 | −78 | D-6 |
| 0c3f5f5 | 8,142 | −993 | D-3+D-4 |
| 964d144 | 8,114 | −28 | D-9，**已达标** |
| 394fd7b | 8,114 | 0 | bump，不动 loop.rs |
| 9fc3bd8 | 8,114 | 0 | 终值锁定 |

净削减 **−4,941 行（−37.9%）**，硬线 <10,500 自 6440421 起持续满足。

### 2.3 变更范围全景（`git diff 93e7e57 HEAD --name-only`，11 文件）

8 代码文件（loop.rs / agent-runtime/session.rs / agent-types/lib.rs / codex-cli lib.rs+repl.rs+run_local.rs / planner/lib.rs / service integration_test.rs）+ Cargo.toml + Cargo.lock + 战报 doc。**sandbox crate 零触碰**，与战报申报"8 files"口径一致（申报为纯代码口径，bump 与战报另计）。

### 2.4 残留 grep 定性

十项应删标识符全量 grep：**零生产代码残留**（仅存手术痕迹注释）。decompose 残留三类合法保留：planner trait 定义 / MockPlanner 测试仪器（服务"单循环零 decompose"断言依赖）/ prompt 文案。

### 2.5 a_arm_act_tally 保护实证（R8 −73.6% 战果资产）

全量 diff（93e7e57→HEAD）中 `a_arm_act_tally` **0 次出现**；现存 9 处全在 loop.rs 原位未动。手术未伤 R8 战果。

### 2.6 测试双跑 diff（零新增回归判据）

同机同命令：`cargo test -p agent-core -p agent-types -p planner -p codex-cli --lib --no-fail-fast`

| 侧 | agent-core | agent-types | codex-cli | planner | 合计 |
|----|-----------|-------------|-----------|---------|------|
| HEAD 9fc3bd8 | 116 passed / 6 failed | 10 / 0 | 29 / 0 | 7 / 0 | **162 / 6** |
| 基线 93e7e57 | 165 passed / 6 failed | 11 / 0 | 29 / 0 | 11 / 0 | **216 / 6** |

测试数对账：基线 222 → HEAD 168，**净删 54**（agent-core −49 / agent-types −1 / planner −4）。与 diff 属性统计闭合：删 64（#[test]+#[tokio::test]，其中 service integration_test.rs 占 5）− 增 5 = 四 crate 净删 54，**分毫不差**；service 删 5 与战报申报（394fd7b）一致。测试随被删功能走，无静默删测掩盖。

HEAD 失败集 F_head（6）：`context::test_archive_per_session_isolation`、`loop::test_node03_acceptance_passed_producer`、`loop::test_rc24_delegate_session_auto_approves_command_table`、`loop::test_single_run_compaction_e2e`、`loop::test_v12_approval_edit_write_detected`、`scheduler::test_bash_exit_code_structured_projection`

基线失败集 F_base（6）：上列除 archive 外 5 项 + `loop::test_r22_hard_slice_archives_early_turns`

**集合 diff 判定（零新增回归成立）**：

| 差异项 | 定性 | 证据 |
|--------|------|------|
| 共同失败 ×5（node03 / rc24 / single_run / v12 / bash_exit） | bash 族环境债，两测同挂 | 本窗 wsl.exe spawn 被拒（stderr 实证），非手术引入 |
| F_head − F_base = {archive_per_session_isolation} | **并行资源竞争闪烁，非回归** | HEAD 重编后单跑 3 次全过（0.01s）；整跑（122 并行）时挂——环境锁/文件竞争型闪烁 |
| F_base − F_head = {r22_hard_slice} | 测试仍在（loop.rs:7337），HEAD 过 = **手术连带改善** | R6-8 载荷缩编改变 archive 路径行为；单跑复验 ok（0.19s）；方向为"变好"，登记备查 |

**新增回归 = ∅。判定成立。**

闪烁记录：agent-core 同日三跑失败数 6/5/6（总测试数 122 恒定），archive 测试在无 fail-fast 跑中过、其余两跑挂——闪烁族实证，与战报对账口径一致。

---

## 三、25 红对账复核（trae 口径 vs 本窗实测）

trae 战报口径：19 基线债（WSL bash UTF-16 乱码等环境校准）+ 闪烁族 + xray +1（D-8 卡内删除×锁定断言冲突，已呈顶层裁）+ service 5（已删，394fd7b）= 25。

本窗复核：两窗环境失败模式不同（本窗 spawn 被拒 / trae 窗 UTF-16 乱码），绝对红数不可直接比对；本窗以双跑 diff 判零新增回归（§2.6），并以 HEAD 失败集逐项定性：

| HEAD 失败测试 | 本窗定性 | 对应 trae 对账科目 |
|--------------|---------|------------------|
| node03_acceptance_passed_producer | bash 族环境债（两测同挂） | 基线债 19 |
| rc24_delegate_session_auto_approves_command_table | 同上 | 基线债 19 |
| single_run_compaction_e2e | 同上 | 基线债 19 |
| v12_approval_edit_write_detected | 同上 | 基线债 19 |
| bash_exit_code_structured_projection | 同上 | 基线债 19 |
| archive_per_session_isolation | 并行竞争闪烁（单跑过） | 闪烁族 |
| （r22_hard_slice：基线挂→HEAD 过） | 手术连带改善，非红 | 行为改善登记 |

本窗实测与 trae 的 25 红对账口径相容：bash 族=基线债、archive=闪烁族、xray+1 与 service 5 属 trae 窗特有科目（xray 测试在本窗四 crate 范围外/已随 D-8 删除路径处理，呈顶层裁定件不变）。

---

## 四、登记事项

1. **计器口径**：trae 行数申报恒偏 +1 且同向（基线报 13,056/实测 13,055；终值报 8,115/实测 8,114）。R-1 口径以本窗 wc -l 实测为准，两处差 1 不影响达标判定，登记备查。
2. **版本注释瑕疵**：0.2.26 bump 行未按仓库惯例追加本版注释（仍挂 v0.2.25 描述）。文档性遗漏，不影响功能；建议随归档仪式补齐。
3. **Windows gate 局限**：`--workspace` clippy 挂于 sandbox linux_impl 平台门控（Windows 固有，非手术引入——全量 diff 实证 sandbox 零触碰）；核心 3 crates clippy 0 error；fmt 全绿。
4. **判据环境**：本窗沙箱拦截 wsl.exe，bash 族测试失败模式为 spawn 被拒；据此采用双跑 diff 判据（见判据设计）。
5. **变更口径**：基线→HEAD 11 文件 = 8 代码 + 2 清单（bump）+ 1 战报 doc；战报申报"8 files"为纯代码口径，两口径相容。
6. **验收过程事故（已闭环）**：本窗为双跑测试做 checkout 往返（93e7e57↔p0-usability-01）后，84 个已跟踪文件工作树丢失（顶层签发件/evidence/判分产物），疑与多窗口并发 git 操作踩踏有关。已全量核实（84/84 均在 HEAD 树）并 `git checkout --` 无损恢复，deleted=0 复核。**教训入库：共享 repo 上 checkout 往返后必须立即 `git status` 全量复核工作树完整性**；建议后续窗口避免在共享主 repo 做 checkout 往返（改用 worktree）。

---

## 五、呈顶层三件（自战报转达，待顶层批复）

1. **R6-8 载荷缩编确认**（TurnCheckpoint→turns+taskgoal）
2. **xray 锁裁定**（D-8 卡内删除 × 锁定断言冲突）
3. **service 集成覆盖重建 D2 候选**（394fd7b 删 5 测试后集成覆盖缺口）

---

## 六、移交：跑测窗真机验收三项（③④⑤）

1. 部署 **0.2.26** 至 .131（构建 md5 双验登记）
2. ③ 盲测：C YES≥13 / G-A≥3；**C-2 回滚触发器挂载验证**（668d4a8 落刀点）
3. ④ 墙钟：双臂 ≥3 轮中位数 ≥50%（主判据），失败数不升、质量不回退；calls=必录观察指标
4. ⑤ SINGLE_LOOP=0 回退实测（`HEARTH_ALLOW_NO_CGROUP=1` 口径）
5. 三项全过 → 归档仪式两签（tag + 双 tar）→ **GitHub 首次正式同步**（按既定拍板：remote → 安全清理 → force push 弃旧镜像）

---

## 七、结论

线C手术战报（9fc3bd8）通过执行窗附录 A 独立验收：**手术本体 VERIFIED**。硬线达标、链路合规、残留清零、R8 战果资产完好、测试零新增回归。验收 7 条 4 过 3 移交，移交项均为真机域，本窗不越权代跑。建议顶层：①确认本验收；②批复呈报三件；③授权跑测窗启动 0.2.26 真机三项；④全过后归档仪式与 GitHub 同步按既定拍板执行。
