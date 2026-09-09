# v24 维护轮 — tag + 文档 + 季度反哺

> 基线：v23 封版（239 tests，三端口架构全部 11 个 WP 闭环）
> 性质：**不进代码**——v23 是"最后一次大规模施工"。v24 只做维护动作。
> 目标三件事：tag v23.0、补版本迭代说明书、跑首次三端口季度体检。

---

## 一、v23 里程碑回顾

```
2026-08-02 ~ 08-04，五阶段，11 个 WP：

WP-0      通用交互原语（内核不认 kind，加交互 0 行内核改动）
WP-1      事件信封与 span 树（深度 3 嵌套还原，G2 重放）
WP-2      SSE 出口与录制重放（Last-Event-ID 断线续传，G4）
WP-3      规划缺口推导器（clarification 走通用原语）
WP-4~7    Observer 四件套（独立 crate，零执行权，G3 确定性指标+规则+熔断）
WP-8      产物登记（Artifact 事件 + open_artifact 安全路由）
WP-9      思考摘要（确定性模板零 LLM + 风格参数化）
WP-10     前端接真流（GET /stream + 真实模式面板 + mock fallback）

最终：239 tests / fmt 0 / clippy 0 / release build ✓ / VM 端到端 ✓
```

---

## 二、v24 三件事

### 1. Tag v23.0（1 分钟）

```bash
git tag -a v23.0 5ec25e9 -m "v23.0: 三端口架构（Human OS + AI OS + Observer OS），11 WP，239 tests"
git push origin v23.0
```

### 2. 版本迭代说明书 — 补 v22→v23 段（1h）

`docs/version-iteration-manual.md` 停在 v22，需要补：

| 新增段 | 内容 |
|---|---|
| v23 总览 | 一句话：三端口架构落地。五阶段，11 WP，239 tests |
| v23 §WP-0~3 | 通用交互/信封/SSE/缺口——内核基础层重构 |
| v23 §WP-4~7 | Observer 四件套——第三权独立 crate |
| v23 §WP-8~10 | 产物/摘要/前端——收尾三件套 |
| 全局收口表刷新 | 三端口架构状态 / 新红线（G2/G3/G4 判据） |
| 基线事实刷新 | HEAD / tag / tests / 季度体检基线 |

### 3. 首次三端口季度体检（4h 机器时间）

维护合同需要升级——不再是"基准+应力+回放"三维，而是三端口各自的可观测性判据。

| 端口 | 体检项 | 判据 |
|---|---|---|
| **AI OS** | 基准 deepseek 20×2 | 通过率 90%±5%（不变） |
| | 应力场 24 次 | 0 panic（不变） |
| | 回放 31 条 | 100%（不变） |
| | 接线 wiring 15/15 | 全绿（不变） |
| **Observer OS** | 指标确定性 | 同一事件流两次计算字节级相同（G3） |
| | 规则触发 | 5 条规则全触发（构造阈值越界事件流验证） |
| | 熔断触发 | sandbox 违规事件 → CircuitBreak 产出 |
| | L2 fail-closed | seq 断档 → Err 拒绝 |
| **Human OS** | 前端接真流 | demo 真实模式面板正常渲染 / 断线重连不丢事件 |
| | 交互链路 | 审批按钮 → POST /interaction → SSE 收到 Resolved |

**执行方式**：
- AI OS 三项（基准/应力/回放）在 VM test 区跑（用 dev-test-coop 协议）
- Observer 体检用 mock 事件流（不烧 token）
- Human OS 体检在 Windows 本机跑 demo

---

## 三、维护合同更新

```yaml
# 原合同（v21 定版）需要加 Observer OS 维度：
observer-os:
  季度: 指标确定性 + 规则全触发 + 熔断触发 + L2 fail-closed
  红线: G3 不重复 / 规则空触发 / 熔断漏报
  门禁: observer 独立 crate（不变）；agent-core grep observer = 0（不变）
```

---

## 四、执行顺序

```
1. tag v23.0（1 分钟）
2. 版本迭代说明书补 v22→v23 段（1h）——可与 3 并行
3. 首次三端口季度体检（4h 机器时间 + 1h 报告撰写）
```

---

## 五、v24 之后

- codex-rust 进入真正的三端口维护期
- 季度体检从三维变六维（AI OS 4 项 + Observer OS 2 项 + Human OS 2 项）
- 唯一未决：RT3 seccomp（用户挂起）、Q1~Q11（top-level-plan §8）
- **不再有"加机制"版本**——维护期只做体检和修复