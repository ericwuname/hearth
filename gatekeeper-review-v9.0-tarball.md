# 守门员审查 · v9.0 Final Tarball 交付验收

> 对象：`codex-rust-v9.0-final.tar.gz`（外部交付包）
> 方法：Python 实算硬指标 + grep 抽查 v7.0–v9.0 关键 feature + 与当前工作区对比

## 硬指标（Python 实算，非报告数字）

| 指标 | Tarball (v9.0) | 工作区 (current) | Δ |
|---|---|---|---|
| LOC | 18,443 | 18,641 | +198 |
| `.rs` 文件 | 50 | 52 | +2 |
| 测试函数 | 156 | 156 | 0 |
| Crate 数 | 19 | 20 | +1 (resource-monitor) |

## v7.0–v9.0 Feature 抽查

| 版本 | 功能 | 源码证据 | 结论 |
|---|---|---|---|
| v7.0 | TelemetryCollector | `routes.rs:34/404` + loop.rs structured log (step/phase/sid) | ✅ |
| v8.0 | UserStore + PerUserStore | `user.rs` + `per_user.rs` + `main.rs` 集成 | ✅ |
| v8.0 | TemplateManager | `routes.rs:38/433` + `templates.rs` | ✅ |
| v8.0 | WebhookManager | `routes.rs:39/441` + `webhook.rs` | ✅ |
| v9.0 | TaskOrchestrator | `orchestrator.rs` + `lib.rs` 导出 | ✅ |

## 闸门：✅ 过闸

Tarball 与工作区**核心一致**（差异仅 +198 LOC 来自工作区的 v10.0 起步工作：resource-monitor crate + agent-core/codex-cli/service 增量改动）。

Tarball 交付包完整，19 crate 全齐，156 测试全在，文档配套完整（含新增 architecture-summary.md / resilience-fmea.md / skills-pdca-design.md / trunk-qualification-audit.md）。
