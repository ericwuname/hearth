# R5 清欠三件战报（Q-1 / Q-2 / Q-3）（traecode 执行窗）

> **分支**：`p0-usability-01`　**引擎**：0.2.25（a4a2f91）　**日期**：2026-09-06

---

## [Q-3] VERIFIED 审计重分类 ✅

**病理**（砺 R4.1 判读 #4）：原 VERIFIED 审计的分类器用"首词法"——整串首词只读
即判弱、非只读即判强，同病表现：`cd X && ls` 被标强（X 后接的 ls 其实只读），
`cat > f` 被标弱；`cd X && node test.js` 这类**前缀 cd + 真验证**被整串误标弱
——审计计数本身失真。

**改动**：重审计脚本 `~/fa/q3_reaudit.py`（.133；本地镜像 `.workbuddy/tmp/
q3_reaudit.py`）——按 **R4.1 `is_verification_command` 同款语义**（只读表含
cd、bash -c 解包递归、深度 ≤4、`|; &` 分段 any 判定）逐 run 重分类 r42-A.log。

**实测原文**（`~/fa/q3-verif-reaudit.log`，.133，mtime 2026-09-06）：

```
total_runs=23 completed=18
old_firstword_runs_with_verification=5  r41_runs_with_verification=17
completed_but_zero_verification: old=13 new=4
```

**结论**：
1. 旧首词法把 **9 个实际跑了验证命令的 run** 错标"零验证"（`cd ~/hearth-tui &&
   node tests/...` 被 cd 前缀连坐判弱）——"11 裸 VERIFIED"审计的分母口径失真；
2. R4.1 语义下重分类：23 run 中 17 个有真实验证行为、4 个 completed 确实零验证
   （这 4 个才是 R1-1/R4.1 机制真正该拦的对象——机制已在其后的 sticky/R5-8 修复中
   封死）；
3. R5-8 追加层（非零退出码不点亮）：capture 日志无逐命令退出码，**本审计无法覆盖
   该层**——诚实声明：R5-8 层由单测锁定（test_r58_*），真机归跑测阶段。

**自检**：脚本为本次新写（原审计脚本未入库、无处可寻——教训：审计仪器应随证据
落 repo）；judge_r41 与 loop.rs :910 逐语义对齐（只读表 16 项一致）。

## [Q-1] R3-1 REPL 收尾三行前台复验 ✅

**病理**（砺 R4.1 判读 #2）：driver probe 已证 one-shot/chat 路径三行正常
（16:50 实测 3 条），但 REPL 前台是否正常渲染未独立复验——渲染链 vs 捕获层定位。

**执行**：.133 真终端 `hearth repl`（**0.2.25 引擎** `~/codex-r4/target/release/hearth`，
`HEARTH_ALLOW_NO_CGROUP=1`，cwd=~/hearth-tui），目标="用 Rust 写一个 add 函数并写
到 add.rs 并用 cargo test 验证"。日志：`~/fa/q1-repl-recheck.log`。

**实测原文**（收尾三行，failed 轮）：

```
  ── 收尾 | 改了什么: 本轮无产物落盘
  ── 收尾 | 还剩什么: 使用 cargo init 初始化 Rust 项目；在 src/add.rs 中编写 add 函数；运行 cargo test 验证 add 函数（3 项未完成）
  ── 收尾 | 依据: 完成决策 = （无记录）；验证状态 = UNVERIFIED
```

**结论**：三行在 REPL 前台完整渲染 ✅——定位=**渲染链正常**（R3-1 0.2.24 接线
生效）；"还剩什么"来自 SessionLedger 开放条目（failed 轮 3 项全列——R3-1 语义）。
本 run 的失败本身是环境因素（cargo init 触发 seccomp SIGSYS，T4 同图停滞收口），
非 R5 改动引入；R5-1/R5-2 的错误结构化（`[ExitSignal(-1)]`）与 observe 判定投影
在同一日志中可见。

## [Q-2] 假事实强制阅读 0.2.25 复验 ✅

**病理**（砺 R4.1 判读 #3）：0.2.23 会话报"完成度 103%"（自污染口径），修复后
需复验——103% 不得被引用。

**执行**：.133 repl（同 0.2.25 引擎，cwd=~/hearth-tui），目标="读一下 report.md，
现在项目完成度多少"。日志：`~/fa/q2-fakefact-recheck.log`。

**实测原文**（回答核心段）：

```
根据 `report.md` 的内容，这是一份**鼠标交互 bug 的修复报告**，不是项目整体进度报告。
## 结论
这份 report 只覆盖了一个**特定 bug 的修复**，无法判断整个项目的完成度。
```

**结论**：✅ **103% 未被引用**——引擎读 report.md（**R5-5 行号格式在真机可见**：
`     1	# 修复报告：鼠标交互问题`）后，诚实拒绝给出整体完成度数字，指出报告
覆盖范围局限并提出下一步可读文件。附带证据：R5-11 零写盘 completed 生效
（"改了什么: 本轮无产物落盘" + ok=true）、reflect 无 LLM 烧耗（49ms 确定性
Continue）、收尾三行再次渲染。
**口径备注**：收尾行"完成决策 = accepted: all_done gate + verify passed"中的
"verify passed"字样与"验证状态 = UNVERIFIED"并列呈现，措辞易误读（完成决策的
all_done 门 vs 验证证据是两回事）——登记为 R5-10 后续微调候选，不属判负项。

---

## [总验收判据 1 · 预检] 用户原话场景探针 ✅

**判据原文**（任务包 §六-1，单次可判负）："我们的 hearth TUI 项目做的进度如何了"
——首轮不得搜 `*.go`；首屏有结论句。

**执行**：.133 repl 0.2.25 单探针，cwd=~/hearth-tui。日志：`~/fa/r5-ga-userphrase.log`。

**实测**：
- `.go` 搜索次数 = **0**（`grep -cE "go\b|\.go"` 原文读数）——首轮 `ls -la` +
  `git log` + `read PROGRESS.md/交接文档`，直接命中真实项目结构 ✅
- 首屏结论句 ✅：回答含项目结构全景、"可立即运行"结论、"下次建议"（修复 8 个
  测试失败等）——有事实有结论，非"搜不到即放弃"
- 零写盘 completed（18 步）+ 收尾三行 ✅（R5-11/R3-1 真机联动）

**定位**：这是**单探针预检**，佐证 R5-4/R5-11 真机生效；正式 G-A 门跑测与砺判读
仍按总验收流程独立复跑（验收窗独立复现原则不变）。

---

## 引擎身份声明（铁律）

- .133 部署：`r5-engine-0.2.25.tar.gz`（本地 `git archive a4a2f91`）→ 解压后
  grep 源码标记（R5-7 注释在位）+ Cargo.toml version=0.2.25 验证 + touch 全部
  .rs（防 mtime 缓存）→ `cargo build --release -p codex-cli` → `--version` 必验。
- 跑测一律 `~/codex-r4/target/release/hearth` 绝对路径（PATH 旧版 0.2.23 陷阱）。
