# Hearth v0.1.1 真机复测问题 → 修复任务书（v0.1.2）

> 日期：2026-08-22 | 基线：`83fbf9d`（v0.1.1 收口）  
> 来源：用户 VM 真机测试 `release/手工测试v0.1.1.txt`（5 个游戏任务 + 1 个简单文件任务）  
> 守门员核验：已读源码确认根因，非凭报告推测



---

## 0. 真机测试结果总览（用户视角）

| 任务                    | 结果                      | 卡点                                                                 |
| --------------------- | ----------------------- | ------------------------------------------------------------------ |
| 贪吃蛇（snake_game_final） | ✗ 只出 index.html 1319 字节 | write_file `missing 'path'` 连撞 3 次 → give_up                       |
| 五子棋（gomoku）           | ✗ 零文件                   | write_file `missing 'content'` 连撞 3 次 → budget exhausted           |
| 象棋（chess_game ×3）     | ✗ 零文件                   | `plan chat failed, error decoding response body` 重试 6 次耗尽（245s 卡死） |
| 中国象棋（budget 25）       | ✗ 零文件                   | 同上，deepseek 读 body 失败死循环                                           |
| 1+1（chess_game 误目录）   | △ 跑偏                    | AI 建 Cargo.toml 触发 workspace 冲突，没回答问题                              |
| hello.py（write_test）  | ✓ 成功                    | 短内容 write_file 正常（budget 耗尽但文件已写）                                  |

**结论**：v0.1.1 修复**只覆盖了短参数 path 场景**，超长 content / deepseek 传输失败两大冰山本体仍暴露。用户原话："问题很多"——属实。

---

## 1. 问题根因分类（守门员已核源码）

### 🔴 P0-A：write_file content 未走容错（v0.1.1 修复盲区）

- **现象**：五子棋/象棋 write_file 报 `missing 'content' argument`（注意不是 path）。
- **根因**：`crates/tools-builtin/src/edit.rs:48` path 走了 `extract_str_arg` 容错，但 `:84-87` content 直接 `args.get("content").and_then(as_str)`——当 LLM 层把截断 JSON 降级成 raw String 时，String 无 `.get("content")`，→ 取不到 → `missing 'content'`。
- **证据**：用户日志 `WARN tool call 'write_file' has non-JSON arguments (EOF at 12004 chars) → missing 'content'`（line 286/302/333）。path 被救回了（render 显示 `{"path": "gomoku.html", ...}`），content 没救回。
- **修复**：content 也走 `extract_str_arg`（与 path 同款前缀提取 + 完整 parse 兜底）。

### 🔴 P0-B：deepseek 非流式读 body 失败无容错（传输层脆弱）

- **现象**：象棋/中国象棋大量 `plan chat failed, error decoding response body`（loop.rs:1271 重试 6 次，指数退避到 32s，单次任务卡 245s）。
- **根因**：`crates/llm-openai/src/lib.rs:327` `resp.text().await` 一次性读非流式 body——deepseek 偶发返回不完整/连接重置，`text()` 直接 Err，且**无重试、无流式降级**。用户 curl 同 key 同模型完全正常（返回 200 + 合法 JSON），证明是 Rust reqwest 读 body 的传输层问题，非 API 拒答。
- **修复**（两层）：
  - B1（治标）：`lib.rs:327` 读 body 失败 → 立即重试同请求 1-2 次（连接级，非语义级），仍失败才上抛。
  - B2（治本）：非流式 chat 改走**流式 SSE 累积**（`stream()` 已实现，line 382）——SSE 逐 chunk 读，单 chunk 损坏不影响整体，且 deepseek SSE 比非流式稳定（用户日志里 write_file 的 tool_call 警告就是流式路径产生的，说明 stream 路径更稳）。
  - B3（防卡死）：loop.rs:1271 重试对 `error decoding response body` 类**传输错误**设更短退避 + 更早上报，避免 245s 卡死；或识别为"供应商瞬时故障"直接 fail fast 给用户可行动错误。

### 🟡 P1：安装验证流程误导（交付瑕疵，非代码 bug）

- **现象**：用户用 `strings /usr/local/bin/hearth | grep force_param_gap` 得 0，误以为二进制没含修复。
- **根因**：Rust release 二进制**符号被 strip**，函数名不在字符串表，`strings` 查 Rust 标识符本身无效。用户后来 `hearth --version` 显示 `0.1.1` 佐证二进制是新的。
- **修复**：① 在 README / 安装手册写清"**不要用 strings 验证，用 `hearth --version` 看版本 + 跑 hello.py 短任务看 write_file 是否成功**"；② 编译时给二进制打 `RUSTFLAGS="--cfg hearth_version_check"` 或在 `--version` 输出追加 commit short hash（如 `hearth 0.1.1 (83fbf9d)`），让用户一眼确认代码版本。

