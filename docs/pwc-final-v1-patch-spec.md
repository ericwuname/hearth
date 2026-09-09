# 项目间窗口协作模块 · FUSION v3 → FINAL v1 补丁规格清单（Patch Specification）

> **目的**：独立可执行的补丁清单。施工窗口据此逐条落地，**无需重读整份设计文档**（`project-window-sync-design.md` 仅作引用）。
> **来源**：第四轮 ChatGPT Architecture Review（2026-08-26，94/100 A-，有条件通过）的必改项 F1–F5 + 增强项 P2-01~P2-04。
> **执行门禁**：本清单所有项**仅在「地基稳」信号达成后**执行（WS11–13 + WS1–6 连续 7 天零 panic / 门禁 100% pass / 断网恢复 ≥95%）。此前不施工。
> **守门员铁律**：每条补丁均须配「故意造失败」核验（见每项的「验收·负面」），断言不能失败 = 断言不存在。

---

## 执行顺序（推荐）

1. F2（UUIDv7 事件 ID）→ 2. F1（schema_version）→ 3. F3（seq allocator）→ 4. F4（异常终态）→ 5. F5（权限矩阵）→ 6. P2-04（identity_epoch）→ 7. P2-01（EVENT_STREAM）→ 8. P2-02（knowledge_conflict）→ 9. P2-03（trust_level，并入既有 IMPORT，仅扩枚举）。

前置依赖：Phase 0 约定层（project.toml / manifest / EVENT_LOG / roles.toml / Task Ledger 模板）已就位；Phase 1 工具化 crate 已建（不污染 `crates/bridge`）。

> 本节顺序**仅指补丁落地的先后关系**，不改变设计文档 §18.4 规定的模块施工阶段顺序（文件约定层 → manifest → EVENT_STREAM → Task Ledger → registry → 恢复机制 → 负面测试）。每条补丁应在其**所属阶段内**按此先后执行；切勿误读为"P2-01 EVENT_STREAM 须排在 F4 Task 异常终态之后才动工"。

---

## F1 · 事件 schema_version + migration layer（P0）

**目标**：事件结构长期可演化，旧事件不被新解析器丢弃。

**动作**：
- 在 EVENT_LOG 每行 JSON 增加顶层字段 `schema_version`（字符串，如 `"1.0"`）。
- Phase 1 引入 **schema migration layer**：以 `schema_version` 为键，旧版（已知）事件 → 内部统一模型；**未知版本 → 拒绝并写专属 `schema_version_rejected` 事件（设计文档 §7.7，不静默跳过）**，不复用 `event_gap_alert`（seq 跳号）或 `window_boot_failed`（启动校验失败）——二者语义不符。
- 当前所有示例/生成器统一输出 `"1.0"`。

**验收·正面**：写入 `schema_version="1.0"` 事件，解析成功；用 migration layer 读取一行为 `null`/缺失字段的旧版样例，能映射到统一模型或显式报错。
**验收·负面**：构造 `schema_version="9.9"` 事件，解析器**必须拒绝**（不 crash、不静默接受），并写入专属 **`schema_version_rejected`** 事件（设计文档 §7.7，字段含实际版本 + 目标 seq + detected_by），**不**复用 `event_gap_alert`（seq 跳号）或 `window_boot_failed`（启动校验失败）——二者语义不符，会误导排障。

**引用**：设计文档 §4（Event Pointer）、§7.2（EVENT_LOG 示例 + migration 说明）。

---

## F2 · event_id = UUIDv7（P0）

**目标**：多写者并发下事件 ID 唯一、时间有序、可排序。

**动作**：
- 废弃 `E004-xyz` 类自编 ID。所有事件 `event_id` 用 **UUIDv7**（时间有序前缀 + 随机后缀）。
- Phase 1 用 Rust 库（如 `uuid` + `uuidv7` feature）生成；Phase 0 约定层可用伪 UUIDv7 字符串占位但格式必须可解析。
- EVENT_LOG 行内 `event_id` 与 `event_id` 字段统一为 UUIDv7。

**验收·正面**：连续生成 10^6 个事件 ID，无重复；按字节序与发生时间一致。
**验收·负面**：并发 16 写者各生成 10^4 事件，断言 **零冲突**（UUIDv7 分布式安全）；故意注入重复 `event_id`，消费端去重断言生效。

**引用**：设计文档 §4、§7.2。

---

## F3 · seq 分配唯一责任方（P1）

