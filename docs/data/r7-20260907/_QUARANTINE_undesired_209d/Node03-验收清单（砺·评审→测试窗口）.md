# Node 03 driver/analyzer 验收清单（砺·评审 → 测试窗口）

> **日期**：2026-09-01　**出具**：砺·评审（🪨）　**执行**：测试窗口
> **基线**：`HEAD = 24ff85a`（Node 07-09 完成，tag v0.2.20）
> **核验对象**：`tools/simuser/driver.py`、`tools/simuser/analyzer.py`、`tools/simuser/run_rc52_matrix.py`
> **性质**：评审只出判据、不改代码。修复属施工，由执行窗口实施。

---

## 0. 分工裁决：测试窗口跑，评审不跑

| 理由 | 依据 |
|---|---|
| ① 总包明文 | 总包 §11："**测试窗口负责 run/capture/detect/archive，执行窗口不得自任 campaign judge**" |
| ② 窗口纪律 | 评审 = 只读守门，走 `.133`，不碰 `.131` 在制品；driver 需真 PTY + 真 LLM + hearth binary，只能跑 `.131` |
| ③ 利益冲突 | 这批代码是执行窗口刚交付的，自跑自判 = 自证。已查出 1 个 🔴 致命缺陷，必须独立第三方复现 |

**评审交付**：静态核验 + 本验收清单（带可失败判据）+ 结果判读。
**测试窗口交付**：run / capture / detect / archive。

---

## 1. 🔴 前置阻断项：C 条件（resume）数据当前 100% 无效

### 1.1 事实（已实测验证，非推断）

`run_rc52_matrix.py:99` 从终端输出提取 session uuid：

```python
m = re.search(r"reports/([0-9a-f-]{36})/", txt)   # 要求 36 位完整 uuid + "reports/" 前缀
```

但 `crates/codex-cli/src/run_local.rs:280` 终端实际打印的是 **8 位短 id**：

```rust
println!("{}  (session {})", isolation_badge(), &session_id[..8]);
```

`reports/<sid>/` 只存在于**磁盘写入路径与注释**（`run_local.rs:611` 注释、`observer/src/lib.rs:62`），**从不打印到终端**。

### 1.2 本机验证结果（评审亲自跑，可复现）

```
输入（真实终端输出格式）: "  ○ local  (session 3f9a1c22)"
[run_c 正则]  r"reports/([0-9a-f-]{36})/"      -> None    ← 恒失效
[driver 正则] r"session ([0-9a-f-]{36}|[0-9a-f]{8})" -> "3f9a1c22"  ← 有效
```

### 1.3 后果链

```
正则恒 None → uuid = "" （无 else 告警，99-101 行缺告警分支）
          → hearth resume "" "继续你的提议吧" 被无条件执行
          → C 条件 3 个样本全部建立在空 session id 上
          → RC52 结论中的 "resume 67%" 无效
```

**讽刺点**：`driver.py:120` 自己的正则**是有效的**，`run_c` 没复用 `d0.run.session_ids`，却另写了一个错的。

### 1.4 最小修复（执行窗口实施，评审不改代码）

```python
# run_rc52_matrix.py run_c()，替换 96-102 行
uuid = d0.run.session_ids[-1] if d0.run.session_ids else ""
if not uuid:
    raise RuntimeError("C-condition: session id 提取失败，本样本作废（不得静默 resume 空 id）")
```

**修复前，C 条件不必跑——跑了也是废数据。**

---

## 2. 🟡 验收判据（测试窗口逐条打勾）

### 2.1 阻断项修复验证

| # | 判据 | 红（未通过） | 绿（通过） |
|---|---|---|---|
| **G-0** | C 条件 session id 提取 | `results.json` 里 C1/C2/C3 的 `session_ids` 含空串或长度≠8 | 三个样本 `session_ids` 均为 8 位非空 hex，且 resume 报告"会话已恢复" |
| **G-0b** | 失败即作废 | uuid 为空时仍产出样本 | uuid 为空时抛错、该样本标记作废 |

### 2.2 Node 03 矩阵复现（n = A5 / B5 / C3 = 13 跑）

| # | 判据 | 说明 |
|---|---|---|
| **G-1** | 13 跑全部完成，`results.json` 有 13 条且无 `error` 字段 | 单跑失败会被 `except` 吞成 error 记录，须逐条确认 |
| **G-2** | 每跑有 `.log` + `.events.json` 双件落盘 | `_dump()` 产出 |
| **G-3** | **A 条件** fresh 失败率与执行窗口自测的 40% 同向（允许 ±1 个样本波动） | 不同向 → 执行窗口数据不可复现，须报差异 |
| **G-4** | **B 条件** contaminated 失败率 100%（5/5） | 若 <5/5，污染放大器结论需重估 |
| **G-5** | **C 条件** resume 失败率 —— **修复后重测值，不得与旧 67% 混表** | 旧值无效，必须隔离标注 |
| **G-6** | 三条件的 `continue_terminal` 分布完整（completed/failed/timeout 三类均有计数，无全 timeout） | 全 timeout → 疑似 driver 或环境问题，非 Core 缺陷 |

