# 项目间窗口协作模块 · 设计规格（FINAL v1）

> 状态：**FINAL v1（已融合四轮外部顾问评审 + 第五轮补丁清单查缺补漏订正：元宝第一轮 9 项 + 查缺补漏第二轮 H-1~L-2 + 第三轮 H-5~H-8/M-6~M-12/L-3~L-7 + 第四轮 ChatGPT 架构评审 F1–F5/P2-01~P2-04 + 第五轮 §7.5 身份防伪/§7.7 schema_version_rejected/§13 清单回填；已定稿，转执行窗口施工）**。
> 来源：2026-08-26 多轮讨论（用户扩展：多项目 × 多窗口 × 动态角色 + 项目级隔离 + 任务面板）+ 三轮外部顾问评审。
> 关联：`hearth-meta-capability-genes-next.md`（已 PAUSED）、`agent-team-orchestrator-design.md`（人肉同步，本模块消灭）、`crates/bridge/src/lib.rs`（模型级共识桥，见 §10）、`crates/codex-cli/src/repl.rs::restore_task_graph`（崩溃恢复，见 §8.4）。
> 执行门槛：**本模块不在地基稳前施工**。地基 = WS11–13 + WS1–6 在真机反复跑绿、基础功能流畅后，执行窗口才据本文件落地。逻辑：先能走，再考虑跑。

---

## 0. 一句话定位

> 旧内部桥（`crates/bridge`）= **多 LLM 群体智能**（智能层）。本模块 = **项目成员协调**（组织层）。两者**互补不合并**。

---

## 0.1 融合记录

- **DRAFT v1 → FUSION v1（元宝第一轮）**：9 项（粒度/契约校验器/IMPORT/manifest 冲突/协调 Agent/memory 叠加/restore 升级/rrule/继承认领）。
- **FUSION v1 → FUSION v2（查缺补漏第二轮）**：H-1~H-4、M-1~M-5、L-1~L-2（并发原子性/生命周期/消费游标/引用不可变/强制访问控制/格式统一/依赖流转/路径安全/restore 迁移/术语统一/周期实例 ID）。
- **FUSION v2 → FUSION v3（第三轮）**：闭合设计缝隙 + 工程落地约束 + 增强：
  - **H-5 request/response 配对**：事件 schema 加 `in_reply_to` + request 加 `await_timeout`；超时引擎写 `request_timeout` 事件（§7.4）。
  - **H-6 seq 跳号处理**：seq 全局单调允许空洞；消费者遇 `seq > 水位线+1` 阻塞等待，超 `gap_timeout`(60s) 发 `event_gap_alert`；预留 `noop` 事件填洞（§7.2/§7.3）。
  - **H-7 多所有者/协作者**：Task 加 `co_owners[]`；owner 迁移状态、co_owners 写 checkpoint/evidence；继承优先 co_owners 再同角色（§8.2/§8.7）。
  - **H-8 文件锁超时/死锁恢复**：锁带超时(5s)+指数退避(最大30s)；启动清理 stale 窗口锁；`lock_contention` 事件；含 kill -9 持锁者负面测试（§7.2/§5.2）。
  - **M-6** `project.toml` schema（§3）；**M-7** 任务 `priority`（§8.2）；**M-8** 结构化 `checkpoint` schema（§8.2）；**M-9** `starting→active/failed` 转换条件（§5.3）；**M-10** 动态订阅 `subscribe_request`/`unsubscribe_request`（§5.1/§7.4）；**M-11** 数据生命周期管理（§7.6）；**M-12** 网络分区/离线 `degraded` 模式（§5.3/§8.7）。
  - **L-3** 可观测性 `metrics.jsonl`（附录 D）；**L-4** `prev_hash` 精确计算范围（§7.2）；**L-5** `roles.toml` 版本化 + `role_updated` 事件（§6.1）；**L-6** Phase 0 Checklist 补负面测试（§13）；**L-7** 粒度枚举 `summary|file|full`（§4/§7.2）。
- **守门员补刀**：H-1~H-8、M-3、M-5、M-12 全部必须用**故意造失败**负面测试核验（并发双写/seq 跳号/stale 误认领/content_hash 篡改/依赖死锁/legacy 回滚/分区恢复/kill-9 持锁）；Windows 锁在进程句柄关闭时由内核释放（kill 亦如此），但超时获取仍必要以防网络锁/bug 永久阻塞。
- **FUSION v3 → FINAL v1（第四轮 · ChatGPT 架构评审，94/100 A-）**：有条件通过，5 项必改（F1 schema_version / F2 event_id=UUIDv7 / F3 seq allocator 责任 / F4 任务异常终态 / F5 owner 权限矩阵）+ 4 项 P2 增强（P2-01 EVENT_STREAM 逻辑/物理分层 / P2-02 knowledge_conflict 事件 / P2-03 IMPORT trust_level（Q3 已含，本轮回合确认）/ P2-04 manifest identity_epoch）。全部闭合后定稿 FINAL v1。详见 §18。

---

## 1. 背景与问题

### 1.1 现状（已源码核验）
- `agent-team-orchestrator-design.md:17`：`codex-rust "窗口" | 共享 memory 文件 | 否（约定） | 无运行时隔离、无消息队列；靠人肉同步`。
- 真实痛点：执行窗口完成审计窗口不知情；信息靠人工转发，不同步、易失真、重复踩坑。
- 已实装 `crates/bridge` 是**模型级**共识桥，非窗口/角色级同步。

