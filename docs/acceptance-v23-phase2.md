# v23 阶段二验收报告：WP-0 通用交互原语全量落地

> 日期：2026-08-02 | 基线：`33d5804`（红队 P0/P1/P2 已闭环）→ 验收后 `b045067`
> 依据：`v23-phase2-plan.md`（已刷新基线 + 红队状态）+ `top-level-plan-v23.md` §2 + `v23-execution-order.md`
> 性质：执行窗口一轮自主施工——**WP-0 全 8 步闭环 / 红队剩余项全部闭环或确认挂账 / phase2-plan 已刷新**

---

## 〇、审查补充优化（用户问"有没有补充优化的地方"）

1. **基线刷新（关键）**：`v23-phase2-plan.md` 写于凌晨（基线 `df41bc7`），其中"红队剩余修复"清单（CT2/RT2/RT9/WT1/RT6）在随后的 P0/P1/P2 批次**已全部闭环**（守门员 R2 验收 12/12 + 施工 P0/P1/P2 + P1-4 门禁，221 passed / 208/208）。已重写为 §〇 状态表：仅 **RT3(seccomp=P2-2，用户明确"先挂着") / N7 密钥复核 / CT1 wiring AST** 挂账。
2. **WP-0 迁移策略修正**：原 plan 说"旧审批测试不改一行仍过"——实际 dispatcher 4 个测试直接引用 `ApprovalState` 接口，迁移后**必须接口改名适配**（语义不变）。已修正验收口径。
3. **对外契约兼容红线**：`AgentEvent::NeedApproval` 事件名 + `POST /approvals` 保留薄适配层——**对外 API 契约零破坏**（否则旧客户端全断）。
4. **对定版的必要补充**：`InteractionResponse` 增加 **`resolved: bool`** 通用传输字段——原 v23 §2.1 定版 Response 无此字段，但内核必须能区分"放行/中止"（approval 的 approve/deny 语义），否则 deny 也会放行工具执行 = 安全漏洞。内核只读 resolved，仍不解析 payload。

---

## 一、WP-0 全 8 步闭环 ✅

| step | 内容 | 证据 |
|---|---|---|
| **① 类型定义** | `api/lib.rs`：`InteractionRequest{id, kind:String, blocking, timeout, on_timeout, payload}` + `InteractionResponse{id, by, resolved, payload, latency_ms}`——kind 开放字符串非 enum | 编译通过（clippy 0） |
| **② 事件迁移** | `loop.rs`：`Event::NeedApproval{id,action}` → `Event::InteractionRequested{id,kind,blocking,timeout,on_timeout,payload}`；emit `kind="approval"`（blocking=true）；**内核不 match kind、不解析 payload** | agent-core grep approval 分支 = **0 处** |
| **③ dispatcher 通用化** | `ApprovalState` → `InteractionState`（NoInteraction/Pending{interaction_id,kind,action}/Resolved/Rejected）；接口 `set_interaction_pending` / `resolve_interaction`（**R1 id 强校验 + R2 一次性消费保留**）/ `interaction_state` / `reset_interaction`；scheduler `check_approval` → `check_interaction`（只认状态转移） | R1/R2 测试全过 + 旧审批测试接口改名适配（语义不变） |
| **④ 通用路由** | `POST /api/v1/sessions/:id/interaction/:iid`（body=InteractionResponse；path iid 与 body id 不一致 → 400） | cli_contract_test 2/2 通过 |
| **⑤ 旧路由适配** | `POST /approvals` 保留薄适配层（decision 字符串 → resolved bool）+ deprecated 标记；`AgentEvent::NeedApproval` 对外事件名不变 | 旧 CLI 契约兼容 |
| **⑥ CLI** | `client.rs` approve/deny 改走 `POST /interaction/{iid}`（InteractionResponse 契约） | **CLI approve 端到端 2/2**（真实 service + 真实 CLI 二进制） |
| **⑦ 门禁** | R1 错 id 必拒 + R2 重复提交必拒（既有测试保留） | 222 passed 零失败 |
| **⑧ dummy kind** | `test_wp0_dummy_kind_not_switched`：approval/clarification/`test-kind`/任意自定义 kind 全走通 | **加新交互不改内核任何代码**——测试 ok |

## 二、验收清单（全勾）

- [x] `agent-core` grep `"approval"` 字面量分支 = **0 处**；`tool-runtime` 仅剩 kind **数据实例**（`"approval".into()`，非分支）
- [x] 现有 **222 tests 零回归**（221 + dummy kind 新测试）零失败
- [x] 新增 dummy kind → 内核 **0 行**代码改动（测试证明）
- [x] R1 错 id → 拒（`test_wp0a_r1_wrong_interaction_id_rejected` ok）
- [x] R2 重复提交 → 拒（`test_wp0a_r2_double_submit_rejected` ok）
- [x] `codex-cli approve` 端到端通过（走 `/interaction/{iid}`）
- [x] **对外 API 契约不变**：`need_approval` 事件名保留 + `/approvals` 薄适配层
- [x] `resolved: bool` 通用字段落地（内核只读它，不解析 payload）

## 三、门禁汇总

| 门禁 | 结果 |
|---|---|
| cargo fmt --check | ✅ 0 |
| cargo clippy -D warnings | ✅ 0 |
| cargo test --workspace | ✅ **222 passed 零失败** |
| cli_contract_test（真实 CLI） | ✅ 2/2（approve 走新路由） |
| window-framework | ✅ 208/208（上轮守门员批次，未改动） |

## 四、红队对应项最终状态（phase2-plan §〇 刷新后）

| 项 | 状态 |
|---|---|
| RT2 civ 三层静默 | ✅ P1-4（含独立门禁 test_p14_*） |
| RT9 `/readyz` 永远 200 | ✅ P1-2（上抛 + None→503 + 写探活） |
| RT9 会话持久化 | ✅ P1-1（读路径回落） |
| WT1 bash 沙箱 | ✅ P0-5（白名单+shlex+禁网） |
| RT6 并发限流 | ✅ P2-1（Retry-After + per-IP + 探针豁免） |
| CT2 测试自身验证 | ✅ 各修复先红后绿（wiring 变异 / R1/R2 / P2 回归） |
| RT4 审批门绕过 | ✅ WP-0a（R1/R2/R3）+ WP-0（本次泛化） |
| **RT3/P2-2 seccomp** | 🔴 **用户 2026-08-02 明确"先挂着"**——大工程+安全风险，独立排期 |
| N7 密钥串用 | 🟡 证据不足，复核挂账 |
| CT1 wiring AST | 🟡 挂账（P0-4 已做剥注释+哈希锁） |

## 五、交付

- commit `b045067`（WP-0 8 步 + phase2-plan 刷新）
- `docs/v23-phase2-plan.md`（已刷新：基线 `33d5804` + 红队状态表 + WP-0 验收清单修正）
- **WP-0 完成后，v23 的 WP-1~WP-10 全部解封**（事件信封/SSE/Observer/指标/规则/熔断/产物/摘要/前端）——下一批可并行开工
