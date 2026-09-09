# 窗口群框架 v1.0 发布说明（README v1.2）

> 定版日期：2026-08-01
> 版本：v1.0.2（七版演进 + UX 六面 + dogfooding，177/177 全绿）
> 设计依据：`docs/project-windows-framework-design.md` v2.1 — 全部 8 条款落地
> 独立项目：`window-framework/`，不依赖 codex-rust 主仓库
>
> **v1.1 审查修正（2026-08-01）**：修正命令面错误（`workflow status --watch` → `workflow watch`）、
> 补快速开始示例、补测试运行方式、补 compress/analyze --confirm 命令。
>
> **v1.0.1 dogfooding（2026-08-01）**：真实 LLM 自建文档站成功（三指标全达成：
> analyze 首胜率 100% / 全链路 done / 人类介入 2 次），修复 12 个集成 bug。
>
> **v1.0.2 维护优化（2026-08-01）**：provider 路由落地（deepseek/zhipu/agnes/openai，
> 不再 hardcode）、VERSION 修正、v10 回归套件（12 项 dogfooding 修复保护）。

---

## 一、框架是什么

```
一个项目 = 一个共享工作空间 + N 个独立会话窗口。
窗口之间不通过中央编排器调度，靠共享工作空间 + 流转规则自己协作。
每个窗口是独立的长对话 AI agent，有自己的上下文、记忆压缩和产出边界。
```

**一句话**：建项目 → 在一个窗口里聊需求 → AI 自动分析需要什么窗口 → 自动建窗口群 → 自动流转 → 人只在 gate 点审核。

---

## 二、命令速查

```bash
# 项目管理
codex project create <name> [--type software|writing|data|research|ops]
codex project open <name>
codex project status

# 窗口管理
codex window list <project>
codex window create --name "win-xxx" --role "xxx" [--prompt "..."] <project>
codex window start <project> <id>
codex window stop <project> <id>
codex window delete <project> <id> [--hard]
codex window restore <project> <id>
codex window snapshot <project> <id>
codex window rollback <project> <id> --to <timestamp>
codex window compress <project> <id>   # v0.4: 手动触发三层压缩（自动触发 tokens>70%）

# 对话
codex window export <project> <id> [--format markdown|json] [--compress|--full]
codex window import --name "xxx" --role "xxx" --source <file> <project>

# 模板
codex template save <name> --from-window <id> <project>
codex template list
codex window create --template <name> <project>

# 全自动
codex window analyze <project> <id>     # AI 分析需求窗口对话 → 产出窗口配置
codex window analyze <project> <id> --confirm  # 确认 → 写入配置供 deploy
   # 人审核 → 确认
codex workflow start <project>          # 自动按流转规则启动窗口群
codex workflow watch <project>          # 持续轮询直到全部完成（v0.7）
codex workflow gate <project> <stage> --approve|--reject
codex workflow retry <project> <stage>  # v0.9: 只重置该 stage 窗口重跑
codex workflow deploy <project> --last  # v0.9: 复用上次 analyze 快照建窗

# 呈现/容错（v0.9）
codex window run <project> <id> --verbose  # 实时步骤流（💭🔍✏️🔧，agent 思考可见）
codex window resume <project> <id>         # 断点续传（Ctrl+C 后从断点继续）
codex status                              # 全局聚合：所有项目窗口状态

# 运维
codex framework check <project>         # 4 断言自检（独立于 codex-rust wiring）
codex conflict list <project>           # 列出共享层冲突（两窗口声明同路径，§8.4）
codex conflict resolve <project> <path> --keep <win-id>  # 仲裁：保留一方版本 + 被否方 blocked
```

---

## 三、架构关键决策（为什么这么设计）

| 决策 | 理由 |
|---|---|
| **不造中央编排器** | 主窗口不需要"领班"——流转规则是配置，不是中心 agent。需求窗口分析完就变成普通窗口 |
| **窗口 = 独立会话** | 每个窗口有完整对话历史 + 独立上下文 + 三层压缩——不会"串话"、不会上下文爆掉 |
| **共享层两层制** | 最终产出入共享（所有窗口可读），中间草稿在窗口内部——你不读完的草稿我不看 |
| **function calling 替代 YAML 解析** | v0.6 集成测试教训：LLM + 自由文本 YAML = 9 次迭代成功 1 次。function calling + JSON schema = 一次成功 |
| **gate 脚本化** | "lint + test 全绿"不是可执行判据——gate 必须是 .sh 脚本 + VERIFY_PASS 退出码 |
| **沙箱** | 每个窗口只写自己声明的 outputs 路径——窗外写操作物理拒绝 |
| **框架断言** | `framework check` 4 条，每条可断线变红——继承 codex-rust wiring 哲学 |

---

## 四、设计 v2.1 全部条款落地证明