### 1.2 目标
1. 同项目多窗口（角色不限于顶层/规划/施工/审计）**实时互通、掌握一手资料**。
2. 不同项目**默认隔离**（default-deny）。
3. 窗口能**自感知**身份与配合关系。
4. 任务**可追溯、不遗忘、可恢复、可继承**（§8）。
5. 核心 KPI = **配合效率 + 项目交付**。

### 1.3 非目标
- 不做多模型辩论/共识（那是 `crates/bridge`）。
- 不在地基稳前施工。

---

## 2. 核心原则

| # | 原则 | 含义 |
|---|------|------|
| P1 | **项目级隔离** | 每项目独立租户；窗口绝不读本项目根外；跨项目仅显式 IMPORT。 |
| P2 | **SSOT 单一事实源** | 一手资料只写一次到 SSOT。 |
| P3 | **作用域视图** | 每窗口只收"事件指针 + 相关 delta"，不拉全文。 |
| P4 | **窗口自感知** | 启动读 manifest，知身份/职能/配合。 |
| P5 | **角色动态化** | 角色由注册表声明，不硬编码。 |
| P6 | **事件驱动** | 同步靠指针 pub/sub，按需取。 |
| P7 | **任务持久化** | 任务进 SSOT Task Ledger，崩溃/归档/中断后可续接。 |
| P8 | **协作模式可配置 + 窗口平等** | `autonomy_level`（full/consult/report_only）；窗口平等无特权。 |
| P9 | **访问控制与一致性由实现层强制** | 读写/并发/引用完整性由实现层（锁/受限 API/哈希）强制，不靠自觉。 |

---

## 3. 架构总览与存储布局

```
项目 A（租户，.workbuddy/ 为天然隔离边界）
├─ project.toml        项目声明（project_id / name / default_import_policy / version）
├─ memory/             长期记忆（知识/经验/判例，沿用）
├─ roles.toml          角色注册表（结构化 + version 字段）
├─ events/
│   ├─ EVENT_LOG.jsonl         当前月事件（JSONL + 哈希链）
│   └─ EVENT_LOG.2026-07.jsonl 历史归档（§7.6）
├─ tasks/              T-042.toml …（每任务一文件 + 乐观锁）
├─ registry/
│   └─ windows/<id>/   manifest.toml + heartbeat + last_processed_seq
├─ archive/            events/ registry/ tasks/ 的冷归档（§7.6）
└─ metrics.jsonl       运行时指标（附录 D，L-3）

窗口：┌顶层┐┌施工┐┌审计┐  各持 manifest，依 project.toml 限定可见根。
```

存储五类：`memory/`（长期记忆）+ `roles.toml`（角色）+ `events/`（EVENT_LOG）+ `tasks/`（Task Ledger）+ `registry/`（窗口清册），新增层经**受限 API 强制**读写（P9）。

### 3.1 `project.toml` Schema（M-6）
```toml
[project]
project_id            = "codex-rust-v1.0-final"
name                  = "Codex Rust Final"
created_at            = "2026-08-20T10:00:00+08:00"
default_import_policy = "deny"     # deny | allow（P1 default-deny）
version               = 1           # 项目级乐观锁
```

---

## 4. 关键概念定义

| 概念 | 定义 |
|------|------|
| **Project（租户）** | 代码/工作根；`.workbuddy/` 为隔离边界。`project.toml` 声明 `project_id`。 |
| **Window（窗口）** | 角色化会话实例；`window_id`（如 `impl-01`）唯一标识。 |
| **Role（角色）** | 职责抽象，由 `roles.toml` 声明；持契约。 |
| **Role Contract** | `{responsibilities[], publishes[], subscribes[{event_type, requested_granularity}], hands_off_to[], autonomy_level}`。 |
| **Manifest** | `{project_id, window_id, role, functions[], subscribes[{event_type, requested_granularity}], coordinates_with[], version, status, last_heartbeat, autonomy_level, identity_epoch}`。 |
| **SSOT** | `docs/` + `.workbuddy/{memory,events,tasks,registry,archive}`。 |
| **Event Pointer** | `events/EVENT_LOG.jsonl` 一行：`{schema_version, seq, event_id, actor(window_id), role, type, target_ref, content_hash?, hands_off_to?, in_reply_to?, await_timeout?, summary, status, prev_hash, provided_granularity}`。**`schema_version`（F1）** = 事件结构版本（如 `"1.0"`），保证长期演化兼容；**`event_id` = UUIDv7（F2）** 时间有序 + 分布式唯一，解决并发 ID 冲突/排序/调试困难。 |
| **Granularity 枚举（L-7）** | `"summary"`（仅摘要+状态）/ `"file"`（文件级引用 target_ref 完整文件）/ `"full"`（全文+相关依赖）。`requested_granularity`=消费方期望，`provided_granularity`=生产方提供。 |
| **Delta** | 订阅者按 `requested_granularity` 从 `target_ref` 取的局部内容，非全文。 |
| **Handoff Contract** | 角色契约 `hands_off_to`（同时出现在事件 schema 便于路由）。 |
| **IMPORT** | 显式跨项目导入，带 provenance + `[imported]`。 |
| **Task** | `{id, instance_of, title, status, owner_window, co_owners[], project_id, cadence, rrule, priority, depends_on, evidence_ref, checkpoint, version, identity_epoch?, …}`。**`status` 全集（F4）**：`pending | in_progress | paused | done | blocked | failed | cancelled | expired`（后三者为异常终态，避免 blocked 被滥用）。 |
| **Task Ledger** | `tasks/` 每任务一文件，持久化。 |
| **Checkpoint（M-8）** | 结构化：`{progress, context_refs[], next_action, updated_at, by_window}`。 |
| **消费水位线** | 每窗口 `last_processed_seq`。 |
| **心跳** | `registry/windows/<id>/heartbeat`，证明在线。 |

