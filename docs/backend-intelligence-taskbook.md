# Hearth 后端真智能 — 执行窗口派遣任务书（B2/B3 质量审计 + 闭环 + CLI 呈现）

> **签发**：顶层（守门员）| 日期：2026-08-22
> **前置已交付**：RT3 seccomp（bcb2080/3beee2e）· Hearth CLI 派 A（9bc2715/bc00047，验收 c30ef3e 后续）
> **设计依据**：`docs/backend-taskbook-01.md`（B1-B4 原始规划，B1 已交付）· `docs/hearth-cli-design.md` · `docs/hearth-roadmap-next.md`（第一梯队=后端真智能）
> **用户比喻锚定**：前端/CLI/桌面=冰山一角，后端=冰山本体；本轮是"水下本体"的核心智能层
> **用户最终期望**：项目安全平稳落地，使用过程不出现意想不到的报错/风险

---

## 0. 守门员核验后的真实现状（避免任务书凭印象）

本轮执行前，顶层独立读源码确认——**B2/B3 不是从零，而是"已实现 + 单测达标"，缺的是"真实任务质量审计 + 端到端闭环验证 + CLI 呈现"**：

- **B2 推导器已真接线**：`crates/planner/src/lib.rs:17 derive_gaps` 存在，门禁契约"每条 gap 必带 `from`+`why`（构造器强制）；`blocking=false` ⇒ `auto_assumed=true`"；`loop.rs:1033-1061` 已 emit `PlanDraft{steps,gaps_found,gaps_to_ask,auto_assumed}`；单测 `test_derive_gaps_*`（821-855）真在。
- **B3 内核链路已真接线**：`loop.rs:1011` 规划期 emit `InteractionRequested{kind:"clarification"}`；`agent-runtime/session.rs:834-864 resolve_interaction` 真在，`:1013-1015` 把 `InteractionRequested`→`NeedApproval`（与契约 §3 吻合）；`run_local.rs:247-263` CLI 直跑已接 approval 内联裁决。
- **缺口**：① 10 个真实任务上 gap 质量未审计（单测是合成输入，非真任务）；② clarify"批量 emit→收齐→resume"在 CLI 直跑端到端未验；③ `plan.draft`/`gap` 在 CLI 上**无可见呈现**——用户跑 `hearth chat` 看不到"规划草案 + 哪些要问你 + 哪些它自己假定了"，产品红线"AI 不得问废话"不可见。

**因此本轮重心 = 质量审计（B2-A）+ 闭环端到端（B3-A）+ CLI 规划呈现（B2-B），不是重写推导器。**

---

## 1. 本轮交付清单（按依赖排序）

### B2-A 真实任务 gap 质量审计（最高优先，产品红线落地证据）
- 选 **10 个真实、多样的任务**（覆盖：写代码/改 bug/查资料/重构/配置/测试/解释概念/多文件/模糊意图/跨语言），用 `hearth chat` 真实跑（VM Linux，真 LLM）。
- 对每个任务的 `plan.draft` 做审计表：每一条 gap 是否带**可追溯 `from`（来自哪步规划）** + **具体 `why`（不答的代价/返工面）**；`blocking=false` 项是否带 `assume` 默认值。
- **硬闸门**：10 任务 gap **100% 带 from+why**；`blocking=false` 项 **100% 带 assume**；无"你有什么要求吗"类通用问题（为问而问即退回）。
- 输出 `docs/b2-gap-audit-10tasks.md`（审计表 + 反例若有）。若发现 `derive_gaps` 在真任务上漏 from/why，回 `planner` 补强制逻辑（构造器已强制，重点查 LLM 返回的 gap JSON 解析路径是否丢字段）。

### B2-B `plan.draft` / gap 在 CLI 的可见呈现（产品红线可见化）
- `hearth chat` 规划相位渲染一段"规划草案"摘要：列出 steps 数、将问你的 blocking gap（带 why）、它自己假定的 non-blocking gap（带 assume）。
- **事实产生权**：CLI 只投影 `plan.draft` 语义字段（steps/gaps/assume），**不**从 BE 拿颜色/HTML/坐标；呈现样式 CLI 自决。
- 红线：`plan.draft` 字段来自契约 `ai-os-event-contract-v1.md`，不得为呈现改 BE 事件结构（要改升 schema_version）。

