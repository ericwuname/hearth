# hearth-slim 六卡验收·静态验收报告 v1.0（执行窗）

- **日期**：2026-09-10 12:55　**验收对象**：trae 六卡收官（c343031..e469e24）
- **验收协议**：对接单 0b53418（每卡交付物→抽验动作）
- **状态**：**静态验收全过**；动态验收（Linux 测试面+真机复测+L1-01）待 VM 开机后执行

---

## 一、六卡 commit 链（c343031..e469e24，逐卡独立 ✓）

| 卡 | commit | 内容 |
|---|---|---|
| S1 | df9c4fa | bash 超时硬化：120s 默认+`HEARTH_TOOL_TIMEOUT_SECS`+结构化 timeout JSON+64KB 截断（C-fix-status 挂死直接对症） |
| S2 | b0a3a95 | 出网默认放开（语义反转，用户拍板 2026-09-09）+[net] 审计投影+文档同步 |
| S3+S4 | 0dcb090 | prompt 瘦身：主系统段 ≤1.2K、宪法出注入层、talent 门控 `HEARTH_TALENT=1`、env 快照 3 行（按 c343031 §二.1 宪法注入移出旧资产清单） |
| S6 | 34a1e07 | compaction 阈值 32K→16K + [compact] 投影钩子 |
| S4/S5 | b116ccb | **give_up 退役+相位机拆除——run() 现为消息循环**（独立 commit ✓ 前置令合规） |
| 平台 | 0ef638c | MSVC 全门禁暴露的 Windows 平台缺口修复（sandbox 测试 cfg 等） |

## 二、抽验结果

| 项 | 判定 | 证据 |
|---|---|---|
| LoopPhase 残留 | ✅ **0**（grep 实证） | 相位机清零 |
| give_up/stalled 残留 | ✅ 15 处**全为注释/doc/投影字符串**（逐条甄别：:1873 投影模板、:5327 测试断言串等），代码路径级逻辑清零 | 战报"仅注释"属实 |
| a_arm_act_tally | ✅ **9 处原位**（红线守住，R8 资产完好） | grep 实证 |
| fmt 门禁 | ✅ rc=0 | 复跑实证 |
| clippy（agent-core/planner/agent-types） | ✅ 0 error | 复跑实证 |
| sandbox 红线甄别 | ✅ **生产语义零变更**——diff 仅 4 处测试 `#[cfg(target_os="linux")]` 门控（MSVC 全门禁必要适配，trae 已申报"平台硬化"） | diff 逐行核 |
| loop.rs 行数 | 8,114 → **8,009**（S1 超时/S2 投影为增项，S4 删项对冲——行数非判据，机制清零是） | wc 实证 |

## 三、待动态验收（VM 开机即执行）

1. Linux 测试面：`cargo test -p agent-core --lib`（.133 干净环境，Windows MSVC link 环境问题已绕行——GNU link 遮蔽+SDK 库路径两连坑，实锚记录在案）；
2. **真机复测**：slim 构建 → run-regression 双语料（对照臂=0.2.25-base 已有 3 轮数据直接复用）→ 墙钟 ≥50% 销账数据；
3. **L1-01 重考**（题面/判据/密卷在 bench/exam/L1-01-log-cli/）；
4. 出判分数据包 → 顶层销账终裁（执行窗不自宣）。

## 四、trae 状态对齐备注

trae 窗口 12:45 发来的"包B 拍板件回填（1fd94af）+闸 2"报告为 **9-8 旧节点状态**（1fd94af 已确认在历史中、其工作早已被 R7-7 签署/开刀/手术/收口全链消费）——非当前任务。当前 trae 有效产出=六卡收官（e469e24）。已向 trae 侧对齐：下一动作=等待执行窗验收结果，无闸 2/开刀申报事项。
