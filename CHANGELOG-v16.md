# CHANGELOG v16.0 — 封刀轮

锻造终结版。不再加新维度，清掉最后三笔债 + 出边界白皮书 + 锻造综述。

## Added
- **experience 文件持久化**（JSONL append-only）：`ExperienceStore.set_path()` + `append_to_disk()`，重启后保留经验数据（`GET /api/v1/experience/metrics` 可验证）。
- **CostGuard 真值接地**：`loop.rs:989` 从硬编码 `cost_ratio: 0.0` 改为 `self.nervous.cost_ratio()`（`nervous-system/src/lib.rs` 新增 `fn cost_ratio()`）。
- **wiring 第 10-11 条红线断言**：`experience-persists`（持久化方法存在）+ `cost-guard-live`（CostGuard 真实读数），合计 11 条全红。
- **能力边界白皮书** `docs/capability-boundaries.md`：按 L1-L5 难度层 + provider 列出已知边界，附证据锚点。
- **锻造最终报告** `docs/forge-final-report.md`：v12→v16 四���锻造完整证据链。
- **守门员审计 v16** `gatekeeper-audit-v16.md`：11 条接线核实全绿。

## Fixed
- 无功能性 bug 修复（v15 回放/择脑已达标，本轮只清债 + 写稿）。

## Changed
- **维护期声明**：v16 后锻造结束，进入季度体检（每季度跑一次基准+应力+回放）。
- 降级：S5 人侧数据未执行，标记为"待补"（不影响 🔴 红线）。

## Metrics
- wiring 11/11 全绿 ✅
- cargo check 零告警 ✅
- 净代码改动 ~74 行

## Known Debt
- T19-merge-duplicate 所有模型全挂（工具链盲区，非模型问题）
- 人侧数据（价值系数）待补
- ST8 approval gate 1/3（安全属性未突破，需进一步分析）
