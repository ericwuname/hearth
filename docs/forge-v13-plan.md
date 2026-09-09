# codex-rust v13 — 锻造稳定期：锁死能力 + 跨 provider 对比 + 去债

> 定位：v12.7 证明了 agent 在单 provider / 单遍条件下能跑通 95-100%。v13 不做新功能——**锁死已证明的能力，推到多 provider / 多遍稳定，清两笔拖了五版的历史烂账**。
> 铁律：三个修复（record_tool_exchange / read_only_view / T15 自洽）已经是 backbone，决不允许任何重构无声退化。

---

## §1 起点状态（v12.7 gatekeeper 过闸）

| 维度 | 数值 |
|---|---|
| 通过率（zhipu, 单遍） | **95%（19/20）** / 重跑 T19 → **100%** |
| 真机三门 | fmt ✅ / clippy ✅ / test **183/0** ✅ |
| 根因修复 | 3 项（grep 死循环、子智能体写冲突、T15 测例矛盾） |
| 主干改动 | `loop.rs` + tool-runtime `dispatcher.rs` |
| bench 设施 | 20 fixture T00-T19 + runner.py batch |
| 剩余债务 | constitution 不读文件 / civ 不自动触发 / experience 无持久化 / subconscious 半硬编码 |

---

## §2 v13 目标：三线并行

### 线 A：接线断言防线（把 v12.7 的三条命锁死）—— 落地机制：建 codex-xray 二进制

**为什么最高优先级**：v10.3 修好 constitution 注入 → v11.4 改坏无人察觉。这次的三项修复价值太高，绝不能重蹈覆辙。

**落地机制（用户裁决：建 codex-xray 二进制）**：在 `crates/project-xray/` 新建 crate，实现 `scan`（facts.json：crate/rs/LOC/test 实算，替代守门员手算）+ `wiring`（按 `wiring-v13.toml` 断言引擎（TOML：toml 已是 workspace 依赖，零新增，规避 VM 编译期网络风险），链上任一环零命中=断裂，exit 1）。`wiring-v13.toml` 为声明式规格（7 条）。该二进制挂两处：① `.github/workflows/ci.yml` 新增 step（GH push 时跑）② `vm_upload.py --gate`（VM 开发循环真机验证时跑）—— GH Actions 只在 push main 触发，护不住 VM 日常开发，故两处都挂。

**自证要求（验收 🔴）**：每条断言必须证明能变红——故意断开一条链（如注释 `record_tool_exchange(`），gate 须 exit≠0，再还原。否则视为未生效。

| 断言 ID | 保护什么 | 断言方式 |
|---|---|---|
| `tool-exchange-wired` | `record_tool_exchange` 在 `do_act` 仍被调用 | grep `loop.rs` 含 `record_tool_exchange(` |
| `readonly-view-strips` | `read_only_view` 仍剥离 `write_file/edit/apply_patch/bash` | grep `dispatcher.rs` 含 `MUTATING_TOOLS` 含 `"bash"` |
| `subagent-uses-readonly` | `spawn_sub_agent` 仍用只读分发器 | grep `loop.rs` 含 `read_only_view()` |
| `tool-exchange-pairing` | 仍然是按下标配对（不依赖 call_id） | grep 含 `for (i, call) in calls.iter().enumerate()` |
| `self-verify-in-prompt` | system prompt 仍含 `cargo test` 自验证指令 | grep 含 `cargo test` + `COMPILER ERRORS` |
| `constitution-reads-file` | constitution 从 `constitution.md` 运行时读（线 C·A 接已落实） | grep `constitution.rs` 含 `read_to_string` 或 `include_str!` + `constitution.md` |
| `civ-auto-written` | `do_reflect`/`do_observe` 真写 `civ_store`（线 C·A 接已落实） | grep `loop.rs` 含 `civ_store` 或 `CivWriter` 在 reflect/observe 调用 |

### 线 B：跨 provider 基准对比（你的 agent 在不同大脑上差多少？）

