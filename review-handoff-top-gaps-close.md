# 守门员审查：handoff-top-gaps-close.md（顶层缺口关闭计划）

> 审查方式：6 项逐一核对真实 v1.2 源码（锚点行号、改动前代码、API 签名、所有权）
> 结论：**有条件通过 —— 2 🔴 必须先修正计划，2 🟡 建议补强，其余 4 项锚点精确、可直接执行**

---

## 一、逐项核验结果

| # | 项 | 锚点核验 | 判定 |
|---|---|---|---|
| 1 | G11 OPENAI_MODEL env | `main.rs:39-44` 改动前代码与源码**逐字一致**；`&String` 可入 `impl Into<String>`（`From<&String>` 存在） | ✅ 可执行 |
| 2 | G9 注释修正 | `sandbox/src/lib.rs:206` 注释原文一致；且修正内容事实正确（`FS_RW = FS_RO \| …`，FS_RO 含 `LANDLOCK_ACCESS_FS_EXECUTE`，见 :202-204） | ✅ 可执行 |
| 3 | G4 API_KEY_REQUIRED | `main.rs:153-161` 改动前代码一致；`main()` 返回 `anyhow::Result<()>`，`anyhow::bail!` 合法 | ✅ 可执行 |
| 4 | T6 replan 硬封顶 | 快路径 :186-193 一致 ✅；**慢路径锚点不存在** 🔴（见 R1） | 🔴 先修计划 |
| 5 | T4 retriever+lsp 接线 | setter 全部真实存在（`session.rs:74/79`、`loop.rs:275/280`、`retriever/lib.rs:118`、`NoopLspBridge::new` :55）；Cargo.toml 已含 lsp-bridge/retriever 依赖 ✅；**但 `openai_key.clone()` 用的是已 move 的变量** 🔴（见 R2） | 🔴 先修计划 |
| 6 | G5 CI yml | yml 本身合法 | 🟡 见 Y1 |

---

## 二、🔴 阻塞问题（执行前必须修正计划）

### R1 — T6 慢路径：`match reflected { ... }` 块不存在

计划第 4 节说"找到 LLM 慢路径的 match 块 `match reflected { ... }`，在 `Ok(ReflectVerdict::Replan)` 分支插入 guard"。
**真实源码（planner/src/lib.rs:250-256）是 if-else 链，不是 match**：

```rust
let verdict = if verdict_text.contains("give_up") || verdict_text.contains("give up") {
    ReflectVerdict::GiveUp
} else if verdict_text.contains("replan") {
    ReflectVerdict::Replan
} else {
    ReflectVerdict::Continue
};
```

执行器按图索骥会找不到锚点（或更糟：自己发挥乱改）。**修正后的补丁**：

```rust
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
```

（注意：分支内是表达式求值，**不能用 `return Ok(...)` 原方案**——用了也能编译，但风格断裂且绕过下方的 `tracing::info!` 记录。）

### R2 — T4 5b：`openai_key.clone()` 是 use-after-move，必编译失败 E0382

`main.rs:39-44` 中 `openai_key` 以值传入 `OpenAiProvider::new`（`api_key: impl Into<String>`，**所有权被 move**）。计划在 :140 之后插入的代码再 `openai_key.clone()` → **E0382 borrow of moved value**。

修正（二选一，推荐 a）：
- **(a)** 把 :43 的 `openai_key,` 改为 `openai_key.clone(),`，保住原变量供后面 retriever 块使用；
- (b) retriever 块内重新 `std::env::var("OPENAI_API_KEY")...` 读一次（与顶部逻辑重复，不推荐）。

---

## 三、🟡 建议补强

### Y1 — fmt/clippy 门槛从未验证过，直接进 CI/gate 有翻车风险

v1.2 三轮 VM 回归只跑过 `cargo test --all`，**从未跑过 `cargo fmt --check` 和 `clippy -D warnings`**。13K 行历史代码大概率有 fmt 差异/clippy warn，CI 首跑即红。建议：
1. 执行器先在 VM 跑一次 `cargo fmt --all && cargo clippy --workspace --all-targets --fix --allow-dirty`，把存量问题清掉再启用 CI；
2. §8 gate 的 clippy 命令补上 `-- -D warnings`（现在 gate 与 ci.yml 标准不一致：CI 有 `-D warnings`，gate 没有）。

### Y2 — T6"无需新增测试"违反本项目铁律

"已有 replan 逻辑测试覆盖"经查不成立：现有测试（:422/:491 附近）均为 `replan_count: 0`，**没有任何测试覆盖 `replan_count >= 3` 的 no-progress 路径**。按"测试必须能失败"铁律，应新增：

```rust
#[tokio::test]
async fn test_t6_replan_hard_cap_forces_giveup() {
    // steps_without_progress >= MAX_STEPS_WITHOUT_PROGRESS 且 replan_count = 3
    // → 期望 GiveUp 而非 Replan（快路径，不触发 LLM）
}
```

---

## 四、🔵 参考项（不阻塞）

- **B1** G11 验证条款说"启动日志显示模型名"，但现有代码并不打印模型名——要么在 patch 里补一行 `info!("openai model: {openai_model}")`，要么把验证改为"编译通过 + env 覆盖生效"。
- **B2** T4 5a 的 `use lsp_bridge;` 是冗余导入（2018 edition crate 名直接可用）；建议改成 `use lsp_bridge::NoopLspBridge;` 并在 5c 里用短名，或干脆不加 import 直接全路径调用。
- **B3** §8 gate 在 VM 执行前记得先把新代码包传上去（计划前置条件里写了 `~/codex` 覆盖，但没写打包上传步骤——沿用现有 vm_verify 流程即可）。

---

## 五、结论

| 维度 | 判定 |
|---|---|
| 锚点准确率 | 4/6 精确，2 项锚点错误（R1 不存在的 match 块、R2 所有权错误） |
| API 真实性 | 全部 setter/构造函数已核实存在 ✅ |
| 门禁 | **修正 R1/R2 后放行执行**；Y1 建议在启用 CI 前先清存量 fmt/clippy |
| 验收标准 | VM `cargo test --all` rc=0（145+1 新增 T6 测试），fmt/clippy 视 Y1 处置结果 |

*审查人：守门员（源码直读核验，2026-07-28）*
