# v23 阶段四验收报告：Observer 四件套（WP-4/5/6/7 全闭环）

> 日期：2026-08-03 | 基线：`78e4d25`（WP-1/2/3 闭环）→ 验收后 `afb90bd`
> 依据：`v23-phase4-plan.md` + `top-level-plan-v23.md` §2.3/§7
> 性质：执行窗口一轮自主施工——**Observer 独立 crate 四件套全闭环**，237 tests 零回归

---

## 〇、审查补充优化（用户问"有没有补充优化的地方"——4 处修正）

1. **象限③数据来源错误**：plan 写 `error_rate ← SpanClose.status`——**SpanClose 没有 status 字段**
   （WP-1 定版只有 span_id/t1/duration_ms）。修正：error_rate 从 **span 内 Error 事件**推断
   （Observer 维护 span 栈，span 内出现 Error → error span）。
2. **象限④数据来源错误**：plan 写 `cost_per_step ← CostMeter`——**cost 不在事件流里**，
   Observer 只读事件流（不得直读内核对象）。修正：从 **Done.report.usage** 聚合
   （agent 完成时 report 带 usage.cost/tokens）。
3. **象限②补 Z-16 例外**：wait_ratio 必须用 **InteractionResponse.latency_ms**
   （客户端采集回传，Observer 只聚合不采集）——已明确。
4. **L2 fail-closed 落点修正**：plan 写"Observer 崩溃 → **内核**拒绝继续"，与
   "agent-core 不 import observer"（独立 crate 铁律）冲突。修正：**fail-closed 在
   service 层**——service 初始化 observer，L2 校验（seq 断档）由 run() 返回 Err，
   协调者拒启/拒续；内核（agent-core）始终不依赖 observer。
5. **补充：响应必须入流**——autonomy_rate/wait_ratio 需要响应数据（by/latency_ms），
   但事件流只有请求。新增 `AgentEvent::InteractionResolved`（service 在
   submit_interaction 成功后 emit）——响应是事实，必须入流（事实产生权）。

## 一、WP-4 Observer 地基 ✅（新建 crates/observer，独立 crate）

| step | 内容 | 证据 |
|---|---|---|
| ① crate 骨架 | `crates/observer/`（Cargo.toml + lib.rs + metrics/rules/circuit 模块） | cargo check 过 |
| ② L1 只读抽头 | 消费 `Vec<EnvelopedEvent>`，纯函数 | test_l1_reads_event_stream ✅ |
| ③ L2 fail-closed | seq 断档 → `run()` 返回 Err（协调者拒绝继续） | test_l2_fail_closed_on_seq_gap ✅ |
| ④ service 注册 | AppState.observer 注入（main.rs 初始化，service Cargo + observer 依赖） | 编译过 |
| ⑤ 门禁 | 独立 crate ✅ / agent-core 不 import observer ✅ / L2 测试 ✅ | 12 observer 测试全过 |

## 二、WP-5 确定性指标引擎 ✅（纯函数禁 LLM，G3）

| 象限 | 指标 | 实现 | 证据 |
|---|---|---|---|
| ① 自主度 | autonomy_rate = by=="Policy" 占比 | InteractionResolved.by 聚合 | test_metrics_autonomy_and_wait ✅ |
| ② 人机交互 | interrupt_count / wait_ratio / clarify_skip_rate | 通用原语计数；wait 用 latency_ms（Z-16）；kind 由 id 匹配请求（不解析 payload） | ✅ |
| ③ 技术质量 | error_rate / retry_rate | span 内 Error 推断（修正①）/ 工具错误率 | ✅ |
| ④ 资源效率 | cost_per_step / token_efficiency | Done.report.usage（修正②） | ✅ |
| **G3** | 同一事件流跑两次 → **字节级相同** | 纯标量累加 + HashMap 仅查不迭代 | test_deterministic_metrics_same_twice ✅ |
| **禁 LLM** | grep llm_gateway/LlmProvider/chat = 0 | 门禁测试（排除测试模块自身） | test_metrics_no_llm_dependency ✅ |

## 三、WP-6 规则引擎与 Finding ✅

| step | 内容 | 证据 |
|---|---|---|
| ① 规则配置 | `observer-rules.toml` 5 条阈值（cost/autonomy/error/retry/wait） | test_rules_config_parses ✅ |
| ② 规则引擎 | Metrics + 规则 → Finding（顺序遍历，确定性） | test_finding_evidence_reproducible ✅ |
| ③ L3 证据强制 | `Finding::new` 空 evidence → Err（构造器校验） | test_l3_finding_evidence_required ✅ |
| ④ 中立性 | evidence 格式化指标实测值（凭值可复现结论） | ✅（evidence 含 "cost_per_step = 9.9000"） |
| ⑤ 双重报告 | report.md（给人）+ report.json（给机器） | test_double_report_formats ✅ |

## 四、WP-7 G0 红线熔断 ✅

| step | 内容 | 证据 |
|---|---|---|
| ① 熔断规则 | sandbox 违规 / 预算耗尽（Done.report.status）/ 重试风暴（retry_rate>0.8） | TOML + 代码 |
| ② 只拉闸 | Observer 产 **CircuitBreak 事件**（表达事实）——不改计划/不换工具/不尝试修复；停机由执行方 | test_circuit_sandbox_violation ✅ |
| ③ 可审计 | reason + rule + ts（RFC3339）全入事件 | 断言 ts 非空 ✅ |
| 干净流 | 不误熔断 | test_circuit_clean_stream_no_break ✅ |

## 五、门禁汇总

| 门禁 | 结果 |
|---|---|
| cargo fmt --check | ✅ 0 |
| cargo clippy -D warnings | ✅ 0 |
| cargo test --workspace | ✅ **237 passed 零失败**（+12 observer 新测试） |
| window-framework | ✅ 208/208（未改动） |

## 六、WP 进度

```
WP-0~3 ✅  通用交互/事件信封/SSE/缺口推导
WP-4 ✅  Observer 地基（独立 crate，零执行权）
WP-5 ✅  确定性指标引擎（四象限，G3 字节级稳定）
WP-6 ✅  规则引擎与 Finding（L3 evidence 强制 + 双重报告）
WP-7 ✅  G0 红线熔断（只拉闸，可审计）
WP-8 ⬜  产物登记（下一批）
WP-9 ⬜  思考摘要（可并行）
WP-10 ⬜ 前端接真流
```

**Observer OS（第三权）已立**：独立 crate、零执行权、只读事件流、确定性指标、
规则→Finding、红线→熔断事件。WP-8/9 可并行，WP-10 待前端。

## 七、交付

- commit `afb90bd`（Observer 四件套：新 crate observer + 3 服务端文件修改）
- 审查补充：4 处数据来源/落点修正 + InteractionResolved 事件（响应入流）
- 备注：本轮 VM 曾关机（ping 不通）——已用 vmrun 远程启动（E:\VMware + "Ubuntu 64 位.vmx"）后继续，无门禁损失
