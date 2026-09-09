# 测试规划：v22.0 双引擎红队审计（找漏洞专项）

> 日期：2026-08-02 | 被测基线：HEAD `85a0f6d`，tag `v22.0`
> 目的：给测试窗口一份**能真找出漏洞**的测试计划——不是验收回归，是红队攻击。
> 原则：零成本优先（replay/mock/单测/静态核验）；真 LLM 端到端标注"烧 token，可选"。
> 方法论（守门员）：不信报告信源码；测试必须能失败；三层递进（存在性→行为→表达力）。

---

## 一、被测对象

| 引擎 | 版本 | 关键件 |
|---|---|---|
| window-framework | v1.0.3（177/177） | `src/framework.py`（agent/工具/沙箱/budget/workflow 引擎/deploy/check）+ dogfooding 脚本 + `_cli_mock.py` |
| codex-rust | v22.0（214 passed） | service（routes/session/loop/planner/nervous/sandbox）+ codex-cli（main/client/render/repl）+ wiring 15/15 + 应力/回放 |
| 文档契约 | — | version-iteration-manual / acceptance 报告 / agenda 数字真实性 |

## 二、🔴 施工方自审薄弱点（红队优先攻击面，诚实自白）

> 以下是我自己最心虚的地方——测试窗口**优先打这些**，比泛测更容易出漏洞。

| # | 薄弱点 | 为什么可能被抓 |
|---|---|---|
| S1 | **`/readyz` 探活语义退化**（codex-rust） | EPIC-B 实现：`list_persisted_sessions()` 在 `memory_store=None` 时返回 `Ok(vec![])` → **永远 200**。探活只在有 store 时才真访问——无 store 部署下 readyz 是否名存实亡？ |
| S2 | **window-framework 沙箱路径规范化**（Python 纯路径检查） | `read_allowed`/`write_allowed` 是否处理 `..`、绝对路径、`~`、符号链接、Windows 盘符？绕过一个白名单 = 越权读写共享区。 |
| S3 | **产出验证 `_has_outputs` 强度** | 只查目录非空还是校验内容？写空文件/伪装文件能否骗过"虚假 done"闸门（v1.0.1 修的）？ |
| S4 | **EPIC-C civ 告警落盘** | `drain_civ_alerts` 返回值 → `civ_note`：`civ_writer` 为 None 时是否**静默丢弃**（告警链修了还是没修全）？ |
| S5 | **budget 极端值** | `max_steps=0`/负数/超大、`max_cost` 极端 → panic 还是正确终止？v1.0.1 修过 tokens 累计，边界未测。 |
| S6 | **wiring 15 断言覆盖面** | 故意改坏一个生产子串，`codex-xray wiring` 是否真 exit 1 抓出？15 条够不够锁全部关键路径？ |
| S7 | **codex-cli resume 边界** | resume 不存在/已 done/cancelled 的 session → 清晰报错还是 raw 异常？审批后 resume 的 session 所有权（无鉴权 API）。 |
| S8 | **analyze FC schema 容错** | LLM 产出非法 JSON/缺字段/超长 → analyze 崩溃还是优雅降级重试？ |

## 三、测试场景设计（编号 T1–Tn）

### A 组：window-framework（零 token，全可跑）

| # | 场景 | 方法 | 判据（漏洞=失败） |
|---|---|---|---|
| WT1 | **沙箱路径逃逸**（S2） | 对 `read`/`write_file` 注入 `../../shared/..`、绝对路径、`~`、符号链接路径 | 若越权读写成功 = 🔴 漏洞 |
| WT2 | **产出验证绕过**（S3） | 写空文件、写 1 字节伪装、写 outputs 外路径后声称 done | 若 rc=0/done = 🔴（虚假 done 复活） |
| WT3 | **provider 路由注入**（S1 邻） | window.toml `provider` 注入未知值/空/大写 → 观察回退与 key 读取 | 若泄漏 key 到错误端点或 panic = 🟡 |
| WT4 | **budget 极端值**（S5） | `max_steps=0/-1/99999`、`max_cost=0` → 跑 agent（replay 模式） | 若 panic/死循环/超限不终止 = 🔴 |
| WT5 | **gate 时序攻击** | 窗口 working 时 approve / 重复 approve / blocked 后 resume | 若 progress done 与窗口状态不一致 = 🟡 |
| WT6 | **working 残留恢复** | 置 state=working + 杀 agent 进程 → 引擎重跑 | 若永久卡死 = 🔴（v1.0.1 修过，验证真修） |
| WT7 | **analyze 容错**（S8） | 注入非法 JSON/缺字段的 LLM 假响应（mock analyze） | 若崩溃/卡死 = 🟡；优雅降级 = ✅ |
| WT8 | **replay 零成本全链路** | `AGENT_MODE=replay` 跑 project→window→analyze→deploy→workflow→check | 任一环断 = 🟡 |

