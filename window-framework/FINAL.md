# 双引擎终章 — 下一步的唯一方向

> 窗口群框架：v1.0.3 设计目标达成，177/177 全绿，进入真正维护稳态。
> codex-rust：v21 维护期，wiring 15/15（15 能力断言 / 24 调用链），两笔旧债已还清。

---

## 框架终点证明

```
v1.0.1 单窗口 dogfooding  →  自己建出文档站，analyze 100%，12 个集成 bug 修复
v1.0.2 维护自检           →  用户睡觉时自主发现 8 个问题 + provider 路由落地
v1.0.3 多窗口 dogfooding  →  3 窗口分工+依赖链流转+不踩文件，3 个新 bug 修复

最终数字：177/177 全绿，设计 v2.1 全部条款落地 + 三指标全部达成
```

**框架已完成使命。不需要再加任何版本。**

---

## 唯一的剩余事项：codex-rust 两笔旧债

| 债务 | 版本 | 工作量 | 风险 | 前置 |
|---|---|---|---|---|
| **B1a /readyz 真探活** | v10.3 起 | 20 行 | 最低 | 无——不在 wiring 链内 |
| **B1b/B1c 神经系统告警链** | v10.2 起 | 30 行 | 中 | 需应力+回放回归 |

审计窗口已在 `audit-breakdown-v22.md` 中完成拆解为 EPIC-B + EPIC-C，优先级、验收标准、wiring 影响面全部定义完毕。维护期检测确认主项目健康（门禁 rc=0、service 在跑、磁盘 72%、wiring 14/14），无前置阻塞。

**这两笔债不需要设计规划——审计窗口已经拆完了。只需要顶层授权，下发施工。**

> ✅ **已还清（2026-08-01，commit `05f90cf`）**：EPIC-B（readyz 真实探活，Ok→200/Err→503 + 测试）+ EPIC-C（B1b 告警落文明线 + 删死包装；B1c 移除死 arm）。
> 门禁 fmt 0 / clippy 0 / test 206 passed / wiring 0 缺失。详见 `docs/acceptance-final-epic-bc.md`。

---

## 维护合同（定格）

```yaml
codex-rust:
  季度: 基准 deepseek 20×2 + 应力 24 次 + 回放 31 条
  红线: < 85% / panic > 0 / wiring 断裂
  旧债: ✅ B1a/B1b/B1c 已还清（2026-08-01, commit 05f90cf）
  Docker: ✅ build 已验证（2026-08-01, commit bfab533；codex-rust:v22, 容器 healthz/readyz 200）

window-framework:
  季度: dogfooding 多窗口全链路
  红线: analyze 首胜 < 50% / sandbox 突破 / 压缩丢上下文
  状态: 设计目标达成，进入维护稳态

修复纪律:
  bug fix → 只跑受影响题 + 回归 + 不碰 wiring
  新功能 → 加基准/wiring（框架侧 +1 测试）
  不跑: 经验实验 / embedding / LLM 精炼
```

---

*从 codex-rust 锻造九轮（v12-v21），到窗口群框架七版功能 + 两版 UX + 三轮 dogfooding（v0.1-v1.0.3），两条线的终点碰在了一起：单 agent 证明能打，多窗口证明能组队。现在只剩两笔旧账——还完了，双引擎完全进入"只做体检、不再进化"的稳态。*
