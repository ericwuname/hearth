# codex-rust v10.5 — 全局全景图

> 全经络贯通。感知→决策→执行→编排→工具生态 闭环完成。
> 日期：2026-07-29
> 测试：163 passed / 0 failed
> Crate 数：21
> 🔴=0 | 🟡=0 | 🔵=0

---

## 一、经络全景

```
  ┌──────────┐    query()    ┌─────────────────┐    collect     ┌──────────────┐
  │  AgentLoop ├────────────►│ NervousSystem   │◄──────────────┤ resource-monitor │
  │ (大脑)     │◄────────────│ (神经系统)       │──────────────►│ (身体感知)        │
  │ plan/refle │   verdict   │                 │   snapshot()  │ cpu/mem/disk   │
  └──────────┘   override   └────────┬────────┘               └──────────────┘
        │                            │
        │ execute_plan               │ civ_store.append
        ▼                            ▼
  ┌──────────────┐          ┌──────────────┐
  │ Orchestrator │          │ civilization  │
  │ (编排器)      │          │ CIV_ALERT     │
  │ retry/repair │          └──────────────┘
  └──────┬───────┘
         │
         ▼
  ┌──────────────┐     search/install    ┌──────────────┐
  │ ToolDispatcher│◄─────────────────────┤ ToolRegistry  │
  │ (工具执行)    │                      │ (工具生态)     │
  └──────────────┘                      └──────────────┘

  闭环：感知→决策→执行→编排→再感知→验收
```

## 二、Crate 接线状态（21/21 全部接通）

| # | Crate | 功能 | v10.5 状态 |
|---|-------|------|:----------:|
| 1 | agent-core | 大脑 + 宪法 + Orchestrator + NervousSystem | ✅ 全线接通 |
| 2 | agent-types | 共享类型 (CivEntry/ToolCall/TaskGraph) | ✅ |
| 3 | api | API 定义 (SSE/messages/sessions) | ✅ |
| 4 | bridge | 跨 agent 通信 | ✅ |
| 5 | code-index | 代码索引 (CodeChunk → retriever) | ✅ |
| 6 | codex-cli | 终端客户端 (14 commands) | ✅ |
| 7 | llm-cn | 国内 LLM (豆包) | ✅ |
| 8 | llm-gateway | LLM 网关 + CostMeter | ✅ |
| 9 | llm-local | 本地 LLM fallback | ✅ |
| 10 | llm-openai | OpenAI provider | ✅ |
| 11 | lsp-bridge | LSP 服务桥 | ✅ |
| 12 | memory | 记忆系统 (civ/workline/PerUser) | ✅ |
| 13 | nervous-system | 神经系统 (brain↔body) | ✅ |
| 14 | planner | 任务规划 (decompose+reflect) | ✅ |
| 15 | resource-monitor | 资源监测 (sysinfo+cpu/mem) | ✅ |
| 16 | retriever | 语义检索 (scan .rs → build index) | ✅ |
| 17 | sandbox | 隔离沙箱 | ✅ |
| 18 | service | HTTP 服务 + SSE + routes | ✅ |
| 19 | telemetry | 原子计数器 (session_count) | ✅ |
| 20 | tool-runtime | ToolRegistry + ToolDispatcher + Manifest | ✅ |
| 21 | tools-builtin | 内置工具 (bash/edit/grep/glob/read) | ✅ |

## 三、v6.0 → v10.5 进化轨迹

| 版本 | 测试 | 里程碑 |
|------|:----:|--------|
| v6.0 | 146 | 基线 (FMT=CLIPPY=0) |
| v7.0 | 149 | 可观测性 (telemetry/replay/coverage) |
| v8.0 | 149 | 多用户 (UserStore/templates/webhooks) |
| v9.0 | 156 | 自主决策 (Orchestrator/Pipeline/Validator) |
| v10.0 | 156 | 自我认知 (resource/constitution/observer) |
| v10.1 | 159 | 行动力 (execute_plan + ROI + critical alerts) |
| v10.2 | 162 | 经络打通 (NervousSystem: brain ↔ body) |
| v10.4 | 163 | 接线闭环 (8/9 债务清仓, orchestator 真接入) |
| **v10.5** | **163** | **工具生态 (ToolRegistry: search/install/auto-discover)** |

## 四、9 项历史债务终局

| # | 债务 | 状态 | 调用链 |
|---|------|:---:|------|
| 1 | telemetry 孤儿 | ✅ | POST /sessions → fetch_add → GET /telemetry |
| 2 | 宪法未注入 | ✅ | build_messages → constitution_prompt → chat |
| 3 | retriever build() | ✅ | main startup → scan *.rs → build(CodeChunks) |
| 4 | webhook fire() | ✅ | session create → fire_event → curl HTTP POST |
| 5 | civ 自动触发 | ✅ | session create → civ_store.append |
| 6 | Orchestrator | ✅ | do_act → execute_plan(dispatcher, TaskStep[]) |
| 7 | WorkLine 调度 | ✅ | main spawn → 60s loop → list+update |
| 8 | 模型自动发现 | ✅ | main startup → providers.json write+read |
| 9 | 工具搜索/安装 | ✅ | ToolRegistry: search/install/auto-discover |
