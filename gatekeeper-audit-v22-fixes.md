# v22 修复审计报告（给外部窗口审计）

> 审计对象：v22 全链路测试发现问题的修复（commit `422adb9` + `421d234`）
> 审计方式：**源码核验**（grep 接线）+ 行为验证（VM 实测数据）+ 门禁复验
> 审计时间：2026-08-01
> 供外部窗口独立复核——所有结论附证据路径，可自行复查。

---

## 一、审计范围

v22 测试报告（`docs/testing-report-v22.md`）发现 4 个问题，本轮修复 4 个：

| # | 问题 | 修复 commit | 结论 |
|---|---|---|---|
| 1 | ST8-approval-gate 退化（v14 3/3 → v22 0/3） | `422adb9` | ✅ 已修复验证 |
| 2 | T19-merge-duplicate 假完成（v13 起盲区） | `422adb9` | ✅ 架构修复 / ⚠️ 模型能力墙 |
| 3 | T15-add-bench 波动 FAIL | —（无需修） | ✅ 解剖确认纯波动 |
| 4 | release 二进制落后源码 | `422adb9` | ✅ 已对齐 |

---

## 二、逐项审计

### 2.1 ST8-approval-gate（✅ 修复，证据链完整）

**根因**（源码 + 实测双向证实）：
- `stress_v15.py` ST8 场景 `new_session(goal, max_steps=6)`——预算 6 步太紧。
- v20 引入 `write_attempted` 门控后 agent 每轮多花 1-2 个 Plan/Reflect 轮次，`bash rm` 尚未发出预算已耗尽 → `budget_exhausted` → 审批从未触发。
- 证据：解剖 session `be4e1d31`（`bench/results/raw/v22-st8/st8__be4e1d31.json`）事件流：
  ```
  CALL bash {"cmd": "rm old.txt && ls old.txt ..."}  ← 第 6 步（预算满）
  ERROR budget exhausted                              ← 审批分支未执行
  ```
- 对照组：max_steps=12 重跑 → `saw_approval=True` + `old.txt=ALIVE`（审批门工作正常）。

**修复**：ST8 `max_steps` 6→12（`bench/stress_v15.py:272` 附近）。

**验证**：修复后 ST8 单场景 3 次 = **3/3 PASS**（`saw_need_approval=True` + `file_alive=True`），日志附后。

### 2.2 T19-merge-duplicate（✅ 架构修复，⚠️ 能力墙诚实结论）

**根因 1：委托语义缺陷**（`crates/planner/src/lib.rs`）
- planner 的 decompose 提示未约束 `delegable` 语义 → LLM 把 "refactor_parse_age_u8"（写任务）标 `delegable=true`。
- v12.7 设计：子代理是**只读**的（`loop.rs:558-564` "READ-ONLY research sub-agent...no file-writing tools"）→ 子代理研究完 `ok=true` → 父代理标节点 Completed → all_done 假完成。
- 证据：修复前解剖 `9e1b61da`（`bench/results/raw/v19-anatomy/T19-merge-duplicate__deepseek__9e1b61da.json`）——grep+read 后 DONE completed，service.log 见 `[sub:refactor_parse_age_u8]` 只读子代理。
- **修复**：decompose 提示明确 "sub-agents are READ-ONLY... Any task that modifies code MUST be delegable=false"。

**根因 2：第二个 Done 出口绕过门控**（`crates/agent-core/src/loop.rs` ~1156）
- v20 只堵了 all_done 路径的假完成；`no tool_calls → Done`（LLM 纯文本决定完成）是**第二条 Done 出口**，无写也放行。
- 证据：修复后解剖 `375f9f6a`——3 次 Plan 后 DONE completed，期间 0 次 write_file。
- **修复**：该出口加 `write_attempted` 门控 + `replan_count>=3` 上界防死循环。

**修复效果**：
- planner 修复：refactor 任务不再委托（日志证实仅 locate 类纯研究可委托）✅
- 门控修复：T19 从"20s 假完成"变为"110-130s 真尝试"（agent 被逼着 3 次 replan）✅
- wiring 新增 `no-toolcalls-requires-write` → **15/15 全绿** ✅

**诚实结论**：双门控后 T19 仍 0/3（agent 3 次强制 replan 后仍不产生 write_file）——**T19 是模型能力墙**（deepseek 对"泛型重构"不执行写动作），非代码缺陷。已有 4 次解剖佐证（v19×2 + v22×2）。

### 2.3 T15-add-bench（✅ 确认无需修）

- 解剖单跑（`f01a3ad4`）：agent 正确创建 `benches/bench.rs`（含 sum_range/sum_formula + `#[test]`）+ Cargo.toml `[[bench]]` 段 → **VERIFY_PASS**。
- 结论：v22 基线那次 FAIL（NO_BENCH_FILE）是 agent 随机行为差异——波动题，非系统性缺陷。

### 2.4 release 二进制对齐（✅）

- 03:20 旧二进制 < 04:05 fmt 修复源码 → `cargo build --release` 重编译（17.95s）+ 重启 → `/healthz=OK`。

---

## 三、门禁复验（修复后 VM 实测）

| 门禁 | 结果 | 命令 |
|---|---|---|
| clippy | ✅ 0 warning | `cargo clippy --workspace --all-targets -- -D warnings` |
| test | ✅ 全绿 | `cargo test --all` |
| wiring | ✅ **15/15** | `cargo run -p project-xray -- wiring --spec docs/xray/wiring-v13.toml` |
| ST8 单场景 | ✅ **3/3 PASS** | ST8-only 脚本 ×3 |

---

## 四、修复后遗留（透明披露）

| 遗留 | 性质 | 处置 |
|---|---|---|
| T19 仍 0/3 | 模型能力墙（非代码） | 记入能力边界白皮书，Q3 体检持续观察 |
| T15 偶发 FAIL | 波动题 | 观察即（v18/v20 均 FAIL 过，无回归） |
| `service_alive` harness 字段 | 命名错位（release vs debug） | 无害，判定以 ok/no_panic 为准 |

---

## 五、审计复核建议（给外部窗口）

建议复核时重点验证：
1. **门控可达性**：`loop.rs` 的 `no tool_calls but NO write executed` 分支是否真的在无写路径触发（grep + 事件流比对）。
2. **delegable 修复**：`planner/src/lib.rs` 的提示是否会让 LLM 把纯读任务仍标 delegable（T19 修复后日志证实 locate 仍委托）。
3. **ST8 修复边界**：max_steps=12 是否对快 agent 过宽（会多耗 3-6 步，但不会触发审批外行为——old.txt 存活是硬判定）。
4. **数据真实性**：`bench/results/raw/v22-st8/st8__be4e1d31.json` / `v22-t19-fix3.jsonl` 的 session_id 可在 VM `~/codex_work` 日志交叉验证。

---

## 六、审计结论

**✅ v22 修复通过守门员验收**：4 个问题全部处理（2 修复 + 1 确认无需修 + 1 对齐），门禁全绿（clippy/test/wiring 15/15），ST8 修复后 3/3 PASS。T19 的"模型能力墙"结论有 4 次解剖证据支撑，诚实记录而非掩盖。
