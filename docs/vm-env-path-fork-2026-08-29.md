# VM 环境诊断：为什么 ssh 进去 `hearth --version` 是 v0.2.3

> 评审窗口 · 2026-08-29 · 双 VM（`.131` 执行 / `.133` 评审）实测
> 性质：**环境层分叉**，非代码缺陷。但污染测试结论，必须先治理。

---

## 0. 结论速览

| 问题 | 结论 |
|------|------|
| 你 ssh 进去看到 v0.2.3，是版本没装对吗？ | **不是。** `/usr/local/bin/hearth` 是 **v0.2.8**（md5 `78850a05…`）。 |
| 那 v0.2.3 哪来的？ | **`~/.cargo/bin/hearth`** —— `cargo install` 留下的 v0.2.3 副本，被 `~/.bashrc` source 的 cargo env 顶到 PATH 第一位。 |
| 两台 VM 都有吗？ | **是，完全一样。** |
| 只是版本号显示问题吗？ | **不是。** 同一处 `.bashrc` 还藏着两个**只在交互式终端生效**的关键 env，直接污染测试结论。 |

---

## 1. 三层 `hearth` 二进制清单（两 VM 完全一致）

| 路径 | 版本 | md5 | 大小 | mtime | 判定 |
|------|------|-----|-----:|-------|------|
| `~/.cargo/bin/hearth` | **0.2.3 (5a0be40)** | `8214e54450395f84f48ba208b89532b5` | 8139408 | 08-26 05:30 | 🔴 **陈旧副本，PATH 第一优先级** |
| `/usr/local/bin/hearth` | **0.2.8** | `78850a05cc03502db0a9284b8e03c096` | 8461488 | 08-28 18:49 | ✅ 正确（与本机 release 一致） |
| `/usr/local/bin/codex` | 0.2.3 (5a0be40) | `d3585bc962a13a8b34b76d90a9160f0a` | 8139408 | 08-26 05:30 | 🟡 遗留副本（与 cargo/bin 同批同大小同时间） |
| `/usr/bin/codex`、`/bin/codex` | codex-cli **0.145.0** | `901f9554…` | — | — | ✅ **对标基准，保留勿删** |

`~/.cargo/bin/` 下 `codex` 与 `hearth` 是**同一次 `cargo install` 的两个名字**（同 8139408 字节、同 08-26 05:30:57）。

## 2. PATH 分叉：登录 shell vs 非登录 shell

```
登录/交互式 shell（你 ssh 进去敲命令）：
  PATH = /home/wutao/.cargo/bin : /home/wutao/.local/bin : /usr/local/sbin : /usr/local/bin : …
         ↑ 命中 ~/.cargo/bin/hearth → v0.2.3

非登录 shell（我的 paramiko exec_command、nohup、CI、任何脚本化调用）：
  PATH = /usr/local/sbin : /usr/local/bin : /usr/sbin : /usr/bin : …
         命中 /usr/local/bin/hearth → v0.2.8
```

实测复现（`.131`）：

| 调用方式 | `hearth --version` |
|----------|-------------------|
| `bash -lc 'hearth --version'` | **`hearth 0.2.3 (5a0be40)`** |
| `bash -c  'hearth --version'` | `hearth 0.2.8 (unknown)` |
| `bash -lc 'which -a hearth'` | `/home/wutao/.cargo/bin/hearth` → `/usr/local/bin/hearth` |

**这就是我上一轮判断失误的原因**：我用非登录 shell 探测，拿到了 v0.2.8，于是误判"你一定是敲了 `codex`"。**实测方式与被复现场景不一致，测出来的结论就是假的。**

---

## 3. 更严重的连带发现：`.bashrc` 里的 env 只在交互式终端生效

`~/.bashrc` 第 118–130 行（两 VM 相同）：

```bash
export OPENAI_API_KEY="sk-omwq7hA65YxmAXuUiDuKlyGlplSAOBsZ09BUF454iwJweNwx"
export OPENAI_API_KEY="cpk-TbY5hUhEuA9CJpzagi4yB2SO90gs0RdoLHfKAi0Y1erVdC2N"   # 覆盖上一条
export OPENAI_API_KEY="cpk-TbY5hUhEuA9CJpzagi4yB2SO90gs0RdoLHfKAi0Y1erVdC2N"   # 再覆盖
export PATH="$HOME/.local/bin:$PATH"
. "$HOME/.cargo/env"
export APIHUB_AGNES_AI_API_KEY=cpk-TbY5hUhEuA9CJpzagi4yB2SO90gs0RdoLHfKAi0Y1erVdC2N
export HEARTH_CGROUP_BASE=/sys/fs/cgroup/hearth
export HEARTH_EGRESS_ALLOWLIST=rust-lang.org,crates.io,docs.rs,doc.rust-lang.org,github.com
alias hearth-cli="bash ~/.local/bin/hearth"
```

**但实测 `bash -lc 'echo $HEARTH_CGROUP_BASE'` 返回空** —— Ubuntu 默认 `.bashrc` 开头有非交互早退（`case $- in *i*) ;; *) return;; esac`），**只有真正的交互式终端才会执行这一段**。

### 3.1 后果一：cgroup base 拿不到 → 直接污染 RT4/沙箱测试

