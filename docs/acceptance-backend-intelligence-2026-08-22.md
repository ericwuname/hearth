# Hearth 后端真智能 — 执行验收报告

> 任务书：`docs/backend-intelligence-taskbook.md`（B2-A/B2-B/B3-A/D-extra，2026-08-22 签发）  
> 验收：2026-08-22 | commit `6c46f33` | 性质：质量审计 + 闭环验证 + CLI 呈现 + 挂账清收



---

## 1. 结论：R1-R4 全过 + 挂账清收（用户授权"把挂账的也做了"）

| 门禁             | 内容                                                       | 结果               | 证据                                                                                                                |
| -------------- | -------------------------------------------------------- | ---------------- | ----------------------------------------------------------------------------------------------------------------- |
| **R1** B2-A 质量 | 10 真实任务 gap 100% from+why；non-blocking 100% assume；无通用问题 | ✅ **10/10 PASS** | `docs/b2-gap-audit-10tasks.md` + 原始 JSONL（bench/audit_results3.jsonl）                                             |
| **R2** B2-B 呈现 | `hearth chat` 规划相位打印"规划草案摘要"（steps + 将问 + 自假定）           | ✅                | 实测：`🗺 规划草案（steps=5 gaps=3 待问=2 自动假设=1）` + `❓ 要问你` + `🤖 假设`；BE 无颜色/HTML（事实产生权合规）                                 |
| **R3** B3-A 闭环 | clarify emit→答→resume 完整；approval 同理；无丢/重复/卡死            | ✅                | 实测：`❓ 需要你确认 [ambiguous_option] 目标含未澄清技术选项...` → 答"Rust" → `✓ 澄清已提交（Rust）` → 规划草案续跑（steps=5）；approval 写文件 y → Done |
| **R4** D-extra | service 无 key → 启动失败（非占位静默）                              | ✅                | `startup_fatal`（无 OPENAI_API_KEY 报原因退出）；cli_contract_test 注入测试 key 验证                                             |

通用门禁：**fmt 0 / clippy 0 / 248 passed / build 0**（最终门禁全绿）。

---

## 2. 任务书核实后的关键发现（守门员没看到的）

1. **契约 v1 的 14 事件里根本没有 `plan_draft`**——任务书说"plan.draft 字段来自契约"但契约里没有（B2 引入未入契约的滞后）。**先补契约（v1.1）再改 BE**（契约先行，红线合规：新增类型不升 schema_version）。
2. **`need_approval` 事件丢 payload**——clarification 的问题内容（from/why）在 `InteractionRequested` 里有，但 map_event 只透传 id+kind，**用户看不到要确认什么**。补 payload 字段（契约补注）。
3. **ambiguous_option 50% 误触发**（审计一轮发现）——LLM 分解的英文任务描述"or"泛匹配。三轮收紧（§4）。

## 3. 交付清单

- **B2-A**：`bench/b2_gap_audit.py`（可重跑）+ `docs/b2-gap-audit-10tasks.md`；derive_gaps 三轮收紧（goal 优先 + 中英强信号 + why 带片段）
- **B2-B**：契约 v1.1 补注 plan_draft/need_approval.payload；BE PlanDraft 加 `gaps_to_ask_details`；CLI 渲染升级（steps + ❓ + 🤖）
- **B3-A**：NeedApproval 带 payload；CLI 澄清交互（❓问题 + 文本回答 + payload 回喂）；手动端到端验证 PASS
- **D-extra**：service sk-placeholder → startup_fatal
- **挂账**：req_body 日志降级、cgroup 明确标注、install.sh、平台分支验证

## 4. 审计三轮演进（这是本轮最有价值的部分）

| 轮  | 发现                                                    | 修复                                  |
| -- | ----------------------------------------------------- | ----------------------------------- |
| 一轮 | 10/10 PASS 但 **5/10 误触发 blocking**（产品红线"AI 不得问废话"被违反） | 去 `or `/`either` 泛匹配                |
| 二轮 | 仍 5/10——**根因：LLM 分解是英文任务描述**                          | 强信号中英二选一模式                          |
| 三轮 | **0 误触发** + 真歧义不丢                                     | **goal（用户原话）优先检测**——LLM 会把"或"消化成单语言 |

**核心洞察**：gap 推导只看 LLM 分解后的 task 描述会丢用户意图——**goal 原话才是歧义的真相源**。

## 5. 挂账状态

| 挂账                       | 状态                                                                                                                                                 |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| llm-openai req_body 日志噪音 | ✅ 已清（debug 级别保留）                                                                                                                                   |
| cgroup 标注                | ✅ 已清（明确"资源限制降级，非安全降级"）                                                                                                                             |
| D6 install.sh            | ✅ 已交付（curl|sh）；跨平台编译需目标平台验证（VM 无 Windows/macOS 工具链，代码平台分支已就位）                                                                                      |
| seccomp KILL 化           | ⚠️ **尝试后回退 ERRNO**——find 实测被 KILL 但 strace 46 syscall 全在白名单，无法定位（seccomp 先于 strace 执行）；ERRNO 下白名单外仍拒绝（EPERM），安全边界完整。**挂账：待 strace/BPF 深挖后恢复 KILL** |
| B4-2 桌面版                 | 任务书明确不属本轮（路线图第三梯队）                                                                                                                                 |

## 6. 用户验收路径

```bash
# 1. 规划草案可见（B2-B）
hearth chat "用 Rust 或 Go 实现一个工具"   # → ❓需要你确认 → 答 → 规划草案续跑
# 2. 审计重跑（B2-A，~30 分钟）
python3 bench/b2_gap_audit.py
# 3. service 独立部署（D-extra）
env OPENAI_API_KEY=<key> service   # 无 key 会启动失败报原因
```
