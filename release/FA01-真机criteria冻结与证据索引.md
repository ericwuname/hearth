# Node 09/12/13 真机任务书 draft — criteria 冻结（批-5 / 复-3.3）

日期：2026-08-30　执行 VM：.131　二进制：/usr/local/bin/hearth（本轮重建，含 FA01 代码）
预算：budget=50，HEARTH_TASK_TIMEOUT_SECS=600，provider=Agnes（config.toml），
env：`unset HEARTH_URL; export HEARTH_ALLOW_NO_CGROUP=1`

> 批-5：故障注入点必须被 criteria 显式覆盖；criteria 全文在 draft 阶段冻结并归档进 Final Report。
> 三任务的 criteria 均为 `hearth chat --acceptance` 参数原文，run 前不再修改。

## Node 09 — calcpkg TDD 受控失败（故障注入点：shout 未实现大写/感叹号）

Goal（8 步脚本）：创建 calcpkg/ 项目；lib.rs 预置 `pub fn shout(s: &str) -> String { s.to_string() }`
（故意 bug）+ `t_shout` 测试断言 `shout("hi")=="HI!"`；cargo test 首跑必失败（受控失败）；
修复 shout（format!("{}!", s.to_uppercase())）；复测通过；RESULT.txt=CONTROLLED_FAILURE_RECOVERED。

**criteria 冻结**：
1. `cmd: cargo test --manifest-path calcpkg/Cargo.toml`
2. `file: calcpkg/src/lib.rs contains to_uppercase`（← 故障注入点的行为级覆盖，非"文件存在"）
3. `file: RESULT.txt contains CONTROLLED_FAILURE_RECOVERED`

## Node 12 — mathlib 复跑 + 上轮 shout bug 闭环（批-1/批-5）

Goal：mathlib/ 项目；`add`（a+b+1 bug）与 `shout`（s.to_string() bug）双受控失败；
`t_add`+`t_shout` 两测试首跑必失败；修复两者；复测通过；RESULT.txt=LONGRUN_RECOVERY_PASSED。

**criteria 冻结（含上轮 OPEN"STATUS 达标但 shout 未修"的行为级闭环）**：
1. `cmd: cargo test --manifest-path mathlib/Cargo.toml`
2. `file: mathlib/src/lib.rs contains to_uppercase`（← shout 真实修复判据）
3. `file: RESULT.txt contains LONGRUN_RECOVERY_PASSED`

步数判据（§17 顶层裁决）：≤52 PASS；>52 须 overhead 层归（基线 42 / 上轮 46）。

## Node 13 — wordcount 非过拟合样本（20-40 步目标）

Goal：wordcount/ 项目（top_words 词频 + main.rs stdin 输出 + 测试）；test 首跑若失败
修复循环；cargo run 管道实测；RESULT.txt=WORDCOUNT_DONE。

**criteria 冻结**：
1. `cmd: cargo test --manifest-path wordcount/Cargo.toml`
2. `file: wordcount/src/main.rs contains top_words`
3. `file: RESULT.txt contains WORDCOUNT_DONE`

## 采集字段（§22）

session_id / steps / failure_class（last_failure_class scratch）/ recovery_strategy /
acceptance_result / terminal / budget_before/after（max_steps vs steps_used）/
deadline（600s）/ tokens。运行日志 + run report 全量归档 docs/data/failure-adaptation-20260830/。