v12.7 只在 zhipu 上跑通了。用户记忆里有多个可用 provider——我们需要知道 agent 在它们上面的表现差异。

| provider | 模型 | 状态（源码核查 2026-07-30） |
|---|---|---|
| zhipu | glm-4-flash | ✅ v12.7 主力（95%），本轮必跑 |
| doubao | deepseek-v4-flash | ✅ 可用（v12.0 曾跑 T02），**本轮用户未选** |
| deepseek | deepseek-v4-flash | ✅ **已注册且为 service 主力**（`service/src/main.rs:204-217`，OpenAI 兼容，代码含 fallback key），本轮纳入 |
| agnes | agnes-2.5-flash | ⚠️ 专线不稳（临时免费通道，延迟高），本轮纳入但先 preflight |
| ollama | qiyuan-8b:latest | 🔑 **VM 不可达**（跑在 Windows 本机 localhost:11434，VM service 够不到）→ 剔除 |
| google / openai | — | ⛔ VM 上 `code=000` 超时，排除 |

> 注：google/openai 在 VM 不可达（历史核查）；ollama 同理。两轮排除项不计入分母。

**产出**：`bench/results/provider-matrix-v13.md` —— **{zhipu, agnes, deepseek} 各 20 题 × 1 遍**（doubao 本轮不打、ollama 剔除）。每个 provider 先 T00 冒烟 preflight 通过才跑 20×，避免白烧 token。

### 线 C：历史烂账决断（五版没清的两笔债）

| 债务 | 选项 A（接） | 选项 B（砍） |
|---|---|---|
| **constitution 不读文件** | 改 `build_messages` 重新调用 `constitution_prompt()`，像 v10.3 那样读运行时文件 | 承认"硬编码摘要 ~50 token"是设计选择，删 `constitution.rs` 里死代码的 `constitution_prompt` 函数 |
| **civ 不自动触发** | 在 `do_reflect` / `do_observe` 中真写 civ_store | 降级为"手动设计意图"（已有多条 API 路由），从债务表正式移除 |

**守卫规则**：不管选 A 还是 B，选了之后必须加 wiring 断言锁死。

---

## §3 阶段拆解（定版）

| 阶段 | 线 | 任务 | 预计 | 依赖 |
|---|---|---|---|---|
| **S1** | A | 建 codex-xray crate：`scan`（facts 实算）+ `wiring` 引擎 + `wiring-v13.toml`（7 条）+ 单测（含"故意断链变红"自证） | 1.5 天 | 无 |
| **S2** | A | codex-xray 挂 CI（`ci.yml` 新 step）+ 挂 VM gate（`vm_upload.py --gate`） | 0.5 天 | S1 |
| **S3** | C | 两笔烂账 A 接实施：constitution 运行时读 `constitution.md` + civ 在 `do_reflect`/`do_observe` 真写（新增 `CivWriter` trait 解耦 agent-core）+ 各补 wiring 断言 | 1.5 天 | 无（与 S1 并行） |
| **S4** | B | provider 对比跑分：{zhipu, agnes, deepseek} 各 20×1（每 provider 先 T00 preflight） | 1 天（机器时间） | v12.7 runner 已就绪 |
| **S5** | B | provider 对比报告：三元组 + 失败分类漏斗（按 provider 分层） | 0.5 天 | S4 |
| **S6** | 全 | 多遍稳定性：zhipu 20×3（先 20×2 看趋势再定第 3 遍） | 1 天（机器时间） | S2/S3 |
| **S7** | 全 | 守门审计 + `tag v13.0` + `forge-report-v13.md` + `CHANGELOG-v13.md` | 0.5 天 | S1-S6 |

