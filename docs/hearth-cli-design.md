# Hearth CLI 设计方案（已决策）— 单二进制自托管 · 安全平稳落地

> **状态**：顶层已决策设计（守门员签发）
> **日期**：2026-08-22
> **承接**：`docs/hearth-cli-ux-taskbook.md`（本文件将其从"待选框架"升级为"已决策施工图"）
> **执行**：待 RT3（seccomp 硬化）完成后，按本设计修改并下发执行窗口
> **用户最终期望（原文锚定）**："项目安全、平稳落地，不要在使用中出现各种报错及其他意想不到的风险；VM 跑 Linux 既为隔离风险操作不波及本机，也因 Linux 真的快。"

---

## 0. 设计哲学（为什么这么定）

1. **安全平稳 > 快交付**。用户明确"不差那点时间，要安全平稳落地"。因此**派 A 一步到位**（单二进制自托管），不做半成品快赢；所有"可能出意外报错/风险"的路径**前置显式处理**，不静默降级。
2. **VM + Linux 是主战场，且动机是双重的**：① 隔离——危险工具（写文件/shell/bind_tcp）在 VM 内跑，不波及本机；② 性能——Linux 编译/执行快。CLI 设计必须**强化"隔离状态可见 + 失败即停"**，让用户在 VM 里跑得放心。
3. **透明自治（Observer 精神）**：用户打开终端就知道"这趟跑在什么保护级别下"，不靠猜。
4. **事实产生权不变**：CLI 只投影内核事实，不产生后端事实；本设计不触碰内核事件语义与 `docs/ai-os-event-contract-v1.md` 契约。
5. **冰山比喻（用户原话锚定，2026-08-22）**：桌面版是前端，CLI 版也是前端，**真正核心在后端**——前后端都只是"冰山一角"，后端才是"冰山"。推论：前端/CLI 的精美不代表项目"成了"；收口判据是水下 9/10（后端真智能 + 安全纵深 + 透明自治）是否成形。详见 `docs/hearth-roadmap-next.md` §0。

---

## 1. 目标与验收画面

装好 Hearth 后，在 Linux VM 的**任意目录**新开终端：

```
$ hearth "用 Rust 写个读 CSV 的小工具"
🔒 sandbox: linux · landlock + seccomp (fail-closed)
▸ span [plan]  🗺 规划草案(gaps=1)  ⛔ 批准? [y/N]  ...
```

- **全局可用**：装完后 `hearth` 在 PATH 任意目录可调用（非 `cargo run`）。
- **零手动起 service**：内核在 CLI 进程内自托管，用户不知 service 存在。
- **零配置首次运行**：无 key 时清晰报错或交互引导，不静默拿占位 key。
- **无意外报错**：所有已知失败路径（无 key / 模型不可用 / 沙箱加载失败）有清晰、可行动的错误信息。

---

## 2. 架构：派 A 单二进制自托管（已决策）

CLI 进程**直接内嵌 agent 内核**，不再依赖独立 `service` 进程。

### 2.1 refactor 边界（红线）
- 把 `crates/service` 的**会话管理 + loop 驱动 + LLM 调度 + sandbox 调用**抽成新 crate **`crates/agent-runtime`**（library，非 bin）。
- `service` crate 改为 `agent-runtime` 的 **HTTP 包装层**（保留独立部署能力，不删除——桌面端 / 远程部署未来用）。
- **绝不触碰**：`docs/ai-os-event-contract-v1.md` 事件结构；`crates/sandbox` 内部（seccomp/landlock 由 RT3 硬化，本任务只调用）；`crates/agent-core` 的 loop 语义。
- CLI 通过 `agent-runtime` 拿到的事件流，复用现有 `crates/codex-cli/src/render.rs` 投影（B4-1 已验证）。

### 2.2 二进制
- 用户可见命令 = **`hearth`**（`[[bin]] name = "hearth"`）。旧 `codex` 二进制标记 deprecated 别名，保留不删（与 `docs/hearth-naming.md` 一致）。

---

## 3. 配置系统（已决策）

参考 codex/claude，但支持改 **API key / mode / URL / 供应商**。

