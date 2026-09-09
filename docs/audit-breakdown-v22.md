# 独立审计与任务拆解 — audit-breakdown-v22

> 角色：审计规划窗口（独立审计，不盲信规划报告）
> 输入：`docs/roadmap-consolidated-v22.md`
> 方法：Grep / Read / Bash 交叉核验源码，行号以实测为准
> 约束：本窗口**只审计 + 写本文档**，未改任何 `crates/` 业务代码，未改任何配置
> 核验环境：Windows 本机（只读静态核验）；一切编译/测试/应力/回放须在真 Linux VM（`ssh wutao@192.168.220.131`）

---

## 0. 一句话结论

规划报告 `roadmap-consolidated-v22.md` 的**关键断言基本属实**（B1a/B1b/B1c/B4/N1 幽灵文档全部核实成立），但存在**系统性行号偏移**、**一处归因错误**、**一处计数单位混淆**、**一条与维护规程直接冲突的条目（B5）**。首批建议以「先封存红线基线（N2→N1）+ 最低风险旧债 B1a」起步，B1b/B1c 合并为神经系统告警链 EPIC 第三批做。

---

## 1. 核验证据矩阵（行号均为 grep 实测）

| 编号 | 报告断言 | 实测结果 | 判定 |
|---|---|---|---|
| **B1a** | `routes.rs:255` 硬编码返回 `healthy`，非真实探活 | `readyz` 函数定义在 `routes.rs:255`；硬编码返回在 **`routes.rs:258`** = `(StatusCode::OK, "OK")`；`routes.rs:257` 的 `let _ = &state.sessions;` 是**空引用（no-op）**，不做任何探活 | ✅ **属实**（语义准确）。细节修正：返回串是 `"OK"` 非 `"healthy"`；真正硬编码行是 **258** |
| **B1b-①** | `loop.rs:417` `drain_nervous_alerts()` 零调用者 | 定义在 **`loop.rs:422`**；`grep -rn drain_nervous_alerts crates/` 全仓仅此定义，**零外部调用者** | ✅ **属实**。行号 417→**422**（偏移 +5） |
| **B1b-②** | `loop.rs:1590` 丢弃 `drain_civ_alerts()` 返回值 | 实为 **`loop.rs:1638`** = `self.nervous.drain_civ_alerts(); // v11.5: prevent unbounded accumulation`，返回值丢弃 | ✅ **属实**。行号 1590→**1638**（偏移 +48） |
| **B1c-①** | `nervous-system:143` `is_critical = Abandon\|DeliverAndQuit` | 实为 **`lib.rs:146`** = `is_critical = action.is_urgent() \|\| action == Abandon`；`is_urgent()`（lib.rs:30）= `matches!(self, Abandon \| DeliverAndQuit)` ⇒ 等价于 `Abandon \| DeliverAndQuit` | ✅ **属实**。行号 143→**146**（偏移 +3） |
| **B1c-②** | `loop.rs:1595` `Simplify` 分支不可达 | 实为 **`loop.rs:1644`**，该 arm 位于 `if perception.is_critical {`（1639）内的 `match`；而 `is_critical` 恒排除 `Simplify` ⇒ **该 arm 为不可达死代码**。且 `query()` 确会产出 `Simplify`（lib.rs:109/119/141：内存>80% / 磁盘<1GB / 成本>50%）却**无任何效果** | ✅ **属实**。行号 1595→**1644**（偏移 +49） |
| **B4** | `crates/service/src/guest.rs` 不存在 | `ls` 确认 **NOT FOUND**；`service/src/` 仅 `lib/main/per_user/routes/session/sse/templates/user/webhook`；`routes.rs` 无任何 `Guest` 符号 | ✅ **属实** |
| **N1** | `docs/quarterly-baseline-q3-2026.md` 不存在（幽灵文档） | `ls` 确认 **NOT FOUND**；被 `maintenance-protocol.md:81`(§7)、`self-evolution-v20-plan.md:59`、`global-panorama-v21.md:90/124`、`review-v21-maintenance.md:51/54/92` 引用 | ✅ **幽灵文档属实**（见 §2 归因修正） |
| **wiring 基线** | wiring **14/14** 常绿 | `[[capability]]` = **14**（能力断言，✓ 与报告一致）；`[[capability.chain]]` = **23**（调用链）；xray 默认 spec 硬编码 `docs/xray/wiring-v13.toml`（`project-xray/src/main.rs:36`） | ✅ **14 能力断言属实**；见 §2 计数单位澄清 |

