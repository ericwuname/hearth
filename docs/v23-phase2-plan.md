# v23 下一阶段施工计划 — WP-0 全量 + 红队剩余 + 第二梯队

> **基线（2026-08-02 13:20 刷新）**：HEAD `33d5804`（红队 P0/P1/P2 已全闭环，221 passed + window-framework 208/208）
> 上次施工的正确决策：WP-0 不开工——4h 连续工程不能半途而废，审批迁移中间态会破坏现有测试
> 现在 WP-0a 前置已完成，且 P0/P1/P2（含 P1-4 门禁）全部落地，可以直接开工 WP-0

---

## 〇、红队状态刷新（2026-08-02，原 §二 清单大部分已闭环）

> 原 plan（02:10 写）的"红队剩余修复"基于 HEAD `df41bc7`。此后守门员 R2 验收 12/12 + 施工方完成 P0/P1/P2 全部 12+7 项，**大部分已闭环**：

| 原清单项 | 现状 |
|---|---|
| CT2 测试自身验证 | ✅ 已做（P0-4 wiring 变异测试先红后绿 / R1/R2 失败用例 / P2 各回归先红后绿） |
| RT2 civ 三层静默 | ✅ 已修（P1-4 + 独立门禁 test_p14_*） |
| RT9 `/readyz` 永远 200 | ✅ 已修（P1-2：错误上抛 + None→503 + 写探活） |
| WT1 bash 沙箱逃逸 | ✅ 已修（P0-5：命令白名单 + shlex + 路径校验 + 禁网） |
| RT6 并发限流 | ✅ 已修（P2-1：Retry-After + per-IP + 探针豁免） |
| RT9 会话持久化 | ✅ 已修（P1-1：读路径回落 load_persisted_session） |
| RT3 seccomp 扩展 | 🔴 **仍挂账**（P2-2 seccomp 白名单——用户 2026-08-02 明确"先挂着"，大工程+安全风险单独排期） |
| N7 密钥串用 | 🟡 证据不足，继续挂账复核 |
| CT1 wiring AST 强化 | 🟡 挂账（P0-4 已做剥注释 + 哈希锁，AST 匹配仍待排） |

---


## 一、阶段2：WP-0 通用交互原语（4h，P0 最高）

WP-0a 的三个前置修复（id 校验 + 防重放 + 契约统一）已经就绪。现有的审批管道是**完整的、被测试覆盖的、已知正确的**。现在可以把现有审批迁移到通用原语上了。

### 施工文件锚点（基于 HEAD `df41bc7`）

| step | 文件 | 动作 | 独立验证 |
|---|---|---|---|
| ① 类型定义 | `api/src/lib.rs` | 新增 `InteractionRequest/Response` struct（§2.1 定版，`kind: String` 非 enum） | `cargo check` 通过 |
| ② 通用暂停 | `agent-core/src/loop.rs` | 新增 `Event::InteractionRequested` 变体 + 通用暂停/续跑逻辑（只认 `blocking`/`timeout`/`id`，不 match kind） | replay 模式跑通暂停→续跑 |
| ③ 审批迁移 | `loop.rs` + `dispatcher.rs` + `scheduler.rs` | 现有 `NeedApproval` 事件 → `InteractionRequested { kind="approval" }`；`ApprovalState` 退化为通用等待态特例 | 旧审批测试不改一行仍过 |
| ④ 通用路由 | `service/src/main.rs` | 新增 `POST /session/{id}/interaction/{iid}` | curl 测试返回 200/400 |
| ⑤ 旧路由适配 | `service/src/session.rs` | `POST /approval` 保留为薄适配层 + deprecated 标记 | 兼容旧 CLI 调用 |
| ⑥ CLI | `codex-cli/src/client.rs` | approve/deny 改用 InteractionResponse（R3 已在 WP-0a 修好） | `codex-cli approve` 可用 |
| ⑦ 门禁门 | 新增 R1/R2 失败用例 | 错 id 必拒 + 重复提交必拒 | 214 tests + 新增全过 |
| ⑧ dummy kind 验证 | — | 新增一个 `kind="test"` 的交互，内核代码改动 = **0 行** | 验证"加交互不改内核" |

