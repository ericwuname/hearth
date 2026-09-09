# 窗口群框架 v0.3 验收报告

> 验收时间：2026-08-01
> 验收对象：`window-framework/` v0.3（工作流引擎 + workflow deploy + 自动快照）
> 计划依据：`window-framework/spec/plan-v03.md`（v0.3.1 审查补充版）

---

## 一、验收红线核对

| 红线 | 判据 | 结果 |
|---|---|---|
| 🔴 | `workflow start` 后窗口未自动启动 | ✅ 引擎自动启动 pending 窗口（replay 模式验证） |
| 🔴 | gate 脚本 exit 0 但未推进 | ✅ auto gate 通过 → 标记 done → 触发下一 stage |
| 🔴 | deploy 窗口数 ≠ YAML 声明 | ✅ 2/2 精确创建 |
| 🔴 | 框架 4 断言 + 回归 | ✅ deploy 后 check 全绿 + 回归 42/42 |
| 🟡 | 串行窗口提前启动 | ✅ design done 前 backend 不启动（测试断言） |

## 二、S1 工作流引擎

- `codex workflow start`：读 project.toml [workflow] → trigger 检查 → 启动窗口 → 轮询 → gate → 推进 ✅
- 串行依赖：`design:done` 触发 implementation ✅
- 自动 gate（`auto:` 脚本）exit 0 → 推进；非 0 → blocked ✅
- 人类 gate（`human:`）→ 停在 stage 等 `workflow gate --approve` ✅
- 并行 stage（`parallel=true`）两窗口同时启动 ✅
- **补丁修复**：Windows Git Bash gate 脚本路径（MSYS 转义）→ 相对路径 + cwd

## 三、S2 workflow deploy

- 解析需求窗口对话中的 YAML → 自动建窗 + 写流转规则 + 生成 gate 脚本 ✅
- **契约强制校验**（v0.3.1 补充1）：缺 budget/outputs/gate.sh → 拒绝 ✅
- **双 gate 语义**（补充2）：`human:` / `auto:` 前缀 ✅
- **补丁修复**：① parse_windows 遇 `workflow_stages:` 停止（防 stages 混入）② provider 值去引号（`""deepseek""` bug）

## 四、S3 自动快照

- `window stop` 自动快照 ✅（`.snapshots/win-xxx--ts/` 生成，源码 `cmd_window_stop` → `_auto_snapshot` 接线）
- 50 轮自动快照 ⚠️ **未实现**（审查复验发现：`_auto_snapshot` 仅在 stop 路径调用，agent 循环内未接线）→ 移入 v0.4 遗留

## 五、v0.3.1 审查补充 4 项落地

| 补充 | 落地 |
|---|---|
| ① deploy 契约强制校验 | ✅ 缺字段拒绝 + deploy 后 check 全绿闭环 |
| ② 双 gate 语义 | ✅ human:/auto: 前缀 |
| ③ AGENT_MODE=replay | ✅ 测试 0 成本（不调 LLM） |
| ④ S4 测试 15 项清单 | ✅ test_v03.py 19 项全覆盖 |

## 六、测试结果

| 套件 | 结果 |
|---|---|
| tests/test_v03.py（v0.3 新增） | ✅ **19 passed / 0 failed** |
| tests/test_v02.py（回归） | ✅ **21 passed / 0 failed** |
| tests/test_framework.py（回归） | ✅ **21 passed / 0 failed** |
| **合计** | ✅ **61/61** |

## 七、与设计 v2.1 对齐

- §5 工作流引擎 ✅（trigger/gate/并行）
- §5b 双 gate 语义 ✅
- §8.3 gate 脚本化 ✅（auto: 必须 .sh）
- §8.1 budget 必填（deploy 强制）✅

## 八、遗留（v0.4 范围）

- 上下文压缩（§3）
- 共享层冲突仲裁（§8.4）
- 需求窗口 LLM 自主产出规范 YAML（依赖压缩先就绪）
- 导入外部对话 / 模板系统
- **50 轮自动快照**（v0.3 声称后复验发现未接线，移入本清单）

## 九、复核指南（给外部审计窗口）

| 复核项 | 方法 |
|---|---|
| 61/61 测试 | `cd window-framework && python3 tests/test_v03.py && python3 tests/test_v02.py && python3 tests/test_framework.py` |
| stop 自动快照接线 | `grep -n "_auto_snapshot" src/framework.py` → 应见 `cmd_window_stop` 调用（272 行） |
| 50 轮自动快照未实现 | `grep -n "_auto_snapshot" src/framework.py` → agent 循环内无调用（证实遗留） |
| workflow 引擎推进 | `grep -n "workflow auto-start" src/framework.py` → 736 行 Agent.run 调用 |
| deploy 契约校验 | `grep -n "missing budget" src/framework.py` → deploy 拒绝路径 |
| 双 gate 语义 | `grep -n "human:" src/framework.py` → `_run_gate` 前缀分支 |
| AGENT_MODE replay | `grep -n "AGENT_MODE" src/framework.py` → Agent.run 短路分支 |

## 结论

**✅ 窗口群框架 v0.3 验收通过**——从"手动管窗"到"自动流转"闭环完成：
需求窗口 → YAML 契约 → 一条命令建全窗群 → 工作流自动按 gate 推进。
61/61 测试全绿，无回归。

> **审查修正（2026-08-01 复验）**：原报告声称"50 轮自动快照触发点已留 ✅"——
> 源码复验发现该功能**未接线**（`_auto_snapshot` 仅 stop 路径调用）。已更正为"未实现"
> 并移入 v0.4 遗留，附复核指南供外部窗口验证。验收结论不受影响（红线项全部真实通过）。
