# codex-rust 项目终局声明（2026-08-04）

> 从 07-27 到 08-04，9 天，13 个主版本 + 11 个 WP + 窗口群框架 7 个版本。
> 这是最后一份规划性质的文档。之后不再有版本 roadmap——只有维护合同。

---

## 终局状态

```
codex-rust（Rust AI agent 引擎）
  tag:         v23.0（5ec25e9）
  tests:       240 passed
  wiring:      15/15
  ���口:        三端口（Human OS 前端 + AI OS 内核 + Observer OS 第三权）
  核心遗产:    加一种人类交互 = 0 行内核改动

窗口群框架（Python 多窗口编排器）
  版本:        v1.0.3
  tests:       208/208
  provider:    4 家（deepseek/zhipu/agnes/openai）
  dogfooding:  单窗口✓ + 多窗口✓

备份:          Desktop/codex-rust-v23.0-backup.tar.gz（1.6M）
```

---

## 维护合同（定格，不再修改）

```yaml
quarterly_checkup:
  ai_os:
    - 基准 deepseek 20×2（90%±5%，需要时批准预算）
    - 应力场 24 次（0 panic）
    - 回放 31 条（100%）
    - wiring 15/15 全绿
  observer_os:
    - G3 确定性指标（两次字节级相同）
    - 5 条规则全触发
    - sandbox 违规 → CircuitBreak
    - L2 fail-closed
  human_os:
    - /stream SSE 正常
    - 交互链路（审批/澄清）
    - G4 断线重连不丢事件

redlines:
  - 基准 < 85%
  - 任何端口 panic
  - wiring 断裂
  - Observer G3 不重复

不跑: 经验实验 / embedding / LLM 精炼 / 治理 RFC 提案
```

---

## 唯一未决（不需要规划，需要决策）

| 项 | 状态 |
|---|---|
| deepseek 20×2 基准 | 待预算批准——批准即跑，不批准就等到预算有 |
| RT3 seccomp 扩展 | 用户明确挂起 |
| Q1~Q11（top-level-plan §8） | 挂账——不影响正常运转 |

---

## 不再需要的东西

- 不再需要"下一步设计规划"——规划已经完成了
- 不再需要新版本——所有机制都在位
- 不再需要新测试计划——季度体检就是测试
- 不再需要新 audit——grep 源码 + test 回归 = 每次门禁

---

*9 天前，一个agent 通过率 0%。现在它有一个三端口架构、一个独立 Observer、一个多窗口编排框架、和一份定格了再也改不动的维护合同。刀已入鞘。下一次醒来不是"设计规划"——是体检到了。*
