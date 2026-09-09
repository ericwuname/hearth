# v23 阶段五验收报告：收尾三件套（WP-8/9/10）+ **v23 封版**

> 日期：2026-08-04 | 基线：`15857e4`（WP-0~7 闭环）→ 验收后 `5ec25e9`
> 依据：`v23-phase5-plan.md` + `top-level-plan-v23.md` §5.1/§10
> 性质：执行窗口一轮自主施工——**WP-8/9/10 全闭环，v23 全部 11 个 WP 收官**，239 tests 零回归

---

## 〇、审查补充优化（用户问"有没有补充优化的地方"——5 处修正）

1. **WP-8 emit 钩子位置**：plan 写"tool-runtime (write_file/edit 成功路径)"——但
   **tool-runtime 不依赖 api crate**（架构边界），且 Tool trait 返回 String 无法填 artifacts。
   修正：emit 钩子在 **loop.rs do_act**——write_file/edit 成功时从 ToolCall.args 统计
   （path/delta_lines=content 行数/size_bytes=content 字节数）→ AgentEvent::Artifact。
2. **WP-8 open_artifact 安全**：补路径校验（拒绝绝对路径/`..`/Prefix + join 后必须在
   workspace 内）——端到端实测原始/URL 编码/绝对路径全 400。
3. **WP-9 摘要成本**：plan 说"调 LLM 生成"——每相位一次 LLM = 烧 token（dogfooding 教训）。
   修正：**确定性模板**（"正在规划任务分解"等，零 LLM）+ `THINK_SUMMARY_STYLE` env
   参数化（concise/verbose）+ `THINK_SUMMARY_LLM` 增强开关（可选渲染，不阻塞门禁）。
4. **WP-10 关键遗漏（GET SSE 路由）**：plan 让前端 EventSource 连 `/events`——但那是
   **JSONL 录制导出（一次性非流）**，且 EventSource 是 GET-only 而当前 SSE 在 POST 响应里。
   **新增 `GET /api/v1/sessions/:id/stream`**（复用 sse_stream_with_replay + Last-Event-ID）——
   这是 plan 未预见、前端接真流的硬前提。
5. **WP-10 前端约束**：前端**不解析 payload 语义**（只渲染原文）——事实投影公理。

## 一、WP-8 产物登记 ✅

| step | 内容 | 证据 |
|---|---|---|
| ① 类型 | `AgentEvent::Artifact{path,kind,delta_lines,size_bytes}` | 编译过 |
| ② emit | loop do_act：write_file/edit 成功 → Artifact 事件（从 ToolCall.args 统计） | 门禁测试 ✅ |
| ③ open_artifact | `GET /sessions/:id/artifact/open?path=`——workspace 内校验 | **实测**：穿越 400 / 正常 200 内容正确 ✅ |
| ④ 前端约束 | 前端零 FS 调用（demo 只用 open_artifact 路由预览） | demo 代码审查 ✅ |

## 二、WP-9 思考摘要 ✅

| step | 内容 | 证据 |
|---|---|---|
| ① 生成 | loop 每相位 close 后 emit_think_summary——**确定性模板**（零 LLM） | 编译 + 测试 |
| ② 风格参数 | `THINK_SUMMARY_STYLE`（concise 默认/verbose）；`THINK_SUMMARY_LLM` 增强开关 | env 参数化 |
| ③ 事件流 | ThinkSummary 带 phase/text；**span_id 由信封自动带**（可关联具体 span） | test_wp9_think_summary_carries_span ✅ |

## 三、WP-10 前端接真流 ✅

| step | 内容 | 证据 |
|---|---|---|
| ① SSE 连接 | **新增 `GET /stream`**（EventSource 直达，复用 replay + Last-Event-ID） | 实测 /stream 200（鉴权 + 路由通）✅ |
| ② span 渲染 | demo 真实模式面板：SpanOpen/Close → 缩进时间线 | demo 代码 |
| ③ 交互渲染 | NeedApproval → 批准/拒绝按钮 → `POST /interaction/{iid}` | demo 代码 |
| ④ artifact 渲染 | Artifact 事件 → 卡片 → `GET /artifact/open` 预览 | demo 代码 |
| ⑤ 断线重连 | EventSource 内置重连 + lastEventId 自动续传（G4） | demo 代码 |
| ⑥ 后退兼容 | mock 按钮保留为 fallback（真实模式面板独立自包含） | demo 代码 |

## 四、门禁汇总

| 门禁 | 结果 |
|---|---|
| cargo fmt --check | ✅ 0 |
| cargo clippy -D warnings | ✅ 0 |
| cargo test --workspace | ✅ **239 passed 零失败**（+2 WP-8/9 新测试） |
| cargo build --release | ✅ 成功（2m29s） |
| VM 端到端（真 service） | ✅ /stream 通 / artifact 穿越全拒 400 / 正常 200 内容正确 |
| window-framework | ✅ 208/208（未改动） |

## 五、🎉 v23 封版——全部 11 个 WP 闭环

```
三端口架构（Human OS 前端 + AI OS 内核 + Observer OS 第三权）：
  WP-0  ✅ 通用交互原语（内核不认 kind，加交互 0 行内核改动）
  WP-1  ✅ 事件信封与 span 树（深度 3 嵌套还原）
  WP-2  ✅ SSE 出口与录制重放（Last-Event-ID 断线续传，G4）
  WP-3  ✅ 规划缺口推导器（clarification 走通用原语）
  WP-4  ✅ Observer 地基（独立 crate，零执行权，L2 fail-closed）
  WP-5  ✅ 确定性指标引擎（四象限，G3 字节级稳定，禁 LLM）
  WP-6  ✅ 规则引擎与 Finding（L3 evidence 强制 + 双重报告）
  WP-7  ✅ G0 红线熔断（只拉闸，可审计）
  WP-8  ✅ 产物登记（Artifact 事件 + open_artifact 安全路由）
  WP-9  ✅ 思考摘要（确定性模板零 LLM + 风格参数化）
  WP-10 ✅ 前端接真流（GET /stream + 真实模式面板 + mock fallback）
```

## 六、v23 封版后待办（维护期）

| 项 | 状态 |
|---|---|
| RT3 / P2-2 seccomp 白名单 | 🔴 用户挂起（大工程+安全风险独立排期） |
| Q1~Q11 未决项 | 🟡 top-level-plan §8 |
| 季度体检（基准 + 应力 + 重放） | 🟡 预算纪律待预算 |
| 版本迭代说明书（version-iteration-manual） | 🟡 需补 v22→v23 段 |

## 七、交付

- commit `5ec25e9`（WP-8/9/10 + demo 真实模式）
- 审查补充：5 处修正（emit 位置 / open_artifact 安全 / 摘要成本 / **GET SSE 路由** / 前端不解析 payload）
- v23 阶段五验收报告 + 封版
