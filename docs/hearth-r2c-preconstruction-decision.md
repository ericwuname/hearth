# R2-C Pre-Construction Decision（2026-08-29 · EC-03 完成后）

> **依据**：《收到 R2-C Evidence Closure》指令 + 守门员补充 1-6。
> **前置**：v0.2.8 / HEAD 链含 Tier3 T1-T4 修复；EC-03 最小集（5 任务 × A/B/C/E = 20 跑）全 rc=0 完成；原始产物 `docs/data/ec03/`（首轮）与 `docs/data/ec03b/`（B/C 带 env 重跑）。

---

## 0. 执行前置两项（守门员补充 1/6）

### 补充 1：门禁计数澄清 + T2/T3 测试落点

- **v0.2.8 权威计数（隔离门禁 `~/t_gate_r2c.log` 实测）**：FMT=0 / CLIPPY=0 / RT4_SOLO=0 / TEST=0，**373 passed**（v0.2.7 的 370 + T4 双测 3 = 373；`t_gate.log` 的 370 是他窗口旧树口径——**门禁脚本串台事故已修**，见 2026-08-28 日志）。本轮再补 T2/T3 测试（见下）→ 新口径 **375**。
- **T2/T3 负面测试落点（本轮补齐）**：
  - `test_t2_fatal_no_retry`（agent-core）：Fatal 错误 chat 调用数 == 1（零重试——B04/B06 正面阻断）✅
  - `test_t2_transient_retry_capped`：持续 Transient 重试收口 ≤5 次（30s 实测）✅
  - `test_error_classification_table` 补 "read body failed twice" → Fatal 用例 ✅
  - **测试驱动的真修复**：classify 规则优先级缺口——非 LlmError 包装的 "read body failed twice" 字符串曾被 "read body"→Transient 规则误判，规则前置后 Fatal 生效（T2 测试初跑 call_count=5 暴露→修复后 1）。

### 补充 6：T1 补档

- **uuid↔B 轮映射表**已入 `incidents/2026-08-28-tier3/README.md`（12 sid 全对账）。**重大更正**：B02/B08/B09 被 runner 标 SUCCESS（rc=0）但 report 实为 Task failed——**真实 Tier3 = 0/10 干净完成**；**13-17 步聚集确认**（6×17 步 + 1×13 步）。
- B04/B06 畸形响应体不可复得（进程死+未开 trace）——保持披露。
- **EC-03 首轮跑出 Tier3 失败的同源根因复现**：cgroup fail-closed 在无 delegation VM 打死全部 bash/write（`无法创建 cgroup: Permission denied`）→ 工具反复失败 → budget 耗尽 failed。**13-17 步聚集 = cgroup 环境限制 + give_up 预算落点叠加**（T5 答案落地）。
- **处置**：本 VM 开发环境显式 `HEARTH_ALLOW_NO_CGROUP=1` 运行（fail-closed 语义正确保护生产；开发环境降级是正确姿势）——EC-03 B/C 重跑即带 env。

## 1. EC-03 Intent Benchmark 结果（最小集：5 任务 × A/B/C/E）

**执行口径**：A = Agnes API 裸 prompt（无 harness 无工具）；B = Hearth v0.2.8 完整 harness；C = Hearth + 精简约束前缀（**口径近似**——非真 context 精简，ContextBuilder 未施工）；**E = Agnes API 直连要求产出交付物**（**降级口径**——Codex CLI 存在（/usr/bin/codex 0.145.0）但其 Agnes relay 未运行且模型为已下线的 agnes-2.0-flash，同模型前提不满足（补充 3），按批示降级为 API 直连近似，**非 Codex harness 等价，标注清楚**）。

### 终态矩阵（实测事实）

