# 后端任务书 #01 验收报告 — 内核契约落地 + Linux CLI 可用版

> 日期：2026-08-21 深夜 | commit：`a05623c`（22 文件，+533）
> 依据：`docs/backend-taskbook-01.md`（B1→B4）
> 性质：C 类（只增不改超集）+ D 类接真后端

---

## 〇、任务书核实结论（不信报告信源码）

任务书声称 5 缺口，**实测 3 个在 v23 已做**（信封/SSE/交互协议），真缺口 4 项已全部补齐：

| 任务书缺口 | 核实状态 | 本轮动作 |
|---|---|---|
| ① 事件信封 | ✅ v23 已做（EnvelopedEvent） | 契约钉死（B1-3） |
| ② SSE 出口 | ✅ v23 已做（/stream + /events） | 契约钉死（B1-3） |
| ③ 规划缺口推导器 | ⚠️ 半做（derive_gaps 有，事件无） | **补 plan.draft 事件 emit**（B2） |
| ④ 交互协议 | ✅ v23 已做（/interaction/{iid}） | CLI 内联审批接真（B3） |
| ⑤ open_artifact | ⚠️ 半做（内容预览有，系统打开无） | **补 /artifact/open-external**（B3-3） |
| 产物形态 | ❌ 未做 | **CLI 接真实流**（B4-1） |

## 一、交付内容

| 项 | 内容 |
|---|---|
| **B1-3 契约文件** | `docs/ai-os-event-contract-v1.md`——13 出流事件 + 信封 v1 + span 树 + 交互协议 + 升级规则（前后端唯一契约） |
| **B2 plan.draft 事件** | do_plan 完成后 emit `{steps, gaps_found, gaps_to_ask, auto_assumed, mode}`；缺口三元组构造器强制（from+why）已在内核——事件化补齐 |
| **B3-3 open-external** | `GET /artifact/open-external?path=`——file→xdg-open / url→浏览器（路径校验 workspace 内，跨平台） |
| **B4-1 CLI 接真实流** | codex-cli `render_events` 重写：**小写契约匹配**（旧大写匹配从未接上真实流！）+ span 树缩进 + artifact/think_summary/plan_draft 渲染 + **need_approval 内联 y/n**（POST /interaction）；create_session 契约对齐（provider/budget/session_id） |

## 二、🔴 关键发现：deepseek thinking mode reasoning_content 未回传

**现象**：CLI 端到端跑几轮后 agent 断——service 日志 400 `reasoning_content in thinking mode must be passed back`。

**根因**：llm-openai 丢弃响应里的 `reasoning_content`（deepseek v4-flash thinking mode 要求原样回传）——agent 多轮即断。**这是 T00/T15 抖动、T19 能力墙的部分主因**（此前 8/7 季度体检 36/40 的 4 个失败 run 可能部分因此）。

**修复**（全链路 4 crate）：`llm-gateway ChatResponse` + `agent-types Message` 加字段；llm-openai 响应解析存 + 发送回传；agent-core 存历史。58 处构造补齐。

**验证**：CLI 端到端重跑——**400 计数 0**，agent 持续多轮（规划→审批→工具→写码→产物→反思）。

## 三、CLI 端到端实证（B1/B3/B4 硬闸门）

```
session 8fb92608-...
▸ span [plan]              ← span 树
🗺 规划草案（gaps=1 待问=0 自动假设=1）  ← plan.draft（B2）
⚡ 假设[missing_goal_source]...         ← auto_assumed 可追溯
⛔ clarification — 批准? [y/N]          ← 内联审批（B3，喂 y 续跑）
⚙ glob / bash / write_file             ← 工具调用
📄 产物 file: Cargo.toml（+4 行）        ← 产物登记
💭 [reflect] 正在反思进度               ← 思考摘要
```

**真实 Linux 终端可用**：`cargo run -p codex-cli -- chat "目标"`（连 service，消费真实事件流，审批内联）。

## 四、门禁

```
fmt --check 0 / clippy -D warnings 0 / test --workspace 240 passed / release build 0
```

## 五、挂账（非阻塞）

- **B4-2 桌面版**：demo 零改动消费真实 SSE 需 CORS 代理/同源服务 + Tauri 构建评估——CLI 已通，桌面版作为下一步（用户"Linux 用"的 CLI 已可达）
- **T19 通过率**：reasoning 修复可能部分改善（待重测）
- **cgroup Permission denied**（VM sandbox 警告，非致命）

## 六、交付物

- commit `a05623c`（契约 + plan.draft + open-external + CLI 接流 + reasoning 修复）
- `docs/ai-os-event-contract-v1.md`（冻结契约）
- VM：`codex --url http://127.0.0.1:3000 chat "goal"` 可跑真实任务