### 1.1 B1b 的更深层事实（报告未言明）

`drain_nervous_alerts`（loop.rs:422）**并非独立的「中断告警通道」**，其函数体只是 `self.nervous.drain_civ_alerts()` 的**零调用包装**。真正的事实链是：

1. `nervous.query()`（lib.rs:94）在检测到异常时，于 **lib.rs:156** 把 `[NERVOUS] ...` 条目 push 进 `civ_pending`（本意：写入文明线）。
2. `do_reflect` 在 **loop.rs:1637** 调 `query()`（触发上述 push），随即 **loop.rs:1638** `drain_civ_alerts()` 把该条目**取走并丢弃**（注释自陈「prevent unbounded accumulation」）。
3. 结果：该 `[NERVOUS]` 文明线条目**从未经 CivWriter 落盘**。仅 `tracing::warn!`（loop.rs:1640）通过 `perception.alerts` 副本打了一条日志。

**结论**：B1b 比报告描述的「日志生成后被扔」更严重——**面向文明线的中断记录被系统性丢弃**，而唯一能把它取回的 `drain_nervous_alerts()` 无人调用。

---

## 2. 缺口 / 冲突 / 遗漏

### 2.1 计数单位混淆（三个数字）
`wiring-v13.toml` 存在三个「条数」表述，须统一口径：
- 文件头注释（第 1 行）仍写 **「v13 线 A，7 条」**——**极度陈旧**（guest 任务书 §2#6 早已要求改，未改）。
- 报告口径 **「14/14」= 14 条 `[[capability]]` 能力断言**（✅ 正确）。
- 本次任务书面框架的 **「14 条 `[[capability.chain]]`」不精确**——实际 `[[capability.chain]]` = **23 条**。
- **建议统一为**：`wiring 基线 = 14 能力断言 / 23 调用链`。

### 2.2 冲突：B5 与维护规程「明确不跑」直接抵触 🔴
- 报告 §3.2 把 **B5 经验回路复测** 列为「有益可等 / 可选」。
- 但 `maintenance-protocol.md` §1「明确不跑」白纸黑字：**「经验实验、embedding 对比、LLM 精炼、正交变量矩阵——锻造期方法，不是维护期日常」**。
- 且 §3.3 已把同类 S1（embedding/LLM 精炼/自主凝练）判为「停止」。
- **判定**：B5「扩大经验库 + 随机序复测」属经验实验，与规程冲突，实为触碰「维护期不跑新实验」这条软红线。**应降级为 S 类或须用户明确授权豁免**，不应混在「有益可等」里。

### 2.3 归因错误：N1 的「v20 CHANGELOG 引用」不实
- 报告 N1 称幽灵文档「`maintenance-protocol §7` 与 **v20 CHANGELOG** 都引用它」。
- 实测 `CHANGELOG-v20.md` **无任何 `quarterly-baseline` 引用**。
- 真实引用点为：`maintenance-protocol.md:81`、`self-evolution-v20-plan.md:59`、`global-panorama-v21.md`、`review-v21-maintenance.md`。
- **不影响 N1 成立**（活规程 §7 确有悬空引用），但归因需修正，避免施工窗口去改错文件。