---

## 5. 窗口自感知与生命周期

### 5.1 Manifest Schema（TOML 草案）
```toml
# .workbuddy/registry/windows/<window_id>/manifest.toml
project_id     = "codex-rust-v1.0-final"
window_id      = "impl-01"
role           = "施工"
functions      = ["照任务书编码", "跑门禁", "交证据"]
version        = 3
status         = "active"        # starting|active|stale|degraded|failed|archived（H-2/M-9/M-12）
last_heartbeat = "2026-08-26T04:00:00+08:00"
autonomy_level = "consult"
identity_epoch = "2026-08-26T03:00:00+08:00"   # P2-04：窗口身份纪元，防止同 window_id 被复用作不同角色导致历史语义污染
subscribes = [
  { event_type = "顶层:taskbook", requested_granularity = "file" },
  { event_type = "规划:plan",     requested_granularity = "file" },
]
coordinates_with = ["顶层", "审计"]
created_at   = "2026-08-26T03:00:00+08:00"
```

### 5.2 启动序列（Boot Sequence）
1. 读 `manifest.toml` → 解析身份字段。
2. 依 `project_id` 定位 SSOT 根，路径经 `canonicalize` 校验前缀（§7.5）。
3. 校验 `version` 与 registry 一致（§5.4）；订阅 EVENT_LOG；读 `last_processed_seq`（§7.3）；读 `tasks/` 认领任务（§8.7）。
4. **自感知完成 → 置 `active`**（M-9）。若第 3 步校验失败（manifest 与 `roles.toml` 不一致 / `project_id` 不匹配 / 路径校验失败）→ 置 **`failed`** 并广播 `window_boot_failed` 事件；`failed` 窗口不分配任务，由顶层或用户决定重试/归档。
5. **清理 stale 窗口持有的锁**（H-8）：扫描 registry 中 `status=stale` 的窗口，强制释放其可能持有的文件锁。

### 5.3 生命周期状态机（H-2 + M-9 + M-12）
- 状态全集：`starting → active → (stale ⇄ active) → archived`；附加 `failed`（启动失败）、`degraded`（离线/分区）。
- **心跳**：运行期每 30s 更新 `heartbeat`（临时文件 + `rename` 原子写）。
- **在线判定**：仅 `status==active` **且** `now - last_heartbeat ≤ 2 分钟` 视为在线；超时自动 `stale` + 广播 `window_stale`。
- **离线/分区（M-12）**：窗口检测到 SSOT 不可写（心跳/EVENT_LOG append 失败）→ 进入 **`degraded`** → 停止状态迁移、暂停认领新任务、本地缓存待恢复后重放；恢复后校验 `last_processed_seq` 连续性，补齐缺失事件再回 `active`。
- 正常退出 → `archived`；崩溃靠心跳超时转 `stale`（无主动注销也能探测）。

### 5.4 权威来源与冲突解决（Q4 + H-1）
- registry 为 SSOT 中普通文件集合（非单点）。**乐观锁 + version**：启动不一致以 registry 为准。
- **原子更新**：registry / task / manifest 写回均「读 → 改 → 临时文件 → `rename` 原子替换」。

---

## 6. 角色系统（动态注册表）

### 6.1 角色注册表（`roles.toml`，M-2 + L-5）
```toml
version = 2                        # roles.toml 自带版本，独立于 manifest.version（L-5）

[role.顶层]
responsibilities = ["全局透视", "架构", "评审", "验收", "任务书"]
publishes        = ["taskbook", "architecture"]
subscribes       = [
  { event_type = "施工:done",    requested_granularity = "file" },
  { event_type = "审计:report",   requested_granularity = "full" },
]
hands_off_to     = ["规划", "施工"]
autonomy_level   = "full"
# …施工/审计/安全 同 FUSION v2，略
```
- 新增/修改 `roles.toml` → 写 **`role_updated`** 事件到 EVENT_LOG；各窗口下次心跳或收事件后重新校验自身契约（L-5）。
- `顶层/规划/施工/审计` 为种子角色；其余按需声明，不碰代码。

### 6.2 契约校验器（Q2 + P9）
- Phase 1 启动期校验 manifest 的 `functions`/`subscribes` 是否与 `roles.toml` 一致；**不一致 fail-closed 拒绝启动**。Phase 0 仅警告。

---

## 7. 同步机制（指针 + Delta，P9 强制）

### 7.1 存储分层（Q6）
- `memory/` 继续长期记忆；新增 `events/`（EVENT_LOG）、`tasks/`（Task Ledger）、`registry/`（窗口清册）并列语义分离，由受限 API 强制读写。`target_ref` 可指向 `memory/` 形成引用。

