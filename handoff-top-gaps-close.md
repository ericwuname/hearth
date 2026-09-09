# 顶层缺口关闭执行计划（handoff-to-executor）

> 基线：v1.2 定版（codex-rust-v1.2）
> 目标：关闭 top-design-gap-analysis.md 推荐立即做的 6 项，将 To-Be 覆盖率从 ~0% 拉到 ~55%
> 预估：合计 < 1 天（4 极小 + 1 小 + 1 中）
> 格式：每项带 文件:行号 → 精确补丁 / 验证命令 / 提交门槛

---

## 前置条件

- Linux VM `wutao@192.168.220.131`（Ubuntu 24.04，landlock/seccomp 可用）
- 本机 Rust 工具链（rustc stable），cargo
- `~/codex` 为项目目录（覆盖源码 + 保留 target 增量编译）

---

## 1. G11 — OpenAI 模型名硬编码 → env 可配置

**文件**: `crates/service/src/main.rs` @39-44

**改动前**（行 39-44）:
```rust
let openai = Arc::new(OpenAiProvider::new(
    "openai",
    "gpt-4o",
    openai_url,
    openai_key,
));
```

**改动后**:
```rust
let openai_model = std::env::var("OPENAI_MODEL")
    .unwrap_or_else(|_| "gpt-4o".into());
let openai = Arc::new(OpenAiProvider::new(
    "openai",
    &openai_model,
    openai_url,
    openai_key,
));
```

**验证**: 编译通过；`OPENAI_MODEL=gpt-4o-mini cargo run` 启动日志显示模型名。

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

**验证**: `grep "no execute" sandbox/src/lib.rs` 无匹配。

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

**验证**: `API_KEY_REQUIRED=1 cargo run` → 应 panic 退出（无 API_KEY）；`API_KEY=test API_KEY_REQUIRED=1 cargo run` → 正常启动；仅 `cargo run` → 旧行为（WARN + 开放）。

---

## 4. T6 — replan 硬封顶

**文件**: `crates/planner/src/lib.rs` @186-193（第 4 路）和 @205-264（第 6 路 LLM 慢路径）

**改动**（第 4 路，行 186-193）:

**加在 `return Ok(ReflectVerdict::Replan);` 前面插入 replan_count 检查**:

```rust
        // Replan: stuck — many steps without progress
        if obs.steps_without_progress >= MAX_STEPS_WITHOUT_PROGRESS {
            // T6 (top-level-design G1): hard-cap replan_count to prevent
            // unbounded replan loops on the "no-progress no-errors" path.
            if state.replan_count >= 3 {
                tracing::warn!(
                    "Giving up: max replan exceeded ({} replans, {} steps w/o progress, threshold={})",
                    state.replan_count,
                    obs.steps_without_progress,
                    MAX_STEPS_WITHOUT_PROGRESS,
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

**改动**（第 6 路，LLM 慢路径，在 @205-264 `reflected` match 的 `Replan` 分支加 guard）:

找到 LLM 慢路径的 match 块 `match reflected { ... }`，在 `Ok(ReflectVerdict::Replan)` 分支内部最前面插入：

```rust
            if state.replan_count >= 3 {
                tracing::warn!(
                    "LLM suggested Replan but max replan exceeded ({}). Forcing GiveUp.",
                    state.replan_count,
                );
                return Ok(ReflectVerdict::GiveUp);
            }
```

**验证**: 现有测试 `cargo test -p planner` 全部通过；无需新增测试（已有 replan 逻辑测试覆盖）。

---

## 5. T4 — retriever + lsp-bridge 生产接线

### 5a. 新增 imports

**文件**: `crates/service/src/main.rs` @8-12，**在 `use memory::JsonlMemoryStore;` 之后加**:

```rust
use lsp_bridge;
use retriever::TantivyRetriever;
```

（`code-index`、`lsp-bridge` 已在 `service/Cargo.toml` 依赖中，无需改 toml。）

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

### 5c. 接线 lsp-bridge（无条件，显式 Noop）

**同上位置，retriever 块之后加**:

```rust
    // T4: Explicitly wire NoopLspBridge — makes the lsp wiring visible.
    // Real LSP client deferred to P5 (crates/lsp-bridge/src/lib.rs:6-9).
    sessions.set_lsp_bridge(Arc::new(lsp_bridge::NoopLspBridge::new()));
```

### 5d. 对应 setter 确认（无需改动，确认存在即可）

- `service/src/session.rs:74-81` `set_retriever / set_lsp_bridge` — 已存在 ✅
- `agent-core/src/loop.rs:279-281` `AgentLoop::set_retriever` — 已存在 ✅
- **验证**: `grep -rn "set_retriever\|set_lsp_bridge" crates/service/src/session.rs` 应命中。

---

## 6. G5 — CI 工作流

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

**验证**: 推送到 GitHub 仓库后 Actions tab 应出现 CI 运行。

---

## 7. 改后全量列表

| 文件 | 改动 |
|---|---|
| `crates/service/src/main.rs` | G11(OPENAI_MODEL env) + G4(API_KEY_REQUIRED) + T4(retriever+lsp 接线 + 2 imports) |
| `crates/sandbox/src/lib.rs` | G9(注释 1 行) |
| `crates/planner/src/lib.rs` | T6(replan 硬封顶 2 处) |
| `.github/workflows/ci.yml` | G5(新建 CI yml) |

---

## 8. 提交门槛（gate）

执行完成后在 Linux VM `wutao@192.168.220.131` 执行：

```bash
cd ~/codex
source $HOME/.cargo/env
cargo fmt --all -- --check             # 格式检查
cargo clippy --workspace --all-targets  # 无警告
cargo test --all -- --nocapture         # 36 suite ok, 0 failed
echo $?                                  # 期望 rc=0
```

**期望结果**: fmt 无差异、clippy 无 warn、cargo test 36 suite ok / 0 failed。

---

## 9. 执行后文档更新

- 更新 `docs/top-level-design.md`：G11/G9/G4/T6/T4/G5 标记为 ✅
- 更新 `top-design-gap-analysis.md`：已闭环项打勾
- 附 `TEST_DONE rc=0` 日志片段
