# CHANGELOG v0.2.5 → v0.2.9 — Harness 锻造与 ContextBuilder 分层（2026-08-28/29 汇总）

> 本文件为 v0.2.5–v0.2.9 五个版本的汇总补档（该区间无逐版 CHANGELOG，违反历史惯例——Final Acceptance 守门员补充 4 追补）。
> 证据链：`docs/` 下 R1/R2-D/R2-C/Tier3/ContextBuilder 各任务书与报告 + VM 隔离门禁实测（最终 387 passed / 0 FAILED）。
> 里程碑：**v0.2.9 = R2-C ContextBuilder Final Acceptance（APPROVED 2026-08-29）稳定基线**。

## v0.2.5 — Harness R1：终态投影与归档隔离（2026-08-28）

- **G1 终态投影规范化**：任务终态（Task completed/failed）成为唯一可信出口投影——修"假 Done"渲染层失效（RC20 家族：英文子串匹配双向失真）的第一步。
- **Completion 投影**：完成语义的物理校验（written_files 非空）显式化——RC1/RC10/RC21 完成语义家族首次正面处理（后续 v0.2.8 ResourceLedger / v0.2.9 Continuity 均继承此线）。
- **G3-03 归档会话隔离**：压缩归档按会话隔离，不再跨会话串档。
- 门禁：VM 358 passed 全绿。

## v0.2.6 — Harness R2-1：Egress 审批闭环 + Cache 证据采集器（2026-08-28）

- **T11 Egress 审批闭环**：受控联网（WS10 deny-by-default）补齐审批回路——白名单外域名产出可审批信号（R2-F 后续接入运行时白名单更新）。
- **R2-A Cache 证据采集器 v1**：`HEARTH_CACHE_TELEMETRY` 开关（默认关零开销）——per-request system_hash/prefix_hash/full_hash jsonl。首次给出 MISS 分类框架的数据底座（EC-01 基线 41 请求即出于此谱系）。

## v0.2.7 — R2-D TaskGoal：目标连续性闭环（2026-08-28）

- **original_goal immutable**：原始目标持久锚定（生命周期修复——非 history.empty 条件）。
- **state_revision 一致性**：目标变更事件（GoalChanged）与状态修订号对齐。
- **Task Continuity 块注入**：原始目标/已完成/剩余/下一步/约束/验收标准，经 Role::System 标签消息注入 history 尾部（dynamic suffix 区，不污染 stable prefix）——R2-A 实测"system 内动态内容 = cache 每步失效元凶"的直接治理。
- 门禁：VM 370 passed 全绿。

## v0.2.8 — R2-C Observe 层 + RT4 sandbox + Tier3 可靠性（2026-08-28）

- **采集器 v2（全出口覆盖）**：cache_telemetry 迁入 llm-gateway + `TelemetryProvider` 装饰器在 composition root 统一包装——v1 漏采 planner 直连 chat（decompose/reflect）的缺口闭合；`HEARTH_TELEMETRY_SID` 会话归属 + exit 字段。
- **ResourceLedger 资源记账（Observe 层）**：dispatcher 单漏斗 per-call/per-tool 字节记账 + 10MB/1GB 阈值 warn（57G 失控事故的直接回应；只观测不拦截——控制流属 D 类另出单）。
- **RT4 sandbox cgroup fail-closed**：`cgroup_base_override` 字段注入（Patch A-G），`test_rt4_cgroup_fail_closed` 去 ignore 转正 + Ok 分支 panic 防失效；实测发现并修复 handoff patch 的 E0382 缺陷。降级开关 `HEARTH_ALLOW_NO_CGROUP=1`（无 delegation 开发环境）。
- **Tier3 T1-T4 可靠性修复**：T1 现场留存（incidents/ 目录：B01-B10 遥测+故障 session 归档）；T2 嵌套重试穿透（provider 3×90s × loop 4× >> 120s cap——provider read-body 3×→1× + loop deadline 前置检查）；T3 连续 read-body 失败 → Fatal 快速失败（90s→60s 收敛）；T4 语义级停滞检测（TaskGraph 签名连续 2 次相同 → Err(stalled)——不随微进展重置）。
- **稳定性 5 连跑**：5/5 成功、写盘 ~3.3KB/任务（57G 对照基线）。
- 门禁：VM 373 passed（隔离门禁 `run_gate_r2c.sh` 建立——终结门禁脚本串台假绿灯事故）。

## v0.2.9 — R2-C ContextBuilder L1-L5 分层（2026-08-29，Final Accepted）

- **build_messages 分层重组**（施工单 v1.1，顶层批准后执行）：system_text 收敛为 L1(stable)+L2(task-stable)；experience（方案 X：降级通道语义不变，注入移 L4）/TaskGraph 状态（→Continuity）/retrieval/LSP 全部迁出 system——MISS-A（likely 根因）直接治理。
- **方案 B 双 sig 分立**：`topology_sig`（id/description/deps，无 status）驱动 L2 Task Topology 块重建；`last_graph_sig`（含 status）保留 Tier3 T4 停滞检测——两 sig 各司其职（裁决 1：实测 last_graph_sig 混入 status 不得复用）。
- **L4 固定顺序**：history → retrieval → LSP → experience → Task Continuity（最终语义锚点居尾）。
- **[FAILED] 事实保全**（裁决 3）：Continuity 增失败节点行（id+output 摘要），单测锁定。
- **Tier3 修复的行为回归治理**：T4 停滞计数在 v20 gate 重入时重置（EC-03 复测实锤误杀正常多步任务）——`note_v20_gate_reentry()` 语义锁双测试。
- **telemetry schema 三合一**：`msg_chain`（chain_k=H(prev‖role‖content_hash) 逐消息 8-hex）+ `tools_hash` + `phase`（main/aux 推导口径）——B 层 Message Prefix Stability 可验证化。
- **list_tools 顺序确定性**：观测器实抓 HashMap 迭代随机 → tools 数组每请求漂移（2-4 unique/会话）→ 按名排序修复——**pre/post 干净对照：主链 token 级 64.9%→74.5%、request 级 74.1%→88.9%**（组合归因：stability 改造 + deterministic ordering，未做单变量拆分）。
- **独立盲评**：T2 B>>C 行为证据确认；T4 基线结论撤销（两变体同为 rm 审批门阻塞）——证据恢复路径挂 RC24 修复单验收项。
- **T3 根因归类 B**（TaskGraph 活性判据缺陷：TaskNode.status 生产路径无更新 → Reflect 虚假 0/N → Replan → Stall）——独立 OPEN，下一施工单最高优先级，不阻塞本验收。
- 门禁：VM **387 passed / 0 FAILED / 0 ignored**（`t_gate_r2c.log` 10:34 实测）；CB 核心回归 6/6。
- 验收：**Final Acceptance APPROVED（2026-08-29）**——v0.2.9 为当前稳定基线，R2-C 停止施工。
