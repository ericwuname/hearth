收到，本轮执行窗口到此收口。

我审阅了《R2-C Pre-Construction Decision（EC-03 完成后）》以及此前 Evidence Closure。结论：**本轮执行任务完成，停止继续探索，不施工 ContextBuilder。**

### 一、当前裁决

EC-01~08 已达到进入顶层评审的条件：

* EC-01 Cache：41 请求闭合，token-level 64.3%，request-level 58.5%
* EC-02 miss 分类 A-F：证据等级保持诚实，MISS-A 仍为 likely，MISS-E unknown 不得升级
* EC-03 Intent：5×4=20 跑完成；E 为 API 直连降级口径，不能宣称 Codex harness 等价
* EC-04 TaskGraph：**方案 B（拓扑 stable + 状态 dynamic）继续成立，但证据等级保持 likely**
* EC-05 Experience：task-stable，放 L2 stable，confirmed
* EC-06/07：57GB 已完成 payload/filesystem 二分；ResourceLedger 只覆盖 payload，filesystem 盲区明确
* EC-08：bash stdout 截断设计完成，仍属于 D 类，禁止本轮直接实现控制流
* Goal Revision：已有 T5 行为证据，三分类仍属于 D 类，禁止直接接入 apply_turn_goal
* compact 前后 cache：保持 OPEN，不因为本轮未触发 compact 而假装闭合
* 第二窗口 rubric 复核：如果尚未完成，保持 OPEN，不影响本轮停止

### 二、EC-03 最重要的架构结论

不要把 C 变体的结果理解成“context 越少越好”。

本轮已经出现明确反例：

* T2/T3：过度精简反而损害执行
* T4：Task Continuity 对换向有正向作用
* T5：当前 Hearth harness 的确存在 Task Control 缺失，纯问询被错误路由成任务

因此 ContextBuilder 的核心原则必须锁死为：

> **稳定优先，但不得以删除执行所需事实为代价。**

也就是说，ContextBuilder 不是“删 prompt 项目”，而是**重新分层、重新放置动态性**。

### 三、现在进入 ContextBuilder 顶层施工前状态

下一阶段只允许准备独立施工单，不允许直接改代码。

施工单应以以下裁决为基线：

1. **TaskGraph 采用方案 B**

   * topology：stable/task-stable 区
   * status/result：dynamic 区
   * 不得删除模型完成任务所需的拓扑与状态事实

2. **Task Continuity 保持唯一注入路径**

   * original_goal
   * completed
   * remaining
   * next_action
   * 不新增第二套 TaskGoal/TaskGraph 注入机制

3. **Experience 上移 L2**

   * 当前任务生命周期内首轮检索后不重新检索，因此按 task-stable 处理
   * 如果未来改变为任务中途刷新，再重新评估

4. **L4 Dynamic 最终候选**

   * history
   * latest user input
   * Task Continuity
   * LSP / observe 动态结果

5. **明确排除 L4**

   * experience
   * TaskGraph topology
   * tool schemas
   * constitution
   * Hearth.md

6. **tool schema 本轮继续全量稳定**

   * 不做相位裁剪
   * 不为了 cache 指标牺牲工具可调用性

7. **Recovery / compaction**

   * resume / compaction 信息必须与正常 dynamic history 区分
   * Task Continuity 不得被 compact 删除

### 四、必须保持的证据纪律

ContextBuilder 施工单中请严格区分：

**confirmed / likely / unknown / open**

特别是：

* “TaskGraph 是 cache miss 头号根因”只能写 **likely**
* “方案 B 一定提升 cache”只能写 **待施工后实测**
* provider cache segment 粒度继续 **unknown**
* compact 前后 cache 继续 **open**
* 57GB 主犯继续 **likely**，不能写成 confirmed
* E 变体不能写成 Codex harness benchmark

### 五、D 类继续隔离

本轮执行窗口已经证明这些问题值得处理，但现在不要混入 ContextBuilder：

* Goal Revision 三分类接入 apply_turn_goal
* bash stdout 截断控制流
* filesystem write accounting
* resource 超限 pause/deny/terminate
* resource-monitor 接线

它们全部保持独立 D 类施工单。

### 六、下一步

请现在只产出：

**《Hearth R2-C ContextBuilder Construction Order v1》**

内容必须包括：

