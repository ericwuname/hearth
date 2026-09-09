# GitHub 首次同步·安全清理清单 v1.0（执行窗→用户）

- **出品**：执行窗　**日期**：2026-09-09 19:40
- **依据**：顶层裁决 e5c9d1b——GitHub force push 须"安全清理审查先行、用户过目点头才动 public"
- **扫描口径**：HEAD 全树（2041 个跟踪文件）× 敏感 pattern 全扫 + 逐文件人工甄别

---

## 一、扫描结论

| 类别 | 命中 | 甄别后真敏感 |
|---|---|---|
| **API key**（Agnes cpk- / OpenAI sk- / 智谱 / Gemini AQ. / 豆包 ark-） | 18 文件 | **18 全真**（含 r9pkgb.sh:18 的 38 位完整 Agnes key） |
| **VM 凭据/内网**（密码 123456 / 192.168.220.x / ssh 会话记录） | 19 文件 | **17 真**（agent-types/lib.rs 与 swagger bundle 两处为数字子串巧合，已排除） |
| .workbuddy（记忆/密钥主档） | **未跟踪且已 ignore** ✓ | 零风险 |
| config.toml / .env | 未跟踪（仅 .env.example）✓ | 零风险 |

## 二、清单明细（37 文件）

**KEY 类 18**：docs/acceptance-y-direction.md、audit-full-v24-post-plan.md、audit-full-v24-post.md、audit-version-manual-v22.md、claude-findings-verification-矾.md、consolidated-remediation-ledger.md、data/memory-context-20260830/measure_tokens.py、data/r7-20260907/_QUARANTINE_undesired_209d/p12.yaml、**data/r9-pkgB-20260908/r9pkgb.sh（完整 key）**、data/手工测试v0.2.23-2026-0.9-0.8.txt、dev-scan-and-direction-y.md、external-advisor-briefing-v14.md、hearth-manual-test-incremental-findings-v1.md、incidents/2026-08-28-tier3/run_telemetry{,2}.py、task-order-y-direction.md、test_gemini .py、vm-env-path-fork-2026-08-29.md

**VM 类 17**：docs/P4-Node14测试报告、audit-fix-taskbook-v22、audit-fix-tracker-v22、audit-full-v24-post-plan、bench-data-paths-2026-08-06、data/r7-20260907/scripts/_r7_sudo.sh、**_r7_t131.sh（含 sshpass 型凭据使用）**、incidents/…/run_telemetry{,2,_r2}.py、p0-usability/R5智能性根治·执行委托书、p0-usability/归档前补件终验与归档放行令、testing-report-v21/v22、testing-runbook-v21/v22、testing-v22-plan

## 三、关键事实（定清理策略）

1. **本机 git 历史自 3b0c9f5（包B 数据）起多 commit 携带真 key**——历史手术（filter-repo 重写 4000+ commits）复杂易漏；
2. **public 旧镜像（7-31 v21）本就与本机不同源**——force push 反正是整仓替换；
3. **key 实际泄露面评估**：public 从未含 key（旧镜像早于 key 产生）——key 未泄露，轮换**可选非必须**；但 push 一旦发生即不可逆，必须 push 前清零。

## 四、处置方案（执行窗建议，等点头）

**方案：orphan fresh-start push**（不做历史手术）：
1. 从当前树建 orphan 分支 → 单一初始 commit；
2. push 前树内清洗：37 文件**移出仓库**（本地 private 归档保留，如 `Desktop/hearth-private-archive/`）+ 脚本类改 env 读取版可留；
3. **验收标准：清洗后全 pattern rescan = 0**（同款扫描脚本复跑）；
4. force push 替换 public 旧镜像。

**备选**：改推 private 仓库（零清洗直接推， sacrificing public 属性）——若你更看重省事，这条路一步到位，public 化留待日后。

## 五、需要你拍板的三项

1. 方案选 **orphan fresh-start（建议）** 还是 **private 仓库**？
2. 37 文件本地归档路径认可？（建议 Desktop/hearth-private-archive/）
3. key 轮换：选不轮换（未泄露，可）或轮换（最稳，Agnes 控制台 1 分钟）？
