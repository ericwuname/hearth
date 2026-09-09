# Hearth v0.1.1 · VM 更新操作手册

> 适用：你 VM（Linux x86_64，用户 `wutao@192.168.220.131`，Ubuntu 24.04）
> 目的：把 VM 上装的 **v0.1.0 旧二进制** 换成 **v0.1.1 新代码**（修了 write_file 截断 bug + 渲染美化 + init 不回显 + REPL 直跑）
> 写于：2026-08-22（守门员）

---

## 0. 先说结论：你该怎么做

**不用"卸载重装整包"。** 你 VM 上之前是通过 `tar -xzf` + `sudo cp /usr/local/bin` 装的，本质就是把 `hearth` / `codex` 两个二进制丢进 `/usr/local/bin`。

v0.1.1 只改了二进制代码（没改配置格式、没改契约、没改目录结构），所以**最干净的做法 = 用新代码重新编译出二进制，替换掉 `/usr/local/bin/hearth` 和 `/usr/local/bin/codex` 这两个文件就行**。配置 `~/.config/hearth/config.toml` 不用动（你之前 init 填的 key 还在）。

---

## 1. 两条路线（选一条）

### 路线 A：VM 内 git 拉新代码 + 源码编译（**推荐，最稳**）

VM 是 Linux，有 cargo，直接在本机编译最省事，且沙箱/安全特性只在 Linux 真验。

```bash
# ① 进你之前放代码的目录（或重新 clone）
# 如果你之前是 scp tarball 进来的，没 git 仓库，就重新 clone：
cd ~
git clone https://github.com/ericwuname/codex-rust-v1.0-final.git hearth-src
cd hearth-src

# 如果你之前已经 clone 过，直接拉最新：
# cd ~/hearth-src && git pull

# ② 确认在 v0.1.1 代码上（应有 83fbf9d 这个 commit）
git log --oneline -1
# 期望看到：83fbf9d feat(repl): 直跑模式 REPL ...

# ③ 编译 release（只编 CLI，~1-2 分钟，取决于 VM 性能）
cargo build --release -p codex-cli
# 编译产物在：target/release/hearth 和 target/release/codex

# ④ 替换 VM 上已装的旧二进制
sudo cp target/release/hearth /usr/local/bin/hearth
sudo cp target/release/codex  /usr/local/bin/codex
# （如果之前没装 codex 别名，这步也顺便装上了）

# ⑤ 验证版本 + 新代码生效
hearth --version
# 期望：hearth 0.1.2 (83fbf9d)  ← 版本号 + commit hash（R3 v0.1.2 起）
# ⚠ 不要用 `strings /usr/local/bin/hearth | grep 函数名` 验证——release 二进制
#   符号被 strip，Rust 标识符不在字符串表（v0.1.1 用户踩坑：strings 查不到 ≠ 没修复）。
#   正确姿势：看 --version 的 hash + 跑 hello.py 短任务看 write_file 成功。
```

### 路线 B：本机重新出 tarball 再 scp（**你本机走不通，标红**）

本机是 **Windows，且没装 cargo/rustup**，无法交叉编译 Linux 包。所以这条目前不可行——除非你本机装了 Rust + `rustup target add x86_64-unknown-linux-gnu` + 交叉链接器（musl 或交叉 gcc），配置成本高于直接在 VM 编。**建议走路线 A。**

> 如果以后要出正式预编译包给别的环境用，流程是（需在 Linux 或配置好交叉的机器上）：
> `cargo build --release -p codex-cli` → 打 `release/hearth-v0.1.1-x86_64-unknown-linux-gnu.tar.gz` → 更新 `bench/install.sh` 的 VERSION 默认 → 发布到 GitHub Release → 用 `HEARTH_BASE_URL=... sh install.sh` 远程装。

---

## 2. 零配置 / 配置验证（确认没回退）

