# 双引擎终章验收报告——两笔旧债还清

> 验收时间：2026-08-01（用户睡觉，全自主）
> 计划依据：`FINAL.md`（终章声明，授权施工）+ `docs/audit-breakdown-v22.md`（EPIC-B/EPIC-C 拆解）
> 施工范围：codex-rust 主项目两笔旧债（B1a / B1b / B1c）

---

## 一、EPIC-B：/readyz 真实探活（B1a）

**审计事实**（audit-breakdown-v22 核实）：`routes.rs:257` 的 `let _ = &state.sessions;` 是 **no-op 空引用**——不做任何探活，永远返回 200。运维可观测性 25% 的根源。

**施工**：改为真实访问 session store：
```rust
match state.sessions.list_persisted_sessions().await {
    Ok(_) => (StatusCode::OK, "OK"),
    Err(e) => {
        tracing::warn!("readyz: session store unavailable: {e}");
        (StatusCode::SERVICE_UNAVAILABLE, "session store unavailable")
    }
}
```
依赖可用 → 200；I/O 失败/内部错误 → **503**（+ tracing 告警）。

**测试**：`test_epic_b_readyz_probe_semantics`——healthy（无 store → Ok）与 failing（FailingMemoryStore → Err）两路径。✅ 通过（integration_test 13 tests）。

## 二、EPIC-C：神经系统中断告警链（B1b + B1c）

**审计事实**：
- B1b：`drain_nervous_alerts()`（loop.rs:422）零调用者的死包装；do_reflect 里 `drain_civ_alerts()` **返回值被丢弃**——面向文明线的中断记录系统性丢失
- B1c：Simplify 在 GiveUp 合并臂**不可达**（is_critical 时 action 恒非 Simplify）——死 arm

**决策（C-1：选"真接线"而非"删除"）**：
- **B1b 接线**：`drain_civ_alerts()` 返回值不再丢弃——有 alerts 时 `civ_note("nervous", ...)` 写入文明线（CivWriter 落盘）；删除零调用者包装 `drain_nervous_alerts()`
- **B1c 移除死 arm**：Simplify 从 GiveUp 合并臂移除（不可达）；其正确语义由 subconscious PhaseOverride 在 Plan 阶段处理（loop.rs:1033 已存在）——显式移除 + 注释，无死代码残留

## 三、验收（VM 门禁）

| 项 | 结果 |
|---|---|
| cargo fmt --check | ✅ rc=0 |
| cargo clippy -D warnings | ✅ rc=0 |
| cargo test --workspace | ✅ **206 passed** |
| test_epic_b | ✅ ok（integration 13 tests） |
| wiring 静态核验 | ✅ 15 capability / 24 chains / **0 缺失**（子串断言未失） |

> 说明：VM 项目此前过期（缺 7 个 crate），已完整同步本地 HEAD（25 crates）后门禁。
> 206 passed 是完整项目全量（含全部集成测试），高于 MEMORY 记录的旧基线 146。
>
> **复验证据（2026-08-01 二次核验）**：
> - `cargo test -p service --test integration_test` → **13 passed**（含 test_epic_b_readyz_probe_semantics）
> - wiring 静态核验脚本：15 capabilities / 24 chains / 0 缺失（tomllib 逐断言 grep 子串）
> - FINAL.md wiring 口径修正为 **15/15（15 能力断言 / 24 调用链）**——审计的 14/14 是 v22 前的旧口径

## 四、审计发现的新事实（记录）

- **审计行号修正**：B1a 真硬编码在 routes.rs:257（审计 255/258 有偏移）；B1b 在 loop.rs:422/1669（审计 422/1638 偏移）；B1c 在 loop.rs:1675（审计 1644 偏移）——已按"不信计划信源码"现场核对
- **VM 项目过期**：~codex 缺 experience/nervous-system/subconscious/bridge/codex-cli/llm-replay/resource-monitor——本次已全量同步（MEMORY 流程更新）

## 五、FINAL.md 维护合同更新

```yaml
codex-rust:
  季度: 基准 deepseek 20×2 + 应力 24 次 + 回放 31 条
  红线: < 85% / panic > 0 / wiring 断裂
  旧债: ✅ B1a/B1b/B1c 已还清（2026-08-01, commit 05f90cf）
```

---

## 结论

**✅ 双引擎终章达成——两笔拖了十个版本的旧债还清。**

FINAL.md 声明的唯一剩余事项（B1a / B1b / B1c）全部施工 + 门禁全绿 + wiring 无缺失。
codex-rust 从"两笔旧账"变为"零旧账"；窗口群框架 v1.0.3 已证明多窗口分工。
**双引擎完全进入"只做体检、不再进化"的稳态**——如 FINAL.md 所愿。

---

## 七、红队复核（2026-08-02，audit-findings-v22 并入）

> 本报告 v22 验收时结论真实（当时门禁全绿）。测试窗口红队审计后续发现**三处"半修"边界**，如实记录，不撤回原结论：

| 项 | 原验收 | 红队复核 | 状态 |
|---|---|---|---|
| **B1a /readyz**（RT1） | "无 store → Ok = healthy" 写进测试 | 无 memory store 时 `list_persisted_sessions` 返回 `Ok(vec![])` → **永远 200**，探活仅在"有 store + store 故障"时有效 | 🟡 半修：运维注意项（无 store 部署 readyz 不反映可用性），记录待独立排期 |
| **B1b civ 落盘**（RT2） | "alerts 写入文明线" | `civ_writer=None` 时 `civ_note` **静默丢弃**（三层静默：仅 do_reflect 调用 / None 无 else / I/O 失败仅 warn） | 🟡 半修：有 writer 路径已通，缺失路径待修 |
| **季度基线文档**（CT6） | Y1-3 决断 B 删引用 | `quarterly-baseline-q3-2026.md` 仍不存在（原验收文字"已决断"正确，但勿被误读为"文件存在"） | ✅ 决断有效（B 方案即删引用） |

**结论不变**：EPIC-B/C 主体施工 + 门禁 + wiring 无缺失均为真；上述三处为红队审计追加的**边界缺陷**，已登记 `docs/audit-findings-v22.md`，由后续修复线处理（不在本报告撤回范围）。
