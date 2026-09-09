# Round 2 执行报告（R2-F / R2-A / R2-B · 2026-08-28 凌晨）

> **执行**：v0.2.6 / commit `5b9ff93` + 本报告补数据 commit
> **依据**：《Hearth R1 评审结论与 Round 2 派工令_chatgpt.md》+ 守门员批注 1-6
> **VM 门禁**：fmt/clippy/test 全 RC=0，**363 passed / 0 failed**（~/t_gate.log）

---

## 一、R2-F Egress Approval ✅（T11 闭环，批注 5 两坑合规）

### 实现
| 环节 | 实现 | 合规 |
|---|---|---|
| deny 分类 | `loop.rs handle_egress_denials`：识别 web.rs 固定 deny 文案前缀，提取 host | 坑①：分类在 loop 层，web.rs 保持纯工具 |
| 交互 | 复用 budget ask 范式（set_interaction_pending → check → take），blocking 60s | 不建新事件系统 |
| approve | ①`Scheduler::set_env_var` 追加运行时白名单（下次 web_fetch 立即生效）②`EgressPersistFn` 依赖注入回调落盘 config（codex-cli 实现，复用 `Config::set_field("egress-allowlist")+save`） | 坑②：复用既有 config 写入路径；agent-core 不反向依赖 config 层 |
| deny/超时 | 注入"勿再尝试该域名"提示（LLM 经 pending_results 可见） | — |
| 防循环 | 同 host 每 run 只问一次（`egress_asked` HashSet） | — |
| CLI | 选项式分支纳入 `egress_allowlist_request`（显示 host/why，approve/deny 选项） | — |

### 测试（全负面可复现）
- `test_egress_approval_flow_and_no_reask`：mock 用户 approve → 断言持久化回调触发 + ctx.env 白名单含放行域 + "已获批准"注入历史 + **同 host 二次 deny 不再重问**
- `test_egress_denial_non_interactive_no_ask`：service/bench 模式保持 deny 原语义

### 真机验证
v0.2.6 release 已构建（`hearth 0.2.6`）——**端到端审批留给你下次手工测试时体验**（需真实白名单外域名触发，自动化测试不烧此预算）。

## 二、R2-A Cache Evidence ✅（24 请求，超 ≥20 门槛）

### 采集
- 工具：新模块 `agent-core/cache_telemetry.rs`（env `HEARTH_CACHE_TELEMETRY` 开关，未设零开销）；每 LLM 请求一条 jsonl（ts/sid/model/tokens/cache_hit/miss/system_hash/prefix_hash/full_hash）
- 接线：单点（do_plan_inner chat = loop 唯一 LLM 出口）
- 通道：Agnes（`agnes-2.5-flash`，用户授权预算）；任务：写文件类 + 读总结 + repl 多轮 × 3 session
- 数据：`docs/data/cache-r2a-20260828.jsonl`（24 行原始证据）

### 关键发现（证据链完整）

| 指标 | 实测 | 含义 |
|---|---|---|
| **cache 命中率** | **57.6%**（session1，40576/70460 hit tokens） | 与平台 95% 宣称差距 37pct——优化空间实锤 |
| **system_hash 稳定性** | **20 请求 7 个唯一值**（should be 1！） | **system prompt 每请求都在变**——stable prefix 从根上不稳 |
| repl 短会话命中率 | **0%**（4/4 全 miss） | 短会话完全吃不到 cache |
| prefix_hash 相邻相同 | 0/19 | 采集器 v2 需改进（见下），但 system_hash 证据已独立成立 |

### system 不稳定根因定位（build_messages 动态注入清单）

实测 `loop.rs` system_text 组装区，以下内容**每请求重算且随运行状态变化**：
1. **TaskGraph 状态注入**（`## Task Plan`：节点 Completed 状态每步变化）——**头号根因**
2. **experience 注入**（`## Past Experience`：按轮有效）
3. **LSP diagnostics / semantic code context**（环境变化即变）

→ **R2-C ContextBuilder 设计的第一决策已有数据支撑**：动态区块必须从 system 挪到 history 尾部（dynamic suffix），否则任何前缀 cache 都在每步失效。

### 采集器 v2 改进项（记入 R2-C 前置）
prefix_hash 当前定义（除末条外）对跨请求比较不敏感——v2 记录逐消息 hash 链，事后算"相邻请求公共前缀长度"，直接量化 cache 可用长度。

## 三、R2-B TaskGoal 设计单 ✅（待你评审，未施工）

`docs/r2b-taskgoal-design.md`——按批注 3 要求产出设计单**不直通 R2-D**。核心决策：
- **不新建 TaskGoal struct**：`RunState` 加 3 字段（original_goal/constraints/acceptance_criteria）+ `<sid>.taskgoal.json` 持久化 + TaskGraph 派生函数（completed/remaining/next_action）
- normalized_goal **DEFER**（无确定性收益，reentry 条件已写明）
- resume 语义：恢复 original_goal → build_messages 注入 Task Continuity 块——G3-05"继续什么"测试的正面依据
- 施工量 ~180 LOC + 5 测试
- **四个评审问题清单**在文档第六节等你裁决

## 四、R2-E ToolInvocation 评估（零代码结论）

源码复核：现有 `ToolCall/ToolResult/NeedApproval` + Envelope 的 `seq/ts/span_id` 已可推导完整生命周期（start=ToolCall ts、end=ToolResult ts、duration=差值、failed=is_error、审批=NeedApproval/Resolved 对）。**新增事件变体 ROI 不足**（派工令 §十三守门原则：不降 friction 不做）。真正缺口在 **Observer 消费侧**（把现有事件流聚合成 lifecycle 视图）——挂 Observer 规则扩展，非本轮。

## 五、状态与下一步

```text
R2-A Cache Evidence    ✅ 24 请求 + 根因实锤（system 不稳 + 57.6%）
R2-B TaskGoal design   ✅ 设计单待你评审（4 个裁决问题）
R2-C ContextBuilder    ⏸ 数据已够，设计待 R2-B 评审后一并出
R2-D integration       ⏸ 等 R2-B 评审通过
R2-E ToolInvocation    ✅ 评估完成（零代码，理由见上）
R2-F Egress Approval   ✅ 施工+测试完成，端到端体验挂你手工测试
R2-G D 类事项          ⏸ 等顶层出单（未动）
R2-H benchmark         ⏸ 等 R2-C/D 后
```

**建议**：醒来先看 `docs/r2b-taskgoal-design.md` 第六节的 4 个裁决问题——R2-D/R2-C 都卡在这几个决定上。
