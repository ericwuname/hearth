# Hearth 骨架层对标成熟 harness：十轮深挖与地基裁决（砺·评审）

> **日期**：2026-09-01　**窗口**：砺·评审（🪨）　**基线**：本机 `HEAD = 7c21805`（P4 Node 02 已落）｜`Cargo.toml = v0.2.19`
> **触发**：顶层提问——"骨架层对标市面成熟 harness 能不能打？缺点在哪？优点在哪？要不要对标加固？做了这么久的骨架，最后地基还是不稳。"
> **性质**：只读审计。零代码改动。
> **纪律**：AS-IS 每条带实测锚点；对标侧每条带外部来源；**差距必须定性为"代差 / 量差 / 无差距 / 领先"四档之一**，不许含糊说"有差距"

---

## 0. 先回答恐惧：地基到底稳不稳

**结论：承重结构是稳的，但有三处软土层，且三处都不是"功能没做"，而是"做的时候没按崩溃模型和对抗模型设计"。**

| 地基分档 | 判定 | 依据（一句话） |
|---|---|---|
| **① 承重结构**（状态机/依赖/类型/门禁） | 🟢 **稳（B+）** | PDCA 显式状态机 + 九态终端 + 失败分类 F1-F10 + 六策略 + 单向依赖 DAG + 464 测试 + CI 五门禁 + `-D warnings`。真材实料，不是纸糊的 |
| **② 崩溃安全**（写盘/持久化/恢复） | 🔴 **不稳（C-）** | 工具层裸 `tokio::fs::write`；超长写**先 truncate 原文件再追加**；落盘只在"正常跑完"时发生，kill 即丢整轮；零快照零回滚 |
| **③ 对抗安全**（沙箱闭环/凭据） | 🟡 **中等偏下（C+）** | seccomp 默认 KILL + landlock 真启用（比多数 harness 严）；但 **egress 白名单只管 `web_fetch`，`bash curl` 出网零管控**；生产沙箱缺 `env_clear()` |

**为什么会恐惧但又不该恐惧**：你担心的"骨架是假的/空壳"，实测不成立——27 crates、464 测试、生产区 5070 行只有 1 处 `unwrap()`、0 处 `todo!()`、单向依赖无环。真正该担心的从来不是"有没有"，而是"**崩了之后会怎样**"和"**模型使坏时会怎样**"——这两处恰好是成熟 harness 花最多力气、而我们几乎没做的。

**要不要对标加固**：**要，但只加固地基，不追功能清单。** 见 §11——必做 3 项（约 M 量级），可选 4 项，明确不必做 5 项。

---

## R1 · 建立对标坐标系：先确定"跟谁比、比什么"

不先定坐标系就直接比，必然变成"拿自己长板比别人短板"的自嗨。所以第一轮先立规矩。

### R1.1 对标对象选取与可比性判据

