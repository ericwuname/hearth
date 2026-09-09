# P2-LONG-RUN-ACCEPTANCE-01 真机任务 criteria 冻结（S-2）

> 冻结时间：2026-08-31 02:3x（Node 08-13 首跑之前）。冻结后禁改。
> 纪律：故障注入点必须被 criteria 显式覆盖（"修复后行为=X"而非"文件存在"）。

## Node 08 — Compact + Resume 单跑成链（闭环 P2-MC Deviation①）

```
--budget 80
--acceptance 'cmd: cargo test --manifest-path chainlib/Cargo.toml'
--acceptance 'file: chainlib/src/lib.rs contains a * 2'
--acceptance 'file: CHAIN_RESULT.txt contains CHAIN_RECOVERED'
```

任务：受控 bug（dbl 乘 3）→ 测试失败 → 修复（乘 2）→ 复测 → 强制压缩（阈值 8000）→
deadline 中断 → resume → 完成。判定：resume 后 0 重教育 + RESULT 正确 + 单跑链完整。

## Node 09 — Long-run Product Task A（多文件）

```
--budget 80
--acceptance 'cmd: cargo test --manifest-path textkit/Cargo.toml'
--acceptance 'file: textkit/src/lib.rs contains fn normalize_whitespace'
--acceptance 'file: textkit/src/lib.rs contains fn count_words'
```

任务：textkit 库（normalize_whitespace + count_words 两函数 + 测试），自然含
inspect/plan/write/test/verification。故意注入 bug：初始 normalize_whitespace
双空格处理错误 → 测试失败 → 修复 → 复测（不同 failure topology：逻辑错误而非常量错）。

## Node 10 — Long-run Product Task B（multi-file change）

```
--budget 80
--acceptance 'cmd: cargo test --manifest-path geoutil/Cargo.toml'
--acceptance 'file: geoutil/src/point.rs contains pub fn distance'
--acceptance 'file: geoutil/src/polygon.rs contains pub fn perimeter'
```

任务：geoutil 双模块（point.rs distance + polygon.rs perimeter 引用前者）——
跨文件依赖拓扑（与 Task A 单文件不同）；point 初始实现用平方和不开方（几何语义
错误）→ polygon perimeter 测试连带失败 → 修复 point → 两级测试通过。

## Node 11 — QA 15+ 轮

无 acceptance（QA 语义）；逐轮记录：give_up / GoalMutation / duplicate execution。
输入序列含事实问题/追问/反例/用户修正/"继续"/"查看状态"/"为什么"/"你刚才说的是什么"。

## Node 12 — Controlled Failure ×2

```
--budget 60
--acceptance 'cmd: cargo test --manifest-path sortlib/Cargo.toml'
```

两轮独立跑同一任务（sortlib 冒泡排序含 bug：内层循环边界写错导致排序结果错误）：
验证 FailureKind/RecoveryStrategy/strategy change/修复/复测。禁盲重试（同错误
同策略重复 = INV-LR04 违例）。

## Node 13 — Cross-compaction Stress（≥2 压缩 + 1 resume + failure + repair）

```
--budget 100
HEARTH_COMPACT_CHAR_THRESHOLD=6000
--acceptance 'cmd: cargo test --manifest-path stresslib/Cargo.toml'
--acceptance 'file: STRESS_RESULT.txt contains STRESS_ALL_OK'
```

任务：mathnotes 库 add bug → 失败 → 修复 → seq 轰炸触发压缩#1 → deadline 中断
→ resume → 第二次受控失败（shout 无感叹号）→ 修复 → seq 轰炸触发压缩#2 → 完成
STRESS_RESULT.txt。压缩计数口径 = archive 行 created_at UTC 窗口（批-6），禁日志 grep。
