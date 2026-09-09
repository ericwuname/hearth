# top-level-plan-v23.md 审查补充 — 拆解优化 + 安全隔离

> v23 规划本身扎实：WP-0 通用交互原语是正确架构、三道隔离层清晰、禁止清单完整。
> 下述补强聚焦三个实践缺口：WP-0 前置修复独立化、WP 具体文件锚点、执行/测试窗口隔离方案。

---

## 补充1：WP-0 前置修复包（三个 RT4 缺陷需要独立的 WP-0a）

规划 §2.4 写得对——RT4-③ id 不校验是"泛化即放大 bug"。但规划把修复塞进 WP-0 本身——等于一个 WP 同时做架构重构 + bug 修复。**风险：重构引入问题 + 旧 bug 留在新架构里，调试时无法区分谁引入的。**

建议拆出 **WP-0a：审批原语先修再泛化**：

| 步骤 | 文件 | 内容 | 独立验证 |
|---|---|---|---|
| R1 | `dispatcher.rs:171-197` | `approval_id` 匹配 pending map 中的项，不匹配拒绝 | 错 id → 报错（有测试） |
| R2 | `dispatcher.rs` + `scheduler.rs` | pending 项一次性消费，重复提交拒绝 | 重复 id → 报错（有测试） |
| R3 | `api/src/lib.rs:69-72` + `codex-cli/src/client.rs:135-138` | 统一 `ApprovalReq.decision: String` 为服务端与 CLI 共用的类型 | `codex-cli approve` 不再 422 |

**WP-0a 独立于 WP-0**：不碰任何架构——只修现有审批管道的缺陷。修完后 wiring 15/15 不得断裂 + 214 tests 不得回归。**WP-0a 是 WP-0 的阻塞前置**——审批准了才敢泛化。

---

## 补充2：WP-0 的具体文件锚点（施工窗口需要精确的行号）

规划 §2.5 有迁移路径但没有具体文件锚点。补：

| step | 文件 | 行参考（1255498） | 动作 |
|---|---|---|---|
| ① 类型定义 | `api/src/lib.rs` | ~13（AgentEvent 旁） | 新增 `InteractionRequest/Response` struct |
| ② 通用暂停 | `loop.rs` | ~41（NeedApproval 旁） | 新增 `Event::InteractionRequested` + 通用暂停逻辑（不 match kind） |
| ③ 审查迁移 | `loop.rs`、`dispatcher.rs`、`scheduler.rs` | grep `NeedApproval` / `Approval` | 把审批改写为 `kind="approval"` 的实例 |
| ④ 通用路由 | `service/src/main.rs` | ~683-699（路由挂载区） | 新增 `POST /session/{id}/interaction/{iid}` |
| ⑤ 旧路由适配 | `service/src/session.rs` | ~594（submit_approval） | 保留为薄适配层 + deprecated 标记 + R3 契约修正 |
| ⑥ CLI 修复 | `codex-cli/src/client.rs` | ~135-138 | approve/deny 用统一类型（R3 闭环） |
| ⑦ 门禁门 | 测试文件 | grep `test_approval` | 旧测试不改一行仍过；新增 R1/R2 失败用例 |

**施工前必须 `grep` 重锚所有行号**（当前基线 `1255498`，施工时可能已推进）。

---

## 补充3：WP 时间估算（给拆解层）

| WP | 内容 | 估时 | 需要真 LLM？ |
|---|---|---|---|
| WP-0a | 审批原语修复（R1/R2/R3） | 2h | 否 |
| WP-0 | 通用交互原语 | 4h | 否（replay 验证） |
| WP-1 | 事件信封 + span 树 | 3h | 否 |
| WP-2 | SSE + 录制重放 | 2h | 否（replay 重放） |
| WP-3 | 规划缺口推导器 | 3h | ⚠️ 门禁需要 "抽 10 个真实任务" — 约 ¥2 |
| WP-4 | Observer 地基 | 2h | 否 |
| WP-5 | 确定性指标引擎 | 3h | 否（禁 LLM） |
| WP-6 | 规则引擎 + Finding | 2h | 否 |
| WP-7 | G0 红线熔断 | 1h | 否 |
| WP-8 | 产物登记 | 2h | 否 |
| WP-9 | 思考摘要 | 1h | ⚠️ 需要 LLM 生成摘要 |
| WP-10 | 前端接真流 | 2h | 否（已有 demo） |

**总计 ~26 小时开发 + ~¥2 真 LLM 验证预算。** WP-0a 和 WP-0 是最优先，它们合计 6h 决定了后续所有 WP 的地基。

---

## 补充4：执行/测试窗口并行安全隔离方案

```
master (v22.0 tagged)
  ├── worktree-test  → 冻结在 v22.0 tag
  │     测试窗口独占，跑 34 项红队测试
  │     产出 audit-findings-v22.md（不改代码）
  │
  └── worktree-dev   → HEAD (1255498)
        执行窗口独占，施工 WP-0a → WP-0 → WP-1 → ...
        git push 到分支 v23-dev
        测试窗口发现的 bug → 另开 fix 分支修，不混入 v23
```

**铁律**：
- 测试窗口的 worktree **不许 commit**——只读 + 输出报告
- 执行窗口的 worktree **不许 force push 到 master**——只在 v23-dev 分支施工
- 两个 worktree 共用同一个 `.git`，但工作区完全独立
- VM 也隔离：测试窗口用 VM 跑 `~/codex_v22_test/`，执行窗口用 VM 跑 `~/codex_v23_dev/`

---

## 补充5：缺失的工作包关联

规划 §5.1 的工作包依赖图正确，但缺少一项：

**WP-3（规划缺口推导器）依赖 WP-0**——因为 gap 推导结果如果是 `blocking=true`，需要通过通用交互原语让人澄清。没有 WP-0，gap 只能写日志不能暂停。添加：`WP-3 depends: WP-0`。

规划 §5.2 的依赖图已隐含此关系（WP-0→WP-3），但 §5.1 表中 WP-3 的依赖列标注为 WP-0。✅ 已正确。

---

## 补充6：§8 Q11 /readyz 复核

规划 §8 Q11 标注了 MEMORY.md 与 RT1 的矛盾——MEMORY 说"已真修"，RT1 说"无 store 时 still 200"。这是本规划需要立即裁决的：

**复核结论**：RT1 的断言是对的。`routes.rs` 中 `/readyz` 的实现是 `list_persisted_sessions()` — 当 `memory_store=None` 时返回 `Ok(vec![])`，永远是 200。这是 EPIC-B 的实现选择（"无 store = 健康"），但审计应诚实记录为"半修"——有 store 时真探活，无 store 时仍是假 200。

建议：在 WP-0a 后独立修，或正式记录为"运维注意项：无 store 部署下 readyz 不影响可用性判断"。**不在本版规划中排期**。

---

## 补充后的执行顺序

```
阶段1（6h，先做）：
  WP-0a 审批修复 → WP-0 通用交互原语

阶段2（6h，WP-0完成后可并行）：
  WP-1 事件信封 → WP-2 SSE → WP-4 Observer地基（串行）
  WP-3 缺口推导器（可与 WP-1/2 并行）

阶段3（6h）：
  WP-5 指标引擎 → WP-6 规则引擎 → WP-7 熔断

阶段4（3h）：
  WP-8 产物登记 + WP-9 思考摘要（可并行）

阶段5（2h）：
  WP-10 前端接真流
```

**测试窗口并行跑**：在阶段1-5全程，测试窗口在 v22.0 frozen worktree 上独立跑 34 项红队测试，互不干扰。