### 2.4 B4「不触红线」表述不准确
- 报告称 B4「不触红线」。但 guest 任务书 §2#6 明确要求**向 `wiring-v13.toml` 追加 4 条断言**——这会把基线从 **14 → 18 能力断言**，即**改变「wiring 14」这个红线数字本身**。
- **判定**：B4 落地会**改动 wiring 基线**，须用户授权 + 同步更新 N1 基线文档；不能算「不触红线」。

### 2.5 基线漂移：B4 guest 任务书前提已过期
- 任务书施工基线 = **v17.0 `fd5829e`**，主干现已到 **v21+**（HEAD `e80bbeb`），中间前进多轮。
- 已发现的漂移：`.gitignore` **现已存在**（104B，Aug 1），而任务书 §2#7 断言「仓库根当前确实没有 .gitignore」——**已陈旧**，交付物 #7 部分已完成。
- 其他风险：§2.1 依赖核验（`tempfile`/`tower` 仅 dev-dep 的硬约束）、路由挂载点行号（§2#5 称 `main.rs:683-699`）均须在当前 HEAD **重新复验**。

### 2.6 系统性行号偏移
报告 B1 系列行号**全部偏低**（+3~+49）：417→422、1590→1638、143→146、1595→1644。施工窗口若照报告行号定位会**落空**。**拆解已全部改用 grep 实测行号**；施工前须再次 grep 锚定（loop.rs 达 2998 行，易漂）。

### 2.7 次要归因偏差（记录备查）
- 报告称 v21.0 = commit `4528fd6`；实测 **tag `v21.0` 指向 `5f5406c`**，`4528fd6` 是 v21.0 工作提交、tag 指向其后的收尾提交。不影响结论，仅记录。

---

## 3. 可行性评估（逐条）

| 条目 | 可 VM 施工？ | 依赖是否齐全 | 报告行号 | 关键前置/风险 |
|---|---|---|---|---|
| **N1** 补幽灵文档 | 纯文档（非 crates），本机可写 | ⚠️ 内容须用**真实数值**（14 wiring 摘要 + 90%±5% + 0panic + 100% 回放） | — | **依赖 N2 产出真值**，否则等于「造假封存」→ 触 🔴 数据造假红线 |
| **N2** 首次季度体检 | ✅ 须 VM | ✅ `bench/runner.py`✓、`project-xray/wiring.rs`✓、`bench/replay/`✓、xray 默认 spec✓ | — | 是红线**自检**（不改码）；产出即 N1 的数据源；replay fixture 实际条数须 VM 侧确认（本机 `bench/replay` 存在） |
| **B1a** /readyz 真实探活 | ✅ 须 VM | ✅ 依赖齐全 | 255→**258** | routes.rs **不在**任何 wiring 链覆盖内 → 低红线风险；须回归 wiring 14/14 |
| **B1b** 告警链修复 | ✅ 须 VM | ✅ | 417→**422** / 1590→**1638** | loop.rs 受 wiring 重覆盖（子串断言），保留关键子串即安全；须应力(0panic)+回放(100%) |
| **B1c** Simplify 空挡 | ✅ 须 VM | ✅ | 143→**146** / 1595→**1644** | 改 nervous-system + loop.rs；改降级语义 → 必跑应力+回放 |
| **B2a** CN-001 归档 | 纯文档 | ✅（来源 `reply-to-yuanbao-rfc004-005-cn001.md`✓） | — | 不触红线；文档/规划窗口可并行 |
| **B2b** RFC-004 v3 | 纯文档 | ✅ | — | 4 阻塞 + 3 需修订，规划活 |
| **B2c** RFC-005 替代路径 | 纯文档/设计 | ✅ | — | 须先回答「推翻架构决策=哪次函数调用」再定 v2 |
| **B3** 守夜层 | 落地才 VM；当前仅**评审** | ⚠️ Q1–Q5 未决（design 文档 §8）；新增**常驻器官** | — | 须审计 + 用户授权；落地会追加 wiring（含 `nightwatch-l3-default-off`）→ 改基线；L3 默认 OFF 须锁字面量 |
| **B4** guest 施工 | ✅ 须 VM | ⚠️ 任务书基线 v17.0 已漂移（见 §2.5） | — | 施工量大(~500行+测试)；追加 4 条 wiring（14→18）须授权；须在 HEAD 复验全部前提 |
| **B5** 经验复测 | ✅ 须 VM | ✅ 技术可行 | — | ❌ **与规程「不跑新实验」冲突**（§2.2）；须用户明确授权豁免 |

