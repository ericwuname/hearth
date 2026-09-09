# 自审报告：顶层缺口关闭 v2（handoff-top-gaps-close-v2）执行闭环

> 审计对象：`codex-rust-v1.2` 之上的 6 项缺口关闭（G11 / G9 / G4 / T6 / T4 / G5）+ 6.0 存量 fmt/clippy 清理
> 审计人：本窗口（执行者自审）
> 审计日期：2026-07-28
> 核心原则（守门员铁律）：**不信报告，信源码**。本报告所有结论均附 `文件:行号` 与真实代码片段；交付给另一窗口时，对方**必须逐条 re-read 源码复核**，不得直接采信本报告陈述。

---

## 0. 摘要（自判结论）

| 维度 | 自判结果 |
|---|---|
| 计划 6 项功能落地 | ✅ G11 / G9 / G4 / T6 / T4 / G5 全部落地（见 §3） |
| 6.0 存量 clippy 清理 | ✅ 全 workspace `CLIPPY_RC=0` |
| 真 Linux 验证 | ✅ fmt / clippy / test 三门全绿 |
| 测试结果 | ✅ **146 passed / 0 failed**（含新增 T6 测试） |
| 原始 🔴 问题 | ✅ R1（T6 慢路径锚点）、R2（T4 use-after-move）均已闭环 |
| 实现率 | 自判 100% |
| 自检可信度 | ⚠️ **自审不可自证**。§5 列出了 7 项必须请另一窗口独立复核的点 |

证据文件：`.workbuddy/gate2.log`（`FMT_RC=0` / `CLIPPY_RC=0` / `GATE_END rc=0`，测试汇总 `TOTAL ok=146 fail=0`）。

---

## 1. 审计范围与目标

- 基线：`codex-rust-v1.2`（已定版，守门员过闸 + 三轮 VM 复验全绿）。
- 目标：关闭 `top-design-gap-analysis.md` 推荐立即做的 6 项，将 To-Be 覆盖率从 ~0% 拉到 ~55%。
- 输入计划：`handoff-top-gaps-close-v2.md`（已吸收 `review-handoff-top-gaps-close.md` 的 R1/R2/Y1/Y2/B1/B2/B3 修正）。
- 不在本次范围：`top-level-design` 其余 To-Be 项、P5 LSP 真实客户端、真实语义检索生产默认开启。

---

## 2. 验证方法论（另一窗口复验指南）

本项目铁律：**本机无 Rust 工具链，所有编译/测试一律在真 Linux VM 执行**。

- VM：`ssh wutao@192.168.220.131`（Ubuntu 24.04，landlock/seccomp 可用）。凭据见 `.workbuddy/memory/MEMORY.md` 或复用 `codex-vm-test` skill（用户已授权直接执行，无需再问）。
- 工作目录：`~/codex`（覆盖源码 + 保留 `target` 增量）。
- **致命坑（必读）**：tar 打包会保留本机编辑 mtime，若早于 VM 上次编译产物，cargo 误判"源码未变"跳过重编，从而**复用旧 target 缓存**，clippy 会原样吐出已修掉的旧错。
  - 正确姿势：解压后 `find ~/codex/crates -name '*.rs' -exec touch {} +` 强刷 mtime 强制重编。
  - 且必须用**前台阻塞**跑 `fmt && clippy && test` 并直读日志，避开后台 `nohup` 的日志竞争/陈旧问题。
- 验收命令（与 `ci.yml` 对齐）：
  ```bash
  cd ~/codex && source $HOME/.cargo/env
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --all
  ```

---

## 3. 逐项源码核验

### 3.1 G11 — OpenAI 模型名硬编码 → env 可配置（含 R2 前置修正）

计划要求：`main.rs` 用 `OPENAI_MODEL` env，默认 `gpt-4o`，打印启动日志；第 5 参必须是 `openai_key.clone()`（否则 T4 的 embed provider 会 E0382 use-after-move，即 🔴 R2）。

