# hearth-slim 第一批手术 · 六卡收官战报 v1.0（执行窗 → 顶层）

> **拟件**：执行窗（traecode）　**日期**：2026-09-10
> **依据**：《hearth-slim 第一批手术计划书·顶层签发件 v1.0》（c343031）+ 长程任务计划书（cbe4dea）+ 施工对接单（0b53418）
> **性质**：六代码卡（S1/S2/S3+S4/S5/S6）全部落刀完毕的收官汇报；**真机复测四卡验收线属执行窗后续，另件申报**。

---

## 一、卡执行总账（代码层全部落地）

| 卡 | 提交 | 落刀摘要 | 状态 |
|---|---|---|---|
| S1 工具超时 | `df9c4fa` | bash 默认超时 120s（`HEARTH_TOOL_TIMEOUT_SECS` 可配）+ 结构化 timeout JSON + 64KB 输出截断 | ✅ |
| S2 出网默认放开 | `b0a3a95` | 白名单空 = 全放（用户 2026-09-09 拍板）；显式 allowlist 仍收紧；`[net]` 审计投影格式钉死 | ✅ |
| S3+S4 prompt 瘦身 | `0dcb090` | 主系统段 ≤1.2K chars；宪法出注入层（语义降级投影层）；talent 停用（`HEARTH_TALENT=1` 可复开）；env 快照 3 行 | ✅ |
| S5 give_up+相位机同刀 | `b116ccb` | **本批最大刀**——见 §二 | ✅ |
| S6 压缩调参 | `34a1e07` | `COMPACT_CHAR_THRESHOLD` 32K→16K（2K 基底比例重调）+ `[compact] N→M chars` 投影 | ✅ |

**施工纪律执行**：
- S5 落刀前 `git archive` 备份 ✅（`.workbuddy/tmp/s5-backup/pre-s5-34a1e07.tar`，17MB）
- S5 独立 commit ✅（b116ccb，整体可 revert）
- 沙箱 crate 零触碰红线 ✅（sandbox 只补了测试 `#[cfg(target_os="linux")]` 门控——Windows `--all-targets` 编译修复，非功能改动；lib.rs 生产代码零触碰）
- a_arm_act_tally 9 处原位保留 ✅（S5 中其 single_loop 门归一后**恒跑**，机制本体未删——顶层附加条款 2 边界未触发申报）
- S2 投影格式先钉后落刀 ✅（[net] 行样式钉死在 b0a3a95 卡内）
- 四红线（C-2 回滚触发器/无自宣判分/账本第二批不动/禁顺手删资产）✅ 执行窗无越权结论

## 二、S5 大刀详情（b116ccb）

**切除**：
1. `enum LoopPhase`（Init/Plan/Act/Done/Error 五相位状态机）整体删除 → `StepNext {Act,Plan,Done,Error}`（do_plan 单次模型调用的"下一步去向"标签，非全局相位）。**`grep "LoopPhase" loop.rs` = 0 命中**（过关线达标）。
2. `Agent` trait 的 `step()` 分发器删除（无外部调用者）；Init 仪式/`Event::Phase(Init)` 相位投影行全消；do_plan/do_act 不再发 Plan/Act 相位投影。
3. `run()` 主循环重写为**消息循环**：每轮按 `pending_tool_calls` 是否非空分发 工具步（do_act）/ 模型步（do_plan 唯一 chat）；文本回应 = 模型 end_turn → `finalize_done()`。无相位变量、无 Init 跳、无 plan/act 相位行。
4. 收尾逻辑提取为可复用助手：`finalize_done`（写盘 verify ≤3 replan / acceptance Reserve 1 / goal_drift observe-only / completed·verify_failed 双报告，`Option<RunReport>`=None 时回喂 replan）、`finalize_error`（run_abort 结构化终止优先 + F9）、`finalize_step_error`——预算臂 route-done 与消息步两处统一调用。
5. single_loop A/B 双态删除（字段/env 门/测试开关/B 臂死机关全清）——消息循环即唯一主路径（R6-9/D-8 两臂同构的实证归一）。
6. give_up 命名清扫：rc52/FA01 活机制**保留**（预算硬停 + 诚实完成核验 = 计划书要保留的护栏），仅旧命名改中性（`GIVE_UP_OVERRIDDEN`→`FA01_VERIFY_RESERVE`、`GIVE_UP_ROUTED_TO_DONE`→`RC52_ARTIFACT_ROUTE`）；scratch key `giveup_unverified`→`budget_stop_unverified`（全仓 grep 无下游契约）；B 臂 search-streak 硬停删除（A 臂 nudge-only 转正）。

