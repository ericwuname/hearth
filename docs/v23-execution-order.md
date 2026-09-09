# v23 执行定版（一轮自主施工范围）— 红队并入 + WP-0a/WP-0

> 日期：2026-08-02 | 基线：HEAD `1255498` | 依据：`top-level-plan-v23.md` + `-supplement.md` + `audit-findings-v22.md`（红队 12 🔴）
> 性质：执行窗口一轮自主施工——**范围收缩，保 WP-0 地基 + 红队 P0 快修 + 文档修正**，其余挂账。

---

## 一、审查结论（三文件 + 红队并入）

1. **v23 规划扎实**：WP-0 通用交互原语 + 三道隔离层 + 禁止清单与 MEMORY 长期公理一致（事实产生权 / 内核不认 kind / Observer 独立 crate）。
2. **supplement 补充有效**：WP-0a 前置修复独立化（避免"重构 + 修 bug"混合调试）✅ 采纳。
3. **红队已打中（12+ 🔴）**：RT4 审批门绕过 = WP-0 阻塞前置（规划 §2.4 已预见）；新增 WT6 working 假完成 / WT17 prompt 注入 / N7 密钥串用 / RT9 会话 404 等——**不修这些，v23 新架构建在旧漏洞上**。

## 二、本轮执行范围（一晚自主）

| 序 | 项 | 内容 | 对应 |
|---|---|---|---|
| 1 | **WP-0a** | RT4 三缺陷：R1 id 强校验 / R2 一次性消费 / R3 decision 契约统一（codex-cli 422 闭环） | audit RT4 / supplement §1 |
| 2 | **WP-0** | InteractionRequest/Response + 通用暂停管道 + 审批迁移 kind="approval" + 通用路由 + 门禁 | v23 §2 |
| 3 | **红队 P0 快修** | WT6 working 残留假完成 / WT17 prompt 注入 TOML（window-framework Python 侧） | audit A 组 |
| 4 | **文档 🔵 修正** | CT3 manual 自相矛盾 / CT6 acceptance 过度声称 / CT8 README 漏跑 v09/v10 | audit C 组 |

## 三、挂账（下轮，本轮不做）

- **WP-1 ~ WP-10**（26h 剩余：事件信封/SSE/Observer/指标/规则/熔断/产物/摘要/前端）
- 红队：RT2 civ 三层静默 / RT3 seccomp 扩展 / RT6 并发限流 / RT9 会话持久化 / WT1 bash 零沙箱 / N7 密钥串用（证据不足，需复核）/ CT1 wiring AST 强化 / CT2 测试自身验证
- v23 §8 挂账 Q1-Q11（含 Q9 API_KEY 默认全放行——单独排，不并入）

## 四、门禁（本轮验收）

- WP-0a：① 现有审批测试零改动全过 ② 错 id 必拒（新测试）③ 重复提交必拒（新测试）④ `codex-cli approve` 不再 422（实测）
- WP-0：① agent-core/tool-runtime grep 不到 `"approval"`/`"clarification"` 字面量分支（审批已迁移为 kind 实例）② 现有 214 tests 零回归 ③ 新增 dummy kind 内核 0 行改动
- 全局：`cargo fmt` 0 / `clippy` 0 / 全测试过

## 五、验收报告交付

`docs/acceptance-v23-wp0.md`：逐项证据（测试输出 + grep + 实测），红队对应项闭环状态表。
