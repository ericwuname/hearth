# Trunk Freeze 分支计划 · codex-rust

> 基线：v2-gaps（6 项缺口已关，gatekeeper-review-v2-gaps.md 过闸）
> 参照：`docs/top-level-design.md` §13（12 缺口）· §14（11 To-Be）
> 产出日期：2026-07-29
> 定位：回答"在冻结主干之前，还剩什么必须关掉的缺口"

---

## 1. 已完成（v1.2 + v2-gaps 两轮关闭，不再重复）

| 缺口 | 修复 | 版本 |
|---|---|---|
| G1 replan 硬封顶 | planner 快/慢双路径 `replan_count>=3→GiveUp` | v2-gaps |
| G3 retriever/lsp 接线 | `RETRIEVER_ENABLED` 门控 + `NoopLspBridge` 显式 | v2-gaps |
| G4 API 鉴权 | `API_KEY` Bearer + `API_KEY_REQUIRED` 强制 | v1.2 |
| G5 CI | `.github/workflows/ci.yml` fmt+clippy+test | v2-gaps |
| G9 sandbox 注释 | execute 注释 + trait `&PathBuf→&Path` | v2-gaps |
| G11 gpt-4o 硬编码 | `OPENAI_MODEL` env | v2-gaps |
| G8 文档断链 | top-level-design.md 替代缺失的 architecture.final.md | 接受 |
| — | 40 项 🔴🟡 安全发现（AUTH-0/OOM-1/MEM-1~5/STREAM-1~4 等） | v1.2 |

**已关闭 8/12 缺口 · To-Be 覆盖率 ≈ 55%**

---

## 2. 剩余缺口（开放，需在本计划中裁决）

去重后（G 与 T 对应项合并），剩余 **5 个实质缺口** + **2 个工程债项**。

### 2.1 🔴 Trunk Freeze 阻塞项（不关不能冻结主干）

| # | 项 | 影响 | 估量级 |
|---|---|---|---|
| **B1** | G6/T2 历史回放 + 文件浏览端点 | 当前无 `GET /sessions/{id}/messages`、无工作区文件浏览。前端只能"实时看 SSE"，无法"回头看历史"，是**类 IDE 体验的硬门槛** | 中（2 端点 + DB 改造 or memory store 复用） |
| **B2** | G7/T3 统一错误码体系 | 当前 `anyhow` 直透给客户端，无 `code` 字段、无错误分类、无 `v2` 迁移策略。API 一旦发布就改不了——**必须在 freeze 前定** | 中（设计 > 代码量） |
| **B3** | T1 OpenAPI / TS 类型生成 | SSE `event:` 通道与 JSON `type` 字段目前手动对齐，前端需手写契约解析。`utoipa` 或 `typeshare` 可生成，消除通道分裂 | 中 |

### 2.2 🟡 建议冻结前完成（收益高、代价小）

| # | 项 | 影响 | 估量级 |
|---|---|---|---|
| **S1** | T10 配置规范化 | `main.rs` 仍有硬编码项：OOM-1 TTL(3600s/300s)、body 上限(2MB 虽已显式)。一两个 env 即可消除 | 小（~5 行） |
| **S2** | G10 telemetry eval 接 T8 CI | `EvalRunner` 已实现且有内部测试，只需在 CI 加一步调用。评估管线建起来，质量可持续 | 小（接一步 CI） |
| **S3** | T11 命名规范 | "P3 技术债"与阶段 P3 同名、A1-A5 含义随阶段变。统一命名表消除歧义 | 极小（文档 1 页） |

### 2.3 ⏸ Post-Freeze 分支（复杂/设计重/非阻塞，冻结后走 feature branch）

| # | 项 | 影响 | 理由 |
|---|---|---|---|
| **P1** | G2/T5 A4 子代理内容级 merge | 多代理协作 | 需 diff/冲突策略设计 + 可能重构 loop.rs。复杂，不适合在 freeze 前 rush |
| **P2** | G12/T9 prompt 注入防御 | 间接注入风险 | 需安全研究 + 方案评审。sandbox + 审批已是事实防线，非零日漏洞 |
| **P3** | — 真实 LSP client（P5） | lsp-bridge 当前 Noop | 已知延后项，top-level-design §6 已登记 |

---

## 3. Trunk Freeze 判定公式

```
可冻结 ⇔ B1+B2+B3 全部关闭
         ∧ S1+S2+S3 关闭或显式声明延后
         ∧ cargo test --all 全绿
         ∧ top-level-design.md 更新至最终态
```

当前 B1/B2/B3 **全部开放** → **不可冻结**。预估工作量为 3×中（B1–B3）+ 2×小（S1–S2）+ 1×极小（S3）≈ **3–5 天**。

---

## 4. 分支策略

```
main (trunk)
│
├── [当前] v1.2 → v2-gaps (8/12 缺口已关)
│
├── [冻结前] B1/B2/B3 + S1/S2/S3
│        ↓ trunk freeze ←── 本计划目标
│        ↓ tag: v3.0-trunk-freeze
│
├── [冻结后 feature branches]
│   ├── feat/a4-content-merge      ← P1 (G2/T5)
│   ├── feat/prompt-injection      ← P2 (G12/T9)
│   └── feat/real-lsp-client       ← P3
│
└── [后续迭代] v3.1 / v4 ...
```