---

## 4. 拆解：EPIC → 任务包

> 每包标注【角色】与**验收标准**。执行/测试**一律在 VM**；行号施工前须 grep 复锚。

### EPIC-A｜维护期红线基线自检与封存（规程自洽根）
| 包 | 角色 | 内容 | 验收标准 |
|---|---|---|---|
| A1 | 【测试(VM)】 | 跑首次季度体检：`fmt && clippy && test` + `codex-xray wiring` + 24 应力 + 31 回放 + deepseek 20×2 基准 | wiring **14/14 全绿**；应力 **0 panic**；回放 **100%**；基准落在 **90%±5%**；产出体检报告（含原始数值） |
| A2 | 【审计→文档】 | 用 A1 真值封存 `docs/quarterly-baseline-q3-2026.md`（14 wiring 摘要 + 通过率 + 0panic + 100% 回放快照） | 文件存在；数值**逐项来自 A1**（禁编造）；`maintenance-protocol §7` 悬空引用被消解 |
| A3 | 【审计→文档】 | 顺手修 `wiring-v13.toml` 头注释「7 条」→「14 能力断言 / 23 调用链」 | 头注释与实测一致（此项仅改注释，**不改任何断言**；如触碰须先授权） |

### EPIC-B｜旧债 B1a：/readyz 真实探活（最低风险首发包）
| 包 | 角色 | 内容 | 验收标准 |
|---|---|---|---|
| B-1 | 【执行(VM)】 | 让 `readyz`（routes.rs:255/258）真实探活会话管理器（如实际访问 session store / 健康查询），失败返回 503 | 依赖不可用时返回 **503**，可用时 200 |
| B-2 | 【测试(VM)】 | 加集成测试覆盖 healthy/unhealthy 两路径 | 两路径测试通过；**wiring 仍 14/14 全绿**；应力/回放不回归 |

### EPIC-C｜神经系统中断告警链修复（B1b + B1c 合并）
> 合并理由：二者同触 `loop.rs` do_reflect 段 + `nervous-system`，合并减少对 loop.rs 的重复触碰与重复回归。
| 包 | 角色 | 内容 | 验收标准 |
|---|---|---|---|
| C-1 | 【审计】 | 定二选一决策：B1b 接线 drain 结果→CivWriter 落盘 **或** 删死包装并记录；B1c 让 Simplify 可达并实现降级 **或** 删死 arm + 记录 | 决策书含红线影响面（哪些 wiring 子串必须保留） |
| C-2 | 【执行(VM)】 | 按 C-1 改 `loop.rs`(422/1638/1644) + `nervous-system/lib.rs`(146) | 中断告警**要么真落文明线、要么显式移除**；Simplify **要么真降级、要么显式删除**；无死代码/死 arm |
| C-3 | 【测试(VM)】 | 应力注入（内存/磁盘/成本触发 Simplify 与 critical）+ 回放 | **0 panic**；回放 **100%**；**wiring 14/14 全绿**（record_tool_exchange / cost_ratio / consecutive_errors>=3 等子串未失） |

### 文档并行轨（不占 VM 带宽，不入首批执行 EPIC）
- D-B2a/B2b/B2c：【规划/文档】按 `reply-to-yuanbao-rfc004-005-cn001.md` 推进，不触红线，可与 A/B/C 并行。

