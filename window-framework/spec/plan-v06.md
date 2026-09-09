# 窗口群框架 v0.6 施工计划 v0.6.1 — 集成测试 + 冲突仲裁收尾

> 基线：v0.5 106/106 全绿，设计 v2.1 目标达成。
> 五版从骨架跑到全自动，最后一版不做机制建设——**用一场真实集成测试把五版积累的"边界代码"全撞一遍**，然后收掉唯一没落地的 §8.4。
>
> **v0.6.1 审查补充（2026-08-01）**：审查发现 5 个缺口——集成测试的"人在需求窗口聊"步骤无人执行、
> 冲突仲裁 diff 算法未定义、三个指标无可执行断言、真实 agent 沙箱需持续路径审计、
> 仲裁后 framework check 断言 4 未联动。已并入 §v0.6.1 补充。

---

## §1 框架演进总结

```
v0.1 ████ 骨架     project/window create, 4 断言, 软删除/恢复
v0.2 ████ 脑子     agent 真跑 LLM, 沙箱, 快照, 导出
v0.3 ████ 流转     工作流引擎, deploy 建窗, human/auto gate
v0.4 ████ 记忆     三层上下文压缩, 质量自检, 回滚保护
v0.5 ████ 自主     AI 需求分析 → 自动建窗群, 导入/模板
v0.6 ░░░░ 收尾     集成测试 + 冲突仲裁
```

---

## §2 核心任务：端到端集成测试（一场真实的"建博客"项目）

不是单测，不是 replay，是**真实 LLM 跑完一个完整项目**：

### 测试场景

```
项目：搭建一个博客后端 API（mini 版）
  - 1 个 auth 端点
  - 1 个 articles CRUD
  - 用 Rust + Axum
```

### 执行流程（全链路，不做假）

```
1. codex project create mini-blog --type software
2. codex window create --name "win-req" --role "需求分析" --prompt "..."
3. codex window start win-req
4. 人在需求窗口聊：博客 API 需求、约束、技术栈
5. 人觉得聊够了 → codex window analyze win-req
   → AI 产出 YAML → 人审 → 确认
6. 自动建窗：win-arch + win-backend + win-review
7. codex workflow start
   → arch 启动 → done → gate → backend 启动 → done → gate → review
8. 全程记录：每个窗口的对话轮数、压缩次数、bug 发现
```

### 收集三个指标

| 指标 | 定义 | 目标 |
|---|---|---|
| **全链路通过率** | 整个 workflow 从头到尾不卡死 | 必须 100% |
| **首次成功 deploy 率** | `window analyze` 产出可用的 YAML 比例 | > 80%（一次过） |
| **压缩有效性** | 压缩后窗口不丢失关键上下文 | 100%（自检过） |

### 预期找到的问题类型

- analyze 产出的 YAML 窗口数不合理（太多/太少）
- 压缩摘要把关键信息丢了（导致下游窗口走偏）
- workflow gate 脚本没有覆盖真正的验收条件
- 两个窗口产出文件路径冲突但没检测到

---

## §3 辅助任务：共享层冲突仲裁（§8.4，唯一未落地条款）

设计 v2.1 §8.4 定义了冲突三层仲裁。v0.6 实现最简版（第一层自动合并已在 v0.1 断言 4 就绪，第二层需要新加）：

```
冲突检测（已有，v0.1 断言 4）→ 两个窗口声明了同一 outputs 路径
        ↓
第二层 自动仲裁（v0.6 新增）：
  - 双方窗口各将当前产出版本存入 .snapshots/conflicts/
  - 框架计算 diff → 如果没有实质冲突（不同行/不同函数）
    → 自动合并：两份产出各写各的
  - 如果有实质冲突（同一行被两方修改）
    → 窗口标记 blocked + reason → 通知人类
  - 人的合并结果 → framework 写入共享层 → 清 conflict 标记
  - 被否窗口 → 标记 blocked，reason = "产出被覆盖需重做"
```

**验收**：两窗口声明同路径 → 并行完成 → 一窗口写入函数 A，另一窗口写入函数 B（不同区域）→ 自动合并成功。两窗口都改同一行 → blocked + 等人类。

---

## 阶段拆解

| 阶段 | 内容 | 预计 | 验收 |
|---|---|---|---|
| **S1** | 冲突仲裁第二层（自动合并 + blocked） | 0.5 天 | 同路径不同区域自动合并 |
| **S2** | 集成测试：mini-blog 全链路跑通 | 1 天 | 无卡死，全窗口 done |
| **S3** | 集成测试报告（三个指标 + 收集的 bug） | 0.5 天 | 报告含可复现步骤 |
| **S4** | 收尾修复（集成测试暴露的 bug） | 1 天 | 修复后测试回归 |
| **S5** | 验收报告 v0.6 + 框架 v1.0 定版 | 0.5 天 | 对照设计 v2.1 全部条款 |

