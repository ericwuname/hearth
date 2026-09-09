# UI/UX 任务书 #01 · 前端呈现优化（A 类四件套）

> 签发：顶层窗口 · 执行方：UI/UX 窗口（施工） · 基线：`demo-v21j-frozen` · 日期：2026-08-07
> 配套章程：`docs/ui-ux-window-guide.md` · 冻结门禁：39 断言 ALL PASS + 结构平衡

---

## 0. 决策（顶层拍板）

UI/UX 窗口提交的 4 条优化，**逐条判定均为 A 类（纯呈现 / 新视图复用旧事实），无 C 类（需新事实）、无 D 类（新人类介入点 / 控制流）**，均不破坏冻结契约，全部批准。

**施工顺序（推荐）**：① 暗色主题 → ② 焦点环 → ④ 减少动效 → ③ 空态/加载态。
**交付分两轮**：Round 1 = ①②（杠杆最高、彼此正交、不碰联动契约，一轮可交付带门禁证据）；Round 2 = ③④。每轮结束门禁全绿再交付验收。

> 从 `git checkout -b ui-ux/ux-polish-01 demo-v21j-frozen` 开工。Round 1 现在即可启动。

---

## 1. 全局门禁（每条都必须满足）

- `node docs/codex-desktop-demo.regress.js` → **39 断言 ALL PASS（exit 0）**。
- 结构：`section 3/3`、`aside 1/1`、`div` 平衡；全脚本 `new Function` 语法 OK。
- **DOM 锚点 id/class 不得改**：`data-mid`、`data-k`、`#rsbTree`、`#rsbCode`、`#rsbArts`、`#rsbPlan`、`#rsbAct`、`#rsbCtx`、`#hsbDiv`、`#rsbDiv`、`.rsb-empty-ws`。新增元素若用 `<div>` 须成对平衡（门禁查平衡不查精确数）。
- 禁止 D 类、禁改后端事实源、禁焊内核（WP-0 kind 做 enum）、禁把 Observer 改成右栏第三列。
- 每条新增/改动行为，须在 `regress.js` 补 DOM 桩冒烟断言，且断言**必须能失败**（旧错误写法可触发）。

---

## 2. ① 深色主题（A 类 · Round 1）

**预检（开工第一步，门禁前置）**
- `grep` 全文件，确认除 `:root` 定义外**无硬编码裸色**（`#xxx` / `rgb()` / `hsl()` 直接写在规则里）。
- 若有裸色：**先抽成 `var(--...)` 再覆写**，禁止只覆写部分变量导致暗色下出现白块/不可读。

**实现**
- 顶栏加主题切换按钮（纯 `<button>`，不为此新增 `<div>` 破坏计数；若确需 div 须平衡），点击在 `document.documentElement.dataset.theme` 的 `""` / `"dark"` 间 toggle。
- `[data-theme="dark"]` **覆写 `:root` 全部颜色变量**（含 `--ai-bg`/`--ai-tx`，保证 AI 气泡在暗色下可读）。
- 可选：localStorage 记忆主题偏好——**仅 FE 本地状态，禁回传后端当作事实**（对照事实产生权公理）。

**禁区**
- 主题切换仅改 `data-theme` 与 CSS 呈现；**不得引入任何 InteractionRequest / 控制流**；不影响任何 agent 行为。

**验收**
- 切暗色后全三栏可读、无白块、无对比度失效；门禁仍 39 PASS；无新增 D 类介入点；预检裸色清单附证据。

---

## 3. ② 键盘焦点环（A 类 · Round 1）

**实现**
- 所有可交互元素（`button` / `.ic` / `.hsb-item` / `.ti` / `.rsb-tab` / 文件树项等）加 `:focus-visible` 焦点环，如 `outline:2px solid var(--accent);outline-offset:2px`；移除默认难看 outline 时改用 focus-visible 接管。
- 纯 CSS，**不增 DOM 锚点**；呼应 OS 切换器已支持的 `Tab/1/2/3/Esc` 快捷键。

**验收**
- 纯键盘 Tab 可清晰看到焦点位置；门禁仍 PASS；焦点环在明暗两主题下均可见（若 Round 1 已含①，须双主题验证；若先交②后交①，注明明暗各自验证计划）。

---

## 4. ③ 空态 / 加载态打磨（A 类 · Round 2）

**实现**
- 中央 `stream`、右栏各 tab 空态（`.rsb-empty-ws` 等）加 skeleton 抖动态 / 克制的空态插画，符合设计 §0 极简原则。
- 加载态仅 CSS 动画，纯呈现。

**硬约束**
- **必须保留 `.rsb-empty-ws` class 与空态触发契约**（regress.js 空态断言依赖它）；只换视觉皮肤，不动触发逻辑与类名。

**验收**
- 空态/加载态视觉成立；门禁空态断言仍 PASS（类与触发不变）；不引入新 DOM 锚点冲突。

---

## 5. ④ 减少动效（A 类 · Round 2）

**实现**
- 加 `@media (prefers-reduced-motion: reduce)`，关闭 `portin` / `spin` / `hsbflash` 等动画（或降为无过渡），属无障碍必做项。
- 纯 CSS。

**验收**
- 开启系统"减少动态效果"后界面无动画/无晕动风险；门禁 PASS；补一条"开启 reduced-motion 后动画节点仍渲染但无 transition"的 DOM 桩自检（断言能失败）。

---

## 6. 交付证据要求（每轮）

向顶层交付时附：
1. 改动 diff 说明（哪条、改了什么、是否碰 DOM 锚点）。
2. 回归输出（`RESULT: ALL PASS` 文本）。
3. 结构校验结果（section/aside/div 平衡 + new Function OK）。
4. 本条专项 DOM 桩冒烟证据（尤④ reduced-motion、② 焦点环可见性自检、① 裸色预检清单）。
5. 明示：是否引入任何 D 类介入点（应为"无"）。

---

## 7. 升级回顶层情形（同章程 §7）

需求触碰 D 类 / 需新事实(C 类) / 改动破坏既有联动契约或需改 DOM 锚点 / 对架构公理有疑问 / 门禁结构性冲突——一律回顶层出任务书或做裁决，不得自行决。

---

*本任务书随基线 `demo-v21j-frozen` 签发；任何与 `ui-ux-window-guide.md` 冲突处以章程为准。顶层保留验收否决权。*
