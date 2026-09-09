# UI/UX 施工窗口验收记录 #01（顶层守门员 · 闸门判定）

> **日期**：2026-08-21
> **基线**：`demo-v21j-frozen`（HTML 头 `❄ 定版冻结`）
> **工作分支**：`ui-ux/ux-polish-01`（自冻结 tag fork）
> **审计原则**：不信报告信源码；测试必须能失败；生产路径实际接线
> **判定**：**PASS（🔴=0，实现率=1.0）** — 可收口归档

---

## 1. 实测证据（源码级，非照抄报告）

| 验收项 | 方法 | 结果 |
|---|---|---|
| 回归门禁 | 实跑 `node docs/codex-desktop-demo.regress.js` | **90 断言 ALL PASS（exit 0）** |
| 脚本语法 | `new Function(js)` | OK（82594 字符） |
| 结构平衡 | tag 计数 | `section 3/3`、`aside 1/1`、`div 206/206` 全平衡 |
| DOM 锚点存活 | grep `data-mid\|data-k\|#rsbTree\|#hsbDiv\|#rsbDiv\|#human\|.stream\|#portHuman\|#rsbCtx` | **15 处全在，0 改名/0 删** |
| 冻结头注释 | grep `❄ 定版冻结` | 在 line 4，未动 |
| 暗色主题真实接线 | grep `:root`(9) + `[data-theme="dark"]`(16) + 🌓按钮(522) + `toggleTheme`(700) + `initTheme`(706) | **全接；`toggleTheme` 仅翻转 `data-theme` + localStorage，无 fetch/无 InteractionRequest（FE-only）** |
| 联动逻辑 intact | grep `rsbSetConv`(1827)/`toggleRsb`(1906)/`rsbJumpToMention`(1990)/`msgJumpToFiles`(2022) | **4 函数全在**；`toggleRsb` 含 ⑤ 修复（关闭时 `flexBasis=RSB_OPEN?rsbW+'px':''` 清空行内，line 1912） |
| D 类越界检测 | grep `InteractionRequest\|NeedApproval\|ApprovalState\|confirm(\|window.prompt` | **0 命中**；唯一 `realModePanel`(2083) 为预冻结既有元素、未改（D 类仅说明不施工，符合契约） |
| 测试能失败 | regress.js 全程 `[自检]` + `LEGACY_*` 旧写法样例 | **每条修复断言配套"能抓出旧写法"自检**（如 line 110/203/209/220/227/240/249/265/271/278/285/295），证明非永真空转 PASS |

---

## 2. 范围纪律

- 仅 2 个原型文件改动：`codex-desktop-demo.html`（+183/−104 行，CSS + 主题切换 + ⑤折叠 bug 修）、`codex-desktop-demo.regress.js`（+109 行门禁）。
- 浅色冻结区视觉 0 改动；39 条基线断言全部继承 PASS（含于 90 内，行为组 41 条跑真实 DOM 桩）。
- `MEMORY.md`、§4 联动契约、tag `demo-v21j-frozen` 0 触碰。

---

## 3. 偏离 / 遗留（🟡，不阻塞过闸）

1. **分支未提交**（报告 §6.2）→ 顶层已收口：本判定同批提交 `ui-ux/ux-polish-01`（仅本地、不 merge master、不碰冻结 tag）。
2. **联动绿闪语义开放项**（报告 §6.1）：已改 accent 色，是否完全去掉只留静默跳转待用户定；属 A 类可逆。
3. **未做项**（报告 §6.3）：跨浏览器（Safari/Firefox）实测、键盘导航完整走查、WCAG 2.1 AA 对比度量化、响应式断点 —— 需做时回顶层出任务书（仍 A/B 类）。
4. **冻结头注释文案滞后**：line 4 仍写「39 断言」，实际已 90；纯注释无害，建议下轮顺手改准。

---

## 4. 结论与后续

- **过闸**：🔴=0、实现率达标，验收通过。施工窗口分类正确（全 A 类纯呈现，零 C/D）。
- **收口路径 A（已执行）**：commit `ui-ux/ux-polish-01` → 归档。
- **红线重申**：C/D 类（真实后端接入 / 真实审批流 / `realModePanel` 落地）触及控制流，必须先回顶层出任务书，本窗口不越界。
- **下一步候选**（交顶层拍板）：B 扩审计面（跨浏览器+WCAG）/ C 真实后端 / D 组件库抽取·高对比主题。
