# 顶层缺口关闭执行计划 v2（handoff-to-executor）

> 基线：v1.2 定版（codex-rust-v1.2）
> 目标：关闭 top-design-gap-analysis.md 推荐立即做的 6 项，将 To-Be 覆盖率从 ~0% 拉到 ~55%
> 预估：合计 < 1 天（4 极小 + 1 小 + 1 中）
> 格式：每项带 文件:行号 → 精确补丁 / 验证命令 / 提交门槛
>
> **v2 变更记录**（相对 v1，依据 `review-handoff-top-gaps-close.md` 守门员审查）：
> - 🔴 R1 已修：T6 慢路径锚点从"不存在的 match 块"改为真实 if-else 链补丁（planner/lib.rs:250-256）
> - 🔴 R2 已修：T4 的 use-after-move —— G11 补丁中 `openai_key` 改为 `openai_key.clone()` 传入，保住原变量
> - 🟡 Y1 已修：新增 §6.0 存量 fmt/clippy 清理步骤；§8 gate 的 clippy 补 `-- -D warnings` 与 ci.yml 对齐
> - 🟡 Y2 已修：T6 新增可失败断言测试 `test_t6_replan_hard_cap_forces_giveup`
> - 🔵 B1 已修：G11 补启动日志打印模型名，使验证条款成立
> - 🔵 B2 已修：删除冗余 `use lsp_bridge;`，改导入 `NoopLspBridge`
> - 🔵 B3 已修：§8 gate 补打包上传步骤

---

## 前置条件

- Linux VM `wutao@192.168.220.131`（Ubuntu 24.04，landlock/seccomp 可用）
- VM 上 `~/codex` 为项目目录（覆盖源码 + 保留 target 增量编译）
- 本机无 Rust 工具链——**所有编译/测试验证一律在 VM 上执行**（项目铁律）

---

## 1. G11 — OpenAI 模型名硬编码 → env 可配置（含 R2 前置修正）

**文件**: `crates/service/src/main.rs` @39-44

**改动前**（行 39-44，与源码逐字一致）:
```rust
let openai = Arc::new(OpenAiProvider::new(
    "openai",
    "gpt-4o",
    openai_url,
    openai_key,
));
```

**改动后**（⚠️ 注意第 5 参必须是 `openai_key.clone()` —— 这是 R2 修正：
`OpenAiProvider::new` 的 `api_key: impl Into<String>` 会 move 所有权，
不 clone 的话第 5 节 T4 的 retriever 块用 `openai_key` 会 E0382）:
```rust
let openai_model = std::env::var("OPENAI_MODEL")
    .unwrap_or_else(|_| "gpt-4o".into());
info!("openai model: {openai_model}");
let openai = Arc::new(OpenAiProvider::new(
    "openai",
    &openai_model,
    openai_url,
    openai_key.clone(),
));
```

**验证**: 编译通过；启动日志出现 `openai model: gpt-4o`（或 env 覆盖后的值）。

---

## 2. G9 — sandbox 注释与实现不一致修正

**文件**: `crates/sandbox/src/lib.rs` @206

**改动前**:
```rust
    // Full r/w access for writable_paths (but no execute)
```

**改动后**:
```rust
    // Full r/w access for writable_paths (includes execute via FS_RO)
```

**验证**: `grep "no execute" crates/sandbox/src/lib.rs` 无匹配。

---

## 3. G4 — API_KEY_REQUIRED 强制鉴权

**文件**: `crates/service/src/main.rs` @152-161

**改动前**:
```rust
    let api_key = std::env::var("API_KEY").ok().filter(|k| !k.is_empty());
    if api_key.is_some() {
        info!("API auth enabled (Authorization: Bearer <API_KEY> required)");
    } else {
        tracing::warn!(
            "API_KEY not set — API is OPEN (anyone reaching this port can run \
             commands via the agent). Set API_KEY in production."
        );
    }
```

**改动后**:
```rust
    let api_key = std::env::var("API_KEY").ok().filter(|k| !k.is_empty());
    if api_key.is_some() {
        info!("API auth enabled (Authorization: Bearer <API_KEY> required)");
    } else if std::env::var("API_KEY_REQUIRED")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false)
    {
        anyhow::bail!(
            "API_KEY_REQUIRED=1 but API_KEY is not set. \
             Refusing to start open API in production mode."
        );
    } else {
        tracing::warn!(
            "API_KEY not set — API is OPEN (anyone reaching this port can run \
             commands via the agent). Set API_KEY in production, \
             or API_KEY_REQUIRED=1 to enforce."
        );
    }
```

