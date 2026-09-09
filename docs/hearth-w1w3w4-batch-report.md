# Hearth 首批施工报告（W1/W3/W4 · 2026-08-29）

> 指令：《已审阅〈Hearth 问题总账 & 整改申请〉》首批切分（W1/W3/W4）+ 守门员 6 条细则。按 §十 12 项格式报告。

## 1. 当前 HEAD / commit

- `7c2c836` W3 活性与完成语义（先红后绿 3红→4绿）
- `c476e77` W4 事实投影/结构化完成报告（先红后绿）
- `ce3a840` W3 收敛修复（真机验证暴露三处残留，实测驱动）
- 独立 commit、独立 gate，W3/W4 未混批 ✓

## 2. 两 VM source/binary provenance

见 `docs/vm-version-sync.md`（W1 交付）。要点：本地 HEAD = Cargo 0.2.9；.131 binary+source 同步（marker 三查）；.133 source 同步（`completion_fact_check`=3 核实）+ binary 重建部署（后台完成）。`~/codex`（他窗口资源）零触碰。

## 3. W3 diff（活性与完成语义，D3=C）

- **事实级 progress**（批示 4/5）：`do_reflect` 双轨活性——节点 Completed + write 类工具（write_file/apply_patch）成功重置 `steps_without_progress`；read/glob/grep/bash（pwd/ls 类）不算。
- **显式 Completed**（补充 2 生产者纪律）：`mark_nodes_completed_on_accept()` 在 **Done 相位 verify 通过后**统一置位（run 级粗粒度，节点级归因留专项）——生产者=事实校验，非 LLM 自报。
- **完成决策事实校验**（补充 3 尖锐反例）：`completion_fact_check()`——original_goal 提及具体文件而产物无关 → **拒绝 Done** + 注入 hint 重入 Plan；write_attempted/v24-post 两个接受点全覆盖。
- **收敛修复（真机实测驱动）**：①reflect prompt 诚实化——Goal 字段改 original_goal（Tier3 发现 #2）+ Observation 增 `successful_write_count` 事实行（发现 #1，抵消 "0/N completed" 误导）；②单节点图 stall 豁免（确定性 decompose 同图≠停滞，Closure 修复建议 3）；③`goal_requires_product` 显式否定信号短路（"不要创建/修改..."——TC-1 纯读）。
- **RC31 轻量锚**：summary 增 `original_goal`/`artifacts`/`completion_decision` 三字段——Original Goal + 产物 + 决策可审计。

## 4. W3 先红后绿证据

`docs/data/w3-red-evidence-20260829.log`：**3 FAILED**（TC-A 计数/TC-B 节点/TC-C 假完成）+ 1 guard ok → 修复后 **4 passed**。VM 实测（非本地）。

## 5. W3 unit/integration/VM gate

隔离门禁（`~/codex_t` + `t_gate_r2c.log`）：FMT_CHECK_RC=0 / CLIPPY_RC=0 / RT4_SOLO_RC=0 / **391 passed / 0 FAILED**（387+4 W3 测试；W4 共用本轮 gate）。

## 6. Tier3 长程实机结果

| 任务 | 首轮（修复前 binary） | 复跑（W3 后 binary） |
|---|---|---|
| T-A 三步（create→run→verify→record） | ❌ failed（reflect give_up，产物却在） | ✅ **completed**，hello_w3.py+w3_result.txt 落盘，无假失败 |
| T-B 纯读问询 | ❌ stalled（T4×单节点图+goal 误分类） | ✅ **completed**（负向信号短路+stall 豁免） |
| T-C 中文错误投影 | —（审批门阻塞） | ✅ 结构化 `✗ 工具出错` 投影 1 次，**错误帧无绿勾误标** |

## 7. W4 diff（事实投影/Completion Report）

- **RC20 终结**：bash 工具非零退出码/超时 → `Err`（结构化，消息保留 exit code+输出）；scheduler 单调用分支 is_error 由 Result 分支决定（废除 `starts_with("error:")` 猜测）；`render::tool_result` 删除 contains 启发式——只投影成功 ✓，失败走 `render::error` ✗；lib.rs/repl.rs SSE 消费结构化 is_error。
- **RC8/RC9 复核**：G1（v0.2.5）九态 `terminal::normalize` + `status_detail` 已闭合（run_local.rs:508-515 + report schema 齐备 goal/status/terminal/reason/steps/tool summary/artifacts/verification scope/remaining）——本轮复核确认无缺口，六终态（completed/verify_failed/give_up/error/timeout/cancelled）经 normalize 封闭集覆盖。

## 8. W4 tests

`test_bash_failure_is_structured_error`（先红：VM 实测 FAILED → 绿）；`test_bash_timeout` 更新为结构化断言；既有 R4 退出码回归不受影响（`; echo RC=$?` 尾置保 exit 0）。

## 9. 终态/错误投影实机结果

T-C：`✗ 工具出错`（结构化）1 次、错误帧绿勾误标=False——中文/任意错误输出经结构化 is_error 正确投影。

## 10. 新发现

1. **goal_requires_product 漏斗**："改/html" 类措辞不触发 product 分类 → QA 旁路绕过事实校验（TC-C 首跑暴露）；负向信号短路已修，正向分类完善归 W8。
2. **nervous 磁盘警戒线与资源枯竭场景**（T-A 首轮一度疑似，实测磁盘 37G 排除）——nervous Abandon 语义正确，留观。
3. **T-C 首探测被审批门阻塞**（bash `>` 重定向写盘触发审批）——RC24 one-shot 已知 OPEN，行为面再次实证。
4. **reflect LLM 对 "0/N completed" 呈现敏感**（T-A 首轮 give_up）——prompt 诚实化修复；未来节点级归因后可彻底消除。

## 11. OPEN / UNKNOWN / DEFER

- **OPEN**：goal 分类正向完善（W8）/ T4 多节点图以外的 stall 场景 / RC24 审批门（T-C 阻塞根因）/ 节点级归因（W3 粗粒度→精细）
- **UNKNOWN**：reflect LLM 在诚实 prompt 下的残余 give_up 率（需更多真机样本）
- **DEFER**：W5/W6/W7/W8/W10（批示 backlog）

## 12. 是否建议进入下一批

**建议**：W3/W4 首批目标达成——"Agent 做了什么 → 系统实际知道什么 → 用户最终看到什么"链路打通（事实 progress → 显式 Completed → 结构化投影）。真机三任务行为达标。**建议顶层验收本批后派 W5-W8/W10 中的下一批**（推荐顺序：W6 审批门（RC24，真机验证反复被它卡）→ W8 Goal Revision）。
