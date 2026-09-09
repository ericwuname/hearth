# Hearth W8 施工单 v1：Goal Revision / 任务类型路由（DEV-1）+ N-2 hotfix + N-1 诊断

> **性质**：顶层裁决稿——**批准后施工**（按既定流程：施工单先行，不直接改代码）。
> **签发依据**：ChatGPT《收到 RC24…》（RC24 FINAL ACCEPTED/CLOSED；DEV-1 → W8 为下一主施工单；DEV-1 先于 DEV-2——先把"不该走这控制流"和"走了但判错"分开）+ 守门员复核（`goal_requires_product` loop.rs:149、既有 QA 直答路径 :1506/:2106——路由有现成底座，非新发明）。
> **基线**：v0.2.9 / HEAD `2f46330` 链；隔离门禁 **399 passed / 0 failed / 1 ignored**（RC24 +8 后实测）。
> **对照数据**：`docs/data/regv-20260829/`（V-1 十轮论述任务 1/10 完成率基线 + DEV-1/DEV-2 归类证据）。

---

## ⛔ 零、红线（冻结清单继续）

- **DEV-2 本单不碰**：reflect LLM 判定质量问题走独立证据窗（DEV-1 路由分流后的干净样本才是它的统计基础）——禁止在本单塞任何"reflect 判定规则"。
- D 类冻结继续：bash stdout 截断、fs accounting、resource 超限控制流、tool schema 相位裁剪、bridge、subagent、TUI、ContextBuilder 架构。
- **分类器单例红线**：本单涉及两个分类维度，各只允许一个入口——①任务类型分类（扩展既有 `goal_requires_product`，planner 入口）②目标修订分类（`classify_user_input`，goal-set 入口）。**禁止造两套互相打架的分类器**，两维度正交（前者决定执行路径，后者决定目标是否变更）。

---

## 一、T-A（主项）：任务类型路由 + Goal Revision 接入（DEV-1）

### A1 · 任务类型路由（论述/问答型短路 TaskGraph 循环）

- **现状锚点**：`goal_requires_product`（`loop.rs:149`）是纯启发式；QA 直答路径**已存在且实测走通**（`:1506/:2106`；V-1 B01 completed 走的就是 QA 判定）——本项不是发明新机制，是**把漏斗正向分类补全 + 把论述型显式路由进既有 QA 路径**。
- **正向分类完善**（V-1 新发现："改/html"类措辞不触发 product）：补齐 product 信号词表/模式（改、修改、实现、编写、创建、修复、重构、生成文件、…+ 指向文件/产物的宾语结构）；负向信号短路（W3 已落）保持。
- **路由语义**：
  ```text
  高置信 QA/论述信号（纯问询/解释/方案论述/无产物宾语）→ QA 直答路径
    （不 decompose、不进 reflect-replan 循环；完成后按 QA 终态收口）
  高置信 product 信号 → TaskGraph 闭环（现状）
  不确定 → 默认 product（安全方向——EC-03 反例：隐含产物任务被误路由直答伤害大）
  ```
- **混合任务**：先答后产物/边答边做类 → 走 product 路径（TaskGraph 内直答步骤合法），不做第三种路由。
- **单测**：正向词表逐条 + 不确定默认 product + 混合任务走 product（先红后绿）。

### A2 · Goal Revision 三分类接入（`classify_user_input` → `apply_turn_goal`）

- 按既定设计接入：Task Control 高置信集（查看状态/看 diff/发生什么了/继续/怎么样了）→ 不动 revision；Conversation → 不动；其余 → Goal Mutation（revision++ + GoalChanged 事件——R2-D 基建复用）。
- **T5 回归断言**：v0.2.8 假完成案例（`✓ completed（2 步）` + 目标替换同帧）作为规则集测试用例锁定。
- 规则判不了的歧义输入 → 保持现状语义（默认 Goal Mutation），**不新增 LLM 分类调用**（成本红线）。

### A3 · RC26 计划块去重

- 实测每份 Plan 草案 missing_goal_source 标注 ×2（块内呈现冗余，574×2=1148）→ 去重为每草案至多 1 次，且仅真歧义时标注。

### A4 · RC31 自动检测（observe-only）

- 长程任务终局时，独立 LLM 调用比对"最终产物/最终陈述"与 original_goal 的语义相关度 → 产出 `goal_drift` 警示事件 + report 字段（**observe 层，不阻塞不强制暂停**——强制暂停仍 D 类冻结）。仅长程任务（steps ≥ 阈值）触发，控制成本。

### A5 · NEW-15 resume exactly-once 语义补测

- 构造"resume 前已有副作用（写盘）"场景，断言 resume 后**不重复执行**该副作用（对照 RC1/RC21 家族）。

## 二、T-B（随单小件）：N-2 hotfix——one-shot Ctrl-C cancelled 终态投影

- `run_local.rs:660` cancelled 分支补投影行（one-shot 无 submit()，注释前提不成立）。
- **独立 commit、独立 gate、独立证据**——不得混入 T-A 的 diff（ChatGPT 明令）。
- 验收：one-shot Ctrl-C 后用户可见 `⏹ cancelled` 终态行（与 REPL 对齐）。

## 三、T-C（并行诊断单）：N-1 G0/sandbox——landlock 对 /dev/null EINVAL（只查不改）

- **现象**：`landlock_add_rule failed for '/dev/null': Invalid argument (os error 22)`——审批门放行后暴露（被旧审批门遮挡至今）。
- **首选假设（先验证）**：landlock `PATH_BENEATH` 规则的 parent 仅支持**目录/常规文件**，`/dev/null` 是 char device → EINVAL 是内核语义必然（非代码 bug）。查内核文档 + 实测确认。
- **修复菜单（按可行性排序，产出诊断报告 + 推荐，未经批准不施工）**：
  1. bash 工具预处理：命令中 `>/dev/null` 重写为工作区 sink 文件（`<w>/.hearth/dev-null-sink`，每次调用截断）——harness 层位桶模拟，landlock 可写；
  2. 接受限制：TC-8 判定层已绿，执行层限制文档化（"沙箱内 /dev/null 不可写"诚实徽章）；
  3. 其他内核机制评估（如 sandbox 内 pre-exec 重定向——查可行性）。
- 产出：`docs/n1-landlock-devnull-diagnosis.md`（根因 + 菜单 + 推荐 + 实测证据）。

## 四、测试矩阵与验收

1. **V-1 十论述任务复跑**（原 prompt 逐字）：路由后完成率对照 1/10 基线——论述型走 QA 直答后预期大幅上升（不预写数字，如实报）；product 任务不被误路由。
2. **T2/T3/T4 product 任务不回归**（EC-03 基线对照）。
3. **T5 问询路由**：`classify_user_input` 落地后"查看状态"类 0 revision（回归断言）。
4. 先红后绿全取证；隔离门禁 ≥399（+本单新测试）全绿；真机 .131。
5. RC26 去重后 missing_goal_source 频次（mine 脚本复验）下降至 ≤574。

## 五、口径与交付

- 口径同前：`~/codex_t` + `HEARTH_ALLOW_NO_CGROUP=1` + Agnes + telemetry 开；V-1 论述任务重跑**逐字用原 prompt**。
- T-A/T-B/T-C 独立 commit 独立 gate。
- 交付：diff/commits、先红后绿证据、V-1 复跑对照表、T5/NEW-15/RC26 测试证据、T-C 诊断报告、N-2 hotfix 证据、OPEN/UNKNOWN 更新。

完成后停，等顶层验收。**DEV-2 证据窗在本单验收后另行设计（用路由后的干净样本）。**
