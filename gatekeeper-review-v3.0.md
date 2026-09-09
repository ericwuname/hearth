# 守门员审查 · v3.0 主干冻结验收

> 审查对象：`v3.0-acceptance-report.md`（执行方声称 5 项落地 + 三门全绿 146/0）
> 基线：v2-gaps 定版
> 原则：不信报告信源码 —— 每项结论来自 `grep`+`Read` 直读真实源码
> 环境限制：本机无 Rust 链，未执行 `cargo test`（门禁数值来自执行方报告，守门员仅做源码级核实）

## 闸门判定：✅ 过闸（源码级验证）

5 项全部在源码中确认存在且逻辑正确。🔴=0。

---

## 5 项逐条核实

| # | 项 | 源码证据 | 结论 |
|---|---|---|---|
| **B1** | 历史回放端点 | `session.rs:29` `pub events: Vec<AgentEvent>`；`session.rs:115` `fn get_history`；`session.rs:648` `fn agent_event_kind`；9 处 `events.push`（:350/360/368/466/471/479/484/502/507）；`routes.rs:167` `get_session_history`；`main.rs:280` GET 路由挂载 | ✅ |
| **B2** | 统一错误码 | `api/lib.rs:108` `ErrorResponse`+`ErrorBody` + `:129` 起 8 个 `ERR_*` 常量；`routes.rs:17` `fn api_err` helper；全部 6 个 handler + `require_api_key` 返回类型统一为 `(StatusCode, Json<ErrorResponse>)` | ✅ |
| **B3** | OpenAPI | `main.rs:39` `#[derive(OpenApi)] ApiDoc` + `:47` `openapi_json` handler + `:284` `/openapi.json` 路由；`agent-types/lib.rs:188` `Budget` + `ToSchema`；`api/lib.rs` 9 个 DTO 加 `ToSchema`；`service/Cargo.toml` 仅 `utoipa="5"`（**无** `utoipa-swagger-ui`）；`main.rs:283` 注释说明 SwaggerUI 移除原因 | ✅ |
| **S1** | TTL env | `main.rs:209` `OOM_TTL_SECS` + `:214` `OOM_SWEEP_INTERVAL_SECS` | ✅ |
| **S2** | eval CI | `.github/workflows/ci.yml:36-39` eval step `cargo test -p telemetry` | ✅ |

---

## B3 SwaggerUI 移除核查

报告 §三.B3 声明"SwaggerUI 因 VM 无 github 出站、编译期 fetch 失败而移除，改为直接暴露 `/openapi.json`"。守门员核实：

- `service/Cargo.toml` 无 `utoipa-swagger-ui` 依赖 ✅
- `main.rs` 无 `SwaggerUi` 引用 ✅
- `main.rs:283` 注释解释下载失败 + 离线 vendor 方案 ✅
- `main.rs:47-51` `openapi_json` handler 直接返回 `ApiDoc::openapi().to_json()` ✅

**裁判**：偏离合理。项目长期记忆已有"`utoipa-swagger-ui` build.rs 从 github 拉资源、VM 不可达 → 404 编译失败"的前例（B3 OpenAPI 场景已验证）。当前方案（`/openapi.json` 路由暴露 JSON）实质目标已达成。Swagger UI 交互页为 nice-to-have，不影响冻结。

---

## 执行方自披露项审查

| 披露 | 守门员裁决 |
|---|---|
| B1 `get_history` 无单独断言测试 | **承认**：由编译 + 路由接线 + 三门全绿保证。建议后续补单测覆盖 in-memory 与 MemoryStore 两分支，但不阻塞冻结 |
| B3 SwaggerUI 偏离原方案 | **已核实**：VM 无 github 出站 → 编译期 fetch 必然失败。当前 `/openapi.json` 方案等价，可接受 |

---

## 诚实声明

- **门禁数值**：执行方报告 `FMT_RC=0 / CLIPPY_RC=0 / 146 passed 0 failed`。守门员未在真 Linux VM 复跑（本机无 Rust）。建议用户在 VM 复验一次做最终确认。
- **CI yml**：S2 eval step 已加入，YAML 语法正确，但未经 GitHub Actions 实跑（仓库未推送或 VM 无 github）。

---

## 结论：通过验收，建议定版 v3.0 Trunk Freeze

🔴=0 · 5 项全部源码验证 · B3 偏离合理文档化 · 三门绿 146/0。

冻结后建议：
1. `top-level-design.md` §13 G6/G7 标记 ✅，§14 T1/T2/T3/T8/T10 标记 ✅
2. 打 v3.0 定版备份包
3. 按 `trunk-freeze-branch-plan.md` 开 `feat/a4-content-merge` / `feat/prompt-injection` / `feat/real-lsp` 三个 post-freeze 分支
4. VM 复跑一次最终 gate 确认
