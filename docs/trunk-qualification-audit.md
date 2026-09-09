# 主干资格全面盘点报告（trunk-qualification-audit）

> 生成时间：2026-07-28 23:35。方法：并行派出 5 个源码级探查代理（执行核心 / 封装 seam / LLM provider / 工具支撑 / workspace 健康），逐项读真实源码、给 `file:line` 证据，非按文件名下结论。结论全部来自探查代理报告。

## 0. 一句话结论

**当前代码够主干资格——但有条件。** 核心（5 相位主循环 + planner + sandbox[Linux] + 工具 + 记忆 + OpenAI 网关 + service 封装 seam）是真实、连贯、生产级成形的"有形状"引擎，不是骨架，可以封板。但封板前必须做三件事：

1. 抽一次 `SubAgentExecutor` trait（已确认 `loop.rs` 写死 `tokio::spawn`，@347）；
2. 把 4 类"默认关闭 / Noop"能力**显式降级为可选枝干**，不能当主干能力冻结；
3. 声明 3 个形状风险（A4 子代理产出丢弃、非 Linux sandbox 为 Noop、approval 键未路由）。

"我们是不是没有形状"的担忧，被这次全局盘点打消：形状存在，缺的是把"真主干"与"可选枝干"白纸黑字分开的纪律——这正是冻结原则要解决的。

## 1. 主干资格清单（TRUNK-READY，可冻结）

| 层 | crate | 证据（file:line） | 结论 |
|---|---|---|---|
| 执行核心 | agent-core | `loop.rs` `run()`@1015-1129；`do_plan`@479 / `do_act`@627 / `do_observe`@701 / `do_reflect`@872 全部真实逻辑；`LoopPhase`@20 | 5 相位状态机完整，TRUNK-READY |
| 规划 | planner | `decompose`@43-149、`reflect`@151-285（`replan_count>=3` 全路径封顶 @185/@203/@265） | 启发式真实、replan 硬顶统一，TRUNK-READY |
| 沙箱 | sandbox | `LinuxSandbox` landlock@265-346 / seccomp@350-458 / cgroups v2@498-549 / 超时+孤儿回收@607-678 | Linux 真实隔离，TRUNK-READY（非 Linux 为 Noop，设计如此） |
| 调度 | dispatcher / scheduler | `dispatch_parallel` join_all；`do_act` 语义审批门 @627-698 | 真实并行 + 审批，TRUNK-READY |
| 工具 | tools-builtin | Bash@bash.rs:74、Edit@edit.rs:79、Glob@glob.rs:85、Grep@grep.rs:131、Read@read.rs:60 全真实 | 5 工具无 stub，TRUNK-READY |
| 记忆 | memory | JsonlMemoryStore 临时文件+原子 rename@99-146、64MiB@136、写锁@95；已接 service@140 | 真实持久化已接线，TRUNK-READY |
| 代码索引 | code-index | tree-sitter `parse_rust_file`@89、`extract_symbols`@114、`chunk_file`@184 | 真实（仅 Rust），TRUNK-READY |
| 封装 seam | service + api | 6 端点全 REAL（routes.rs:61-124）；SSE 桥接 session.rs:284-317；main.rs:136 真实构造并驱动 AgentLoop | seam 真实完整，TRUNK-READY |
| LLM 主路 | llm-openai | 真实 HTTP + SSE 解析 @139-383；超时 300s/10s@212 | 默认主路径，TRUNK-READY |
| LLM 可选 | llm-local / llm-cn | Ollama/vLLM/Hunyuan 真实客户端；env 门控注册 | 真实，TRUNK-READY（可选） |

## 2. 必须降级为"可选枝干"的清单（SKELETON / Noop-DEFAULT，不可当主干能力冻结）

| crate | 现状 | 证据 | 处置 |
|---|---|---|---|
| lsp-bridge | 仅 `NoopLspBridge`，真实 LSP 推迟到 P5 | lib.rs:64 仅 Noop 实现；main.rs:166 / loop.rs:255 默认接线 | 声明 optional branch / off-by-default；P5 实装 |
| retriever | 真实 BM25+向量融合，但默认关闭 | `RETRIEVER_ENABLED` 门控；session.rs:68 默认 None | 可选枝干，env 启用 |
| fallback | `FallbackChain` 逻辑真实但默认关闭 | `FALLBACK_CHAIN` 门控 @main.rs:98-119 | 可选枝干 |
| telemetry | **无任何导出 sink**，纯骨架 | crate 仅 re-export eval；无 OTLP/file sink | 降级为 eval-only 枝干 / feature-gate，勿以"telemetry"之名冻结为空壳 |
| eval | 真实但仅测试夹具，未接生产 | `EvalRunner`@eval.rs:173 仅自测调用 | 测试夹具，非主干能力 |

