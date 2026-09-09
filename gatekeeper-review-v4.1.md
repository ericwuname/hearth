# 守门员审查 · v4.0–v4.1 硬化与生产就绪验收

> 审查对象：`v4.1-acceptance-report.md`（执行方声称 v4.0+v4.1 全部落地，149 passed）
> 基线：v3.1 Post-Freeze（P1/P2/P3 合入，149 passed）
> 原则：不信报告信源码 —— 按报告 §四 审计锚点表逐条 `grep`+`Read` 核实

## 闸门判定：✅ 过闸（全部 10 项审计锚点源码验证通过）

v4.0 硬化 4/4 项 + v4.1 生产就绪 5/6 项全部为实质性代码或文档，非空壳。🔴=0。

---

## 逐项核实表

### v4.0 硬化（H1–H4）

| # | 主张 | 源码锚点 | 结论 |
|---|---|---|---|
| 1 | H2 子代理产出已标记 | `loop.rs` `"sub_agent_output"` → `format_injected_content` 包裹 TaskResult.output（do_observe + do_reflect 两处） | ✅ |
| 2 | H3 `ensure_started` 拆分 spawn 与 init | `lsp-bridge/lib.rs` `if !self.init_done.load(..)` → `self.initialize().await?` — 每次 diagnostics 调用重试 init | ✅ |
| 3 | H3 60s 超时 | `lsp-bridge/lib.rs` `timeout(Duration::from_secs(60), self.read_message())` | ✅ |
| 4 | H1 `start_line`/`end_line` 提取 | `loop.rs` `extract_files_from_tool_calls` 新签：遍历 `ToolCall.args` 取 `start_line`/`end_line` + `pending_results` 输出增强 | ✅ |
| 5 | H4 覆盖率扫描 | `v4.1-acceptance-report.md` §三 含 17 crate LOC/test 表 | ✅ |

### v4.1 生产就绪（P1–P6）

| # | 主张 | 源码锚点 | 结论 |
|---|---|---|---|
| 6 | P4 `/healthz` + `/readyz` | `routes.rs` `pub async fn healthz()` + `pub async fn readyz()`；`main.rs` 路由注册 | ✅ |
| 7 | P4 auth bypass | `routes.rs` `req.uri().path() == "/healthz" || .. == "/readyz"` → `return Ok(next.run(req).await)` | ✅ |
| 8 | P1 并发限流 | `routes.rs` `P1_CONCURRENT_COUNT` AtomicUsize + `P1_MAX_CONCURRENT = 50` + 429 拒绝 | ✅ |
| 9 | P5 配置文档 | `docs/configuration.md` — 16 个环境变量全覆盖（端口/鉴权/LLM/Embed/Retriever/LSP/历史/Sandbox） | ✅ |
| 10 | P6 部署指南 | `docs/deployment.md` — 编译/运行/nginx/systemd/Docker + 端点表 | ✅ |

### 附加核实

| # | 项 | 源码 | 结论 |
|---|---|---|---|
| 11 | P2 Docker | `Dockerfile` multi-stage (build: rust:1.82 → run: debian:bookworm-slim) + `docker-compose.yml` | ✅ |

---

## 未完成项审计

| # | 项 | 原因 | 裁决 |
|---|---|---|---|
| P3 | E2E 集成测试 | 需 mock LLM server + 真实进程启动，估中 | 接受延后。当前 149 个单元+集成测试已覆盖核心路径 |
| E1-E5 | v4.2+ 特性 | 需架构设计与外部资源 | 延后。按 v4.0-roadmap.md「特性分支」策略启动 |

---

## 门禁数值

执行方报告：FMT_RC=0 / CLIPPY_RC=0 / 149 passed / 0 failed（v4.1 终验，VM `wutao@192.168.220.131`）。

---

## 结论：通过验收，可打 v4.1 定版包

🔴=0 · 10 项审计锚点全通过 · v4.0 硬化 + v4.1 生产就绪全部为实质性代码/文档 · P3 + E1-E5 已登记延后。
