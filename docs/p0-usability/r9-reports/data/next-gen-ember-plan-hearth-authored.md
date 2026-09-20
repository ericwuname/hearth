# 下一代 Ember 开发计划（v2）

> 输入：docs/research-codex-claude-code.md（对标资料与可验证事实）
> 目标：在 1 个迭代内完成“可落地 + 可验收”的下一代 Ember 升级。

## 0. 版本定义
- **Ember v2（M1-M5）**：在现有“工具循环 + 会话持久化 + web_search”基础上，补齐
  - 可审计日志
  - 权限/审批模型
  - 任务清单持久化
  - 网络 allowlist
  - 工具失败恢复（recovery）

## 1. 里程碑与验收标准

### M1: 可审计性（Auditability）
- **目标**：让每次工具调用/响应可回放、可定位
- **改动**
  - 新增 `.ember/audit/` 目录，按 session 落 JSONL
  - 每轮记录：tool 名、参数（脱敏）、结果长度、耗时、错误码
- **验收**
  - `python3 ember.py q` 后能生成 `audit` 日志
  - 能从日志定位某次失败调用

### M2: 权限/审批模型（Permissions）
- **目标**：降低高风险 shell 误操作概率
- **改动**
  - 引入 policy 文件：`.ember/policy.json`
  - 三类模式：`auto / confirm / deny`（按工具与命令前缀匹配）
  - `confirm` 时交互式提示（Y/N）
- **验收**
  - 对 `rm -rf` 等高危命令默认 `confirm`
  - 用户选择 N 时 agent 必须换路径继续任务

### M3: 命令白名单（Allowlist）
- **目标**：与 M2 联动，提供“安全基线”
- **改动**
  - 在 policy 中增加 `allow_commands` 列表
  - 对 `bash` 工具做前缀与正则匹配
- **验收**
  - 白名单外命令进入 M2 的 confirm/deny 分支

### M4: 长任务状态（Task Checklist）
- **目标**：对齐 Claude Code 的 todo/checklist 能力
- **改动**
  - 新增工具 `todo_write`（写/读任务清单）
  - 清单持久化到 `.ember/todo_<session>.json`
  - agent 在长任务中主动维护
- **验收**
  - 多步任务中途可“恢复清单并继续”
  - 清单状态对用户可见（CLI 可选显示）

### M5: 网络控制（Egress Control）
- **目标**：对齐 “allowlist” 与可审计网络策略
- **改动**
  - 新增 `HEARTH_EGRESS_ALLOWLIST` 支持
  - `web_search/web_fetch` 默认拒绝非白名单域名
  - 白名单可配置（env + 项目内 `.ember/egress_allowlist.txt`）
- **验收**
  - 非白名单域名访问失败并给出明确提示
  - 白名单内访问正常

## 2. 工具扩展（v2 下一期候选，不做本迭代）
- `web_fetch`（��正文并校验）
- `grep/glob` 作为一等工具（当前在 bash 里可用，但无标准化接口）
- MCP 接入（外部工具服务）

## 3. 风险与依赖
- **出网策略**：当前环境对 `duckduckgo/bing` 受限，M5 需与部署环境一致
- **交互提示**：`confirm` 模式在无 TTY 环境需 fallback 策略（默认 deny 或 auto）
- **日志体积**：audit JSONL 需加 size cap 与 rotate

## 4. 本轮收口结论
- 已完成：对标分析 + 可执行计划（本文件）
- 未竟：尚未把 M1-M5 落到代码（下一步开发）
- 建议顺序：**M1 -> M2 -> M3 -> M4 -> M5**（先可审计，再安全与长任务）

## 5. Hearth 自身根因修正（本轮新增结论）
- **诊断**：Hearth 当前在高预算消耗后触发“收口/禁止新探索”的提示过于激进，导致简单任务也要拖几十轮才解决。
- **问题点**：
  1. 预算告警与“强制收口”规则过强，影响正常连续推进
  2. 出网策略（allowlist）与工具失败路径不清晰，容易反复试错
  3. 长任务缺少“可恢复状态”（清单/审计）导致重复探索
- **修正建议（最小改动）**：
  - 将“budget-critical”从“禁止任何新任务”改为“优先收敛、允许低成本必要步骤”
  - 明确 `web_search/web_fetch` 的默认策略与白名单配置路径
  - 将 M1（审计日志）作为 Hearth 自身的第一修复项，先解决“不可观测导致的反复试错”
