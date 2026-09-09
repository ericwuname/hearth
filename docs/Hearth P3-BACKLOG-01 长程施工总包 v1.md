# Hearth P3-BACKLOG-01 长程施工总包 v1.0

## 总账清欠：配置语义 / 边界收敛 / 小修 / 真机复验

**日期**：2026-08-31（v1.0 定稿，派工依据 = `consolidated-remediation-ledger-v3.md` §3 批次 1-4）
**性质**：长程施工总包（修复 + 复验；**不含 RC52 归因**——那是 P0-ATTRIBUTION-01 的活）
**执行方式**：Node 00→06 连续自主执行；普通失败自行诊断修复；仅 STOP 允许暂停
**最终交付**：一次性 Final Report + ledger v3 状态回填
**当前基线**：v0.2.18（tag v0.2.18；本包发版目标 = **v0.2.19**）
**开工前置**：**P0-ATTRIBUTION-01 Final Report 提交之后**（其 env-gated 观测代码
入树为本包基线）。**两包禁止同时改树**——P0 是零修复 + 观测代码，本包是行为
修复，混流会毁掉两边的归因与记账。

---

# 0. 使命

总账 v3 的结论："已闭环 17 条；真正的问题在 **11 条从未立项 + 6 条改了没验 +
7 条已定位未修**。" 本总包把其中**不依赖 RC52 归因结论**的部分一次清欠：

> **W2 配置语义（3 条从未立项）+ W7 边界收敛（2 条从未立项 + 1 项决策落地）
> + 小修 3 条 + 真机复验 6 组。**

不进的：RC52（等 P0 归因）/ RC48 修法（须顶层批准，维持 ACCEPTED DEVIATION）/
批次 5 战略项（W5/W12/评估栈，D8-D11 待拍板）/ D7 密钥擦历史（不可逆，顶层
单独裁决）/ SimUser 与 file_issue（SIMUSER-01 另行）。

---

# 1. 裁决前置（本包已内置守门人裁定，顶层可翻案；翻案须先改本节再开工）

## D4：`HEARTH_URL` 双语义拆分 → **裁定 = 方案 A（拆双名 + 旧名兼容一版）**

- 新名：`HEARTH_SERVICE_URL`（service 模式端点）/ `HEARTH_LLM_URL`（LLM base）；
- 旧名 `HEARTH_URL` **保留一个 release cycle**：语义承接为 `HEARTH_LLM_URL`
  （当前主用法），启动时打 **stderr deprecation warning**（指明两个新名）；
- 行为变更点：**旧名不再静默触发 service 模式**——service 模式只认
  `HEARTH_SERVICE_URL` 或显式 flag；此变更必须有能失败的测试锁定
  （"设 HEARTH_URL 不再切 service 模式"先红后绿）。

## D-防线B：seccomp 加 `SENDTO`（沙箱内 bash DNS）→ **裁定 = 不加位**

- 理由：当前 DNS 被 seccomp 拦截**意外构成沙箱出口边界的一部分**；加
  `SYS_SENDTO` = 沙箱能力放宽（触碰历轮 STOP 红线），且会让沙箱内 bash 绕过
  egress-allowlist 的上层语义（allowlist 在应用层，seccomp 放行后 allowlist
  管不住 bash 直连）。
- 落地动作：把"**沙箱内 bash 无 DNS = 有意边界**"写入
  `docs/core-freeze-review/known-deviations.md`（does-not-guarantee 段）+
  ledger v3 该条改 CLOSED-DECIDED（附本裁定出处）；未来若需沙箱内出网，
  走专项评估（allowlist 下沉 seccomp 层）——**DEFER**，本包不做。

## D6：两 VM 源码同步时机 → **裁定 = A（发版后同步）**（延续既有纪律）

## 状态口径 → **采纳 ledger §0 八词词汇表**（"待复测"禁用；Final Report 与
ledger 回填必须用该口径）。

---

# 2. 全局禁止 / STOP

