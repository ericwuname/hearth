# 卡：TUI-polish3c —— 只修 1 处编译错误（polish3 遗留的半成品收尾）

- **状态**：**已发射**（polish3 被 `timeout 2400` 强杀，遗留 **1 个编译错误**；本卡只修它，不做别的）
- **施工者**：EMBER（`.133`，`/home/wutao/hearth-tui-new/`）
- **预算**：**≤6 步工具循环**（**本卡只有一件事**）
- **唯一允许改动的文件**：`src/main.rs`
- **当前状态（已实测）**：`src/main.rs` md5 `ccf66a343f82d76026f81b1f6fdb4907`、634 行；
  `cargo build --release` → **`build_rc=101`，1 个错误（E0308）**。
- **⚠️ 不要回退、不要重写**：polish3 已落的改动是**对的**（新增 `find_answer_rendered_index` + `pending_auto_scroll` + T3 显式配对），
  **只差一个类型转换**。回退会白扔 40 分钟成果。

---

## 编译错误原文（照抄自实测，不用你自己去复现）

```
error[E0308]: mismatched types
   --> src/main.rs:281
    |
281 |                 let max_off = total.saturating_sub(area_height);
    |                                                             ^^^^^^^^^^^ expected `usize`, found `u16`
    |
help: you can convert a `u16` to a `usize`
    |
281 |                 let max_off = total.saturating_sub(area_height.into());
```

**成因**：同一段里 `let term_h = terminal.size().map(|s| s.height).unwrap_or(24);` → `term_h: u16`；
`let area_height = term_h.saturating_sub(9).saturating_sub(2);` → 也是 `u16`；而 `total` 是 `usize`。
（区块高度算式 `-9` 是**正确的**：`draw()` 的 `Layout::vertical([Length(3), Min(1), Length(3), Length(3)])`
⇒ `chunks[1].height = 屏幕高 - 9`，再减 2 条边框 = 可视高度。**不要改这个算式**。）

## 唯一要求

把 `area_height` 变成 `usize`，二者等价任选其一：

```rust
let area_height = term_h.saturating_sub(9).saturating_sub(2) as usize;
```
或
```rust
let area_height = usize::from(term_h.saturating_sub(9).saturating_sub(2));
```

**只允许这一处修正**（外加为消除 warning 所必需的显式类型标注）。
**禁止**：重构、重命名、加功能、改 T3 逻辑、改 `fold` 默认值、动 `hearth-slim/`、动渲染函数。

## 硬约束

1. 每条 `cargo` 命令前 `export PATH=$HOME/.cargo/bin:$PATH`。
2. 日志只追加：`run-polish3c.log` / `trace-polish3c.err`；**证据接续写** `evidence-polish3.log`（用 `>>`，**不得覆盖**已有的 RED 段）。
3. tmux 会话名以 `pl3c` 开头；**不得碰** `ht3` / `t`；顺手把自己的残留会话 `pl3r` 收掉（`tmux kill-session -t pl3r`）**是可选的**。
4. **轮次纪律**：错误行号已给你，**不要**做任何探索（不要 `ls`、不要多次 `read`）。改完就 build。

## 自测（逐条把原始输出贴进 `evidence-polish3.log`）

| # | 自测 | 期望 |
|---|---|---|
| a | `cargo build --release` | **0 error / 0 warning** |
| b | 100×24 启动 + `只回答一个数字：3+4=?` + Enter，**不按任何键**，抓屏 | 对话框内 `◂ span` **≥1** 且 `✓ Done` **≥1**（G1 PASS） |
| c | **可失败性对照** | 同断言跑在改前存档 `/tmp/v20-cap/11-final.txt` 上必须 **FAIL**（`ANSWER_CANDIDATE: none`） |
| d | `PgUp` ×1 后等 2 秒 | 视口停在手动位置（`scroll:a/b` 的 `a` 不回落） |
| e | 回归：`7` 在屏、`done (` 出现、无 `running` 残留 | 全绿 |
| f | `Ctrl+Q` | 会话销毁（`tmux has-session` 非 0） |

**收尾**：把 `md5sum src/main.rs` / `wc -l src/main.rs` / `ls -la target/release/hearth-tui` 追加进 `evidence-polish3.log`；
如实声明 b/d/e 哪些真跑了（**没跑就说没跑，禁止虚报**）。
