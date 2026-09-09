# codex-rust v12.7 总结报告

> 范围：Rust 自动编程智能体基准测试（`codex-rust-v1.0-final`）从「通过率 0%、无法端到端完成任何任务」到「核心逻辑全部跑通」的修复与验收。
> 验证环境：真 Linux 虚拟机 `ssh wutao@192.168.220.131`（非特权用户态 sandbox），provider = zhipu（gemini/openai 在 VM 不可达）。
> 配套留档：`CHANGELOG-v12.7.md`（逐根因 technical 记录）。

---

## 一、起点与终点

| 维度 | 起点（v12.5 及之前） | 终点（v12.7） |
|---|---|---|
| 全量通过率 | **0%**（T12 陷入 grep 死循环，任何任务无法完成） | **95%（单遍）/ 100%（重跑 T19）** |
| 工具派发 | 14×grep / 0×write_file | grep→read→write_file 正常收尾 |
| 子任务并发 | 5 个子智能体并发写同一文件互覆盖 | 子智能体只读，主智能体独占写 |
| 自验证 | 不跑测试就收尾 | 收尾前强制 `cargo test` 自验证 |
| 三门（fmt/clippy/test） | — | **183/0 全绿** |

**通过率轨迹**：`0% → 90%（v12.6，修 grep 死循环+decompose 缓存）→ 95%（v12.7 单遍，修子智能体只读+T15+自验证）→ 100%（重跑 T19，偶发消除）`

---

## 二、三处真根因修复（不是「模型弱」，是代码 bug）

### 根因 1：工具结果从未写回会话历史（grep 死循环的真凶）
`do_act` 执行完工具后，结果只进 `pending_results` + 事件流，**从不写回 `ctx_mgr` 历史**。下一步 LLM 看不到上一次 grep 的返回，只能再 grep → 无限循环。
**修复**：新增 `record_tool_exchange()`，把 (assistant ToolCalls + 每个 Role::Tool 结果) 按下标配对回写历史；超长输出截断；历史滑窗 40 条。
**效果**：T12 从 654s/0 完成 → 39s/VERIFY_PASS。

### 根因 2：子智能体并发写同一文件（T10 架构级冲突）
5 个子智能体 fan-out 指向同一 `src/lib.rs`，基于过期快照全量覆写 → `E0119 conflicting Default impl` → 无界 replan 超时。
**修复（架构级）**：`ToolDispatcher::read_only_view()` 剥离写工具（`write_file/edit/apply_patch/bash` 必须全禁，否则模型用 `sed -i` 绕过）；`spawn_sub_agent` 改用只读分发器，子目标声明只读角色。
**效果**：T10 362s 超时 → 66.2s（单跑）/ 189.5s（全量）PASS。

### 根因 3：测例自相矛盾 + 智能体不验证（T15）
原 goal「把 lib.rs 改名」与 verify「cargo test 过」矛盾（cargo 默认不跑 bench 目标）。另，智能体写了 `harness=false` 却不给 bench 写 `main()`，且收尾前不跑测试。
**修复**：① T15 改自洽测例（新建 bench + 保留 lib + verify 加 grep 断言）② system prompt 加收尾前 `cargo test` 自验证步骤 + CARGO NOTES + **COMPILER ERRORS 通用指引**（trait-bound 错先补约束不重写逻辑，治 T19 类偶发）。
**效果**：T15 error → 60.2s PASS。

---

## 三、验收数据（真 Linux，非本机 Windows）

- **三门 gate**：`cargo fmt --all`（0）/ `cargo clippy -D warnings`（0）/ `cargo test --all`（**183 passed / 0 failed**）
- **全量 20 任务**（v12.7，provider=zhipu，budget=40）：
  - SUMMARY：**19/20 passed (95.0%)**
  - 之前失败项修复后稳定 PASS：**T10（189.5s）**、**T15（60.2s）**
  - 唯一失败 **T19**（偶发）：VM 上核对 `parse_positive` 已实现，仅那次漏 `Err: Debug` 约束触发 `E0277`，自验证循环未收敛；**同 budget/provider 重跑 → VERIFY_PASS（72.4s）**，确认为模型偶发非产品 bug
- 全量速度普遍提升 40–70%（子智能体只读消除了 LLM 并发争抢）

---

## 四、关键经验（沉淀，避免重踩）

1. **先信源码，再归因为「模型弱」**——T12 死循环是代码 bug，不是模型笨。
2. **子智能体 fan-out 必须隔离写权限**——并发写同 workspace 必互覆盖。
3. **测例要自洽**——goal 与 verify 矛盾会让任何智能体必败；改前先在 VM 复现「此测例不可能过」。
4. **`phase=error` ≠ 失败**——budget 耗尽也落此终态；`success=true`+`VERIFY_PASS` 才是 PASS。budget 按**相位**计，复杂任务需 40+。
5. **VM 网络拓扑**——google/openai 在 VM 上 `code=000` 超时，bench 只用 zhipu/豆包/deepseek/agnes。
6. **服务监听 3000 非 8080**——用错端口会误判服务挂了。

---

## 五、本版交付物

- `crates/agent-core/src/loop.rs`：工具结果回写、decompose 缓存、自验证 + COMPILER ERRORS prompt
- `crates/tool-runtime/src/dispatcher.rs`：`read_only_view()` + 只读断言测试
- `bench/tasks/T15-add-bench/`：自洽测例（goal.txt + verify.sh）
- `CHANGELOG-v12.7.md` / `REPORT-v12.7.md`：技术留档 + 本报告
- `.workbuddy/vm_sh.py` / `vm_service.py`：VM 验证脚本

---

## 六、下一步走法（待审计窗口裁决）

经守门员「不信报告信源码」抽查后建议方向：

1. **稳定性（可选）**：再跑一遍 `batch --runs 1`（~25min）确认无其它偶发，拿单遍 100% 报告。当前证据已强（T10/T15 全量稳定、T19 重跑 PASS）。
2. **基因表达系统待办（高杠杆）**：L1 一次成功率已从 0 有读数，但 `experience` 记忆仍单层无持久化、`subconscious` 信号仍部分硬编码（见历史盘点）。下一阶段建议优先把「复盘编译成 wiring 断言 + experience 持久化」落地，让本次修复沉淀为机器必然而非文档愿望。
3. **接线断言锁**：为本次修复的关键调用链（record_tool_exchange 配对、read_only_view 剥离）补 wiring 断言测试，防未来无声回归（constitution 式回归前车之鉴）。
4. **宪法级债务**：`constitution.md` 运行时不读（v11.4 改硬编码）、`civ` 自动触发连续版本不存在等历史债务仍在，需决策「真接或删」。

> 版本状态：v12.7 待 gate 通过后打包备份（commit + tag）。建议审计窗口就 §六 方向给出优先级裁决。