禁止：新事实模型 / 新 Memory 架构 / Terminal 语义变更 / Approval·Sandbox·
Seccomp 放宽（D-防线B 的裁定就是"不加位"，反向收紧不限）/ 触碰 RC52 归因域
（`last_graph_sig` 比较逻辑、T4 阈值 `loop.rs:2641`、progress 口径——全部
P0/后续批管辖）/ 顺手修 DEFERRED 六项。

**STOP-1**：发现必须放宽安全边界才能完成（除裁定外）。
**STOP-2**：RC16 拆分发现双语义比总账记载的更纠缠（如 service 模式判定还依赖
其它 env）→ 停在设计边界，扩裁决。
**STOP-3**：RC25 限域导致既有真机任务回归且无法在限域内解决 → 停，给限域
方案选项。
**STOP-4**：任何一条的修复需要动 planner schema 或 RC48/RC52 语义 → STOP
（改判归 P0 后续批）。
**STOP-5**：provenance 无法建立（binary/source/tag 不一致 → 先修环境，参考
CFR Node 00 先例）。

---

# 3. Node 00 — Baseline / 状态口径声明 / 环境顺手项

1. 双 VM 三查 + sha256 + df（标准动作，沿用 CFR Node 00 先例）；
2. **gate 基线记账**：以 P0-ATTRIBUTION 提交后的测试计数为 baseline
   （447 + P0 新增观测测试，Node 00 实测登记）；
3. **ledger v3 §0 状态口径采纳声明**写入 Final Report 头部；
4. 环境顺手项：E6 死 alias `hearth-cli`（`.bashrc:126`）清理（ENV 类，
   非代码）；
5. 输出 `docs/data/p3-backlog-20260831/node00-baseline.md`。

---

# 4. Node 01 — W2 配置语义三连（RC16 / RC18 / RC13）

**RC16**：拆分实现（按 D4 裁决）。验收 = 三条能失败的测试：
①`HEARTH_LLM_URL` 生效于 LLM 通道；②`HEARTH_URL` 仅作 LLM 兼容 + warning；
③**设 `HEARTH_URL` 不再切 service 模式**（先红：现行为会切）。
**RC18**：`--provider` 与 url 联动止血——config set / flag 指定 provider 时，
若显式 url 与 provider 默认端点不一致 → **stderr 警告**（"key 可能送错端点，
Agnes key→deepseek url 即 401"场景）+ 测试锁定。不做自动改写（那是 D5 全量
方案，本包只做提示止血）。
**RC13**：service 路径白名单读 config.toml（现只读 env，
`service/src/main.rs:409-416` 实锤）——与 CLI 侧 `merge_allowlist(env, cfg)`
同构；测试 = config 设白名单 + env 不设 → service 会话内 allowlist 生效。

每条：先红后绿 + 锚点 + ledger 回填。

---

# 5. Node 02 — W7 边界收敛（RC25 / RC27 + D-防线B 落地）

**RC25**：读范围 = 全盘 `/` → 限域到工作区 + 显式白名单目录。要求：
①限域清单可配置（config），默认 = workspace + `.hearth/`；
②**既有真机任务不回归**（拿盲测 run-001..012 的任务类操作做冒烟——read/
glob/grep/报告写盘全部仍可达）；③越界读的行为 = 结构化拒绝（非静默空结果）。
**RC27**：PATH 同名护栏（`hearth` 解析到非 `/usr/local/bin/hearth` 时告警——
E1 家族防线）。
**D-防线B 落地**：SENDTO 不加位裁定写入 known-deviations.md + ledger 回填
（§1 已述，本 Node 只落文档 + 复核 `sandbox/lib.rs:475/621` 现状注释）。

---

# 6. Node 03 — 小修批（RC34 / RC45 / RC15）