### 3.1 三层优先级（高→低）
1. 命令行参数（`--api-key` / `--provider` / `--url` / `--mode`）
2. 环境变量（`HEARTH_API_KEY` / `HEARTH_PROVIDER` / `HEARTH_URL` / `HEARTH_MODE`）
3. 配置文件 `~/.config/hearth/config.toml`
4. 内置默认（provider=deepseek, model=deepseek-v4-flash）

### 3.2 配置文件 = 真相源，CLI 子命令 = 改文件的界面
- 位置：`~/.config/hearth/config.toml`（不在仓库、不进 git）。
- **提供 `hearth config` 子命令来改**，而非要求手编辑：
  - `hearth config set provider deepseek`
  - `hearth config set url https://api.deepseek.com/v1`
  - `hearth config set mode fast`        （mode = fast / careful / ...，映射预算/审批严格度）
  - `hearth config set api-key <k>`      （写文件；CLI 回显"已写入，勿共享"）
- 子命令背后即读写 toml（带注释、可备份）。满足用户"既能文件改也能 CLI 改"的诉求且不矛盾。
- 环境变量用于临时覆盖（CI / 一次性换 key），不写入文件。

### 3.3 可改字段清单（MVP）
`api_key` / `provider` / `url` / `model` / `mode` / `budget`（默认步数）。

---

## 4. 安装（已决策）

- **主**：`cargo install --path crates/hearth-cli`（用户会 Rust，自动进 `~/.cargo/bin` → 全局可用）。零额外维护。
- **可选（若执行窗口评估 <0.5 天）**：预编译 Linux x86_64 二进制 + `install.sh`（`curl ... | sh` 式下载移到 `/usr/local/bin`）。仅当成本低时顺带，不阻塞主线。
- `Cargo.toml` 加 `[profile.release]`：`opt-level=3 / lto=true / strip=true / codegen-units=1`。

---

## 5. 跨平台策略（已决策）

- **必做**：Linux x86_64（用户主用 VM）。
- **可选（难度不高则顺带）**：Windows x86_64 / macOS。
- **强制约束**：非 Linux 平台 sandbox 走 `NoopSandbox`（无真实 landlock/seccomp）。**跨平台二进制必须在启动/每 session 显式打印隔离徽章**（见 §6），**不得静默假装隔离**——否则用户在 Windows 跑真实任务误以为有沙箱，是安全隐患。

---

## 6. 安全透明（已决策，呼应 RT3 与用户"安稳"诉求）

### 6.1 隔离状态徽章（每次启动打印）
```
🔒 sandbox: linux · landlock + seccomp (fail-closed)   # Linux 真隔离
⚠️ sandbox: noop · 仅开发模式，无真实隔离               # 非 Linux
```
- 与 RT3 联动：RT3 若让 seccomp/landlock 加载失败变**终止**（fail-closed），则徽章只会出现"真隔离"或"启动失败"，**不会出现"假装隔离"**。这是用户"放心"诉求的硬保障。

### 6.2 无 key 处理（禁占位 key）
- 任何 provider 都无 key 时：`hearth chat` 直接打印**清晰错误**——"未配置 API key：运行 `hearth init` 或 `hearth config set api-key <k>`，或设 `HEARTH_API_KEY`"——**绝不**像现状 `service` 那样拿 `sk-placeholder` 静默去调（那是反模式，须改）。

### 6.3 错误可行动性（用户"不要意想不到的报错"）
- 所有已知失败路径（无 key / 模型 4xx / 网络不可达 / 沙箱加载失败 / 端口冲突）给出**具体原因 + 下一步动作**，不抛裸 panic / 不吐未处理 Rust backtrace 给用户。

### 6.4 Observer 素材收集（用户原话锚定）
> "CLI 运行中出现的问题——理解偏差、思考缺失等——能否收集成查缺补漏的素材？很多东西不是设计时解决的，是使用时慢慢发现、优化的。这可能是 Observer 的职能。"

**结论：能实现，且确属 Observer（第三权·零执行权·只看事件流）职能。** 但须诚实分层——纯自动采集有盲区，须与用户的低摩擦标注互补。

