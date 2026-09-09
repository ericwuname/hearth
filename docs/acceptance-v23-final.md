# v23 封版说明

> 日期：2026-08-04 | 里程碑：`5ec25e9`（v23 全部 11 个 WP 闭环）
> 性质：v23 规划（top-level-plan-v23）三端口架构施工完成，进入维护期

---

## 一、v23 施工历程（2026-08-02 ~ 08-04，五阶段）

| 阶段 | 内容 | 门禁基线 |
|---|---|---|
| 阶段一（WP-0a + 红队 P0） | 审批原语修复（R1/R2/R3）+ 红队 P0 快修 + 文档修正 | 214 tests |
| 阶段二（WP-0） | 通用交互原语（InteractionRequest/Response，内核不认 kind） | 222 tests |
| 阶段三（WP-1/2/3） | 事件信封+span 树 / SSE 重放 / 规划缺口推导 | 225 tests |
| 阶段四（WP-4~7） | Observer 四件套（独立 crate，零执行权，确定性指标+规则+熔断） | 237 tests |
| 阶段五（WP-8/9/10） | 产物登记 / 思考摘要 / 前端接真流（GET /stream） | **239 tests** |

## 二、三端口架构（v23 目标达成）

```
Human OS（北向）          AI OS（南向，唯一事实源）       Observer OS（第三权）
  WP-10 前端接真流    ←→    WP-0~3 内核 + 事件链      ←→    WP-4~7 独立 crate
  demo 真实模式面板         通用交互/信封/SSE/缺口           只读事件流，零执行权
  （mock 保留 fallback）    WP-8 产物 / WP-9 摘要           确定性指标/规则/熔断
```

**关键成果**：
- **加一种人类交互 = 0 行内核改动**（WP-0：内核只认 id/blocking/timeout，kind 开放字符串）
- **事实产生权**（WP-1/5/8）：信封/响应/产物全部由 BE 产生入流；Observer 只投影不采集
- **G2/G3/G4 可观测性判据**：录制重放确定性 / 两次计算字节级相同 / 断线停在最后事实 seq
- **L2 fail-closed + G0 熔断只拉闸**：Observer 崩溃 → 协调者拒启；红线 → CircuitBreak 事件

## 三、门禁汇总（封版基线）

| 门禁 | 结果 |
|---|---|
| cargo fmt --check | ✅ 0 |
| cargo clippy -D warnings | ✅ 0 |
| cargo test --workspace | ✅ **239 passed 零失败** |
| cargo build --release | ✅ 成功 |
| window-framework | ✅ 208/208 |
| VM 端到端 | ✅ /stream / artifact（穿越全拒） / events 实测通过 |

## 四、维护期待办（v23 后）

| 项 | 状态 | 说明 |
|---|---|---|
| RT3 / P2-2 seccomp 白名单 | 🔴 挂起 | 用户 2026-08-02 明确"先挂着"；大工程+安全敏感 |
| Q1~Q11 未决项 | 🟡 挂账 | top-level-plan §8（含 Q9 API_KEY 默认全放行） |
| 季度体检 | 🟡 待预算 | 基准 deepseek 20×2 + 应力 + 重放（预算纪律） |
| 版本迭代说明书 | 🟡 待补 | version-iteration-manual 需补 v22→v23 段 |
| N7 密钥串用复核 | 🟡 挂账 | 证据不足 |
| CT1 wiring AST 强化 | 🟡 挂账 | P0-4 已做剥注释+哈希锁 |

## 五、tag 建议

`git tag v23.0 5ec25e9`（v22.0 之后的真实里程碑线）
