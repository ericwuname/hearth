# 治理审计：v12.7（守门员独立抽查）

> 方法论：code-audit-gatekeeper —— 不信报告信源码，grep+Read 逐文件核实；测试必须能失败；生产路径必须实际接线；硬指标实算。
> 审计对象：已提交 commit `dee84bf` + tag `v12.7`（57 files, +1253/−129）。
> 配套：CHANGELOG-v12.7.md（技术留档）、REPORT-v12.7.md（总结报告）。

---

## 一、阶段总览

| 阶段 | 实现率 | 🔴 | 🟡 | 🔵 | 结论 |
|---|---|---|---|---|---|
| v12.7 基准跑通 | 100% | **0** | 1 | 1 | **过闸** |

过闸条件：`🔴=0 且 实现率 ≥ 0.9` → **满足**。

---

## 二、硬指标（实算，非抄报）

- `crates/`：54 个 `.rs` / **20,898 LOC** / **183 个 test 属性**（正则 `#[(tokio::)?test]` 自算，与自报 183/0 一致）
- 真 Linux 三门：`cargo fmt --all --check` = OK / `cargo clippy --workspace --all-targets -- -D warnings` = 0 / `cargo test --all` = 183 passed, 0 failed

---

## 三、逐文件抽查（不信报告信源码）

### 1. `record_tool_exchange` —— 治 grep 死循环（loop.rs）
- 定义 `loop.rs:777`；**生产调用 `loop.rs:1228`**（`do_act` 内 `self.pending_results` 落定后）—— 生产路径真调用 ✅
- 按下标配对：`for (i, call) in calls.iter().enumerate()` + `results.get(i)`（`:794–795`），注释明言「不信任 `ToolResult.call_id`（orchestrator 会改写）」✅
- `tool_call_id` 经 `msg.meta.source = Some(call.call_id.clone())`（`:812`，llm-openai 读此字段）✅
- 真回写历史：`turn.messages.push(assistant_msg); turn.messages.extend(tool_msgs)`（`:816–819`）✅
- 判定：**真接线、非空壳**。

### 2. `read_only_view` —— 治子智能体并发写冲突（dispatcher.rs / loop.rs）
- `MUTATING_TOOLS`（dispatcher.rs:46）= `["write_file","edit","apply_patch","bash"]` —— **含 `bash`**（防 `sed -i`/`cat >` 绕过）✅
- `read_only_view`（dispatcher.rs:99）：`filter(|(name,_)| !MUTATING_TOOLS.contains(&name.as_str()))`（:104）—— 真过滤 ✅
- `spawn_sub_agent`（loop.rs:528）：`let dispatcher = Arc::new(self.scheduler.dispatcher().read_only_view());` —— **生产路径真用只读分发器** ✅
- 判定：**架构级修复真落地**。

### 3. 测试 `test_read_only_view_strips_mutating_tools`（dispatcher.rs:297）
- `assert!(ro.has(allowed))` / `assert!(!ro.has(denied))` / `assert_eq!(ro.len(), 3)`
- 关键真断言：`ro.dispatch("write_file", ...).await.unwrap_err()` 且 `err` 含 `"tool not found"`（:315–319）—— **能失败的测试**（若未剥离，dispatch 成功不会 err，测试即失败）✅
- 断言父 dispatcher 不受影响（:321+）
- 判定：**真断言、非空壳**。

### 4. T15 测例自洽化（bench/tasks/T15-add-bench/）
- `goal.txt`：新建 `benches/bench.rs` + 注册 `[[bench]]` + **保留** `src/lib.rs`
- `verify.sh`：`grep sum_range` / `grep sum_formula` / `grep '\[\[bench\]\]'` + `cargo test` 全绿 —— 客观裁判，防蒙混 ✅
- 判定：**自洽、可客观裁判**（原「移走 lib 却要 cargo test 过」的矛盾已消除）。

---

## 四、偏离率

- 🔴 **0**：无接口/trait 违规、无零断言测试、无未接线（库建好且生产入口真调用）。
- 🟡 **1**：`COMPILER ERRORS` prompt 指引为降偶发率（锦上添花，非阻塞，非产品缺陷）。
- 🔵 **1**：T19 偶发泛型约束缺失（model 偶发，非产品 bug；同 budget/provider 重跑即 `VERIFY_PASS`）。

---

## 五、闸门判定

**过闸**（🔴=0 且 实现率 ≥ 0.9）。v12.7 已 `commit dee84bf` + `tag v12.7` 备份，可交付/归档。

---

## 六、下一步走法（交用户裁决）

经守门员抽查，建议优先级：

1. **稳定性（可选，🟡）**：再跑一遍 `runner.py batch --runs 1`（~25min）拿单遍 100% 报告。当前证据已强（T10/T15 全量稳定 PASS、T19 重跑 PASS），非必须。
2. **基因表达系统（高杠杆）**：`experience` 记忆仍单层无持久化、`subconscious` 部分信号硬编码（历史盘点结论）。下一阶段建议优先把「复盘编译成 wiring 断言 + experience 持久化」落地，让本次修复沉淀为机器必然而非文档愿望。
3. **接线断言锁**：为 `record_tool_exchange` 下标配对、`read_only_view` 剥离补 `wiring.yaml` 断言测试，防未来无声回归（constitution 式回归前车之鉴）。
4. **宪法级债务**：`constitution.md` 运行时不读（v11.4 改硬编码）、`civ` 自动触发连续版本缺失等历史债务仍在，需决策「真接或删」。

> 执行方自报完成已验证：v12.7 核心逻辑全部跑通（通过率 0% → 95% 单遍 / 100% 重跑），三处真根因修复均经源码核实为真接线、非空壳、测试有真断言。
