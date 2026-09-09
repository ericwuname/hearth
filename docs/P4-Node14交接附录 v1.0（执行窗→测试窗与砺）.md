# P4 Node 14 交接附录 v1.0（执行窗 → 测试窗 / 砺·评审）

**发出方**：执行窗（Node 13 施工者）｜**日期**：2026-09-01｜**版本基线**：**v0.2.21**
**性质**：交接与可验证据，**不含疗效判读**（判读分离：α/β/γ 与 fresh 失败率结论归测试窗/顶层/外部 AI）。

---

## 1. 复测基线（provenance，可直接引用）

| 项 | 值 |
|---|---|
| 版本 | `0.2.21`（Cargo.toml:34；`hearth --version` = `hearth 0.2.21 (unknown)`） |
| commit / tag | `1366710` / `v0.2.21`（master） |
| 部署 | `.131` `/usr/local/bin/hearth`，md5 `5e5ac1a327d806f27af77b2e8b89f14e`（8,678,496 B） |
| 旧件备份 | `/usr/local/bin/hearth.v0.2.20.bak`（md5 `33444d79…`），需要回退直接 cp 回 |
| 门禁 | fmt ✅ / clippy `-D warnings` ✅ / `cargo test --workspace` **460 passed · 0 failed · 61 targets** |
| 测试环境 | `.131`（测试窗既定环境）；**`HEARTH_ALLOW_NO_CGROUP` 不要设**（见 §4） |

## 2. 本版行为变更（测试窗需要知道的三件事）

1. **RC52 回填**：跨 run 不再销毁产物事实。预期效果 = 「已验证完成的任务收到『继续』」不再假阴性 failed。**判据见 §3 预注册。**
   - 触发时会在诊断日志留下 `RC52_ARTIFACT_HYDRATION`（`~/.config/hearth/diagnostics.log`）——**该行的出现次数可作为回填发生频次的客观计数**，建议纳入采集。
   - 注意：回填只恢复「路由资格」，产物存活性仍由 Done 相位确定性校验裁决（文件被删/为空照常打回）——**不要把回填误读为"免检"**。
2. **截断标记**：宪法注入 / goal_drift 输入 / failed_nodes 摘要 / CLI 诊断错误，截断时都会带 `\n[... N chars truncated]` 尾注。若日志中出现该标记，说明对应文本被截断——**请记录其出现位置与 N**，这是信息销毁的可见化产物（非新缺陷）。
3. **R-1 仪器**：session id 改由 sessions 目录文件系统 diff 提取（见 §5 验证证据）。

## 3. 预注册预期（防事后叙事）

- **主判据**：v0.2.21 上 fresh（A 条件、REPL「继续」链路）失败率应**显著低于 40%**。
- **反向信号**：若 fresh 仍 ~40%，视为**修复无效**，回炉重查（不要再追加补丁掩盖）。
- 分母锁定不变：**REPL 内「继续」（无显式 goal 恢复）** 才是病灶链路；`hearth resume`（显式 revision 1）结构性绕过病灶，不能用来替代 A 条件。
- RC52 是概率性吸引子（同二进制同输入曾出现 completed/failed 两种终态），**40% 不可当精确门槛**，只能看方向与非重叠区间。

## 4. 环境规程更正（本窗实测，请照此执行）

- **跑测模板**：`source ~/.cargo/env && unset HEARTH_URL && export HEARTH_CGROUP_BASE=/sys/fs/cgroup/hearth`（**不设 `HEARTH_ALLOW_NO_CGROUP`**——与 sandbox `test_rt4_cgroup_fail_closed` 前提互斥，设了必红；早前"3 测试需此变量"是误判，真因并行污染）。
- **⚠️ 该规矩有作用域，别套错地方**（本窗先前的说法过宽，此处更正并给出实测依据）：

| 场景 | `HEARTH_ALLOW_NO_CGROUP` | 依据 |
|---|---|---|
| **`cargo test`（门禁）** | **不要设** | sandbox `test_rt4_cgroup_fail_closed` 前提要求该变量不存在（设了必红）。已实测 agent-core 126/126、sandbox 22/22 在不设时全绿 |
| **真机跑批 / campaign（hearth chat、PTY driver）** | **必须设 `=1`** | 见 §5.6——不设则 bash 被 RT4 fail-closed 拒绝，轨迹与基线不同。v0.2.20 基线（40%/100%）也是在设了该变量的条件下采集的（`run_rc52_matrix.py` 的 `env()` 内含此项） |

- **C 条件（resume）**：R-1 仪器已修并干跑验证（§5），**仍 BLOCKED**——执行窗不自解封，请砺复核后放行。

## 5.6 🔴 可比性混淆变量：该变量直接改变任务轨迹（A/B 实测）

同任务、同二进制 v0.2.21、同 provider，只改这一个环境变量：

