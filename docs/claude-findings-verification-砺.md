# Claude 外部评审发现 · 源码核实裁决书（砺·评审）

> 对应材料：`docs/hearth-manual-test-incremental-findings-v1.md`（Claude 回函）
> 核实原则：**不信报告信源码**；每条发现给 VERDICT + 源码锚点 + 实证。
> 复核环境：本地 HEAD = v0.2.9 工作区源码；实证在评审 VM `.133`（v0.2.8 二进制，相关 crate 未变）`cargo test -p tool-runtime -p planner -p sandbox` → **44 passed / 0 failed**。

---

## 🚨 0. 明文密钥泄漏（操作安全紧急事件，已拿铁证）

**VERDICT：VERIFIED，且比 Claude 判断更严重。**

Claude 认为"git 提交密钥扫描没覆盖日志文件"。实测恰恰相反：

| 证据 | 实测结果 |
|---|---|
| `.gitignore` 覆盖日志？ | **否**——只忽略 `release/*.tar.gz` 与 `*.log`，**未忽略 `release/*.txt` / `*.md`** |
| 含明文 key 的日志是否进 git？ | **是**——`git ls-files release/` 显示 `hearth-manual-test-logs-ALL.txt`（1.6MB）等已被跟踪 |
| 是否已 commit？ | **是**——`git log` 显示提交 `059f160`（"v0.2.8 权威门禁 375 passed"） |
| 是否推到远程？ | **暂未**——`059f160` 不在 `origin/main`（`f19db66...`），本地泄露提交尚未推送 |

已 grep 实锤 `release/hearth-manual-test-logs-ALL.txt` 含 **6 个不同明文 key**，其中 **`cpk-f4UBH3NaHUUN2SAcW1LhZT0kyuqlu9TF5lTItbuXHSTP3w0c`（你当前在用的活跃 Agnes key）命中 3 次**（行 10801/10802/25401）。

**窗口期判断**：密钥已提交在本地、remote 已配置（`github.com/ericwuname/codex-rust-v1.0-final.git`）但**当前泄露提交未推送**——趁未 push 是最后阻断窗口。

**必修动作（不可拖延）**：
1. **立即轮换全部 6 个 key**（尤其活跃 Agnes `cpk-f4UB...` 与旧 Agnes `sk-8LBZ...`）——用户侧控制台操作，AI 无法代做。
2. **阻断推送**：在清理完成前禁止 `git push`。
3. **清理历史**：`git filter-repo` / BFG 擦除这些文件中的 key 后强制推送（**改写历史=不可逆跨边界操作，需你显式确认后由执行窗口执行**）。
4. **补 .gitignore**：加入 `release/*.txt`、`release/*手工测试*.md`、`release/hearth-manual-test-*`、`docs/*findings*` 等，并加一份"手工测试日志"专用密钥扫描（不只扫 git 提交）。

> 守门员不动 git 历史（不可逆），上述 3/4 待你拍板后执行。

---

## 1. 审批 fail-open 疑点（Claude 标为"唯一动摇可信委托根基"）

**VERDICT：REFUTED（源码推翻最坏假设）。** 但发现一个旁证偏差（RC28）。

**源码依据（双路径）**：
- **CLI REPL 路径** `crates/codex-cli/src/repl.rs:171-188`：审批等待 `timeout(30s, rx.recv())`；超时走 `_ =>` 分支执行 `submit_approval(&sid, aid, false, None)`（**auto-deny**），并打印 `"approval timeout — auto-denied"`。**明确 fail-closed。**
- **服务模式路径** `crates/agent-runtime/src/session.rs`：仅 `resolve_interaction` 由客户端显式调用翻转状态；全仓 `grep auto_approve|yolo|auto_allow` **零命中**——**不存在无人应答自动放行的代码路径**。无人应答时 interaction 保持 `Pending`（阻塞，不自动放行）。

**实证**：`.133` 单测 `test_interaction_deny` / `test_interaction_flow` / `test_interaction_isolated_between_sessions` / `test_wp0a_r1_wrong_interaction_id_rejected` / `test_wp0a_r2_double_submit_rejected` 全绿 → dispatcher 层无任何 auto-approve，拒绝/隔离/防重放均成立。

