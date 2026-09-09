# 顶层规划差距分析 · v1.2 基线

> 基准文档：`docs/top-level-design.md`（2026-07-28）
> 分析方法：逐项对撞文档 §13（12 缺口）与 §14（11 To-Be），每个出具"v1.2 已覆盖什么 / 还差什么 / 估量级"

---

## 总览

| 维度 | 完成度 | 说明 |
|---|---|---|
| §1–12 as-built 现状 | **100%** | 文档本身就是 v1.2 源码的真实写照，§1–12 全部带 `文件:行号` 可复核 |
| §13 已知缺口 | **7/12 关闭**（G8 已接受 + v2-gaps 闭环 G1/G3/G4/G5/G9/G11） | G8（TRACE-1 注释 + 本文档替代 architecture.final.md）；v2-gaps 新增 G1(replan 硬封顶)/G3(retriever+lsp 接线)/G4(API_KEY_REQUIRED)/G5(CI)/G9(注释+trait)/G11(OPENAI_MODEL) |
| §14 To-Be 路线图 | **5/11 已闭环**（T4/T6/T7/T8/T10 随 v2-gaps 落地；T8 的 G10 eval 待接） | T1/T2/T3/T5/T9/T11 未开工 |

---

## §13 缺口逐项 delta

| # | 缺口 | v1.2 进展 | 还差什么 | 估量级 |
|---|---|---|---|---|
| G1 | replan 无严格硬封顶 | **✅ 已闭环(v2-gaps：T6 第 4/6 路加 `replan_count>=3→GiveUp` 硬封顶)** | `planner/lib.rs:186/205` 第 4/6 路加 `replan_count` 上限，或改 Reflect→GiveUp 兜底。现仅靠 consecutive_errors 间接兜底 + max_steps 预算硬顶 | 小（1–2 行） |
| G2 | A4 子代理结果无内容级 merge | **无** | `loop.rs:842-846` 加 diff/冲突策略；或明确声明"基于共享文件系统的隐式 merge"为设计意图 | 中（需架构决策 + 50–200 行） |
| G3 | retriever/lsp 生产未接线 | **✅ 已闭环(v2-gaps：T4 `main.rs` 接线 `set_retriever` + `NoopLspBridge`)** | `main.rs` 两行 `set_retriever` / `set_lsp_bridge` 调用（lsp 需先换 Noop→Real LspBridge）；或决策退役 | 小（接线 2 行，但 lsp 真实实现需 P5） |
| G4 | API 默认无鉴权开放 | **✅ 已闭环(v2-gaps：`API_KEY_REQUIRED=1` 强制鉴权，缺 `API_KEY` 时 refuse 启动；G4 完全闭环)** | v1.2 AUTH-0 已加 Bearer 中间件（`routes.rs:28-58`），机制存在；但 `API_KEY` 未设时全开放，仅 WARN | 生产部署设 `API_KEY=xxx` 即可关闭；或改代码让未设时拒绝启动（更安全，但 break dev 体验） | **近乎零**（改一个 env var 即闭环） |
| G5 | 无 CI | **✅ 已闭环(v2-gaps：`.github/workflows/ci.yml` fmt / clippy -D warnings / `cargo test --all`)** | 建 `.github/workflows/ci.yml`：fmt / clippy / `cargo test --all`（需 Linux runner for sandbox） | 中（配置 + runner，估半天） |
| G6 | 无历史回放/文件端点 | **无** | 补 `GET /sessions/{id}/messages` + 工作区文件浏览端点 | 中（新端点 + SSE/polling 设计） |
| G7 | 无 API 版本策略/错误码 | **无** | 统一 `ErrorResponse{code, message}`；定义 `/api/v2` 升级策略 | 中（设计 > 代码量） |
| G8 | 事前文档断链 | **已接受**：v1.2 TRACE-1 修复 `api/lib.rs:8` 注释（"本 enum 即契约"）；**本文档本身就是 architecture.final.md 的替代品** | 无代码动作；`execution-plan.md` 如确需可从阶段报告反推 | **零**（本文档已闭环） |
| G9 | writable_paths 含 execute 与注释不符 | **✅ 已闭环(v2-gaps：`sandbox/lib.rs:207` 注释改"含 execute(FS_RO)"；trait `cwd: &Path` 重构)** | `sandbox/lib.rs:206` 注释改为"含 execute（FS_RW 继承自 FS_RO）"，或改 sandbox 路径权限剥离 execute | 极小（1 行注释 or 小代码改动） |
| G10 | telemetry eval 孤儿（0 调用） | **无**（v1.2 列为 P3 技术债） | 接线：接 CI 或 `cargo test` 集成，或决策退役并删除 crate | 小~中（CI 集成） |
| G11 | OpenAI 模型名 gpt-4o 硬编码 | **✅ 已闭环(v2-gaps：`OPENAI_MODEL` env var，默认 gpt-4o，含启动日志)** | 加 `OPENAI_MODEL` env var，默认 `gpt-4o` | 极小（2 行） |
| G12 | prompt 注入无专门防御 | **无** | 工具结果标记/隔离，或依赖 sandbox + 审批组合已是事实防线。需设计决策 | 中~大（设计 > 代码量） |

