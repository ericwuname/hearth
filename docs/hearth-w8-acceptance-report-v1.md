# Hearth W8 验收报告 v1：Goal Revision / 任务路由（DEV-1）+ N-2 hotfix + N-1 诊断

> **性质**：施工完成 + 验收证据汇报（执行窗口 → 顶层）。
> **施工单**：`docs/hearth-w8-goal-revision-task-routing-construction-order-v1.md`（基线 HEAD `2f46330` / 门禁 399）。
> **结果**：**T-A/T-B/T-C 全部完成；隔离门禁 407 passed / 0 failed（+8 W8 测试）；V-1 十论述复跑 10/10 completed（基线 1/10）；N-2 真机投影达成；N-1 根因实锤（与施工单首选假设相反）**。1 项新边缘如实披露。

---

## 一、commit 链（独立 commit / 独立 gate / 独立证据）

| commit | 内容 |
|---|---|
| `6ca3938` | **T-B**：N-2 hotfix——one-shot Ctrl-C cancelled 终态投影（独立 gate 399/0） |
| `db15d37` | **T-A**：A1 路由 + A2 Goal Revision + A3 RC26 + A4 goal_drift + A5 补测（gate 407/0） |
| `d5ecebb` | **T-C**：N-1 诊断报告（只查不改） |

## 二、T-A 施工内容

### A1 · 任务类型路由（DEV-1 主项）
`goal_requires_product` 重构为**三态漏斗**（bool 签名不变、4 个调用方零改动）：
```
1. 负向信号短路（W3 语义保持）        → QA 直答
2. 产物动作动词（创建/修改/改成/优化/更新/重构/补充/美化/接入/…）→ Product
3. 论述/QA 信号（问句标记/论述动词/论述要求词）→ QA 直答
4. 默认 → Product（EC-03 安全方向）
```
- **关键变化**：旧名词表（"函数/代码/文件/实现"裸词）迁出 product 信号——名词是论述内容的一部分（V-1 论述 prompt 因"函数/实现"被误判 product）；"实现"降级为"实现一个/实现这"（"核心实现思路"是论述）。
- **先红后绿**：旧逻辑 Python 复刻实测 **11 红**（discourse 误路由 ×1 + product 漏分类 ×6 + 默认方向 ×4），取证脚本与输出在案。
- 新增单测 3 个（论述 8 例 / product 动词 13 例 / 默认与混合 5 例）全绿。

### A2 · Goal Revision 三分类接入
- 新建 `classify_user_input`（goal-set 入口**单例分类器**，正交于 A1，零 LLM 调用）：TaskControl / Conversation / GoalMutation（歧义默认）。
- `run()` 接入：TaskControl/Conversation 输入**不替换 current_goal、不 revision++、无 GoalChanged 事件**（用户消息仍进历史）；首轮（original absent）永远 Mutation。
- **T5 回归锁定**：v0.2.8 假完成案例投影行形态（`✓ completed（2 步）`/`✗ Task failed…`/`⏹`）规则集断言全绿。

### A3 · RC26 计划块去重
- CLI `render.rs` plan_draft 第二段 auto_assumed 重复渲染删除（根因：旧残留段，⚡ 假设行 ×2）。
- planner `missing_goal_source` 收紧为**仅真歧义**（goal 引用外部上下文"上面/刚才/该文件…"而无来源标注）——直接指令不再无差别标注。
- 既有 planner 测试同步更新（外部指代 → 有 gap；直接指令 → 无 gap）。

### A4 · RC31 goal_drift 自动检测（observe-only）
- `check_goal_drift`：steps ≥ 20 的长程任务终局时独立 LLM 比对最终产物/陈述与 original_goal → DRIFT/ALIGNED/None。
- 产出 `⚠ [goal_drift]` 警示事件 + report `summary.goal_drift` 字段；**不阻塞不强制暂停**（D 类冻结遵守）；LLM 失败 → None 静默跳过。
- 纯函数解析 + MockLlm 三态直调测试。

### A5 · NEW-15 resume exactly-once 补测
- 构造"resume 前已写盘"场景：哨兵文件内容不变 + 空图 decompose 不覆盖已完成图（节点保持 Completed）——RC1/RC21 家族机制断言绿。

## 三、T-B（N-2 hotfix）

`run_local.rs` cancelled 分支补投影 + `repl.rs` 重复行移除（统一投影点，两类入口各渲染一次）。**真机证据**（.133 release）：
```
INFO hearth::run_local: run cancelled by Ctrl-C
  ⏹ 本轮已取消（Ctrl-C）——历史保留，可 resume 或输入新目标继续
```

## 四、T-C（N-1 诊断，只查不改）——根因与施工单假设相反

C 矩阵实测（kernel 7.0.0-30，`/tmp/ll_diag.c`）：