| `HEARTH_ALLOW_NO_CGROUP` | bash 工具 | cgroup 拒绝次数 | 步数 | 终态 |
|---|---|---|---|---|
| 未设 | **fail-closed 拒绝**（`无法创建 cgroup … Permission denied (os error 13)`，RT4） | 1 | **5 步** | Task completed |
| `=1` | 正常 | **0** | **9 步** | Task completed |

**结论**：不设该变量 → bash 不可用 → agent 只能走 write_file 等路径 → **轨迹与基线不同（5 步 vs 9 步）**，RC52 的 give_up 触发条件随之改变。**Node 14 必须沿用 v0.2.20 基线的设置（=1）**，否则与 40%/100% 的对照不可比。
（注：任务仍能完成，属降级可用，不是缺陷；问题只在**口径一致性**。）

## 5. R-1 仪器干跑证据（供砺复核，可直接复跑）

脚本：`/tmp/r1_instrument_dryrun.py`（.131，隔离临时目录，**未触碰真实会话、未跑 LLM**）。
复跑：`python3 /tmp/r1_instrument_dryrun.py`（退出码 0 = 全过）。

| 判据 | 结果 |
|---|---|
| R1-1 空目录快照为空；无变化时返回空串（DRIVER-INDUCED 断言可触发） | PASS |
| R1-2 新增 36 位 uuid 会话可取回 | PASS |
| R1-3 旧会话不动 + 新增 → 只返回新增者（不误取旧会话） | PASS |
| R1-4 既有会话被追加（mtime 变化）也能检出 | PASS |
| R1-5 连开两个 → 返回 mtime 最新者 | PASS |
| R1-6 真实 `~/.config/hearth/sessions` 存在，且 **264 个 .jsonl 全部为 36 位 uuid**（fs-diff 前提成立） | PASS |

合计 **8/8 PASS**；静态自检确认 `assert uuid` 非空断言与旧正则 fallback 均在位。
仪器双落点 md5 一致：`0002871267aefa1effc7d3a1e1af74b7`（`~/codex/tools/simuser/` 与 `~/fa/simuser/`）。

## 5.5 🔴 开工前必读：provider 出口可达性（Node 14 阻塞级，2026-09-01 17:0x 实测）

测试窗**开工前先跑一遍连通性探测**，否则会把"网络失败"误记成"任务失败"（与 R-1 同类 DRIVER-INDUCED 污染）。

`.131` 实测（plain curl，与 hearth 无关）：

| 端点 | 结果 | 判定 |
|---|---|---|
| `https://api.agnes-ai.cn/v1/models`（Agnes，v0.2.20 基线所用） | **超时**（A/AAAA 均不通；A=172.64.191.148 为 Cloudflare） | 🔴 不可用 |
| `https://1.1.1.1`（Cloudflare 对照） | **超时** | 国际出口被拦，非 Agnes 单点故障 |
| `https://www.baidu.com` | 200 | 国内站点正常 |
| `https://open.bigmodel.cn/...`（智谱） | **401**（端点存活，需鉴权） | 🟢 可达 |
| `https://ark.cn-beijing.volces.com/...`（豆包） | **401** | 🟢 可达 |
| `https://generativelanguage.googleapis.com/...` | 超时 | 🔴 不可达 |

**现象佐证**：本窗 16:58 的中性冒烟中，planner 首次调用**成功**返回规划（steps=1），其后所有 LLM 请求 `error sending request` 并按设计指数退避重试至超时——即**链路是间歇性的，不是恒不通**（diagnostics.log 可查）。

### 5.5.1 已解决：用户挂 VPN，.131 出口改道（17:35 实测）

- **VPN 确实作用到了 .131**（不只是 Windows 本机）：出口 IP 由国内变为 **`45.88.202.26`（挪威 terrahost.no）**，路由下一跳 `192.168.220.2`。
- Agnes 探活 **3/3 = 401**（1.8–2.8s）；**端到端冒烟通过**：`✓ Task completed（9 步）`、**耗时 14s**、产物 `VPN.txt` = OK、cgroup 拒绝 **0** 次（已设 `ALLOW_NO_CGROUP=1`）。
- **结论**：网络阻塞解除，Node 14 可开工。派工单已出：`docs/P4-Node14测试派工单 v1.0（执行窗→测试窗口）.md`。
- **残留风险**：VPN 掉线 → 请求 `error sending request` → 该样本须判 `PROVIDER_UNREACHABLE` 并**出分母**（派工单 §5 已写明）；跑批前先探活。
- 附带的观测：出口改道后延迟未变差（冒烟 14s），但若出现批量超时，先怀疑 VPN 抖动而非代码。

### 5.5.2 二进制↔源码对齐（E7 最强反证）
- 全树归一后**重新 `cargo build --release`**，产物 md5 与**已部署的** `/usr/local/bin/hearth` **完全相同**：`5e5ac1a327d806f27af77b2e8b89f14e`。即：当前部署件 == 当前源码树 == tag v0.2.21 的代码内容。

