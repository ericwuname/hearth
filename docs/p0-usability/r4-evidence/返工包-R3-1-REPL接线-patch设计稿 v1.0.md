# R4 判读后返工包 · R3-1 REPL 接线 patch 设计稿 v1.0（执行窗备妥，判读后实施）

> **背景**：R4 重放发现 R3-1 收尾三行只在 one-shot 终态渲染（run_local.rs）生效，**REPL 路径漏接**（W-E/G-D/W-D 判读受影响项）。判据冻结期不改代码（sha 锁定 70730e7）；本 patch 备妥，砺判读后一键实施。
> **改动量**：两文件约 40 行。**同口径注意**：patch 实施后引擎 sha 变更，若砺要求补测重跑须用新 sha 登记（或判读认定收尾三行为纯投影增补不构成行为变化——由砺裁）。

---

## Patch 1 · loop.rs — Event::Done payload 扩展（数据源同源化）

**位置**：completed 分支 `self.emit(Event::Done(serde_json::json!({...})))`（当前 :5448 附近，"ok": true / "status": "completed" / "goal" / "steps" 四字段处）。

**新增字段**（与 one-shot report 同源——`ledger_pending_texts()` 访问器 R3-1 已备）：

```rust
self.emit(Event::Done(serde_json::json!({
    "ok": true,
    "status": "completed",
    "goal": goal.text,
    "steps": steps,
    // R3-1 REPL 收尾三行数据源（与 one-shot report 同源事实，非模型自报）
    "artifacts": self.written_files.iter().map(|w| w.path.clone()).collect::<Vec<_>>(),
    "ledger_pending": self.ledger_pending_texts(),
    "completion_decision": self.last_completion_decision.clone().unwrap_or_default(),
    "verification": self.verification_state(),
    "known_failing_open": self.ctx_mgr.state().ledger
        .open_in(agent_types::LedgerColumn::KnownFailing)
        .iter().map(|e| e.text.clone()).collect::<Vec<_>>(),
})));
```

**注意**：failed 路径（Error 臂）如需三行，同样在 Error 臂的 emit 处扩展 `ledger_pending`（failed 时"还剩什么"更重要——**必做**）。

## Patch 2 · repl.rs — Done 分支渲染三行（:206-211）

```rust
"Done" => {
    let steps = sse.data.get("steps").and_then(|s| s.as_u64()).unwrap_or(0);
    let ok = sse.data.get("ok").and_then(|o| o.as_bool()).unwrap_or(false);
    render::done(steps, ok);
    // ── R3-1 收尾三行（REPL 路径返工：与 one-shot 同源事实）──
    if ok {
        let artifacts: Vec<String> = sse.data.get("artifacts")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let pending: Vec<String> = sse.data.get("ledger_pending")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let decision = sse.data.get("completion_decision")
            .and_then(|v| v.as_str()).unwrap_or("");
        let verification = sse.data.get("verification")
            .and_then(|v| v.as_str()).unwrap_or("UNVERIFIED");
        render::info(&format!("  ── 收尾 | 改了什么: {}",
            if artifacts.is_empty() { "本轮无产物落盘".into() }
            else { format!("{}（{} 个）", artifacts.join("、"), artifacts.len()) }));
        render::info(&format!("  ── 收尾 | 还剩什么: {}",
            if pending.is_empty() { "账本无未完成项".into() }
            else { format!("{}（{} 项未完成）", pending.join("；"), pending.len()) }));
        render::info(&format!("  ── 收尾 | 依据: 完成决策 = {}；验证状态 = {verification}",
            if decision.is_empty() { "（无记录）" } else { decision }));
    }
    done_seen = true;
}
```

## Patch 3 · 测试（断言能失败）

1. loop.rs：Event::Done payload 含 `ledger_pending` 且非空（账本有条目时）——旧 payload 必红；
2. repl.rs 渲染为终端 IO（同 RC34 口径 e2e 终验）——单测锁 payload 字段存在即可。

## 实施序

1. 砺判读回执（确认返工）→ 2. 本 patch 实施（~40 行）→ 3. cargo fmt/clippy/test（agent-core+codex-cli）→ 4. REPL 实测一轮收尾三行出现 → 5. 新 sha 登记入 evidence-index（口径：投影增补，行为判据不变——由砺确认）。
