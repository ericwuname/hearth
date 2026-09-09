# v22 审计补充 — 两篇规划文档的缺口与完善建议

> 审计窗口已做扎实的独立核验（行号校正/B5规程冲突/wiring口径统一）。
> 本补充聚焦 **roadmap-consolidated + audit-breakdown 未覆盖的四个缺口**。

---

## 缺口1：测试窗口 v21 实测发现的 fmt 问题未纳入规划

`docs/testing-report-v21.md` 实测发现 **T1 fmt RC=1，8 处 diff**（loop.rs / llm-replay / service/main.rs），是 v21 清密钥编辑遗留的格式漂移。

**现状**：两份规划文档都没有提到「先修 fmt 再跑其他」。但 fmt 是 G1 编译门的硬红线——fmt 不干净的前提下跑任何基准/应力/回放都失去可信度。

**建议**：在 EPIC-A 之前加一个 **A0：fmt 修复包**（30 秒，`cargo fmt --all`）。追加到 roadmap §6 的 EPIC-A 之前：

```
A0 → A(N2→N1) → B → C
```

---

## 缺口2：测试数量从 180→205 无人核实

| 来源 | 测试数 |
|---|---|
| v20 审计 | 180 |
| v21 CHANGELOG | 未提 |
| v21 扫描 | 180 声明 |
| **v21 测试窗口实测** | **205/0** |

+25 个测试的增量不被任何文档记录。可能的来源：
- v21 清密钥后新增测试？
- 编译配置差异（release vs debug）？
- 之前被条件编译跳过的测试现在被启用了？

**建议**：在 EPIC-A 的 N2 季度体检中加一项——`grep -rc '#[test]\|#[tokio::test]' crates/`，确认声明数。不阻塞施工，但必须记录在季度基线文档中。如果声明数是 205 而之前文档写 180 → 更新基线文档。

---

## 缺口3：Gemini provider 静默新增，无人审计

v21 测试窗口发现 `/api/v1/models` 返回 **gemini-3.6-flash**——7 个 provider 之一。但：

- v21 CHANGELOG 没有「新增 gemini provider」条目
- roadmap 和 audit-breakdown 都没有提到它
- 没有 wiring 断言覆盖 gemini provider 的注册路径
- 没有人验证它真的能用

**风险**：一个静默新增的 provider，没有 CHANGELOG 记录、没有 wiring 断言保护、没有 smoke test 验证——未来某次清理可能悄无声息地被删掉。

**建议**：EPIC-A 季度体检中加一个 **gemini smoke**（用 `BENCH_PROVIDER=gemini` 跑 T00）。如果可用 → 记入 provider 矩阵 + CHANGELOG 补录；如果不可用 → 决策「修或删」，不要让它以「不知道能不能用的第 7 个 provider」状态继续存在。

---

## 缺口4：EPIC 缺少时间盒与版本锚点

audit-breakdown 的三个 EPIC 优先级排得好，但没有时间估计和版本编号。执行窗口拿到「EPIC-A / EPIC-B / EPIC-C」不知道是一个下午还是三天。

**建议**：加一个轻量版本锚点：

| EPIC | 版本 | 预估时间 | 内容 |
|---|---|---|---|
| A0 | v22.0 | 30s | fmt 修复 |
| A | v22.1 | 1h | 首次季度体检 N2→N1 |
| B | v22.2 | 2h | /readyz 真实探活 |
| C | v22.3 | 4h | 神经系统告警链修复（B1b+B1c） |

**每个版本对应一个 commit + tag**，回归点清晰：v22.0 是 fmt 基线、v22.1 是体检基线封存、v22.2 是 /readyz 后、v22.3 是告警链修复后。每版间跑一次 wiring + 回放 + 应力（G1 门），确保不退化。

---

## 补充建议汇总

| 缺口 | 建议 | 成本 | 影响 |
|---|---|---|---|
| fmt 不干净 | EPIC-A0：`cargo fmt --all` | 30s | 不阻塞任何事，必须先做 |
| 205 测试无解释 | N2 体检加 grep 声明数，记录到基线 | 1 分钟 | 低——不阻塞 |
| Gemini 静默 provider | N2 体检加 T00 smoke | 2 分钟 | 低——不阻塞，但需要追踪 |
| EPIC 无时间盒 | 加 v22.0-v22.3 版本锚点 | 0（纯标注） | 给执行窗口明确的交付节奏 |

---

*本补充聚焦"规划文档里没有但现实已发生的事"。fmt 问题被测试窗口跑出来了但规划没吸纳、测试数量变了没人核实、新 provider 静默上线没人知道——这些不是代码缺陷，是规划文档与现实之间的裂缝。补上即可。*