实际落地（已 re-read 源码）：

`crates/service/src/main.rs`
```rust
// :44-53
let openai_model = std::env::var("OPENAI_MODEL")
    .unwrap_or_else(|_| "gpt-4o".into());
info!("openai model: {openai_model}");
let openai = Arc::new(OpenAiProvider::new(
    "openai",
    &openai_model,
    openai_url,
    openai_key.clone(),          // :50  ← R2 修正点
));
```
T4 的 embed provider 同样使用 clone（`:156`）：
```rust
// :156
    openai_key.clone(),
```

**R2 闭合验证**：全库 `grep openai_key` 在 `service/src/main.rs` 仅出现 `openai_key.clone()` 两处，无任何未 clone 的 move。若删除 `:50` 的 `.clone()`，`:156` 必报 E0382，证明 R2 是真闭合而非掩盖。

吻合度：✅ 与计划一致（计划标注 `:39-44`，实际落在 `:44-53`，因前序 import/变量布局微调，语义等价）。

---

### 3.2 G9 — sandbox 注释与实现不一致修正

计划要求：`sandbox/src/lib.rs` 改 1 行注释（`but no execute` → `includes execute via FS_RO`）。

实际落地：
```rust
// crates/sandbox/src/lib.rs:207
    // Full r/w access for writable_paths (includes execute via FS_RO)
```
`grep "no execute" crates/sandbox/src/lib.rs` → 无匹配 ✅。

**⚠️ 超出计划范围披露**：本轮在 clippy 清理（6.0）中，把 sandbox trait 的 `cwd` 参数签名由 `&PathBuf` 改为 `&Path`（needless_borrow），共 3 处：
- trait 定义：`crates/sandbox/src/lib.rs:30`
- Noop 实现：`crates/sandbox/src/lib.rs:121`
- Linux 实现：`crates/sandbox/src/lib.rs:567`

调用点（`lib.rs:711/728/762/780/810/977` 的 `&PathBuf::from(".")`）靠 Rust 的 `&PathBuf → &Path` 自动 coercion 通过，`clippy` 全绿验证无回归。这属于"计划外但必要的 clippy 修复"，**请另一窗口确认该签名变更未改变 sandbox 运行语义**。

吻合度：✅ 计划内注释改动已落地；🔶 额外做了 trait 签名重构（clippy 驱动），已在 §5.2 列入复核。

---

### 3.3 G4 — API_KEY_REQUIRED 强制鉴权

计划要求：`main.rs` 在 `API_KEY` 缺失且 `API_KEY_REQUIRED=1/true` 时 `anyhow::bail!` 拒绝启动。

实际落地（`crates/service/src/main.rs:183-197`）：
```rust
    } else if std::env::var("API_KEY_REQUIRED")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false)
    {
        anyhow::bail!(
            "API_KEY_REQUIRED=1 but API_KEY is not set. \
             Refusing to start open API in production mode."
        );
    } else {
        tracing::warn!(...);  // 旧开放行为
    }
```
`bail!` 合法（`main()` 返回 `anyhow::Result<()>`，已核实）。与计划逐字一致 ✅。

---

### 3.4 T6 — replan 硬封顶（🔴 R1 修正版 + 🟡 Y2 新测试）

计划要求：两条路径都加 `replan_count >= 3 → GiveUp` 守卫；慢路径必须是真实 if-else 链（非计划的"不存在的 match 块"，即 R1）；新增可失败断言测试。

实际落地（`crates/planner/src/lib.rs`）：

常量：`const MAX_STEPS_WITHOUT_PROGRESS: u32 = 5;`（`:28`）

快路径守卫（`:182-191`）：
```rust
        if obs.steps_without_progress >= MAX_STEPS_WITHOUT_PROGRESS {
            if state.replan_count >= 3 {
                tracing::warn!("Giving up: max replan exceeded ...");
                return Ok(ReflectVerdict::GiveUp);
            }
            tracing::warn!("Replanning: .. replan #{}/3", state.replan_count + 1);
            return Ok(ReflectVerdict::Replan);
        }
```