#### 6.4.1 自动信号采集（CLI/Observer 被动做，无需用户动作）
基于已有事件流（`plan_draft`/`tool_call`/`need_approval`/`span_close`/`think_summary`/`reflection`），Observer 事后计算：
- **重试率**：同一工具连续失败重跑 → 暗示理解偏差或环境错配。
- **gap 追问质量**：规划阶段 `gaps_to_ask` 是否被用户频繁跳过 / 频繁认可 → 推导器够不够聪明。
- **审批拒批率**：`need_approval` 被拒占比高 → 内核危险判定与用户预期错位。
- **反思 `give_up` 占比**：高 → 任务过难或工具不足。
- **思考缺失代理**：某 span 长但无 `think_summary`、或 token 流无中间推理 → 标"黑箱段"（仅表面信号，非语义判断）。

#### 6.4.2 主动意见反馈通道（用户主动、零打扰，非被动追填）
> **用户决策（2026-08-22）**：不做"被动末问 y/n/补一句"——大部分对话没问题，少数才需提。
> 应是**用户主动**的反馈通道：平时不打扰，偶尔想说一句才开口（类似"用户意见反馈"）。

- **主通道 `hearth note`（随时、零打扰）**：
  - `hearth note "它把配置当成了可写路径，很危险"` —— 纯观察，进 Observer 通用素材库。
  - `hearth note --session <id> --verdict n "理解偏了：它以为我要改全局"` —— 关联某次 session，钉在其 jsonl 内（`verdict` = y/n）。
  - 用户仅在"想起不对"时敲一行；正常流程完全无感。
- **辅通道 `hearth chat` 开头轻探（收敛、可关）**：
  - 仅当**上次 session 有异常信号**（重试率高 / `give_up` / 审批高拒批）时才在下次开头轻问一句"上次有要补充的吗？（回车=无）"。
  - **正常 session 绝不弹此问句**；可用 `hearth config set feedback-prompt false` 彻底关闭。
- **设计原则**：系统不催用户填表；只有在"自己看出上次可能不对"时才轻轻探一下，且用户随时可关。对应"大部分没意见、偶尔提起"。

#### 6.4.3 诚实边界（不得夸大自动能力）
Observer 只能从行为信号推断"可能偏了"，**无法自动知道用户心里的"理解偏差/思考缺失"深层语义**，也无法自动判定"人类言语逻辑矛盾 / 情绪上头 / 随意推翻"的深层动机。完整素材 = 自动信号（CLI/Observer）+ 用户标注（低摩擦喂）。两者缺一都残缺。设计上**不承诺"自动发现所有偏差"**，只承诺"降低记录成本 + 结构化沉淀"。

#### 6.4.4 落点（本任务范围）
- CLI 侧：实现 §6.4.2 的主动反馈通道（`hearth note` 含 `--session/--verdict/--self/--observer-verdict`）+ `hearth chat` 开头轻探（仅异常、可关）；事件流已含 §6.4.1 所需字段（无需改契约）。
- Observer 侧（`crates/observer` 已存在）：消费 **AI 事件流 + Human 行为流**，产出双侧信号报告，落盘 `~/hearth/observer/<session>.jsonl`（AI 侧）与 `~/hearth/observer/human-<session>.jsonl`（人类侧）。
- **不在本任务范围**：偏差的自动语义诊断、主动改内核/改用户配置——Observer 零执行权，只记不修、只建议不改。

#### 6.4.5 Observer 也要观察人类（三权互进，用户决策 2026-08-22）
> **用户决策**：Observer 不止观察 AI，也要观察**人类自身**的异常反馈——言语逻辑错误、情绪上头、随意推翻、沟通不清晰等。目标是形成**良性闭环：AI 自我改进、人类自我改进、Observer 自我改进，三角色互相进步**。