（`main()` 返回 `anyhow::Result<()>`，`bail!` 合法——已核实。）

**验证**: `API_KEY_REQUIRED=1 cargo run` → 报错退出；`API_KEY=test API_KEY_REQUIRED=1 cargo run` → 正常启动；仅 `cargo run` → 旧行为（WARN + 开放）。

---

## 4. T6 — replan 硬封顶（R1 修正版 + Y2 新增测试）

### 4a. 快路径（no-progress 分支）

**文件**: `crates/planner/src/lib.rs` @186-193

**改动前**（与源码一致）:
```rust
        if obs.steps_without_progress >= MAX_STEPS_WITHOUT_PROGRESS {
            tracing::warn!(
                "Replanning: {} steps without progress (threshold={})",
                obs.steps_without_progress,
                MAX_STEPS_WITHOUT_PROGRESS,
            );
            return Ok(ReflectVerdict::Replan);
        }
```

**改动后**:
```rust
        if obs.steps_without_progress >= MAX_STEPS_WITHOUT_PROGRESS {
            // T6 (top-level-design G1): hard-cap replan_count to prevent
            // unbounded replan loops on the "no-progress no-errors" path.
            if state.replan_count >= 3 {
                tracing::warn!(
                    "Giving up: max replan exceeded ({} replans, {} steps w/o progress)",
                    state.replan_count,
                    obs.steps_without_progress,
                );
                return Ok(ReflectVerdict::GiveUp);
            }
            tracing::warn!(
                "Replanning: {} steps without progress (threshold={}), replan #{}/3",
                obs.steps_without_progress,
                MAX_STEPS_WITHOUT_PROGRESS,
                state.replan_count + 1,
            );
            return Ok(ReflectVerdict::Replan);
        }
```

### 4b. 慢路径（LLM verdict 解析，⚠️ R1 修正：这里是 if-else 链，不是 match）

**文件**: `crates/planner/src/lib.rs` @250-256

**改动前**（真实源码，if-else 链）:
```rust
let verdict = if verdict_text.contains("give_up") || verdict_text.contains("give up") {
    ReflectVerdict::GiveUp
} else if verdict_text.contains("replan") {
    ReflectVerdict::Replan
} else {
    ReflectVerdict::Continue
};
```

**改动后**（只改 `replan` 分支；分支是表达式求值，**禁止用 `return Ok(...)`**——会绕过下方的 verdict tracing 日志）:
```rust
let verdict = if verdict_text.contains("give_up") || verdict_text.contains("give up") {
    ReflectVerdict::GiveUp
} else if verdict_text.contains("replan") {
    // T6: hard-cap — LLM cannot suggest unbounded replans either.
    if state.replan_count >= 3 {
        tracing::warn!(
            "LLM suggested Replan but max replan exceeded ({}). Forcing GiveUp.",
            state.replan_count,
        );
        ReflectVerdict::GiveUp
    } else {
        ReflectVerdict::Replan
    }
} else {
    ReflectVerdict::Continue
};
```

### 4c. 新增回归测试（Y2：现有测试 `replan_count` 全为 0，硬封顶路径零覆盖）

**文件**: `crates/planner/src/lib.rs`，加在 `test_p3_reflect_three_way`（@408-473）之后，同一 `mod tests` 内。
复用该测试的既有写法（`DefaultPlanner::new(mock)`、`TaskGraph`/`PlanState`/`Observation` 字面量——字段名已核实）：