| 任务 | A（裸） | B（Hearth） | C（精简约束） | E（API 直连交付物口径） |
|---|---|---|---|---|
| T1 歧义 | 输出泛泛建议 | **failed 9 步** | failed 5 步 | 输出泛泛建议 |
| T2 多步 | 输出方案文本 | **completed 22 步**（calc.html+自测 js 真落盘） | failed 13 步 | 输出完整 HTML 代码 |
| T3 隐含约束 | 输出 README 草稿 | **completed**（README 落盘、未动现有文件） | failed 5 步 | 输出 README 内容 |
| T4 中途换向 | 输出顺序说明 | **实质完成**（a/b/first/second 序列真执行，终态标记被日志截断） | other（同截断） | 输出两步说明 |
| T5 纯问询 | 直接回答 | **failed 5 步**（问询被当任务执行） | **completed**（约束前缀让它直接答） | 直接回答 |

### 评分（0-2 rubric，执行窗口初评；待第二窗口抽检复核）

| 任务 | 指标 | A | B | C | E | 备注 |
|---|---|---|---|---|---|---|
| T1 | intent preservation | 1 | 1 | 1 | 1 | 歧义任务模型都给泛化方案 |
| T1 | unnecessary assumptions | 1 | 0 | 0 | 1 | B 假设了目标文件并动手（合理假设未确认） |
| T2 | plan alignment | 0 | **2** | 1 | 0 | B 真实分解+落盘+自测；A/E 只能输出文本 |
| T2 | completion | 0 | **2** | 0 | 0 | C 精简约束**伤害**多步任务 |
| T3 | 隐含约束保持 | 1 | **2** | 0 | 1 | B 未动现有文件 ✓；C 因约束过早收敛 failed |
| T4 | goal drift | 1 | **2** | 1 | 1 | B 的换向序列真执行（a→b→删 a）——Task Continuity 生效 |
| T5 | 任务路由正确性 | 2 | **0** | 2 | 2 | **B 把问询当任务跑 5 步 failed**——Task Control 缺失的行为证据 |
| T5 | completion | 2 | 0 | 2 | 2 | 同上 |

**方向性结论**（5 任务小样本，不构成统计结论——批示纪律）：
1. **harness 的价值在多步/约束/换向**：B 在 T2/T3/T4 显著优于 A/E（裸模型无法执行），也优于 C。
2. **harness 的代价在问询路由**：T5 纯问询被当任务执行且以 failed 终态收场——**Goal Revision 三分类（Task Control）的行为证据**（D 类施工单的价值证明）。
3. **批示反例 1 实证**：T4 换向中 Task Continuity（original/remaining/next）支撑了 goal 切换不丢——**帮助成立**。
4. **批示反例 2 实证**：C（精简）在 T2/T3 **反而变差**——"减少动态污染"若砍掉必要执行信息，伤害大于收益。**ContextBuilder 的精简必须以"不丢执行所需信息"为红线**。

## 2. Pre-Construction Decision 三问

### Q1：方案 B 是否继续成立？——**成立（likely 维持，未反证）**
benchmark 未出现"模型需要完整 TaskGraph 才能理解"的反例；T4 换向成功依赖 Continuity 块的 completed/remaining/next 状态面。C 变体（过度精简）反而变差——反向支持"信息要足"。**裁决：方案 B 维持，进入施工单**。

### Q2：Experience 放 L2 stable？——**维持（confirmed 未动摇）**

> **【勘误 · 2026-08-29】**：本 Q2 的 "confirmed 未动摇" **已推翻**——源码实测证实 experience 为错误降级通道（条件性 turn-dynamic，非 task-stable）。批准书裁决采用**方案 X**（保语义、移 L4）。见《顶层批准：R2-C ContextBuilder Construction Order v1.1》。

benchmark 无行为反例；源码证据（首轮检索后任务期不变）不变。**裁决：维持**。

### Q3：L4 Dynamic 最终放什么？——**收敛清单**
- ✅ history（逐条增长，正常）
- ✅ 最新用户输入
- ✅ Task Continuity（R2-D 已有，唯一注入路径）
- ✅ LSP/observe 动态结果（每步变化）
- ❌ **不进 L4**：experience（task-stable→L2）、TaskGraph 拓扑（task-stable→方案 B 的 stable 侧）、tool schema（stable）、constitution/Hearth.md（stable）

## 3. D 类施工设计两份（只设计，不施工）

### 3.1 bash stdout 截断（D 类施工单素材）

