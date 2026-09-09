# Hearth R2-C ContextBuilder Closure Window 报告（v0.2.9）

> **指令**：《收到 v0.2.9 施工报告。顶层复核结论》（Closure Window 五事项）+ 守门员 6 条补充。
> **性质**：只闭合证据，不改 ContextBuilder 架构；禁改清单全遵守（Experience X/Y / TaskGraph 事实模型 / Goal Revision / bash stdout / fs accounting / resource 控制流 / phase pruning / bridge / subagent / TUI 零触碰）。
> **基线 commit**：施工报告末态 → 本轮 `1f5d293...<final>`（见 §1）。

---

## 1. commit/diff

| commit | 内容 |
|---|---|
| `1f5d293` | telemetry schema 三合一（msg_chain/tools_hash/phase）+ T2 stall 语义锁两测试 + T3 trace 证据归档 |
| `<fix1>` | message_chain E0382（VM 门禁暴露） |
| `<fix2>` | stall 保留测试去 unwrap_err（StepOutcome 无 Debug） |
| `<fix3>` | stall 组合语义测试重写（gate 重置打断→Reflect 路径重累计→Err） |
| `<fix4>` | **list_tools 按名字排序**（Closure-2 观测器实抓：HashMap 迭代随机→tools 数组每请求漂移）+ 确定性测试 |
| docs commit | T3 根因归类入 incidents README；本报告；盲评包；DS 对照数据 |

## 2. gate 实测

- 隔离门禁（`~/run_gate_r2c.sh`，目标 ~/codex_t）：FMT_CHECK_RC=0 / CLIPPY_RC=0 / RT4_SOLO_RC=0 / **TEST 387 passed / 0 FAILED / 0 ignored**（381 + 本轮 6 新测试：gateway chain/tools hash 2 + stall 两臂 + chain 前缀 + list_tools 顺序）——最终轮 `t_gate_r2c.log` 10:34 实测。

## 3. B-layer hash-chain evidence（Closure §1）

**schema**（守门员补充 1：一次动刀三项合并）：
- `msg_chain`: `chain_k = H(chain_{k-1} ‖ role ‖ content_hash)` 逐消息 8-hex，链式传播（role 变化断链可检）
- `chain_head`: 全链终值
- `phase`: msgs≥3=main（主链正式组装）/ msgs≤2=aux（planner decompose/reflect 独立调用）——补充 4 固定口径

**单测证明**（顶层复核 §1 要求的最小证明）：
- `test_message_chain_prefix_stability`（gateway）：同输入 chain 确定；尾部追加 → 前缀值不变；role 篡改断链
- `test_cb_b_layer_chain_stable_prefix`（agent-core）：system 内容相同 → system hash 相同 → **stable prefix chain 相同**；动态消息（experience）只追加在允许的 L4 尾部（前缀 chain 不变）

**真机证据**（DeepSeek 对照 54 请求，§6）：稳定段相邻主链公共前缀达 `15/16`、`17/18`、`8/9`——前缀链保持；`0/N` 断点与 system_hash 变化逐点对应（见 §6 口径披露）。

## 4. tool_schema_hash evidence（Closure §2）

- 观测器落地：`tools_hash = sha256(serde_json(tools))[..12]`，只观测（tools 内容与注册生命周期零改动，无 phase pruning）。
- **实测抓到真偏差**：DeepSeek 对照 3 会话 tools_hash unique = 2/3/4（应全为 1）。根因：`list_tools()` 直接迭代 `HashMap::values()`——**每请求工具数组顺序随机**（内容稳定、顺序不稳）。
- **修复**：`list_tools()` 按名字升序（不改内容/注册，服务"tools 全量稳定"）+ `test_list_tools_deterministic_order` 锁死。
- 结论：schema 内容稳定 confirmed；**顺序稳定性修复后待下轮采集确认 unique=1**（本报告数据采自修复前二进制）。

## 5. T3 根因归类及证据（Closure §3）

**归类：B. Tier3 逻辑问题**（活性判据缺口为主因 + T4 stall 为放大器）。

证据链（RUST_LOG=trace 复跑实录，`incidents/2026-08-28-tier3/t3-trace-rerun-20260829.log`）：
1. 工具 6/6 全成功，`wrote 2041 bytes to TRACE_SUMMARY.md` 落盘（任务实质完成）
2. reflect prompt 实录：`Tasks completed: 0/2`（**TaskNode 永远 Pending——活性判据 RC1/RC10/RC21 家族：节点状态无生产路径更新**）+ `Goal: <图首节点 desc>`（非 original_goal）
3. reflect LLM 看到虚假 0/N → 判 Replan → 单节点图确定性 decompose 产出同图 → `T4 stall_count=2 → Err(stalled)` → Task failed
4. 五分类排除：A（CB）排除——判据与组装无耦合、施工前同型失败已存在、CB 单测 6/6；C（工具）排除——6/6 ✓；D（provider 波动）排除——trace 全程可复现；E——预算 74% 剩、启发式未触发，非 T5 已解类