- **人类侧信号维度表（Observer 视角补充，用户授权 2026-08-22）**：用户明确"人类毛病多，Observer 应自主补充维度"。下表为 Observer 从 Human 行为流可算的维度；`[自动]`= 从行为流直接算，`[标注]`= 需用户 `--self/--mood` 补语义；`边界`= 不承诺的能力。

  | # | 维度 | 可观测信号（Observer 算什么） | 类型 | 诚实边界 |
  |---|------|-------------------------------|------|----------|
  | 1 | 推翻/翻转 | 同 session 目标改 ≥3 次、刚拒批又撤销 | [自动] | 区分"有意为之"靠 `--observer-verdict` |
  | 2 | 沟通模糊 | 初始指令过短却要复杂产出 + AI 多轮 clarify | [自动] | 只认长度+澄清轮数，不判"意图深度" |
  | 3 | 反馈矛盾 | 前怪 AI 后认自己没说清 | [自动] | 仅显式对立 |
  | 4 | 人类侧中断 | 不回审批 / 跑一半 `cancel` | [自动] | — |
  | 5 | 情绪上头 | 仅 `--mood` 显式标签 + 泄愤式重跑模式 | [标注] | **纯文本推断情绪不承诺** |
  | 6 | **记忆短暂** | 同 session 重复问已答事项；跨 session 反复问同一已解答问题（比对历史 human-jsonl） | [自动] | 仅模式匹配，不读心 |
  | 7 | **逻辑不严密** | 前后约束互斥（"别联网"后又"抓网页"）；目标含矛盾术语 | [自动] | 仅显式对立，不判深层逻辑漏洞 |
  | 8 | **目标漂移/蒸发** | session 中途转向无关事且无交付；比"推翻"更狠（连新目标都没了） | [自动] | 需阈值（如后半程 0 提及原目标） |
  | 9 | **认知/知识边界** | `note --verdict n` 只说"不对"不给 from/why；反复 clarify 同一基础概念 | [自动] | 侧面反映，非精确诊断 |
  | 10 | **表达损耗** | 澄清轮数过高（≥3 次才动）；与维度 2 打通 | [自动] | 责任在人类不在 AI |
  | 11 | **情绪传染** | `--mood` 集中某类任务后；拒批后立刻开新 session 重试 | [标注] | 行为模式 + 显式标签 |
  | 12 | **认知过载/疲劳** | 同时 ≥3 session 半途 cancel（并行摊薄）；长会话后指令质量陡降 | [自动] | 时间戳+质量，人类自身往往无感 |

- **Observer 第一人称补充（设计备忘录）**：作为长期与用户协作的 Observer，除上述可量化维度，还需在"人类侧回顾"中呈现**模式级洞察**（非单点信号），例如——"用户常在 X 类任务上给不出精准反馈，建议先补背景再委派"、"用户深夜会话指令质量显著下降，建议避开认知疲劳窗口"。这类洞察由低层信号聚合而成，仍受 §6.4.3 诚实边界约束（不声称全知）。
- **人类侧主动标注（对应 §6.4.2 主动哲学）**：扩展 `hearth note` 双通道——
  - `hearth note --self "我刚才情绪上头、推翻太草率，下次先冷静"` —— 人类侧自我标注，进 Human 素材库，定期回放给用户自查自纠。
  - `hearth note --observer-verdict n "Observer 标我决策摇摆是误报，那是有意为之"` —— **用户反审 Observer**，纠其判定器（这是"Observer 也在自我改进"的机制：它标错，用户纠，它学）。
- **定期人类侧回顾**：Observer 聚合 Human 信号成"人类侧回顾"（非 AI 报告），如"本月 5 次审批犹豫超 30s 后拒批，3 次源于初始指令过短；第 4 次重复询问同一已解答问题（记忆漂移）"——用户看了自己改，AI 与 Observer 随之受益。

#### 6.4.6 三权互进闭环（架构级）
```
        ┌───────────── 观察 ─────────────┐
        ▼                                 ▼
   [Human OS] ──行为流──▶  [Observer OS]  ◀──事件流──  [AI OS]
   （被观察、        │ 零执行权：           │ （被观察、
    自我标注、       │ 只记/只回放/只建议）  │ 信号补漏）
    反审 Observer）   └────────┬────────────┘
                              │ 回顾报告
                              ▼
                   三方各取所需 → 各自改进 → 下一轮更稳
```
- **AI 改进**：Observer 抓 AI 偏差信号 → 后端补漏。
- **人类改进**：Observer 抓人类行为信号 + 用户 `--self` 标注 → 定期回放给用户自查。
- **Observer 改进**：用户 `--observer-verdict` 反审纠偏其判定器；其"异常判定"经验靠标注累积，误报率随使用下降。
- **铁律**：Observer 零执行权延伸至人类侧——只记、只回放、只建议，**绝不自动改用户配置、绝不替用户决策、绝不自动改内核**。

