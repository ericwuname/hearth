# v22 执行桥梁 v1.1 — 从审计/设计到施工的七个补丁

> 两份文档（`gatekeeper-audit-v22-fixes.md` + `project-windows-framework-design.md v2.1`）审查完毕。
> 下述七项为执行窗口开工前必须补齐的操作指引。非架构设计——是"怎么跑"。
>
> **v1.1 变更（2026-08-01 审查定版）**：审查发现 v1.0 与设计 v2.1 有 4 处不一致——命令面缺
> budget/snapshot/conflict 类命令、契约 YAML 缺 budget/outputs/gate 字段、gate 仍为自然语言
> （违反 §8.3 脚本化）、§8.8 窗口 wiring 断言无落地方式。新增补丁 5/6/7 修正。

---

## 补丁1：框架设计中缺少的命令面

`project-windows-framework-design.md` 描述了大量机制（`/project my-app window create` 等），但没有统一的命令参考。执行窗口需要确切知道 **v0.1 骨架阶段要实现哪些命令**：

```bash
# === 项目管理 ===
codex project create <name> [--type software|writing|data|research|ops]
  # 创建项目文件夹 + project.toml + 目录骨架
codex project open <name>
  # 切换到项目（或自动发现当前目录的 project.toml）
codex project status
  # 显示项目进度板、各窗口状态

# === 窗口管理 ===
codex window list [--active|--all|--adhoc]
  # 列出本项目下所有窗口 + 状态
codex window create --name "win-xxx" [--role "role-name"] [--prompt "..."]
  # 手工新增窗口（自动生成 window.toml）
codex window start <id>
  # 启动指定窗口（pending→working）
codex window stop <id>
  # 暂停窗口（working→blocked）
codex window delete <id>         # 软删除 → .trash/
codex window delete <id> --hard  # 彻底清除（需二次确认）
codex window restore <id>        # 从 .trash 恢复
codex window archive <id>        # 归档 → .archive/
codex window unarchive <id>      # 取消归档

# === 对话操作 ===
codex window export <id> [--format markdown|json] [--output path]
  # 导出窗口对话
codex window import --name "win-xxx" [--role "..."] --source some-file.json
  # 导入外部对话为窗口初始历史

# === 模板 ===
codex template save <name> --from-window <id>
codex template list
codex window create --template <name>

# === 流转控制 ===
codex workflow start     # 按流转规则启动窗口群
codex workflow resume    # 从断点继续
codex workflow gate <stage-id> --approve|--reject
  # 人类审核 gate（通过=继续下一个 stage；拒绝=标记阻塞原因）
```

**v0.1 实现优先级**：`project create/open/status` + `window list/create/start/stop/delete/restore` — 这 8 条是骨架必备。

---

## 补丁2：需求窗口分析 → 窗口清单的输入/输出契约

需求窗口是整个流程的发动机。它的输入边界和输出格式必须固化——不能让 AI 自由发挥：

### 输入（需求窗口收到的初始 prompt）