```rust
    // ── T6: replan hard-cap ──

    #[tokio::test]
    async fn test_t6_replan_hard_cap_forces_giveup() {
        // 快路径命中：steps_without_progress >= MAX_STEPS_WITHOUT_PROGRESS(5)
        // 且 replan_count 已达 3 → 必须 GiveUp 而非第 4 次 Replan。
        // 不触发 LLM（快路径优先），mock 给一条兜底响应即可。
        let mock = Arc::new(MockProvider::new(vec![
            ChatResponse { content: Some("continue".into()), tool_calls: vec![], finish_reason: Some("stop".into()), usage: None },
        ]));
        let planner = DefaultPlanner::new(mock);
        let tg = TaskGraph {
            nodes: vec![TaskNode {
                id: "task".into(), description: "do something".into(),
                deps: vec![], status: TaskStatus::Pending, delegable: false, result: None,
            }],
        };
        // 对照组：replan_count=2 未达上限 → 仍是 Replan（证明测试能失败）
        let state_under_cap = PlanState { task_graph: tg.clone(), current_node_index: Some(0), replan_count: 2, total_steps: 10 };
        let state_at_cap = PlanState { task_graph: tg.clone(), current_node_index: Some(0), replan_count: 3, total_steps: 10 };
        let obs_no_progress = Observation {
            recent_results: vec![],
            task_graph: tg.clone(),
            consecutive_errors: 0,
            steps_without_progress: 6,
            budget_remaining: 40,
            steps_used: 10,
        };

        let v = planner.reflect(&obs_no_progress, &state_under_cap).await.unwrap();
        assert_eq!(v, ReflectVerdict::Replan, "replan_count=2 < cap → still replan");

        let v = planner.reflect(&obs_no_progress, &state_at_cap).await.unwrap();
        assert_eq!(v, ReflectVerdict::GiveUp, "replan_count=3 hits cap → forced give_up");

        eprintln!("T6 PASS: replan hard-cap forces GiveUp at count 3");
    }
```

**验证**: `cargo test -p planner` 全过，且包含新测试 `test_t6_replan_hard_cap_forces_giveup`。
（自检：若把 4a 的 guard 删掉，此测试第二个断言必须失败——"测试必须能失败"铁律。）

---

## 5. T4 — retriever + lsp-bridge 生产接线（R2/B2 修正版）

### 5a. 新增 imports（B2 修正：不要 `use lsp_bridge;` 裸导入）

**文件**: `crates/service/src/main.rs` @8-12，**在 `use memory::JsonlMemoryStore;` 之后加**:

```rust
use lsp_bridge::NoopLspBridge;
use retriever::TantivyRetriever;
```

（`retriever`、`lsp-bridge` 已在 `service/Cargo.toml` 依赖中——已核实，无需改 toml。）

### 5b. 接线 retriever（env-gated）

**文件**: `crates/service/src/main.rs` @140-141，**在 `sessions.set_memory_store(memory_store);` 之后、`info!("memory store wired: {memory_dir}");` 之前**插入:

```rust
    // T4 (top-level-design G3): Wire semantic retriever for code context
    // injection. Gated by RETRIEVER_ENABLED=1; needs OPENAI_API_KEY for
    // embeddings (default model text-embedding-3-small).
    if std::env::var("RETRIEVER_ENABLED")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false)
    {
        let embed_model = std::env::var("EMBED_MODEL")
            .unwrap_or_else(|_| "text-embedding-3-small".into());
        let embed_provider = Arc::new(OpenAiProvider::new(
            "embed",
            &embed_model,
            std::env::var("OPENAI_BASE_URL").ok(),
            openai_key.clone(),
        ));
        let mut retriever = TantivyRetriever::new();
        retriever.set_embed_provider(embed_provider);
        sessions.set_retriever(Arc::new(retriever));
        info!("retriever wired (embed model: {embed_model})");
    }
```

（⚠️ 此块能编译的前提是第 1 节 G11 已把 @43 改为 `openai_key.clone(),`——R2。
**执行顺序约束：必须先做第 1 节，再做本节。**）

### 5c. 接线 lsp-bridge（无条件，显式 Noop）

**同上位置，retriever 块之后加**:

```rust
    // T4: Explicitly wire NoopLspBridge — makes the lsp wiring visible.
    // Real LSP client deferred to P5 (crates/lsp-bridge/src/lib.rs:6-9).
    sessions.set_lsp_bridge(Arc::new(NoopLspBridge::new()));
```

### 5d. 对应 setter 确认（无需改动，确认存在即可）

- `service/src/session.rs:74-81` `set_retriever / set_lsp_bridge` — 已核实存在 ✅
- `agent-core/src/loop.rs:275-281` `AgentLoop::set_retriever` — 已核实存在 ✅
- `retriever/src/lib.rs:118` `TantivyRetriever::new` + `set_embed_provider` — 已核实存在 ✅
- `lsp-bridge/src/lib.rs:55` `NoopLspBridge::new` — 已核实存在 ✅
- **验证**: `grep -rn "set_retriever\|set_lsp_bridge" crates/service/src/session.rs` 应命中。