### 7.2 EVENT_LOG（JSONL + 追加原子 + 哈希链 + 锁，H-1/H-4/H-8/M-4/L-4）
```json
{"schema_version":"1.0","seq":4,"event_id":"0198f4a2-1b3c-7d2e-8f11-2b4c6d8e0a10","actor":"audit-07","role":"审计","type":"audit_report",
 "target_ref":"audit/ws11-report.md","content_hash":"sha256:9f86d0...","hands_off_to":["顶层"],
 "in_reply_to":null,"await_timeout":null,
 "summary":"wiring 断言全过，无T11-T14","status":"done","prev_hash":"sha256:aa11...",
 "provided_granularity":"full"}
```
- **`schema_version`（F1）**：每行携带，标识事件结构版本；Phase 1 引入 **schema migration layer**，将旧版事件转换为内部统一模型，保证历史事件长期可读。
- **`event_id` = UUIDv7（F2）**：时间有序（毫秒时间戳前缀）+ 分布式唯一，天然适合日志系统，杜绝多写者 ID 冲突与排序歧义。
- **seq 分配责任（F3）**：seq 由**唯一责任方**生成 —— Phase 0（约定层）由单协调进程/人工分配；**Phase 1 由原子的 sequence allocator**（在文件锁内 `max(seq)+1`）生成，确保全局单调"允许空洞"语义唯一可落地。
- **原子追加（H-1/H-8）**：`O_APPEND` 打开 + **文件锁保证单写者**；锁带**超时(5s)**，超时放弃并**指数退避重试（最大30s）**；竞争写 `lock_contention` 事件。Windows 进程句柄关闭时内核自动释放锁（kill 亦释放），超时获取仍必要以防网络锁/bug 永久阻塞。
- **哈希链（M-4/L-4）**：`prev_hash = sha256(本行 JSON 排除 prev_hash 字段后的 UTF-8 字节)`；每条含上一条哈希，形成可复现防篡改链；Phase 1 校验，Phase 0 可仅校验和文件。
- **引用不可变（H-4）**：`content_hash = sha256(target_ref 指向文件内容)`，生产者算、消费者校验；重要产出 `target_ref` 指向不可变版本（版本化名/git hash）。

### 7.3 事件消费游标与幂等（H-3）+ seq 跳号（H-6）
- 每窗口 `last_processed_seq` 水位线，仅处理 `seq > 水位线`，处理后原子更新。seq 由唯一责任方分配（F3，§7.2）。
- **seq 跳号（H-6）**：seq 全局单调、**允许空洞**；消费者遇 `seq = 水位线 + n (n>1)` **阻塞等待**直到洞被填补，或超 `gap_timeout`(60s) 发 `event_gap_alert` 告警；预留 **`noop`** 事件类型显式填已知洞。

### 7.4 请求/响应与动态订阅（H-5 / M-10）
- 事件 schema 加 **`in_reply_to: Option<EventId>`** 指向被回复事件；request 事件加 **`await_timeout: u64`**（秒），超时引擎自动写 **`request_timeout`** 事件（H-5）。配对字段使并发多 request、重播因果链、超时重试成为可能。
- **动态订阅（M-10）**：`subscribe_request` / `unsubscribe_request` 事件动态增删订阅；目标收到后更新 registry 中 manifest（`version`+乐观锁）并写 `subscription_changed` 事件。
- request/response 均落 EVENT_LOG，受 §7.3 游标与幂等约束。

### 7.5 路径安全与强制访问控制（M-1/M-4，P9）
- 窗口经受限文件 API 读写 SSOT；每次路径 `canonicalize` + 前缀校验在项目根内，**拒绝符号链接逃逸与越界**；项目内访问控制实现层强制执行，窗口无法绕过订阅范围。
- **身份防伪（防 actor / owner 伪造，第五轮订正）**：受限 API 写入 EVENT_LOG 的 `actor`、写入 Task 的 `owner_window` **不接受调用方显式传入**，由 API 内部从当前进程已加载的本机 `manifest.window_id` **自动填充**；跨窗口写入（如顶层代人 `assign`、继承认领）必须走专门的 `assign` 事件路径，**不得复用普通写接口伪造 actor**。此机制是 F5「伪造 actor 越权」负面测试的可实现前提（§8.7.1 权限矩阵依赖之）。

### 7.6 数据生命周期管理（M-11）
| 数据类型 | 保留策略 |
|----------|----------|
| EVENT_LOG | 按月切分（`EVENT_LOG.YYYY-MM.jsonl`），归档到 `.workbuddy/archive/events/`；近 12 个月在线，更早压缩。 |
| archived 窗口 registry | 保留 30 天 → 迁 `.workbuddy/archive/registry/`。 |
| `done` 任务 | 保留，90 天后标 `archived` → 迁 `.workbuddy/archive/tasks/`。 |
| `failed`/`cancelled` 任务 | 永久保留（审计需要），可压缩。 |

**逻辑/物理抽象（P2-01）**：对外暴露统一逻辑概念 **`EVENT_STREAM`**；其物理实现为 `events/` 目录下按自然月切片的多文件（`EVENT_LOG.YYYY-MM.jsonl`）。未来切片策略变更（如按大小/按周）只改物理层，读写代码不感知，避免重构。消费游标 `last_processed_seq` 跨切片单调递增。

### 7.7 事件类型目录（含 P2-02 knowledge_conflict）
事件 `type` 全集（便于 schema migration 与契约校验）：
- **生命周期**：`window_boot_failed`、`window_stale`、`role_updated`、`identity_epoch_mismatch`（P2-04：同 `window_id` 复用不同 `identity_epoch` 时告警，设计文档 §5.1/§7.7 引用）。
- **任务**：`task_done`、`task_unblocked`、`task_orphaned`、`task_claim_conflict`、`assign`、`request_timeout`。
- **同步/协调**：`subscription_changed`（M-10）、`lock_contention`（H-8）、`event_gap_alert`（H-6）、`noop`（H-6 填洞）。
- **请求/响应**：`request` + 对应 `response`（`in_reply_to` 配对，H-5）。
- **知识冲突（P2-02）**：`knowledge_conflict` —— 当 IMPORT 或跨角色写入与 SSOT 既有知识**正确性**冲突（非文件一致性，文件一致性由最后写入+prev_hash 裁决）时，写入此事件提请人工/审计介入，避免静默覆盖错误知识。
- **schema 拒绝（第五轮订正）**：`schema_version_rejected` —— 解析器读到**无法识别的 `schema_version`** 历史事件时写入；字段至少含 `event_id` / `schema_version`(实际值) / `target_seq`(原始行 seq) / `detected_by`。语义**独立于** `event_gap_alert`（seq 跳号超时，H-6）与 `window_boot_failed`（启动期 manifest/角色校验失败，§5.2），避免排障误判为"seq 缺口"或"某窗口启动失败"。F1 负面测试引用本类型。

