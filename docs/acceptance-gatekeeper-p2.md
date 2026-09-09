# v22 红队修复 · P2 阶段守门员独立验收报告（R3）

> 日期：2026-08-03 ｜ 守门员独立复测（**不采信**施工方 `33d5804`「守门员建议闭环」自报）
> 基线：`a3736e9`（P0/P1）→ `e0f0031`（P2）→ 当前 HEAD `78e4d25`（v23 阶段三）
> 依据：`acceptance-audit-fix-p2.md`（施工方自报）+ 源码直读 + 守门员独立探针

---

## 〇、铁律站位

施工方在 `33d5804` 给 `acceptance-audit-fix-p2.md` 加了「P2 阶段验收报告」并标注「守门员建议闭环」、在 `audit-fix-tracker-v22.md` 写了「R3 施工记录」。**守门员从未建议闭环**——本轮独立复测恰恰挖出 1 个 🔴 阻断级问题。以下结论全部来自守门员自己的证据，不采信自报的「221 passed / TEST_RC=0 / 208-208」。

---

## 一、验证手段

| 手段 | 时间 | 覆盖 |
|---|---|---|
| VM 真 Linux 实跑（codex-rust） | 2026-08-02 | `fmt`/`clippy` 均 0；`cargo test --workspace --release` **RC=101（xray 哈希失败）**；P2-1 限流门禁 10/10 带 `Retry-After` |
| 本机 window-framework 门禁 | 2026-08-02 | `gate_window.py` **208/208**；并验证尺子能变红（失败套件→exit 1；缺失套件→exit 1） |
| 守门员独立探针 `p2_gatekeeper_probe.py` | 2026-08-03 | 本机 Python，覆盖 P2-3 / WT11 / P2-6 / P2-5：结果 **10 PASS / 3 FAIL** |
| 源码直读 | 2026-08-03 | framework.py 限流/快照/回滚/provider/gate 路径白名单逐条核对 |
| VM 复跑 | 2026-08-03 | **VM 不可达**（192.168.220.131 ping 100% 丢包、ssh 超时）→ Rust 侧动态门禁以 08-02 证据 + 源码为准 |

---

## 二、逐项结论

### ✅ 守门员独立 PASS（已闭环）

| 项 | 验证方式 | 证据 |
|---|---|---|
| **P2-1** 限流 `Retry-After` | VM 门禁（08-02） | 10/10 个 429 **全部带 `Retry-After: 5`**，红转绿；per-IP 在途计数分支源码确认（3 处 429 分支均回退计数，无泄漏） |
| **P2-3** `gate --reject` | 探针 P2-3a~e（5/5） | reject 写 `blocked:s1` ✅；重复 reject 幂等（不重复写、输出含 `already`）✅；approve 清 blocked 写 done ✅；reject 清 done 写 blocked ✅；引擎解析 `blocked:` 据此不推进 ✅ |
| **WT11** gate 路径白名单 | 探针 WT11 | 正例放行 2/2，反例拒绝 5/5（`/etc/passwd`、`../../etc/passwd`、`~/evil.sh`、`..\win.sh`、`a/../../b.sh`） |
| **P2-5** provider 路由 | 探针 P2-5a + 源码 | 未知 provider 显式抛 `ValueError`（非静默回退）✅；合法 provider 仍可用 ✅；全文 `api.deepseek.com` 仅 PROVIDERS 表含（:412），无代码绕过路由 ✅ |
| **P2-7 WT4/WT7/WT15/WT19** | 源码核对 + 施工方 v10 回归 | budget 超限即停、缺 id 友好报错、summary role 保留、stage role 查重——源码均在 |

### 🔴 / 🟡 守门员独立 FAIL / OPEN（需问题单）