### 暂缓轨（须用户授权后再排）
- **B3 守夜层**：Q1–Q5 未决 + 新增常驻器官 + 追加 wiring → 先出评审结论与授权，再拆落地包。
- **B4 guest**：先做「基线复验包」（在 HEAD 复验 §2.1/§2#5/#7），确认漂移后再重签任务书 v1.2；追加 4 条 wiring 须授权。
- **B5 经验复测**：与规程冲突（§2.2），须用户明确授权豁免，否则维持「停止」。

---

## 5. 红线校验（maintenance-protocol §1/§2）

红线定义：通过率 <85% / 应力 panic >0 / **wiring 断裂任一条** / 数据造假；§1 门禁要求 **wiring 14 条全绿**；§1「明确不跑」经验实验。

| 条目 | 触红线？ | 说明 / 须授权点 |
|---|---|---|
| N2 | ⚪ 自检不改码 | 它**是**红线校验本身；但可能**发现**红线破裂 |
| N1 | 🟡 间接 | 若填非真值 → 触 🔴 **数据造假**；必须用 N2 真值 |
| B1a | 🟢 低 | routes.rs 不在 wiring 链；仅须回归确认 14/14 |
| B1b | 🟡 中 | 触 loop.rs（wiring 重覆盖，子串断言）；改后须应力+回放+wiring |
| B1c | 🟡 中 | 触 loop.rs + nervous-system，**改降级语义** → 必跑应力+回放 |
| B2a/b/c | 🟢 无 | 纯文档 |
| B3 | 🔴 **须授权** | 新增常驻器官 + 追加 wiring（改基线）+ L3 安全默认 |
| B4 | 🔴 **须授权** | 追加 wiring 4 条 → 基线 14→18；改红线数字本身 |
| B5 | 🔴 **须授权** | 与 §1「维护期不跑新实验」直接冲突 |

**授权门清单（施工前必须拿到用户批准）**：B3、B4（含 wiring 基线变更）、B5（规程豁免）、以及任何**改动 `wiring-v13.toml` 断言内容**（A3 仅改注释除外，但稳妥起见也建议先告知）。

---

## 6. 优先级建议（首批 ≤3 EPIC）

排序原则：**必要度 × 低风险 × 解耦 × 为后续提供对标基线**。

1. **EPIC-A（N2→N1）— 最先做**。理由：① 消解 `maintenance-protocol §7` 悬空引用（规程自洽）；② 首次季度体检本就是维护期红线自检义务；③ 产出的真值基线是**其后所有改动「改前/改后」对标的唯一锚**。不改业务码，风险最低但价值最高。
2. **EPIC-B（B1a /readyz）— 首个执行窗口练手包**。理由：完全解耦、routes.rs 不在 wiring 链、验收清晰（「改后 /readyz 真实探活 + wiring 14/14 全绿」）、红线风险最低。适合验证 VM 施工—测试闭环。
3. **EPIC-C（B1b+B1c）— 第三批**。理由：同属神经系统告警/降级链、强相关、合并降低对 loop.rs 的重复回归；风险中等（须应力+回放），放在基线封存后做便于对标。

**并行**：文档轨 B2a/b/c 可由文档/规划窗口与上述并行，不占 VM。
**暂缓**：B3 / B4 / B5 均须用户授权后再进（B4 另需先做基线复验包）。

---

## 7. 交接备注
- 本文档所有行号为本次 Windows 本机 grep 实测；`loop.rs` 达 2998 行，**施工前须再次 grep 锚定**（`drain_nervous_alerts` / `drain_civ_alerts` / `Simplify` / `is_critical`）。
- 一切 fmt/clippy/test/wiring/应力/回放**必须在 VM**（`ssh wutao@192.168.220.131`）；Windows 本机无 Rust 工具链，本机验证一律无效。
- 本窗口未改任何 `crates/` 代码与配置，仅新增本文件。