| 场景 | `HEARTH_CGROUP_BASE` |
|------|---------------------|
| 用户交互式 `hearth chat` | `/sys/fs/cgroup/hearth` |
| 任何脚本化调用（含我的探针、执行窗口的测试脚本、nohup） | **空 → 回落默认 `/sys/fs/cgroup`** |

补充实测：`/sys/fs/cgroup/hearth` **目录已存在**（08-28 23:48 创建，属主 `wutao`，含 `cgroup.controllers` 等）。
→ 说明已有人建了委派目录，**之前记忆里"10 个 sandbox 测试因 cgroup 权限失败"的前提可能已经变了，需要重测**。

### 3.2 后果二：出网白名单拿不到 → **这就是 RC12「35 拒 0 成功」的最后一公里**

`.bashrc:125` 明明白白配了：
```
HEARTH_EGRESS_ALLOWLIST=rust-lang.org,crates.io,docs.rs,doc.rust-lang.org,github.com
```

但 `web.rs:91-96` 只读 **`ctx.env`**，而 `ctx.env` 由 `run_local.rs:219-228` 注入，来源是：
- config.toml 的 `egress-allowlist`（**未配**）
- 进程 env `HEARTH_EGRESS_ALLOWLIST`（**非登录 shell 下为空**）

→ **ctx.env 里没有 → deny-by-default → 全拒。**

**上一轮我说 RC12 的"最后一公里未闭合，需要受控复现"—— 现在闭合了。** 白名单一直配着，只是从未到达它该到的地方。

**注**：这是第三例同型故障（前两例：完成语义的 `status` 不渲染、压缩摘要丢弃 97% 事实）—— **配置/事实存在于某处，但没投影到真正消费它的那一层。**

### 3.3 后果三：`alias hearth-cli` 是死的

`.bashrc:126` → `alias hearth-cli="bash ~/.local/bin/hearth"`
但 `~/.local/bin/hearth` **不存在**（该目录下只有 `hearth-cli`(820B 文本)、`hearth-config`、`hearth-config-manager.py`、`hearth-log`、`hearth-log-analyzer.py`、`codex-relay`）。

### 3.4 ⚠️ 安全提示：`.bashrc` 里的 Agnes key 与记忆中的不一致

`.bashrc` 内 3 处 key：
- `sk-omwq7hA65YxmAXuUiDuKlyGlplSAOBsZ09BUF454iwJweNwx`（被后两条覆盖）
- `cpk-TbY5hUhEuA9CJpzagi4yB2SO90gs0RdoLHfKAi0Y1erVdC2N` ×2（OPENAI_API_KEY + APIHUB_AGNES_AI_API_KEY）

**记忆里登记的 Agnes key 是 `cpk-f4UBH3NaHUUN2SAcW1LhZT0kyuqlu9TF5lTItbuXHSTP3w0c`——两者不同。**
需你确认哪个有效；失效的那个应从 `.bashrc` 清除（明文 key 落盘且被覆盖两次，本身也是隐患）。

---

## 4. 治理建议（待你/顶层拍板，评审窗口不执行改动）

| # | 动作 | 影响面 | 建议 |
|---|------|--------|------|
| 1 | 删 `~/.cargo/bin/hearth`（保留 `codex`，或一并删） | 交互式 `hearth` 立刻变 v0.2.8 | 🟢 建议做 |
| 2 | 把 `HEARTH_CGROUP_BASE` / `HEARTH_EGRESS_ALLOWLIST` 从 `.bashrc` 移到**非交互也能加载**的位置（如 `~/.profile` 的非早退段，或显式写进 config.toml） | 消除"手工能通、脚本不通"分叉 | 🔴 建议做 |
| 3 | `egress-allowlist` 写进 `~/.config/hearth/config.toml` | web_fetch 在非交互场景也能用，彻底闭合 RC12 | 🔴 建议做 |
| 4 | 修/删 `alias hearth-cli`（指向不存在的路径） | 消除死 alias | 🟡 可选 |
| 5 | 清理 `.bashrc` 明文 key + 三处重复覆盖 | 安全 + 不再误导 Agnes 通道判断 | 🟡 建议做 |
| 6 | 把 `provider` 从 deepseek 切到 Agnes（现配置 100% 烧 deepseek 且踩 read-body 故障通道） | 成本 + 稳定性 | 🔴 建议做 |
| 7 | 保留 `/usr/bin/codex`、`/bin/codex`（codex-cli 0.145.0 对标基准） | — | ✅ **勿动** |

---

## 5. 我自己的方法论失误（记录，避免再犯）

上一轮我用**非登录 shell** 探测，得到 v0.2.8，据此判断"你看到的 v0.2.3 是 `codex` 副本"。
**错了。** 真实原因是 `~/.cargo/bin/hearth`。

**教训升级**：
1. 用户转述的观察，必须实测后再写进记忆（已记）
2. **实测方式本身必须与被复现场景一致** —— 环境类问题，非登录 shell 的结果不能代表交互式终端。**探测前先确认：用户是怎么跑的？**
3. 同一类信息（`hearth` 二进制）要**穷举所有 PATH 入口**再下结论，不能只查 `which -a` 在一种 shell 下的结果
