# v23 阶段四施工计划 — Observer 四件套（WP-4/5/6/7）

> 基线：WP-1/2/3 完成（225 tests），事件流(信封+SSE+重放) + 规划链(缺口+澄清) 已通
> 关键前提：WP-2 的信封事件流已就绪——Observer 直接消费它
> 审查发现 key fix：`take_interaction_response` 让 WP-3 的 clarification 回读 payload（WP-0 原设计漏洞，已补）

---

## 一、当前 WP 进度

```
WP-0 ✅  通用交互原语
WP-1 ✅  事件信封与 span 树
WP-2 ✅  SSE 出口与录制重放
WP-3 ✅  规划缺口推导器
WP-4 ⬜  Observer 地基       ← 本次
WP-5 ⬜  确定性指标引擎       ← 本次
WP-6 ⬜  规则引擎与 Finding   ← 本次
WP-7 ⬜  G0 红线熔断          ← 本次
WP-8 ⬜  产物登记
WP-9 ⬜  思考摘要
WP-10 ⬜ 前端接真流
```

Observer 四件套必须**串行**——WP-4 建 crate，WP-5 建指标，WP-6 加规则，WP-7 加熔断。每层依赖下层。

---

## 二、WP-4：Observer 地基（新建 crate `observer`，2h）

**铁律（v23 §7）**：
- Observer **必须是独立 crate**——禁塞进 `nervous-system`（后者 `NerveAction` 有干预执行权）
- Observer **零执行权**——只产 Finding 和报告
- L2 fail-closed：Observer 崩溃 / 不可用 → 内核拒绝继续执行

| step | 文件 | 动作 | 验证 |
|---|---|---|---|
| ① crate 骨架 | `crates/observer/` | 新建 crate：`Cargo.toml` + `lib.rs`（零业务逻辑，纯骨架） | `cargo check -p observer` |
| ② L1 只读抽头 | `observer/src/lib.rs` | 接收 `Vec<EnvelopedEvent>` 事件流（从 WP-2 的 JSONL 或实时 SSE 读），只读 | 测试：喂 10 条事件 → observer 正确解析 |
| ③ L2 fail-closed | `observer/src/lib.rs` | `Observer::run()` 返回 `Result<Vec<Finding>>` —— Err → 内核拒绝继续 | 测试：observer panic → 内核 Abort |
| ④ 注册到 service | `service/src/main.rs` | `Observer::new()` 在 service 启动时初始化，注入 session 事件流 | 编译通过 |
| ⑤ 门禁 | 测试 | ① observer 是独立 crate ✅ ② `agent-core` 不 import observer ✅ ③ L2 fail-closed 测试 | `cargo test -p observer` 全绿 |

---

## 三、WP-5：确定性指标引擎（2h）

**铁律（v23 §7）**：纯确定性代码，禁 LLM。

四象限指标（top-level-plan §2.3）：

| 象限 | 指标 | 算法 | 数据来源 |
|---|---|---|---|
| ① 自主度 | `autonomy_rate` | `count(by=Policy) / count(all interaction)` | InteractionResponse.by |
| ② 人机交互 | `interrupt_count` / `wait_ratio` / `clarify_skip_rate` | 通用原语计数（不 match kind） | InteractionRequest/Response |
| ③ 技术质量 | `error_rate` / `retry_rate` | span 状态统计 | SpanClose.status |
| ④ 资源效率 | `cost_per_step` / `token_efficiency` | cost meter + span time | CostMeter + span t0/t1 |

| step | 动作 | 验证 |
|---|---|---|
| ① 指标结构 | `Metrics` struct：四象限所有字段，全 `f64`/`u64` | 编译 |
| ② 计算逻辑 | 喂事件流 → 遍历一次 → 计算全部指标 | 同一事件流跑两次 → 结果**字节级相同**（G3 确定性） |
| ③ 禁 LLM | `grep -r "llm\|chat\|generate" crates/observer` | 0 命中 |
| ④ 门禁 | `test_deterministic_metrics`：两次计算字节一致 + observer 内 grep LLM=0 | ✅ |

---

## 四、WP-6：规则引擎与 Finding（2h）

| step | 动作 | 验证 |
|---|---|---|
| ① 规则配置 | `observer-rules.toml`：阈值定义（如 `cost_per_step > 0.05 → warn`） | 可解析 |
| ② 规则引擎 | 读规则配置 + Metrics → 匹配 → 产出 `Finding{rule, severity, evidence, recommendation}` | 测试 |
| ③ L3 证据强制 | Finding 构造时 `evidence` 必填（编译期强���），空 evidence → 编译失败 | test_finding_no_evidence_fails |
| ④ 中立性闸门 | 随机抽 10 条 Finding → 100% 凭 evidence 可复现结论 | 不达标 → 退回 |
| ⑤ 双重报告 | `report.md`（给人）+ `report.json`（给机器） | 双格式 |

---

## 五、WP-7：G0 红线熔断（1h）

| step | 动作 | 验证 |
|---|---|---|
| ① 熔断规则 | 预算耗尽 / sandbox 违规 / 重试风暴 → 触发 | TOML 配置 |
| ② 只拉闸 | 熔断 = 停机——**不改计划、不换工具、不尝试修复** | 事件流含 `CircuitBreak` 事件 |
| ③ 可审计 | 熔断入事件流（原因 + 触发规则 + 时间戳） | 事件流可 grep |

---

## 六、执行顺序

```
WP-4 (2h) → WP-5 (2h) → WP-6 (2h) → WP-7 (1h)
 ↓ 建crate     ↓ 指标       ↓ 规则       ↓ 熔断
串行——每层依赖下层输出
```

**总计 ~7h**。全部在 **新 crate `observer`** 内施工——不改 agent-core/service 核心逻辑（WP-4 step ④ 只加一行初始化）。

---

## 七、门禁汇总

| WP | 门禁 |
|---|---|
| WP-4 | ① observer 独立 crate ② `agent-core` grep observer = 0 ③ L2 fail-closed 测试 |
| WP-5 | ① 同一事件流跑两次字节相同 ② observer 内 grep LLM=0 |
| WP-6 | ① L3 空 evidence 构造失败 ② 10 条 Finding 100% 凭 evidence 可复现 |
| WP-7 | 熔断只停机不改计划 |
| 全 | fmt 0 / clippy 0 / 225 tests 不得回归 |

---

## 八、阶段5 预览（本轮不展开）

WP-4~7 完成后 → WP-8（产物登记） + WP-9（思考摘要）并行 → WP-10（前端接真流）。预期下下批。