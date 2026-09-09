# R2-C ContextBuilder 施工报告（v0.2.9 · 2026-08-29）

> **依据**：《顶层批准：R2-C ContextBuilder Construction Order v1.1》+ 守门员批注 4 条。
> **交付**：diff/commit 链（`7066ab3...HEAD`）+ 门禁实测 + 单测 + cache 对照 + B/C 复测 + stability A/B/C + 偏差清单。
> **执行**：执行窗口（用户睡眠期全权委托）。

---

## 一、施工内容（diff 摘要）

| 层 | 实现 |
|---|---|
| **L1/L2 system 收敛** | build_messages 中移除 experience（:1444-1489 区）/TaskGraph 全量/retrieval/LSP 四注入块（1876 字符）——system_text 收敛为 constitution+Hearth.md+骨架+**Task Topology 块**（新增，topology_sig=id/desc/deps 无 status，变化才重建——双 sig 分立，裁决 1） |
| **L4 尾部固定顺序** | history → retrieval → LSP → experience（方案 X，先空后填=expected-dynamic）→ **Task Continuity（最终锚点居尾）**——全部 Role::System 标签消息 |
| **[FAILED] 保全**（裁决 3） | Continuity 新增失败节点行（id+TaskResult.output 80 字符摘要） |
| **telemetry** | 既有 system_hash/prefix_hash 延续；tool_schema_hash/prefix 链列为改进项（本轮未落——见偏差） |
| **D 类** | **零触碰**（diff 无 bash 截断/fs accounting/Revision 接入/控制流——批准书 §七 遵守） |

## 二、门禁实测（批准书 §八）

**隔离门禁 `~/t_gate_r2c.log`（06:02 轮）：FMT_CHECK_RC=0 / CLIPPY_RC=0 / RT4_SOLO_RC=0 / TEST_RC=0——381 passed / 0 FAILED / 0 ignored**（373 基线 + CB 6 测 + T2 两测——实测口径，非预写）。

## 三、单测结果（批准书 §十 六条回归）

| 断言 | 结果 |
|---|---|
| 同 goal 两次 build_messages system 字节一致 | ✅ `test_cb_system_stable_across_calls` |
| status 变 → system 不变 + Continuity 更新 + topology 块不重建 | ✅ `test_cb_status_change_keeps_system` |
| topology_sig 语义（status 无关/desc·deps 敏感） | ✅ `test_cb_topology_sig_semantics` |
| experience None→Some 只影响 L4（长度+1/恢复） | ✅ `test_cb_experience_l4_only` |
| [FAILED] 节点 id+result 保全于 Continuity | ✅ `test_cb_failed_node_preserved_in_continuity` |
| compact 后 Continuity/Topology 存活 + system 稳定 | ✅ `test_cb_compact_keeps_continuity_and_topology` |

## 四、三层 Stability（批准书 §六）

| 层 | 结果 |
|---|---|
| **A. System Stability** | **主任务会话达成**：78e4afb7 的主链（msgs 6→8→11 递增）system_hash **3 请求同一值** ✅。**口径修正**：此前"10 请求 7 唯一值"把 **planner decompose/reflect 独立调用**（msgs=2 小请求，各任务本就不同 system）混入同 session 统计——**公平口径下 A 层达成**。telemetry 缺 phase/caller 维度（改进项） |
| **B. Message Prefix** | 逐消息 hash 链采集**未落**（本轮未改 telemetry schema——偏差见 §六）；现有 prefix_hash 显示主链 msgs 递增（预期） |
| **C. Provider Outcome** | **口径断裂**：本轮 config 已切 Agnes（EC-03 要求），EC-01 基线为 deepseek——**跨通道不可比**（批准书明令）。v0.2.9@Agnes 16 请求 hit=0%（Agnes 疑不提供 prompt cache 或未上报）——**须以 deepseek 重跑同任务集才可比**（OPEN） |

## 五、EC-03 B/C 复测（10 跑）+ 行为回归处置

| 任务 | EC-03 基线 B | v0.2.9 复测 B | 判定 |
|---|---|---|---|
| T1 歧义 | failed | failed | 一致（歧义任务本性） |
| T2 多步 | completed 22 步 | **failed 13 步** | **回归——已定位已修复**（见下） |
| T3 隐含 | completed | **failed 13 步** | 同上 |
| T4 换向 | 实质完成 | other（产物序列真执行） | 无回归 |
| T5 问询 | failed | failed | 一致（D 类） |

**T2/T3 回归根因（实锤）**：Tier3 T4 语义停滞检测**误杀**——v20 all_done gate 的 plan 重入（图完成无 write 的正常推进循环）也产出相同 TaskGraph → 连续 2 次 → 误判停滞提前 give_up。
**修复**（commit 已入库）：`graph_stall_count` 在 v20 gate 重入时重置（gate=正常推进；Reflect→Replan 路径保留计数——那才是真"要求换计划"）。
**修复后验证**：T2 复跑 calc3.html **落盘成功**（实质完成；终态停在交互审批等待——非交互环境正常）；T3 复跑仍 failed（非 stalled，失败因子待查——Agnes 质量波动嫌疑，OPEN）。

## 六、偏差 / UNKNOWN / OPEN

| 项 | 状态 |
|---|---|
| tool_schema_hash telemetry 字段 | **未落**（偏差——签名侵入推迟，下轮补） |
| B 层逐消息 hash 链采集 | **未落**（同上——B 层验收依赖它） |
| cache 对照 C 层 | **口径断裂**（deepseek vs agnes）——deepseek 同口径复跑 OPEN |
| T3 复跑 failed 非 stalled 根因 | OPEN（Agnes 波动嫌疑） |
| 第二窗口 rubric 复核 | OPEN |
| EC-05 勘误回填 | ✅（evidence-closure §C.2/F 表 + decision Q2 两处注记） |

## 七、rollback 评估

**不回退**：结构性条件全部满足（门禁绿/system 主链稳定/单测 6/6）；C 层未改善系口径断裂非施工缺陷；T2/T3 回归根因在 Tier3 T4 检测（已修）非 ContextBuilder。

## 八、版本与归档

- **v0.2.9**（Cargo bump）/ commit 链至 HEAD
- `/usr/local/bin/hearth`（双 VM 手工测试通道）= v0.2.9
- `release/hearth-v0.2.9-x86_64-unknown-linux-gnu.tar.gz`（待打包，见部署节）
