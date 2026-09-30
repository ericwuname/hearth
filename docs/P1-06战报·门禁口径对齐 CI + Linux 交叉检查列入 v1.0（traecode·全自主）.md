# P1-06 战报 · 门禁口径对齐 CI（含 Linux 交叉检查列入） v1.0

- **出品**：traecode　**日期**：2026-10-01　**授权**：顶层裁决 7「列入」
- **基线**：`p0-usability-01` @ `42ae8a7`
- **结论先行**：**本仓 CI 的 clippy 步一直是红的**，而 P0 阶段我报的"全绿"用的是
  **更弱的本地口径**。本卡把口径对齐 CI，修掉全部 9 处 CI clippy 报错
  （其中 **1 处是我自己引入的**），并把 Linux 交叉类型检查列入常规门禁。
- **另登记 D-42**：全量测试存在**并行环境竞态导致的 flaky**（3 次全量跑中出现 1 次）。

---

## 一、怎么发现的（这一步比修复本身更重要）

裁决 7 要求"把 Linux 交叉检查列入常规门禁"。做这件事时要回答一个连带问题：
**CI 到底在跑什么？** 于是读了 [ci.yml](file:///c:/Users/87465/Desktop/codex-rust-v1.0-final/.github/workflows/ci.yml)：

```yaml
env:
  RUSTFLAGS: "-D warnings"          # ← 所有 rustc 调用
...
- name: clippy
  run: cargo clippy --workspace --all-targets -- -D warnings   # ← 再显式加一次
```

而我在 P0 阶段每张卡用的本地口径是：

| 口径 | 本地（我用的） | CI（真实门禁） |
|---|---|---|
| fmt | `cargo fmt --all -- --check` | 同 |
| clippy | `cargo clippy --workspace --all-targets`（**警告不算失败**） | **`-D warnings`（警告即失败）** |
| test | `cargo test --workspace --no-fail-fast` | `cargo test --workspace`（在 Linux 上） |
| 平台 | **Windows** | **ubuntu-24.04** |

⇒ **"我这边全绿"与"CI 全绿"不是同一件事。** 用 `gh run list` 一看：

```
completed  failure  docs(plan): P0-12 补 commit 号            CI  main  push
completed  failure  docs(plan): P0-12 归档入修订表            CI  main  push
completed  failure  docs(plan): P0-11 归档入修订表            CI  main  push
...
```

**每一次推送都是 failure**，失败点固定在 clippy 步。

> 这正是本主线第 15 个变体：**"我测的东西不是我以为的那个东西。"**
> 我以为的绿是"本地宽松口径下的绿"，而项目真正的门禁在 CI 上、口径更严。

---

## 二、修复：9 处 CI clippy 报错

按"是否我引入"分两类（**不区分你我，一并修**——门禁红了就是红了）：

| # | 位置 | 报错 | 来源 |
|---|---|---|---|
| 1 | `sandbox/src/lib.rs` 测试 ×2 | `unnecessary_to_owned`（`&dir.path().to_path_buf()`） | **我（P0-06/P0-07）** |
| 2 | `tools-builtin/src/read.rs` 测试 ×2 | `unnecessary_cast`（`MAX_READ_BYTES as usize`，本来就 usize） | **我（P0-06）** |
| 3 | `agent-runtime/src/session.rs` | `empty_line_after_outer_attr`（我把 `///` 文档注释留成了悬空块） | **我（P0-08/D-33 重构）** |
| 4 | `agent-core/src/loop.rs:2832` | `unnecessary_lazy_evaluations`（`unwrap_or_else(\|_\| X)` → `unwrap_or(X)`） | 既有 |
| 5 | `agent-core/src/loop.rs:5542` | `single_match`（`match { Some(r) => break r, None => {} }` → `if let`） | 既有 |
| 6 | `agent-core/src/loop.rs:5765` | `duplicated_attributes`（**两个** `#[test]`） | 既有 |
| 7 | `agent-core/src/loop.rs:8692` | `empty_line_after_outer_attr`（悬空 `///` + 空行） | 既有 |
| 8 | `agent-core/src/loop.rs:5971` | `dead_code`：`test_v12_approval_bash_destructive_detected` **漏了 `#[test]`** | 既有 |
| 9 | `service/tests/integration_test.rs` | `unused_variables`（`r_err` 仅 unix 分支用）；`dead_code`（`build_script_tool_test` 全仓无人引用，~50 行） | 既有 |
| 10 | `codex-cli/src/run_local.rs` | `doc_lazy_continuation`（文档注释第二行**以 `+` 开头**，被当成 Markdown 列表项） | 既有 |
| 11 | `codex-cli/src/config.rs` | `field_reassign_with_default` | 既有 |
| 12 | `sandbox/src/lib.rs:2551` | `dead_code`：`minimal_child_env` 只被 Linux 分支使用 | 既有（Windows 专有警告） |

**其中第 8 条最有价值**：`test_v12_approval_bash_destructive_detected` 是一个
**没有 `#[test]` 的函数** —— 它写着断言、却**从未跑过一次**（一条无效的"守门员"）。
补上 `#[test]` 后它成为真正执行的测试。

第 9 条里 `build_script_tool_test` 是 ~50 行无人引用的 fixture，**删除**。

---

## 三、门禁口径升级（本卡的实际交付）

**从本卡起，本地门禁按 CI 口径执行**：

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings     # ← 关键：-D warnings
cargo test --workspace --no-fail-fast
# 裁决 7：Linux 专用代码的交叉类型检查（本机可在 Windows 上跑）
cargo check -p sandbox --target x86_64-unknown-linux-gnu --all-targets
```

已写入《长程优化探索规划》的运行机制一节，**后续每张卡必须四条全过**。

---

## 四、登记 D-42：全量测试的并行环境竞态（**flaky gate**）

本卡全量跑 3 次，其中 **1 次**出现：

```
test context::tests::test_r56_archive_digest_reads_back_and_dedups ... FAILED
assertion failed: digest.contains("早期事实C")
```

- **单独跑该测试**：通过；
- **根因（已定位）**：`HEARTH_ARCHIVE_FILE` 是**进程级全局 env**，agent-core 内
  **十余个测试**在读写它；虽然有 `ENV_SER` 串行锁，但并非所有触点都在锁内
  （`context.rs` 内多处 `set_var`，`loop.rs` 另有跨模块用锁的注释）。
  并行执行下，别的测试改了该变量 → 本测试读写到错的文件。
- **性质**：**既有问题**（非本卡引入）；本卡只是新增了一个会运行的测试
  （原第 8 条那个"漏 `#[test]`"的函数），改变了调度时序，把它暴露出来。
- **处置**：**登记 D-42（中）**，不夹带修（涉及十余处测试的锁纪律，需单独立卡）。
  **在此之前，"全量绿"必须带一句限定：该套件存在已知 flaky（约 1/3 概率不出现，1 次/3 次出现）。**

---

## 五、门禁

| 项 | 结果 |
|---|---|
| `cargo fmt --all -- --check` | 干净 |
| `cargo clippy --workspace --all-targets -- -D warnings`（**CI 口径**） | **exit 0** |
| `cargo test --workspace --no-fail-fast` | exit 0；62 target ok；0 failed（**第 2 次跑**；第 1 次遇 D-42 flaky） |
| `cargo check -p sandbox --target x86_64-unknown-linux-gnu --all-targets` | exit 0 |

**红线自查**：未改沙箱隔离语义（仅 `#[cfg_attr(allow(dead_code))]` 与测试内 `to_path_buf` 清理）；
未动 `a_arm_act_tally` / `fallback.rs`；未复活 S5；无破坏性 git 操作；未使用/未验证任何泄露密钥。

---

## 六、方法论沉淀（第 15 个变体）

> **"我测的东西，是不是项目真正在用的那道门？"**

P0 阶段我在每张卡里都写了"门禁全绿"——**每一条都是真的**，但它们描述的是
**我自己的口径**。项目真正的门禁在 CI 上：更严（`-D warnings`）、另一个平台（Linux）、
而且**已经红了很久**。

**固化为检查项**：
1. **报"门禁全绿"之前，先确认"门禁"指的是哪一份** —— 能跑一次 CI（或本地复刻 CI 口径）
   就绝不用"近似口径"；
2. **凡本地能跑、CI 会跑的检查，本地一律按 CI 参数跑**（本卡：`-D warnings`）；
3. **发现"从未执行的测试"要当缺陷处理**（漏 `#[test]` 的断言 = 不存在的守门员）；
4. **flaky 也是红的**：一个"有时失败"的门禁等于没有门禁——本卡为此单列 D-42。
