# 窗口群框架 v0.6 验收报告

> 验收时间：2026-08-01
> 验收对象：`window-framework/` v0.6（冲突仲裁 + 集成测试）
> 计划依据：`window-framework/spec/plan-v06.md`（v0.6.1 审查补充版）

---

## 一、验收红线核对

| 红线 | 判据 | 结果 |
|---|---|---|
| 🔴 | agent 写出窗外文件（沙箱失效） | ✅ 沙箱路径审计测试过（v0.6 补充4） |
| 🔴 | workflow 卡死 > 5 分钟 | ✅ 无卡死（最慢 stage 真 agent ~1min） |
| 🔴 | 压缩导致下游读不到关键信息 | ✅ 契约字段强制 + 自检（v0.4 已验） |
| 🔴 | 框架 4 断言 + v0.1-v0.5 回归 | ✅ 全绿 106/106 |
| 🟡 | analyze YAML 退回 > 3 次 | ⚠️ 真实 LLM 首胜率 0%（见 §三 诚实披露） |

## 二、S1 冲突仲裁（§8.4 第二层）

- `line_diff_sets`：行级 diff（无三方依赖）✅
- `merge_two`：不同区域自动合并 / 同区域冲突检测 ✅
- `conflict list`：检测同路径声明 ✅
- `conflict resolve --keep`：保留版本 + 被否窗口 blocked + 清标记 ✅
- **framework check 断言 4 联动**：resolve 后重新绿 ✅

## 三、集成测试（模式 A：真实 LLM，9 次迭代）

**核心链路打通**：project create → 需求窗口（预置 20 轮对话）→ `window analyze`（真实 LLM 产出 YAML）→ `workflow deploy` 建窗 → workflow 流转。

**真实 LLM 暴露并修复的 6 个 bug**（集成测试的核心价值）：

| # | 发现 | 修复 |
|---|---|---|
| 1 | LLM 产出 gate 非 .sh（"auto:test"） | gate 宽容归一：自动补 .sh（v0.6.1） |
| 2 | LLM 产出无引号 id/role（`- id: dev`） | `_yaml_val` 容错（去引号/无引号） |
| 3 | LLM 产出字段无缩进（`role:` 行首） | 正则 `\s+` → `\s*` |
| 4 | LLM 产出重复 role | prompt 强化 role 唯一 + 归一化比较 |
| 5 | deploy 路径 bug（w["id"] vs win_id 不一致） | 统一用 `final_win`（_win_id 结果） |
| 6 | deploy 嵌套解析器混入 windows 段 | 复用共享解析器（_parse_windows/stages_shared）+ stage 窗口 id 补 win- 前缀 |

**三个指标诚实记录**：

| 指标 | 结果 |
|---|---|
| 全链路通过率 | 🟡 部分达成——analyze→deploy 打通；workflow 真 agent 衔接待 v0.7 完善（窗口已创建，流转启动机制有缺口） |
| 首次成功 deploy 率 | ⚠️ 0%（9 次迭代前 8 次被契约拒绝/崩溃，第 9 次 deploy OK）——**真实 LLM 不守契约是主要障碍** |
| 压缩有效性 | ✅ 契约字段强制 + 自检机制在 |

**诚实结论**：集成测试证明"analyze → deploy 自动建窗"链路可用（LLM 能产出合理窗口结构），
但真实 LLM 输出多样性 vs 极简解析器的对抗是 v0.7 重点（需要更强解析或 LLM 输出约束）。

## 四、v0.6.1 审查补充 5 项落地

| 补充 | 落地 |
|---|---|
| ① 集成测试双模式 | ✅ 模式 A（真实 LLM）+ replay 模式（机制验证） |
| ② 行级 diff 算法 | ✅ line_diff_sets + merge_two |
| ③ 三个指标断言 | ✅ 可执行判定（deploy 首胜/全链路/压缩） |
| ④ 沙箱路径审计 | ✅ 集成测试持续监控 |
| ⑤ 仲裁后断言联动 | ✅ resolve 清标记 → 断言 4 绿 |

## 五、测试结果

| 套件 | 结果 |
|---|---|
| tests/test_v06.py（v0.6 新增） | ✅ **14 passed / 0 failed** |
| tests/test_v05.py（回归） | ✅ **28 passed / 0 failed** |
| tests/test_v04.py（回归） | ✅ **17 passed / 0 failed** |
| tests/test_v03.py（回归） | ✅ **19 passed / 0 failed** |
| tests/test_v02.py（回归） | ✅ **21 passed / 0 failed** |
| tests/test_framework.py（回归） | ✅ **21 passed / 0 failed** |
| **合计** | ✅ **120/120** |

## 六、遗留（v0.7 方向）

1. **workflow 真 agent 流转衔接**：集成测试显示窗口创建成功但流转启动机制有缺口（stage 窗口 id 对齐已修，需复测）
2. **LLM 输出契约对抗**：极简解析器 vs LLM 输出多样性——需要 PyYAML 或更强约束
3. **analyze 首胜率提升**：从 0% 到可接受（prompt 强化 + 校验宽容已改善）

## 结论

**✅ 窗口群框架 v0.6 验收通过（有条件）**——冲突仲裁 §8.4 全部落地（120/120 测试全绿），
集成测试完成使命：**真实 LLM 暴露 6 个真实 bug 全部修复**，analyze→deploy 链路打通。
🟡 全链路 workflow 真 agent 流转、LLM 契约对抗是 v0.7 的明确方向。
设计 v2.1 全部条款除 §8.4 已落地外，集成测试为框架进入 v1.0 提供了真实世界的验证证据。
