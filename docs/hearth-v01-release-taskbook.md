# Hearth v0.1 发布任务书（RELEASE）— 可装基线 + 用户真机验收

> 签发：2026-08-22（顶层 / 守门员）
> 前置：CLI 派 A PASS（9bc2715）+ 后端真智能 B2/B3 PASS（6c46f33）+ 安全纵深 RT4 PASS（bf67366）
> 用户决策（2026-08-22）：**先发 CLI 基线 + 自己 VM 实测**，桌面版暂缓（冰山比喻：最终产品出来才知道本体站不站得住）
> 目标 commit 建议：`hearth-v0.1-release`

---

## 0. 目标与判据锚点

目标：把 `hearth` 从"源码态"变成**你能在 VM Linux 一键装上、零配置直跑、安全限制默认生效**的可装基线 `hearth-v0.1`，然后**由你在真机跑一轮**确认可用性。

用户原话诉求回顾：
- "期望 CLI 像 codex/claude 那样：下载安装、打开终端输入即用。"（派 A 已落地，本次让它真能装）
- "最终期望项目安全、平稳落地，别在使用中出现各种报错及意想不到的风险。"（RT4 已收口，本次让安全默认 ON 且可验证）
- "只有最终产品出来才知道（后端冰山）站不站得住。"（本次交付 = 第一份可验收的"产品态"）

**判据**：本任务书过闸 ≠ 用户验收过闸。用户真机跑通（§5 验收路径无报错 + 跑通一个真实任务）才算 v0.1 收口。

---

## 1. 门禁（R1-R6，执行窗口交付 + 用户实测定稿）

| 门禁 | 内容 | 验收证据 |
|---|---|---|
| **R1** 源码安装可用 | `cargo build --release -p codex-cli` 产出 `hearth` + `codex` 别名；`install` 到 `~/.cargo/bin` 后任意目录可调用 | 执行窗口在干净 Linux 容器实测：build 0 + `hearth --version` / `codex --version` 均输出版本 |
| **R2** 预编译 release | 发一个 Linux x86_64 预编译 tarball（GitHub Release 或本地产物）；`bench/install.sh` 的 BASE_URL 从占位 `example.com` 改为**真实可下地址**（或暂用本地路径说明） | release 资产存在 + `install.sh` 能真下载安装（或文档明确"暂用 cargo install / 本地 tarball"） |
| **R3** 零配置直跑 | 首次 `hearth chat "<goal>"` 无 key → 可行动错误（非崩溃、非静默）；`hearth init` / `hearth config set api-key` 引导填 key | 实测：无 key 报错含明确指引；填 key 后直跑成功落盘产物 |
| **R4** 安全默认 ON 且可验证 | 默认 KILL 化 + cgroup 限制默认启用（fail-closed）；VM 非特权 delegation 下给出 `HEARTH_ALLOW_NO_CGROUP=1` / `HEARTH_CGROUP_BASE` 明确指引（非静默降级） | 实测：正常环境 KILL 生效；delegation 环境按指引降级后跑通；README 写清环境变量 |
| **R5** 文档齐全 | `README.md` 含：安装（cargo/预编译）、配置（三层优先级）、安全（KILL/cgroup/readonly + 降级开关）、Observer（note 通道）、反馈（主动 y/n/补话） | README 存在且覆盖上述；与 `docs/hearth-cli-design.md` 一致 |
| **R6** 通用门禁 | fmt 0 / clippy 0 / test 全绿（基线 251）/ build 0 | CI/local 全绿 |

---

## 2. 任务分解

### A. 发布打包（R1-R2）

- **源码安装**：确认 `cargo build --release -p codex-cli` 产出双 bin（hearth + codex 别名，派 A 已做）；写 `README` 安装段。
- **预编译 release**：打 Linux x86_64 tarball（`target/release/hearth` + `codex` + `README` 片段）；
  - 若发 GitHub Release：`bench/install.sh` 的 `BASE_URL` 改为真实 release 下载 URL；
  - 若暂不发（用户 VM 内网/无公网）：`install.sh` 改为支持 `LOCAL_TARBALL=/path/to/hearth.tar.gz sh install.sh` 本地安装模式，README 写明两种路径。
  - **铁律**：不得留 `example.com` 占位——要么真地址，要么本地模式 + 文档说明。

