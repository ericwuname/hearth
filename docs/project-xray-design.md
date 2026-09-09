# codex-rust 顶层透视设计文档 v2.0

> **定位**: 本文档是本项目的"外部记忆"——当你感觉失控、看不清全局时，读这一份就够了。
> 它回答三个问题：**现在是什么样子**（Part A）→ **为什么是这个样子**（Part B）→ **怎么让它不再失控**（Part C）。
> 
> **维护规则**: 每次定版更新 Part A（数字 + 表格）；每次重大设计决策更新 Part B（对应节）；Part C 随 project-xray crate 的建设迭代更新。

---

## Part A: 此刻的全局快照（2026-07-30 v11.5 未提交状态）

### A.1 规模仪表盘

| 指标 | 数值 | 评价 |
|---|---|---|
| workspace 成员 | **23 crate** | 3 天前 20 个 |
| .rs 文件 | **56** | — |
| 总代码行数 | **~20,440** | +1,679 vs v10.1 |
| 测试声明数 = 真机实跑数 | **181 = 181** | ✅ 首次一致（v10.5 前每版虚高 ~50） |
| loop.rs | **2,474 行** | ⚠️ 持续长胖（v10.1 时 2,290） |
| service/main.rs | **4,404 行** | ⚠️ 组合根过重 |
| 路由 | **25 条** | GET/POST/PATCH |
| CLI 命令 | **21 条** | codex-cli |
| 启动步骤 | **20 步** | main.rs 编排 |
| 未提交改动 | **v11.4/v11.5** | HEAD=v11.3 (6e7b40a) |

### A.2 分层依赖（6 层无环 DAG）

```
L6  codex-cli        HTTP客户端, 零内部依赖
L5  service           组合根, 依赖15个crate, 唯一见到具体实现的地方
L4  agent-core        大脑, 3334行, 依赖10个crate, 全是Arc<dyn Trait>
L3  llm-openai/local/cn ｜ planner/code-index/retriever/bridge/memory ｜ tools-builtin(+sandbox) ｜ nervous/subconscious/experience
L2  llm-gateway / tool-runtime / api / lsp-bridge / resource-monitor
L1  agent-types       全局词汇表, 零依赖, 被15个crate引用
```

**关键约束**: agent-core 不依赖任何具体 LLM 实现；CLI 与内核是 HTTP 进程边界而非链接边界；三个脑区 crate 全是轻叶子。

### A.3 当前真实的接线状态（非宣称，源码 grep 可证）

| # | 能力 | 真状态 | 证据坐标 |
|---|---|---|---|
| 1 | telemetry 计数器 | 🟢 service 本地版真接 | `routes.rs:114` fetch_add → `/api/v1/telemetry` load |
| 2 | constitution 注入 | 🔴 **回归！** | v10.3 修好 → v11.4 改回硬编码摘要；`constitution_prompt()` 零调用 |
| 3 | retriever | 🟢 真接（浅扫描） | `main.rs:241-277` build + `loop.rs:1093` search；仅扫顶层非递归 |
| 4 | webhook | 🟢 真接 | `routes.rs:119` fire_event(session_created) |
| 5 | civ 自动触发 | 🔴 **从未接线** | loop.rs 零 civ 自动写入 |
| 6 | orchestrator | 🟢 可达（窄触发） | `loop.rs:927` do_act→execute_plan；task_graph 非空且工具>1 |
| 7 | workline 60s | 🟢 真接 | `main.rs:363` tokio::spawn + sleep(60s) |
| 8 | 模型发现 | 🟡 半接 | `main.rs:194-224` 写读但只 log 不 register |
| 9 | 工具生态 | 🟡 注册链路通，执行断裂 | search/install 路由真接，但 InstalledTool 无执行路径 |

**9 项中真清零 4 项、半接线 2 项、仍断裂 3 项。v10.5 宣称"9/9"不成立。**

### A.4 三个新"脑区"的真实成色

| 脑区 | 接线状态 | 核心断裂点 |
|---|---|---|
| **nervous-system** | 🟢 主链真接 | `update_cost` 零调用(cost 恒 0)；`drain_alerts` 只用不排 |
| **experience** | 🟡 闭环真但降级 | 无 embedding(只跑 keyword)；内存存储无持久化；单层非五层 |
| **subconscious** | 🟡 骨架真但信号假 | `last_success` 硬编码 true；`last_action: None`；只挂 ConstitutionGuard；DeliverAndQuit 被 `_=>{}` 吞掉 |

**规律**: 三个脑区"形先于神"——结构都搭对了，但神经末梢没接满。

### A.5 决策影响力矩阵（改 X → 影响 Y）