```
你是一个项目需求分析师。你的任务是和人类聊清楚项目之后，产出一份结构化的"项目窗口配置"。

在聊的过程中，你需要搞清楚：
1. 项目目标（一句话描述）
2. 项目类型（软件/写小说/数据分析/科研/运维/其他）
3. 关键约束（时间、预算、技术栈、特殊要求）
4. 产出物是什么

当人类说"可以了"之后，你必须输出以下 YAML 配置（只输出 YAML，不要废话）：

---
project_type: software
goal: "搭建一个博客后端 API"
windows:
  - id: "win-arch-01"
    role: "架构设计"
    prompt: "你是博客项目的架构师。根据 PRD 设计技术方案..."
    depends_on: []
    parallel: false
    budget:                      # v1.1: §8.1 必需字段（缺则框架拒绝启动）
      max_steps: 20
      max_cost_cny: 0.2
      provider: "deepseek"
    outputs: ["shared/outputs/arch.md"]     # v1.1: 产出路径声明（§5b）
    gate: "shared/gates/arch-gate.sh"       # v1.1: gate 必须是脚本（§8.3）
  - id: "win-backend-01"
    role: "后端开发"
    prompt: "你是博客后端开发者。根据架构方案实现..."
    depends_on: ["win-arch-01"]
    budget:
      max_steps: 40
      max_cost_cny: 0.5
      provider: "deepseek"
    outputs: ["src/"]
    gate: "shared/gates/backend-gate.sh"    # 脚本内约定：VERIFY_PASS 退出码 0
  - id: "win-frontend-01"
    role: "前端开发"
    prompt: "你是博客前端开发者。根据 API 文档实现..."
    depends_on: ["win-arch-01"]
    parallel_with: "win-backend-01"    # 和 win-backend-01 并行
    budget:
      max_steps: 40
      max_cost_cny: 0.5
      provider: "deepseek"
    outputs: ["web/"]
    gate: "shared/gates/frontend-gate.sh"
workflow_stages:
  - id: "design"
    trigger: "project_start"
    windows: ["win-arch-01"]
    gate: "人类审核方案"              # 人类 gate 保留自然语言（人审点）
  - id: "implementation"
    trigger: "design:done"
    windows: ["win-backend-01", "win-frontend-01"]
    gate: "所有 tests 全绿"           # 自动 gate 必须对应各窗口 gate 脚本
  - id: "review"
    trigger: "implementation:done"
    windows: ["win-review-01", "win-docs-01"]
    parallel: true
    gate: "人类批准发布"
```

### 输出（框架从 YAML 中提取并执行）

框架解析 YAML → 创建 `windows/win-arch-01/` 等目录 → 写入 `window.toml` → 写入 `.workflow.yaml` → 人类审核确认 → `codex workflow start`。

**唯一需要人审的地方**：窗口清单 + 流转规则。人一句话确认后全部自动。

---

## 补丁3：v22 修复审计中的施工要点（给执行窗口的提示）

`gatekeeper-audit-v22-fixes.md` 记录了 4 项修复。执行窗口如果要在窗口群框架中使用这些修复后的能力，需要注意：

| 修复 | 对窗口群框架的意义 |
|---|---|
| ST8 gate 预算 6→12 | 窗口群框架的 gate 脚本化（§8.3）中，**审批门 gate 必须给足预算**——比 v20 前多 3-6 步。这是框架的默认预算建议 |
| T19 双门控（no-toolcalls+write_attempted） | v20 的 `write_attempted` 单门控不够——窗口群框架内每个窗口的 Done 判定必须**双门控**：all_done 一路 + no_toolcalls 一路，两路都要求 write |
| wiring 15/15 | 框架 §8.8 的窗口级 wiring 断言（4 条）应与 codex-rust 的 15 条 wiring **平行管理**——不混在一起，但遵循同一哲学 |
| T19 诚实结论（模型能力墙） | 框架的窗口角色分类时，**不需要为"泛型重构"专门建一个窗口**——所有已知模型都做不好。不要分配给任何 AI 窗口 |

---

## 补丁4：框架设计与 codex-rust 的版本对齐

`project-windows-framework-design.md v2.1 §6` 对接表中的 wiring 数是 **15 条**——这是 v22 修复后的最新值。执行窗口施工时：

```
当前基线（施工前必读）：
  - wiring: 15 条（codex-rust 主仓库 docs/xray/wiring-v13.toml）
  - test: 全绿
  - 经验系统: 自适应开关（连续失败≥3 才注入） + 遗忘已部署
  - 能力边界: T19(模型墙/T13波动/T15波动) — 不给对应角色窗口派这些活
  - 上下文压缩: 尚未实现——这是窗口群框架 v0.2 的任务
```

**施工时不碰 codex-rust 主仓库的任何源码**——框架是独立项目，引用 codex-rust 的经验系统/wiring 哲学/能力边界结论作为参考，但不是代码依赖。

---

## 补丁5：命令面补全——v2.1 机制的命令承载（v1.1 新增）

v1.0 命令面缺 4 类机制命令。补全：