### 5.5.3 挂起疑案：本次未复现
- 复现尝试：`cargo test --workspace --no-fail-fast`（限时 420s）→ **EXIT=0，460/0/61 正常完成**，未再出现 22 分钟挂起。
- 判定：**间歇性**（fail-fast 口径至今 4 次全绿、no-fail-fast 2 次中 1 挂 1 正常）。推测与子进程/socket 等待在并行负载下的竞态有关，**未根因定位**。
- 处置建议：**门禁继续用 fail-fast 口径**；不建议在 Node 14 期间花时间追这个（性价比低），留作观察项。
**判读与选项（裁决归顶层，执行窗不自行切换）**：
1. **等待重试**：间歇性故障，可能自行恢复；复测前先 curl 探活，通了再跑。
2. **换国内通道（智谱/豆包）**：可达，但**换了模型层 → 与 v0.2.20 基线（agnes-2.5-flash）的 40%/100% 不再可比**。RC52 是 decision 层（RC47 族）问题，机制结论可能仍成立，但**定量对照必须重新声明口径**，否则账目自相矛盾（与议题 β 联动）。
3. 无论选哪条，**采集侧必须区分「provider 不可达」与「任务失败」**——不可达样本不得计入 fresh 失败率分母。

## 6. 本窗已清理的隐患（减少复测噪声）

- **docs CRLF 存量污染已清**：.131 与本机 `docs/` 全 370 个文件**字节级一致**（此前 330 个不一致）。其中 326 个纯行尾差异；3 个真实内容差异（ledger / ledger-v2 / n1sbx 报告）经逐项 diff 判定**本机为新版**，按既定流向回灌（若顶层认为 VM 侧另有更新，请回滚这 3 个文件）。
- **根因已防复发**：新增 `.gitattributes`（`* text=auto eol=lf`）——Windows `core.autocrlf=true` 曾把文本转 CRLF 后回灌 Linux，导致 project-xray wiring FNV 锁失配。**待顶层追认**。
### 6.1 双树已归零（E7 家族本轮清零）
- **全树（排除 target/.git/.workbuddy/release）963 个文件，本机 ↔ .131 字节级完全一致（0 差异）**，全量门禁复跑 **460/0/61**。
- 差异分类结论：304 个不一致中 **303 个纯行尾差异、真实内容差异 0**（首次分类误判的 19 个"内容差异"是本机自身 CRLF 造成的假象，双侧去 CR 后即归零）。
- 顺带修正两处：
  - **`Cargo.lock` 本机钉在 `0.1.2`**（本机从不跑 cargo，锁文件长期与发版脱节）→ 已从 .131 取回同步到 0.2.21 并提交（`ebd21d8`）。
  - **`.131` 独有的 `crates/agent-core/src/cache_telemetry.rs`**（R2-A v0.2.6 遗留物，无 `mod` 声明、不参与编译）→ 已取回本机并提交（`74a44a1`）以防再次丢失；**接线或删除由顶层裁决**。
- 残余：258 个文件仍含 CR，但**两树对称、非测试输入**，下次 git checkout 由 `.gitattributes` 归一化。

### 6.2 发版备份
- `~/Desktop/hearth-backups/hearth-v0.2.21-20260901-src.tar.gz`（4.27 MB，sha256 `fcdee9cb07ff31ce34043f46d4950c44ac5dce46929b01994b1f526fe84900d9`），由 `git archive v0.2.21` 生成。
- 二进制旧件：`/usr/local/bin/hearth.v0.2.20.bak`。

### 6.3 环境就绪核验（Node 14 可直接开工）
| 项 | 状态 |
|---|---|
| `~/fa/simuser/`（driver/analyzer/run_rc52_matrix/metrics.yaml） | ✅ 齐备 |
| 黄金集 `~/fa/campaign/golden/replay41.txt` | ✅ sha256 `14764e80…` 吻合（4,987 B）；`手工测试v0.2.18.txt` 189,333 B |
| 部署二进制 | ✅ `hearth 0.2.21` |
| sessions 目录可写、264 个 36 位 uuid 会话 | ✅ |
| `~/fa/simuser/personas`、`scenarios` | ⚠️ **空目录**（M-1 挂账：块 C persona 采样需按规划书 v1.1 §2.1 自构输入） |
| LLM provider 出口 | 🔴 **见 §5.5** |

## 7. 挂账（非本窗处置，供顶层排期）

| 项 | 说明 |
|---|---|
| `.131` 树 `tools/` 目录曾整体缺失（E7 变体） | 已补建；同步管线根因挂 v0.2.22/23 |
| `cargo test --workspace --no-fail-fast` 曾挂起 22 分钟 | agent_core 测试二进制 CPU 近零、子进程阻塞 socket recv；kill 后 fail-fast 口径未复现。**未根因定位**，发版 gate 用 fail-fast；若测试窗复现请单独立项 |
| 测试二进制落后源码（E7 家族） | 每次开工先 `hearth --version` + md5 双查 |
