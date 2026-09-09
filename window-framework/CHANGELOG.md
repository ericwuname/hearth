# 窗口群框架 v1.0-UX 发布说明 + 维护期启动

> UX 定版：v0.9 六面全闭，160/160 全绿
> 功能定版：v0.7 七版演进，设计 v2.1 全部落地
> 两大资产维护期同时启动

---

## v1.0.3 多窗口 dogfooding（2026-08-01）））

```
框架灵魂首次被真实 LLM 证明：多窗口分工 + 依赖链流转 + 不踩文件
- analyze 产出 3 窗口（info-architect / html-generator / qa-reviewer）✅
- 全部窗口 done（各 ~40 轮真实 LLM）✅
- check 4/4 PASS + conflict list 空 ✅
- 14 html 产出（7 页 × 窗口+共享），人类介入 2 次 ✅
- 修复 3 个真 bug：gate approve 未验证窗口状态（假 done 根源）/
  human gate 断言误报 / provider key 缺失报错静默混淆
- 177/177 全绿（v10 +5 回归保护）
```

---

## v1.0.2 维护优化（2026-08-01）

```
dogfooding v1.0.1（上午完成）：
  真实 LLM 自建文档站成功——analyze 首胜率 100% / 全链路 done / 人类介入 2 次
  修复 12 个单测测不出的集成 bug（reasoning_content 回传 / 虚假 done / 探索死循环 /
  working 残留死锁 / tokens 爆炸 / stage gate 缺失等），全量回归 160/160

v1.0.2（下午维护优化）：
  1. provider 路由落地：deepseek/zhipu/agnes/openai 四家 endpoint/key/model 映射，
     不再 hardcode（Agent provider=auto 从 window.toml budget.provider 路由）
  2. VERSION 0.1.0 → 1.0.2（此前 --help 显示错误版本）
  3. 新增 tests/test_v10.py（12 项）：dogfooding 修复回归保护 + provider 路由验证
  4. README 同步 172/172
```

---

## 两线汇总

```
codex-rust（单 agent 引擎）
  v12-v21 锻造九轮 → wiring 15/15 + 基准 90% + 应力 0 panic + 回放 100%
  v21 维护期 → 季度体检

窗口群框架（多 agent 协作）
  v0.1-v0.7 七版 → 项目容器 + agent 引擎 + 工作流 + 压缩 + 自主分析 + 冲突仲裁
  v0.8-v0.9 UX 六面 → 上手/反馈/边界感/呈现/容错/polish
  160/160 全绿
```

---

## 下一步：端到端真实项目验证

不再加机制。两个引擎都建好了。用真正的 LLM，跑一次完整项目，证明一切连起来真的能用。

### 建议场景：搭建 codex-rust 自己的文档站

```
1. codex project create codex-docs --type software
2. 需求窗口 —— 和 LLM 聊：从 codex-rust 的 docs/ 目录提取架构文档，建一个静态站点
3. analyze → deploy → 自动建窗口群
4. workflow 流转：arch 设计站点结构 → backend 生成页面 → review 审
5. 产出：一个可访问的 docs 站点
```

**这是 dogfooding——用自己造的引擎，处理自己的文档。**

### 三个观测指标

| 指标 | 目标 |
|---|---|
| analyze 首胜率 | > 80%（function calling + 六面 UX 的叠加效果） |
| 全链路不卡死 | arch → backend → review 全部 done |
| 人类介入次数 | ≤ 3（需求审核 1 次 + gate 审核 ≤ 2 次） |

---

## 维护合同

```yaml
codex-rust:
  季度体检: deepseek 20×2 + 应力 24 次 + 回放 31 条
  红线: < 85% / 应力 panic > 0 / wiring 断裂

window-framework:
  季度集成测试: mini-blog 全链路
  红线: analyze 首胜率 < 50% / sandbox 突破 / 压缩丢上下文

按需增量: 新功能加基准 + wiring / 修 bug 跑受影响题
```

---

*两条线，五个小时的锻造，从"一个 agent 能跑通 90% 基准"到"N 个独立窗口 AI 自动组队流转、六面 UX 全闭"。进入稳态。刀已入鞘。*