**RC34**：×2 渲染重复——render.rs 加去重/折叠（**渲染快照对比测试**：同一
事件流渲染两次输出快照 diff 为空才算去重生效；先红后绿）。
**RC45**：`verify_replan_count` 双上限（Act<1 `:2785` / Done<3 `:4775`）统一。
修法按 failure-recovery-model §2 预案（分账或统一上限，执行窗口依源码实测定，
**两案均须 fixture 先红后绿**）。**边界**：不得顺手改 `acceptance_replan_count`
（RC48 域）；不得改 T4/RC52 语义；修复后须在报告写明与 RC48 disposition 的
交互说明（不改变其有界定性）。
**RC15**：web_fetch "全域名被拒却标 PASS"判定口径专项复验——构造被拒场景，
确认投影为失败而非 PASS；若仍失真，最小修复 + 测试。

---

# 7. Node 04 — 真机复验批（只测不改；**全部绑 v0.2.19 发版 binary**）

| 项 | 实验 | 判定 |
|---|---|---|
| RC36 | 同一问题连问 3 次 | goal_revision 不增（修 B 后应 0 churn）；原始日志留证 |
| RC33 | revision 对照 | 与 v0.2.8（71）/ v0.2.18 盲测（10）建对照基线 |
| RC24-B | TC-8b 真机复跑 | landlock `/dev/null` EINVAL 不再复现；introspect 可用 |
| RC29 | `trust on` | 不弹窗 + 审计留痕 |
| 复测包 | TC-8b / ×2 渲染 / A-5·A-36 连续性 / 12·18 跑矩阵 | 逐项原始日志 |
| RC2 | 渲染层专项回归 | 人可见输出无 contains 式子串匹配残留（与 P0-A 投影项呼应） |

注意：**复验对象是"改动已落但从未真机验证"的机制**——任何一条复验失败，
如实登记（NEEDS-FIX 新条目进 ledger），**不当场修**（本包修复范围以 Node 01-03
为限）。

---

# 8. Node 05 — W13 常量体检（audit-only，不改行为）

子 agent 4096（= Agnes 512K 的 0.8%）等全库常量清单化：值 / 位置 / 消费点 /
当前 provider 下的实际含义。**只出报告 + ledger 登记（DECISION），不改任何
常量**——改常量 = 行为变更，另行立项。

---

# 9. Node 06 — Final Gate / ledger 回填 / Final Report

1. **版本 bump v0.2.18 → v0.2.19 + CHANGELOG，必须在 Final Gate 之前**（先
   bump 后 gate，binary 记录发版号）；
2. `~/run_gate_r2c.sh`：FMT=0 / CLIPPY=0 / RT4_SOLO=0 / TEST=0；测试计数以
   Node 00 基线记账（added/removed 逐项）；
3. **ledger v3 回填**：本包每条按 §0 口径迁移状态（CLOSED 必附源码锚点；
   复验类附真机日志路径 + session id）；
4. Final Report：执行 provenance / 逐条 disposition / 先红后绿记录 / 双 VM
   登记 / OPEN 残留。

---

# 10. 附：守门员批注（定稿依据）

1. **总账 v3 质量确认**：状态口径冻结（八词 + "待复测"禁用）是防漂移的正确
   机制；RC34 "待复测→从未实现"的修正本窗实测复核属实（render.rs 零去重
   代码）；RC45 双上限、RC16 双语义、RC13 env-only、RC32 阈值=2 四组锚点
   全部实测命中。
2. **四个审批点的裁定**：①状态口径**采纳**；②批次 1/2 **立项**（本包
   Node 01/02）；③D4=A、防线B=不加位（理由见 §1，可翻案但翻案须先改 §1）；
   ④DEFERRED 六项**维持**（其中"40 切片入 archive"在 RC52 修复批后重估）。
3. **与 P0 的关系**：本包开工前置 = P0 Final Report 提交；RC52/RC48/T4/progress
   四域本包零触碰——归因未完不修因，这是本轮全部教训的浓缩。
4. **明确不进本包**：D7（密钥擦历史，不可逆，顶层单独裁决）/ D8-D11 与批次 5
   （战略项，待拍板）/ RC48 修法 / SimUser 与 file_issue（SIMUSER-01）。
5. 守门人零代码改动；执行中每条 disposition 须可复算。