**修复不在 Closure 授权内（禁改 TaskGraph 事实模型）→ OPEN**，修复建议三条已留 incidents README（活性判据/reflect prompt Goal 字段/单节点图 stall 豁免）。

## 6. DeepSeek EC-01 cache 对照（Closure §4）

**通道前提（补充 3）**：EC-01 基线 = deepseek-v4-flash 直连 `https://api.deepseek.com/v1`（jsonl provider 字段实锤，**不经 relay**）；本轮同端点同模型同 key；key 验活通过（余额曾报 ¥4.4，本轮 4 任务后仍有余量）；任务完成后 VM 配置已恢复 agnes。

**严格复用**：EC-01 原 4 任务逐字 + 同顺序 + 同 TelemetryProvider v2 + `HEARTH_ALLOW_NO_CGROUP=1` + 同采集方式。数据：`docs/data/cache-closure-deepseek-20260829.jsonl`（54 请求，4/4 任务 completed——同口径下行为全绿）。

**四指标对比**（主链 = msgs≥3，补充 4 口径）：

| 指标 | EC-01 基线（v0.2.8） | Closure 对照（v0.2.9） | 结论 |
|---|---|---|---|
| token-level（主链） | 64.3%（全请求口径） | **64.9%** | 持平（小样本不声称提升） |
| request-level（主链） | 58.5%（全请求） | **74.1%** | 方向性偏好 |
| system_hash unique（主链） | 1/1/4 | 1/3/7 | 未全部 =1（见披露） |
| prefix chain | 无观测手段 | 15/16、17/18 稳定段 + 0/N 断点 | 可观测，断点=system 变化点 |

**诚实披露**：
- **system_hash 未达"每主链 session=1"**：含 replan 换图的会话（1c5a90dd=3、5b9262b3=7）>1。归因：replan 产出**不同结构**图 → topology_sig 变化 → L2 Topology 块按设计重建（task-stable 的正确语义：稳定以图结构为单位）。图结构稳定会话（ca2233d8）= 1 达成。**这是设计内代价而非回归**——但意味着"replan 换图"场景的 system 前缀仍会失效，计入 ContextBuilder 效果的已知边界。
- **按批示不预写"cache 提升"**：A/B 层达成度如上；C 层持平 → **结论按批示模板：Hearth 侧 stability 部分达成（图稳定会话达成；replan 换图会话属设计内重建），provider outcome 未改善；provider segment/granularity 仍 UNKNOWN。** 不回退 ContextBuilder。

## 7. EC-03 rubric 复核（Closure §5）

按补充 5：盲评材料包已制作 → `docs/data/rubric-blind-pack/`（T2 多步 + T4 换向 × B/C 双变体 = 4 样本，标签剥离乱序；6 指标 0-2 rubric 表 + 评分纪律在包 README；ANSWER_KEY 单独存放标注评审窗口禁读）。

**执行状态：材料就绪，评分待 Claude 评审窗口执行**（执行窗口自身有利益相关不做盲评人）。回传后与 EC-03 基线逐项对比入库。

## 8. T2 stall regression test（补充 6）

- `test_t2_gate_reentry_resets_stall_count`：gate 重入重置（臂 1）
- `test_t2_stall_retained_on_reflect_replan`（组合场景）：同图 replan 累计 → gate 重入打断归零 → 再累计 → Err(stalled)——若 gate 重置语义泄漏进 Reflect→Replan 路径则永不 Err（臂 2）
- MockLlm/MockPlanner 构造，零预算（补充 6 口径）

## 9. OPEN / UNKNOWN / DEFER

| 项 | 状态 | 说明 |
|---|---|---|
| T3 修复（活性判据/reflect Goal 字段/单节点图 stall 豁免） | **OPEN** | 根因已证，修复待顶层出单（Closure 禁改） |
| rubric 盲评执行 | **OPEN** | 材料 ready，等 Claude 评审窗口回传 |
| tools_hash unique=1 复测 | **OPEN** | 排序修复后下轮采集确认 |
| provider segment/granularity | **UNKNOWN** | C 层未改善的 provider 侧原因（MISS-E 延续） |
| replan 换图 → system 前缀失效 | **已知边界**（设计内） | topology 按图结构稳定的语义代价 |
| ContextBuilder 最终验收 | **待顶层** | 本报告 + 盲评回传后由顶层裁决 |

## 10. 是否满足最终验收

**执行窗口自评：五事项全部执行完毕，证据闭合；不自行宣布最终验收**——按批示"照例停等顶层"。建议裁决输入：本报告 §3-§8 + 盲评回传（届时补一节）。

---

# Final Evidence Closure 追加（2026-08-29 ·《R2-C Final Evidence Closure》执行）