慢路径守卫（`:261-276`，**真实 if-else 链，非 match；分支用表达式求值，禁用 `return Ok` 以免绕过下方 verdict 日志**）：
```rust
        let verdict = if verdict_text.contains("give_up") || verdict_text.contains("give up") {
            ReflectVerdict::GiveUp
        } else if verdict_text.contains("replan") {
            if state.replan_count >= 3 {            // T6 硬封顶
                tracing::warn!("LLM suggested Replan but max replan exceeded ({}). Forcing GiveUp.", state.replan_count);
                ReflectVerdict::GiveUp
            } else {
                ReflectVerdict::Replan
            }
        } else {
            ReflectVerdict::Continue
        };
```

新增回归测试（`:565-628`）：
```rust
    async fn test_t6_replan_hard_cap_forces_giveup() {
        // 快路径命中：steps_without_progress=6 >= 5，replan_count=3 → GiveUp
        // 对照组：replan_count=2 → 仍是 Replan（证明测试能失败）
        let v = planner.reflect(&obs_no_progress, &state_under_cap).await.unwrap();
        assert_eq!(v, ReflectVerdict::Replan, "replan_count=2 < cap → still replan");
        let v = planner.reflect(&obs_no_progress, &state_at_cap).await.unwrap();
        assert_eq!(v, ReflectVerdict::GiveUp, "replan_count=3 hits cap → forced give_up");
    }
```

**R1 闭合验证**：慢路径守卫落在真实存在的 if-else 链（`:261-276`），不是计划 v1 误指的 `match { Replan => }` 块。
**Y2 可失败性**：若删除 `:182` 快路径 guard，第二个断言（期望 `GiveUp`）必失败 → 测试确能失败，符合铁律。

吻合度：✅ 与计划一致；🔶 计划标注慢路径在 `:250-256`，实际落在 `:261-276`（行号因源码布局微调，逻辑等价）。

---

### 3.5 T4 — retriever + lsp-bridge 生产接线（R2/B2 修正版）

计划要求：显式导入 `NoopLspBridge` / `TantivyRetriever`；retriever env-gated 接线；lsp-bridge 无条件接 Noop；确认 setter 存在。

实际落地（`crates/service/src/main.rs`）：
```rust
// :13  use lsp_bridge::NoopLspBridge;
// :15  use retriever::TantivyRetriever;

// :158-166  retriever (env-gated) + lsp (Noop)
        let mut retriever = TantivyRetriever::new();
        retriever.set_embed_provider(embed_provider);
        sessions.set_retriever(Arc::new(retriever));
        info!("retriever wired (embed model: {embed_model})");
        // ...
        sessions.set_lsp_bridge(Arc::new(NoopLspBridge::new()));
```
对应 setter 已确认存在：`agent-core/src/loop.rs:280` `set_lsp_bridge`、`:285` `set_retriever`（grep 命中；`session.rs` 中同名 setter 亦存在）。

**⚠️ 设计披露**：retriever 仅当 `RETRIEVER_ENABLED=1` 才接线，否则 `sessions.retriever = None`（生产默认无语义检索注入）。lsp-bridge 仅接 `NoopLspBridge`，真实 LSP 客户端延后 P5。见 §5.3 / §5.4。

吻合度：✅ 与计划一致（B2 已落实，无裸 `use lsp_bridge;`）。

---

### 3.6 G5 — CI 工作流（Y1 修正版）

计划要求：新建 `.github/workflows/ci.yml`，含 fmt + `clippy -D warnings` + test 三步；并在建 CI 前先做 6.0 存量清理。

实际落地（`.github/workflows/ci.yml`，已 re-read）：
```yaml
env:
  RUSTFLAGS: "-D warnings"
steps:
  - run: cargo fmt --all -- --check
  - run: cargo clippy --workspace --all-targets -- -D warnings
  - run: cargo test --workspace -- --nocapture
```
文件存在 ✅，三步与计划一致。