### B3-A clarify 闭环端到端验证（CLI 直跑）
- 构造 1-2 个**真会触发 blocking clarify** 的任务（如"用 Rust 或 Go 实现"二选一），跑 `hearth chat`：内核 emit `InteractionRequested{kind:"clarification"}` → CLI 打印问题 → 用户答 → `resolve_interaction` 收齐 → resume 后续链路，**不丢事件、不重复问、不卡死**。
- **硬闸门**：端到端日志证明"emit→等→答→resume"链路完整；G4 断线重连不丢（若 VM 可复现断线则验，否则 INCONCL 单列）。
- 验证 approval 同理（危险操作前 emit `need_approval`，CLI 内联 y/N，裁决后续跑）。

### D-extra（顺手，来自 CLI 验收 🟡1）：service 路径占位 key fail-closed 对齐
- `crates/service/src/main.rs:127-128` 的 `sk-placeholder` 改成交互失败（无 key → 启动失败报原因），与 CLI 路径 D2/D4 红线对齐；或明确标注"service 路径不推荐、仅独立部署用"。
- 这是 CLI 验收遗留 🟡1，本轮顺手清，避免"保留的独立部署能力带反模式"。

---

## 2. 门禁（硬闸门，守门员独立验收）

- **R1（B2-A 质量）**：`docs/b2-gap-audit-10tasks.md` 提交；10 任务 gap 100% from+why；non-blocking 100% assume；无通用问题。
- **R2（B2-B 呈现）**：`hearth chat` 规划相位真实打印"规划草案摘要"（steps + 将问 + 自假定），非源码注释；事实产生权合规（BE 无颜色/HTML）。
- **R3（B3-A 闭环）**：clarify 端到端日志（emit→答→resume 完整）；approval 同；无事件丢失/重复/卡死。
- **R4（D-extra）**：`service` 路径无 key → 启动失败（非占位静默）；或显式标注不推荐。
- 通用：`fmt --check` 0 / `clippy -D warnings` 0 / `test --workspace` 全过（baseline 248 + 新增）。

---

## 3. 红线（执行窗口不得违反）

- 禁为"过测"在 BE 硬编码前端展示细节（颜色/HTML/坐标）——事实产生权。
- 禁静默破坏 `ai-os-event-contract-v1.md`（未升 schema_version 改事件结构）。
- **规划缺口推导器禁输出无 `from`/`why` 的通用问题**——产品红线，B2-A 硬闸门。
- 禁把 `blocking=false` 项当 blocking 打断用户（反向违反 auto_assumed 语义）。
- 禁 Observer 越权（本轮不涉及 Observer 改动，但不得借机改其零执行权）。
- 禁删 `service` 独立部署（D-extra 仅改其 key 失败语义）。
- 不引重依赖。

---

## 4. 交付物（回顶层验收时提交）

- `docs/b2-gap-audit-10tasks.md`（B2-A 审计表）
- B2-B / B3-A 代码 commit（CLI 规划呈现 + 闭环验证）
- D-extra commit（service 占位 key 对齐）
- 验收报告 `docs/acceptance-backend-intelligence-<日期>.md`（含 R1-R4 实测证据 + 端到端日志 + 10 任务审计表）

**节奏**：按 B2-A → B2-B → B3-A → D-extra 施工，每批跑门禁，交证据后回顶层（守门员）独立验收。CLI 已 PASS，本任务可立即开工。

---

## 5. 与路线图/挂账的关系

- 本任务是 `hearth-roadmap-next.md` **第一梯队（后端真智能）**的落地——直接决定"AI 是否值得委派"。
- **B4-2 桌面版**（第二/三梯队交界）不属本轮；本轮只验证 CLI 呈现规划草案，桌面版在路线图第三梯队单独排期。
- cgroup fail-closed / seccomp KILL 化（第二梯队安全纵深）不属本轮，归挂账。
- Q1-Q6 业务决策仍归用户，不属执行窗口。
