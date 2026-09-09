# 基准测试缺陷清单（给执行窗口 / 任务书）

> **来源**：codex-rust 双引擎红队基准。证据 = 160-run deepseek 马拉松（`deepseek-v24.jsonl`）+ 400-run zhipu 马拉松（`zhipu-v24.jsonl`）+ 10-run 现场 SSE 抓取实录（`captured_local/run_*.json`）。
> **角色边界**：本文档由**测试/守门角色**产出，**只诊断、列问题、给排查方向，不含实现代码**。执行窗口据此派工修改。
> **无关项**：本清单剔除测试侧工具自身 bug（如抓取脚本 verify 用 `shell=True`+`source` 在 dash 下失效，已自修），只列 agent / harness 真实缺陷。

---

## P1 — Agent 指令遵循：不遵守「精确符号名」硬约束（最高优先）

**现象**
- T19-merge-duplicate 要求实现名为 `parse_positive` 的**泛型函数**，agent 稳定不遵守函数名。
- verify.sh 第一步 `grep` 该函数名即失败，判 `NO_GENERIC_FN`。

**证据（复现稳定）**
- 160-run 马拉松：T19 = **0/8**（8/8 全 `NO_GENERIC_FN`）。
- 10-run 实录：T19-merge-duplicate run0/run1 均 `success=False`，`phase=done`（agent 跑完了，不是 abort）。
- captured run_T19-merge-duplicate_0.json：`verify` 字段含 `NO_GENERIC_FN`。

**性质**
- 这是**真实能力/指令遵循缺口**，非 API 抖动（phase=done，agent 完成了任务只是命名不对）。

**建议排查方向（执行窗口定实现）**
1. 任务 `goal.txt` 是否足够强调「必须严格使用指定符号名」？是否应在任务描述里把函数签名写死并加粗/加约束标记。
2. agent 系统提示词 / planner 是否缺「严格匹配题目要求的标识符名称，禁止自创/改名」的硬约束。
3. verify.sh 仅靠 `grep` 名字是否过脆（例如大小写/泛型参数写法差异导致误判）——可考虑同时校验行为（调用该符号的行为测试）而非仅名字。

---

## P2 — 难任务稳定性：T13-fix-index 抖动

**现象**
- T13-fix-index（中等偏难）通过率明显低于其它任务，且有 run 间抖动。

**证据**
- 160-run 马拉松：T13 = **3/8（37.5%）**。
- 10-run 实录：T13-fix-index run0 `success=True`、run1 `success=False`（`TEST_FAIL`，测试未过）。

**性质**
- 真实能力边界问题，非环境/API。其余 18/20 任务在 deepseek 上 ≥7/8。

**建议排查方向**
- 单独拎 T13 做失败样本归因：是任务描述歧义、还是 agent 在特定代码模式（index/边界）上易错。
- 优先于 P1 之后处理（影响面小，仅 1 个任务）。

---

## P3 — 执行环境：agent sandbox 内 `cargo`/`rustc` 不在 PATH

**现象**
- agent 在 sandbox 内执行 `cargo test` 反复失败（`cargo: 未找到命令` + landlock 警告），把步数耗在「修环境」上，无法自测改动。

**证据**
- captured run_T00-smoke_0.json 时间线：agent 连续 3 次 `bash cargo test` 失败 → `reflection=replan` → 最终 `budget exhausted`（25 步用尽）。但代码本身写对了（service 侧 verify `success=True`）。
- 9/10 实录 run 终态为 `budget exhausted`，主因即是 sandbox 内 cargo 不可达导致的环境折腾。

**影响**
- agent 无法自测自己的改动，只能盲改 + 依赖 service 侧 verify；真实马拉松里虽不影响最终判分，但浪费 agent 步数、降低自检能力、恶化长任务表现。

**建议排查方向**
- sandbox 启动时为 agent 预置 `export PATH="$HOME/.cargo/bin:$PATH"`（或在 sandbox 配置里固化 Rust 工具链路径）。
- 或 verify 环节不依赖 `source ~/.cargo/env`，改用绝对路径调用 cargo。

---

## P4 — 可审计性：基准不落盘 session transcript

**现象**
- 跑完 560 runs（160 deepseek + 400 zhipu），agent 真实对话（思考 / 工具调用 / 代码 diff）**一个都没持久化**。service 重启即丢。

**证据**
- `~/codex_work/sessions/` 仅 6 个残留文件，对不上 560 runs。
- `crates/service/sessions/` 有 240 文件，但不 1:1 对应 deepseek 的 160 run（sid 不匹配）。
- 本次 10-run 实录是**临时抓取 SSE 流**才得到的，基准自身不存。

**影响**
- 无法事后审计 agent 行为、无法精确定位失败根因（只能看最终 `success` + `verify_tail`）。P1/P3 的根因定位因此被迫走迂回路径。

**建议排查方向**
- runner / service 在每个 run 结束时，把 SSE 事件流（含 goal / 所有 tool_call+result / reflection / verify）落盘到 `results/transcripts/<session_id>.jsonl`，与 `results/raw/*.jsonl` 同源。

---

## 已知但非缺陷项（仅记录，不派工）

- **Zhipu 14.5% 是可靠性数字非能力数字**：400-run 里 342/400=85.5% 为 `phase=error`（免费通道掉连接 abort），与能力无关。这是外部免费通道的环境限制，非本项目 bug。若要 zhipu 数字可信需换稳定通道或剔除 error 样本。
- **T19 是唯一双方共有的真能力缺口**（zhipu 0/20 + deepseek 0/8），已并入 P1。

---

## 附件（绝对路径，可直接打开）

- 对话实录报告（10 run 可折叠轨迹）：`C:\Users\87465\Desktop\codex-rust-v1.0-final\.workbuddy\captured-report.html`
- 原始 run 数据（json，10 个）：`C:\Users\87465\Desktop\codex-rust-v1.0-final\.workbuddy\captured_local\`
- VM 同源：`/home/wutao/captured/run_*.json`
- 对比终版报告：`docs/bench-zhipu-vs-deepseek-2026-08-06.md`
- 数据路径说明：`docs/bench-data-paths-2026-08-06.md`
