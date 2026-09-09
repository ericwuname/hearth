# UI/UX 窗口 · 任务指导说明（职责章程）

> 配套冻结基线：`demo-v21j-frozen`（Human OS 前端原型 v21j，2026-08-07 定版冻结）
> 本文件由**顶层窗口**签发，是 UI/UX 窗口的唯一权威职责说明。任何与本文冲突的理解，以本文 + 冻结基线源码为准。

---

## 1. 你的角色与边界

| 项 | 说明 |
|---|---|
| **角色** | UI/UX 窗口 = **施工方**（仅前端样式/交互实现） |
| **起点** | 从 `git checkout demo-v21j-frozen` 拿到冻结基线，在其上 fork 演进 |
| **你对什么负责** | 前端视觉、布局、交互手感、可达性；照顶层下发的任务书实现 |
| **你不负责** | 后端事实产生、架构判断、评审、验收、任务书起草——这些归顶层 |
| **硬边界** | **禁止改动后端事实源**；**禁止把 UI 焊进内核**（见 §3）；**禁止破坏既有联动契约**（见 §4） |

你不是独立决策者，而是**受任务书驱动的施工方**。拿到的每一条需求先判断属于 A/B/C/D 哪类（§3），D 类一律回顶层出任务书，不得擅自开工。

---

## 2. 冻结基线（你的起点，不可擅改其契约）

基线三件套（已锁定，git tag `demo-v21j-frozen`）：

- `docs/codex-desktop-demo.html` —— 原型本体（三栏 + 右栏工作台 + 联动）
- `docs/codex-desktop-demo.regress.js` —— **冻结门禁（39 断言 ALL PASS）**
- `docs/human-os-design.md` —— §6 设计全记录，§6.13 定版冻结声明

**冻结门禁（每次改动后必须仍绿）**：
- 结构：`section 3/3`、`aside 1/1`、`div 206/206` 平衡；全脚本 `new Function` 语法 OK
- 行为：回归 **39 断言 ALL PASS（exit 0）**
- 复跑命令：`node docs/codex-desktop-demo.regress.js`

> 注意：HTML 头部已加 `❄ 定版冻结` 注释。它不是装饰，是给施工方的**禁改提示**——在冻结文件上直接改样式违反本章程，请从 tag fork 到你的工作分支。

---

## 3. 必须遵守的架构铁律（踩中任意一条都算事故）

### 3.1 事实产生权公理
**后端产生事实，前端投影事实。前端可以有状态，但不能有真相。**
- 判定式：**这个数据会不会被别人引用？**（进报告 / 被审计 / AI 读 / 需 grep 回溯）→ 必须后端独占；只影响此刻这屏 → 前端独有且在 FE。
- BE 给语义（如 `severity:"warn"`），FE 给呈现（FE 决定黄色）。**BE 绝不返回颜色/HTML/坐标**。
- 防腐化铁律：**禁止「FE 先本地算着用、以后再挪后端」**——这是解耦架构腐化的起点。

### 3.2 UI 变更四分类（开工前必判）
- **A 呈现 / B 新视图复用旧事实** → 后端零影响，可直接做。
- **C 需要新事实** → 只增不改（超集原则），且必须先让顶层把事实产生权定到后端。
- **D 新人类介入点**（新增审批/澄清/确认等控制流入口）→ **唯一真风险**，必须回顶层出任务书，禁擅自加。

### 3.3 WP-0 通用交互原语
任何「人类介入点」必须建在 `InteractionRequest{id,kind:String,blocking,timeout,on_timeout,payload}` / `Response{id,by,payload,latency_ms}` 之上。
- 内核只认 `blocking/timeout/id`，**绝不 match kind、绝不解析 payload**。
- `kind` 必须是开放字符串；做成 Rust enum = 把 UI 焊进内核（禁止）。
- 内含 R1 id 强校验 / R2 一次性消费防重放 / R3 客户端服务端共用类型。

### 3.4 Observer 冻结裁决（不可破）
Observer OS 已冻结为**顶栏徽章 + 抽屉**，**不进右栏常驻第三竖栏**。这是架构裁决，任何把它做成右栏常驻第三列的方案都直接驳回。

### 3.5 可达性 ②.5 层（血泪教训）
DOM 里有、逻辑全对、JS 断言全 PASS，**仍可能用户点不到**——CSS 裁剪容器（`nowrap+overflow:hidden+ellipsis`）会把按钮省略号裁掉。
- 任何新增可点击控件，必须保证不被裁剪容器吞掉。
- 自检法：结构约束断言「裁剪类内出现 `onclick` 一律判失败」；且要证明该断言真能失败（旧写法可触发）。

### 3.6 审计工具自身被审计
`codex-desktop-demo.regress.js` 既是门禁也是被测对象。**你新增的 UI 行为，必须同步补 DOM 桩冒烟断言**。
- 铁律：**断言不能失败 = 断言不存在**。每条断言都要能证明它真能 fail（用旧错误写法验证过）。

---

## 4. 既有联动契约（改前端时不可破坏）