### 2.3 仪器接线验证（Node 03 明确要求，当前存疑）

| # | 判据 | 当前状态 |
|---|---|---|
| **G-7** | **planner prompt dump** 落盘且可被定位 | `HEARTH_DEBUG_PLANNER_INPUT=1` 已在 `run_rc52_matrix.py:39` 开启 ✅，但 analyzer 侧 `planner` 仅 1 次提及 → **dump 是否真落盘、落哪，需实测确认** |
| **G-8** | **TaskGraph fingerprint** 可提取 | `analyzer.py` 中 `fingerprint` = **0 命中** → 当前无采集能力，须补或明确豁免 |
| **G-9** | **terminal** 字段进分析结果 | `analyzer.py` 中 `terminal` = **0 命中**（driver 有 terminal 事件，未聚合进分析） |

### 2.4 污染前缀可复现性

| # | 判据 | 说明 |
|---|---|---|
| **G-10** | `PREFIX`（run_rc52_matrix.py:17-31，硬编码 13 行）能对应到 run-001..012 的真实原文 | 当前硬编码，无法证明"逐字重放"。**建议改为从 `release/手工测试v0.2.18.txt` 或既有 log 读取** |

---

## 3. 执行前置检查（跑之前必做，否则数据作废）

| # | 检查项 | 命令 / 方法 | 依据 |
|---|---|---|---|
| **P-1** | **binary 与 source 同版本**（E7 家族已三次复发） | VM 上 `hearth --version` 比对 `Cargo.toml` 的 v0.2.20；不同则重建 | 记忆：binary 落后 source 是常态 |
| **P-2** | 跑测试模板 | `unset HEARTH_URL` + `export HEARTH_CGROUP_BASE=/sys/fs/cgroup/hearth` + `source ~/.cargo/env` → `cargo test --workspace` | 记忆：跑测试模板 |
| **P-3** | 污染源隔离 | 跑矩阵前确认 `/home/wutao/fa/p4` 已清空旧结果 | 防止新旧 results.json 混表（违反 S6 跨 tag 数据不得混表） |
| **P-4** | cgroup 降级开关 | `run_rc52_matrix.py:38` 已设 `HEARTH_ALLOW_NO_CGROUP=1` —— **确认这是有意为之**，它绕过了 cgroup fail-closed | 若无意绕过，S5 相关结论受影响 |

---

## 4. 已知非阻断瑕疵（登记，不阻断验收）

| # | 位置 | 问题 |
|---|---|---|
| D-1 | `run_rc52_matrix.py:106` | `run_c` 的 `task_terminal` 硬编码 `"chat"`，非真实终态 |
| D-2 | `driver.py:117-118` | 死代码 `for m in ("session ", "(session "): pass` |
| D-3 | `driver.py:119` | `import re` 在热路径函数 `_pump` 内，宜移至模块顶部 |
| D-4 | `driver.py:75` | `preexec_fn=os.setsid` 宜改 `start_new_session=True`（preexec_fn 在多线程下不安全） |
| D-5 | `driver.py:28` | `TURN_TERMINAL_MARKERS` 只认 "Task completed"/"Task failed"，**give_up / deadline_exceeded / cancelled 会被误记为 timeout** → 污染 DRIVER-INDUCED 归因 |
| D-6 | 检测器数量 | 实测 **13** 个（1-10 + anaphora_resolution_fail + escalation_run + repetition_amplifier）。总包 §10 要求 Node 10 补到 **15**，当前未到，属 Node 10 范围不阻断 Node 03 |

---

## 5. 结果判读规则（评审执行，测试窗口只需交数据）

- **G-0 / G-0b 任一红** → 整个 C 条件作废，RC52 结论不得引用 resume 数据。
- **G-3 / G-4 与执行窗口自测值不同向** → 执行窗口自跑自判数据不可复现，**RC52 从 CAUSE LIKELY 暂缓升格 CONFIRMED**。
- **G-6 全 timeout** → 优先怀疑环境/driver（第七层 DRIVER-INDUCED），不得直接归因 Core。
- 全部判据绿 → 评审出具验收通过，RC52 可维持 CAUSE LIKELY 并进入 Node 13 修复批。

**⚠️ 提请顶层注意**：Node 03 矩阵是执行窗口自跑自判的（`4fdbda3`）。总包 §11 禁令字面针对 Node 11，但"自证"风险同构。**在测试窗口独立复现前，RC52 不宜从 CAUSE LIKELY 升格为 CONFIRMED。**