1. 施工目标
2. 当前 build_messages() 真实代码路径与迁移点
3. L1~L5 最终结构
4. TaskGraph topology/status 的具体拆分方式
5. Task Continuity 唯一路径收编方案
6. Experience 从现有位置迁移到 L2 的方案
7. history / LSP / latest user input 的 L4 组装顺序
8. recovery / compact 边界
9. tool schema 保持稳定的约束
10. 兼容性与行为回归风险
11. 最小测试矩阵
12. cache telemetry 对比验收方法
13. intent benchmark 的 B/C 对比复测方法
14. rollback / fail-safe 边界
15. 明确列出**本施工单不允许修改的 D 类事项**

**不要施工代码。不要继续扩大调查范围。不要自行处理 OPEN/UNKNOWN/D 类事项。**

产出施工单后停在这里，等待顶层批准。

---

# 守门员批注（2026-08-29 凌晨 · Pre-Construction Decision 独立复核后补充，与正文同效力）

> 复核实证：`~/t_gate_r2c.log` 隔离门禁 **FMT_CHECK/CLIPPY/RT4_SOLO/TEST 全 RC=0，TOTAL_PASSED=373** ✅（新门禁设计含 RT4_SOLO 独立行——Claude handoff 的门禁拆分真落地）；门禁脚本串台事故以"分窗口独立 log 文件"收口。以下 6 条随 Construction Order v1 施工单执行。

## 补充 1：benchmark 口径 caveat 必须写进施工单

EC-03 的 B 变体（当前 Hearth）全部在 **`HEARTH_ALLOW_NO_CGROUP=1` 降级姿势**下跑（无 memory/cpu/pids cgroup 资源限制）——"harness 价值"数据是在无资源限制护栏的环境取得的。施工单第 13 条（B/C 复测）必须声明同口径；并预告：未来在具备 delegation 的环境复测若出现行为差异，引用此 caveat。另注：开发环境长期降级 = 再出写盘失控时无 cgroup 内存限制兜底——**filesystem write accounting（D 类）的优先级因此上调**。

## 补充 2：T5 行为证据 = Goal Revision D 类单升为 D 类首位

T5"纯问询被当任务跑 5 步 failed"是**有真实用户伤害的行为证据**（用户问一句，agent 跑了 5 步还以 failed 收场）——建议 D 类施工单排序：**Goal Revision 三分类接入第一**（有 EC-03 行为证据 + 真实体感伤害 + 纯函数 `classify_user_input` 已设计好），bash stdout 截断第二（57G 结构缺陷），filesystem accounting 第三，resource 控制流最后。Construction Order 只引用此排序，不混入实现。

## 补充 3：13-17 步结论两处回填（防旧口径被引用）

真实 Tier3 = **0/10**（B02/B08/B09 被 runner 误标 SUCCESS）——这个更正除 incidents/README 外，**Tier3 任务书 T5 节也要补一行更正注记**，防止后续任何文档引用旧口径"3 挂起 + 7 失败"（实际 0 干净完成，且 13-17 步聚集 = cgroup 环境限制 + give_up 预算落点叠加，非隐藏阈值）。

## 补充 4：Construction Order 基线声明用实测值

decision 文档写"新口径 375"是**预估**（T2/T3 测试并入后）——施工单的基线声明必须用**合入后实测的 gate 计数**（跑完落 `~/t_gate_r2c.log` 新 RC 行），不许预写 375。

## 补充 5：15 项清单外追加 4 条强制项

1. **基线声明**：373 隔离门禁口径 + `~/t_gate_r2c.log` 路径（防再串台）；
2. **cache 对比验收**：与 EC-01 41 请求基线**同任务集/同通道/同采集脚本**对比，禁止跨口径比百分比；
3. **B/C 复测**：声明 cgroup 降级口径（补充 1）；
4. **"本施工单不允许修改的 D 类事项"清单放最前**（四件：bash 截断实现 / fs accounting / Revision 接入 / resource 控制流——各自独立施工单），执行窗口一眼看到红线。

## 补充 6：EC-03 的 E 变体降级口径保持诚实

正文已正确标注"E = API 直连 ≠ Codex harness"——施工单与后续所有引用保持此口径；真 Codex harness 对比需先修 relay + agnes-2.5-flash 配置，列 OPEN 不阻塞。