| 条款 | 版本 | 证据 |
|---|---|---|
| 项目容器 + 窗口扫描 + 生命周期 | v0.1 | 21 tests |
| agent 引擎 + 沙箱 + budget 运行时追踪 | v0.2 | +21 tests |
| 工作流引擎 + deploy 建窗 + human/auto gate | v0.3 | +19 tests |
| 三层上下文压缩 + 快照回滚 | v0.4 | +17 tests |
| AI 需求分析 + 导入/模板 | v0.5 | +28 tests |
| 冲突仲裁 (merge + blocked) | v0.6 | +14 tests（`conflict list/resolve`） |
| function calling 结构化输出 + workflow --watch | v0.7 | +19 tests |
| UX 呈现/容错/polish（run --verbose / resume / retry / status） | v0.9 | +21 tests |
| dogfooding 真实 LLM 全链路（12 集成 bug 修复） | v1.0.1 | 160/160 回归 |
| provider 路由 + dogfooding 修复回归保护 | v1.0.2 | +12 tests（v10） |
| **合计** | | **177/177** |

---

## 五、快速开始（5 分钟跑通全自动链路）

```bash
# 0. 准备（replay 模式 0 token 验证机制；real 模式需配置 LLM key）
export CODEX_PROJECTS_ROOT=~/.codex-projects
export AGENT_MODE=replay    # 测试/演示用；生产去掉此行
# v1.0.2 provider 路由：窗口 budget.provider 决定 endpoint/key/model（deepseek/zhipu/agnes/openai）
# deepseek 为例：
export DEEPSEEK_API_KEY=xxx
# 用智谱（赠送大量 token）：建窗时 provider=zhipu，设 ZHIPU_API_KEY=xxx（model 默认 glm-4.5）

# 1. 建项目
python3 src/framework.py project create mini-blog --type software

# 2. 建需求窗口 + 注入对话（真实使用：codex window start 后和它聊天）
python3 src/framework.py window create mini-blog --name req --role 需求分析 --prompt "你是需求分析师"
python3 src/framework.py window start mini-blog win-req
# ⚠️ replay 模式下 start 不调 LLM、窗口直接 done，不会真的和人聊天——
#    analyze 需要对话内容，真实使用须用 real 模式聊天若干轮；测试可用预置对话 JSONL
#    （见 tests/integration_v06.py 的 REQ_CONV 预置示例）

# 3. AI 分析需求 → 产出窗口配置
python3 src/framework.py window analyze mini-blog win-req --confirm

# 4. 自动建窗口群 + 写流转规则
python3 src/framework.py workflow deploy mini-blog win-req

# 5. 自动流转（watch 持续轮询直到全部完成）
python3 src/framework.py workflow watch mini-blog
```

## 六、测试运行方式

```bash
cd window-framework
export AGENT_MODE=replay   # 测试不调 LLM（0 token）
# 单套件
python3 tests/test_v07.py        # 最新
# 全量回归（统一门禁入口，208/208；任一失败 exit 1）
python3 gate_window.py
# 或手动逐套件（v09/v10 为 UX 六面 + dogfooding 修复回归，勿漏跑）
for t in v10 v09 v07 v06 v05 v04 v03 v02 framework; do python3 tests/test_$t.py; done
# 真实 LLM 集成测试（模式 A，需 key + VM）
export AGENT_MODE=real DEEPSEEK_API_KEY=xxx FW_SRC=$(pwd)/src/framework.py
python3 tests/integration_v06.py
```

---

## 七、维护规程

**季度集成测试**（类似 codex-rust 维护期协议）：
- 跑一次 `mini-blog` 全链路场景（project create → analyze → deploy → workflow → 多窗口 agent 完成）
- 红线：analyze 首胜率 < 50% / sandbox 被突破 / 压缩丢失上下文

**按需增量**：
- 新项目类型 → 新增 `FC_TOOL` schema 里的 `enum` + 一条测试场景
- 修 bug → 跑受影响模块的测试 + 回归 177/177

**明确不跑**：经验实验、embedding 对比——锻造期方法，不是维护期日常。

---

## 八、已知边界（诚实声明）

| 边界 | 说明 |
|---|---|
| 真实 LLM 集成测试仅跑通 analyze→deploy 链路 | workflow 全窗口 agent 同时跑需要 `--watch` + 更长机器时间 |
| 冲突仲裁仅测试 replay 模式 | 真 agent 并行冲突场景需要更长集成测试时间 |
| 不对接 codex-rust agent-core | 框架使用自己的 agent 实现（LLM API 直接调用）。codex-rust 的 subconscious/experience/nervous-system 未对接 |
| YAML 降级路径存在 | 若 function calling 不可用 → 回退 YAML 模式 + 警告 |

---

## 九、从单 agent 到多窗口的完整弧线

```
codex-rust v12-v21（锻造九轮）
  → 证明了单个 agent 在 benchmark/应力/回放/wiring 四维火力下能稳定工作
  → 积累了 wiring 断言哲学、自适应经验系统、上下文压缩经验

codex-rust v21（维护期）
  → 身体不崩、脑子能长、唯一真 bug 已修

窗口群框架 v0.1-v0.7（七版演进）
  → 把"一个 agent 能干活"放大成"N 个独立窗口 agent 自动组队流转"
  → 复用 codex-rust 的 wiring/gate/压缩哲学
  → 从骨架到全自动，177/177，一天完成
```

两个项目共享同一条底层信念：**边界越硬、上下文越干净、单元产出越可靠。**

---

*框架已定版。分发执行窗口后进入维护期——季度集成测试，按需修复。不再有"加机制"的版本。*
