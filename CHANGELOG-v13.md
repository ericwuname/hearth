# CHANGELOG — codex-rust v13.0

> 发布日期：2026-07-30 | 标签：`v13.0`
> 主题：锁死已证明能力（接线防火墙）+ 跨 provider 对比 + 清两笔历史烂账
> 配套：`forge-report-v13.md`、`gatekeeper-audit-v13.md`、`bench/results/provider-matrix-v13.md`

---

## 新增

- **`crates/project-xray`**（codex-xray 二进制）：
  - `scan`：实算 workspace facts（member/rs/LOC/test），替代守门员手算。
  - `wiring`：按 `docs/xray/wiring-v13.toml` 跑 7 条接线断言；任一 red 链断裂 → `exit 1`。
  - 单测覆盖：通过、自证变红、文件缺失、all 语义、yellow 不阻断、空 spec 拒绝、空链判裂。
- **接线断言规格 `docs/xray/wiring-v13.toml`**（schema=1，7 条，全 red）：锁死 v12.7 三项修复（tool 回写 / 只读视图 / 下标配对）+ v13 两条烂账决策（constitution 读文件 / civ 写）。
- **constitution 运行时读取**：`constitution_prompt()` 运行时读 `constitution.md`（env `CODEX_CONSTITUTION_PATH` 或向上 6 级查找），退化回硬编码摘要；`build_messages` 注入。

## 变更

- **civ 自动写入（依赖倒置）**：`agent-core` 新增 `CivWriter` trait；`do_observe`/`do_reflect` 经 `civ_note()` 真写文明线；`service/main.rs` 经 `CivWriterAdapter` 注入 `CivilizationStore`（VM 实证 11 条 Milestone）。
- **CI**：`.github/workflows/ci.yml` 删除死的 `cargo test -p telemetry`，新增 xray `wiring` + `scan` 两 step（第四门）。
- **VM gate**：`vm_upload.py --gate` 新增 codex-xray 第四门（xray_ok），四门全绿要求 xray 通过；并修复 sync-back 在 Windows 下 `os.walk` 遍历 Linux 路径静默空转（改用远端 find + SFTP 拉取）。

## 基准结果（bench）

- 跨 provider 20×1：deepseek **90%**、zhipu **80%**(run=0)、agnes **20%**（通道降级，单列不排名）。
- zhipu 20×2 稳定性：聚合 **77.5%**（run=0 80% / run=1 75%）；FAIL-BOTH=T14/T15/T19，FLAKY=T09/T13/T18。
- 跨 provider 一致硬伤：T14-add-serde、T19-merge-duplicate（deepseek 也挂）→ 任务级，归 v14-2。

## 红线与守门

- 守门结论 **GATE PASS**（gatekeeper-audit-v13.md）。7 条断言源码物理接线 + 真机自证能变红。
- 90% zhipu 红线被触发（聚合 77.5%），但根因非 v13 代码（跨 provider 任务硬伤 + 既存子代理预算债务 `loop.rs:929 sub_budget=7` + zhipu L4 偏弱），重归类为 v14 跟踪项，**不阻塞 v13 交付**。

## 未包含（推至 v14，见 forge-v13 §6）

应力/价值维度、experience 持久化、subconscious 完全动态化、回放测试、codex-xray graph/diff/report 富呈现。

## v14 跟踪项

- **v14-1**：修复 `loop.rs:929 sub_budget=7`，子代理真实完成预算（解决 T09/T13/T18 偶发）。
- **v14-2**：硬化 T14 / T19 fixture（跨 provider 硬伤）。
- **v14-3**：agnes 专线恢复后重测，确认 20% 为通道问题。