```bash
# ① 零配置报错（应该给你 3 步下一步，不是崩溃、不是静默）
hearth chat "hi"
# 期望看到：未配置 API key。下一步：① hearth config set ... ② export HEARTH_API_KEY=... ③ hearth init

# ② 你的 key 还在吗（之前 init 写进去了，不该丢）
cat ~/.config/hearth/config.toml
# 期望能看到 provider / api_key / base_url 三行

# ③ 若 config 丢失或想重配（v0.1.1 改进：key 不回显 + 三问分隔）
hearth init
# 交互：① provider [deepseek]: 直接回车  ② API key（粘贴不回显）: 粘贴  ③ base URL: 直接回车
```

---

## 3. 安全降级（你 VM 是非特权用户，必看）

你 VM 是普通用户 `wutao`，**没有 cgroup 写权**，所以默认 cgroup 限制会 fail-closed 退出（这是设计，不是 bug）。

```bash
# 带显式降级开关跑（你之前验证过能跑通）
HEARTH_ALLOW_NO_CGROUP=1 hearth chat "写一个贪吃蛇的 html 小游戏，包含 index.html style.css game.js 三个文件"

# 不带开关会直接报错退出（符合"安全限制无法保证就别偷偷跑"的设计）：
# hearth chat "..."  →  报错：安全限制无法保证（cgroup 不可用），用 HEARTH_ALLOW_NO_CGROUP=1 显式确认降级
```

启动时应看到隔离徽章 `🔒`（seccomp KILL 化 + landlock 默认 ON）。

---

## 4. 重跑真机闸门（验证 v0.1.1 修复真生效）

这是你亲手能验证"冰山本体修好"的判据——之前 v0.1.0 只写出 1383 字节 index.html 就 budget exhausted，v0.1.1 应写出完整三文件。

```bash
mkdir -p ~/snake_game && cd ~/snake_game

HEARTH_ALLOW_NO_CGROUP=1 hearth chat "写一个贪吃蛇的 html 小游戏，分三个文件：index.html（结构）、style.css（样式）、game.js（逻辑），要能直接在浏览器打开玩"

# 跑完后检查：
ls -la ~/snake_game
# 期望：index.html + style.css + game.js 三个文件都在，且 index.html 不是截断的
wc -l ~/snake_game/index.html
# v0.1.0：只有 1383 字节（截断）
# v0.1.1：应 ~474 行 / 11834 字节（完整）

# 浏览器打开 index.html 看能不能玩
```

**判据**：三文件齐全 + index.html 完整无截断 + 游戏能玩 = v0.1.1 修复 PASS，v0.1 真机收口成立。

---

## 5. 试试新加的 REPL（顺手交付）

```bash
# 直跑模式（无 --url）：本地多轮交互，reedline 行编辑 + 上下键历史
HEARTH_ALLOW_NO_CGROUP=1 hearth repl
# 提示符：hearth>  输入目标回车 → 跑 → 渲染 → 再输入
# 退出：/quit 或 /exit 或 /q 或 Ctrl-D

# 对比以前的一次性模式：
HEARTH_ALLOW_NO_CGROUP=1 hearth chat "你的目标"
```

---

## 6. 回滚（万一新代码有问题）

```bash
# 旧二进制你之前是 scp tarball 装的，如果还留着 tarball：
# tar -xzf <旧tarball> && sudo cp hearth /usr/local/bin/hearth && sudo cp codex /usr/local/bin/codex
# 或者 git 回退到 v0.1.0 的 commit 重新编：
cd ~/hearth-src && git stash && git checkout <v0.1.0 的 commit> && cargo build --release -p codex-cli
```

---

## 7. 已知遗留（不影响你用）

- `hearth --version` 仍打 `0.1.0`（version 字符串未 bump，二进制已是 v0.1.1 代码）。
- bash 工具在 seccomp KILL 化下特定命令偶发 `exit=-1`（sandbox 内 bash 多数正常，贪吃蛇可跑通；若复现再排查）。
- 非 Linux 无真实沙箱（你 VM 是 Linux，不受影响）。