**目标**：全局单调"允许空洞"的 seq 有唯一生成者，避免实现各自拍板。

**动作**：
- Phase 0（约定层）：由**单协调进程/人工**负责 `seq` 分配，文档写明"非并发约定期单写者"。
- Phase 1：**原子 sequence allocator** —— 在 §7.2 的文件锁内执行 `max(existing_seq)+1` 生成，锁保证单写者；allocator 为唯一责任方，其他组件只读。
- 消费端 `last_processed_seq` 语义不变（§7.3）。

**验收·正面**：allocator 连续分配 seq 1,2,3…（允许因崩溃留洞，如 1,2,4）。
**验收·负面**：两写者**同时**尝试分配（绕过 allocator 各 `max+1`）→ 锁竞争 + `lock_contention` 事件，断言最终 seq 无重复且单调；allocator 进程被 kill -9（持锁）→ 启动清理 stale 锁后分配恢复。**注**：seq 分配复用的是 EVENT_LOG 的同一把文件锁（设计文档 §7.2 原子 allocator 在文件锁内 `max+1`），其超时/清理路径与既有 **H-8** 的 kill-9 持锁测试是**同一把锁的同一清理逻辑**，本项仅在其后**追加**"恢复后 seq 连续无重复"断言，不另起炉灶重复实现锁超时/清理。

**引用**：设计文档 §7.2（seq 分配责任）、§7.3（跳号/水位线）、§5.2（启动清锁）。

---

## F4 · Task 异常终态（P1）

**目标**：任务生命周期完整，避免 `blocked` 被滥用承载不可续跑的任务。

**动作**：
- Task `status` 枚举扩为：`pending | in_progress | paused | done | blocked | failed | cancelled | expired`。
- `failed`（执行/测试失败）、`cancelled`（人工取消）、`expired`（周期模板弃用/生命周期到期）为**吸收性终态**。
- 状态机（§8.5）约束：终态不可再迁移到 active/in_progress/done/blocked；认领与依赖自动流转（§8.5/§8.7）忽略终态任务。

**验收·正面**：`in_progress → failed` 合法；`failed → in_progress` 被拒；owner 崩溃后其 `failed` 任务不被认领源选中。
**验收·负面**：若 `T-A.depends_on = [T-B]`，将 `T-B` 标记为 `expired`（而非 `done`）→ 断言 `T-A` **不会**因"依赖全部完成"规则被误判并自动 `blocked → pending`，也**不会**被自动认领；终态任务被强行 `assign` → 断言拒绝。

**引用**：设计文档 §4（Task）、§8.2（status 枚举）、§8.5（状态机）、§8.7（继承认领，忽略终态）。

---

## F5 · owner 权限矩阵（P1）

**目标**：owner / co_owner / observer 三方权限明确，防实现漂移。

**动作**：
- 实现 §8.7.1 权限矩阵：

| 权限 | owner | co_owner | observer |
|------|:-----:|:--------:|:--------:|
| 读取 | √ | √ | √ |
| 写 checkpoint | √ | √ | × |
| 追加 evidence_ref | √ | √ | × |
| 状态迁移 | √ | × | × |

- 任意写操作前校验调用者角色/身份：observer 写 checkpoint → 拒；co_owner 迁移状态 → 拒；非 owner/co_owner 写 evidence → 拒。
- 继承（§8.7）后权限随新 owner 自然继承。

**验收·正面**：owner 迁移 `done` 成功；co_owner 写 checkpoint 成功；observer 读成功。
**验收·负面**：observer 尝试写 checkpoint → 断言拒绝（非 panic）；co_owner 尝试 `done` 迁移 → 断言拒绝；调用方**显式传入伪造 `actor`** 冒充 owner → 受限 API 因 `actor` 由本机 `manifest.window_id` **自动填充、拒绝显式传入**（设计文档 §7.5 身份防伪机制）而被拒，普通写接口无法伪造 actor；跨窗口写必须走 `assign` 事件路径。

**引用**：设计文档 §8.7.1（权限矩阵）、§8.7（继承）、§7.5（受限 API 强制访问控制）。

---

## P2-04 · manifest identity_epoch（增强，FINAL 后择机）

**目标**：防止同一 `window_id` 被复用作不同角色导致历史语义污染。

**动作**：manifest 增加 `identity_epoch`（时间戳/uuid）；窗口所有产出（EVENT_LOG actor、Task owner）与该 epoch 绑定；读取历史时校验 epoch 一致性，不一致标记 `identity_epoch_mismatch` 告警。

