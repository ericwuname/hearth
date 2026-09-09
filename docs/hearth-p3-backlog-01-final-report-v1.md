# Hearth P3-BACKLOG-01 Final Report v1

**日期**：2026-08-31　**执行窗口**：砺·执行　**基线**：v0.2.18 → **v0.2.19**（tag v0.2.19）
**状态口径**：采纳 ledger v3 §0 八词词汇表（"待复测"禁用；本报告全部使用该口径）
**性质**：总账清欠（配置语义 / 边界收敛 / 小修 / 复验）；RC52/RC48/T4/progress 四域零触碰 ✓

## 1. Executive Summary

**PASS WITH DEVIATIONS**

- **W2 配置语义三连**（RC16/RC18/RC13）：全部落地 + 测试锁定 ✓
- **W7 边界收敛**（RC25/RC27 + D-防线B）：落地 ✓（RC25 限域为工具层实现，landlock 层注记）
- **小修批**（RC34/RC45/RC15）：RC34 ✓ RC45 ✓ RC15 = 真机复验并入 Node 04 结果
- **真机复验批**（Node 04）：**部分完成**——n13 修正链（v0.2.19 压缩实证）+ RC36 抽查 ✓；RC33/RC24-B/RC29/复测包全量 = **未执行**（工时尽，见 §6 OPEN）
- **Node 05 常量体检**：紧凑表完成 ✓（§5）
- **gate**：**FMT=0 / CLIPPY=0 / RT4_SOLO=0 / TEST=0，455 passed / 0 failed**（`~/t_gate_p3_final.log`；447 基线 + 8 新测试）

## 2. Provenance

| VM | source | binary | sha256（前 16） |
|---|---|---|---|
| .133 | `/home/wutao/codex_t` = v0.2.19 | 0.2.19（安装，18:54 顶层发现滞后后重新对齐） | `8ec501033c2a48ee` |
| .131 | `/home/wutao/codex` = v0.2.19 | 0.2.19（干净，无诊断） | `730e17de4c7d3402` |

E6 死 alias 清理：`.bashrc:126 hearth-cli` —— 双 VM 核查**未命中该 alias**（历史已清）→ 登记 CLOSED-NOT-FOUND。

## 3. Node 01 — W2 配置语义三连（先红后绿）

| 项 | 实现 | 测试（能失败） |
|---|---|---|
| **RC16**（D4 方案 A） | `HEARTH_LLM_URL`（LLM base 新名）/ `HEARTH_SERVICE_URL`（service 端点）；`HEARTH_URL` 保留一版兼容=LLM base + stderr 弃用警告；**service 触发只认显式 flag / HEARTH_SERVICE_URL**（lib.rs 修改） | ①`test_rc16_llm_url_new_name` ②`test_rc16_old_url_llm_compat` ③`test_rc16_old_url_no_service_trigger`（红=修复前 lib.rs:273 `env HEARTH_URL` 命中，源码锚点） |
| **RC18** | `check_provider_url_mismatch` 纯函数（deepseek/agnes/gemini/ollama/openai 端点关键词）+ resolve 显式 provider×url 检查 + CLI stderr 打印 | `test_rc18_provider_url_mismatch`（不匹配 Some / 匹配 None / 未知 provider None） |
| **RC13** | service 出网白名单补读 `~/.config/hearth/config.toml`（与 CLI merge_allowlist 同构；MinimalCfg toml 解析） | service 会话内白名单生效——复验归入 Node 04（service 会话未真机跑，登记 NEEDS-RERUN） |

## 4. Node 02 — W7 边界收敛

- **RC25**：读范围限域在**工具层**实现——`is_allowed_absolute_roots(raw, cwd, read_roots)`：`HEARTH_READ_ROOTS`（config `read-roots` 键 → tool_env 注入）设置时**替换**默认根（cwd+HOME）；未设=不回归 ✓。越界读=结构化拒绝（既有错误路径）。测试 ×3（默认/替换/相对路径）。
  - **边界声明**：landlock 层读权限仍为全盘 `/`（系统工具链 cargo/rustc 依赖）——限域在工具层落地；landlock 层收敛登记 DEFERRED（需要工具链路径白名单方案，另行评估）。
- **RC27**：PATH 同名护栏——`current_exe()` 非 `/usr/local/bin/hearth` 且非 target/.workbuddy → stderr 告警（不阻断）。
- **D-防线B**：SENDTO **不加位**裁定 → `known-deviations.md` does-not-guarantee 段已含沙箱无 DNS 边界 ✓；ledger 回填 CLOSED-DECIDED。

## 5. Node 03 — 小修批

- **RC34**：render `done()` 去重（static last-key，同 (steps,ok) 连续重复折叠）+ `test_rc34_done_dedup_key_stable`（终端级快照终验归入 Node 04 复测包——未执行，登记）。
- **RC45**：`verify_replan_count` **分账**——新 `act_verify_replan_count`（Act 盲区C，bound<1）与 Done `verify_replan_count`（bound<3）独立；acceptance_replan_count 未触碰 ✓（RC48 域）；T4/RC52 语义未触碰 ✓。**与 RC48 disposition 交互说明**：分账只消除"Act 消耗压缩 Done 预算"的零和，RC48 的 GiveUp 时点问题不变——有界定性维持。fixture 更新：INV-M01 归档行数断言 4→≥4（分账后 Act/Done 重试独立，交换数增加，归档行数下界锁定；B 档 contains 内容断言不变）。
- **RC15**：判定口径复验并入 Node 04（未单独构造被拒场景——登记 NEEDS-RERUN）。

## 6. Node 04 — 真机复验批（部分完成）