```bash
# === 预算与状态（§8.1）===
codex window budget <id>              # 查看窗口预算消耗（步骤/token/成本）
codex window budget <id> --set max_cost_cny=1.0   # 调整预算上限

# === 快照与回滚（§5f）===
codex window snapshot <id>            # 手动快照
codex window snapshot list <id>       # 列出窗口快照（最多 10 个）
codex window rollback <id> --to <timestamp>   # 回滚到指定快照

# === 冲突仲裁（§8.4）===
codex conflict list                   # 列出项目所有冲突标记（merge_conflict=true）
codex conflict resolve <file> --keep <window-id>  # 人类仲裁：选版本
codex conflict view <file>            # 查看冲突双方版本

# === 窗口 wiring 断言（§8.8，见补丁7）===
codex framework check                 # 跑窗口群自身 4 条断言
```

**v0.1 实现优先级调整**：原 8 条（project create/open/status + window list/create/start/stop/delete/restore）+ `framework check`（1 条自检）——9 条是 v0.1 必备。快照/回滚/冲突/budget 属 v0.2-v0.3，但命令面在 v0.1 就写进 `--help` 占位（报 "not yet implemented"）。

---

## 补丁6：契约字段与设计强制对齐（v1.1 新增）

补丁2 的 YAML 契约 v1.0 版缺 `budget/outputs/gate` 三个关键字段（与 §5b/§8.1/§8.3 不一致）。**已在上文契约示例修正**。执行窗口校验规则：

```
契约 → window.toml 映射（框架解析时强制校验）：
  id            → window.id            （必填，唯一）
  role/prompt   → window.name/role     （必填）
  depends_on    → dependencies.upstream （可选）
  budget.*      → budget.*             （必填！缺失 → 拒绝创建，防无界消耗）
  outputs.*     → outputs.files        （必填！缺 → 拒绝，防产出无处落）
  gate          → outputs.gate         （必填！必须指向存在的 .sh 脚本，§8.3）
  provider      → budget.provider      （可选，默认 deepseek）
```

**双 gate 语义**（v1.1 明确）：`workflow_stages[].gate` 分两类——
- 人类 gate（`"人类审核方案"` 等）→ 保留自然语言，人工审批点
- 自动 gate（`"所有 tests 全绿"` 等）→ 必须能解析为各窗口 `outputs.gate` 脚本的组合，框架只认脚本退出码

---

## 补丁7：窗口群 wiring 断言的落地方式（v1.1 新增）

v2.1 §8.8 定义了 4 条断言但没说"怎么跑"。落地方式（对齐 codex-rust project-xray 哲学）：

```
存放：框架项目内 tests/framework_checks.rs（或等价 pytest）
触发：codex framework check（补丁5 命令）+ CI 每次提交

断言实现（每条都是可失败测试）：
  window.toml-exists:
    扫描 windows/*/，每个子目录必须有 window.toml；
    缺 → 测试红 + 报"无法识别窗口 X"
  gate-script-exists:
    读每个 window.toml 的 outputs.gate → 文件存在且可执行；
    缺 → 测试红（gate 永失败，窗口必 blocked）
  conflict-marked:
    检查 shared/outputs/ 下所有文件——若两个窗口的 outputs.files
    声明了同一路径且都有产出 → 断言冲突标记已写（merge_conflict=true）；
    未标记 → 测试红（静默覆盖 = 数据丢失风险）
  budget-declared:
    每个 window.toml 必须有 [budget] 段（max_steps/max_cost_cny/provider）；
    缺 → 测试红（防无界消耗）

与 codex-rust 主仓库 wiring 的关系：
  平行管理（独立测试文件），不混入 docs/xray/wiring-v13.toml（15 条不动）
```

---

## 审查结论

| 文档 | 状态 | 施工前补的活 |
|---|---|---|
| `gatekeeper-audit-v22-fixes.md` | ✅ 可直接过 | 补丁3 的施工要点 |
| `project-windows-framework-design.md v2.1` | ✅ 可直接过 | 补丁1 的 8 条命令参考 + 补丁2 的需求窗口契约 |
| `execution-bridge-v22.md v1.1` | ✅ 本轮定版 | 补丁5 命令面补全 + 补丁6 契约对齐 + 补丁7 断言落地 |
| 两份文件之间 | ✅ 已对齐 | 补丁4 的版本对齐声明 + v1.1 的 4 处不一致修正 |