---

## 8. 任务面板（Task Ledger）

### 8.1 放进本模块的理由
任务状态持久化在 SSOT，本模块已拥有 SSOT + 隔离 + 事件指针 + 受限 API，直接复用。结论：SSOT 一等公民 + 边界清晰的子组件（任务引擎），不另立模块。

### 8.2 数据模型（每任务一文件，M-2/H-1/M-7/M-8/H-7）
```toml
# .workbuddy/tasks/T-042.toml
id            = "T-042"
instance_of   = ""                 # recurring 填模板 ID，如 "T-040"
title         = "落地 WS11 会话韧性"
status        = "done"             # pending|in_progress|blocked|done|paused|failed|cancelled|expired（F4 异常终态）
owner_window  = "impl-01"          # window_id（L-1）
co_owners     = ["impl-02"]         # 协作者（H-7）
project_id    = "codex-rust-v1.0-final"
cadence       = "one-shot"         # one-shot|recurring|long_running
rrule         = ""
priority      = "p1"               # p0|p1|p2|p3（M-7）
depends_on    = ["T-040"]
evidence_ref  = "crates/codex-cli/src/repl.rs"
checkpoint    = { progress = "repl.rs 已改", context_refs = ["crates/codex-cli/src/repl.rs:352-357"],
                  next_action = "VM 跑 wiring 断言", updated_at = "2026-08-26T02:00:00+08:00",
                  by_window = "impl-01" }   # 结构化（M-8）
version       = 5                  # 任务文件内乐观锁（H-1）
created_at    = "2026-08-25T10:00:00+08:00"
completed_at  = "2026-08-26T02:00:00+08:00"
```
- 每任务一文件 + 独立乐观锁 + 原子 rename 写回（H-1）。

### 8.3 三类需求覆盖
- 短/周期/长程不遗漏：全登记；周期 `cadence=recurring`+`rrule`（§8.6）。
- 窗口卡死/归档 → 继承续接（§8.7）。
- API 欠费/中断/暂停 → `paused`+`checkpoint` 续跑；状态变更发 EVENT_LOG。

### 8.4 与现有代码关系（Q7 + M-5）
- `restore_task_graph` 统一升级：主后端改 SSOT Task Ledger；**保留 legacy fallback + 配置开关 `task_store_backend = "ledger" | "legacy"`**；保留接口不变。
- 迁移：Phase 1 引入 ledger+开关（默认 legacy→回归）→ 全绿切 ledger → 再跑完整回归（含故意失败）→ 通过再移除 legacy。

### 8.5 状态机 + 依赖自动流转（M-3）
- `pending → in_progress → (paused ⇄ in_progress) → done`；`in_progress/blocked → done`。
- **异常终态（F4）**：测试/执行失败 → `failed`；人工取消 → `cancelled`；生命周期到期（如 recurring 模板被弃用） → `expired`。三者为吸收性终态，`failed`/`cancelled`/`expired` 不再参与认领与自动流转，避免 `blocked` 被滥用承载本不应续跑的任务。`blocked` 仍需依赖满足或人工解除。
- **依赖自动流转**：`depends_on` 全 `done` → 自动 `blocked → pending` + `task_unblocked` 事件。

### 8.6 周期任务（Q8 + L-2）
- rrule（RFC 5545 子集，rrule-rs）。**实例 ID `模板#日期`**（如 `T-040#2026-08-26`），实例增 `instance_of` 指向模板。
- 实例生成前检查资源可用性（active+心跳有效窗口）；失败标 `missed`+原因+告警；补产：人工/顶层置 `pending` 重试。

### 8.7 继承认领与孤儿（Q9 + H-2 + M-3 + H-7 + M-12）

#### 8.7.1 任务权限矩阵（F5）
避免实现阶段权限漂移，明确三方对任务的权限：

| 权限 | owner | co_owner | observer（审计等只读角色） |
|------|:-----:|:--------:|:--------------------------:|
| 读取 | √ | √ | √ |
| 写 checkpoint | √ | √ | × |
| 追加 evidence_ref | √ | √ | × |
| 状态迁移（done/blocked/paused/failed…） | √ | × | × |

owner 独有状态迁移权；co_owners 可写进度与证据；observer 仅只读用于审计。继承后权限随新 owner 自然继承。

- **默认按角色自动认领**：owner 窗口 `archived`/`stale` 时，其 `in_progress` 任务按角色分配给下一 `active` 且心跳有效窗口。
- **多所有者优先（H-7）**：owner 崩溃 → **优先 co_owners 认领**，其次同角色自动认领。co_owners 可写 checkpoint/evidence_ref，owner 独有状态迁移权。
- **冲突**：多同角色在线 → `task_claim_conflict` → 顶层/人工裁决。
- **手动覆写**：顶层发 `assign` 事件。
- **孤儿（M-3）**：无同角色在线 → 标 `blocked` + `task_orphaned` 告警。
- **离线（M-12）**：`degraded` 窗口不被认领源；恢复后先补齐事件再参与。
- 全过程 EVENT_LOG 留痕，审计可见。

