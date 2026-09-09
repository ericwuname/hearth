# CFR 真机任务 criteria 冻结（S-2，2026-08-31 冻结于首跑前）

## Node 06 C-probe（防污染设计，批-5）

无 acceptance（探针）；判定 = 回复确定性 substring 含 `VAULT-3341` 且 **零 cat/read/echo 重执行**（否则 C 反证污染）。
防污染三要素：goal 文本不含事实字符串（经 `echo > seed.txt; cp` 命令链进入早期 tool result）；阈值 1500 + 6×seq 轰炸保证早期轮全压缩；提问轮显式禁读。

## Node 08 canonical（chainfree，单跑成链闭环）

```
--budget 80, HEARTH_COMPACT_CHAR_THRESHOLD=7000（S-3 声明）
--acceptance 'cmd: cargo test --manifest-path chainfree/Cargo.toml'
--acceptance 'file: FREEZE_RESULT.txt contains CHAIN_OK'
```

链：plan→write→test fail→repair→retest→compact→stop→resume→complete。判定：re-teach=0 / 零重复破坏性 / 零 goal mutation / 零假完成。

## Node 09 Task A（todoapi）/ Task B（units）

```
--budget 80
A: --acceptance 'cmd: cargo test --manifest-path todoapi/Cargo.toml'
   --acceptance 'file: todoapi/NOTES.md contains todoapi'
B: --acceptance 'cmd: cargo test --manifest-path units/Cargo.toml'
   --acceptance 'file: units/CHANGELOG.md contains units'
```

## Node 10 Case A（configlib 常量错误拓扑）/ Case B（目录缺失环境拓扑）

```
A: --budget 60, --acceptance 'cmd: cargo test --manifest-path configlib/Cargo.toml'
   --acceptance 'file: configlib/src/lib.rs contains MAX_RETRIES: u32 = 3'
B: --budget 40, --acceptance 'file: legacy_app/FINAL.txt contains ADAPTED_OK'
```

Case B 故障注入 = 探查步（ls 2>&1 观察）——环境适配拓扑（与 Case A 常量错误不同）。

## Node 11 QA 16 轮

无 acceptance。逐轮记录：give_up / GoalMutation / duplicate execution。
"继续"×2（进行中形态 + 已完成形态）+ 情绪+任务复合输入 ×2 + 自我指涉 + 用户纠正。