### WP-0 验收清单

- [ ] `agent-core` / `tool-runtime` grep 不到 `"approval"` / `"clarification"` 字面量分支（测试名/注释不算分支；`tool_call_needs_approval` 语义函数保留）
- [ ] 现有 221 tests **零回归**（dispatcher 审批测试仅接口改名适配——ApprovalState→InteractionState，语义不变）
- [ ] 新增 dummy kind → 内核 0 行代码改动（`test_wp0_dummy_kind_not_switched`）
- [ ] R1 错 id → 拒（有测试）
- [ ] R2 重复提交 → 拒（有测试）
- [ ] `codex-cli approve` 端到端通过（改用 POST /interaction/{iid}）
- [ ] **对外 API 契约不变**：`AgentEvent::NeedApproval` 事件名保留；`POST /approvals` 保留为薄适配层（deprecated）
- [ ] 交互响应含通用 `resolved: bool`（内核只读它决定放行/中止，不解析 payload）——**对 v23 §2.1 定版的必要补充**（原定版 Response 无 resolved，但内核必须能区分"放行/中止"）

---

## 二、红队剩余修复（按严重度排）

> ⚠️ **本清单为 02:10 版本（历史），各项状态见 §〇 刷新表**——RT2/RT9/WT1/RT6/CT2 已闭环，仅 RT3(seccomp)/N7/CT1 仍挂账。

| 序 | 项 | 严重度 | 内容 | 估时 |
|---|---|---|---|---|
| **前** | CT2 测试自身验证 | 🔴 阻塞 | 随机禁用一个测试 → 套件必须报红。**先做这个——不验证测试能失败，修别的 bug 等于瞎修** | 30min |
| 1 | RT2 civ 三层静默 | 🔴 | `civ_writer=None` 时 drain_civ_alerts → 静默丢弃。EPIC-C 只修了有 writer 的路径，无 writer 路径需要 log warning + 不丢弃 | 1h |
| 2 | RT9 `/readyz` 永远 200 | 🟡 | 无 store 时 `list_persisted_sessions()` 返回 `Ok(vec![])`——探活退化。**记录为已知边界，不修**（已在 acceptance 标注半修） | 文档 |
| 3 | WT1 bash 沙箱逃逸 | 🔴 | window-framework agent 对 `bash` 工具无路径拦截。需要在 `_allowed_write` 前加命令白名单校验 | 2h |
| 4 | RT6 并发限流 | 🟡 | >50 请求 → 429 生效？边界 (49/50/51) 测试 | 1h |
| 5 | RT3 seccomp 扩展 | 🟡 | sandbox seccomp 白名单是否覆盖所有必要 syscall？需 VM 上真跑 | 1h |
| 6 | RT9 会话持久化 | 🟡 | service 重启后 session 状态恢复 | 2h |

---

## 三、阶段优先级与执行顺序

```
批次 A（紧急，2h）：
  CT2 测试自身验证 → WP-0 通用交互原语

批次 B（红队，4h）：
  RT2 civ 三层静默 → WT1 bash 沙箱 → RT6 并发限流 → RT3 seccomp

批次 C（文档 + 低优）：
  N7 密钥复核 + RT9 会话持久化
```

**批次 A 是一天中最高优先**——WP-0 做完后，WP-1~WP-10 全部解封可以并行开工。

---

## 四、门禁

- 每批次完成后：fmt + clippy + test（不得回归）
- WP-0 额外：I2 内核不认 kind 断言
- 红队修复额外：对应的 audit-findings 项标记闭环

---

## 五、给执行窗口的决策授权

| 场景 | 决策 |
|---|---|
| 施工中发现文件锚点偏移 | 以 grep 实际行号为准，更新任务书标注 |
| WP-0 耗时超 6h 仍卡在 step ③前 | 上报——审批迁移路径可能有设计假设错误 |
| 红队剩余项中某一项超过 2h | 记录进展，转入下一批——不要卡整体进度 |
| CT2 验证测试能失败，但某测试永远不会失败 | 标记为 🟡 测试质量缺陷，单独列入 backlog |