| 对象 | 选取理由 | 可比性 | 采信来源 |
|---|---|---|---|
| **OpenAI Codex CLI（codex-rs）** | 同为 **Rust 重写**的终端 agent，同为 OS 内核级沙箱。唯一与 Hearth 在工程范式上同构的对象 | **最强**（语言/形态/沙箱机制三维同构） | [DeepWiki 5.3](https://deepwiki.com/halfprice06/codex/5.3-sandboxing-and-security)、[Platform Implementation](https://codex.danielvaughan.com/2026/04/08/codex-sandbox-platform-implementation)、[Internals](https://codex.danielvaughan.com/2026/04/10/codex-cli-internals-queue-pair-guardian-sandbox)、[botmonster 综述](https://botmonster.com/posts/openai-codex-cli-rust-powered-ai-agent) |
| **Claude Code** | 工程完成度天花板：54 内置工具、27 类 hook 事件、subagent 隔离三模式 | 中（TS 实现，但架构分层是公开研究的标杆） | [Agent Loop 官方文档](https://code.claude.com/docs/en/agent-sdk/agent-loop)、[Subagents 文档](https://code.claude.com/docs/en/sub-agents)、[arXiv:2604.14228 架构剖析](https://claude-wiki.com/dive-into-claude-code-the-design-space-of-today-s-and-future-ai-agent-systems-ar.html) |
| OpenHands / Aider / Cline | 不单列 | 弱（Python/容器范式，与 Hearth 不同构） | — |

### R1.2 差距四档定义（后九轮统一使用）

| 档 | 定义 | 处置 |
|---|---|---|
| 🔴 **代差** | 机制层面缺失，不是参数调优能补的 | 必做或明确接受风险 |
| 🟠 **量差** | 机制相同，参数/覆盖度/成熟度差 | 按需调参 |
| ⚪ **无差距** | 实质等价 | 不动 |
| 🟢 **领先** | 我们有而标杆没有 | 保护，别在"对标"中丢掉 |

### R1.3 规模对照（先把期望值校准，避免用错尺子）

| 项 | Hearth | Codex CLI | 倍数 |
|---|---|---|---|
| 贡献者 | 1（+AI 协作者） | 400+ | — |
| 发布标签 | v0.2.19 | 640+ | — |
| commits | — | 5,075+ | — |
| GitHub stars | — | 72,000+ | — |
| Rust 占比 | 100% | ~95% | ≈1× |

**判读**：任何"功能清单数量"的直接比较都是**无效比较**。有效比较只有两类：**机制有无**（布尔）和**崩溃/对抗下的行为**（布尔）。所以下面九轮我**刻意不比工具数量**——8 vs 54 是人力差，不是架构差，拿它说事是自欺。

⚠️ **本轮就推翻了一个前提**：我自己的长期记忆里写着"crates = 23"，实测 `ls -1 crates/ | wc -l` = **27**。陈旧条目已骗到我了——这正是本项目反复强调"排期前必须 grep 源码"的原因。

---

## R2 · 执行引擎与状态机

### R2.1 AS-IS（全部实测复核）

| 项 | 实测 | 锚点 |
|---|---|---|
| 相位状态机 | `LoopPhase` enum **7 变体**（Init/Plan/Act/Observe/Reflect/Done/Error），单一 `match` 分发器 7 臂 | `loop.rs:29-37`、`4324-4395` |
| 迁移载体 | `StepOutcome{next, emit}`；`Agent::step(phase) -> StepOutcome` | `loop.rs:839-842`、`847` |
| 驱动 | `let mut phase = Init` → `loop { ... phase = outcome.next; steps += 1 }` | `loop.rs:4483`、`4522`、`4752-4753` |
| 终端状态 | **九态封闭集** starting/running/waiting_for_user/paused/completed/failed/aborted/cancelled/deadline_exceeded | `terminal.rs:16-26` |
| 失败分类 | **F1–F10** 十类 | `terminal.rs:261-316` |
| 恢复策略 | **六策略** Retry/Repair/Replan/Verify/Escalate/Stop，**已接线** | `terminal.rs:324-354`、`loop.rs:3960/3970` |
| 重试 | `MAX_TRANSIENT_RETRIES=4`、`TOTAL_RETRY_CAP=120s`、退避 2/4/8/16s；401/403 不重试、免费 429 快失败 | `loop.rs:2960-2961`、`2996-3025` |
| 循环上限 | `max_steps` 默认 **50**；replan 上限 3；verify_replan 上限 3 | `agent-types` Budget Default 330、`loop.rs:2828/4773` |
| 活性检测 | **5 个**：graph_stall(≥2)、stuck_loop(2 步无 edit)、steps_without_progress、consecutive_errors(≥3 降级)、same_tool_repeat(≥2) | `loop.rs:2572-2579`、`3556-3582`、`2893`、`terminal.rs:306-308` |

### R2.2 标杆怎么做的

Codex CLI 的 agent loop 是"三层循环结构 + 异步 channel"（[InfoQ 2026-02](https://www.infoq.com/news/2026/02/codex-agent-loop/)、[OpenAI Blog](https://openai.com/index/unrolling-the-codex-agent-loop/)）；Claude Code 是 `queryLoop()` + 五层架构（Surface/Core/Safety-Action/State/Backend）。**两者都没有 Hearth 这种"显式相位状态机 + 分类化恢复策略"的公开结构**——它们的循环更扁平，恢复主要靠模型自己看着办。

### R2.3 裁决

| 维度 | 判定 | 说明 |
|---|---|---|
| 相位状态机 | 🟢 **领先** | 显式 7 相位 + `StepOutcome` 迁移，比扁平 loop 更易推理和观测。这是真优势，**别在重构中丢掉** |
| 失败分类 + 六策略 | 🟢 **领先** | F1–F10 × 六策略是结构化恢复，标杆主要靠 LLM 自决 |
| 终端九态 | ⚪ **无差距** | 与 Codex 的 Result subtype 等价 |
| 重试/退避 | ⚪ **无差距** | 4 次 + 120s 上限 + 指数退避，是标准做法 |
| 死循环防护 | 🟠 **量差** | 5 个检测器够用，但**全部是启发式计数**，没有基于"状态哈希重复"的严格环检测 |

### R2.4 但有一个结构性隐患（本轮最重要的发现）

`loop.rs` **9535 行，生产区 5070 行（占 53.2%），测试区 4464 行（46.8%）**。生产区实测：

- `do_plan_inner` = **831 行**（`2390-3220`）
- `run` = **635 行**（`4398-5032`）
- `do_reflect` = **438 行**（`3885-4322`）
- `do_act` = **387 行**（`3221-3607`）
- `build_messages` = **289 行**（`2041-2329`）

而 `grep -c "── " loop.rs` = 26 处分区注释，**全部在测试区**（首个在 `:5079`）；生产区 `1-5070` **零分区注释**。

好消息（实测）：生产区 `unwrap()` = **1 处**、`unsafe` = **0 处**（2 处都在测试）、`todo!()/unimplemented!()` = **0 处**。

**判读**：这不是"代码烂"，是"**没有拆**"。一个 831 行的 `do_plan_inner` 里塞了规划、缺口推导、重试分流、replan 判定——它会持续拖慢每一次改动，也会让下一轮新会话的 Hearth 读不懂它。**这是地基的"可维护性软土"，但不影响当前正确性。**

---

## R3 · 工具层

### R3.1 AS-IS

**工具集 8 个**（实测 `run_local.rs:242-252`，逐个 `register`）：bash / read / edit(write_file) / patch / glob / grep / introspect / web_fetch。

| 维度 | 实测 | 锚点 | 判定 |
|---|---|---|---|
| 统一 trait | `Tool` trait：name/description/execute/declared_timeout | `dispatcher.rs:57-69` | 有机制 |
| 参数校验 | **全部手写 if**，8 处 `ok_or_else(|| anyhow!("missing 'x' argument"))`；`#[derive(Deserialize)]` **零命中** | `bash.rs:117`、`edit.rs:59/105`、`patch.rs:60/62/64`、`glob.rs:153`、`grep.rs:73`、`read.rs:45`、`web.rs:86` | 有机制但有缺陷 |
| 超时 | 全局默认 30s；`effective = min(timeout, remaining_task_time)`；`tokio::time::timeout`；超时 **SIGKILL + cgroup 移除** | `dispatcher.rs:66-67/162/263-288`、`sandbox` `1128-1143` | 有机制 |
| 超时分级 | bash 声明 610s（默认 180s / cap 600s），其余 7 个 30s | `bash.rs:88-89/140-141` | 有机制 |
| 并发 | `dispatch_parallel` + `join_all`，多调用并行单调用串行 | `dispatcher.rs:308-318`、`scheduler.rs:48-104` | 有机制 |
| 错误结构化 | `ToolErrorKind` 五类 + downcast 分类（禁字符串解析） | `agent-types/lib.rs:281`、`scheduler.rs:8-17` | 有机制 |
| **错误对模型可见性** | `loop.rs:2348-2354` 只 `format!("ERROR: {}", r.output)`，**`error_kind` 不进上下文**（生产区仅 `:3956` 一处消费） | `loop.rs:2348-2354` | 🔴 **有缺陷** |
| **原子写 / 备份 / 回滚** | **零命中**（`grep -rni "dry_run\|backup\|\.bak\|undo\|rollback\|snapshot_file"` 于 tools-builtin + agent-core → 无输出） | — | 🔴 **代差** |
| **文件锁 / 并发写互斥** | **零命中**（`grep -rn "FileLock\|flock\|lockfile"` → 无输出） | — | 🔴 **代差** |
| **进程复用（shell 状态）** | **零命中**（`grep -rn "unified_exec\|persistent\|shell_state"` → 无输出）；`bash.rs:176` 每次 `spawn` 新进程 | `bash.rs:176` | 🟠 **量差** |
| **hooks 扩展点** | **零命中**（webhook 是通知，不是生命周期 hook） | `service/src/lib.rs:9-10` | 🟠 **量差** |
| **MCP** | **零命中** | — | 🟠 **量差** |

### R3.2 标杆怎么做的

| 项 | Codex CLI | Claude Code |
|---|---|---|
| 进程复用 | `UnifiedExecProcessManager` 复用长生命周期进程，**保持 shell 状态（env/cwd）**；上限 64 进程，60 起告警 | — |
| 输出上限 | `HeadTailBuffer` **1 MiB**，保留 head+tail 丢弃 middle | — |
| 改前快照 | — | **修改前自动存档受影响文件，随时回滚**；`--fork-session` 分叉 |
| 工具数 | 核心少数 | 54 内置工具 |
| 扩展点 | MCP client + server、GitHub Action | 27 类 hook 事件、MCP |

### R3.3 裁决

| 维度 | 判定 | 理由 |
|---|---|---|
| 超时 + 确定性 kill | ⚪ 无差距（甚至更细：deadline_clamped 与 tool timeout 语义分离，`dispatcher.rs:300-302`） | 机制到位 |
| 结构化错误 | 🟠 量差——**机制有，但没送到模型**。结构化为控制流服务，模型侧退化成纯字符串 | 补一行格式的代价，收益明确 |
| 原子写 / 回滚 | 🔴 **代差** | Claude Code 改前快照是标配；我们零 |
| 文件锁 | 🔴 **代差** | 多会话并发写 = 后写覆盖 |
| 进程复用 | 🟠 **量差** | `cd xxx` 在下一条命令失效，长任务里是真实摩擦 |
| 输出上限 | 🟠 量差 | 我们 6000 字符（head 4000 + tail 1500，`loop.rs:5033-5046`，**有截断标记** ✅）；Codex 1 MiB。差 170 倍，但**策略同构**（都是保头尾） |
| 工具数量 8 vs 54 | ⚪ **不判** | 人力差，不是架构差。刻意不纳入评分 |

### R3.4 本轮最硬的一处（复核确认）

`crates/tools-builtin/src/edit.rs:141-158` 超长写分块逻辑实测：

```rust
let head: String = content.chars().take(WRITE_CHUNK).collect();
tokio::fs::write(&path, head.as_bytes()).await        // ← 先把原文件 truncate 掉
    .map_err(|e| anyhow!("write failed for {}: {e}", ...))?;
let rest: String = content.chars().skip(WRITE_CHUNK).collect();
let mut f = tokio::fs::OpenOptions::new().append(true).open(&path).await ...;
for chunk in rest.as_bytes().chunks(WRITE_CHUNK * 3) { f.write_all(chunk).await ...; }
```

**崩溃后果**：原内容已被 truncate 销毁，新内容只写了一部分 → **用户文件半截且不可恢复**。短路径 `edit.rs:162`、`patch.rs:113`、`patch.rs:151` 同样是裸 `tokio::fs::write`，无 tmp + rename + fsync。

**讽刺点（必须写下来）**：`crates/memory` 和 `codex-cli/session_store.rs` **都有原子写**（tmp + `sync_all` + rename，见 `session_store.rs:44-60`、`memory:96-142`），**偏偏真正改用户代码的工具层没有**。这说明不是团队不会，是这件事从来没被当成过需求。

---

## R4 · 沙箱与安全模型

### R4.1 AS-IS（关键项已亲自复核）

| 机制 | 状态 | 锚点 | 复核 |
|---|---|---|---|
| seccomp-BPF | **默认 KILL_THREAD**（`0x0000_0000`）；`HEARTH_SECCOMP_MODE=errno` 才退化为 ERRNO\|EPERM | `lib.rs:738-798`、`781-786`、`805-809`、`1090-1092` | ✅ 默认最严档 |
| landlock | **真启用**（非 DEFERRED）；`/` 全树 FS_RO + cwd FS_RW + `/dev/null` 显式 FS_RW；`handled_access_net = 0` | `lib.rs:339`、`357-449`、`389-390`、`415`、`422-425` | ✅ 真启用 |
| cgroup v2 | 默认 512MB / 10 CPU-s / 32 pids；build 档 4GB / 600 / 512 | `lib.rs:87-89`、`118-121` | ✅ |
| **namespace 隔离** | **零命中**（`grep -rn "unshare\|pivot_root\|chroot\|CLONE_NEW"` → 仅 2 处注释） | `lib.rs:4`、`1039` | 🔴 **代差** |
| **seccomp 拦网络** | **不拦**。`SYS_SOCKET=41`/`CONNECT=42`/`RECVFROM=45`/`SOCKETPAIR=53` **均在白名单** | `lib.rs:475-478`、`621-624` | 🔴 **代差** |
| seccomp 白名单规模 | `[u32; 123]` | `lib.rs:597` | — |
| **生产路径 `env_clear()`** | **缺失** | `lib.rs:1078-1085` 仅 `.envs(env_vars_clone)` | 🔴 **代差** |
| 进程加固 | 无 `PR_SET_DUMPABLE`、无 `RLIMIT_CORE=0`、无 DYLD strip | — | 🟠 量差 |
| 沙箱拒绝后升级 | **无**（fail-closed 直接打死工具）；逃生阀 `HEARTH_ALLOW_NO_CGROUP=1` | `lib.rs:886-888/903/922/955/984` | 🟠 量差 |

### R4.2 我亲自复核的三处（不采信子代理结论）

**① `env_clear()` 倒挂 —— 确认属实，且比报告更严重**

生产路径 `LinuxSandbox::spawn`（`lib.rs:1078-1085`）：
```rust
let mut child = StdCommand::new(&cmd_clone);
child.args(&arg_clone).envs(env_vars_clone).current_dir(&cwd_clone) ...   // 无 env_clear()
```
而 **dev 用的 `NoopSandbox`（`lib.rs:185`）反而有 `.env_clear()`**：
```rust
tokio::process::Command::new(cmd).args(args).current_dir(cwd).env_clear().envs(...)...
```
→ **生产路径比非 Linux 开发路径更宽松**，父进程全部环境（含 `HEARTH_API_KEY` / `CODEX_API_KEY`）直透 LLM 生成的子进程。这是明确的倒挂，不是设计选择。

**② landlock 全树可读 —— 我给子代理的结论降档**

子代理判为"🔴 最高价值发现"。我复核 `lib.rs:80-83`：
```rust
// grants READ+EXECUTE ONLY (FS_RO) — writes stay forbidden everywhere
// except `writable_paths` (the workspace cwd), so the host filesystem
// cannot be modified. (Analogue of `firejail --ro-root`.)
std::path::PathBuf::from("/"),
```
→ 注释**明说**这是 `firejail --ro-root` 的对应物，是**有意设计**。且 Codex CLI 同样是 "grants read access to the entire filesystem"（[Platform Implementation](https://codex.danielvaughan.com/2026/04/08/codex-sandbox-platform-implementation)）。
→ **降档为 ⚪ 行业共性**。全树可读（含 `~/.ssh`、`.env`）是 landlock 模型的固有边界，不是我们的缺陷。子代理报告在此处过度告警。

**③ cgroup fail-closed 影响面 —— 子代理修正了我的前提**

我原以为 fail-closed 会打死全部 bash/write。实测只打死走 sandbox 的三工具：**bash**（`bash.rs:18`）、**glob**（`glob.rs:20`）、**grep**（`grep.rs:19`）；`edit`/`patch` **直连 `tokio::fs`、不过 sandbox**（`edit.rs:63-65` 自承 "SBOX-1 accepted risk"）→ 写文件不受 cgroup 故障影响，但也**不受 cgroup 资源约束**。

### R4.3 标杆怎么做的

| 项 | Codex CLI Linux |
|---|---|
| 隔离层数 | **三层**：Bubblewrap（`--ro-bind / /` + `--unshare-user --unshare-pid --unshare-net`）+ Landlock + seccomp |
| 网络 | seccomp **明确拒绝** `connect`/`bind`/`sendto`/`sendmsg`，AF_UNIX 豁免保 IPC |
| 出网治理 | `codex-network-proxy`（Rama 框架 MITM 代理）强制域名白名单 |
| 进程加固 | `prctl(PR_SET_DUMPABLE,0)` + `setrlimit(RLIMIT_CORE,0)`，跨 mac/Linux |
| 沙箱拒绝 | `is_likely_sandbox_denied()` 启发式 → 提示用户 → 获批后无沙箱重试（escalation flow） |
| 敏感路径 | `.git` / `.codex` 在 writable root 内**重新置为只读** |

### R4.4 裁决

| 维度 | 判定 | 说明 |
|---|---|---|
| seccomp 默认档 | 🟢 **领先** | 默认 KILL_THREAD 比 ERRNO 更狠（进程直接死，不给绕过机会） |
| landlock 启用 | ⚪ 无差距 | 与 Codex 同构 |
| cgroup 资源限制 | ⚪ 无差距 | 有，且 build 档 CPU 限制因 `max_cpu<100` 条件跳过（`lib.rs:974`）属小瑕疵 |
| **namespace 隔离** | 🔴 **代差** | Codex 有 bwrap 三层，我们只有 LSM 两层且同 namespace |
| **网络管控** | 🔴 **代差** | 我们 socket/connect 全放行；Codex seccomp 拒 + MITM 代理 |
| **`env_clear()`** | 🔴 **代差 + 倒挂** | 生产比 dev 宽松，key 直透子进程 |
| 沙箱拒绝升级流 | 🟠 量差 | 我们 fail-closed 打死；Codex 有"申请无沙箱重试" |

### R4.5 交叉印证的一条（两个独立来源同时命中）

`crates/tools-builtin/src/web.rs:9-10` 源码自承：
> "bash curl 后门治理依赖全局代理 env…列为 🟡 遗留"

即：**`egress-allowlist` 只约束 `web_fetch` 一个工具，`bash curl` 完全不受约束**，而 curl 所需的 socket/connect 全在 seccomp 白名单（`lib.rs:621-622`）。我派出的两个独立取证代理分别在"安全模型"和"网络层"视角下命中同一点，且源码自己承认。
→ **`web_fetch` 号称的"唯一 sanctioned 出网口"是自我声明，不是机制保证。** 这是 R4 最该修的一条。

---

## R5 · 上下文管理与持久化（**软土层最集中的一轮**）

### R5.1 AS-IS

| 项 | 实测 | 锚点 |
|---|---|---|
| 压缩阈值 | `COMPACT_CHAR_THRESHOLD = 32_000`、`COMPACT_KEEP_TURNS = 2` | `context.rs:173`、`176` |
| **provider-aware 注入** | 阈值 = `window × ratio(0.6) × 2.55`；Agnes 硬编码 `Some(128_000)` → **195,840 字符** | `context.rs:234-240`、`llm-openai/src/lib.rs:312`、注释实证"探针 est=68 thr=195840" |
| 优先级陷阱 | `env > 注入 > 常量`——env 一旦设置会压掉 provider-aware 主路径 | `context.rs:234-240` |
| 压缩实现 | `summarize_turn` 纯 `format!` 模板，**不调 LLM** | `context.rs:420-486` |
| 丢弃内容 | Assistant 全文 + tool_result **100% 不进摘要**（只含 goal/tools/files 三字段） | `context.rs:482-484` |
| **压缩前归档** | ✅ 有：JSONL 追加到 `~/.config/hearth/archive/<sid>.jsonl` + 摘要头注入 grep 检索提示 | `context.rs:288`、`366-382`、`343-350` |
| 模型窗口探测 | **零**（硬编码 128K，真实 512K） | `llm-openai/src/lib.rs:312` |
| token 配额管理 | **零**。无 tokenizer、无按组件预算；`set_system_chars` 仅事后记账供 `introspect` 显示 `fill_pct` | `loop.rs:2154`、`1334-1343` |

**静默截断族实测 6 处**（进入模型输入的）：

| # | 位置 | 阈值 | 截断标记 |
|---|---|---|---|
| 1 | `loop.rs:5033-5046` | 6000（head 4000 + tail 1500） | ✅ 有 |
| 2 | `agent-types/src/lib.rs:558/565-566` | 4096 | ✅ 有 |
| 3 | `web.rs:17/127-132` | 8000 | ✅ 有 |
| 4 | `constitution.rs:20/79-81` | 6000 | ❌ **无**（当前 1430 < 6000，不触发） |
| 5 | `loop.rs:2207-2229` | 40 条消息 | ✅ 有 |
| 6 | `context.rs:430-434`（摘要 goal） | 60 字符 | ❌ 仅 `…` |

### R5.2 持久化与崩溃恢复（**本轮核心**）

| 项 | 实测 | 锚点 | 判定 |
|---|---|---|---|
| CLI 侧持久化 | `session_store.rs` JSONL，**完整 Turn 历史**，**原子写**（tmp + `sync_all` + rename） | `session_store.rs:44-60` | 有机制 |
| service 侧 | `memory` JsonlMemoryStore **只存元数据 + 1 条 done 事件**，不存对话轮次；原子写 + 写锁 + 64MiB + 坏行容错 | `memory:96-142`、`session.rs:339/714` | 有机制（粒度不足） |
| **落盘时机** | `save_snapshot` 唯一调用点 `run_local.rs:505`，位于 `Ok(Ok((r, agent)))` 分支内 → **kill / 崩溃 / 超时 / 取消，该轮历史零落盘** | `run_local.rs:501-505`、`:500`、`:554` | 🔴 **代差** |
| resume | `hearth resume <id>` → `restore_history(turns)`；REPL 崩溃重建 | `lib.rs:589-641`、`repl.rs:465/479` | 有机制 |
| **checkpoint** | **无** | — | 🔴 代差 |
| **工具层原子写** | **无**（见 R3.4） | `edit.rs:141-162`、`patch.rs:113/151` | 🔴 代差 |
| **改前快照 / 回滚** | **零命中** | — | 🔴 代差 |
| 并发写互斥 | **零命中** | — | 🔴 代差 |
| **session fork** | **零命中** | — | 🟠 量差 |

### R5.3 标杆怎么做的

- **Claude Code**："write the conversation to disk **as events occur**"（[arXiv §9](https://claude-wiki.com/dive-into-claude-code-the-design-space-of-today-s-and-future-ai-agent-systems-ar.html)）——append-only JSONL + parent-UUID 链支持分支；**修改前自动存档受影响文件，随时回滚**；`--fork-session` 分叉；subagent **sidechain 独立 JSONL**（子 agent 历史不污染主会话，只回传 summary）。
- **Codex CLI**：`model_context_window` / `model_auto_compact_token_limit` 两个**配置项**（我们只有 env + 硬编码），可把窗口推到 1M。
- 两者都有 **PreCompact hook**（压缩前归档完整 transcript）——我们的 `archive_compacted_turns` 实质对标上了。

### R5.4 裁决

| 维度 | 判定 | 说明 |
|---|---|---|
| 压缩前归档 | ⚪ 无差距 | 我们有 `archive_compacted_turns`，与 PreCompact hook 同构 |
| 压缩算法质量 | 🟠 量差 | 都是"摘要替换"，但标杆用 LLM 摘要（保留语义），我们纯模板丢 Assistant 全文 + 全部 tool_result |
| 窗口自发现 | 🟠 量差 | 标杆靠**配置项**显式声明（可推到 1M）；我们硬编码 128K，**真实 512K 的 1/4 → 在真实容量 15% 处触发压缩** |
| **事件级持久化** | 🔴 **代差** | 我们"跑完才写"，标杆"边跑边写" |
| **改前快照/回滚** | 🔴 **代差** | 零命中，而这是长任务唯一的安全网 |
| **工具层原子写** | 🔴 **代差** | 且超长写"先毁原文件"是数据损毁放大器 |
| 静默截断 | 🟠 量差 | 6 处中 2 处无标记（宪法、摘要 goal），但当前均不触发/影响小 |

⚠️ **本轮更正我自己的长期记忆**：记忆里写"32k 硬编码 → 在真实容量 4–10% 处触发压缩"。实测已升级为 provider-aware 注入 **195,840 字符**，硬编码窗口 128K vs 真实 512K → **15%**，且 32k 现已降级为"无 provider 信息时的回退常量"。**记忆条目已过时，须更正。**

---

## R6 · 人机交互与审批通道

| 项 | 实测 | 锚点 | 判定 |
|---|---|---|---|
| 审批分类 | **非裸字符串前缀**：按 `\|;&\n` 切段 → 命令位取词 → 剥路径与后缀 → 比对 18 词表 | `lib.rs:591-647`、`609-623`、`635-636` | 有机制，质量不错 |
| 硬红线 | `/dev/`、`/proc/`、`/sys/` → HardRedline；`/dev/null` 豁免 | `lib.rs:576`、`579` | 有机制 |
| 非交互拒绝 | **非静默跳过**：emit `InteractionRequested{denied:"noninteractive"}` + `tracing::warn` + `run_abort` + 返回 `Error` | `loop.rs:3293-3343`（`:3316`/`:3330`/`:3335`/`:3340`）、`run_local.rs:310` | ✅ **已修复**（此前记录的 RC24 静默失败已解决） |
| clarify 闭环 | `InteractionRequested{kind="clarification"}` | `loop.rs:2440-2454` | 有机制 |
| 审批审计 | delegate 分支 `audit=true`；**交互式 approve/deny 分支零审计日志** | `lib.rs:3273-3279` vs `3381-3402` | 🟠 量差 |
| 审批粒度 | 默认每次问（`Interactive` 为 `#[default]`）；会话委托仅内存 `Vec`，**无持久化** | `lib.rs:449-450`、`1006`、`3280` | 🟠 量差 |
| 硬红线不被委托放行 | ✅ `if !has_hard_redline` | `lib.rs:3265` | 好设计 |

**标杆**：Codex 四审批策略（untrusted / on-failure / on-request / never）+ `is_known_safe_command()` 命令白名单 + `with_escalated_permissions`；Claude Code 权限系统 **deny-first + ML 分类器** + 27 类 hook。

**裁决**：审批分类质量 ⚪ 无差距（我们的切段取词比裸前缀匹配严谨）；审批策略丰富度 🟠 量差（我们是单策略，标杆是四策略可选）；审批持久化 🟠 量差。

**给顶层的一条更正**：此前记录在案的 **RC24「`/dev/null` 审批门导致 one-shot 静默失败」现已修复**（`loop.rs:3293-3343` 显式 emit + warn + abort）。总账里这条应标 CLOSED，别再当成未修项排期。

---

## R7 · 可观测性与重放

| 项 | 实测 | 锚点 | 判定 |
|---|---|---|---|
| Observer crate 独立性 | ✅ **不依赖 agent-core**（依赖仅 api + serde/toml/uuid 等；`grep agent-core` 于 observer + api 的 Cargo.toml → 无输出） | `observer/Cargo.toml:7-14` | 🟢 **领先** |
| 三端口架构 | Human OS / AI OS / Observer OS（零执行权） | 架构级 | 🟢 **领先** |
| Projection 违约 | **系统性违约**：tracing subscriber 输出到 stderr，与 Projection 契约冲突 | `lib.rs:264-266`（Node 02 已证） | 🔴 代差 |
| 截断族无标记 | 8 处中 6 处无标记（Node 02 §E 已证） | `docs/core-revalidation/projection-audit.md:34-44` | 🟠 量差 |
| Debug 泄漏 | 16 行 | Node 02 §D | 🟠 量差 |
| replay | `llm-replay` crate 621 行 + `agent-core/src/replay.rs` 51 行；wiring 有 replay 接线断言 | `crates/llm-replay`、wiring | 有机制 |
| 事件流/SSE | service 有 SSE | `crates/service` | 有机制 |

**标杆**：Claude Code 有 27 类 hook 事件（ observability 的扩展点）；Codex 有 queue-pair Op/Event 协议（外部客户端可对接）。

**裁决**：**架构理念 🟢 领先**——三端口 + Observer 零执行权是 Codex/Claude **都没有**的第三方观察权设计，这正是"审计工具必须自审计"的落地。**但实现 🔴 有代差**——有了独立的观察器官，却把最基础的投影隔离做错了（stderr 违约），等于装了摄像头却对着墙。

**判读**：这是本轮最该记的一句话——**我们有别人没有的架构，却没把它最基础的一层做对。** 加固时优先修 Projection 隔离，而不是去追 hook 事件数量。

---

## R8 · 模型/供应商抽象与容灾

| 项 | 实测 | 锚点 |
|---|---|---|
| provider 抽象 | 6 个 provider crate：llm-openai(886) / llm-cn(795) / llm-local(1423) / llm-replay(621) / llm-gateway(1750) / bridge(284) | `crates/llm-*` |
| 多 provider 共识 | `crates/bridge` `BridgeSession`，被 agent-runtime + service 引用 | `crates/bridge` |
| 窗口声明 | 硬编码：`llm-openai:312` → 128_000；`llm-cn:412` → 32_000；`llm-local:382/839` → 128_000；`llm-replay:351` → None | 硬编码，零探测 |
| 失败转移 | 重试 4 次 + 120s 上限 + 指数退避；401/403 不重试、免费 429 快失败 | `loop.rs:2960-3025` |
| token/成本 | `CostGuard`；wiring 有成本纪律断言 | wiring |
| 流式 | SSE 支持 | `llm-openai`、`service` |
| **自动 failover 到备用 provider** | **未取证到**（bridge 是"共识"非"故障转移"） | `crates/bridge` |

**标杆**：Codex 有 `model_context_window` 配置项可 push 到 1M；Claude Code 有 effort 分级（low/medium/high）控成本。

**裁决**：provider 抽象 ⚪ 无差距；**窗口声明方式 🟠 量差**（硬编码 vs 配置项，我们是 env 覆盖、标杆是配置文件，实质接近）；**failover 🔴 代差**（单 provider 挂掉就整体失败，无自动降级到备用通道）。

**这一条对本项目特别重要**：我们的战略是"**弱 LLM 下也能交付高质量**"，而弱 LLM 通道（Agnes 会员配额）恰恰是最可能限流/降级的。**没有 failover，弱 LLM 战略就没有容灾。**

---

## R9 · 测试与门禁：464 个测试到底证明什么

### R9.1 分布（实测）

`grep -rn "#\[test\]\|#\[tokio::test\]" --include=*.rs crates/ | wc -l` = **464**（1 个 `#[ignore]`）。
Top5：agent-core 124 / tools-builtin 49 / project-sync 45 / llm-gateway 27 / sandbox 24。
`loop.rs`：`#[cfg(test)]` 起于 **:5071**（实测 `grep -n "^#\[cfg(test)\]"`），测试区 4464 行（46.8%），**83 个测试**，均 ~54 行/测（不是一行流）。

### R9.2 抽样 10 个（子代理抽样，我复核统计口径）

**烟测 2 个**：`loop.rs:6036`（构造函数回读）、`loop.rs:6057`（Debug 字符串断言）。

**真断言 8 个**：
1. `loop.rs:9507` 造 50 条消息（>40）→ 断言输出含 `[history note]`，删标记即红
2. `loop.rs:6908` 10 组真实中文 prompt 正反例，含 T5 回归
3. `context.rs:1033-1047` 压缩保真：**读盘**验证 archive 真落盘 + 逐项断言字段存活
4. `context.rs:1282-1289` 显式标注"修复前恒 false = 死代码"的红测试
5. `memory/src/lib.rs:397` **手工注入坏行** `{ this is not valid json` → 断言 2 条有效事件存活
6. `edit.rs:260` `/etc/passwd`、`../escape` 必须 err（路径穿越）
7. `edit.rs:450` 12000 字符中文分块后 `assert_eq!(saved, long)` 逐字符相等
8. `sandbox/src/lib.rs:1180` `DIR_ONLY_MASK | FILE_MASK == 0x7FFF` 位掩码无漂移

### R9.3 门禁实测

| 项 | 实测 | 判定 |
|---|---|---|
| CI | `.github/workflows/ci.yml` 45 行：fmt → clippy(`-D warnings`，且全局 `RUSTFLAGS: "-D warnings"`) → test --workspace → xray wiring → xray scan | ✅ 五门禁，配置完整 |
| 接线门禁 | **16 条 capability / 26 条 chain link**（文件头自称"15/24"，注释已过期）；引擎 = `strip_comments_and_strings` + `contains` → **100% 源码子串存在性断言** | 有机制但只锁结构 |
| 引擎自测 | wiring 引擎自身有 4 个测试防注释误判 | ✅ 扎实 |
| **性质测试 / fuzz** | **零命中**（`grep -rn "proptest\|quickcheck\|arbitrary\|fuzz" --include=Cargo.toml` → 0；无 `fuzz/` 目录） | 🔴 代差 |
| 集成测试 | 仅 3 文件 / 18 测：service 侧 **有真端到端**（MockProvider + 真实 dispatcher → create_session → SSE drain 至 Done）；**`run_local.rs`（954 行，CLI 主路径）零集成测试** | 🟠 量差（且缺口位置最危险） |

### R9.4 裁决

| 维度 | 判定 | 说明 |
|---|---|---|
| 测试规模/密度 | ⚪ 无差距 → 偏优 | 464 测、loop.rs 测试占比 46.8%、抽样 80% 为真断言（含手工注入坏行、路径穿越、多字节分块） |
| CI 完整性 | ⚪ 无差距 | 五门禁 + `-D warnings`，比多数开源项目严 |
| **性质测试/fuzz** | 🔴 **代差** | 零。而解析器类代码（bash 切段 `lib.rs:591-647`、patch 匹配）正是 fuzz 收益最高的地方 |
| **CLI 主路径集成测试** | 🔴 **代差** | resume/落盘/崩溃恢复全在 `run_local.rs`，恰好零覆盖 |
| 接线门禁 | 🟠 量差 | 全是子串存在性，**不锁运行时行为**——`constitution-reads-file` 检查 `read_to_string` + `constitution.md` 两子串存在，不验证宪法内容真进了 prompt |

**给"464 tests · 0 fail"一句公道话**：它**确实**证明了函数级行为正确（80% 抽样是真断言，不是烟测），**不**证明：① 崩溃时行为正确 ② 并发时行为正确 ③ 恶意输入下行为正确 ④ 端到端流程可用。这四项恰好是 R3/R5/R4 的软土层。

---

## R10 · 综合裁决：能不能打 / 地基稳不稳 / 要不要加固

### R10.1 能不能打？分三种"打"分别回答

| 问法 | 答案 | 理由 |
|---|---|---|
| **功能完整度能不能打**？ | ❌ **打不过，且不该打** | Codex 400+ 贡献者 / 640 releases / 54 工具 / MCP / subagent。我们是单人 + AI 协作。比功能清单是拿人力差当架构差，无效比较 |
| **架构先进性能不能打**？ | ✅ **有两处真领先** | ① **三端口 + Observer OS 零执行权**（crate 级独立，不依赖 agent-core）——Codex/Claude 都没有第三方观察权设计；② **宪法八条显式注入 + trust-but-verify**——标杆没有认知宪法层；③ **PDCA 显式相位状态机 + F1-F10 失败分类 + 六策略**——比扁平 loop 更易推理 |
| **能不能安全地跑长任务**？ | ⚠️ **当前不够，加固后够** | 承重够，但崩溃安全和对抗安全有三处会在长任务中必然暴露（见下） |

### R10.2 地基三档判定（直接回答恐惧）

| 档 | 判定 | 三处软土（按严重度） |
|---|---|---|
| ① 承重结构 | 🟢 **稳（B+）** | 无 |
| ② 崩溃安全 | 🔴 **不稳（C-）** | **S1** 工具层裸写 + 超长写先毁原文件（`edit.rs:141-162`）<br>**S2** 落盘只在正常结束时（`run_local.rs:501-505`），kill 丢整轮<br>**S3** 零改前快照 / 零回滚 |
| ③ 对抗安全 | 🟡 **中等偏下（C+）** | **S4** egress 白名单只管 `web_fetch`，`bash curl` 零管控（`web.rs:9-10` 源码自承）<br>**S5** 生产沙箱缺 `env_clear()`，比 dev 路径宽松（`lib.rs:1078-1085` vs `:185`）<br>**S6** 无 namespace 隔离（Codex 有 bwrap 三层） |

**三处软土的共同点（这是本次审计最重要的一句话）**：
> **它们都不是"功能没做"，而是"做的时候没按崩溃模型和对抗模型设计"。**
> 原子写（`session_store.rs:44-60`）、坏行容错（`memory:397` 手工注入坏 JSON）、路径穿越防护（`edit.rs:260`）——这些能力我们**全都会**，但只用在"自己的状态文件"上，**没有用在用户代码上**。

**所以你的恐惧可以精确化**：不是"骨架是假的"，是"**骨架给自己穿了救生衣，给用户代码没穿**"。这个恐惧**是对的**，而且比"骨架是空壳"更值得处理——因为它更容易被忽略。

### R10.3 要不要对标加固？——要，但**只加固地基，不追功能清单**

**判据**：加固项必须满足两个条件之一——① 会在长任务中**必然暴露**（不是"可能"）；② 是**崩溃/对抗**维度（不是功能丰富度）。

---

## 11. 加固清单（按 ROI 排序）

### 必做（S 级，M 量级工作量，全是"把已有能力复用出去"）

| # | 项 | 锚点 | 做法（复用现有能力） | 解决 |
|---|---|---|---|---|
| **S1** | **工具层原子写** | `edit.rs:141-162`、`patch.rs:113/151` | 照抄 `session_store.rs:44-60` 的 tmp + `sync_all` + rename；分块改为**先写 tmp，最后 rename**（不再 truncate 原文件） | 崩溃不再毁用户文件 |
| **S2** | **改前快照 + 回滚** | 新增 | 写操作前把原文件复制到 `~/.config/hearth/snapshots/<sid>/<hash>`；提供 `hearth rollback` | 长任务唯一安全网（对标 Claude Code） |
| **S3** | **增量 checkpoint** | `run_local.rs:501-505` | 每 N 步（或每轮结束）`save_snapshot`，而非只在 `Ok` 分支；kill 后可 resume 到最近检查点 | kill 不再丢整轮 |
| **S4** | **补齐 `env_clear()`** | `lib.rs:1078-1085` | 生产路径加 `.env_clear()` 再 `.envs(白名单)`，与 `NoopSandbox:185` 对齐（1 行改动） | key 不再泄漏给 LLM 子进程 |
| **S5** | **出网统一收口** | `web.rs:9-10`、`lib.rs:621-622` | 二选一：① seccomp 拒 `connect`（AF_UNIX 豁免，对标 Codex，但会打死 DNS 与 LLM 调用，需放通白名单 IP）；② bash 命令级代理 env 强制注入（当前源码自承的 🟡 遗留） | egress 白名单从"自我声明"变"机制保证" |

### 可选（A 级，看排期）

| # | 项 | 理由 |
|---|---|---|
| A1 | **bash 进程复用**（保持 cwd/env） | 长任务真实摩擦：`cd xxx` 下条命令失效。对标 Codex `UnifiedExecProcessManager` |
| A2 | **fuzz / 性质测试** | 优先喂 `lib.rs:591-647` 的 bash 切段解析器和 patch 匹配器——解析器类代码 fuzz 收益最高 |
| A3 | **`run_local.rs` 集成测试** | 954 行 CLI 主路径零覆盖，而 resume/落盘/崩溃恢复全在这 |
| A4 | **wiring 门禁加运行时断言** | 现 16 条全是子串存在性；至少给 `constitution-reads-file` 加一条"宪法内容真进 prompt"的行为断言 |

### 明确不必做（D 级，别在这些上花时间）

| # | 项 | 理由 |
|---|---|---|
| D1 | 追工具数量（8 → 54） | 人力差不是架构差。核心 8 个够用，缺的是质量不是数量 |
| D2 | MCP / subagent / TUI | 属 v0.3 功能层，不是地基。且记忆已证：subagent 只有 `max_tokens` 常量，是功能缺口不是地基缺口 |
| D3 | 拆 `loop.rs`（9535 行） | 疼但不致命：生产区只有 1 处 `unwrap()`、0 处 `todo!()`。**列为技术债，不与地基加固抢排期** |
| D4 | landlock 全树可读 | ⚪ 行业共性（`firejail --ro-root` 同构，Codex 同样全树可读）。我给子代理的 🔴 判定**降档** |
| D5 | 全面对标 Codex 沙箱三层（bwrap） | 引入外部二进制依赖，与"单二进制零依赖"战略冲突。S4/S5 收口后风险已大幅下降 |

### 顺带：总账更正 2 条

1. **RC24（`/dev/null` 审批门导致 one-shot 静默失败）→ 已修复**，应标 CLOSED。证据：`loop.rs:3293-3343` 显式 emit `InteractionRequested{denied:"noninteractive"}` + `tracing::warn` + `run_abort` + 返回 `Error`；触发条件 `stdin 非 tty`（`run_local.rs:310`）。此前"静默失败"的描述已过时。
2. **"crates = 23" → 27**；**"压缩 32k 硬编码" → provider-aware 195,840（32k 降为回退常量）**；**"测试 455" → 464**。三条陈旧数据须同步更正。

---

## 12. 溯源表

| 结论 | 取证方式 | 是否亲自复核 |
|---|---|---|
| crates = 27 / 测试 = 464 / loop.rs 生产区 5070 行 | `ls -1 crates/ \| wc -l`、`grep -c "#\[test\]"`、`grep -n "^#\[cfg(test)\]"` | ✅ 亲自 |
| 生产区 `unwrap()`=1、`unsafe`=0、`todo!`=0 | `sed -n '1,5070p' loop.rs \| grep -c` | ✅ 亲自 |
| `env_clear()` 生产缺失 vs dev 存在 | `sed -n '1074,1092p'` vs `sed -n '180,190p'` | ✅ 亲自 |
| landlock `FS_RO` 含 EXECUTE、`read_only=["/"]` | `sed -n '245,256p'`、`sed -n '80,92p'` | ✅ 亲自（并据注释**降档**子代理的 🔴 判定） |
| `edit.rs` 分块先 truncate 原文件 | `sed -n '135,170p'` | ✅ 亲自 |
| 压缩阈值 32k/2 轮、provider-aware 注入 | `sed -n '170,182p'`、`228,246p` | ✅ 亲自（并**更正自己的陈旧记忆**） |
| 工具注册 8 个 | `sed -n '238,256p' run_local.rs` | ✅ 亲自 |
| Observer 不依赖 agent-core | `grep agent-core observer/Cargo.toml api/Cargo.toml` → 无输出 | ✅ 亲自 |
| dry-run/backup/rollback/文件锁/MCP/hooks 零命中 | `grep -rni` 于 tools-builtin + agent-core | ✅ 亲自 |
| 状态机/失败分类/重试/活性检测锚点 | 子代理取证，我复核统计口径与关键行 | 部分 |
| 审批分类/非交互拒绝/沙箱机制锚点 | 子代理取证 | 部分 |
| Codex CLI 沙箱三层/bwrap/MITM 代理/unified_exec | [DeepWiki](https://deepwiki.com/halfprice06/codex/5.3-sandboxing-and-security)、[Platform Impl](https://codex.danielvaughan.com/2026/04/08/codex-sandbox-platform-implementation)、[Internals](https://codex.danielvaughan.com/2026/04/10/codex-cli-internals-queue-pair-guardian-sandbox) | 外部来源 |
| Claude Code 事件级持久化/改前快照/sidechain/54 工具 | [Agent Loop](https://code.claude.com/docs/en/agent-sdk/agent-loop)、[Subagents](https://code.claude.com/docs/en/sub-agents)、[arXiv:2604.14228](https://claude-wiki.com/dive-into-claude-code-the-design-space-of-today-s-and-future-ai-agent-systems-ar.html) | 外部来源 |

**零改动声明**：本轮全程只读。`git status --short` 中 `driver.py` 的 M 与 `run_rc52_matrix.py` 的 ?? 为执行窗口在制品，非本轮产生。