### 🔵 P2：cgroup Permission denied 刷屏（环境噪音）

- **现象**：非特权 VM 每个工具调用刷 2 行 `cannot add pid N to cgroup: Permission denied`（line 821）。
- **根因**：`crates/sandbox/src/lib.rs:821` 是 `tracing::warn!`，每次 add_pid 失败都打。非特权环境必然失败，属预期降级，不该 warn 级刷屏。
- **修复**：降级为 `tracing::debug!` + **首次失败时打印一次 info 级提示**（"cgroup 不可用，已按 HEARTH_ALLOW_NO_CGROUP=1 降级"），后续静默。

### 🔵 P2：AI 在错误目录建项目触发 cargo workspace 冲突

- **现象**：`1+1` 任务 AI 在 `~/chess_game` 建 `Cargo.toml`，被 `~/Cargo.toml` workspace 吞 → `cargo test` 报 workspace 冲突。
- **根因**：AI 行为问题，非核心 bug。但可缓解：write_file 检测到生成的 `Cargo.toml` 与上级 workspace 冲突时，给 AI 回 feed "检测到 workspace 冲突，建议加 `[workspace]` 空表独立"。
- **修复**：write_file 执行后若内容是 Cargo.toml 且 cwd 在另一 workspace 内 → 追加 hint 到工具结果（轻量，不阻塞主路径）。

---

## 2. 任务书（给执行窗口）

### R1（P0-A 必做）：content 容错对齐 path

- `edit.rs:84-87` content 取参改用 `crate::extract_str_arg(&args, "content")`，与 path 同款逻辑。
- 单测：构造 raw String 降级（截断 content 在对象中部）→ 断言能前缀提取 content 成功。
- 覆盖 read/grep/glob 若也有类似"第二个关键字段"漏接，一并补 `extract_str_arg`。

### R2（P0-B 必做）：deepseek 传输层容错

- B1：`lib.rs:327` 读 body 失败 → 重试同请求 2 次（指数 1s/2s），仍失败上抛。
- B2：非流式 `chat()` 改为内部调用 `stream()` 累积（SSE 更稳定），或至少对 deepseek 默认走 stream。
- B3：`loop.rs` 重试逻辑对 `error decoding response body` 设更短上限（如最多 3 次 + 总时长 cap 60s），超则 fail fast 给用户可行动错误（"deepseek 连接不稳定，建议重试或换模型"）。
- 单测：mock 一个第一次 text() 失败第二次成功的 client，验证重试生效。

### R3（P1 必做）：版本可验证

- `hearth --version` 输出追加 commit short hash：`hearth 0.1.2 (abcdef0)`。
- README §1 + `docs/hearth-v0.1.1-vm-manual.md` 改验证指引：删 `strings` 检查法，改"version 含 hash + 跑 hello.py 看 write_file 成功"。

### R4（P2 噪音）：cgroup 日志降级

- `lib.rs:821` warn → debug + 首次失败 info 级单次提示。

### R5（P2 轻量）：Cargo.toml workspace 冲突 hint

- write_file 执行后若生成 Cargo.toml 且 cwd 处于上级 workspace → 工具结果追加一行 hint。

### R6（通用门禁）

- `cargo fmt --check && cargo clippy --all-targets && cargo test`（VM --test-threads=2）。
- **真机复测判据**（用户执行）：用 v0.1.1 同款 5 任务重跑——
  - 贪吃蛇/五子棋/象棋/中国象棋：至少能写出首个文件（index.html / gomoku.html / 等），不再 `missing 'path'` / `missing 'content'` 连撞。
  - 象棋类任务不再 245s 卡死（deepseek 读 body 失败有重试或 fail fast）。
  - 简单任务（hello.py / 1+1）正常。
- 重新打 tarball：`release/hearth-v0.1.2-x86_64-unknown-linux-gnu.tar.gz`（更新 install.sh VERSION 默认 + 手册）。

---

## 3. 范围外（明确排除）

- cgroup 非特权 delegation 真限制（环境配置，README 已写 `HEARTH_ALLOW_NO_CGROUP=1`）。
- 桌面版 / 多用户 / Win-macOS 包（路线图后续）。
- deepseek 模型本身质量（用 v4-flash 时规划 JSON 偶发解析失败 → `Failed to parse LLM task plan` fallback，已是容错，不在此轮修）。
