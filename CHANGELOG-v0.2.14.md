# CHANGELOG v0.2.13 → v0.2.14 — P1-EXECUTION-DECISION-01（2026-08-30）

> 使命：Execution Decision Truth——让 Harness 不把"正确事实"变成"错误决策"。
> 证据：docs/execution-decision-flow.md + docs/data/execdec-20260830/ + Final Report。

## 核心修复（44 步病灶闭环）

- **修 D（Node 05，顶层授权控制流）**：give_up 决策点前插 readiness 先决——
  criteria 非空时「无验证不放弃」+「已验证通过不否决」。planner GiveUp 臂
  与三常量原样（修 A 标定权限备而不用，Node 09 数据未触发）。
  **真机实证**：mathlib 同任务复跑，budget-low 触发 Reserve 核验 →
  `GIVE_UP_OVERRIDDEN → Task completed（46 步，66s）`——三带基线（29/42/44
  步 failed）首次 completed。
- **修 C（RC44）**：acceptance_verification_status 反折叠 failed object
  （此前被 as_str() 洗白成 pending）；+ failures 明细访问器。
- **修 B（Node 07）**：classify_user_input 疑问句类别——纯疑问输入
  → Conversation 不动 goal（revision 峰值 71 机械侧收口）；Product+疑问正交。

## 新增结构（零新事实源）

- **CompletionReadiness 纯函数**（terminal.rs）：四态 NotReady/
  RequiresVerification/ReadyForCompletion/Conflicted——只组合既有结构化事实
  （禁 LLM/禁读文件，守门员约束 2）；全组合单测含反例。
- **REFLECT_FACT_CONFLICT 六分类器**（Node 04）：completion misread/budget
  artifact/verification omission/progress misread/task-type error/unknown。
- **classify_failure 七分类**（Node 06）：结构化输入确定性分类（禁错误文本
  关键词猜测——INV-H 纪律）；本轮观察接入，retry 差异化留后续。

## Governance（Node 12）

- INV-A（TaskControl/疑问不增 revision）/ INV-G（criteria 冻结）单测锁定；
  INV-C 引 P1-LTR T1/T4；INV-E 引 RC24 审批门；INV-F 由 verifier 三道防线设计保证。

## 修正落实

- 修 F：Node 01 fixture 把 GiveUp 臂"不咨询验收事实"从 likely 升 confirmed；
  基线归因更正（模型能力 → 确定性陷阱）。
- 修 E：决策类 Node 全附反例用例；证据当场归档（禁留 /tmp）。
- 守门员约束 1-6 全部执行（Case D/F/E 行为预写钉死、readiness 纯函数、
  Reserve 沿用 acceptance_replan_count、失败层级战报、bump 前置 gate、INV 选型）。

## 遗留 OPEN

- 步数判据 #6（≤42）未达：completed 46 步——增量来自拦截核验+复测步
  （Reserve 消耗既有 steps 的合规成本）；机制/决策/模型三层归因见 Final Report。
- retry 策略差异化（Node 06 分类器的消费端）未实施——后续单。
- 修 A 常量标定：本轮未启用（数据未要求）。