---

## 9. 协调与配合

- 交接：事件带 `hands_off_to`，接收者自动拾取。
- 冲突：同 `target_ref` 并发写 → 以 SSOT 最后写入 + `seq`/`prev_hash` 为准；争议顶层仲裁。
- 协调 Agent（Q5）：Phase 0/1 纯 pub/sub；Phase 2 仅 >10 窗口时加「仲裁窗口」（普通窗口、无特权）。

---

## 10. 与旧内部桥（`crates/bridge`）的边界

| 维度 | `crates/bridge`（已实装） | 本模块（待施工） |
|------|--------------------------|------------------|
| 层级 | 智能层 | 组织层 |
| 主体 | 多个 LLM 模型 | 多个窗口/角色 |
| 目的 | 模型辩论/共识 | 项目成员协作 |
| 机制 | `BridgeSession`+`BridgeStrategy` | SSOT+角色注册表+EVENT_LOG+TASKS+manifest |
| KPI | 共识质量 | 配合效率+项目交付 |

不合并语义；窗口推理时仍可调用 `bridge`。

---

## 11. 跨项目隔离与 IMPORT 信任模型（Q3）

### 11.1 隔离基础
项目根 `.workbuddy/` 为租户边界；跨项目读取默认拒绝（default-deny）；路径经 §7.5 规范化。

### 11.2 IMPORT provenance
```toml
[import]
source_project_id = "other-project"
source_role       = "施工"
source_timestamp  = "2026-08-20T..."
import_reason     = "复用 WS11 崩溃恢复实现思路"
trust_level       = "verified"      # internal|verified|external|unknown（P2-03）
```
- artifact 文件名/metadata 加 `[imported]` 标签；IMPORT 默认关闭，需显式开启。
- **P2-03 已满足**：IMPORT 信任等级 `trust_level` 已随 Q3 provenance 落地，取值扩展为 `internal|verified|external|unknown` 四类，导入者在 provenance 中标注。

---

## 12. 与协作模式谱系的关系

默认「AI 从属人类」；`autonomy_level` + 推送开关（Phase 2）承载平权 / 监护托管（G0 放心走开）/ 认知外延，不绑定任一哲学。

---

## 13. 实现路线（分阶段）

### Phase 0 · 约定层（纯文件）
- 新增 `project.toml`、`registry/windows/<id>/`（manifest+heartbeat+last_processed_seq）、`events/EVENT_LOG.jsonl`、`roles.toml`、`tasks/T-xxx.toml` 模板。
- 契约校验器 Phase 0 仅警告级。
- **Phase 0 验收 Checklist**：
  - [ ] 创建 `project.toml` 声明 `project_id`
  - [ ] 两窗口 manifest 验证自感知（含 starting→active/failed 转换）
  - [ ] 模拟 EVENT_LOG（JSONL+prev_hash）写入与订阅过滤（含 requested_granularity）
  - [ ] Task Ledger 状态迁移（pending→in_progress→done / paused→续跑）
  - [ ] 崩溃（停心跳）后继承任务（仅认 active+心跳有效；co_owners 优先）
  - [ ] 重复 seq / 篡改 content_hash 被拒
  - [ ] **（L-6）路径逃逸（`../../etc/passwd`、symlink）被受限 API 拒绝**
  - [ ] **（L-6）文件锁持有者 kill -9 后清理与恢复（H-8）**
  - [ ] **（L-6）seq 跳号（E001、E003 缺 E002）验证消费者阻塞（H-6）**
  - [ ] **（L-6）动态订阅/退订后过滤正确性（M-10）**
  - [ ] **（L-6）`default_import_policy` allow/deny 的 IMPORT 差异（M-6）**
  - [ ] **（M-12）模拟 SSOT 不可达 → degraded → 恢复重放**
  - [ ] **（F1）未知 `schema_version` 事件被拒并写 `schema_version_rejected`，不静默接受/崩溃**
  - [ ] **（F2）并发写者 UUIDv7 零冲突 + 重复 `event_id` 去重生效**
  - [ ] **（F3）双写者绕过 allocator → 锁竞争无重复 seq + kill-9 持锁恢复后 seq 连续**
  - [ ] **（F4）终态任务（failed/cancelled/expired）不可再流转、不被误认领**
  - [ ] **（F5）observer/co_owner 越权写被拒 + 伪造 actor 被拒（§7.5 身份防伪）**
  - [ ] **（P2-01）物理切片损坏/改名不静默读空**
  - [ ] **（P2-02）知识正确性冲突不被文件一致性规则静默吞（`knowledge_conflict` 写入）**

### Phase 1 · 工具化（Rust）
- 新 crate（不污染 `bridge`）：`crates/collab/` 或 `crates/project-sync/`。
- 职责：manifest 校验（fail-closed）、EVENT_LOG（JSONL+O_APPEND+跨平台锁+哈希链+gap 处理）、订阅过滤、消费水位线、registry（乐观锁+原子 rename）、受限文件 API（canonicalize+前缀）、Task Ledger（每文件+乐观锁+原子写回+状态机+检查点+rrule+实例 ID+依赖流转+孤儿告警）、心跳探测、动态订阅、`restore_task_graph` ledger 主后端+legacy fallback+开关。

### Phase 2 · 可选
- 跨项目 IMPORT、推送式 EVENT_LOG、仲裁窗口。