**验收·正面**：同一 `window_id` 在不同 `identity_epoch` 下先后运行，两段历史事件各自与对应 epoch 正确关联、读取不混淆；`identity_epoch` 一致的窗口重启后能正常继续认领/归因历史任务。
**验收·负面**：复用 `impl-01` 但 `identity_epoch` 不同的窗口，其旧历史事件不被新身份静默认领/归因。

**引用**：设计文档 §5.1、§4（Manifest）。

---

## P2-01 · EVENT_STREAM 逻辑/物理分层（增强）

**目标**：切片策略变更不触发代码重构。

**动作**：对外 API 暴露逻辑 `EVENT_STREAM`；物理实现为 `events/EVENT_LOG.YYYY-MM.jsonl` 多文件；`last_processed_seq` 跨切片单调递增。Phase 1 读写层以「逻辑流」为接口，物理文件名由切片策略决定。

**验收·正面**：跨月边界（上月末→本月）消费游标连续，不丢事件。
**验收·负面**：直接删/改名物理切片文件 → 断言恢复路径报错（不静默读到空）。

**引用**：设计文档 §7.6（数据生命周期 + 逻辑/物理抽象）。

---

## P2-02 · knowledge_conflict 事件（增强）

**目标**：知识**正确性**冲突（非文件一致性）提请人工/审计，避免静默覆盖错误知识。

**动作**：事件类型增设 `knowledge_conflict`（§7.7）；当 IMPORT 或跨角色写入与 SSOT 既有知识正确性冲突时写入，triggers 审计窗口介入；与文件一致性（最后写入 + prev_hash 裁决）区分。

**验收·负面**：构造「新写入与既有知识语义冲突但文件层无冲突」→ 断言 `knowledge_conflict` 事件被写入、且不被最后写入规则静默吞掉。

**引用**：设计文档 §7.7（事件类型目录）、§11.2（IMPORT）。

---

## P2-03 · IMPORT trust_level 四类（增强，并入既有）

> **【性质：仅回归验证，无新增开发工作】**——设计文档 §18.2 已确认本项在 Q3 provenance 阶段完成（`trust_level` 已并入 §11.2）。本轮只确认枚举扩为四类且 IMPORT 校验生效，施工窗口**不估新增代码量**。

**目标**：跨项目导入信任分级更细。

**动作**：`trust_level` 枚举扩为 `internal | verified | external | unknown`（取代原 high/medium/low）；IMPORT provenance 中由导入者标注；`[imported]` 标签保留。

**验收·正面**：四种等级均可写入并影响（未来）审计权重。
**验收·负面**：写入非法 `trust_level`（不在上述四类中）→ IMPORT 校验拒绝（对应汇总门禁表 P2-03 行）。
**引用**：设计文档 §11.2（IMPORT provenance）、§18.2（已并入确认）。

---

## 汇总 · 守门员门禁（施工收尾必跑）

| 类别 | 负面测试（故意造失败） |
|------|----------------------|
| F1 | schema_version 未知版被拒 |
| F2 | 并发 UUIDv7 零冲突 + 重复 ID 去重 |
| F3 | 双写者绕过 allocator → 锁竞争无重复 + kill-9 持锁恢复 |
| F4 | 终态任务不可再流转/不误认领 |
| F5 | observer/co_owner 越权写被拒 + 伪造 actor 被拒（§7.5 身份防伪） |
| P2-01 | 物理切片损坏不静默读空 |
| P2-02 | 知识冲突不被文件一致性静默吞 |
| P2-03 | 非法 `trust_level`（不在四类中）被 IMPORT 校验拒绝 |
| P2-04 | `identity_epoch` 不一致不静默归因/认领 |
| H/M（既有） | 并发双写 / seq 跳号 / stale 误认领 / content_hash 篡改 / 依赖死锁 / legacy 回滚 / 分区恢复 / kill-9 持锁 |

**交付判定**：上表全部「负面测试能失败」→ 补丁通过，可进入 Phase 0 后续项或 Phase 1 收尾。任一正面测试无失败能力 = 该断言作废，须重做。

---

## 与 FUSION v3 设计文档的关系

本清单是 **FINAL v1 的 delta 执行视图**。完整概念、架构、生命周期、跨项目边界见 `project-window-sync-design.md`（FINAL v1）。施工窗口遇歧义以设计文档 § 引用为准；若发现本清单与设计文档冲突，**以设计文档为准并回报顶层**。