**⚠️ 披露**：该 yml **未在真实 GitHub runner 跑过**（仓库未推送）。验证方式是按等价命令在 VM 跑通（见 §4），YAML 语法/action 版本（actions/checkout@v4、dtolnay/rust-toolchain、Swatinem/rust-cache@v2）未经真机实测。见 §5.5。

---

### 3.7 6.0 — 存量 fmt/clippy 清理（Y1）

计划要求：启用 CI 前清一遍全 workspace 存量 warn，否则 CI 首跑即红。

实际清理项（均为老代码存量，非 v2 功能引入；由 `CLIPPY_RC=0` 全 workspace 验证全清）：

| crate / 文件 | lint | 修复 |
|---|---|---|
| `agent-core/src/loop.rs` | `needless_borrow`（`Some(ref …)`） | 去 `ref`（`:443/:451` 等） |
| `agent-core/src/loop.rs` | `option_map_unit_fn` | `.map(\|turn\|{..})` → `if let` |
| `agent-core/src/loop.rs` | `redundant_closure` | `.any(\|tc\| f(tc))` → `.any(f)` |
| `agent-core/src/loop.rs` | `useless_conversion` | `.is_some_and(...)` 替换 `map_or` |
| `agent-core/src/scheduler.rs` | `redundant_closure` | `.zip(outputs.into_iter())` → `.zip(outputs)` |
| `llm-openai/src/lib.rs` | `question_mark` | SSE 解析用 `?` |
| `llm-local/src/lib.rs` | `never_loop` | `for tc` → `if let Some(tc)` |
| `llm-local/src/lib.rs` | `useless_conversion` | `.to_string().into()` → `.to_string()` |
| `llm-cn/src/lib.rs` | `useless_conversion` | `.to_string().into()` → `.to_string()` |
| `telemetry/src/eval.rs` | `field_reassign_with_default` | `ToolContext { cwd: ws.clone(), ..Default::default() }` |
| `telemetry/src/eval.rs` | 未用 import | 删 `PathBuf` |
| `service/tests/integration_test.rs` | `field_reassign_with_default` | 同上 |
| `tools-builtin/src/{bash,edit,glob,grep,read}.rs` | `new_without_default` | 补 `impl Default for` ×5 |
| `tools-builtin/src/*.rs` | `field_reassign_with_default` | `ctx.cwd` 模式 ×8 改为构造式 |
| `lsp-bridge/src/lib.rs` / `code-index/src/lib.rs` / `memory/src/lib.rs` | `needless_borrow` | `&PathBuf` → `&Path` trait 重构 |

全局复扫确认：`grep -rn "ctx\.cwd ="` 与 `grep -rn "\.to_string()\.into()"` 均无残留 ✅。

---

## 4. 验证结果（来自 `.workbuddy/gate2.log`）

| 门槛 | 命令 | 结果 |
|---|---|---|
| fmt | `cargo fmt --all -- --check` | `FMT_RC=0` ✅ |
| clippy | `cargo clippy --workspace --all-targets -- -D warnings` | `CLIPPY_RC=0` ✅ |
| test | `cargo test --all` | `GATE_END rc=0`，**146 passed / 0 failed** ✅ |

各 test binary 结果节选（均 `0 failed`，"36 FAILED"等仅为测试函数名包含 `fail` 字样，非真实失败）：
```
test result: ok. 16 passed; 0 failed; ...   (agent-core)
test result: ok. 20 passed; 0 failed; ...   (planner，含 test_t6_replan_hard_cap_forces_giveup)
test result: ok. 146 passed; 0 failed; ...  (汇总 TOTAL ok=146 fail=0)
```

---

## 5. 诚实披露：残留风险与请另一窗口重点复核项

> 以下为执行者自认的薄弱点 / 假设，另一窗口**不应默认通过**，应独立 re-read 源码 + VM 复验。

