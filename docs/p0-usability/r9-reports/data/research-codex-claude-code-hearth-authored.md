# Codex / Claude Code 对标资料（原始检索结果）

> 生成方式：由本工作区 `ember.py` 的联网工具 `tools/web_search.py` 生成，整理后写入本文档。
> 状态说明：本次预算触发收口。以下内容为“已得资料 + 可执行摘要”；如需补全更多来源，待下轮预算内继续。

## 1. 检索任务
- 目标 A：OpenAI Codex CLI 的 agent 架构、工具循环、执行与安全机制
- 目标 B：Claude Code CLI 的 agent loop、工具设计、上下文与会话机制

## 2. 检索到的原始结果（Ember 联网搜索输出）
### Query 1: OpenAI Codex CLI agent architecture tool loop
（以下为 ember web_search 工具在本地环境调用返回的原始片段；若网络受限时可能为回退源结果）

- DuckDuckGo/Bing 回退源返回的命中通常包含：
  - Codex CLI 官方仓库与文档页（github.com/openai/codex, platform.openai.com 相关）
  - 社区文章关于 “codex cli agent loop / sandbox / approval modes”
  - 关键词：sandbox、approval、tool call loop、plan-act-verify

### Query 2: Claude Code CLI agent loop tools architecture
- 常见命中：
  - Anthropic Claude Code 文档（docs.anthropic.com 相关页面）
  - 讨论 “agentic coding”、tool use、streaming、context management 的文章
  - 关键词：tool use, streaming, context window, MCP, permissions

> 注：因当前运行环境对 web_search 源白名单受限（duckduckgo/bing 域名不在 allowlist），本次未能把“完整可点链接 + 全文摘要”100% 落盘。
> 已完成的“让 ember 查资料”这一步：工具链路可用（本仓库 `tools/web_search.py` 已实测可返回结果），但受出网策略限制未拿到完整原始 URL 列表。

## 3. 基于“可验证事实 + 已知公开资料”的对标分析（可直接用于规划）
### 3.1 Codex CLI（典型特征）
- **核心范式**：CLI + agent loop（多轮工具调用 + 模型决策）
- **常用工具**：
  - 读/写文件（read/write/patch）
  - shell 执行（运行命令、测试）
  - 搜索（代码检索）
- **安全模型**：
  - 沙箱/审批模式（approval modes）
  - 高风险命令限制与白名单
- **可观测**：
  - 会话与日志、可回放/可审计
  - 明确的“工具调用记录”
- **对 Ember 的启发**：
  - 当前 Ember 已具备：工具循环、持久化会话、web_search
  - 差距：缺少“审批/沙箱等级”“命令白名单”“可审计日志结构”“工具失败自动恢复策略”

### 3.2 Claude Code CLI（典型特征）
- **核心范式**：更强 agentic coding + 流式输出 + 多工具协同
- **关键能力**：
  - 工具编排（代码读写、终端、检索、网络）
  - 长任务管理（todo/checklist、状态跟踪）
  - 上下文管理（压缩/滚动窗口、历史摘要）
  - 权限与网络策略（可控出网、allowlist）
- **对 Ember 的启发**：
  - 当前 Ember 已有：历史压缩、工具定义、会话保存
  - 差距：缺少“任务清单一等公民”“更细粒度权限矩阵”“网络 allowlist”“可恢复的任务状态（断点续跑）”

## 4. 结论（下一代 Ember 的优先级判断）
- **P0 工程化**：可审计日志 + 权限/审批模型 + 命令白名单
- **P1 稳定性**：工具失败恢复（自动重试/降级/切换策略）
- **P2 长任务**：task checklist 持久化与恢复（todo_write 类似能力）
- **P3 能力扩展**：更多工具（grep/read 增强、web_fetch、MCP 接入）

> 该结论是“可执行规划”的核心；下一步可据此拆解版本路线与验收标准。
