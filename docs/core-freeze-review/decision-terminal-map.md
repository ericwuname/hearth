# Decision → Terminal Map — CORE FREEZE REVIEW-01 Node 02

> 源码实测（loop.rs reflect GiveUp 臂 / do_plan T4 / Done 相位 / interaction 路径）。
> 三层区分纪律：Stop ≠ GiveUp ≠ Failed；Escalate ≠ Approval ≠ Stop；GiveUp ≠ Completion。

## 正式映射表

| Decision | Terminal 映射 | 等待用户 | 能自动继续 | delegation 影响 |
|---|---|---|---|---|
| Continue | 无终态——继续 Act/下一相位 | 否 | ✓（即继续） | 无 |
| Replan | 无终态——needs_decompose → Plan（cap 3 + Reserve 零和） | 否 | ✓（有界） | 无 |
| Complete | **completed**（盲区C 产物校验 + acceptance 双证据；criteria 空→产物校验单证据） | 否 | — | 无（完成判定不因 delegation 放水） |
| Escalate | **InteractionRequested 事件**——非终态；无通道时 | **是（有通道）** | 无通道→结构化拒绝/挂起待续 | **不能消除语义不确定**——delegation 只解 operational approval |
| Stop | **failed**（T4 停滞 Err / 预算耗尽 abort，经 give_up 拦截链） | 否 | Reserve 复核→可转 completed | 无 |
| GiveUp | **failed**（F9 打 giveup_unverified；criteria 空+产物在+0 errors → RC47 路由 Done） | 否 | RC47 路由 | 无 |

## 五组语义区分（实测锚点）

| 断言 | 实测 |
|---|---|
| Stop ≠ GiveUp | ✓ Stop = T4/预算机制性终止（bounded）；GiveUp = planner 意愿放弃（拦截链覆盖） |
| Stop ≠ Failed | ✗ **注意**：当前实现 Stop 落 `LoopPhase::Error` → 报告 failed（与 GiveUp 同终态投影，报告 reason 可区分："stalled: ..." vs "give_up: ..."）——**语义区分存在于 reason 层而非 terminal 枚举层**（九态封闭集设计如此） |
| Escalate ≠ Approval | ✓ Escalate = 语义澄清请求（InteractionRequested）；Approval = 破坏性命令授权（RC24 结构化拒绝） |
| Escalate ≠ Stop | ✓ Escalate 非终态 |
| GiveUp ≠ Completion | ✓（INV-LR03；Case D/F 反例：核验通过后 give_up 被否决降级为记录——正是 GiveUp≠Completion 的镜像证明） |

## Escalate + delegation active + no human channel（专项）

实测（P1-LTR T11 + RC24-B 语义）：delegation（DelegateSession）**只解除 operational approval**（命令表级破坏性操作自动放行，fork bomb/设备写/内核接口仍拦）。语义不确定（Escalate）时：
- 交互模式 → InteractionRequested 等待用户（真正的等待）；
- 非交互（stdin 非 tty）→ DenyAllNonInteractive 结构化拒绝（approval_denied_noninteractive，秒级终止）——**自动降级为 Stop，不挂死、不静默**（n12r2 真机实证）。

结论：无隐式行为——Escalate×无通道 = 结构化降级 Stop（显式 reason 投影）。

## RC48 分界（Node 03 结论预登记）

- **criteria 空** give_up → RC47 已修（路由 Done 盲区C 兜底）。
- **criteria 非空** + Reserve 零和（早期核验失败耗尽唯一 Reserve）+ 后续真实修复完成 → T4/预算低位终止时**无第二次核验机会** → false stop（n12r1 实证：VERIFICATION_RESERVE=1 + GIVE_UP_INTERCEPTED=1 + 独立复验 sortlib passed 但 failed）。
- 分界 = **criteria 空与非空**；共同根因 = "核验机会零和"设计（防死循环的有界性代价，INV-LR04 ✓）。修复方向（Reserve 分账/acceptance-passed 消费扩展）**须顶层批准**——本登记不修。