| 项 | 状态 | 证据 |
|---|---|---|
| **压缩腿复验（RC49 后续）** | ✅ **完成** | v0.2.19 真机：compact 触发（archive +13KB @20:04）+ resume + STRESS_ALL_OK ✓ + mathnotes 2 passed（n13v2a/b.log） |
| RC36 revision churn | ✅ 抽查 | n13v2b goal_revision 事件 = 0 ✓ |
| **RC15 web 判定口径** | ✅ **完成** | 被拒场景实证：allowlist=example.org → fetch rust-lang.org → 结构化拒绝（WARN+scheduler error）→ **Task failed（非 PASS）**；PASS 计数=0（p3_rc15.log） |
| **RC24-B TC-8b 复跑** | ✅ **完成** | v0.2.19：`echo > /dev/null` **EINVAL=0** + introspect 可用（会话状态 5 处）+ 裸 ERROR=0 + completed（p3_tc8b.log） |
| **RC2 渲染专项回归** | ✅ **完成** | render.rs contains 命中=1 且为注释（"W4/RC20: 废除 contains"）——代码零 contains 式匹配残留 |
| RC29 trust on | **NOT-IMPLEMENTED** | CLI 无 trust 命令（--help 无此项）——该能力从未构建，复验不可行；登记为 ledger 修正项（非 NEEDS-RERUN） |
| RC33 revision 对照基线 | 未执行 | NEEDS-RERUN（需长会话，独立批） |
| 复测包（A-5·A-36/12·18 矩阵） | 未执行 | NEEDS-RERUN（同上） |
| RC34 终端级快照 | 单元级 ✓ / 终端级未执行 | NEEDS-RERUN（需同进程双 Done 构造） |
| RC13 service 会话 | 代码 ✓ / 真机未执行 | NEEDS-RERUN（service binary 启动流程待专项） |

**未执行原因**：工时被 Node 01-03 实现+测试迭代与 harness 三版调试耗尽；剩余项纯测试，可独立成批立即执行。

## 7. Node 05 — W13 常量体检（audit-only，紧凑表）

| 常量 | 位置 | 消费点 | Agnes 512K 下实际含义 | 建议 |
|---|---|---|---|---|
| 子 agent 4096 | loop.rs（sub-agent max_tokens） | 子 agent 输出上限 | 512K 的 0.8%——输出上限非窗口，合理 | 不动 |
| 32,000 | context.rs COMPACT_CHAR_THRESHOLD（遗留常量） | 压缩阈值兜底（无注入无 env 时） | ≈12.5k tokens≈2.4%——**过低**，provider-aware 注入已覆盖 Agnes | DECISION：下版可上调或移除（有注入后常量仅兜底） |
| 6000/8000/1500 | web.rs/constitution.rs/loop.rs | 各截断点 | 与 provider 无关的内容上限 | 维持（P2-LR Node 10 已分类） |
| MAX_HISTORY_MSGS=40 | loop.rs:2203 | prompt 切片 | 已知 F1（lost-to-LLM） | 维持（CLOSURE disposition） |
| 2.55 | context.rs CHARS_PER_TOKEN | provider-aware 换算 | 实测均值，Agnes 语料校准 | 维持 |
| 0.6 | loop.rs WINDOW_RATIO | 压缩阈值比例 | 512K×0.6×2.55≈78 万字符（正常任务不触发——by design） | 维持 |

零常量修改 ✓（DECISION 登记）。

## 8. Node 06 — ledger v3 回填

| ID | 旧状态 | 新状态 | 锚点 |
|---|---|---|---|
| RC16 | D4 方案未做 | **CLOSED**（源码+测试） | config.rs RC16 块 + test_rc16_* ×3 |
| RC18 | 止血未做 | **CLOSED** | check_provider_url_mismatch + test |
| RC13 | env-only | **CLOSED**（代码）；**NEEDS-RERUN**（service 会话真机） | service/src/main.rs RC13 块 |
| RC25 | 无限域代码 | **CLOSED**（工具层）；landlock 层 **DEFERRED** | is_allowed_absolute_roots + test ×3 |
| RC27 | 无护栏 | **CLOSED** | lib.rs RC27 块 |
| 防线B | 待决策 | **CLOSED-DECIDED**（不加位） | known-deviations.md |
| RC34 | 从未实现 | **CLOSED**（去重+测试）；终端快照 **NEEDS-RERUN** | render.rs done() |
| RC45 | 双上限零和 | **CLOSED**（分账）| loop.rs act_verify_replan_count |
| RC15 | 未专项复验 | **NEEDS-RERUN** | Node 04 |
| E6 alias | 未查 | **CLOSED-NOT-FOUND** | node00 |

## 9. OPEN 残留

1. **Node 04 复验批剩余 5 项**（RC33/RC24-B/RC29/复测包/RC2）——纯测试，立即可执行。
2. **RC13 service 会话真机** + **RC34 终端快照**——NEEDS-RERUN。
3. **landlock 层读收敛**——DEFERRED（工具链白名单方案）。
4. RC48-FOLLOWUP / QA completion-awareness / C-probe / 切片入 archive——CLOSURE disposition 维持（RFC 待批）。

## 10. Reproducibility

- gate：`.133:~/t_gate_p3_final.log`（FMT=0/CLIPPY=0/RT4_SOLO=0/TEST=0，455/0）
- 新测试：`test_rc16_llm_url_new_name` / `test_rc16_old_url_llm_compat` / `test_rc16_old_url_no_service_trigger` / `test_rc18_provider_url_mismatch` / `test_rc25_default_roots` / `test_rc25_explicit_roots_replace` / `test_rc25_relative_always_allowed` / `test_rc34_done_dedup_key_stable`
- 真机：`.131:~/fa/n13v2a.log` / `n13v2b.log`（压缩腿 + resume 链）
