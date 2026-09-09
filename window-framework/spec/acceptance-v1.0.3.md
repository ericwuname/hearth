# 窗口群框架 v1.0.3 验收报告——多窗口 dogfooding

> 验收时间：2026-08-01（用户睡觉，全自主）
> 计划依据：`spec/next-plan-v1.0.3.md`（v1.0.3.1 审查修正版）
> 验证方式：真实 LLM（deepseek-v4-flash）多窗口分工建文档站

---

## 一、三指标判定（plan v1.0.3.1 验收清单）

| # | 指标 | 目标 | 实测 | 判定 |
|---|---|---|---|---|
| 1 | analyze 产出多窗口 | ≥ 3 窗口 | **3 窗口**（info-architect / html-generator / qa-reviewer） | ✅ |
| 2 | 窗口全部 done（无 blocked/残留） | 全部 done | **3/3 done**（各 ~40 轮真实 LLM） | ✅ |
| 3 | 窗口不互相踩文件 | check 断言4 绿 + conflict list 空 | **check 4/4 PASS**（conflict-marked 绿）+ **(no conflicts)** | ✅ |
| 4 | 产出 ≥3 html 含 index | 可访问文档站 | **14 html**（7 页 × 窗口+共享）+ index.html | ✅ |
| 5 | 人类介入 ≤ 3 | ≤ 3 | **2 次**（需求审核 1 + gate 1） | ✅ |
| 6 | 全量回归无回归 | 172/172 | **177/177**（v10 +5） | ✅ |

**产出物**：`window-framework/dogfood-docs-multi/` —— 7 页完整静态站（38KB），
带侧边栏导航的专业文档站（index/architecture/genes/panorama/versions/progress/decisions）。

**三窗口真实分工链**（框架灵魂首次被证明）：
```
需求窗口 → analyze 产出 3 窗口
  → win-info-architect（设计信息架构 → 产出 ia-sitemap.json）
  → win-html-generator（按架构生成 7 页 HTML）
  → win-qa-reviewer（质量审查 → 产出 qa-review-summary.md）
  → gate 依依赖链推进 → ALL DONE
```

---

## 二、本轮发现并修复的 3 个真 bug

| # | 问题 | 根因 | 修复 |
|---|---|---|---|
| 1 | **gate approve 未验证窗口状态**——窗口 pending 也能 approve，progress.md 残留 done 标记 → 引擎跳过窗口 → 全链路假 done（30s 假完成） | `cmd_workflow_gate` 无条件写 done | approve 前验证该 stage 窗口全部 done，否则拒绝（rc=1） |
| 2 | **human gate 断言误报**——window.toml 存 `human:描述` 时断言 2 找脚本路径 FAIL | 断言 2 未豁免 human: 前缀 | gate 以 `human:` 开头不查脚本 |
| 3 | **provider key 缺失静默混淆**——LLM 产出 provider=openai 但 VM 无 OPENAI_API_KEY，报错笼统 | 报错硬编码 "DEEPSEEK_API_KEY not set" | 报错含实际 provider 名和所需 env key |

## 三、计划审查修正（v1.0.3.1 定版）

- **「并行」概念澄清**：现有引擎是顺序执行（stage 依赖驱动），本轮验证"多窗口分工 + 顺序流转"，
  真并行（线程级）超出"不再加机制"边界——审查时修正，避免执行走偏。

## 四、测试状态

| 套件 | 结果 |
|---|---|
| test_v10.py（v1.0.2 12 + v1.0.3 +5） | ✅ **17 passed** |
| v09/v07/v06/v05/v04/v03/v02/framework（回归） | ✅ 160 passed |
| **合计** | ✅ **177/177** |

## 五、遗留与后续

- **多窗口时间并行**（真并行）未做——引擎顺序执行，符合"不再加机制"；如需可作未来 EPIC
- codex-rust 两笔债（/readyz、神经系统告警链）——按分工铁律需任务书，本轮仅记录决断
  （维护期检测显示主项目健康，无前置阻塞，可执行但待授权）
- dogfooding 场景 LLM 产出 provider=openai 是常态——VM 侧强制 deepseek 是脚本层兜底，
  生产环境应配对应 provider key

---

## 结论

**✅ v1.0.3 验收通过——框架灵魂（多窗口分工）首次被真实 LLM 证明。**

单窗口能跑（v1.0.1）→ 多窗口分工 + 依赖链流转 + 不踩文件（v1.0.3）——设计 v2.1 的
"需求分析完 → 自动建多窗口 → 按依赖流转"完整闭环落地。框架从"能用"升级为"设计目标达成"，
进入真正的维护稳态。177/177 全绿，3 个真 bug 修复 + 5 项回归保护新增。
