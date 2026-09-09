# 下一步：多窗口 dogfooding（修正版 v1.0.3.1）+ codex-rust 债务决断记录

> v1.0.2 状态：172/172 全绿，单窗口全链路 dogfooding 通过（analyze 100%/全done/人类2次），provider 路由落地，文档对齐。
> 维护期自主发现 MEMORY.md 超限截断——框架甚至能在用户睡觉时自检自修。

---

## 下一步一：多窗口 dogfooding——框架的真正价值还没被证明

v1.0.1 dogfooding 是**单窗口**完成全部活（LLM 设计用单窗口做了文档站，没拆成多窗口）。框架的灵魂是"一个需求窗口分析完 → 自动建多个窗口 → 多窗口按依赖流转"——这一段还没被真实 LLM 验证过。

> ⚠️ v1.0.3.1 审查修正（2026-08-01）：「并行」概念澄清——现有 WorkflowEngine 是**顺序执行**
> （stage 依赖 trigger 驱动，窗口逐一跑，无线程/异步）。本轮验证的是**多窗口分工 + 顺序流转**
> （analyze 产出 ≥3 窗口 → 各窗口按依赖链全部 done → gate 推进），**不是时间并行**。
> 真并行（线程级）超出"不再加机制"边界，不纳入本轮。

### 建议场景：两窗口并行建文档站

```
需求窗口 → analyze 产出 arch + writer 两个窗口
         → arch 设计站点结构
         → writer 生成页面内容
         → 两个窗口并行（arch 不用等 writer，writer 也不等 arch）
         → review 窗口 审两端产出
```

**收集三个新指标**：
| 指标 | 现在的基线 | 多窗口后的目标 |
|---|---|---|
| analyze 产出多窗口合理 | 单窗口即可完成 | LLM 正确判断需要 ≥3 个窗口 |
| 窗口不互相踩文件 | 无多窗口场景 | framework check 断言 4（conflict-marked）绿 + conflict list 空 |
| gate 在依赖链上正确推进 | 单窗口串行 | done + done → gate → next stage，全部窗口 done |

---

## 下一步二：codex-rust 两笔旧债——拖了十个版本，该决断了

maintenance 报告中提到的三项 Rust 债务（/readyz 假实现、drain_nervous_alerts、Simplify 空挡）已经在 roadmap-consolidated-v22 中标记为 B1a/B1b/B1c，被审计拆解为 EPIC-B 和 EPIC-C。

**现在 v1.0.2 维护期检测显示主项目健康（门禁 rc=0, service 在跑, 磁盘 72%, wiring 14/14），这两笔债不需要等任何前置条件——可以直接按审计拆解执行。**

| 债务 | 工作量 | 风险 |
|---|---|---|
| B1a /readyz 真探活 | ~20 行 routes.rs | 最低——不在 wiring 链 |
| B1b/B1c 神经系统告警链 | ~30 行 loop.rs + nervous | 中——需应力+回放回归 |

---

## v1.0.3.1 审查补充（2026-08-01 定版）

**补充1：「并行」= 多窗口分工 + 顺序流转**（引擎无线程/异步，真并行超"不再加机制"边界）。
本轮验证：analyze 产出 ≥3 窗口 → deploy → workflow 按依赖链全部 done → gate 推进。

**补充2：冲突判定方法**——`framework check <project>` 断言 4（conflict-marked）为绿 +
`conflict list` 输出为空 = 窗口未踩文件；若触发冲突 → 走 `conflict resolve` 验证阻断机制。

**补充3：codex-rust 债务决断**——/readyz、神经系统告警链是主项目 Rust 代码，按分工铁律
需任务书（顶层不下发则不擅动）。本轮框架侧施工只负责**记录决断**：两笔债已在
roadmap-consolidated-v22 标记 B1a/B1b/B1c，维护期检测显示主项目健康（门禁 rc=0），
无前置阻塞——**结论：可执行，但需顶层任务书授权后由施工窗口实施**。

**补充4：测试预算控制**——真实 LLM 每窗口 max_steps 轮（默认 40）成本高；本轮 analyze 限
1-2 次���首胜率观测），窗口 max_steps 在 analyze prompt 约束到 ≤12（budget 预算内），
workflow 总 deadline 45min。预期 token 消耗 < 单窗口版 2 倍。

**补充5：验收清单**（可执行断言）：
1. analyze 产出窗口数 ≥ 3
2. 全���窗口 state=done（无 blocked/working 残留）
3. framework check 断言 4 绿 + conflict list 空
4. 产出 ≥3 个 html（含 index.html）
5. 人类介入 ≤ 3 次
6. 全量回归 172/172 无回归

---

## 建议执行顺序

```
v1.0.3: 多窗口并行 dogfooding（框架侧）
  → arch+writer 并行建文档站
  → 收集并行指标

codex-rust EPIC-B: /readyz 真探活（主项目侧，30分钟）
  → 改完 wiring 14→14 不变（不在链内）

codex-rust EPIC-C: 神经系统告警链（主项目侧，2小时）
  → 应力+回放回归

季度维护检查：两边引擎体检
```

两个项目都在维护期，不需要并行开发——优先级排好，按序推进。多窗口 dogfooding 优先做，因为它可能暴露框架最后一个未验证假设；Rust 债务往后排，因为它们已在 roadmap 上躺了十个版本，不差这一轮。

---

*锻造教会我们：先测出来哪里有洞，再修洞。单窗口已经证明能跑。多窗口并行如果也跑通了——框架从"能用"升级为"设计目标达成"，进入真正的维护稳态。*
