# Node 07-09 Audits — P4-REVALIDATION-01（Intent / Decision-Terminal / Memory）

日期：2026-08-31　执行窗口：砺·执行　基线：v0.2.20

## Node 07 — Intent Adversarial Audit（v0.2.20 真机抽查 + 语料映射）

**活体抽查（.131，v0.2.20）**：

| 对抗输入类 | 样本 | 结果 | 判定 |
|---|---|---|---|
| material dump（陈述喂料 38.3%） | 盲测风格日志 ERROR/WARN 喂料+回头讨论 | **completed**；零 TaskGraph 异常、零 revision churn | ✓ 分类正确（分析任务，无规划爆炸） |
| vague delegation（3.6%） | "帮我看下刚才那个"（悬空指代，无前文） | **failed**（诚实失败——无上下文可看，零误入恢复循环/零 approval 误触发） | ✓ 有界失败，非缺陷 |
| "继续"两形态 | CFR Node 11 实证 | 已完成形态=RC47 族 base rate 40%（Node 04） | RC52 修复批管辖 |

**语料映射**（规划书 v1.1 §2.1）：七类 persona 的行为分布（继续型 26.9%/纠偏 10.4%/状态 8.1%…）已定义；**对抗输入的黄金集 = 盲测 41 条输入逐字剧本**（已冻结 `docs/data/p0-attribution-20260831/`）——campaign 时逐条回放即覆盖。**结论：Intent 路由在 v0.2.20 无新缺陷**；高收益探针（5.6%）的放大采样按规划书执行。

## Node 08 — Decision / Terminal Mapping Audit

五 Decision × Terminal 映射 + 五组语义区分 + **Escalate+delegation+无人响应=结构化降级 Stop**——CFR `decision-terminal-map.md` 已正式建表（v0.2.19 审计）。
**v0.2.20 重验锚点**（漂移检查）：RC47 路由 / 五态接线 / RC24-B 结构化拒绝——P3/P4 测试全绿（gate 455→459 无回归）✓。**无新增 authority**。

## Node 09 — Memory / Context Reality Audit

- **A. 40 切片**：`[history note]` 标记在树（v0.2.17+）✓；被切信息用户不可知/不可恢复（lost-to-LLM）= CLOSURE ACCEPTED DEVIATION 维持 ✓。
- **B. archive 三件事**：exists=**PROVEN**（session 归档文件实证）/ searchable=工具通道在（B 档提示）/ **model can use=UNKNOWN**（C-probe 污染 ×2，v4 setup 失败——CLOSURE disposition 维持）。
- **C. compaction 时机**：v0.2.19 真机实证（RC49 重跑 +13KB @20:04；n13v2 +13KB @2026-08-31 20:04）——**修复后压缩真实发生** ✓；sawtooth 形态（merge 折叠单 turn → 需 ≥2 新交换再触发）= 机制注记。
- **D. calibration 交叉验证**：**COMPACT_DBG 真机数据修正**——Agnes provider caps 实际返回 **128,000 tokens**（非 512K 理论值）→ 注入阈值 = 195,840 字符 ≈ 76.8k tokens ≈ Agnes 真实窗口的 **15%**——保守方向安全（欠使用优于过使用）；512K 理论值与 provider 上报不一致 → **登记 provider caps 申报值待核**（供应商侧问题，非 Core）。

## 处置

三审计**零新 F0/F1**；RC51-A/B 修复（v0.2.20）已消除 P0 两项 NEW F1 的主体。Node 10（SimUser Stage 2）→ Campaign（Node 11-12）→ Node 13 修复批 → Freeze Candidate。