这些是 v21h/v21i/v21j 已锁定的行为契约，任何 UI 改动都须保其可继承部分 PASS：

| 契约 | 关键锚点 | 不可破坏点 |
|---|---|---|
| 左栏选中对话 → 中央切内容 → 右栏投影该对话独立工作区 | `pickSession(id)`→`rsbSetConv(id)`；`CONV_WS[对话id]` | 每会话 ws 隔离；上下文标签 `#rsbCtx`；空态防串台 |
| 新建对话即时生成示例工作区 | `newConv`→`CONV_WS[id]=genSampleWS(conv)` | 不再是空态；骨架 README/计划/活动 |
| 点文件 → 中央跳到提及该文件的消息 | `rsbOpen(k,true)`→`rsbJumpToMention(k)` | 只在主动点文件时触发（不干扰正向联动）；refs 优先 + 文本扫描回退 + 软降级 |
| 点消息 📂 → 右栏高亮对应文件 | `msgJumpToFiles(mid)` | `jump=false` 防与上式成死环；refs 反向 + 文本扫描 + 软降级 |
| 三栏可拖动 | `#hsbDiv`/`#rsbDiv` + `MIN_MAIN=320` clamp | 中间区不被压缩/遮挡；左右栏钳位 |
| Observer 顶栏徽章 | 顶栏 DOM | 不进右栏 |

**DOM 锚点（勿随意改 id/class，否则断言失效）**：消息 `data-mid`、文件树项 `data-k`、右栏 `#rsbTree/#rsbCode/#rsbArts/#rsbPlan/#rsbAct`、上下文 `#rsbCtx`、splitter `#hsbDiv/#rsbDiv`。

---

## 5. 接任务与交付流程

1. **收任务书**：顶层下发任务书（含需求、范围、验收口径、门禁要求）。任务书未到的需求，先回顶层澄清，不猜。
2. **分类**：按 §3.2 判 A/B/C/D。D 类无论多小，回顶层出任务书再动。
3. **fork 开工**：`git checkout -b ui-ux/<feature> demo-v21j-frozen`，在分支上改。
4. **同步补门禁**：新增/改动 UI 行为，必须在 `regress.js` 补 DOM 桩冒烟断言（§3.6）。
5. **自验**：跑 `node docs/codex-desktop-demo.regress.js`，**39 门禁 + 你新增断言全 PASS**；结构平衡；`new Function` 语法 OK。
6. **交付证据**：向顶层交付时附带——改动说明、回归输出（ALL PASS 截图/文本）、结构校验结果、本条改动的 DOM 桩冒烟证据。
7. **顶层验收**：顶层按任务书验收，门禁红则打回。

---

## 6. Do / Don't 速查

**Do**
- 从 `demo-v21j-frozen` fork，不直接在冻结文件改。
- 纯 A/B 类改动先保 39 断言绿，再考虑扩展。
- 新增可点击控件自查可达性（不被裁剪容器吞）。
- 每改必跑门禁，交付带证据。

**Don't**
- ❌ 改后端事实源 / 在 FE 算本属 BE 的真相。
- ❌ 把 UI 焊进内核（kind 用 enum、match 具体类型）。
- ❌ 擅自加 D 类人类介入点（审批/澄清/确认等控制流）。
- ❌ 把 Observer 做成右栏常驻第三列。
- ❌ 只加「永远 PASS」的虚假断言（断言必须能失败）。
- ❌ 删/改既有 DOM 锚点 id/class 而不同步更新 39 断言。

---

## 7. 升级回顶层的情形（必须回，不得自行决）

- 需求触碰 **D 类**（新控制流/人类介入点）。
- 需求需要**新事实**且事实产生权未定（C 类）。
- 改动会**破坏既有联动契约**或需改 DOM 锚点。
- 对**架构公理**（事实产生权 / WP-0 / Observer 冻结）有疑问或想挑战。
- 门禁出现**结构性冲突**（39 断言与需求不可兼得）。

以上一律回顶层出任务书或做架构裁决，UI/UX 窗口不越权。

---

## 8. 快速上手

```bash
# 1. 拿到冻结基线
git fetch --tags
git checkout -b ui-ux/<feature> demo-v21j-frozen

# 2. 本地预览
#    直接用浏览器打开 docs/codex-desktop-demo.html
#    （或起静态服务：python3 -m http.server 后访问）

# 3. 跑门禁
node docs/codex-desktop-demo.regress.js
# 期望：RESULT: ALL PASS（39 断言），exit 0

# 4. 结构校验
node -e 'const fs=require("fs");const s=fs.readFileSync("docs/codex-desktop-demo.html","utf8");const c=r=>(s.match(r)||[]).length;console.log("section",c(/<section\b/g)+"/"+c(/<\/section>/g),"aside",c(/<aside\b/g)+"/"+c(/<\/aside>/g),"div",c(/<div\b/g)+"/"+c(/<\/div>/g));'
```

---

*签发：顶层窗口（规划/评审/验收职责） · 生效：2026-08-07 · 基线：`demo-v21j-frozen`*
*本说明随架构铁律演进时由顶层修订，UI/UX 窗口收到新版即日起执行。*