---

## 5. 冻结前详细行动项

### 5.1 B1 — 历史回放 + 文件浏览端点

**目标态**：
- `GET /api/v1/sessions/{id}/messages` → 返回该 session 所有已发送/接收消息（含 LLM 回复 + tool calls + 结果）的 JSON 数组
- `GET /api/v1/sessions/{id}/files?path=<rel>` → 浏览工作区文件树/读取文件内容

**实施方案**：
- 历史回放：`SessionManager` 已通过 `MemoryStore` 持久化事件（`session.rs:340-361`），可复用 `JsonlMemoryStore::load_session` 或直接在内存中维护一个 `Vec<Message>` 管道
- 文件浏览：`ReadTool` 已有实现，新增路由调用 `ReadTool::execute` 或直接用 `tokio::fs::read_to_string`（cwd 受限 + 路径穿越检查）

**文件**：`crates/service/src/routes.rs` + `crates/api/src/lib.rs`（新 DTO）

**验证**：`curl localhost:3000/api/v1/sessions/{id}/messages` 返回 JSON 数组

---

### 5.2 B2 — 统一错误码体系

**目标态**：
```json
{ "error": { "code": "INVALID_API_KEY", "message": "...", "details": null } }
```
定义 8–12 个错误码覆盖：认证/资源未找到/参数校验/审批超时/LLM 错误/沙箱错误/服务内部错误。

**实施方案**：
- `api` crate 加 `ErrorResponse{code, message, details}` struct
- `routes.rs` 统一 `Result<T, (StatusCode, Json<ErrorResponse>)>` 返回
- 现 `anyhow::bail!` / `anyhow::Result` 逐步替换

**文件**：`crates/api/src/lib.rs` + `crates/service/src/routes.rs`

**设计决策**：需确定 code 命名空间（`AUTH_*` / `SESSION_*` / `TOOL_*` / `LLM_*` / `INTERNAL`）。建议简表：
| code | HTTP status | 含义 |
|---|---|---|
| `UNAUTHORIZED` | 401 | 缺/错 API_KEY |
| `SESSION_NOT_FOUND` | 404 | session_id 不存在 |
| `INVALID_PARAM` | 400 | 请求参数非法 |
| `APPROVAL_TIMEOUT` | 408 | 审批等待超时 |
| `LLM_ERROR` | 502 | LLM 后端错误 |
| `TOOL_ERROR` | 500 | 工具执行失败 |
| `SANDBOX_ERROR` | 500 | 沙箱限制 |
| `INTERNAL` | 500 | 未知内部错误 |

---

### 5.3 B3 — OpenAPI / TS 类型生成

**目标态**：`cargo build` 时自动生成 `openapi.json` + `types.d.ts`。

**实施方案**：
- `utoipa` crate：在 `api` crate 的 struct/enum 上加 `#[derive(ToSchema)]`，在 `service` 加 `#[derive(OpenApi)]`
- 或 `typeshare`：直接从 Rust 类型生成 TS interface
- SSE `event:` 映射在文档中以注释标注

**文件**：`crates/api/Cargo.toml`（加 utoipa dep）、`crates/api/src/lib.rs`（加 derive）、`crates/service/src/main.rs`（挂 /openapi.json 端点）

---

### 5.4 S1 — 配置规范化

**硬编码项 → env**：
- `OOM_TTL_SECS`（默认 3600）
- `OOM_SWEEP_INTERVAL_SECS`（默认 300）

已在 `main.rs` 显式声明的留原样（`body_limit` 2MB 已有注释说明）。

---

### 5.5 S2 — telemetry eval 接 CI

**行动**：`.github/workflows/ci.yml` 加一步：
```yaml
- name: eval
  run: cargo test -p telemetry -- --nocapture
```
`telemetry` 已有 `test_p5_eval_harness_regression`（2 个硬断言），直接跑即生效。

---

### 5.6 S3 — 命名规范

**行动**：在 `top-level-design.md` §12 追加"代号冲突解疑"章节：明确 `P3(技术债) ≠ P3(阶段)`，建议技术债改用 `TD-*` 前缀。

---

## 6. 冻结后 Post-Freeze 分支简要

| 分支 | 内容 | 输入 |
|---|---|---|
| `feat/a4-content-merge` | A4 子代理结果 diff/冲突 → 写入观测 | top-level-design G2/T5 |
| `feat/prompt-injection` | 工具结果标记/隔离 + 威胁模型文档 | top-level-design G12/T9 |
| `feat/real-lsp` | rust-analyzer subprocess 真 LSP client | top-level-design P5 延后项 |

---

## 7. 结论

**当前状态**：8/12 顶层缺口已关闭，To-Be ≈ 55%。可运行、可部署、安全面翻绿。

**不可冻结**：3 个 🔴 阻塞项（B1 历史回放、B2 错误码、B3 OpenAPI）全开放。这三项都是"契约层"问题——不关就冻结，API 一发布就没法改了。

**推荐路径**：3 个阻塞项 + 2 个小优化（S1/S2）作为"冻结前最后一轮"集中冲刺，预计 3–5 天。完成后 trunk freeze → tag v3.0 → 开 feature branches。
