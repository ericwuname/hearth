# v23 阶段五施工计划 — 收尾三件套（WP-8/9/10）+ v23 封版

> 基线：WP-0~7 完成（237 tests），Observer 独立 crate 立起，三端口架构地基全部就绪
> 剩余三个 WP 都是"轻量收尾"——分别 2h/1h/2h，可在一批内全部完成

---

## 一、当前 WP 进度

```
WP-0~7 ✅  三端口地基（通用交互/事件链/缺口/Observer 四件套）
WP-8  ⬜  产物登记       ← 本次
WP-9  ⬜  思考摘要       ← 本次（与 WP-8 并行）
WP-10 ⬜  前端接真流     ← 本次（WP-8/9 完成后）
```

---

## 二、WP-8：产物登记（2h）

v23 规划 §5.1：agent 在 write_file / edit 成功后 emit `artifact` 事件（path / delta / kind / timestamp）。

| step | 文件 | 动作 | 验证 |
|---|---|---|---|
| ① artifact 类型 | `api/src/lib.rs` | `AgentEvent::Artifact{path, kind, delta_lines, size_bytes}` | cargo check |
| ② emit 钩子 | `tool-runtime` (write_file/edit 成功路径) | 工具执行成功 → emit Artifact 事件 | 测试：write_file → 事件流含 Artifact |
| ③ open_artifact | `service/src/session.rs` | `POST /session/{id}/artifact/open`：读产物文件内容 → 返回 | curl 测试返回文件内容 |
| ④ 前端约束 | `docs/` | 前端**零文件系统调用**——靠 artifact 事件流 + open_artifact 路由获取产物 | grep 验证 |

---

## 三、WP-9：思考摘要（1h，与 WP-8 并行）

v23 规划 §5.1：span 关闭时生成 `think_summary`——一句话人类可读的描述（"我在实现登录API"），风格可参数化。

| step | 文件 | 动作 | 验证 |
|---|---|---|---|
| ① summary 生成 | `agent-core/src/loop.rs` | 每个 phase 完成时，调 LLM 生成一句摘要（"我在做 X"），写入 `SpanClose` 或独立 `ThinkSummary` 事件 | replay 模式：摘要非空 |
| ② 风格参数 | `loop.rs` | `think_summary_style` 配置（默认 "concise"），不硬编码语气 | 编译配置化 |
| ③ 事件流 | 测试 | ThinkSummary 事件带 span_id → 前端可关联到具体 span | 事件流测试 |

**注意**：WP-9 的 LLM 调用是**可选渲染**——replay 模式下用 mock 摘要，生产模式才调 LLM。不阻塞任何门禁。

---

## 四、WP-10：前端接真流（2h，WP-8/9 完成后）

v23 规划 §10（唯一等 Human OS 的 WP）：把 demo (`codex-desktop-demo.html`) 的 mock SSE 换成真实后端 SSE 事件流。

| step | 动作 | 验证 |
|---|---|---|
| ① SSE 连接 | 前端 `EventSource` 连接 `GET /api/v1/sessions/:id/events`（WP-2 产物） | 控制台看到信封事件（seq/ts/span_id） |
| ② span 渲染 | 前端根据 `SpanOpen/SpanClose` 构建可折叠的相位时间线 | 点击展开/折叠 |
| ③ 交互渲染 | `InteractionRequested` → 渲染审批/澄清 UI；`InteractionResolved` → 更新状态 | 审批按钮可用 |
| ④ artifact 渲染 | `Artifact` 事件 → 产物卡片列表；`open_artifact` → 预览文件内容 | 文件卡片可点击打开 |
| ⑤ 断线重连 | `Last-Event-ID` → 重连后事件连续 | 关掉 SSE 3 秒 → 重开 → 无丢事件 |
| ⑥ 后退兼容 | demo 在原位置打开，不破坏已有 UI | 旧 mock 按钮保留为 fallback |

---

## 五、执行顺序

```
WP-8 (2h) ──┐
            ├── WP-10 (2h) ──→ v23 封版
WP-9 (1h) ──┘
```

WP-8 和 WP-9 **完全独立**，改不同的文件——可以并行。WP-10 需要 WP-8 的 artifact 事件流 + open_artifact 路由，所以排在后面。

**总计 ~5h**。

---

## 六、门禁

| WP | 门禁 |
|---|---|
| WP-8 | ① write_file 后事件流含 Artifact ② `open_artifact` 返回正确文件内容 ③ 前端 grep 无文件系统调用 |
| WP-9 | ThinkSummary 事件含 span_id（可关联到具体 span） |
| WP-10 | ① demo 零改动消费真实事件流即跑通（A-08 总闸门）② 断线重连不丢事件 |
| 全 | fmt 0 / clippy 0 / 237 tests 不得回归 |

---

## 七、v23 封版后

```
v23 全部 11 个 WP 闭环：
  三端口架构（Human OS 前端 + AI OS 内核 + Observer OS 第三权）
    ├── WP-0      通用交互原语（内核不认 kind）
    ├── WP-1      事件信封与 span 树
    ├── WP-2      SSE 出口与录制重放
    ├── WP-3      规划缺口推导器
    ├── WP-4~7    Observer 四件套（独立 crate，零执行权，G3 确定性）
    ├── WP-8      产物登记
    ├── WP-9      思考摘要
    └── WP-10     前端接真流

  剩余待办（v23 后维护期处理）：
    - RT3 seccomp 扩展（用户挂起）
    - Q1~Q11 未决项（见 top-level-plan §8）
    - 季度体检（基准+应力+回放）
```