### B 组：codex-rust（除 RT7 外零 token）

| # | 场景 | 方法 | 判据（漏洞=失败） |
|---|---|---|---|
| RT1 | **/readyz 探活强度**（S1） | 无 memory store 启动 service → curl readyz；有 store 但 store 故障（FailingMemoryStore）→ curl readyz | 若无 store 也 200 且不区分 = 🟡（探活名存实亡） |
| RT2 | **civ 告警落盘**（S4） | 触发 nervous 告警（critical 场景）→ 检查文明线文件是否出现 "nervous" 条目；`civ_writer=None` 路径 | 若告警仍丢失/静默 = 🔴（EPIC-C 未修全） |
| RT3 | **sandbox landlock 逃逸** | 容器/VM 内：bash 写 `/`、`/etc`、`/workspace` 外 → 检查是否被拦（readonly=/） | 若越权写成功 = 🔴 |
| RT4 | **审批门绕过** | 无 API key 下 approve 任意 sid/aid；repl 外 Chat 场景 need_approval 后直接 approve | 若未校验直接执行 = 🟡 |
| RT5 | **codex-cli resume 边界**（S7） | resume 不存在 id / done / cancelled / 权限不足 | 若 raw panic/栈错误 = 🟡；清晰报错 = ✅ |
| RT6 | **并发压测** | 55 并发 held requests + probe → 429；5 并发 session 正常 | 若全挂/panic = 🔴 |
| RT7 | **应力 8 场景**（ST1-ST8，烧 token 可选） | `bench/stress_v14.py` 8×3=24 次（deepseek） | 任何 panic = 🔴 |
| RT8 | **回放 31 条**（零 token 必做） | `bench/replay.py`（REPLAY_DIR fixtures，provider=replay） | 通过率 < 100% = 🟡 回归 |

### C 组：契约与测试自身（红队最后防线）

| # | 场景 | 方法 | 判据（漏洞=失败） |
|---|---|---|---|
| CT1 | **wiring 破坏测试**（S6） | 临时改坏一个 wiring 子串（如 `record_tool_exchange`）→ `codex-xray wiring` | 若 exit 0 = 🔴（wiring 形同虚设） |
| CT2 | **测试自身验证** | 随机禁用一个 window-framework 测试 / 改坏一个断言 → 套件 | 若仍全绿 = 🔴（测试假绿） |
| CT3 | **文档数字复验** | 214 passed / wiring 15/15 / Docker 验证 / 177/177 逐项复跑 | 任一数字对不上 = 🔵 文档失真 |
| CT4 | **Docker 复验** | VM 上 `docker_verify.sh` 复跑（镜像已缓存，快） | build 失败 = 🔴 |

## 四、执行建议（给测试窗口）

1. **先打最狠的**：CT1（wiring 真防回归？）→ CT2（测试真能失败？）→ WT1/WT2（沙箱/产出绕过）→ RT1/RT2（探活/落盘）
2. **零成本全覆盖**：A 组 8 项 + B 组除 RT7 外 7 项 + C 组 4 项 = **19 项零 token**
3. **烧 token 可选**：RT7 应力（deepseek 24 次调用）+ 真 LLM dogfooding 复跑（每轮 ~10-20 分钟）
4. **每个发现按严重度**：🔴 可利用缺陷 / 🟡 契约不符 / 🔵 文档失真——附证据（复现步骤 + 输出）
5. **报告格式**：每项一行（通过/失败 + 证据路径），失败项给复现命令

## 五、验收标准

- **测试有效**：发现 ≥1 个 🔴/🟡（我不信自己无漏洞——S1-S8 里至少有一个会被抓）
- **测试无效**：0 发现 → 说明测试设计不够狠（红队纪律：不是"没漏洞"而是"没打中"）
- 最终产出：`docs/audit-findings-v22.md`（逐项结果 + 严重度 + 证据）

## 六、诚实预期（施工方自评可能被抓的点）

- **S1 /readyz**：无 store 时永远 200——**最可能被抓**（我上轮实现时接受"无 store=健康"的语义，但严格说探活退化了）
- **S2 沙箱路径**：Python 路径检查对 `..` 的处理——**高概率可绕过**（v1.0.1 只测过标准路径）
- **S4 civ 落盘**：civ_writer None 分支——**可能静默**（EPIC-C 只修了有 writer 的路径）
- 其余为 🟡 级边界问题