### 量化"地基稳"标准
- WS11–13 + WS1–6 连续 7 天零 panic；门禁 100% pass（含故意失败）；断网/欠费恢复 ≥95%。

---

## 14. 开放问题决议（第一轮 · 元宝 9 项，已关闭）
| # | 决议 |
|---|------|
| Q1 | 粒度下放消费方 `requested_granularity`（§5.1/§7.2） |
| Q2 | 启动期契约校验器 fail-closed（§6.2） |
| Q3 | IMPORT provenance + `[imported]`（§11.2） |
| Q4 | 乐观锁+version（§5.4） |
| Q5 | 协调窗口仅 Phase 2 >10 窗口（§9） |
| Q6 | memory 之上叠加 events/tasks/registry（§7.1） |
| Q7 | 升级 restore_task_graph 后端（§8.4） |
| Q8 | rrule + missed + 补产（§8.6） |
| Q9 | 按角色自动认领 + 冲突仲裁 + 手动 assign（§8.7） |

## 15. 工程完备性决议（第二轮 · H-1~L-2，已关闭）
| 编号 | 决议 |
|------|------|
| H-1 | EVENT_LOG=JSONL+锁+哈希链；Task 每文件+乐观锁+rename；registry 临时文件 rename（§7.2/§8.2/§5.4） |
| H-2 | registry status+heartbeat；仅 active≤2min 在线（§5.3） |
| H-3 | last_processed_seq 水位线 + 幂等（§7.3） |
| H-4 | content_hash + 不可变 target_ref（§7.2） |
| M-1 | 受限文件 API 强制访问控制（§7.5） |
| M-2 | roles.toml/JSONL/每任务一文件/window_id 唯一（§3/§6.1/§7.2） |
| M-3 | 依赖自动流转 + 孤儿告警（§8.5/§8.7） |
| M-4 | canonicalize 防逃逸 + 哈希链（§7.2/§7.5） |
| M-5 | restore_task_graph 保留 legacy fallback + 开关（§8.4） |
| L-1 | owner 用 window_id；事件加 hands_off_to；粒度分 provided/requested（§4/§7.2） |
| L-2 | 周期实例 ID 模板#日期 + instance_of（§8.6） |

## 16. 第三轮决议（H-5~H-8 / M-6~M-12 / L-3~L-7，已关闭）
| 编号 | 决议（写入位置） |
|------|------|
| H-5 | request/response 配对：`in_reply_to` + `await_timeout` + `request_timeout` 事件（§7.4） |
| H-6 | seq 单调允许空洞；消费者遇跳号阻塞，`gap_timeout`(60s)→`event_gap_alert`；`noop` 填洞（§7.3） |
| H-7 | 任务 `co_owners[]`；owner 迁移状态、co_owners 写 checkpoint/evidence；继承优先 co_owners（§8.2/§8.7） |
| H-8 | 锁超时(5s)+指数退避(最大30s)；启动清理 stale 锁；`lock_contention` 事件；含 kill -9 负面测试（§7.2/§5.2） |
| M-6 | `project.toml` schema（§3.1） |
| M-7 | 任务 `priority`（§8.2） |
| M-8 | 结构化 `checkpoint` schema（§8.2） |
| M-9 | `starting→active`（自感知完成）/ `failed`（校验失败 + `window_boot_failed`）（§5.3） |
| M-10 | 动态订阅 `subscribe_request`/`unsubscribe_request` + `subscription_changed`（§7.4） |
| M-11 | 数据生命周期管理（按月切分/归档/保留策略）（§7.6） |
| M-12 | 网络分区/离线 `degraded` 模式（§5.3/§8.7） |
| L-3 | 可观测性 `metrics.jsonl`（附录 D） |
| L-4 | `prev_hash = sha256(本行 JSON 排除 prev_hash 字段后字节)`（§7.2） |
| L-5 | `roles.toml` 自带 `version` + `role_updated` 事件（§6.1） |
| L-6 | Phase 0 Checklist 补 5 项负面测试（§13） |
| L-7 | 粒度枚举 `summary|file|full` 及语义（§4/§7.2） |

---

## 17. 交付与治理（Handoff）

1. DRAFT v1 → 元宝评审（9 项）→ FUSION v1。
2. FUSION v1 → 查缺补漏第二轮（H-1~L-2）→ FUSION v2。
3. FUSION v2 → 第三轮（H-5~H-8/M-6~M-12/L-3~L-7）→ FUSION v3。
4. FUSION v3 → **第四轮 ChatGPT 架构评审（94/100 A-，有条件通过）→ 闭合 F1–F5 + P2-01~P2-04 → 定稿 FINAL v1（本文件）**。
5. FINAL v1 + 地基稳信号 → 执行窗口施工（Phase 0 起，按 §18 补丁清单 + §13 路线/Checklist）。
6. **守门员门禁**：H-1~H-8、M-3、M-5、M-12 等全部须"故意造失败"核验（并发双写/seq 跳号/stale 误认领/content_hash 篡改/依赖死锁/legacy 回滚/分区恢复/kill-9 持锁）；F1–F5 亦须配套测试（schema_version 兼容、UUIDv7 唯一、seq allocator 单责任、异常终态不可再流转、权限矩阵越权拒绝）。

---

## 18. 第四轮架构评审与 FINAL v1 进入条件

**评审方**：ChatGPT Architecture Review（2026-08-26）。**结论**：FUSION v3 已达工程可施工状态，**94/100，A-，有条件通过**。无阻断性架构错误，主体设计闭合。

