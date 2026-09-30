# P0-02 战报 · 注入与 SSRF 安全加固 v1.0

- **出品**：traecode　**日期**：2026-10-01　**授权**：用户全权
- **基线**：`p0-usability-01` @ `1729646`（= 远端 `origin/main`）
- **方法**：两路并行只读审计（`crates/service` HTTP 层 + `crates/tools-builtin` 全部工具）+ **关键结论亲验**
- **结论先行**：查出 **9 项确证缺陷**（其中 **4 项为注入/SSRF 类安全缺陷**）。**本卡修 4 项**：
  `open_external` 命令注入 / `grep` 选项注入 / `web_fetch` 重定向 SSRF /
  `webhook` 选项注入 + SSRF。其余 5 项按严重度立卡（见第五节）。**全部为新增发现**，与此前修复无重叠。

> 本卡延续 P0-01 的方法论：**"测试全绿"不等于"没有缺陷"**——本卡的 4 项安全缺陷，
> 在 61 个 target 全绿、clippy 干净的状态下**全部存在**。

---

## 一、修复 1 · `open_external` 命令注入（**认证用户 → 宿主机任意命令执行**）

**位置**：`crates/agent-runtime/src/session.rs:19-36`（原实现）

```rust
#[cfg(target_os = "windows")]
let mut cmd = {
    let mut c = std::process::Command::new("cmd");
    c.arg("/C").arg("start").arg("").arg(target);   // ← target 来自请求参数
    c
};
...
cmd.arg(target);                                     // ← 又加一次（次生缺陷）
```

**触发**：`GET /api/v1/sessions/:id/artifact/open-external?path=http://a%26calc.exe`
—— Query 解码把 `%26` 还原成 `&`，`cmd` 视其为**命令分隔符** → 第二条命令被执行。

**为什么严重**：API key 的语义是"允许驱动 agent"，不是"允许在宿主机执行任意命令"。
这是**权限越界**，不是便利功能。

**修复**：
1. **不再经 shell** —— Windows 改用 `explorer.exe`（`std::process::Command` 走 CreateProcess
   直接传参，无元字符解析）；
2. 去掉重复传参（原 Windows 分支 target 被传两遍）；
3. 新增 `validate_open_target()` 前置校验（拒绝空串、控制字符、双引号）。

---

## 二、修复 2 · `grep` 选项注入（可升级为命令执行）

**位置**：`crates/tools-builtin/src/grep.rs`（原实现在 `execute` 内联构造 argv）

```rust
argv.push(pattern.to_string());     // ← 无 `--` 分隔
argv.push(search_path.clone());
```

**触发**：
- `pattern = "--help"` → rg 打印帮助并 exit 0，被上层**误当"命中结果"返回**；
- 更危险：`pattern = "--pre"` → rg 的 `--pre COMMAND` 会**对每个文件执行该命令**，
  而其后的位置参数正是**用户可控的 `path`** → 可构成命令执行。

**修复**：在 pattern 前插入 `--` 终止选项解析；同时把 argv 构造**抽为纯函数
`build_search_argv`**，并新增单测 `test_search_argv_has_option_separator`
（断言 `--` 存在、位置正确、且 `--glob`/`--include` 仍在 `--` 之前）。

**先红后绿**：`tools-builtin` 单测 81 → **82 passed**。

---

## 三、修复 3 · `web_fetch` 重定向绕过出网白名单（SSRF）

**位置**：`crates/tools-builtin/src/web.rs:122-125`（原实现）

原实现只对**初始 URL** 做 `egress_allowed` 校验，随后用 reqwest **默认策略**发请求
——默认策略**自动跟随最多 10 次重定向且不复检白名单**。

**触发**：白名单放行某个可控来源页 → 该页 302 到 `http://169.254.169.254/…`（云元数据）
或 `http://127.0.0.1:…`（本机服务）→ **请求已经发出**，出网治理被一条 302 绕过。

**修复**：自定义重定向策略，**每一跳都复检白名单**，不通过即中断，并限跳 5 次。

---

## 三·补、修复 4 · `webhook` 选项注入 + SSRF（比代理初报更严重）

**位置**：`crates/service/src/webhook.rs`（`fire`）

```rust
let cmd = tokio::process::Command::new("curl")
    .args(["-s", "-X", "POST", "-H", "…", "-d", &body, &url])   // ← url 原样进 argv
```

**问题 ①（比"SSRF"更严重）：curl 选项注入 → 宿主机文件读写原语。**
`url` 虽是最后一个位置参数，但 **curl 会在整个 argv 中解析选项**——只要 `url` 以 `-` 开头
即被当作选项。例如 `url = "-K/tmp/evil.conf"`（curl 读该配置文件 → 注入任意 curl 选项）
或 `url = "-o/tmp/out"`（把响应体写到任意路径）。注册一个 webhook 即可获得**宿主机文件读写**。