| 改动意图 | 受影响 crate | 风险级别 | 备注 |
|---|---|---|---|
| 加新 LLM provider | service/main.rs 1 处注册 + 新 crate | 🟢 低 | agent-core 不动 |
| 改 AgentLoop 相位 | agent-core/loop.rs + 可能 service/session.rs | 🟡 中 | loop.rs 2474 行，改动风险集中 |
| 加新路由 | service/routes.rs + main.rs 1 行注册 | 🟢 低 | 框架隔离好 |
| 改 constitution | constitution.md + agent-core/loop.rs:597 | 🟡 中 | 当前已是硬编码摘要，非真读文件 |
| 修三个脑区 | 各自 crate + loop.rs 钩子处 | 🟡 中 | 每个改动行数 <50 |
| 改 tool 系统 | tool-runtime + dispatcher + registry | 🟡 中 | InstalledTool 执行路径缺失 |
| 改 sandbox | sandbox crate + tools-builtin | 🔴 高 | VM 安全禁用 unshare，需真 Linux 复测 |
| 删任何 crate | Cargo.toml + 所有依赖该 crate 的 Cargo.toml | 🔴 高 | 需全局 grep 确认零引用 |
| 打包/部署 | Dockerfile + docker-compose.yml | 🟢 低 | 独立文件 |

---

## Part B: 为什么是这个样子——设计决策记录

### B.1 做对了的五件事（无需改动）

1. **依赖倒置是真货**。agent-core 的 10 个依赖里没有任何具体 LLM 实现。换模型 = 改 main.rs 一处。
2. **CLI 是进程边界**。codex-cli 零内部依赖，纯 HTTP 客户端。CLI 永远不可能绕过审批门/沙箱。
3. **agent-types 极度克制**。零依赖、15 个 crate 引用，没腐化成杂物抽屉。
4. **三个脑区全是轻叶子**。experience/subconscious 零到极简依赖，拆掉任何一个不伤 DAG。
5. **sandbox 收纳干净**。仅被 tools-builtin 依赖，隔离边界清晰。

### B.2 三个结构性隐患（建议治理）

1. **loop.rs 持续长胖**(2474行)。Phase 数量增长 + 钩子叠加 → 未来每次改动影响面扩大。治理：extract SubAgentExecutor（W1 seam，已延期）；考虑 do_plan/do_reflect 拆到独立文件。
2. **service/main.rs 启动序列 20 步**。线性编排无错误恢复语义——任一步 panic 全崩。治理：考虑分阶段懒加载（健康检查先行，重资源延后）。
3. **版本治理空白**。version=0.1.0、无 tag、无 CHANGELOG、v11.4-11.5 未提交、根目录 62 个 md。治理：补 tag + 建 CHANGELOG + 归档旧报告到 docs/audits/。

### B.3 首例回归的教训（constitution v10.3 修好 → v11.4 改坏）

**这是本项目最重要的制度发现**。没有接线断言测试锁住调用链，任何"已清零"的债务都能在下一版被无声退化。解决：project-xray 的 wiring 断言（§C.4）+ 短期先手写 8-10 个调用链存在性测试。

---

## Part C: project-xray 工具——让透视不再依赖人

> Part C 既是工具规格书，也是给执行窗口的实施蓝图。分为 X1-X5 五个阶段，全部在新 crate 内完成，不动主干一行。

### C.1 六视图模型

| 视图 | 回答的问题 | 数据源 | 产出 |
|---|---|---|---|
| **V1 全局观** | 多大？多少 crate/行/测试？ | Cargo.toml + walkdir | 指标卡 |
| **V2 结构观** | 谁依赖谁？分层？有环？ | deps 解析 | mermaid DAG |
| **V3 局部观** | 这个 crate 是干嘛的？暴露什么？ | 依赖反查 + pub 扫描 | 每 crate 一张卡 |
| **V4 热点观** | 哪些文件是咽喉？哪些在涨？ | fan-in/out + LOC | 热点榜单 |
| **V5 接线观** | 宣称的能力真的可达吗？ | wiring.yaml + regex | 接线表 + CI 退出码 |
| **V6 演化观** | 和上版比长/断了什么？ | facts.json diff | 漂移报告 |

### C.2 管线架构

```
[采集] cargo-toml/walkdir/git → facts.json → [分析] DAG/热点/wiring → [呈现] md/html/mermaid/diff
```

facts.json 是唯一中间格式——采集与呈现完全解耦，schema 带 version 字段。

### C.3 命令面

```
codex-xray scan    [--root .]          → facts.json
codex-xray graph   [--format mermaid]  → DAG
codex-xray report  [--html]            → 六视图报告
codex-xray wiring  [--spec wiring.yaml] → 接线检查(断裂=exit 1)
codex-xray diff    <old> <new>         → 漂移报告
```

### C.4 wiring.yaml — 接线断言（防回归防线的物质载体）