### 18.1 FINAL v1 必改清单（F1–F5，已完成）
| 编号 | 修改项 | 优先级 | 落点 |
|------|--------|:----:|------|
| F1 | 事件 `schema_version` + migration layer | P0 | §4/§7.2 |
| F2 | `event_id` = UUIDv7 | P0 | §4/§7.2 |
| F3 | `seq` 分配责任（Phase0 单协调 / Phase1 原子 allocator） | P1 | §7.2/§7.3 |
| F4 | Task 异常终态 `failed`/`cancelled`/`expired` | P1 | §4/§8.2/§8.5 |
| F5 | owner 权限矩阵（owner/co_owner/observer） | P1 | §8.7.1 |

### 18.2 增强项（P2，FINAL 后可择机执行，本轮已预置）
| 编号 | 增强 | 状态 |
|------|------|------|
| P2-01 | `EVENT_STREAM` 逻辑/物理分层（按月切片物理层） | 已写入 §7.6 |
| P2-02 | `knowledge_conflict` 事件（知识正确性冲突提请审计） | 已写入 §7.7 |
| P2-03 | IMPORT `trust_level` 四类（internal/verified/external/unknown） | 已并入 §11.2（Q3 provenance） |
| P2-04 | manifest `identity_epoch`（防同 window_id 语义污染） | 已写入 §5.1 |

### 18.3 FINAL v1 进入条件（评审方裁定，现已满足）
- [x] F1–F5 五项补丁已写入正文；
- [x] 无 P0 级未决项；
- [x] 三轮 + 本轮累计 **9 + 11 + 16 + 9 = 45 项**开放问题全部闭合；
- [ ] **剩余执行前置条件（非设计问题）**：地基稳信号（WS11–13 + WS1–6 连续 7 天零 panic / 门禁 100% pass / 断网恢复 ≥95%）——见 §13，待真机达成。

### 18.4 评审方施工警告（采纳）
> 当前最大风险已不是设计不足，而是施工过程中把一个清晰系统重新复杂化。

据此，§13 施工顺序保持原设计（文件约定层 → manifest → EVENT_STREAM → Task Ledger → registry → 恢复机制 → 负面测试），**不继续增加大型能力**；任何新增须先回顶层出任务书，不擅自扩 scope。

### 18.5 第五轮查缺补漏订正（2026-08-26，补丁清单核查）
核查对象：`pwc-final-v1-patch-spec.md`。结论：F1–F5、P2-01/02/04 与设计文档一致；发现 2 处真设计空白（已闭合）+ 7 处清单体例/覆盖问题（已订正）。

- **问题 1（设计空白，已闭）**：F1「未知 schema_version 拒绝」原含糊复用 `event_gap_alert`/`window_boot_failed`（语义不符）。§7.7 新增专属事件 `schema_version_rejected`（含 event_id/实际 schema_version/target_seq/detected_by），F1 负面测试引用之。
- **问题 2（设计空白，已闭）**：F5「伪造 actor」负面测试引用的 §7.5 原只讲路径安全，不含身份防伪。§7.5 新增**身份防伪机制**：`actor`/`owner_window` 由 API 从本机 `manifest.window_id` 自动填充、拒显式传入；跨窗口写走 `assign` 事件。F5 负面测试引用之。
- **清单体例订正（问题 3–9）**：汇总门禁表补入 P2-04 并给 P2-03 补负面用例（非法 trust_level 被拒）；P2-04 补「验收·正面」；F4 负面测试消除「上游/下游」歧义；补丁执行顺序与 §18.4 模块施工顺序的关系说清；F3 kill-9 测试明确复用 H-8 同一把锁；P2-03 标注「仅回归验证、无新增开发」；§13 Phase 0 Checklist 已回填本轮负面测试（单一权威清单）。

> 经本轮订正，FINAL v1 设计文档与补丁清单在负面测试层面已无"空话引用"，**可移交执行窗口施工**（仍 gated by 地基稳信号，§18.3）。

---

## 附录 A · 术语速查
- SSOT / manifest / delta / handoff / IMPORT / default-deny / Task Ledger / checkpoint / rrule(`instance_of`) / autonomy_level / last_processed_seq / content_hash / prev_hash / co_owners / degraded / gap_timeout / noop / lock_contention

## 附录 B · 与本仓库其它文档关系
- `hearth-meta-capability-genes-next.md`：已 PAUSED，本模块不依赖。
- `agent-team-orchestrator-design.md`：本模块消灭"人肉同步"。
- `crates/bridge/src/lib.rs`：模型级共识桥，互补不合并（§10）。
- `crates/codex-cli/src/repl.rs::restore_task_graph`：统一升级 + legacy fallback（§8.4）。
- `docs/hearth-v02-taskbook.md`：地基来源，执行门槛依据（§13）。

## 附录 C · 工程落地约束与增强索引（第三轮）
- M-6 `project.toml` → §3.1；M-7 `priority` → §8.2；M-8 结构化 `checkpoint` → §8.2；M-9 `starting→active/failed` → §5.3；M-10 动态订阅 → §7.4；M-11 数据生命周期 → §7.6；M-12 `degraded` → §5.3/§8.7。
- L-3 可观测性；L-4 prev_hash 范围；L-5 roles.toml 版本化；L-6 Checklist 补项；L-7 粒度枚举——分别见 §13 / §7.2 / §6.1 / §13 / §4。

## 附录 D · 可观测性（L-3，Phase 2 可选）
- `.workbuddy/metrics.jsonl` 暴露：活跃窗口数、各状态任务数、EVENT_LOG 写入 QPS、平均认领延迟、心跳超时率、锁竞争次数。
- 可选接入 Prometheus 格式外部监控。