---

## 6. G5 — CI 工作流（Y1 修正版）

### 6.0 前置：清理存量 fmt/clippy 问题（Y1，必做，否则 CI 首跑即红）

本项目三轮 VM 回归**只跑过 `cargo test`，从未跑过 fmt/clippy**。启用 CI 前先在 VM 清一遍存量：

```bash
cd ~/codex && source $HOME/.cargo/env
cargo fmt --all                                                  # 自动格式化存量
cargo clippy --workspace --all-targets --fix --allow-dirty       # 自动修可修的 warn
cargo clippy --workspace --all-targets -- -D warnings            # 剩余 warn 手工清零
cargo test --all                                                 # 确认清理没改坏行为
```

清理产生的 diff 需带回本机源码（fmt/clippy 改动一并纳入本轮提交）。
若某条 clippy warn 属误报，用精确的 `#[allow(clippy::xxx)]` + 注释说明，禁止全局 allow。

### 6.1 新建 CI 文件

**新建文件**: `.github/workflows/ci.yml`

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  check:
    name: check + test
    runs-on: ubuntu-24.04
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - uses: Swatinem/rust-cache@v2

      - name: fmt
        run: cargo fmt --all -- --check

      - name: clippy
        run: cargo clippy --workspace --all-targets -- -D warnings

      - name: test
        run: cargo test --workspace -- --nocapture
```

**验证**: 推送到 GitHub 仓库后 Actions tab 出现 CI 运行且全绿（前提：6.0 已清完存量）。

---

## 7. 改后全量列表

| 文件 | 改动 |
|---|---|
| `crates/service/src/main.rs` | G11(OPENAI_MODEL env + 日志 + `openai_key.clone()`) + G4(API_KEY_REQUIRED) + T4(retriever+lsp 接线 + 2 imports) |
| `crates/sandbox/src/lib.rs` | G9(注释 1 行) |
| `crates/planner/src/lib.rs` | T6(replan 硬封顶 2 处 + 1 个新测试) |
| `.github/workflows/ci.yml` | G5(新建 CI yml) |
| （若干文件） | 6.0 存量 fmt/clippy 清理产生的机械 diff |

**执行顺序**: §1 → §2 → §3 → §4 → §5 → §6.0 →（VM gate 全绿后）→ §6.1。
（§1 必须先于 §5：R2 所有权依赖。）

---

## 8. 提交门槛（gate）（Y1/B3 修正版）

### 8.0 打包上传（B3：VM 上是旧代码，先同步）

在本机（沿用现有 vm_verify 流程）：

```bash
tar --force-local --exclude='target' --exclude='.workbuddy' --exclude='.git' \
    -czf "C:/Users/87465/AppData/Local/Temp/codex_v11.tgz" \
    -C "C:/Users/87465/Desktop/codex-rust-v1.0-final" .
# scp/paramiko 上传到 VM: ~/codex_v11.tgz
# VM 端解压（保留 target 增量）:
#   find ~/codex -maxdepth 1 -mindepth 1 ! -name target -exec rm -rf {} + \
#     && tar -xzf ~/codex_v11.tgz -C ~/codex
```

### 8.1 VM 验收命令

```bash
cd ~/codex
source $HOME/.cargo/env
cargo fmt --all -- --check                            # 格式检查（6.0 后应无差异）
cargo clippy --workspace --all-targets -- -D warnings  # 与 ci.yml 同标准（Y1 修正：补 -D warnings）
cargo test --all -- --nocapture                        # 全量测试
echo $?                                                # 期望 rc=0
```

**期望结果**:
- fmt 无差异
- clippy 0 warning（`-D warnings` 下即 0 error）
- cargo test：36 suite 全 ok，**146 passed（145 + 新增 `test_t6_replan_hard_cap_forces_giveup`）/ 0 failed**

---

## 9. 执行后文档更新

- 更新 `docs/top-level-design.md`：G11/G9/G4/T6/T4/G5 标记为 ✅
- 更新 `top-design-gap-analysis.md`：已闭环项打勾
- 附 VM `TEST_DONE rc=0` 日志片段（含 fmt/clippy 通过记录）

---

*v2 修订人：守门员审查闭环（R1/R2/Y1/Y2/B1/B2/B3 全部合入），2026-07-28。
审查依据见 `review-handoff-top-gaps-close.md`；锚点均经真实 v1.2 源码直读核验。*