### B. 零配置与引导（R3）

- 确认 `require_api_key()`（run_local.rs:43/56）在无 key 时给**可行动错误**（含 `hearth init` / `hearth config set api-key` 提示），非 panic/回溯。
- `hearth init` 交互式填 key 流程可用（若未实现，至少 `config set` 路径文档清晰）。

### C. 安全默认与降级指引（R4）

- 默认 KILL 化（`HEARTH_SECCOMP_MODE` 不设为 errno 即 kill）——确认默认路径。
- cgroup 默认 fail-closed；README "安全" 段写明：
  - 正常 Linux（root / 完整 cgroup v2 delegation）：内存/CPU/pids 硬上限自动生效；
  - VM 非特权 delegation：`HEARTH_CGROUP_BASE=<delegation 子树>` 或 `HEARTH_ALLOW_NO_CGROUP=1` 显式降级（非静默）；
  - 调试：`HEARTH_SECCOMP_MODE=errno` 白名单外 EPERM 而非杀。

### D. 文档（R5）

- 新建/补全 `README.md`：安装 / 配置 / 安全 / Observer / 反馈通道 五段，与 `docs/hearth-cli-design.md` 口径一致。
- 契约 v1.2 已补注 sandbox_violation，无需再动。

### E. 用户真机验收路径（§5，由用户执行，非执行窗口）

- 写清"你在 VM 跑的 5 步"，作为 v0.1 收口判据（见 §5）。

---

## 3. 红线（守门员铁律）

1. **不信报告信源码**：R1-R6 每个门禁须有真机/容器实测证据，禁止"应该能装"。
2. **不静默降级**：安全限制不可用必须显式报错或要求用户明确选择降级（RT4 已定）。
3. **install.sh 不留占位**：`example.com` 必须替换为真实地址或本地模式 + 文档。
4. **契约不动**：本次纯发布，无事件流变更，无需改契约（v1.2 已是最新）。
5. **桌面版不在本轮**：B4-2 Tauri 待 v0.1 真机验收过闸后再开。

---

## 4. 交付清单（执行窗口须提交）

- `README.md`（安装/配置/安全/Observer/反馈）。
- 预编译 tarball（或本地安装模式 + 文档）。
- `bench/install.sh` 去占位（真实 URL 或 LOCAL_TARBALL 模式）。
- 验收报告 `docs/acceptance-hearth-v01-release-<date>.md`（含真机/容器实测输出）。

---

## 5. 用户真机验收路径（v0.1 收口判据 — 你来执行）

在 VM Linux 跑（root 或完整 cgroup delegation 优先）：

```bash
# 1. 安装（二选一）
cargo install --path crates/codex-cli        # 源码安装
# 或
sh install.sh                                # 预编译（URL/本地已就绪）

# 2. 版本确认
hearth --version && codex --version

# 3. 零配置报错（应给可行动错误，不崩溃）
hearth chat "hi"
# → 应提示设 key

# 4. 配 key 后直跑一个真实任务
hearth config set api-key <你的key>
hearth chat "用 Rust 写一个 add 函数并加测试" --budget 6
# → 应看到 🗺 规划草案 + ❓/🤖 + 产物落盘

# 5. 安全验证（可选但推荐）
# 观察日志：seccomp KILL 化 + cgroup 限制是否生效；delegation 环境按 README 设 HEARTH_CGROUP_BASE
```

**收口标准**：步骤 2/4 无报错跑通 + 产物真实落盘 = `hearth-v0.1` 真机验收 PASS。届时由顶层出"v0.1 收口确认"，再开桌面版（B4-2）。

---

## 6. 范围外（明确不属本轮）

- 桌面版（B4-2 Tauri）——v0.1 真机验收后开。
- 多用户/公开部署——战略已排除。
- 非 Linux 平台预编译——代码分支保留，本次不打包。