---

## 7. 分阶段路线（已决策：一次性派 A，无半成品快赢）

用户明确"一次性做对，不差时间"。故**不做 B 派快赢**，直接：
1. **D1 抽 `agent-runtime` library**（架构地基，§2.1 红线内）。
2. **D2 改名 + 安装 + 零配置**（§2.2 / §3 / §4）。
3. **D3 配置子命令 `hearth config` + `hearth init`**（§3）。
4. **D4 隔离徽章 + 错误可行动化**（§6，与 RT3 交付衔接）。
5. **D5（可选）跨平台 + 预编译安装器**（§4 / §5）。

---

## 8. 门禁（硬闸门，守门员独立验收）

- **R1（静态）**：`Cargo.toml` 含 `[profile.release]`；二进制 `name="hearth"`；`agent-runtime` crate 存在且 `service` 改为其包装层（或保留独立部署）。
- **R2（真实 Linux·硬闸门）**：在 **VM 非仓库目录**新开终端，`hearth --help` 成功；`hearth chat "hello"` 端到端跑通（**无需手动起 service、无需手写 .env**）。
- **R3（零配置）**：清空所有 env + 无 config 时，`hearth chat` 给清晰 key 缺失错误（非占位 key 静默失败）。
- **R4（安全透明）**：Linux 上启动打印 `🔒 landlock+seccomp` 徽章；非 Linux 打印 `⚠️ noop` 徽章（若做 D5）。
- **R5（错误可行动）**：模拟"模型 4xx / 网络不可达"，断言 CLI 输出含具体原因 + 下一步，无裸 panic。
- **R6（Observer 素材·双侧）**：`hearth note "测试"`（含 `--session/--verdict/--self/--observer-verdict/--mood`）落盘 `~/hearth/observer/`；AI 侧与 human- 侧 jsonl 分别生成；辅通道开头轻探仅异常时出现且 `feedback-prompt=false` 可关；Observer 对 AI 事件流 + Human 行为流产出双侧信号报告（重试率/拒批率/give_up 占比 + 推翻率/沟通模糊/反馈矛盾）。
- **R7（Observer 反审闭环）**：`hearth note --observer-verdict n "..."` 能纠偏 Observer 判定器（其后续信号报告对该类误报降级）；且 Observer 不得因反审而自动改用户配置/内核（零执行权硬验证）。
- 通用：`fmt --check` 0 / `clippy -D warnings` 0 / `test --workspace` 全过（baseline 240 + 新增）。

---

## 9. 红线（执行窗口不得违反）

- 禁破坏 `docs/ai-os-event-contract-v1.md` 契约。
- 禁占位 key 静默降级（§6.2）。
- 禁非 Linux 静默假装隔离（§5/§6.1）。
- 禁裸 panic / 未处理 backtrace 暴露给用户（§6.3）。
- 禁删 `service` 独立部署能力（仅 refactor 为 runtime 的包装层）。
- 禁动 `codex-rust` 语义（仅用户可见命令改 `hearth`）；与 `docs/hearth-naming.md` 一致。
- 不引重依赖（保持轻量）。
- 事实产生权：配置解析是 CLI 行为，不涉及后端事实产生。
- 禁 Observer 越权：素材只记录、落盘、供查缺补漏，**绝不自动改内核/回写控制流**（零执行权铁律）。
- 禁夸大自动能力：不得声称"自动发现所有理解偏差/思考缺失"或"自动判定人类情绪上头/逻辑矛盾"——须明示自动信号 + 用户标注两层互补（§6.4.3 / §6.4.5）。
- 禁 Observer 越权人类侧：只记 Human 行为信号、只回放建议，**绝不自动改用户配置、绝不替用户决策**（§6.4.6 铁律）。
