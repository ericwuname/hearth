# CHANGELOG v20.0 — 收官轮

修掉横跨六版的 planner 毒债（T13/T19 根因）+ 部署经验遗忘机制 + 封存季度体检基线。

## Added
- **S1 planner all_done 修复**：`loop.rs` 加 `write_attempted` 门控——all_done 但从未写操作时强制 replan（replan_count < 3 兜底）。根治 T13/T19"只读即完成"毒债。wiring 新断言 `all-done-requires-write`。
- **S3 遗忘机制**：`service/main.rs` observer 每小时巡检调用 `prune(0.3, 90)` + `upgrade_core()`（v11 写好未接线，v20 部署）。wiring 新断言 `experience-prune-wired`。

## Fixed（六轮毒债）
- **T13-fix-index**：0/8（v13-v18）→ **2/3 PASS**（v20 S2）——planner 不再"读完就算完"，agent 真的写文件了（事件流证实 grep→read→write_file→bash）
- **T19-merge-duplicate**：仍 0/3（NO_GENERIC_FN）——确认为真工具链盲区，需工具链升级（v20 后留待）

## Baseline（Q3-2026 季度体检基线封存）
- deepseek 固定序 20×2：**35/40 = 87.5%**（v19 92.5% 是上浮，均落在 90%±5% 带内）
- **wiring 14/14 全绿**（11 基础 + 3 新增），workspace 测试全绿
- **季度规程**：新功能 → 加基准任务 + wiring 断言；修 bug → 只跑受影响题 + 回放 + 应力场；季度全量体检

## Changed
- `loop.rs` all_done 判定行为变更（无写操作不再直接 Done）
- `service/main.rs` observer 任务增加经验维护
- `docs/xray/wiring-v13.toml` +2 断言（共 14 条）

## Known Debt（v21+）
1. **T19 工具链盲区**：写不出泛型合并函数——需要工具链/验证脚本升级（唯一剩余盲区）
2. T13 从必挂变波动（修复"不写"，剩余"写错"是模型质量）
3. S5 zhipu 自适应子集验证（两次降级待补，deepseek 侧已无损失验证）
4. deepseek 余额 ~¥7（备用）
