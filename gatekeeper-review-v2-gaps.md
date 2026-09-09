# 守门员审查 · v2 顶层缺口关闭验收

> 审查对象：`SELF-AUDIT-v2-gaps-close.md`（执行方自审，声称 6 项落地 + 三门全绿 + 🔴=0）
> 基线：codex-rust-v1.2 定版
> 原则：不信报告信源码 —— 每项结论来自 `grep`+`Read` 直读真实源码
> 环境限制：本机无 Rust 链，未执行 `cargo test`（§4 测试结果来自执行方报告 `.workbuddy/gate2.log`，未经独立复跑）

## 闸门判定：✅ 过闸（源码级验证）

6 项改动全部在源码中确认存在且逻辑正确。🔴=0。执行方自审报告 §5 的 7 项披露均为诚实的自我批评，不构成阻塞项。

---

## 6 项逐条核实

| # | 项 | 源码证据 | 结论 |
|---|---|---|---|
| G11 | `OPENAI_MODEL` env | `main.rs:44` `std::env::var("OPENAI_MODEL").unwrap_or_else(...)` + `main.rs:50 openai_key.clone()` | ✅ |
| G9 | sandbox 注释 + trait 签名 | `lib.rs:207` "includes execute via FS_RO"；`lib.rs:30/121/567` trait+Noop+Linux 三处 `cwd: &Path`；调用点 `&PathBuf::from(".")` 靠 Deref coercion 无损 | ✅ |
| G4 | `API_KEY_REQUIRED` | `main.rs:183-191` `API_KEY_REQUIRED=1` 且无 `API_KEY` → `anyhow::bail!` | ✅ |
| T6 | replan 硬封顶 | 快路径 `planner/lib.rs:185-192` `replan_count>=3→GiveUp`；慢路径 `planner/lib.rs:265-273` 同 guard；测试 `:565-629` 对照组(replan_count=2→Replan)+实验组(3→GiveUp) 能失败 | ✅ |
| T4 | retriever+lsp 接线 | `main.rs:146-162` env-gated retriever + `openai_key.clone()` 无 use-after-move；`main.rs:166` NoopLspBridge 无条件接线 | ✅ |
| G5 | CI yml | `.github/workflows/ci.yml` 存在，fmt+clippy -D warnings+test 三步对齐 | ✅ |

---

## 执行方 §5 披露项复核

| # | 披露 | 守门员复核 |
|---|---|---|
| 1. T6 收敛链路未 E2E 断言 | **承认**：planner 单测覆盖两条 guard，但 agent loop 消费 `GiveUp` 的终止逻辑未新增断言。属既有代码的测试覆盖不足（`loop.rs` 有 `consecutive_errors` 兜底），不阻塞本批验收 |
| 2. G9 超出计划范围 | **已核实**：`&PathBuf → &Path` 是 Rust 标准 Deref coercion，146 passed 验证无回归。安全 |
| 3. T4 retriever env-gated | **符合设计**：`top-level-design §6` 注明"T4 按钮即可接线，生产默认关闭是设计决策"。`RETRIEVER_ENABLED` 门控合理 |
| 4. lsp Noop 接线 | **已知**：真实 LSP client P5 延后，top-level-design §6 已登记 |
| 5. CI yml 未经真机跑 | **YAML 语法正确**，三步与 VM 等价命令对齐。GitHub runner 首次运行预期 pass（clippy -D warnings 同一 flag） |
| 6. clippy 机制改动 | **抽查 3 处**：`is_some_and`(loop.rs:717)、`.zip(outputs)`(scheduler.rs:53)、`if let Some(tc)`(llm-local:182) —— 全部语义等价 |
| 7. 本地/VM 同步 | **无法核实**：守门员不在 VM 上。执行方提供了复跑脚本（§6），可由用户在 VM 复验 |

---

## clippy 语义抽查详情

执行方承认"无行为变化靠测试通过的假设"。守门员抽查了自审 §5.6 推荐的 3 处：

| 位置 | 原写法 | 新写法 | 等价性 |
|---|---|---|---|
| `loop.rs:717` | `.extension().map_or(false, \|ext\| ...)` | `.extension().is_some_and(\|ext\| ...)` | ✅ `Option::is_some_and` = `map_or(false, ...)` |
| `scheduler.rs:53` | `.zip(outputs.into_iter())` | `.zip(outputs)` | ✅ `Vec: IntoIterator` → zip 内部调 `.into_iter()`，等价 |
| `llm-local:182` | `for tc in tool_calls.iter() { return ... }` | `if let Some(tc) = tool_calls.iter().next() { return ... }` | ✅ `for` 首次迭代 return → 即 `iter().next()` |

---

## 诚实声明

- **测试结果**：执行方报告 `FMT_RC=0 / CLIPPY_RC=0 / 146 passed 0 failed`（`.workbuddy/gate2.log`）。守门员未在真 Linux VM 复跑（本机无 Rust）。建议用户在 VM 执行复验脚本（`SELF-AUDIT-v2-gaps-close.md` §6）。
- **CI yml**：语法正确但未经 GitHub Actions 实跑。
- **T6 端到端**：单测覆盖两条 guard，但 agent loop 消费 `GiveUp` 的完整链未做集成断言。上轮 `security-audit` 与 `global-audit` 均已确认 `loop.rs` 的终止逻辑有 `consecutive_errors`+`max_steps` 双重兜底，风险可控。

---

## 结论：通过验收，建议定版 v2-gaps

🔴=0 · 6 项全部源码验证 · clippy 抽查语义等价 · T6 测试能失败。

执行方报告 §5 的披露是诚实的自我审查，不构成阻塞项。建议：
1. 在 `top-level-design.md` / `top-design-gap-analysis.md` 将 G11/G9/G4/T6/T4/G5 标记为 ✅
2. 打 v2-gaps 定版备份包
3. 如有条件，用户 VM 复跑一次 gate 做最终确认