**对 Claude 三个假设的裁定**：
- 假设 #3（超时默认放行）→ **不成立**，源码为默认拒绝。
- 假设 #1（用户其实没真离开、手动点了 y）→ **最可能是真相**：67.9s 后"批准"只能由真人 `approve` 触发（REPL 30s 早已 auto-deny，服务模式不自动 resolve）。
- 假设 #2（用不同超时配置）→ 部分成立但反向：**证据包 C.1 声称的 60s 实为 30s**（repl.rs:173），即 Claude 引用的证据包本身有偏差（见 RC28）。

**结论**：审批机制在源码层面 **fail-closed，无 fail-open**。Claude 的"可信委托根基"担忧可被源码+实证排除。无需构造 30s 空等复现——超时分支逻辑 `repl.rs:184-186` 即铁证，且单测已覆盖 dispatcher 拒绝语义。

---

## 2. 无人监督期间目标漂移（自建另一个项目）

**VERDICT：LOG-OBSERVED（日志事实，非源码可定责）**。Claude 给的行号 `v0.2.4:L14181`（用户原话"感觉偏离很远"）属日志实证，守门员无法从当前源码"证伪"。
**源码视角**：规划器没有"目标锁定/防替换"护栏；`decompose` 完全由 LLM 自由分解。这是**行为层缺失**，建议单独立项（不并入 replan/give_up 类）。非代码缺陷，是长程无人监督下的 LLM 行为风险。

---

## 3. 工作区访问范围过宽（可读取无关文件）

**VERDICT：PARTIAL-VERIFIED（源码确认读范围=全盘）**。

**源码依据** `crates/sandbox/src/lib.rs:74-85`：`read_only_paths` 默认 = `vec![PathBuf::from("/")]`（**整个 `/` 只读开放**，FS_RO）；`writable_paths` 默认仅 `current_dir()`。即：
- **写**：严格限定 cwd（landlock 实证 `test_p5_landlock_denies_outside_write` ✅ / `allows_workspace_write` ✅）。
- **读**：agent 的 glob/read/grep 经同一 landlock FS_RO，可读取 `/` 下任意文件——**确能命中无关 IDE 扩展文件、其他 home 项目**（Claude 三处观察成立）。

**性质**：这是**为跑构建工具（需读 /usr/bin 等）的设计取舍**，非"意外越权写"。真正的缺口是**行为层**——agent 选择 `glob /home/wutao/**` 而非问用户确认路径。源码无"读范围收紧到项目目录"的强制；属于隐私/范围卫生问题，非破坏性问题。

**新 RC 编号：RC25**（读范围 = 全盘 `/`，建议评估对 agent 的 glob/read 加"默认项目目录限域 + 越界需显式确认"）。

---

## 4. missing_goal_source 对所有任务触发，失去区分度

**VERDICT：VERIFIED（源码确认，属有意但失效设计）**。

**源码依据** `crates/planner/src/lib.rs:20-27`：`derive_gaps` 对 goal 做 `if !g.contains("来源") && !g.contains("source:") && !g.contains("based on")` → 必推 `missing_goal_source` gap。普通"写一个贪吃蛇游戏"不含这些词 → **必触发**。
**实证**：`.133` 单测 `test_derive_gaps_missing_goal_source_non_blocking` ✅ 断言它**必出**且 `non_blocking + auto_assumed`。

**结论**：该标记对几乎所有任务触发（与 Claude 统计一致），确无区分度。设计意图是"标歧义"，实现却变成"固定模板文字"。建议：仅在 goal 真正含歧义信号（多目标/或选项/指代不明）时标记，而非默认全标。

**新 RC 编号：RC26**。

---

## 5. "向用户提问"通道几乎从未被使用

**VERDICT：PARTIAL-VERIFIED（机制存在，使用率属 LLM 行为）**。

**源码依据**：通用交互机制已落地——`dispatcher` 支持 `kind="clarification"`（见 `dispatcher.rs:749` 遍历测试含 `"clarification"`）；`planner/lib.rs:16-17` 明确 `blocking=true` 的 gap "由 loop 走通用交互澄清（kind=clarification，内核不加分支）"。即**提问通道在架构上已接通**。
Claude 观察到的"待问=0 占绝大多数"是 **LLM 决策层少用**，非机制缺失。源码无法证伪其统计，但可确认：若真有 blocking gap，通道可用。属行为调优项，非代码缺陷。

---

## 6. Agent 自建工具与 hearth 主二进制 PATH 命名碰撞