**问题 ②：SSRF。** 服务端代注册者发请求，可打内网 / 云元数据（`169.254.169.254`）。

**修复**：
1. 新增纯函数 `validate_webhook_url(raw, allowlist)` —— 强制 `http://`/`https://` 前缀
   （同时挡掉 `-K…` 这类非 URL 的选项注入）、必须有 host；不通过则**跳过并告警**（不静默）；
2. **仅在显式设置 `HEARTH_EGRESS_ALLOWLIST` 时**按白名单收紧（支持 `*.domain` 写法）。
   **默认不阻断**——webhook 的语义本就允许指向任意外部服务，强加白名单会破坏功能本身；
   与 README「不设/空 = 全放」口径一致。
3. argv 中 url 前加 `--` 终止选项解析（纵深防御）。

**回归测试** `test_webhook_url_validation`（含选项注入、非法 scheme、白名单命中等 11 个断言）；
`service` 单测 4 → **5 passed**。

---

## 四、门禁

| 项 | 结果 |
|---|---|
| `cargo fmt --all` | 干净 |
| `cargo clippy --workspace --all-targets` | **exit 0** |
| `cargo test --workspace --no-fail-fast` | **exit 0**，**61 target 全绿，零失败** |
| `tools-builtin` 单测 | **82 passed**（+1 新回归测试） |

**红线自查**：未触碰沙箱隔离语义；未动 `a_arm_act_tally`；未动 `llm-gateway/fallback.rs`；
未复活 S5；**未使用/未验证任何泄露密钥**；未做任何破坏性 git 操作。

---

## 五、已确证但**本卡未修**（立卡，按严重度排序）

| # | 缺陷 | 位置 | 后果 | 级别 |
|---|---|---|---|---|
| 1 | **会话泄漏 → 无界内存+磁盘** | `agent-runtime/src/session.rs:320`、`:935-961` | 从不 `send_message` 的会话 `finished_at` 恒 `None`，`cleanup_finished` 一律保留；每会话还建 `sessions/{uuid}` 目录 | **高（DoS）** |
| 2 | **输出全量入内存 → OOM** | `sandbox/src/lib.rs:1204-1217` | 读线程 `read_to_end` 无上限，截断发生在之后；`bash("yes \| head -c 2G")` 可打爆主进程 | **中高** |
| 3 | **cgroup 失败路径泄漏子进程** | `sandbox/src/lib.rs:1185-1192` | `apply_cgroups_impl` 失败时 `Child` 被 drop（Rust 不 kill 不 wait）→ 孤儿/僵尸 | **中** |
| 4 | **NoopSandbox 超时不杀进程组** | `sandbox/src/lib.rs:190-193` | Windows 走此路径，`bash -c "sleep 100 &"` 的孙进程成孤儿 | **中** |
| 5 | **`read` 无字节上限** | `tools-builtin/src/read.rs:105` | 限的是"2000 行"不是字节，单行超大（压缩 JS）会整行入内存 | **中** |

> **webhook 任意出网**原列本表第 1 项（**高**），已在同批修复 → 移入「三·补」。

**可疑需验证**（低置信，未展开）：
- `is_allowed_absolute_roots`（`tools-builtin/src/lib.rs`）放行 HOME 下任意绝对路径，
  其安全性**依赖 landlock 兜底**；而 Windows 走 NoopSandbox **无 landlock** →
  非 Linux 上 `write_file` 可写 HOME 内任意文件。**与我 P0-01 修的路径守卫属同一
  "Windows 双重防线失效"根因**，建议下一卡专项复核。
- `service` 请求路径上的 `.expect()` / `lock().unwrap()`（panic 仅终结该连接任务，非整进程）。
- 内部错误细节（含文件系统路径）直接 `format!("{e}")` 回客户端。

---

## 六、方法论沉淀（写入规划）

**本卡再次验证 P0-01 的结论，并推到更强：**

> **"61 个 target 全绿 + clippy 零警告"** 与 **"没有注入/SSRF 缺陷"** 之间**没有推论关系**。
> 本卡 3 项安全缺陷 100% 存在于全绿状态下——因为现有测试**从不断言安全边界**：
> 它测"grep 能找到模式"，不测"grep 不会被 pattern 当选项"；测"能打开 URL"，不测"打不开内网"。

**可复用建议（未实施）**：为工具层补一类**负向安全测试**（注入/越界/SSRF 的"必须失败"断言），
与现有正向功能测试并存。这正是 P0-01 那条"缺少接线断言"的同类缺口——**缺的是边界断言**。