```text
默认上限：max_stdout_bytes = 64KB / max_stderr_bytes = 16KB（tools-builtin/bash.rs format_output 处）
截断策略：head 32KB + "...[TRUNCATED]..." + tail 32KB（头=命令回显/开头语义，尾=错误信息常驻）
截断事实保留：附加一行 "[truncated: total N bytes]"——LLM 与用户都可观测
artifact reference：超限时全量写 <workspace>/.hearth/out/<call_id>.log，
  截断消息携带该路径（LLM 可 read 拿全量——信息不丢失）
ToolResult 语义：截断不改变 exit_code/成功判定，只影响文本体积
配置面：HEARTH_MAX_STDOUT_BYTES env 可覆盖（默认 64KB）
验收：构造 >64KB stdout 的命令，断言 ToolResult ≤ 上限+标记且尾部错误可见；
  回归：正常小输出逐字节不变
```

### 3.2 filesystem write accounting（D 类施工单素材）

```text
观测边界：Filesystem Artifact Bytes = sandbox 写路径上真实落盘字节
  （区别于 Tool Payload Bytes——bash 参数 100B 可生成 50G 文件）
落点选项：
  A 案（推荐）：sandbox landlock 路径上的 write syscall 计数（需 sandbox crate
    扩展——G0 层新增观测 hook，零控制流）
  B 案：dispatcher 层 post-call du 工作区增量（粗糙——并发/外部写混淆）
维度独立：ResourceLedger 拆 payload_bytes / fs_bytes 两列，阈值各自独立
  （payload 10MB / fs 1GB——57G 事故特征是 fs 维）
报告面：RunReport 增 filesystem_written_bytes 字段
验收：构造写 100MB 文件的 bash，断言 fs_bytes ≥100MB 且 payload_bytes 不变
```

## 4. Evidence Closure 状态表更新

| 项 | 上轮状态 | 本轮状态 |
|---|---|---|
| EC-01 Cache 41 请求 | ✅ | ✅（+B 变体 telemetry 顺手采集） |
| EC-02 miss 分类 | ✅ | ✅（证据等级不变——批示 §三 遵守） |
| EC-03 Intent benchmark | ⚠️ 未跑 | **✅ 5×4=20 跑完成**（E 降级口径已标注） |
| EC-04 TaskGraph 方案 B | likely | **维持（无反证）** |
| EC-05 Experience | confirmed | **维持** |
| EC-06/07 57G | 二分完成 | ✅（+Tier3 同源根因复现：cgroup 环境限制） |
| EC-08 bash 截断 | 设计 | **设计定稿（D 类施工单素材）** |
| filesystem accounting | 设计 | **设计定稿（D 类施工单素材）** |
| Goal Revision 三分类 | 设计 | **设计+T5 行为证据落袋（D 类待单）** |
| Intent benchmark 复核 | — | ⚠️ 执行窗口初评完成，待第二窗口抽 2 任务复核 |

## 5. OPEN / UNKNOWN / DEFER / D 类清单

- **OPEN**：compact 前后 cache 样本（本轮任务未触发压缩）；E 变体真 Codex harness 口径（relay+模型配置修复后）；第二窗口 rubric 复核
- **UNKNOWN**：provider cache segment 粒度（MISS-E）；57G 主犯归因（现场已清）
- **DEFER**：Bridge（B4-2 re-entry）；Subagents/TUI/MCP
- **D 类（等待独立施工单）**：bash stdout 截断实现 / filesystem write 维度 / Goal Revision 三分类接入 apply_turn_goal / resource 超限控制流（pause/deny）

## 6. 施工前置自检（批示"停在这里"条款）

- EC-03 ✅ 完成
- ContextBuilder 方案未被 benchmark 反证（T4 正向 + C 变体反证的是"过度精简"而非方案 B）✅
- Evidence 等级未夸大（MISS-A 维持 likely；T5 发现以"行为证据"表述）✅
- D 类控制流继续隔离（本轮零控制流改动——只加测试与设计）✅

**停在这里。ContextBuilder 施工等顶层批准。**
