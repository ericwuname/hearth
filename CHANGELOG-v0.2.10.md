# CHANGELOG v0.2.9 → v0.2.10 — 审批门语义化 / 任务路由 / 沙箱权限集修复（2026-08-29/30）

> 本版本区间覆盖三个施工窗口：RC24（审批门三合一）、W8（Goal Routing/任务路由五件套）、N1-SBX + O-1（沙箱权限集与 headless guard）。
> 证据链：`docs/hearth-rc24-approval-gate-acceptance-report-v1.md` / `docs/hearth-w8-acceptance-report-v1.md` / `docs/hearth-w3w4-reverify-report-v1.md` / `docs/n1-landlock-devnull-diagnosis.md` + 归档 `docs/data/rc24-20260829/`、`docs/data/w8-20260829/`、`docs/data/w3w4-reverify-20260829/`、`docs/data/n1-landlock/`。
> 门禁演进：391 → 399（RC24 +8）→ 407（W8 +8）→ **411（N1-SBX +4）passed / 0 FAILED**（.133）。
> 里程碑：**W3/W4 四证明新基线复验闭合（顶层 CLOSED 2026-08-30）**。

## RC24 — 审批门语义化 + 非交互结构化拒绝 + 会话级审批委托（2026-08-29，commit 90c703d）

- **A1 审批判定语义化**：`bash_cmd_is_destructive` 重构为 `BashRisk` 三级分类（Benign/CommandTable/HardRedline）+ `redirect_targets` 重定向目标扫描器——`/dev/null` 位桶精确豁免，其余 `/dev/*`、`/proc/`、`/sys/` 设备与内核接口保留拦截；**无空格写法与带空格同权**（TC-9 根因修复）。先红后绿：旧代码 4 红取证。
- **B 非交互审批结构化拒绝**：`ApprovalPolicy` 三态（Interactive/DenyAllNonInteractive/DelegateSession）——headless（stdin 非 tty）下审批请求照常进事件流后**立即结构化拒绝**（`approval_denied_noninteractive` + 可行动 hint），对照旧基线 23s 阻塞/挂起 200s；`run_abort` 结构化终止通道 + terminal.rs normalize 映射行（九态封闭集不变）。
- **C 会话级审批委托（RC29）**：`--approve-within session`（one-shot）/ `trust on|off`（REPL）——命令表级破坏性操作自动放行 + 三件套审计（tracing / CLI `[delegated]` 帧 / run report 审计字段）；**硬红线永不委托**（fork bomb/设备写/内核接口）。真机：委托后 0 弹窗、rm 真实执行。

## W8 — Goal Routing / 任务类型路由五件套（2026-08-29，commit db15d37）

- **A1 任务类型路由（DEV-1）**：`goal_requires_product` 三态漏斗（负向短路 → 产物动作动词 → 论述/QA 信号 → 默认 product）——论述/问答型短路 TaskGraph/Reflect 循环走 QA 直答。**V-1 十论述任务复跑 10/10 completed（基线 1/10）**——DEV-1 结构性死因闭合。先红后绿：旧代码 11 红取证。
- **A2 Goal Revision 三分类**：`classify_user_input`（单例，零 LLM）接入 `run()`——TaskControl/Conversation 不动 current_goal/revision；T5 假完成投影行（`✓ completed（N 步）`）规则集锁定。
- **A3 RC26 计划块去重**：plan_draft 双渲染删除 + `missing_goal_source` 收紧为仅外部指代真歧义。
- **A4 RC31 goal_drift**：长程任务（steps≥20）终局 observe-only 语义相关度检测（DRIFT/ALIGNED/None），警示事件 + report 字段，不阻塞。
- **A5 NEW-15**：resume exactly-once 机制补测（哨兵文件不变 + 空图不覆盖已完成图）。

## N1-SBX — Landlock 对象类型感知权限集（2026-08-30，commit f8f549a，G0）

- **根因**：`FS_RW`/`FS_RO` 混入目录专有权（READ_DIR/REMOVE_DIR/MAKE_*/REFER 共 11 位）——非目录 parent_fd（`/dev/null` 等）内核必 EINVAL；**T6 的 /dev/null 放行自 v0.2.3 起从未生效**（被旧审批门遮挡至 RC24 放行后暴露）。诊断矩阵 C1-C5 实测（`docs/n1-landlock-devnull-diagnosis.md`）。
- **修复**：`FILE_MASK`（文件级 4 位全集）/`DIR_ONLY_MASK`（11 位）+ `FS_FILE_ONLY = FS_RW & FILE_MASK` 动态掩出（禁手写常量——未来 ABI 位自动正确）+ 编译期覆盖性断言（15 位无交叠无遗漏）；`add_landlock_rule()` fstat 类型感知，调用点零改动；**FS_RO 同病同修**（FS_RO_FILE）。fail-closed warn+skip 行为分毫不动。
- **真机两层验收**：①规则层零 EINVAL；②执行层 `echo hi >/dev/null` 沙箱内成功（TC8_EXEC_OK）——**TC-8 完整转绿**（判定层 RC24 + 执行层 N1-SBX）。目录/普通文件规则不回归。
- **总账回填**：T6 条目勘误（历史"已落地"改写为"从未生效"留痕）+ TC-8/TC-9 闭环批注。

## O-1 — headless interaction guard（2026-08-30，commit 1d5bd86）

- budget_reassess/clarification 交互分支补 `is_terminal` guard（CLI 消费端，照抄 RC24-B approval 先例）——headless 下无人应答不阻塞等待 stdin（实证：V2R2 卡 200s → guard 后秒级 `budget_exhausted` 结构化收口）。Budget 数值语义与 ApprovalPolicy 零改动。

## 其他

- **N-2**（已并入）：one-shot Ctrl-C `⏹` 终态投影（commit 6ca3938）。
- **W3/W4 新基线复验**（commit e9c8a32）：四证明维持/转绿闭合（顶层 CLOSED）——resume 3/3（A2 TaskControl 正交重证）、六终态 5/6 自然证据 + verify_failed 机制层覆盖、RC20 中文错误锚 pass。
- **已知缺口（OPEN，记录不修）**：
  - O-3：seccomp allowlist 缺 `SYS_MKDIRAT(258)`——`mkdir` 等建目录命令 SIGSYS（exit 159，与 chmod/getent 159 同族）。fail-closed 方向缺口（安全无损，功能受限）。G0 变更须独立批准。
  - Q-3：landlock ABI ≥5 未 handled 位放行（kernel 7.0.0-30）——observe-only。
  - P1-LTR-01：deadline 为 loop 级——单步长工具调用穿透任务剩余期（设计单另行出）。
  - OPEN-W8-1：约束性负向 + 产物意图混合任务误路由 QA（Task Type + Constraints 分离原则，后续路由设计处理）。
