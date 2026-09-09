# 整体开发情况扫描 + Y 方向规划（2026-08-01）

> 扫描基线：HEAD=`06795df`（2026-08-01 18:24），全部结论经源码/git 核验，非凭报告。
> 决策前提：用户选 **Y（达成"单人好用"）**；A/B/C 中仅 A 单人部署有效，B/C 永久排除。

---

## 一、整体开发现状（两条产品线，已进稳态）

| 产品线 | 版本 | 状态 | 硬证据 |
|---|---|---|---|
| codex-rust（Rust 编码 agent） | v22 终章（未 tag） | 功能收官 + 旧债还清 | `git log` EPIC-B/C 还清；test `212` 个 `#[test]`/`#[tokio::test]`；wiring `15/15` |
| window-framework（Python 多窗口编排） | v1.0.3 | 维护期 + dogfooding 真跑 | CHANGELOG v1.0.3 `177/177`；dogfood-docs/ + dogfood-docs-multi/ 真实 LLM 产物 |

---

## 二、已清项（源码核验，Y 之前不必再做）

| # | 项 | 原状态 | 现状态 | 核验锚点 |
|---|---|---|---|---|
| 1 | `/readyz` 真实探活 | 🔴 假实现 | ✅ 已修 | `crates/service/src/routes.rs:254-263`（EPIC-B：list_persisted_sessions 失败返 503） |
| 2 | EPIC-C 神经系统告警链 / Simplify 空挡 | 🔴 死代码 | ✅ 已修 | `loop.rs:1674-1676`（Simplify 从 GiveUp 臂移除）；`docs/acceptance-final-epic-bc.md` |
| 3 | provider 路由（框架 hardcode deepseek） | 🔴 卡点 | ✅ 已修 | `window-framework/src/framework.py:385-413`（PROVIDERS 表，v1.0.2，provider=auto 路由） |
| 4 | dogfooding 真 LLM 端到端 | 🟡 已备未跑 | ✅ 已跑（单+多窗口） | commit `34b3fd4`+`63bc888`；产物 `window-framework/dogfood-docs/` + `dogfood-docs-multi/`；`spec/acceptance-dogfooding-v1.0.1.md` |
| 5 | Docker 交付物 | ⬜ 不存在 | 🟡 文件在，未 build | `Dockerfile`（685B，rust:1.82 多阶段，EXPOSE 3000）存在但未 `docker build` 验证 |
| 6 | 窗口群框架层 UX 六面 | ⬜ 0% | ✅ 全闭 | v0.8+v0.9 六面全闭（前轮已核） |

---

## 三、仍开项（Y 必须处理）

| # | 项 | 严重度 | 核验锚点 | Y 中归属 |
|---|---|---|---|---|
| A | **v22 未打 tag** | 🟡 | `git tag` 停在 v21.0 | 收口 P1 |
| B | **季度基线幽灵文档** | 🟡 | `Glob quarterly-baseline*.md` = 0 命中（被 10+ 文档引用，文件不存在） | 收口 P1 |
| C | **🔴 main.rs:208 硬编码真实 Agnes key** | 🔴 安全/合规债 | `crates/service/src/main.rs:208`（`sk-8LBZ1Gtu...` 真实密钥写死，仅作 env fallback） | 收口 P1 优先 |
| D | **底层 codex-rust §6 六面 UX** | 🟡 | `git log` 无底层 UX commit；`plan-ux-v08.md §6` 仍"待启动" | **Y 主线 P2** |
| E | **Docker 未 build 验证** | 🟡 | 仅文件存在，沙箱/landlock 在容器内行为未证 | 收口 P1 |
| F | **23 个未提交文件** | 🟡 | `git status --short` = 23 | 收口 P1 |

> 备注：底层 Rust 测试里 `llm-openai`/`llm-cn` 的 `sk-test`/`sk-abc` 均为**单元测试桩**，无害；唯一真实残留是 C 项。

---

## 四、Y 方向规划（达成"单人好用"）

### 4.1 核心判断
- "单人好用"现在分两层：**窗口群框架层已真被 LLM 证明好用**（dogfooding 单+多窗口跑通）；**底层 codex-rust 直接用 `codex-cli` 跑单 agent 的 UX 仍 0%**——这是 Y 的实质新增工作。
- 之前 Y 设想的"provider 路由 + 提交 + 智谱跑 dogfooding"**已全部由执行窗口做完**，不再重复。

### 4.2 三阶段排期（粗）

**P1 — 安全 + 收口（约 0.5~1 天，全机械、不触红线）**
1. 🔴 清 `main.rs:208` 硬编码 Agnes key → 改为纯读 `AGNES_API_KEY` env，无 fallback 写死（密钥入库即泄露）。
2. `v22` 正式打 tag（含窗口群 v1.0.3 概念上到终章，定一个统一版本号）。
3. 季度基线幽灵：要么补真实 `quarterly-baseline-*.md`，要么删 10+ 处引用（推荐删，维护期不强制季度文档）。
4. `docker build` 验证 Dockerfile 可构建 + 容器内 sandbox 行为（landlock/seccomp 在容器无 userns 时走 exec+pre_exec）。
5. 提交 23 个未提交文件（docs / dogfooding 产物）。

**P2 — 底层 codex-rust §6 六面 UX（Y 主线，主要工作量）**
对照 `plan-ux-v08.md §6`，给 `codex-cli` 直接调用的单 agent 补体验层（**纯呈现/引导，不碰 G0/G1**）：
- 指令面：CLI 子命令好记、help 清晰
- 呈现面：思考/工具调用/进度可见（colored + 结构化）
- 反馈面：失败清晰报错、可定位、可重试（禁假绿）
- 上手面：首次引导 + `.env` 配置 + 首任务顺畅
- 边界感面：审批门为何停/怎么继续，一句话可懂（之前画过红高亮逻辑落进来）
- 容错呈现面：失败后看得懂、能重来
- 验证：replay 模式 + 单测 + 静态核验（零成本，不调真 LLM）。

**P3 — 真验（拿"单人好用"终极证据）**
- 本机 Windows 用智谱免费 key 跑一次 `codex-cli` 单 agent 端到端编码任务（框架 provider 路由已支持，直连即可，无需 VM）。
- 验收标："首次上手 ≤ N 步发起首任务；失败 100% 可见；审批挂起一句话可懂"。

### 4.3 边界（维护期纪律）
- 不碰 G0/G1 结构基因；新增功能维度冻结。
- 禁止假绿：失败必须可见（Gemini 教训）。
- 零成本验证优先：replay + 单测 + 静态核验。
- B/C 多用户/对外发布永久排除，不排期。

### 4.4 给执行窗口的任务书框架（下一步细化）
- EPIC-Y1：P1 安全+收口（5 项，标不触红线可直接做，C 项优先）
- EPIC-Y2：P2 底层 §6 六面 UX（拆 6 面，replay 可验）
- EPIC-Y3：P3 真验（智谱本机端到端，验收条目见上）

---

## 五、结论
项目已比"上线距离图"那一版**大幅前进**：4 道硬门槛中 `/readyz`、provider 路由、dogfooding 已清；剩下收口是机械活（tag/季度基线/Docker/未提交）+ 1 道安全债（main.rs key）。Y 的实质增量只剩**底层 codex-rust 六面 UX**——补完即达成"单人好用"。整体已从"锻造期"进入"收口+体验打磨期"。
