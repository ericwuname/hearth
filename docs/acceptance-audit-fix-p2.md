# v22 红队修复 · P2 阶段验收报告（守门员建议闭环）

> 日期：2026-08-02 | 施工方自报（守门员 R2 验收 12/12 通过后，按其建议执行下一批）
> 基线：commit `a3736e9`（P0/P1）→ 本轮 `e0f0031`
> 依据：`acceptance-gatekeeper-v22.md` §四 建议 1-3 + `audit-fix-taskbook-v22.md` P2 区块
> 门禁：VM `gate_p2e.log`（FMT 0 / CLIPPY 0 / **221 passed**）+ 本地 `gate_window.py` **208/208**

---

## 一、守门员三条建议的响应

| 守门员建议 | 响应 |
|---|---|
| **建议 1：P1-4 补独立门禁**（当时仅源码确认） | ✅ 补 2 个动态测试：① `test_p14_civ_fallback_written_when_no_writer`（agent-core：不接线 writer → `civ_note` 必须落盘 `civ-fallback.jsonl`，断言含 category/content/tags）② `test_p14_readyz_civ_degraded_threshold`（service：`civ_store_degraded()` 判定函数，0/5→健康、6→降级 503）。**P1-4 从 🔶 升 ✅** |
| **建议 2：P2 挂账项排入下一批** | ✅ 本轮闭环 **5/7**：P2-1（限流 Retry-After）/ P2-3（gate reject）/ P2-5（provider 路由）/ P2-6（快照回滚）/ P2-7（边界硬化 6 小项）。**P2-2（seccomp 白名单）单独挂账**（大工程+安全风险，不混入本轮） |
| **建议 3：window-framework CI 接入** | ✅ `gate_window.py` 统一门禁（9 套件，任一失败 exit 1，**缺失套件计 FAIL**——防 CT8 类"文档 187 但命令复现不出"失真）+ README 更新 |

---

## 二、逐项结论（7 项全闭环）

