# 维护期操作规程（v21 发布）

> 生效：2026-08-01（v21 刀入鞘轮）
> 适用：codex-rust 维护期（v20 之后的常态）
> 目标：**稳定第一，按需增量，季度全量体检**——不再有"每版全量基准"的漂移

---

## 1. 体检节奏

### 季度全量体检（每年 1/4/7/10 月第一周）

| 项目 | 内容 | 基准 |
|---|---|---|
| 基准 | deepseek 20×2 固定序（`bench/runner.py batch --runs 2`） | **90% ± 5%** |
| 应力场 | 24 次并发/异常注入 | **0 panic** |
| 回放 | 31 条 replay fixture | **100%** |
| 门禁 | fmt + clippy + test + wiring | **全绿（wiring 14 条）** |

### 按需增量（随时）

| 场景 | 动作 |
|---|---|
| 新功能 | 加 1+ 基准任务 + wiring 断言 |
| 修 bug | 只跑受影响题 + 回放 + 应力受影响的场景 |
| 模型升级 | 全量基准（新模型跑一次基线） |

### 明确不跑

经验实验、embedding 对比、LLM 精炼、正交变量矩阵——锻造期方法，不是维护期日常。

## 2. 红线（任一触发 → 问题分析）

| 红线 | 阈值 | 响应 |
|---|---|---|
| 🔴 通过率 | < 85% | 逐题分析，找失败模式；对比上季度数据 |
| 🔴 应力场 panic | > 0 | 复现 panic 场景，修复或降级该场景 |
| 🔴 wiring 断裂 | 任何一条 | 定位断言对应代码被改/删；恢复或重评审 |
| 🔴 数据造假 | 任何 | 立即停线，审查来源 |

## 3. 红线触发后的响应流程

```
1. 复现：重跑失败题/场景（≥3 次确认非偶发）
2. 分类：planner 缺陷？模型能力？工具链？基础设施？
3. 决策：修（写任务书）或记录（能力墙/已知盲区，更新债务台账）
4. 验证：修复后跑受影响题 + 回放 + 应力场 + wiring
5. 记录：季度体检报告 append 结论
```

## 4. 新功能 Checklist

- [ ] 新增基准任务（含 goal.txt / fixture / verify.sh / level 标注）
- [ ] wiring 断言覆盖新功能关键行为
- [ ] 新任务单题跑通（deepseek，≥1 PASS）
- [ ] 影响面确认：改动 crate 的测试全绿
- [ ] CHANGELOG 记录
- [ ] 更新 `docs/version-iteration-manual.md`：追加本版本一节（做了什么 / 没做哪些 / 关键决策），见该文档 §0 更新纪律

## 5. 年度回顾（跨季度趋势报告）

每年 12 月：汇总 4 个季度体检数据 → 趋势报告
- 通过率趋势（是否稳定在 90%±5%）
- 失败题清单变化（能力墙是否演进）
- 成本统计（deepseek API 消耗）
- 债务台账复核（是否新增/清除）
- 下一年维护策略建议

## 6. 环境与密钥

- VM：`ssh wutao@192.168.220.131`（密码见本地记录）
- **API key 一律从 `.env` 读取**（v21 起，仓库内零硬编码）
- VM 的 `.env` 位于 `/home/wutao/codex_work/.env`（不进 git）
- 本地新增 key 时同步写 VM `.env` + 本地 `.env`（.gitignore 已排除）

## 7. 文档索引

| 文档 | 内容 |
|---|---|
| `docs/design-decision-experience.md` | 经验=降级通道（v17/v18/v19 证据） |
| `docs/design-decision-planner-v20.md` | all_done 需要真实 Write（v19 解剖） |
| `docs/acceptance-final-epic-bc.md` | 终章验收（Q3 基线值：wiring 15/15 + 206 passed + 0 panic） |
| `docs/capability-boundaries.md` | 能力边界（T14/T09/T19） |
| `docs/top-level-telos-anchor.md` | 终点与决策过滤器 |