### §3.1 执行注记（VM 血泪陷阱，必读）
- 真代码在 `~/codex_work`，**不是** `~/codex`（`~/codex` 是旧快照）。gate 一律 `cd ~/codex_work && cargo fmt/clippy/test`。
- tar 上传后必须 `touch` 所有 `.rs`（或 `find ~/codex_work/crates -name '*.rs' -exec touch {} +`），否则 cargo 见"源码比产物旧"→ 复用旧 target → 假通过/假报错。
- VM 上 github 不可达 → 严禁引入编译期 fetch 依赖（v13 不加新依赖；若 S1 的 codex-xray 需 `regex`/`serde_yaml` 等新 crate，须确认 crates.io 可达、github 不可达不影响）。
- 服务监听 **3000** 非 8080。

---

## §4 线路优先级与执行窗口

```
窗口 A（高优）：线 A 接线断言（S1+S2）→ 防回归，最急
窗口 B（中优）：线 C 烂账决断（S3）→ 每个决策带断言锁
窗口 C（并行）：线 B 跨 provider 跑分（S4+S5）→ 机器时间，可后台
窗口 D（收尾）：S6 多遍稳定 + S7 守门审计 + tag
```

**周目标**：
- Day 1: S1+S2+S3 完成（接线断言上线 + 烂账决策）
- Day 2: S4 启动 provider 对比跑分（后台留 VM 跑）
- Day 3: S5 对比报告 + S6 多遍跑分 + S7 tag v13.0

---

## §5 验收红线

| 级别 | 判据 |
|---|---|
| 🔴 | v12.7 三项修复 + constitution/civ 对应的 **7 条** wiring 断言任一不在 CI 产出里，或 codex-xray 未在 VM gate 跑 |
| 🔴 | **wiring 断言未经自证**（无法证明会变红） |
| 🔴 | 任何改动导致 zhipu 通过率 < 90%（S6 多遍跑分证） |
| 🔴 | constitution/civ 决策只有代码修改没有对应的 wiring 断言 |
| 🟡 | provider 对比只有单遍，未排除偶发 |
| 🔵 | 多 provider 对比报告格式 |

---

## §6 v14 预留（本轮不做）

- **应力测试 S 维度**（forge-design §3）：恶意输入/资源枯竭/并发轰炸
- **价值维度 V**（forge-design §5）：人机对照实验
- **experience 持久化**：当前内存 RwLock<Vec> → 文件 + 向量化（**确认不在 v13 范围**，与 governance/REPORT 并称"高杠杆"但 v13 已收窄为"锁死+对比+去债"，故推至 v14）
- **subconscious 信号完全动态化**：当前仍有部分硬编码
- **回放测试**：用 v12.7 成功 session 消息作为回放 fixture
- **codex-xray 完整度**：本轮只做 `scan`+`wiring`（v13 够用）；`graph`(mermaid 拓扑)/`diff`(演化漂移)/`report` 富呈现延至 v14

## §7 定版决策（用户裁决，2026-07-30）

| 议题 | 决策 | 落地 |
|---|---|---|
| Constitution 债务 | **A 接**：运行时读 `constitution.md` | S3 改 `constitution_prompt()` 读文件 + 加 `constitution-reads-file` 断言 |
| CIV 债务 | **A 接**：`do_reflect`/`do_observe` 真写 `civ_store` | S3 加 `CivWriter` trait 解耦 + 调用 + 加 `civ-auto-written` 断言 |
| Wiring 门机制 | **B 建 codex-xray 二进制** | S1 新建 `crates/project-xray/`（scan+wiring），挂 CI + VM gate |
| Provider 对比范围 | **{zhipu, agnes, deepseek}** | S4 跑分（deepseek 已注册主力、agnes preflight、ollama/doubao 排除） |
| 评审补充采纳 | 8 项全采纳（wiring 可执行性/自证/provider 可行性/决策门/VM 陷阱注记/CHANGELOG-v13/v14 一致性/S6 成本） | 已并入 §2–§5 与 §3.1 |

---

*锻造断语：v12.7 是"能跑了"。v13 回答三个问题——能一直跑吗（锁死：codex-xray 接线门）？换脑子跑一样吗（对比：跨 provider 矩阵）？堆了五版的债该清了吧（决断：constitution/civ 双 A 接）？*
