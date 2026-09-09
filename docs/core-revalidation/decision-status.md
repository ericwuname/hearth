# Decision Status — Core Revalidation（P4 滚动维护）

更新：2026-09-01（P4 Node 13 执行窗修复 + v0.2.21 发版后滚动）

## 工程状态
**CORE FREEZE SUSPENDED / CORE REVALIDATION IN PROGRESS**

## 冻结结论处置
- CLOSURE-01 freeze-decision（CORE FREEZE）= 历史轨迹；自 v0.2.18 盲测起 SUSPENDED。
- 冻结红线（总包 §1 不可触碰区）继续有效——SUSPENDED 解除的是"验收结论"，不是"修改边界"。

## P3 复验剩余（4 NEEDS-RERUN + 1 NOT-IMPLEMENTED）
| 项 | 状态 | 计划 |
|---|---|---|
| RC33 长会话 revision 对照 | NEEDS-RERUN | P4 Node 07-09 审计期 |
| 复测包（A-5·A-36 / 12·18 矩阵） | NEEDS-RERUN | P4 Node 12 campaign |
| RC34 终端级快照 | NEEDS-RERUN | P4 Node 12（×2 渲染场景） |
| RC13 service 会话真机 | NEEDS-RERUN | 独立专项（service 启动流程） |
| RC29 trust on | **NOT-IMPLEMENTED** | ledger 修正项（能力未构建，非复验） |

## 当前 OPEN 缺陷登记（滚动）
| ID | 层 | 状态 |
|---|---|---|
| RC52 | **归因闭合（Node 03/04）**：decision（RC47 族）主因 + 污染放大器 40%→100%，CAUSE LIKELY | **机制级修复已落（v0.2.21 = Node 13）**：根因=run() 重置块清 `written_files` 销毁跨 run 产物事实 → 最小 diff=session 级 `session_written_files` + give_up 臂门控回填（零写盘+零错误）；产物存活性仍由 Done 相位校验裁决（INV-LR03 不动）。**待测试窗 Node 14 独立复测验证疗效**（预注册：fresh 失败率应显著 <40%；若仍 ~40% = 修复无效信号回炉）。升格 CONFIRMED 仍归顶层/外部 AI |
| RC51-A/B | Projection isolation+completeness | **已修（v0.2.20：diagnostics.log + 产物清单投影）** |
| RC48 | mechanism(bounded)+model | ACCEPTED DEVIATION + RFC（维持） |
| RC53 | model tool misroute | **已修（v0.2.20 bash 内部工具名提示）** |
| RC54 | whitespace 边界 | **CLOSED**（P1-8 lenient 已在 v0.2.4，fixture 锁定——ledger 记载过时已更正） |
| REPL 程序化驱动 | NEW F1 | **已修（SimUser PTY driver，矩阵 13 会话实证）** |
| 静默截断 ×6 | 投影 completeness | **已修 4 处（v0.2.21）**：constitution 注入 / goal_drift 输入 / failed_nodes 摘要 / CLI 诊断错误；**2 处豁免登记**（loop.rs:3781 检索 query 内部构造、lib.rs:679 纯 UI 摘要）——原则：事实投影侧截断必须可见，内部构造与纯显示豁免 |
| R-1 仪器（session id 提取） | 测试仪器 | **已修（v0.2.21）**：改 sessions 目录 fs diff + uuid 非空断言（旧 `reports/<36>/` 正则在 8 位短 id 下恒 None）。**C 条件仍 BLOCKED**——待砺·评审复核后方可重跑 |
| docs CRLF 污染（E7 变体） | 同步管线 | **已定位根因**：Windows `core.autocrlf=true` + 无 `.gitattributes` → 文本文件被改写 CRLF 后二进制回灌 Linux → 字节级测试锁失配（project-xray wiring FNV 锁已红过一次）。**已加 `.gitattributes`（`* text=auto eol=lf`）防复发**（待顶层追认）；.131 存量 docs 污染未全清（当前无其他测试受影响） |
| 测试环境模板 | 工程规程 | **更正（分作用域）**：`cargo test`（门禁）**不要**设 `HEARTH_ALLOW_NO_CGROUP`（与 sandbox `test_rt4_cgroup_fail_closed` 互斥）；**真机跑批必须设 `=1`**（不设 bash 被 RT4 fail-closed，轨迹 5 步 vs 9 步，与基线不可比）。早前"3 测试需该变量"系误判（真因并行污染） |
| provider 出口（2026-09-01） | 环境 | **已解决**：国际出口曾整体被拦（Agnes/Cloudflare/Google 全超时，baidu 200）→ 用户挂 VPN，.131 出口改道 `45.88.202.26`（挪威），Agnes 探活 401、端到端冒烟 14s 通过。**残留风险**：VPN 掉线样本须判 PROVIDER_UNREACHABLE 并出失败率分母 |
| Node 14 派工 | 转测试窗 | **已发**：`docs/P4-Node14测试派工单 v1.0（执行窗→测试窗口）.md`（A×5+B×5；C 待砺解封；确定性判定器 `~/fa/n14/n14_judge.py` 口径已与砺 Step3 人工结论对齐：旧数据 9/10 复现、B4 判 DRIVER_INDUCED） |
| Agnes caps 申报 128K vs 理论 512K | provider | 登记待核（供应商侧） |