```yaml
schema: 1
capabilities:
  - id: constitution-injection
    claim: "宪法进入 build_messages"
    severity: red          # 断裂 CI 失败
    chain:
      - { file: crates/agent-core/src/loop.rs, pattern: 'Rules|constitution', meaning: "宪法摘要存在" }
      - { file: crates/agent-core/src/constitution.rs, pattern: 'pub fn constitution_prompt', meaning: "函数未被删除" }

  - id: experience-closed-loop
    claim: "经验闭环：搜→用→强化→记"
    severity: red
    chain:
      - { file: crates/service/src/main.rs, pattern: 'ExperienceStore::new', meaning: "构造" }
      - { file: crates/agent-core/src/loop.rs, pattern: '\.search\(.*goal', meaning: "检索" }
      - { file: crates/agent-core/src/loop.rs, pattern: 'store\.append', meaning: "记经验" }

  - id: nervous-reflect-override
    claim: "资源临界 → do_reflect GiveUp"
    severity: red
    chain:
      - { file: crates/agent-core/src/loop.rs, pattern: 'self\.nervous\.query\(\)', meaning: "感知" }
      - { file: crates/agent-core/src/loop.rs, pattern: 'ReflectVerdict::GiveUp', meaning: "override" }
```

**首批 12 条能力规约**使用 v11.5 盘点已确认的接线 + 曾回归的高危项。每条规约的链上任一环零命中 = 该能力断裂，severity=red 使 CI 失败。

### C.5 五阶段拆解

| 阶段 | 交付物 | 验收判据 |
|---|---|---|
| **X1** 事实采集 | `scan` 命令 + facts.json | crate 数=23；≥5 真断言测试 |
| **X2** 拓扑分析 | `graph` 命令 + mermaid | 符合 6 层结构；agent-types fan-in=15 |
| **X3** 接线引擎 | `wiring` 命令 + 12+ 规约 | 🟢 全过、🔴 正确报断；故意断线能变红 |
| **X4** 报告渲染 | `report`/`diff` 命令 + HTML | 零外链；diff 能报删除/新增 |
| **X5** CI 第四门 | gate 脚本 + 快照归档 | VM 四门全绿(fmt/clippy/test/wiring) |

### C.6 铁律（违反即 🔴）

1. 不动主干。全部代码在 `crates/project-xray/` 内。
2. 只读。产出写入 `xray-out/`。
3. 零编译期网络。无 build.rs 联网、无 CDN 依赖。
4. 单文件 HTML（内嵌 JS/CSS）。
5. 真断言测试（能失败的对固定 fixture 断言）。
6. 真机验收（VM 三门 + xray 自扫描）。

---

## Part D: 执行方案——从现在到"再不失控"

### D.1 当前最紧急（今天能做的）

| 优先级 | 行动 | 预计时间 |
|---|---|---|
| **P0** | 提交 v11.4/v11.5 的未提交改动 | 5 分钟 |
| **P0** | `cargo fmt --all` — v11.4 FMT 7 处 diff | 30 秒 |
| **P1** | 补 git tag v11.3 + v11.5 | 1 分钟 |
| **P1** | 根目录 62 个 md 归档到 `docs/audits/` | 10 分钟 |

### D.2 接线防御（防回归，最高杠杆）

| 任务 | 产出 |
|---|---|
| 写 8-10 个"调用链存在性"测试 | `test_constitution_in_build_messages` / `test_experience_append_on_run` / `test_nervous_query_in_reflect` 等 |
| 固化 wiring.yaml 首批 12 条规约 | `xray/wiring.yaml`（基于 v11.5 盘点结论） |

### D.3 "最后一厘米"三件套（每件都是小改）

| 任务 | 文件 | 行数估计 |
|---|---|---|
| ① subconscious 回填 `last_success`/`last_action` 真值 + 处理 `DeliverAndQuit` | `loop.rs` | ~15 行 |
| ② experience 挂 `set_embed_fn` + 文件持久化 | `main.rs` + `experience/lib.rs` | ~40 行 |
| ③ dispatcher 支持执行 `InstalledTool.command` | `dispatcher.rs` | ~30 行 |

### D.4 两笔烂账决断

| 项目 | 选项 A（修） | 选项 B（砍） |
|---|---|---|
| constitution_prompt | 改 `build_messages` 重新调用 `constitution_prompt()` | 删 `constitution.rs` + `lib.rs` re-export，接受硬编码摘要 |
| telemetry crate | 让 service 真依赖 telemetry，删本地 TelemetryCollector | 删 `crates/telemetry/`，接受"计数器在 service" |
| civ 自动触发 | 在 do_reflect/do_observe 中真写入 | 降级为"手动设计意图"，从债务表移除 |

### D.5 project-xray 建设（X1→X5）

按 Part C 的 C.5 节执行。预计 5 个阶段，每阶段 2-4 天，新增 ~1200-1800 行新代码。

---

## 本文档维护纪律

- Part A 每次定版更新（数字 + 接线表）
- Part B 每次重大设计决策追加条目
- Part C 随 project-xray 工具建设迭代（X1→X5 每完成一阶段更新对应节）
- Part D 每个版本结束时更新"完成/延期"状态
- **更新人**: 审计窗口 → 执行窗口收到拆解任务后，任务完成后回写状态

---

*最后更新*: 2026-07-30 01:24  
*数据基线*: v11.5 未提交状态（HEAD=6e7b40a v11.3）  
*参考文档*: `global-panorama-v11.5.md`, `architecture-perspective-v11.5.md`, `final-audit-v11.4.md`