**VERDICT：VERIFIED（源码确认无任何同名防护）**。

**源码依据**：`grep` tools-builtin/src 下 `.local/bin` / `bin/hearth` / `collision` / `same name` / `executable` **零命中**——写文件工具（`edit.rs` write_file）无任何"目标名是否与运行二进制同名"的检查。更糟：`crates/tools-builtin/src/bash.rs:142-162` 在 sandbox 内 **把 `cargo_bin` 前置进 PATH**（`*v = format!("{}:{}", cargo_bin.display(), v)`），若 `~/.local/bin` 或 `cargo_bin` 目录排在真 hearth 之前，agent 写入的同名 `hearth` 会遮蔽真二进制。
**实证**：`.133` tool-runtime 16 测试**无任何覆盖该场景的用例** → 该防护确属空白。

**新 RC 编号：RC27**（给"生成可执行文件"类调用加同名/敏感路径护栏；PATH 注入顺序评估）。

---

## 7. 疑似路径幻觉（n=1，低置信度）

**VERDICT：LOG-OBSERVED（n=1，无法源码定责）**。Claude 自标低置信度（Windows 路径 `C:\Users\Administrator\...` 出现在 Linux SSH 会话）。守门员保留记录、留意复现，不据此立 RC。

---

## 8. 编辑类工具偏向"整份重写"而非"最小修改"

**VERDICT：PARTIAL-VERIFIED（工具默认即整份覆盖，支持该观察）**。

**源码依据** `crates/tools-builtin/src/edit.rs:33,48`：write_file 默认 `mode="write"` = **整份覆盖**；`append` 才是增量。工具默认行为天然偏向"整份重写"，与 Claude 观察到的 `write_file` 19 次 / `apply_patch` 2 次一致。属工具默认语义，是否"因此丢内容"需抽查具体 diff（Claude 也仅推断）。建议：对"重写已存在文件"加 diff 安全校验（确认无既有内容意外丢失）。

---

## 9. DIGEST 抽取方法元发现（非 Hearth bug，分析管线 bug）

**VERDICT：META（不影响 Hearth，影响分析可信度）**。
- DIGEST 把 assistant 生成的文档行（"版本: v1.0 | 更新时间: 2024"）误判为"用户原话" → 抽取启发式失效。这是 `release/hearth-manual-test-DIGEST.md` 生成脚本的 bug，非 Hearth。
- 该行还暴露**日期幻觉**（2026 年会话却写"2024"）——与更早 `hearth_bug_log.md` 的"2026-05-19"同类，样本独立 ×2 → 升为"生成文档时易编造错误日期"类问题。建议：生成文档类内容时把系统日期显式注入上下文。

---

## 裁决汇总

| # | Claude 发现 | VERDICT | 新 RC |
|---|---|---|---|
| 0 | 明文密钥泄漏 | **VERIFIED（更严重：已 commit 未 push）** | 操作安全·紧急 |
| 1 | 审批 fail-open | **REFUTED（源码 fail-closed）** | RC28（证据包 60s≠实际 30s） |
| 2 | 目标漂移 | LOG-OBSERVED | 行为层·单独立项 |
| 3 | 工作区读范围过宽 | PARTIAL-VERIFIED | **RC25** |
| 4 | missing_goal_source 失效 | VERIFIED | **RC26** |
| 5 | 提问通道少用 | PARTIAL-VERIFIED（机制在） | — |
| 6 | PATH 同名碰撞 | VERIFIED（无防护） | **RC27** |
| 7 | 路径幻觉 n=1 | LOG-OBSERVED | 留档 |
| 8 | 整份重写倾向 | PARTIAL-VERIFIED | 工具默认语义 |
| 9 | DIGEST 抽取/日期幻觉 | META | 分析管线 |

**实证背书**：`.133` `cargo test -p tool-runtime -p planner -p sandbox` = **44 passed / 0 failed**，覆盖 #1（dispatcher 拒绝语义）、#3（landlock 读写范围）、#4（missing_goal_source 必触发）。

**守门员结论**：Claude 回函质量高、交叉盲区价值大（尤其 #0/#1/#6 是此前 DIGEST 未覆盖的角度）。#1 的"可信委托根基"担忧经源码+实证**排除**；#0 是当下最该优先处理的真实紧急事件；#3/#4/#6 是三个可落 RC 的真实代码级缺口。