**过关数据**：`LoopPhase`=0；`give_up`/`stalled` 仅注释/历史审计字符串残留。agent-core 测试 **121/121**（S5 后 ×3 复跑全绿）——未删任何旧测试；r69/r75 单循环语义测试保持（移除已删开关）。

## 三、门禁/平台硬化（S6 门禁中暴露并同批修复）

Windows（MSVC）门禁首跑暴露的存量移植缺口，全部修复并随批提交：
- **工具链**：默认 GNU → **MSVC**（顶层已裁定"新路线标准化 MSVC"；GNU dlltool 缺失即症状）。`rustup default stable-x86_64-pc-windows-msvc` + 补装 rustfmt/clippy。
- **sandbox 测试门控**：4 处引用 `linux_impl` 的单测缺 `#[cfg(target_os="linux")]` → 补门控（S6 commit）。
- **bash 解析**：Windows system32 WSL bash 存根损坏（Bash/Service/0x8007072c）→ 真 bash 测试统一经 `HEARTH_BASH_BIN` → Git Bash（S1 同法）。
- **审批门 Windows bug**：`tool_call_needs_approval` 用 `is_absolute()` 判逃逸路径，`/etc/passwd` 在 Windows `is_absolute()=false` → 改 `has_root()`。
- **env 竞态 flake**：`HEARTH_ARCHIVE_FILE` 进程全局泄漏致 `test_archive_per_session_isolation` 偶发 NotFound（~1/8）。根因 = 测试双锁（ENV_SER+ENV_LOCK）纪律不齐 + 隔离测试不清 env。补 4 处缺失锁 + 防御性快照清除。**修复后连跑 11 轮全绿**（原 ~1/8 flake）。
- **project-sync（独立 commit `0ef638c`）**：fs_api 绝对路径判定 `has_root()` 补位；event_log Windows fs2 锁（append-only 句柄无读权限 → LockFileEx ACCESS_DENIED；锁竞争 raw os error 33 未归一 WouldBlock → 同作退避竞争）。44/44 绿。

## 四、当前测试状态

- fmt ✅ / clippy（workspace --all-targets）✅ 零 error
- agent-core **121/121**（多轮）；project-sync **44/44**；project-xray **11/11**（xray_test 集成 1 失败除外——见下）
- 全 workspace 唯一失败：`project-xray real_workspace_wiring_all_green` —— 读 `docs/xray/wiring-v13.toml` 找不到；该文件在 **HEAD 存在**、被**他窗未提交的 docs 大规模删除**（git status `D`，1463 文件）波及。与 hearth-slim 六卡零耦合，执行窗不恢复/不处理（不越权干涉他窗手术）。

## 五、遗留与后续（执行窗窗口外，申报不代办）

1. **顶层总验收四条**（c343031 §一.1 销账线）：L1-01 重考三判据 + 双语料墙钟 ≥50%（转轨义务）+ 基底 ≤2K（S3 实证中）+ 吴涛 3 真实任务抽查——均需真机，由执行窗后续跑批、判读口径照 c070411 预钉款，销账判定留顶层。
2. **ember 式最小会话真机**（S5 过关第四项"中途无相位投影行"）——真机项。
3. **corpus 复测单条累计 prompt max < 80K**（S6 过关）——跑测窗验证项。
4. S5 账本（91 处投影化）= 计划书明确**第二批**，本批不动。
5. 他窗工作区 docs 删除（wiring-v13.toml 等）与 xray 集成测试的关联，建议由该窗收口时核对。

---

*执行窗 · 2026-09-10 · 六刀落尽，类型与结构层到位；真机复测线交还执行窗跑批、判读与销账归顶层*