## F-1 ① tools_hash 重采（修复后二进制，DeepSeek 同口径）

重采：4/4 任务 completed，36 请求（余额监控全程，2.23 起/2.23 报告口径——API 余额端点有缓存延迟，实际消耗约 ¥2）。数据：`data/cache-post-toolssort-20260829.jsonl`。

**tools_hash 判定（含一个重要发现）**：
- **排序修复有效**——同 agent 同 phase 内 hash 恒定（主 agent 主链全程 `95491b52bb17`、planner aux 全程 `4f53cda18c2b`），HashMap 顺序漂移已消除。
- 会话级 unique>1 的剩余变异**100% 归因于 read-only sub-agent 混入**：`spawn_sub_agent` 用 `read_only_view()` 只读工具集（loop.rs:1225）共享同一 provider+SID，其 chat（msgs 3→5→7→8 独立生长链 + 独立 system_hash）落入 msgs≥3 主链口径（task2 会话双值实证）。**口径问题非漂移**——同 phase 同角色下 unique=1 达成。
- OPEN（QUARANTINE Q-2）：telemetry 缺 depth/role 维度，建议与 phase 同批一次动刀补齐。

**补充 3 pre/post 干净对照表**（同任务集同口径，主链 msgs≥3）：

| 指标 | PRE（修复前 54 请求） | POST（修复后 36 请求） |
|---|---|---|
| 主链 token 级 | 64.9% | **74.5%** |
| 主链 request 级 | 74.1% | **88.9%** |
| tools_hash 会话 unique | 2/3/4 | 1-2（全部=sub-agent 混入） |

命中率上升与"顺序漂移是真实污染源"的机制解释一致（补充 1 预期兑现）——**MISS-E 权重下调但未消除**（replan 换图重建仍在，未达理论值）。方向性结论（小样本），不宣称因果定量。

## F-2 ② 独立盲评（Claude 评审窗口 · 无标签隔离执行）

盲评机制：4 样本复制至隔离目录（**ANSWER_KEY 不可见**）、乱序命名 S1-S4，由独立 Claude 评审实例（零施工上下文）按 6 指标 0-2 rubric 评分。回传结果与变体映射（评分完成后由执行窗口解钥）：

| 样本 | 变体 | 任务 | 得分 | 终态 |
|---|---|---|---|---|
| S2 | **B**（Hearth 完整） | T2 多步 | **11/12** | ✓ Task completed（自测失败→replan 修复→复测过→自主交付） |
| S4 | **C**（精简约束） | T2 多步 | **7/12** | ✗ failed（两次 replan 同草案、同命令无效循环→give_up） |
| S1 | **C** | T4 换向 | **11/12** | ⛔ rm 审批门阻塞（审批前有产物验证步骤） |
| S3 | **B** | T4 换向 | **11/12** | ⛔ rm 审批门阻塞 |

**与 EC-03 基线逐项对比**：
- **T2：B>>C 确认**（11 vs 7，差距集中在 drift 可回正 vs 无效循环、零干预 vs 需人工）——基线方向成立。
- **T4：基线初评被修正**——两变体**同为 rm 审批门阻塞**（11/11 同分），基线"B 实质完成/C 失败"的显著差异不成立（差异是审批门，非变体能力；C 的 S1 甚至多了产物验证步）。**勘误**：此前"Task Continuity 帮助成立（T4 换向实证）"的表述降级——该结论由 T2 证据独立支撑（B 自主闭环交付），T4 不再作为证据引用。
- 结构性发现（非变体差异）：**rm 破坏性审批门在 one-shot 模式阻塞交付**（RC24 /dev/null 审批家族的行为面）——两变体同扣 intervention 分。OPEN 归 D 类单。

## F-3 ③④⑤ 合规确认

- 本追加仅补证据：MISS-B 勘误入 evidence-closure、T3 优先级注记入 incidents README、QUARANTINE 备忘（Q-1 拓扑增量注入/Q-2 depth 维度）入整改台账——**ContextBuilder 架构/D 类禁改清单零触碰** ✓
- T3 活性判据保持独立 OPEN，优先级升至 D 类之前（守门员补充 2）。

## F-4 ⑥ Final Acceptance 提交

①（tools_hash 修复有效+干净 pre/post 表）②（盲评回传，T2 确认/T4 修正）均**无结构性反证**——按批示提交 **R2-C Final Acceptance**：ContextBuilder L1/L2/L4 架构、双 sig 分立、方案 X、[FAILED] 保全、A 层（同图结构会话 system_hash=1）、B 层（chain 前缀稳定+tools 顺序确定）全部达成；C 层 74.5%/88.9% 为当前干净口径基线。**不再扩大施工范围**。遗留（全部 OPEN 待顶层排单）：T3 活性判据（优先级最高）/ rm 审批门 one-shot 阻塞 / Q-1 Q-2 QUARANTINE / rubric 全量 5 任务扩展（可选）。