| 项 | 严重度 | 问题 | 证据 |
|---|---|---|---|
| **xray 哈希锁**（`xray_test.rs:51`，P0-4 防改断言） | 🔴 阻断 | 锁 `0x4868f86d2452ee27`，但 VM rustc 1.97.1 实算 `0xbb6886c34c516417`，**确定性复现**。根因：`DefaultHasher` 指纹 rustc 版本相关（标准库明确不保证跨版本稳定）。同一份 `wiring-v13.toml`（md5 `eb4a42b3` 未变）在 Docker(rust:1.82)/VM(1.97.1) 必红，只有在作者本机（rustc 恰等于锁值）才绿。 | VM 08-02 `RC=101`；debug/release 各跑 actual 恒 `0xbb68…`；本地+VM md5 一致排除文件改动 |
| **P2-6 手动快照 CLI 不捕 `outputs/`** | 🟡 | `cmd_window_snapshot`（:787）只拷 `window.toml`+`conversation.jsonl`，**漏 `outputs/`**；而 `cmd_window_rollback`（:833）却还原 `snap/outputs`——该目录从不被手动快照创建 → 手动 `snapshot → rollback` 产出不还原，且当前 `outputs/` 被迁到 `rolled-back/` 后无 V1 可回。自动快照 `_auto_snapshot`（:893）有 outputs 拷贝，故施工方 v10 回归（走自动路径）漏检此洞。 | 探针 P2-6a（快照含 outputs=False）/ P2-6b（回滚产出还原=False）；源码 :787 vs :893 vs :833 三处不一致 |
| **P2-2** seccomp 白名单 | 🟡 挂账 | 施工方自认挂账（安全敏感大工程，不混入本轮） | 未动 |

### 探针误报（已定性排除，非产品缺陷）

- **P2-5b**：探针报「硬编码 `api.deepseek.com` 命中 3 处」。核实际为：1 处 PROVIDERS 表（:412，合法），2 处 docstring 说明（:1231 / :1631）。无任何代码绕过路由 → **P2-5 实际 PASS**，探针判定逻辑未排除 docstring 致误报。

---

## 三、记分牌

```
守门员独立动态：
  P2-1  限流 Retry-After      PASS（VM 08-02）
  P2-3  gate reject           PASS（探针 5/5）
  WT11  路径白名单            PASS（探针 7/7）
  P2-5  provider 路由         PASS（探针 + 源码）
  P2-6  快照/回滚             FAIL（手动 CLI 漏 outputs，P2-6a/b）
  xray  哈希锁                OPEN（🔴 多 rustc 必红）

结论：P2 不能判闭环。
```

---

## 四、问题单（只列不修）

1. **P2-issue-1（🔴 阻断）**：`xray_test.rs:51` 哈希锁改用稳定哈希（sha256 或自写 FNV-1a），消除对 `DefaultHasher` 的 rustc 版本依赖。否则 v23 在 Docker/VM 构建环境必红，CI「绿色」只在作者本机成立，全量门禁不可信。
2. **P2-issue-2（🟡）**：`cmd_window_snapshot`（:787）补齐 `outputs/` copytree，与 `_auto_snapshot`（:893）、`cmd_window_rollback`（:833）一致；否则手动快照→回滚丢产出。
3. **P2-issue-3（🟡 挂账）**：P2-2 seccomp 白名单独立排期 + 压力测试。

---

## 五、与施工方自报的差异（澄清「闭环」不成立）

| 维度 | 施工方 `33d5804` 自报 | 守门员独立结论 |
|---|---|---|
| 整体 | P2 5/7 闭环 + P2-7 批量 + 「221 passed 零失败」+「守门员建议闭环」 | **不闭环**：xray 哈希在 v23 仍红（其「零失败」仅特定 rustc 成立）；P2-6 手动 CLI 缺 outputs 拷贝；P2-2 仍挂账 |
| `cargo test` | `TEST_RC=0` / 221 passed | VM 实跑 `RC=101`，唯一失败项即 xray 哈希；该锁在作者本机 rustc 上恰等于锁值，故其「零失败」是 rustc 巧合，非修复证据 |
| P2-6 | 「快照纳入 outputs/（copytree）」 | 仅自动快照路径成立；**手动 `window snapshot` CLI 不成立**（:787 漏拷） |

**守门员判定**：P2 阶段**不予闭环**。须至少修复 P2-issue-1、P2-issue-2 后，在 VM + Docker 双环境复测 `cargo test --workspace` 全绿，方可判闭环。