| 用例 | 结果 |
|---|---|
| /dev/null + FS_RW 全量 | **EINVAL**（真机现象复现） |
| /dev/null + 纯文件权限 | **OK** |
| **常规文件 + FS_RW 全量** | **EINVAL（推翻"char device 特例"假设）** |
| 常规文件 + 纯文件权限 | OK |
| 目录 + FS_RW 全量 | OK |
| /dev/full + 纯文件权限 | OK |

**真根因**：`FS_RW` 权限集混入目录专有位（READ_DIR/REMOVE_DIR/MAKE_DIR）——内核规定目录专有权 + 非目录 parent_fd → EINVAL（landlock_add_rule(2) ERRORS 原文）。**首选假设"char device 不被支持"被实测推翻**——char device 用纯文件权限完全可放行。T6 自 v0.2.3 起从未生效。
**推荐菜单 0**（替代原菜单 1/2）：非目录路径的 landlock 规则改用 `FS_FILE_ONLY = EXECUTE|WRITE_FILE|READ_FILE|TRUNCATE`——一行级修复，待顶层批准后独立施工。详见 `docs/n1-landlock-devnull-diagnosis.md`。

## 五、真机验收（.133 / release 二进制 / Agnes / HEARTH_ALLOW_NO_CGROUP=1 / telemetry）

### 验收① V-1 十论述任务复跑（原 prompt 逐字，--budget 40）——**10/10 completed（基线 1/10）**

| # | 本轮 | 基线 | 备注 |
|---|---|---|---|
| B01-B03 | ✅ completed（30-32s） | 1 completed + 3 failed | 首跑真实 LLM 论述 |
| B04-B10 | ✅ completed（2.5-4.9s） | 6 failed | 同 prompt 重复 → Agnes 侧缓存加速；**真实性已抽查**（B05：单轮 calls=1、completion 2597 tokens、日志 10478 字节完整论述） |

全部走 QA 直答收口，0 次 reflect-replan 循环，0 次 give_up——**DEV-1 结构性死因闭合**。

### 验收② product 任务不回归
- `PRODUCT_regress`（write_file 创建任务）：✅ completed（21s）——product 不被误路由。
- **T3_regress**（原 prompt："写一个 README.md…不要修改任何现有文件"）：❌ failed（T4 停滞，17 步）——**如实披露为新边缘 OPEN-W8-1**：负向短路（W3 语义）优先于产物动词，"负向约束 + 真实产物意图"混合任务被误路由 QA。分类结果与旧代码**完全一致**（无回归——旧代码同为 false），死因暴露的是该边缘本身。建议（未实施，交顶层）：负向信号含"现有/其他"限定修饰时视为约束不短路。待 DEV-2 证据窗一并设计。

### 验收③ T5 路由断言
`classify_user_input` 规则集单测全绿（投影行/控制语 0 revision；歧义默认 Mutation）+ 集成测试（"继续"输入 → revision 保持 1、current_goal 不变、无 GoalChanged）。

### 验收④ RC26 mine 复验
12 个真机 run 全部 `assumed_dup=False`；`missing_goal_source` 出现 0 次（收紧后直接指令不再标注）——**≤574 达成**。

### 验收⑤ N-2 SIGINT
真机 one-shot Ctrl-C → `⏹ 本轮已取消` 投影行出现（对照 RC24 验收时无投影）。

## 六、门禁

| 轮 | fmt | clippy | RT4_SOLO | 全量测试 |
|---|---|---|---|---|
| T-B 独立 gate | 0 | 0 | 0 | 399/0 |
| T-A gate（最终） | 0 | 0 | 0 | **407 passed / 0 failed / 1 ignored** |

## 七、既有测试适配记录（A1 语义翻转的连带，非回归）

- `test_p3_planner_injected_into_loop`：goal "test planner injection"（无信号）在新语义下默认 product，其 bash-echo 交不了 product 卷 → goal 加 "explain" 维持测试本意（QA 收口）。
- service 三测（p1_natural_done / p4_three_backend_switch / p5_session_survives_restart）：同理，goal 加 "explain" 前缀；p5 持久化断言同步更新。

## 八、OPEN / 待顶层

| 编号 | 内容 | 建议 |
|---|---|---|
| OPEN-W8-1 | 负向短路优先于产物动词："不要修改任何现有文件"式**约束性负向** + 真实产物意图 → 误路由 QA（T3 场景） | 负向含"现有/其他"限定词时不短路；建议与 DEV-2 证据窗一并设计 |
| N-1 修复单 | 菜单 0（FS_FILE_ONLY）待批准 | 批准后独立施工 + TC-8 完整转绿 |
| DEV-2 | reflect LLM 判定质量 | 按裁决：本单验收后用路由后的干净样本设计证据窗 |
| .131 同步 | 执行 VM binary 未含 W8 | 验收批复后 vm-version-sync |

## 九、证据归档

`docs/data/w8-20260829/`：V1_B01/B05/B10 抽查日志、T3_regress、PRODUCT_regress、acceptance summary、sigint 证据（7 文件）。
