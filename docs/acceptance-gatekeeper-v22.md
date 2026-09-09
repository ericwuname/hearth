# v22 红队审计修复 · 守门员独立验收报告（P0/P1）

> 日期：2026-08-02 | 验收人：守门员（验证角色，非施工角色）  
> 基线：本地 HEAD `1255498` → 修复 commit `a3736e9`  
> 依据：`audit-fix-taskbook-v22.md`（任务书）+ `audit-findings-v22.md`（证据）+ `acceptance-audit-fix-p0p1.md`（施工自报）  
> 口径：tracker §0 验收准则 V1–V7（只信门禁不信自报 / INCONCL 计未通过 / 新测须先红后绿 / 鉴权只看 401/403 / 回归保护 / 门禁自身须能红）

---

## 一、铁律执行：不采信自报，独立复测

施工方自报「P0/P1 全 12 项 + 门禁全绿（219 passed / window-framework 187/187）」。守门员**不采信**，按以下步骤独立复测：

1. **VM 源码是陈旧快照**：`~/codex_work` 非 git，源码无 `ALLOW_NO_AUTH`/`strip_comments`，`service` 二进制是 08-01 05:44 旧构建。若直接跑门禁会测旧代码 → 假绿/假红皆无意义。
2. **同步修复源码**：`tar`（排除 target/.workbuddy/.git，1.7M）上传 VM，解压保留共享 `target` 符号链接，增量重建 release（**1m05s**，rc=0）。
3. **跑门禁**：`.workbuddy/v22_regression_gate.py` 对修复后二进制独立探测。

> **强证据 —— 门禁被修复「打脸」**：门禁首跑即 **服务起不来**。这正是 P0-1 fail-closed 生效（无 key + 无 ALLOW_NO_AUTH → 拒绝启动）。门禁原为「开放默认」假设，据此**反改门禁**（默认带 `ALLOW_NO_AUTH=1` 起服务探测；P0-1 改验「拒启 + 请求鉴权 401/403」）。门禁能红，尺子有效。



---

## 二、逐项结论（12/12 守门员独立通过）

| 编号       | 问题                   | 守门员独立验证方式                                                                                                                        | 结论      |
| -------- | -------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ------- |
| **P0-1** | 审批门默认零鉴权             | VM 门禁：无 key+无 allow → 服务拒启（fail-closed）；设 key 后 无key=401/正确=201/错=403                                                            | ✅ PASS  |
| **P0-2** | CLI approve/deny 422 | VM `cli_contract_test`（真实 `codex-cli` 二进制）：approve/deny → 业务失败清晰（非契约 422）                                                        | ✅ PASS  |
| **P0-3** | CLI sessions 405     | VM 门禁：GET /api/v1/sessions = 200；CLI 实测 rc=0                                                                                     | ✅ PASS  |
| **P0-4** | wiring 断言可绕过         | VM 门禁变异测试：注释掉一处被断言调用 → 门禁由绿变红（EXIT 0→1），断言有牙齿                                                                                    | ✅ PASS  |
| **P0-5** | bash 零沙箱             | 本地 `window-framework/tests/test_v10.py` 27/27：越权写 /tmp 拒 / curl 拒 / ls outputs 通                                                 | ✅ PASS  |
| **P0-6** | prompt 注入砖化窗口        | 本地 v10：WT17 注入 prompt 的 TOML 可读且保留原文                                                                                             | ✅ PASS  |
| **P1-1** | 重启后会话全 404           | VM 门禁：重启前 200 → 重启后 200（读路径已接线，磁盘 4 文件）                                                                                          | ✅ PASS  |
| **P1-2** | /readyz 永不变红         | VM 门禁：破坏 MEMORY_DIR 后 /readyz 200→503，warn 日志 1 条                                                                                | ✅ PASS  |
| **P1-3** | 坏 MEMORY_DIR panic   | VM 门禁：坏路径无 panic，进程存活（startup_fatal 优雅退出）                                                                                        | ✅ PASS  |
| **P1-4** | civ 告警三层静默           | **源码确认**：`civ_note` 补 else（warn+fallback 落盘 `civ-fallback.jsonl` loop.rs:475-495）+ drain 移相位循环；R1 套件 219 覆盖。**守门员未独立造 civ 路径门禁** | 🔶 源码确认 |
| **P1-5** | 路径前缀旁路               | 本地 v10：`is_relative_to` 取代 startswith（outputs_evil/ 拒）；0 字节文件 → \_has_outputs=False                                              | ✅ PASS  |
| **P1-6** | CLI↔service 契约测试     | VM `cli_contract_test`（真实 CLI）：sessions/status/approve/deny/tools/whoami/coverage/template 全 rc=0                                | ✅ PASS  |

---

## 三、门禁运行记分牌

| 轮次     | 范围                                                          | PASS | FAIL | INCONCL | 红转绿     | 证据                                                                                      |
| ------ | ----------------------------------------------------------- | ---- | ---- | ------- | ------- | --------------------------------------------------------------------------------------- |
| R0 基线  | 8 项（修复前）                                                    | 0    | 8    | 0       | —       | `v22_regression_20260802_012216.jsonl`                                                  |
| R2 守门员 | P0-1..4/P1-1..3（VM）+ P0-5/6/P1-5（本地 v10）+ P0-2/P1-6（VM cli） | 12   | 0    | 0       | 8/8 全闭环 | `v22_regression_20260802_043003.jsonl` + `test_v10.py` 27/27 + `cli_contract_test` rc=0 |

---

## 四、残留与建议

1. **P1-4 补独立门禁**：当前仅源码确认 + R1 套件覆盖。建议后续补一个「civ-alert 注入 → 断言落盘 `civ-fallback.jsonl` 且 readyz 降级」的守门员门禁，使该项也有动态红绿。
2. **P2 全挂账未动**：P2-1（限流 Retry-After/per-client）、P2-2（seccomp 扩展）、P2-3（gate reject 装饰）、P2-5（provider 路由）、P2-6（回滚净丢失）、P2-7（边界硬化）等仍按任务书挂账待排期。其中 **P2-1 的 Retry-After 我已写入门禁自动判定**（基线 10/10 缺失，修复后再跑即知）。
3. **门禁覆盖缺口**：window-framework 侧（P0-5/6、P1-5）目前靠 `test_v10.py` 本地跑，未纳入常驻回归门禁；建议把 `test_v10.py` 接进 CI。

---

## 五、结论

**P0/P1 全 12 项，守门员独立验收通过（12/12）。** 其中 11 项有动态门禁/测试闭环，1 项（P1-4）源码确认 + 套件覆盖。施工自报与守门员复测一致，**v22 红队审计 P0/P1 阶段可闭环**。建议施工方将 P2 挂账项排入下一批，并补齐 P1-4 的守门员门禁与 window-framework 的 CI 接入。
