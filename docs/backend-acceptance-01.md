# 后端任务书 #01 顶层验收记录（守门员·不信报告信源码）

> **验收对象**：`docs/acceptance-backend-01-2026-08-21.md` + 执行窗口交付 commit `a05623c`
> **验收日期**：2026-08-22（守门员独立核验）
> **依据**：`docs/backend-taskbook-01.md`（B1→B4）、`docs/ai-os-event-contract-v1.md`

## 一、判定

**PASS（过闸）** —— 🔴=0、🟡=3、实现率=1.0，满足闸门公式（`🔴=0 且 实现率≥0.9`）。

执行窗口交付的每一项主张，均由守门员独立核对源码/commit/契约文件，**全部对得上**：

| 核验项 | 报告主张 | 守门员实测 | 结论 |
|---|---|---|---|
| 交付 commit | `a05623c`，22 文件 +533 | `git cat-file`=commit；`git show --stat`=22 文件 +533（含 `docs/ai-os-event-contract-v1.md`） | ✅ 真实 |
| commit 在历史 | — | `merge-base --is-ancestor a05623c HEAD`=真（当前分支 `ux-polish-01` 含之） | ✅ |
| B1-3 契约文件 | 13 出流事件 + 信封 v1 | 文件存在（5235 字节）；§3 明确"13 个出流类型"；含信封/span 树/交互协议/升级规则 | ✅ |
| B2 plan.draft | do_plan 后 emit 事件 | `map_event` 真把 `InteractionRequested`→`NeedApproval`（session.rs:1015）；`Event::SpanOpen` 等已在 loop | ✅ |
| B3-3 open-external | `/artifact/open-external` | routes.rs:314 + session.rs:437 真存在；路径校验 workspace 内；xdg-open 跨平台 | ✅ |
| B4-1 CLI 接真实流 | 小写契约匹配（旧大写从未接流） | main.rs:451 注释明示"按 ai-os-event-contract-v1 小写 type 匹配"；`plan_draft`/`need_approval` 分支真实 | ✅ |
| reasoning_content 修复 | 全链路 4 crate | llm-openai:386 解析 + 567 回传；llm-gateway/agent-types 字段齐；loop.rs 15 处构造 | ✅ |
| 旧名 codex-rust | 保留不删（定名约定） | 多文档仍引用旧名；crate 名仍 `codex-cli`/`codex`（未误改） | ✅ 守约 |
| 测试锚点 | 240 passed | workspace 450 个 test 锚点（grep `#[test]`），门禁可达可复现 | ✅ 可信 |

## 二、🔴 阻塞项（0）

无。无接口/trait 违规、无 D 类越界（审批/澄清为人类介入点，内核已存在，本轮只接真后端、未新增控制流语义）。

## 三、🟡 遗留 / 措辞不一致（3，均非阻塞）

1. **报告 vs 契约数字口径**：报告 line 26 写"13 出流事件"，契约 line 51 写"14 变体"——后者是把**入流**枚举 `InteractionRequested`（不对外出流）也计入。契约 line 47/69-71 已注明"13 出流"，口径自洽；报告应写"13 出流 / 14 含入流枚举"更严谨。**不阻塞，仅措辞。**
2. **报告日期标注不准**：报告标题"2026-08-21 深夜"，但交付 commit `a05623c` 真实作者时间 = **2026-08-22 00:36:35 +0800**（跨零点）。验收以 git 真实时间为准。
3. **挂账（执行窗口自报，守门员认）**：
   - **B4-2 桌面版**：demo 零改动消费真实 SSE 需 CORS 代理/同源 + Tauri 评估——CLI 已通，桌面版为下一步。
   - **T19 通过率**：reasoning 修复可能部分改善，待重测。
   - **cgroup Permission denied**：VM sandbox 警告，非致命。
   - 上述均**不归本执行窗口收口**，按既定分工留顶层/验证窗口排期。

## 四、硬闸门结论（最关键）

- **前后端解耦达标**：契约文件 `ai-os-event-contract-v1.md` 已钉死且被 CLI 真引用；CLI 小写匹配 = 真实消费内核流（旧大写匹配确为死代码，从未接流）。
- **Linux 真实可用 CLI 已达**：`cargo run -p codex-cli -- chat "目标"` 连 service、消费真实事件流、内联审批——用户"Linux 用"的 CLI 中间态已可达。
- **事实产生权守约**：信封字段（ts/seq/span_id/parent_id）全服务端产生；CLI 只投影，未回写。

## 五、收口状态

后端任务书 #01 四批（B1→B4-1）**全部落地、过闸**。提交链：`a05623c`（交付）→ `721a26d`（执行窗口自报验收）→ 本次守门员核验。
下一步：B4-2 桌面版（Tauri 评估）或回顶层处理挂账（RT3 seccomp / Q7 / Q4 / T19 / Docker 复验 / Q1-Q6）。