---

## 验收红线

| 级别 | 判据 |
|---|---|
| 🔴 | 集成测试中 agent 写出窗外文件（沙箱失效） |
| 🔴 | workflow 在某 stage 卡死超过 5 分钟 |
| 🔴 | 压缩导致下游窗口读不到关键信息 |
| 🔴 | 框架 4 断言 + v0.1-v0.5 回归 |
| 🟡 | analyze 产出 YAML 被退回 > 3 次才成功 |

---

## §v0.6.1 审查补充（5 个缺口修正）

> 审查 `plan-v06.md` v0.6 与现有代码（framework check 断言 4 / Agent 沙箱 / 压缩）的一致性。
> 以下 5 项随 v0.6 一起施工。

### 补充 1：集成测试双模式（解决"人在需求窗口聊"无人执行）

S2 的场景说"人在需求窗口聊"——测试环境**没人**。两种模式：

```
集成测试模式 A（默认，CI 用）：需求对话 replay 化
  预置一份"需求对话 JSONL"（模拟人聊了 20 轮：博客 API 需求/约束/技术栈）
  → 注入需求窗口 → 其余全链路 real（analyze LLM → deploy → workflow 真跑 agent）
  → 唯一"人"的步骤是 analyze 确认（自动 --confirm）和 human gate（自动 --approve）
集成测试模式 B（可选，开发用）：全 replay
  AGENT_MODE=replay 全链路（0 成本，验证机制）——单测已覆盖，集成测试必须用 A

成本预算：模式 A 预计 ~¥1-3（analyze 1 次 + agent 每窗口 3-8 轮 × 3 窗口）
```

### 补充 2：冲突仲裁 diff 算法（行级，无三方依赖）

独立项目禁加三方 diff 库。最简行级算法：

```
merge_two(path, versionA, versionB) -> (merged, conflict)
  linesA = versionA.split('\n'); linesB = versionB.split('\n')
  # 逐行比较：相同行保留，不同行如果位置不重叠 → 追加合并
  # 简化判定：统计"被修改的行号集合"
  #   A 改的 {i} 与 B 改的 {j} 无交集 → 自动合并（A 的改动 + B 的改动）
  #  有交集 → conflict=True（同区域被双方修改）→ blocked 等人类
  实现：line_diff_sets(a_text, base_text) 计算修改行集合
```

验收用例（§3 已定义）：函数 A 写 10-20 行、函数 B 写 30-40 行 → 行集合无交集 → 合并；
同 30 行 → 交集 → blocked。

### 补充 3：集成测试三个指标的可执行断言

| 指标 | 断言（可执行） |
|---|---|
| 全链路通过率 | workflow 结束时所有窗口 state=done，无 blocked（除 human gate 已 approve） |
| 首次成功 deploy 率 | analyze 产出 → validate_deploy_yaml 通过（首轮，不重试） |
| 压缩有效性 | 下游窗口能引用上游产出文件（文件存在 + 内容含关键符号） |

### 补充 4：集成测试沙箱路径审计

红线"agent 写出窗外文件"不能只靠单测——集成测试持续监控：

```
workflow 结束后：遍历所有窗口 outputs + shared/outputs
  → 收集实际写入的文件路径
  → 断言每个路径在允许前缀内（窗口 outputs 或 shared/outputs）
  → 违规 → 集成测试 FAIL（红线 🔴）
```

### 补充 5：仲裁与 framework check 断言 4 联动

v0.1 断言 4（conflict-marked）：两窗口声明同 outputs 路径 → 红。
仲裁后必须能**重新绿**：

```
conflict resolve <path> --keep <win_id>：
  1. 写入被保留窗口的版本到共享层
  2. 更新冲突标记：清除 merge_conflict（双方 window.toml 的 outputs 去重或标注 resolved）
  3. 被否窗口 state=blocked + reason="产出被覆盖需重做"
  4. framework check 断言 4 重新绿（同路径声明消除）
```

---

## v0.6 之后

- 框架定版 v1.0：所有设计 v2.1 条款落地
- 进入维护期：每季度跑一次集成测试
- 两个可能方向（不展开，待定）：
  - A) 对接 codex-rust agent-core（用更成熟的 agent 引擎替换内置 LLM 调用）
  - B) 多项目管理（跨项目经验共享、组队决策卡落 experience）