> 关键判断：这些不是"藏在主干里的隐藏骨架"——它们都被 cfg / env 正确门控，是**显式可选能力**。问题在于冻结时若不明确降級，外界会误以为它们是主干能力。所以处置是"声明 + 降级"，不是"补完"。

## 3. 封板前必须声明的"形状风险"

1. **A4 子代理产出被丢弃**：`RunReport`@159-165 仅 `steps/ok/summary`，`do_observe`@839-862 / `do_reflect`@879-896 父代理只写生成串，子代理自由文本从不合并 → 委托结果静默丢失。归枝干（且正是 `SubAgentExecutor` seam 一并解决：`RunReport.output_text`）。
2. **非 Linux 平台 sandbox 为 Noop**：`LinuxSandbox` 仅 `#[cfg(target_os="linux")]`，Windows/其他走 `NoopSandbox`（sandbox/src/lib.rs:97 "Do NOT use in production"）。**若在你 Win11 上冻结，隔离并不生效**——trunk 的隔离能力只在 Linux 真验（VM 146 passed 即在 Ubuntu）。须声明平台限制。
3. **approval_id 未参与审批路由**：`ApprovalReq.approval_id` 反序列化但未传给 `resolve_approval`（dispatcher.rs:141 仅按 `session_id` 解析）→ 多并发审批会错位。封板前建议修（影响封装契约）。
4. **LSP/Retrieval 事件降级为 Phase 文本**：session.rs:567-573 `map_event` 降级；LSP 为 Noop。已知。
5. **ModelInfo 死 DTO**：api/lib.rs:83-87 定义但 `/models` 未用。已知小缺口。
6. **静默占位 key**：`OPENAI_API_KEY` 缺失 → "sk-placeholder" 启动不报错（main.rs:39-40）——"running" ≠ functional。须文档化。

## 4. 封板前必做动作（pre-freeze checklist）

- [ ] **抽 `SubAgentExecutor` trait**（唯一允许的 trunk 改动）：`loop.rs`@227/@347/@378 把 `tokio::spawn` 收进 `InProcessExecutor`，句柄泛化；同时 `RunReport` 加 `output_text` 启用 A4 合并。
- [ ] **写可选能力清单**：lsp-bridge / retriever / fallback / telemetry 显式列为 "optional branch, off-by-default"，不计入主干能力。
- [ ] **修封装契约小缺口**：approval_id 路由（dispatcher.rs:141）+ ModelInfo 对齐 `/models`。
- [ ] **声明平台限制**：sandbox 在非 Linux 为 Noop；trunk 隔离仅在 Linux 验证（VM Ubuntu 146 passed）。
- [ ] VM 复跑 `cargo test --all` 全绿 + 守门员读主干 + 打 `v1.3-trunk-frozen`。

## 5. 全局判定

- **"够不够主干资格" → 够。** 核心有形状、真实、连贯，可封板。
- 之前"一直改核心骨架会变没形状"的担忧，根因不是"没有骨架"，而是**增强项（skills/modes/resilience）全想改 `loop.rs`/`planner` 函数体**——那才是真风险。盘点证明：只要把增强赶进枝干 seam，主干本身就是干净的。
- **方向判定**：封板（冻逻辑、开一条 `SubAgentExecutor` seam）+ 枝干增量生长 = 正确。下一步落 `docs/trunk-freeze-branch-plan.md`（冻结清单 + 枝干 backlog + seam 钩子点 + 封板步骤 + 验收红线）。

## 附：workspace 健康度（探查代理 E）

- 17 crate 全部连通至 `service`，仅 `telemetry` 无入边（自测孤立）。
- CI 存在（`.github/workflows/ci.yml`）：fmt + clippy `-D warnings` + test，ubuntu-only，无 release/audit/MSRV 门。
- 全仓 `unimplemented!`/`todo!` 仅出现在 `#[cfg(test)]` mock 内（code-index:355、retriever:318/348），非生产路径；`Noop*` 集中在 lsp-bridge(11) / sandbox(13) / service(6) / agent-core(2)，均为设计内可选默认。
- 测试真实、无全局 `#[ignore]`；`service/tests/integration_test.rs` 24 测试 fn。
- 结论：**HAS-SHAPE-RISKS**（LSP Noop 默认、非 Linux sandbox Noop、telemetry 未接线），但编译/测试/冻结为库核心足够。