---

## §14 To-Be 路线图逐项状态

全部 **未启动**。v1.2 聚焦于安全加固（🔴 门禁），三波路线图尚未开始执行：

| 波次 | 行动项 | 状态 | 估量级 |
|---|---|---|---|
| 第一波 契约 | T1 OpenAPI/TS 类型生成 | ⏳ 未开工 | 中 |
| | T2 历史回放 + 文件浏览端点 | ⏳ 未开工 | 中 |
| | T3 统一错误码 | ⏳ 未开工 | 中 |
| 第二波 增强 | T4 main.rs 接线 retriever/lsp | ✅ 已闭环(v2-gaps) | 小 |
| | T5 A4 子代理 merge | ⏳ 未开工 | 中 |
| | T6 replan 硬封顶 | ✅ 已闭环(v2-gaps) | 小 |
| | T7 修正 execute 注释 | ✅ 已闭环(v2-gaps) | 极小 |
| 第三波 规范 | T8 CI + eval | ✅ 已闭环(v2-gaps：CI 已建；G10 eval 待接) | 中 |
| | T9 威胁模型 + prompt 注入 | ⏳ 未开工 | 中~大 |
| | T10 配置规范化 | ✅ 已闭环(v2-gaps：G11 OPENAI_MODEL) | 小 |
| | T11 路线图命名规范 | ⏳ 未开工 | 极小 |

---

## 推荐推进顺序

v1.2 刚过闸（安全面翻绿），现在是回补顶层缺口的好时机。按 **收益/成本比** 排：

### 立即可做（< 1 小时，纯代码，无设计决策）

| 次序 | 项 | 动作 | 状态 |
|---|---|---|---|
| ① | G11 | 加 `OPENAI_MODEL` env var，默认 `gpt-4o` | ✅ 已闭环(v2-gaps) |
| ② | G9 | 修正 `sandbox/lib.rs:206` 注释与实现一致 | ✅ 已闭环(v2-gaps) |
| ③ | G4 | 不是代码问题——部署时设 `API_KEY` 即关闭。可加 `API_KEY_REQUIRED=true` 强制鉴权（替代当前"不设即开放"） | ✅ 已闭环(v2-gaps) |
| ④ | T6 | `planner/lib.rs:186` 第 4 路加 `if replan_count >= 3 { ReflectVerdict::GiveUp }`（1 行） | ✅ 已闭环(v2-gaps) |

### 半天可完成（有设计但代码量小）

| 次序 | 项 | 动作 |
|---|---|---|
| ⑤ | T4 | `main.rs` 接线 retriever（2 行）。lsp 暂时保持 Noop（真实 LSP client 延后 P5，与 §6 注释一致） | ✅ 已闭环(v2-gaps) |
| ⑥ | G5 | 建 `.github/workflows/ci.yml`（fmt + clippy + `cargo test --all`） | ✅ 已闭环(v2-gaps) |

### 需架构决策（建议下次迭代一起做）

| 次序 | 项 | 涉及 |
|---|---|---|
| ⑦ | G1+G2 | replan 硬封顶 + A4 merge——与 loop.rs 整体可靠性有关，值得一次 mini-phase 集中处理 |
| ⑧ | G6+T1+T2+T3 | API 契约层——历史回放、文件端点、OpenAPI 生成、错误码——这些是一组关联改动，可打包为"API v2 规范"阶段 |

---

## 结论

**v1.2 是一次安全装甲的全面安装，不是一次功能补全。** 顶层规划的 12 项缺口与 11 项目标全部停留在登记态。好消息是缺口集中、大部分估量级小（4 项"极小"、4 项"小"）——不是"要重写核心"，而是"补遗和连线"。按推荐顺序推进，**前 6 项合计约 1 天**即可从 0/11 拉到 6/11 ≈ 55% To-Be 覆盖率。