| 项 | 问题 | 修复 | 动态证据 |
|---|---|---|---|
| **P1-4 门禁** | civ 告警无 writer 静默 | fallback 落盘（上轮代码）+ 本轮补动态断言 | agent-core `test_p14_civ_fallback_written_when_no_writer` ok；service `test_p14_readyz_civ_degraded_threshold` ok（221 passed 内） |
| **P2-1** | 429 无 Retry-After / 全局计数单客户端吃满 / 探针不豁免 | ① 429 补 `Retry-After: 5` ② **per-IP 在途计数**（`OnceLock<Mutex<HashMap<IpAddr,usize>>>`，单 IP 上限 50）+ 全局总量放宽到 500（防洪水）③ `/healthz` `/readyz` 豁免 ④ serve 注入 `ConnectInfo<SocketAddr>` | 编译 + clippy 0（门禁项"429 带 Retry-After"待守门员 R4 复测：其门禁已写自动判定 10/10 缺失→修复后应转绿） |
| **P2-3** | `gate --reject` 装饰（引擎只读 done: 不读 blocked:） | run() 读 `blocked:` 标记（reject 后 stage 不推进）+ approve/reject **幂等**（重复操作 rc=0 不重复写）+ **对侧标记清理**（approve 清 blocked / reject 清 done，status 文件保持干净） | v10 回归 8 项全绿（reject 写 blocked / 引擎不推进 / approve 幂等 1 行） |
| **P2-5** | compress/analyze 三处硬编码 DEEPSEEK_API_KEY 绕过 PROVIDERS 路由 | `_llm_summary`/`_fc_analyze`/`_yaml_analyze_fallback` 全走 `Agent.PROVIDERS`（base_url/env_key/model）；`CompressionEngine` 补 provider 路由字段（曾缺失导致 AttributeError）；**未知 provider 显式报错**（原静默回退 deepseek）；analyze 从 window.toml 读 provider | v10 回归：未知 provider `anthropic` 抛 ValueError（原"回退 deepseek"断言已按任务书反转） |
| **P2-6** | 快照只含 2 文件（无 outputs）/ 回滚先 move 再判断（目标无效丢当前）/ --to 路径遍历 | ① 快照纳入 `outputs/`（copytree）② 回滚顺序改「先校验目标（缺 conversation.jsonl 拒）→ 备份当前 → 替换」③ `--to` 白名单（拒绝 `/ \ .. ~`）④ outputs 一并还原（当前 outputs 移 rolled-back 防混合） | v10 回归 4 项：`--to ../../x` 拒 / 快照含 outputs / 坏快照回滚 rc=1 / 当前对话完好 |
| **P2-7 WT7** | analyze 缺 id 窗口 → `w["id"]` KeyError | `validate_deploy_yaml` 前置 `"id" in w` 检查（友好报错） | v10：缺 id window 被拦截（非 KeyError） |
| **P2-7 WT11** | `auto:../../x` gate 路径遍历 | `_gate_path_ok()`（拒绝对路径/`..`/`~`/超根）+ `_run_gate` 与 deploy 占位脚本两处接线 | v10：`../../etc/passwd` 拒 / 绝对路径拒 / 合法相对放行 |
| **P2-7 WT19** | 重复 role 不查 | **stage 级** role 查重（FrameworkCheck 新增断言 5：同 stage 窗口 role 重复 → FAIL）。不做全局查重（多 dev 同 role 是合法场景，v10 证明） | v10：`stage-role-unique` 断言存在；framework check 4→5 断言，测试同步更新 |
| **P2-7 WT4** | budget 极端值（max_steps=99999 → 海量快照） | ① CLI run 未显式 `--max-turns` 时尊重 `budget.max_steps`（default 12→None）② **replay 模式也查预算**（超限即停 blocked rc=2，不产生每 50 轮快照 ×2000） | v10：max_turns=99999 + tokens 超限 → rc=2 且快照 <10 |
| **P2-7 WT15** | 导出截断 200 字符 / 导入丢 summary role + 强改时间戳 | ① md 导出不截断 ② import 接受 `summary` role + 保留 `t/ts/timestamp` + 保留 meta ③ 修 import 对单 dict 消息的处理（原遍历 dict key） | v10：往返断言（summary role 保留 + 原时间戳保留） |

---

## 三、门禁证据

```
VM（codex-rust）gate_p2e.log：
  FMT_RC=0 / CLIPPY_RC=0 / TEST_RC=0
  221 passed（219 + test_p14_civ_fallback_written_when_no_writer
             + test_p14_readyz_civ_degraded_threshold）

本地（window-framework）gate_window.py：
  208 passed / 0 failed（framework 21 + v02 21 + v03 19 + v04 17 + v05 28
                        + v06 14 + v07 19 + v09 21 + v10 48）
  v10 含本轮新增回归 24 项（P2-3×8 + WT7/11/19/4/15 + P2-6×4 + P0-5/P1-5 既有）
```

---

## 四、状态与挂账

- **P0/P1 全 12 项**：守门员独立验收通过（R2 结论）
- **P1-4 独立门禁**：✅ 已补（R3 动态闭环）
- **P2 本轮 5 项 + P2-7 批量**：✅ 全闭环
- **P2-2 seccomp 白名单**：🔴 **单独挂账**——sandbox 白名单化（默认拒绝）涉及
  生产沙箱行为反转 + landlock 硬失败语义，属安全敏感大工程，须独立排期并
  过压力测试，不混入本轮。
- **P2-1 per-IP 维度**：代码已落地（per-IP 上限 + 全局放宽），VM 单 IP 无法
  动态双客户端验证——按任务书标注为**人工审查项**（读 `routes.rs` 限流中间件
  确认 per-IP 分支存在即可）。
- **P2-4 / P2-8**：上轮已闭环（WT6 / CT3+CT6+CT8）。

**建议守门员 R4 复核点**：① P2-1 门禁自动判定（429 带 Retry-After，基线 10/10
缺失 → 修复后应转绿）② P1-4 动态测试是否满足"civ-alert 注入 → 落盘 +
readyz 降级" ③ P2-2 排期。