1. **replan 收敛链路未端到端断言**：T6 仅在 `planner` 单测覆盖快/慢两条 guard；`replan_count` 在 **agent loop 中的递增点**、以及 loop 如何消费 `ReflectVerdict::GiveUp`（是否真正终止而非继续 Replan）**未新增断言**。请另一窗口追 `replan_count` 递增处（planner 返回 Replan 后由谁 +1）与 agent loop 消费逻辑，确认两条 cap 都能真正收敛、无越界路径。

2. **G9 超出计划范围**：sandbox trait `cwd: &PathBuf → &Path` 是 clippy 驱动的额外重构（计划仅要求改注释）。请确认调用方语义无回归（当前 `&PathBuf::from(".")` 靠 coercion 通过，但需确认 sandbox 运行行为未变）。

3. **T4 retriever 仅 env-gated 接线**：`RETRIEVER_ENABLED` 未设时 `sessions.retriever = None`，生产默认不注入语义检索上下文。这是"部分接线"——是否符合 `top-level-design` G3 的预期？请确认设计意图。

4. **lsp-bridge 仅 Noop 接线**：真实 LSP 客户端延后 P5，属已知未完成项，非缺陷，但交付报告须如实标注。

5. **ci.yml 未经真机实测**：YAML 仅在 VM 按等价命令验证，未推 GitHub 跑 Actions。action 版本与 `RUSTFLAGS=-D warnings` 在 CI runner 的环境差异未实测。

6. **6.0 存量清理的"无行为变化"假设**：~18 处机械改动（mostly style lint）虽 146 测试全过，但属"应等价"假设。建议另一窗口抽查 2-3 处语义：`loop.rs` 的 `is_some_and`（`:717/:740`）、`scheduler.rs` 的 `.zip(outputs)`、`llm-local` 的 `if let Some(tc)`（是否改变了"取第一个 tool_call"的语义）。

7. **本地源码与 VM 一致性靠整包 tar 覆盖**：本地已从 VM 拉回 fmt 后源码（`.workbuddy/sync_from_vm.py`），理论上与 VM 一致；但同步未做逐文件 diff 校验。建议另一窗口在 VM 重新跑一次完整 gate，比对与 `.workbuddy/gate2.log` 是否一致。

---

## 6. 给另一窗口的复验脚本

```bash
# 凭据见 .workbuddy/memory/MEMORY.md（或 codex-vm-test skill）
# 本机打包（排除 target/.workbuddy/.git）
tar --force-local -czf /tmp/codex_audit.tgz \
    -C "C:/Users/87465/Desktop/codex-rust-v1.0-final" .
# 上传 VM: ~/codex_audit.tgz → 解压到 ~/codex（保留 target）→ touch 强制重编
# 前台跑：
cd ~/codex && source $HOME/.cargo/env
find ~/codex/crates -name '*.rs' -exec touch {} +
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --all
```
或直接调用 `codex-vm-test` skill 走标准流程。复验应输出 `FMT_RC=0 / CLIPPY_RC=0 / 146 passed 0 failed` 方为通过。

---

## 7. 结论

- 自判：**6 项功能 + 6.0 清理全部落地，真 Linux 三门全绿，146/0，原始 🔴 R1/R2 已闭环，实现率 100%，🔴=0**。
- 但**自审不可自证**。本报告 §3 的每段均附 `文件:行号` 与真实代码片段，请另一窗口逐条 re-read 源码复核，并重点处理 §5 的 7 项披露。
- 若另一窗口复核通过，建议下一步：在 `top-level-design.md` / `top-design-gap-analysis.md` 将 G11/G9/G4/T6/T4/G5 打勾，并打一个含 CI 修复的干净备份包定版。

---

*本报告由执行者自审生成（2026-07-28）。所有证据指向 `.workspace` 内源码与 `.workbuddy/gate2.log`。核验铁律：不信报告，信源码。*
